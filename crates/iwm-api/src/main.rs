use std::{
    env, fs, io,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime},
};

use anyhow::{Context, Result};
use axum::{
    extract::{DefaultBodyLimit, Multipart, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use iwm_detector::{detect_package, load_package, selected_executable, DetectionVerdict};
use iwm_runtime_model::{read_runtime_package_dir, validate_runtime_package, CompatibilityLevel};
use serde::Serialize;
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use tokio::{io::AsyncWriteExt, net::TcpListener, sync::Semaphore, time::timeout};
use tower_http::services::{ServeDir, ServeFile};

const MAX_UPLOAD_BYTES: u64 = 512 * 1024 * 1024;
const MAX_PACKAGE_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_STORED_GAMES: usize = 16;
const GAME_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const PARSE_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone)]
struct AppState {
    games_dir: PathBuf,
    staging_dir: PathBuf,
    static_dir: PathBuf,
    parse_slot: Arc<Semaphore>,
}

struct Config {
    bind: SocketAddr,
    data_dir: PathBuf,
    static_dir: PathBuf,
}

impl Config {
    fn from_env() -> Result<Self> {
        let bind = env::var("IWM_BIND")
            .unwrap_or_else(|_| "0.0.0.0:3000".into())
            .parse()
            .context("IWM_BIND must be a socket address")?;
        Ok(Self {
            bind,
            data_dir: env::var_os("IWM_DATA_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("data")),
            static_dir: env::var_os("IWM_STATIC_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("runtime/dist")),
        })
    }
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
struct UploadResponse {
    id: String,
    status: &'static str,
    compatibility: CompatibilityLevel,
    package_url: String,
    warnings: Vec<String>,
}

#[derive(Serialize)]
struct ErrorResponse {
    status: &'static str,
    error: String,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn unsupported_media(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNSUPPORTED_MEDIA_TYPE,
            message: message.into(),
        }
    }

    fn unprocessable(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            message: message.into(),
        }
    }

    fn unavailable(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    fn internal(error: impl std::fmt::Display) -> Self {
        eprintln!("internal error: {error}");
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "The server could not process this game.".into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                status: "failed",
                error: self.message,
            }),
        )
            .into_response()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    let games_dir = config.data_dir.join("games");
    let staging_dir = config.data_dir.join("staging");
    fs::create_dir_all(&games_dir)?;
    fs::create_dir_all(&staging_dir)?;
    cleanup_expired_games(&games_dir)?;

    let state = AppState {
        games_dir,
        staging_dir,
        static_dir: config.static_dir,
        // ponytail: one parser keeps memory bounded; add a job queue when real traffic needs it.
        parse_slot: Arc::new(Semaphore::new(1)),
    };
    let app = app(state);
    let listener = TcpListener::bind(config.bind).await?;
    println!(
        "iwm-api {} listening on {}",
        env!("CARGO_PKG_VERSION"),
        config.bind
    );
    axum::serve(listener, app).await?;
    Ok(())
}

fn app(state: AppState) -> Router {
    let api = Router::new()
        .route("/v1/games", post(upload_game))
        .fallback(|| async { ApiError::not_found("Unknown API endpoint") });
    let static_files = ServeDir::new(&state.static_dir)
        .fallback(ServeFile::new(state.static_dir.join("index.html")));

    Router::new()
        .route("/healthz", get(healthz))
        .nest("/api", api)
        .nest_service("/games", ServeDir::new(&state.games_dir))
        .fallback_service(static_files)
        .layer(DefaultBodyLimit::max(
            (MAX_UPLOAD_BYTES + 1024 * 1024) as usize,
        ))
        .with_state(state)
}

async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn upload_game(
    State(state): State<AppState>,
    multipart: Multipart,
) -> std::result::Result<Json<UploadResponse>, ApiError> {
    let permit =
        state.parse_slot.clone().try_acquire_owned().map_err(|_| {
            ApiError::unavailable("Another game is being parsed. Try again shortly.")
        })?;
    let (staging, input_path, id) = save_upload(multipart, &state.staging_dir).await?;
    let games_dir = state.games_dir.clone();

    let task = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        process_game(&input_path, &staging, &games_dir, id)
    });

    // spawn_blocking cannot be cancelled safely; the held permit keeps timed-out work bounded.
    let response = timeout(PARSE_TIMEOUT, task)
        .await
        .map_err(|_| ApiError {
            status: StatusCode::GATEWAY_TIMEOUT,
            message: "Parsing took longer than 120 seconds.".into(),
        })?
        .map_err(ApiError::internal)??;
    Ok(Json(response))
}

async fn save_upload(
    mut multipart: Multipart,
    staging_dir: &Path,
) -> std::result::Result<(TempDir, PathBuf, String), ApiError> {
    let staging = tempfile::tempdir_in(staging_dir).map_err(ApiError::internal)?;
    let mut saved_upload = None;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::bad_request("Malformed multipart upload"))?
    {
        if field.name() != Some("game") || saved_upload.is_some() {
            return Err(ApiError::bad_request(
                "Upload exactly one file in the 'game' field",
            ));
        }
        let filename = field
            .file_name()
            .ok_or_else(|| ApiError::bad_request("The upload has no filename"))?
            .to_owned();
        let extension = upload_extension(&filename).ok_or_else(|| {
            ApiError::unsupported_media("Only .exe and .zip uploads are accepted")
        })?;
        let input_path = staging.path().join(format!("upload.{extension}"));
        let mut output = tokio::fs::File::create(&input_path)
            .await
            .map_err(ApiError::internal)?;
        let mut size = 0_u64;
        let mut hasher = Sha256::new();

        while let Some(chunk) = field
            .chunk()
            .await
            .map_err(|_| ApiError::bad_request("Upload stream ended unexpectedly"))?
        {
            size = size
                .checked_add(chunk.len() as u64)
                .ok_or_else(|| ApiError::bad_request("Upload size overflow"))?;
            if size > MAX_UPLOAD_BYTES {
                return Err(ApiError::bad_request("Upload exceeds the 512 MiB limit"));
            }
            hasher.update(&chunk);
            output.write_all(&chunk).await.map_err(ApiError::internal)?;
        }
        output.flush().await.map_err(ApiError::internal)?;
        if size == 0 {
            return Err(ApiError::bad_request("The uploaded file is empty"));
        }
        saved_upload = Some((input_path, format!("{:x}", hasher.finalize())));
    }

    let (input_path, id) =
        saved_upload.ok_or_else(|| ApiError::bad_request("Missing 'game' upload field"))?;
    Ok((staging, input_path, id))
}

fn upload_extension(filename: &str) -> Option<&'static str> {
    match Path::new(filename)
        .extension()
        .and_then(|extension| extension.to_str())?
        .to_ascii_lowercase()
        .as_str()
    {
        "exe" => Some("exe"),
        "zip" => Some("zip"),
        _ => None,
    }
}

fn process_game(
    input_path: &Path,
    staging: &TempDir,
    games_dir: &Path,
    id: String,
) -> std::result::Result<UploadResponse, ApiError> {
    cleanup_expired_games(games_dir).map_err(ApiError::internal)?;
    let final_dir = games_dir.join(&id);
    if let Some(response) = read_valid_package(&final_dir, &id)? {
        return Ok(response);
    }

    let stored_count = fs::read_dir(games_dir)
        .map_err(ApiError::internal)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .count();
    if stored_count >= MAX_STORED_GAMES {
        return Err(ApiError::unavailable(
            "The Beta package cache is full. Try again later.",
        ));
    }

    let loaded = load_package(input_path).map_err(|error| {
        eprintln!("package load failed: {error}");
        ApiError::unprocessable("This file is not a readable IWanna package")
    })?;
    let detection = detect_package(&loaded).map_err(|error| {
        eprintln!("detection failed: {error}");
        ApiError::unprocessable("This file could not be identified")
    })?;
    match detection.verdict {
        DetectionVerdict::Gm8Likely => {}
        DetectionVerdict::GmsLikely => {
            return Err(ApiError::unprocessable(
                "GameMaker Studio packages are outside this Beta",
            ));
        }
        DetectionVerdict::Unknown => {
            return Err(ApiError::unprocessable(
                "This package could not be identified as a supported GM8 game",
            ));
        }
        DetectionVerdict::Blocked => {
            return Err(ApiError::unprocessable(
                "This package uses an unsupported engine or layout",
            ));
        }
    }

    let executable = selected_executable(&loaded).map_err(ApiError::unprocessable)?;
    let package_dir = staging.path().join("package");
    iwm_parser::build_package(executable, &package_dir, &detection.dlls).map_err(|error| {
        eprintln!("parser failed: {error:#}");
        ApiError::unprocessable("The GM8 package could not be parsed")
    })?;
    if directory_size(&package_dir).map_err(ApiError::internal)? > MAX_PACKAGE_BYTES {
        return Err(ApiError::unprocessable(
            "The generated package exceeds the 1 GiB Beta limit",
        ));
    }

    let package = read_runtime_package_dir(&package_dir).map_err(ApiError::internal)?;
    let validation = validate_runtime_package(&package);
    if !validation.valid {
        eprintln!(
            "generated package failed validation: {:?}",
            validation.errors
        );
        return Err(ApiError::internal("generated package failed validation"));
    }

    fs::rename(&package_dir, &final_dir).map_err(ApiError::internal)?;
    let mut warnings = detection.warnings;
    warnings.extend(package.manifest.warnings.clone());
    warnings.sort();
    warnings.dedup();
    Ok(UploadResponse {
        id: id.clone(),
        status: "ready",
        compatibility: package.manifest.compatibility,
        package_url: format!("/games/{id}"),
        warnings,
    })
}

fn read_valid_package(
    package_dir: &Path,
    id: &str,
) -> std::result::Result<Option<UploadResponse>, ApiError> {
    if !package_dir.exists() {
        return Ok(None);
    }
    match read_runtime_package_dir(package_dir) {
        Ok(package) if validate_runtime_package(&package).valid => Ok(Some(UploadResponse {
            id: id.into(),
            status: "ready",
            compatibility: package.manifest.compatibility,
            package_url: format!("/games/{id}"),
            warnings: package.manifest.warnings,
        })),
        Ok(_) | Err(_) => {
            fs::remove_dir_all(package_dir).map_err(ApiError::internal)?;
            Ok(None)
        }
    }
}

fn cleanup_expired_games(games_dir: &Path) -> io::Result<()> {
    let now = SystemTime::now();
    for entry in fs::read_dir(games_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let modified = entry.metadata()?.modified()?;
        if now.duration_since(modified).unwrap_or_default() > GAME_TTL {
            fs::remove_dir_all(entry.path())?;
        }
    }
    Ok(())
}

fn directory_size(root: &Path) -> io::Result<u64> {
    let mut total = 0_u64;
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                pending.push(entry.path());
            } else if file_type.is_file() {
                total = total
                    .checked_add(entry.metadata()?.len())
                    .ok_or_else(|| io::Error::other("package size overflow"))?;
            }
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{header, Request},
    };
    use tower::ServiceExt;

    #[test]
    fn upload_extension_accepts_only_exe_and_zip() {
        assert_eq!(upload_extension("game.EXE"), Some("exe"));
        assert_eq!(upload_extension("game.zip"), Some("zip"));
        assert_eq!(upload_extension("game.rar"), None);
    }

    #[tokio::test]
    async fn upload_endpoint_rejects_unsupported_file_types() {
        let root = tempfile::tempdir().unwrap();
        let games_dir = root.path().join("games");
        let staging_dir = root.path().join("staging");
        let static_dir = root.path().join("static");
        fs::create_dir_all(&games_dir).unwrap();
        fs::create_dir_all(&staging_dir).unwrap();
        fs::create_dir_all(&static_dir).unwrap();
        let boundary = "iwm-test-boundary";
        let body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"game\"; filename=\"game.txt\"\r\nContent-Type: text/plain\r\n\r\nnot a game\r\n--{boundary}--\r\n"
        );
        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/games")
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(Body::from(body))
            .unwrap();

        let response = app(AppState {
            games_dir,
            staging_dir,
            static_dir,
            parse_slot: Arc::new(Semaphore::new(1)),
        })
        .oneshot(request)
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }
}
