use crate::models::{FileEntry, PackageInputKind};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use walkdir::WalkDir;

#[derive(Debug)]
pub struct LoadedPackage {
    pub source_name: String,
    pub input_kind: PackageInputKind,
    pub root_dir: PathBuf,
    pub executables: Vec<PathBuf>,
    pub dlls: Vec<String>,
    pub files: Vec<FileEntry>,
    pub _temp_dir: Option<TempDir>,
}

pub fn load_package(path: &Path) -> Result<LoadedPackage, String> {
    if path.is_dir() {
        return load_directory(path);
    }

    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_ascii_lowercase())
    {
        Some(ext) if ext == "exe" => load_single_exe(path),
        Some(ext) if ext == "zip" => load_zip(path),
        _ => Err(format!("unsupported input path: {}", path.display())),
    }
}

fn load_directory(path: &Path) -> Result<LoadedPackage, String> {
    let mut executables = Vec::new();
    let mut dlls = Vec::new();
    let mut files = Vec::new();

    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let full = entry.path().to_path_buf();
        let relative = full
            .strip_prefix(path)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let metadata = fs::metadata(&full).map_err(|e| e.to_string())?;
        let extension = full
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        if extension == "exe" {
            executables.push(full.clone());
        }
        if extension == "dll" {
            dlls.push(relative.clone());
        }

        files.push(FileEntry {
            relative_path: relative,
            extension,
            size: metadata.len(),
        });
    }

    Ok(LoadedPackage {
        source_name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        input_kind: PackageInputKind::Directory,
        root_dir: path.to_path_buf(),
        executables,
        dlls,
        files,
        _temp_dir: None,
    })
}

fn load_single_exe(path: &Path) -> Result<LoadedPackage, String> {
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    Ok(LoadedPackage {
        source_name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        input_kind: PackageInputKind::Exe,
        root_dir: path.parent().unwrap_or(Path::new(".")).to_path_buf(),
        executables: vec![path.to_path_buf()],
        dlls: Vec::new(),
        files: vec![FileEntry {
            relative_path: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            extension: "exe".into(),
            size: metadata.len(),
        }],
        _temp_dir: None,
    })
}

fn load_zip(path: &Path) -> Result<LoadedPackage, String> {
    let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    archive.extract(temp.path()).map_err(|e| e.to_string())?;
    load_directory(temp.path()).map(|mut package| {
        package.source_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        package.input_kind = PackageInputKind::Zip;
        package._temp_dir = Some(temp);
        package
    })
}
