use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use walkdir::WalkDir;

use crate::models::{FileEntry, PackageInputKind};

const MAX_ZIP_ENTRIES: usize = 4_096;
const MAX_ZIP_ENTRY_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ZIP_TOTAL_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Debug)]
pub struct LoadedPackage {
    pub source_name: String,
    pub input_kind: PackageInputKind,
    pub root_dir: PathBuf,
    pub executables: Vec<PathBuf>,
    pub dlls: Vec<String>,
    pub files: Vec<FileEntry>,
    pub warnings: Vec<String>,
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

pub fn selected_executable(package: &LoadedPackage) -> Result<&Path, String> {
    match package.executables.as_slice() {
        [exe] => Ok(exe.as_path()),
        [] => Err("no executable found".into()),
        executables => Err(format!(
            "multiple executable candidates found: {}",
            executables
                .iter()
                .map(|path| {
                    path.strip_prefix(&package.root_dir)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .replace('\\', "/")
                })
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn load_directory(path: &Path) -> Result<LoadedPackage, String> {
    let mut executables = Vec::new();
    let mut dlls = Vec::new();
    let mut files = Vec::new();
    let mut warnings = Vec::new();

    for entry in WalkDir::new(path) {
        match entry {
            Ok(entry) if entry.file_type().is_file() => {
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
            Ok(_) => {}
            Err(err) => warnings.push(format!("failed to inspect path: {err}")),
        }
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
        warnings,
        _temp_dir: None,
    })
}

fn load_single_exe(path: &Path) -> Result<LoadedPackage, String> {
    let parent = path.parent().unwrap_or(Path::new("."));
    let mut package = load_directory(parent)?;
    let selected_exe = fs::canonicalize(path).map_err(|e| e.to_string())?;

    package.executables.sort_by_key(|candidate| {
        if fs::canonicalize(candidate)
            .map(|canonical| canonical == selected_exe)
            .unwrap_or(false)
        {
            0
        } else {
            1
        }
    });
    package.source_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    package.input_kind = PackageInputKind::Exe;

    Ok(package)
}

fn load_zip(path: &Path) -> Result<LoadedPackage, String> {
    let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    if archive.len() > MAX_ZIP_ENTRIES {
        return Err(format!(
            "zip contains too many entries: {} (maximum {MAX_ZIP_ENTRIES})",
            archive.len()
        ));
    }

    let mut declared_total_bytes = 0_u64;
    let mut extracted_total_bytes = 0_u64;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|e| e.to_string())?;
        let relative_path = safe_zip_entry_path(entry.name())?;
        reject_special_zip_entry(&entry)?;

        if entry.size() > MAX_ZIP_ENTRY_BYTES {
            return Err(format!("zip entry is too large: {}", entry.name()));
        }
        declared_total_bytes = declared_total_bytes
            .checked_add(entry.size())
            .ok_or_else(|| "zip expanded size overflow".to_string())?;
        if declared_total_bytes > MAX_ZIP_TOTAL_BYTES {
            return Err("zip expanded size exceeds 1 GiB".into());
        }

        let output_path = temp.path().join(relative_path);
        if entry.is_dir() {
            fs::create_dir_all(&output_path).map_err(|e| e.to_string())?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut output = fs::File::create(&output_path).map_err(|e| e.to_string())?;
        let copied = io::copy(&mut entry.take(MAX_ZIP_ENTRY_BYTES + 1), &mut output)
            .map_err(|e| e.to_string())?;
        output.flush().map_err(|e| e.to_string())?;
        if copied > MAX_ZIP_ENTRY_BYTES {
            return Err(format!(
                "zip entry expanded beyond the size limit: {}",
                output_path.display()
            ));
        }
        extracted_total_bytes = extracted_total_bytes
            .checked_add(copied)
            .ok_or_else(|| "zip extracted size overflow".to_string())?;
        if extracted_total_bytes > MAX_ZIP_TOTAL_BYTES {
            return Err("zip extracted size exceeds 1 GiB".into());
        }
    }

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

fn safe_zip_entry_path(name: &str) -> Result<PathBuf, String> {
    let normalized = name.replace('\\', "/");
    let has_drive_prefix = normalized.as_bytes().get(1) == Some(&b':');
    let unsafe_component = normalized.split('/').any(|component| {
        component == ".." || component == "." || is_windows_reserved_name(component)
    });

    if normalized.starts_with('/') || has_drive_prefix || unsafe_component {
        return Err(format!("unsafe zip entry path: {name}"));
    }

    let path = PathBuf::from(normalized);
    if path.as_os_str().is_empty() {
        return Err("zip entry path is empty".into());
    }
    Ok(path)
}

fn is_windows_reserved_name(component: &str) -> bool {
    let stem = component
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches([' ', '.'])
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || stem
            .strip_prefix("COM")
            .or_else(|| stem.strip_prefix("LPT"))
            .is_some_and(|suffix| {
                matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
            })
}

fn reject_special_zip_entry(entry: &zip::read::ZipFile<'_>) -> Result<(), String> {
    let Some(mode) = entry.unix_mode() else {
        return Ok(());
    };
    let file_type = mode & 0o170000;
    if file_type == 0 || file_type == 0o040000 || file_type == 0o100000 {
        return Ok(());
    }
    Err(format!("zip entry is not a regular file: {}", entry.name()))
}
