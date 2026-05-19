# GM8 Detector Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Rust workspace to the existing docs-first repository and build a detector that classifies uploaded IWanna packages as `gm8-likely`, `gms-likely`, `unknown`, or `blocked`.

**Architecture:** Use a small Rust workspace with one library crate for package inspection and one CLI crate for local execution. The detector works on directories, single EXEs, and ZIP archives, builds a normalized file inventory, runs signature heuristics, and emits structured JSON output for later backend integration.

**Tech Stack:** Rust 1.77+, Cargo workspace, `clap`, `serde`, `serde_json`, `zip`, `walkdir`, `sha2`, `tempfile`, Rust test framework

---

## File Structure

Planned files for this phase:

- Create: `Cargo.toml`
- Modify: `.gitignore`
- Modify: `README.md`
- Create: `crates/iwm-detector/Cargo.toml`
- Create: `crates/iwm-detector/src/lib.rs`
- Create: `crates/iwm-detector/src/models.rs`
- Create: `crates/iwm-detector/src/signatures.rs`
- Create: `crates/iwm-detector/src/package.rs`
- Create: `crates/iwm-detector/src/detect.rs`
- Create: `crates/iwm-detector/tests/detect_directory.rs`
- Create: `crates/iwm-detector/tests/detect_zip.rs`
- Create: `crates/iwm-cli/Cargo.toml`
- Create: `crates/iwm-cli/src/main.rs`
- Modify: `docs/notes/sample-corpus.md`

Responsibilities:

- `crates/iwm-detector`: reusable detector library
- `models.rs`: shared structs and JSON output schema
- `signatures.rs`: engine detection signatures and matching rules
- `package.rs`: input expansion and file inventory building
- `detect.rs`: verdict calculation
- `iwm-cli`: developer-facing CLI wrapper
- `docs/notes/sample-corpus.md`: records the project-local sample workflow used by detector smoke tests

### Task 1: Add Cargo Workspace To The Existing Repository

**Files:**
- Create: `Cargo.toml`
- Modify: `.gitignore`
- Modify: `README.md`

- [ ] **Step 1: Write the failing check by verifying the repository does not yet contain a Cargo workspace**

Run:

```bash
cargo test
```

Expected:

```text
error: could not find `Cargo.toml`
```

- [ ] **Step 2: Create the root workspace manifest**

```toml
[workspace]
resolver = "2"
members = [
  "crates/iwm-detector",
  "crates/iwm-cli",
]

[workspace.package]
edition = "2021"
license = "MIT"
version = "0.1.0"

[workspace.dependencies]
clap = { version = "4.5.7", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
sha2 = "0.10"
tempfile = "3.10"
walkdir = "2.5"
zip = "2.1"
```

- [ ] **Step 3: Extend `.gitignore` for Rust workspace outputs while preserving existing local sample and vendor rules**

```gitignore
/target
/out

# Rust and Cargo
Cargo.lock
```

- [ ] **Step 4: Update the root README current-status section with the detector phase**

```md
# iwanna-gm8-web-engine

Browser-playable IWanna MVP targeting legacy GM8-style fangames.

## Current Phase

Phase 1 adds a Rust workspace and a detector that classifies game packages before parser or runtime work begins.

## Local Commands

```bash
cargo test
cargo run -p iwm-cli -- detect --input C:\\path\\to\\game
```
```

- [ ] **Step 5: Run tests again to confirm Cargo now resolves the workspace**

Run:

```bash
cargo test
```

Expected:

```text
error: failed to load manifest for workspace member
```

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml .gitignore README.md
git commit -m "chore: add detector workspace"
```

### Task 2: Create Detector Library and CLI Crates

**Files:**
- Create: `crates/iwm-detector/Cargo.toml`
- Create: `crates/iwm-detector/src/lib.rs`
- Create: `crates/iwm-cli/Cargo.toml`
- Create: `crates/iwm-cli/src/main.rs`

- [ ] **Step 1: Write the failing test by checking the workspace still cannot compile missing crates**

Run:

```bash
cargo test
```

Expected:

```text
failed to read `crates/iwm-detector/Cargo.toml`
```

- [ ] **Step 2: Create the detector crate manifest**

```toml
[package]
name = "iwm-detector"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
tempfile.workspace = true
walkdir.workspace = true
zip.workspace = true
```

- [ ] **Step 3: Create the detector crate root**

```rust
pub mod detect;
pub mod models;
pub mod package;
pub mod signatures;

pub use detect::detect_input;
pub use models::{DetectionReport, DetectionVerdict, EngineFamily, PackageInputKind};
```

- [ ] **Step 4: Create the CLI crate manifest**

```toml
[package]
name = "iwm-cli"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
clap.workspace = true
serde_json.workspace = true
iwm-detector = { path = "../iwm-detector" }
```

- [ ] **Step 5: Create a compile-only CLI shell**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "iwm-cli")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Detect { #[arg(long)] input: String },
}

fn main() {
    let _ = Cli::parse();
}
```

- [ ] **Step 6: Run tests to verify the workspace compiles further**

Run:

```bash
cargo test
```

Expected:

```text
error[E0583]: file not found for module `detect`
```

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml crates/iwm-detector crates/iwm-cli
git commit -m "chore: add detector and cli crates"
```

### Task 3: Define the Detection Model Schema

**Files:**
- Create: `crates/iwm-detector/src/models.rs`
- Modify: `crates/iwm-detector/src/lib.rs`
- Test: `crates/iwm-detector/tests/detect_directory.rs`

- [ ] **Step 1: Write the failing test for serializable report output**

```rust
use iwm_detector::{DetectionReport, DetectionVerdict, EngineFamily, PackageInputKind};

#[test]
fn detection_report_serializes_expected_verdict_names() {
    let report = DetectionReport::minimal(
        "example".into(),
        PackageInputKind::Directory,
        DetectionVerdict::Gm8Likely,
        vec![EngineFamily::Gm8],
    );

    let json = serde_json::to_value(report).unwrap();
    assert_eq!(json["verdict"], "gm8-likely");
    assert_eq!(json["input_kind"], "directory");
}
```

- [ ] **Step 2: Create `models.rs`**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DetectionVerdict {
    Gm8Likely,
    GmsLikely,
    Unknown,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EngineFamily {
    Gm8,
    Gms,
    Unity,
    RpgMaker,
    Clickteam,
    Godot,
    Nwjs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageInputKind {
    Directory,
    Exe,
    Zip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub relative_path: String,
    pub extension: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionReport {
    pub source_name: String,
    pub input_kind: PackageInputKind,
    pub verdict: DetectionVerdict,
    pub signals: Vec<EngineFamily>,
    pub executable_count: usize,
    pub dlls: Vec<String>,
    pub files: Vec<FileEntry>,
    pub warnings: Vec<String>,
}

impl DetectionReport {
    pub fn minimal(
        source_name: String,
        input_kind: PackageInputKind,
        verdict: DetectionVerdict,
        signals: Vec<EngineFamily>,
    ) -> Self {
        Self {
            source_name,
            input_kind,
            verdict,
            signals,
            executable_count: 0,
            dlls: Vec::new(),
            files: Vec::new(),
            warnings: Vec::new(),
        }
    }
}
```

- [ ] **Step 3: Add the test file**

```rust
use iwm_detector::{DetectionReport, DetectionVerdict, EngineFamily, PackageInputKind};

#[test]
fn detection_report_serializes_expected_verdict_names() {
    let report = DetectionReport::minimal(
        "example".into(),
        PackageInputKind::Directory,
        DetectionVerdict::Gm8Likely,
        vec![EngineFamily::Gm8],
    );

    let json = serde_json::to_value(report).unwrap();
    assert_eq!(json["verdict"], "gm8-likely");
    assert_eq!(json["input_kind"], "directory");
}
```

- [ ] **Step 4: Run the test**

Run:

```bash
cargo test -p iwm-detector detection_report_serializes_expected_verdict_names -- --exact
```

Expected:

```text
PASS
```

- [ ] **Step 5: Commit**

```bash
git add crates/iwm-detector/src/models.rs crates/iwm-detector/src/lib.rs crates/iwm-detector/tests/detect_directory.rs
git commit -m "feat: add detector report models"
```

### Task 4: Build Package Inventory Support

**Files:**
- Create: `crates/iwm-detector/src/package.rs`
- Modify: `crates/iwm-detector/src/lib.rs`
- Test: `crates/iwm-detector/tests/detect_directory.rs`

- [ ] **Step 1: Write the failing test for directory inventory**

```rust
use std::fs;

#[test]
fn inventory_directory_collects_exes_dlls_and_files() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("game.exe"), b"MZfake").unwrap();
    fs::write(temp.path().join("bass.dll"), b"dll").unwrap();
    fs::write(temp.path().join("audio.ogg"), b"ogg").unwrap();

    let package = iwm_detector::package::load_package(temp.path()).unwrap();

    assert_eq!(package.input_kind, iwm_detector::PackageInputKind::Directory);
    assert_eq!(package.executables.len(), 1);
    assert_eq!(package.dlls, vec!["bass.dll"]);
    assert_eq!(package.files.len(), 3);
}
```

- [ ] **Step 2: Implement `package.rs`**

```rust
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

    match path.extension().and_then(|ext| ext.to_str()).map(|s| s.to_ascii_lowercase()) {
        Some(ext) if ext == "exe" => load_single_exe(path),
        Some(ext) if ext == "zip" => load_zip(path),
        _ => Err(format!("unsupported input path: {}", path.display())),
    }
}

fn load_directory(path: &Path) -> Result<LoadedPackage, String> {
    let mut executables = Vec::new();
    let mut dlls = Vec::new();
    let mut files = Vec::new();

    for entry in WalkDir::new(path).into_iter().filter_map(Result::ok).filter(|e| e.file_type().is_file()) {
        let full = entry.path().to_path_buf();
        let relative = full.strip_prefix(path).unwrap().to_string_lossy().replace('\\', "/");
        let metadata = fs::metadata(&full).map_err(|e| e.to_string())?;
        let extension = full.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();

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
        source_name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
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
        source_name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        input_kind: PackageInputKind::Exe,
        root_dir: path.parent().unwrap_or(Path::new(".")).to_path_buf(),
        executables: vec![path.to_path_buf()],
        dlls: Vec::new(),
        files: vec![FileEntry {
            relative_path: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
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
        package.source_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        package.input_kind = PackageInputKind::Zip;
        package._temp_dir = Some(temp);
        package
    })
}
```

- [ ] **Step 3: Expose the package module publicly**

```rust
pub mod detect;
pub mod models;
pub mod package;
pub mod signatures;

pub use detect::detect_input;
pub use models::{DetectionReport, DetectionVerdict, EngineFamily, PackageInputKind};
```

- [ ] **Step 4: Extend the test file**

```rust
use std::fs;

use iwm_detector::PackageInputKind;

#[test]
fn inventory_directory_collects_exes_dlls_and_files() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("game.exe"), b"MZfake").unwrap();
    fs::write(temp.path().join("bass.dll"), b"dll").unwrap();
    fs::write(temp.path().join("audio.ogg"), b"ogg").unwrap();

    let package = iwm_detector::package::load_package(temp.path()).unwrap();

    assert_eq!(package.input_kind, PackageInputKind::Directory);
    assert_eq!(package.executables.len(), 1);
    assert_eq!(package.dlls, vec!["bass.dll"]);
    assert_eq!(package.files.len(), 3);
}
```

- [ ] **Step 5: Run the targeted test**

Run:

```bash
cargo test -p iwm-detector inventory_directory_collects_exes_dlls_and_files -- --exact
```

Expected:

```text
PASS
```

- [ ] **Step 6: Commit**

```bash
git add crates/iwm-detector/src/package.rs crates/iwm-detector/src/lib.rs crates/iwm-detector/tests/detect_directory.rs
git commit -m "feat: add package inventory loading"
```

### Task 5: Add Signature Matching and Verdict Logic

**Files:**
- Create: `crates/iwm-detector/src/signatures.rs`
- Create: `crates/iwm-detector/src/detect.rs`
- Test: `crates/iwm-detector/tests/detect_directory.rs`
- Test: `crates/iwm-detector/tests/detect_zip.rs`

- [ ] **Step 1: Write the failing test for GM8 classification**

```rust
use std::fs;

#[test]
fn detect_directory_reports_gm8_likely() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("game.exe"),
        b"Game Maker Version 8 D3DX8.dll room_goto keyboard_check",
    )
    .unwrap();

    let report = iwm_detector::detect_input(temp.path()).unwrap();

    assert_eq!(report.verdict, iwm_detector::DetectionVerdict::Gm8Likely);
    assert_eq!(report.executable_count, 1);
}
```

- [ ] **Step 2: Implement `signatures.rs`**

```rust
use crate::models::EngineFamily;

pub fn known_signatures() -> &'static [(EngineFamily, &'static [&'static [u8]])] {
    &[
        (EngineFamily::Gm8, &[b"Game Maker", b"Version 8", b"D3DX8.dll", b"room_goto", b"keyboard_check"]),
        (EngineFamily::Gms, &[b"data.win", b"YoYo Games", b"audiogroup"]),
        (EngineFamily::Unity, &[b"UnityPlayer.dll", b"UnityEngine"]),
        (EngineFamily::RpgMaker, &[b"RPG_RT.exe", b"Game.rgss", b"www/js/plugins"]),
        (EngineFamily::Clickteam, &[b"Clickteam", b"Fusion"]),
        (EngineFamily::Godot, &[b"Godot Engine", b"godot_windows"]),
        (EngineFamily::Nwjs, &[b"nw.exe", b"nw_elf.dll"]),
    ]
}

pub fn match_signals(bytes: &[u8]) -> Vec<EngineFamily> {
    let mut matched = Vec::new();
    for (family, needles) in known_signatures() {
        if needles.iter().any(|needle| bytes.windows(needle.len()).any(|window| window == *needle)) {
            matched.push(*family);
        }
    }
    matched.sort_by_key(|family| *family as u8);
    matched.dedup();
    matched
}

pub fn match_inventory_signals(paths: &[String]) -> Vec<EngineFamily> {
    let haystack = paths
        .iter()
        .map(|path| path.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("\n");

    let mut matched = Vec::new();

    if haystack.contains("data.win") {
        matched.push(EngineFamily::Gms);
    }
    if haystack.contains("unityplayer.dll") {
        matched.push(EngineFamily::Unity);
    }
    if haystack.contains("rpg_rt.exe") || haystack.contains("game.rgss") || haystack.contains("www/js/plugins") {
        matched.push(EngineFamily::RpgMaker);
    }
    if haystack.contains("nw.exe") {
        matched.push(EngineFamily::Nwjs);
    }

    matched.sort_by_key(|family| *family as u8);
    matched.dedup();
    matched
}
```

- [ ] **Step 3: Implement `detect.rs`**

```rust
use crate::models::{DetectionReport, DetectionVerdict, EngineFamily};
use crate::package::load_package;
use crate::signatures::{match_inventory_signals, match_signals};
use std::fs;
use std::path::Path;

pub fn detect_input(path: &Path) -> Result<DetectionReport, String> {
    let package = load_package(path)?;
    let mut signals = Vec::new();
    let mut warnings = Vec::new();

    if package.executables.is_empty() {
        warnings.push("no executable found".into());
    }

    for exe in package.executables.iter().take(2) {
        let bytes = fs::read(exe).map_err(|e| e.to_string())?;
        signals.extend(match_signals(&bytes[..bytes.len().min(8_000_000)]));
    }

    let inventory_paths = package
        .files
        .iter()
        .map(|file| file.relative_path.clone())
        .collect::<Vec<_>>();
    signals.extend(match_inventory_signals(&inventory_paths));

    signals.sort_by_key(|family| *family as u8);
    signals.dedup();

    let verdict = classify(&signals, package.executables.len());

    Ok(DetectionReport {
        source_name: package.source_name,
        input_kind: package.input_kind,
        verdict,
        signals,
        executable_count: package.executables.len(),
        dlls: package.dlls,
        files: package.files,
        warnings,
    })
}

fn classify(signals: &[EngineFamily], executable_count: usize) -> DetectionVerdict {
    if executable_count == 0 {
        return DetectionVerdict::Blocked;
    }
    if signals.contains(&EngineFamily::Gms) {
        return DetectionVerdict::GmsLikely;
    }
    if signals.iter().any(|s| matches!(s, EngineFamily::Unity | EngineFamily::RpgMaker | EngineFamily::Clickteam | EngineFamily::Godot | EngineFamily::Nwjs)) {
        return DetectionVerdict::Blocked;
    }
    if signals.contains(&EngineFamily::Gm8) {
        return DetectionVerdict::Gm8Likely;
    }
    DetectionVerdict::Unknown
}
```

- [ ] **Step 4: Add the directory detection tests**

```rust
use std::fs;

use iwm_detector::{DetectionVerdict, PackageInputKind};

#[test]
fn detect_directory_reports_gm8_likely() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("game.exe"),
        b"Game Maker Version 8 D3DX8.dll room_goto keyboard_check",
    )
    .unwrap();

    let report = iwm_detector::detect_input(temp.path()).unwrap();

    assert_eq!(report.input_kind, PackageInputKind::Directory);
    assert_eq!(report.verdict, DetectionVerdict::Gm8Likely);
    assert_eq!(report.executable_count, 1);
}

#[test]
fn detect_directory_reports_blocked_for_unity() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("game.exe"), b"UnityPlayer.dll UnityEngine").unwrap();

    let report = iwm_detector::detect_input(temp.path()).unwrap();

    assert_eq!(report.verdict, DetectionVerdict::Blocked);
}
```

- [ ] **Step 5: Add the zip detection test**

```rust
use std::fs::File;
use std::io::Write;

#[test]
fn detect_zip_reports_gm8_likely() {
    let temp = tempfile::tempdir().unwrap();
    let zip_path = temp.path().join("sample.zip");
    let file = File::create(&zip_path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();

    zip.start_file("game.exe", options).unwrap();
    zip.write_all(b"Game Maker Version 8 D3DX8.dll").unwrap();
    zip.finish().unwrap();

    let report = iwm_detector::detect_input(&zip_path).unwrap();

    assert_eq!(report.verdict, iwm_detector::DetectionVerdict::Gm8Likely);
    assert_eq!(report.input_kind, iwm_detector::PackageInputKind::Zip);
}
```

- [ ] **Step 6: Run detector tests**

Run:

```bash
cargo test -p iwm-detector
```

Expected:

```text
PASS
```

- [ ] **Step 7: Commit**

```bash
git add crates/iwm-detector/src/signatures.rs crates/iwm-detector/src/detect.rs crates/iwm-detector/tests
git commit -m "feat: add detector heuristics and verdict logic"
```

### Task 6: Expose a Useful CLI Interface

**Files:**
- Modify: `crates/iwm-cli/src/main.rs`

- [ ] **Step 1: Write the failing CLI behavior check**

Run:

```bash
cargo run -p iwm-cli -- detect --input C:\\does-not-exist
```

Expected:

```text
thread 'main' panicked
```

- [ ] **Step 2: Implement CLI output and exit behavior**

```rust
use clap::{Parser, Subcommand};
use iwm_detector::detect_input;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "iwm-cli")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Detect { #[arg(long)] input: PathBuf },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Detect { input } => match detect_input(&input) {
            Ok(report) => {
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
            }
            Err(err) => {
                eprintln!("{err}");
                std::process::exit(1);
            }
        },
    }
}
```

- [ ] **Step 3: Run the CLI against a real local sample**

Run:

```bash
cargo run -p iwm-cli -- detect --input "C:\\Users\\59164\\Desktop\\iwanna examples\\gm8-core\\IWBT_Dife"
```

Expected:

```text
{
  "source_name": "...",
  "verdict": "gm8-likely"
}
```

- [ ] **Step 4: Run the CLI against a non-target sample**

Run:

```bash
cargo run -p iwm-cli -- detect --input "C:\\Users\\59164\\Desktop\\iwanna examples\\non-target\\I Wanna Be The GBC"
```

Expected:

```text
{
  "verdict": "gms-likely"
}
```

- [ ] **Step 5: Commit**

```bash
git add crates/iwm-cli/src/main.rs
git commit -m "feat: add detector cli output"
```

### Task 7: Update Local Sample Workflow Notes

**Files:**
- Modify: `docs/notes/sample-corpus.md`
- Modify: `README.md`

- [ ] **Step 1: Write the failing documentation check**

Run:

```bash
rg "gm8-core|needs-manual-check|non-target" README.md docs
```

Expected:

```text
existing matches from the current docs set
```

- [ ] **Step 2: Update `docs/notes/sample-corpus.md`**

```md
# Sample Corpus Notes

Current project-local sample root:

- `C:\Users\59164\work\playground\iwanna-gm8-web-engine\samples\local\iwanna-examples`

Current local categories:

- `gm8-core`
- `gm8-extended`
- `needs-manual-check`
- `non-target`

Detector development order:

1. Run all `gm8-core` samples and confirm `gm8-likely` or `unknown`
2. Run all `non-target` samples and confirm they are not classified as `gm8-likely`
3. Review `needs-manual-check` output and record missing heuristics
4. Defer DLL-heavy edge cases until detector stability is proven

Practical rule:

- future scripts, plans, and local smoke tests should prefer this project-local sample path
- do not assume the old desktop path exists anymore
```

- [ ] **Step 3: Extend the root README**

```md
# iwanna-gm8-web-engine

Browser-playable IWanna MVP targeting legacy GM8-style fangames.

## Current Phase

Phase 1 builds the repository skeleton and a detector that classifies game packages before any parsing or runtime work begins.

## Sample Workflow

This repo does not commit copyrighted game binaries.

Use the local sample corpus described in:

- `docs/notes/sample-corpus.md`

## Local Commands

```bash
cargo test
cargo run -p iwm-cli -- detect --input C:\\path\\to\\game
```
```

- [ ] **Step 4: Run the documentation grep**

Run:

```bash
rg "gm8-core|needs-manual-check|non-target" README.md docs
```

Expected:

```text
README.md
docs/notes/sample-corpus.md
```

- [ ] **Step 5: Commit**

```bash
git add README.md docs/notes/sample-corpus.md
git commit -m "docs: add sample corpus workflow"
```

### Task 8: Final Verification for Phase 1

**Files:**
- Modify: none
- Test: workspace-wide verification only

- [ ] **Step 1: Run formatting**

Run:

```bash
cargo fmt --all
```

Expected:

```text
no output
```

- [ ] **Step 2: Run all tests**

Run:

```bash
cargo test
```

Expected:

```text
test result: ok
```

- [ ] **Step 3: Run both real-sample smoke checks again**

Run:

```bash
cargo run -p iwm-cli -- detect --input ".\\samples\\local\\iwanna-examples\\gm8-core\\IWBT_Dife"
cargo run -p iwm-cli -- detect --input ".\\samples\\local\\iwanna-examples\\non-target\\I Wanna Be The GBC"
```

Expected:

```text
first output contains "gm8-likely"
second output contains "gms-likely" or "blocked"
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "test: verify detector foundation"
```

## Self-Review

Spec coverage for this plan:

- upload-package classification: covered
- engine heuristics: covered
- sample corpus workflow: covered
- normalized package build: not covered in this plan, deferred to the next plan
- browser runtime: not covered in this plan, deferred to a later plan
- compatibility metrics beyond CLI output: partially covered through structured JSON, fuller observability deferred

Placeholder scan:

- no `TODO`
- no `TBD`
- no undefined “handle appropriately” style steps

Type consistency notes:

- canonical verdict strings are `gm8-likely`, `gms-likely`, `unknown`, `blocked`
- detector entrypoint is `detect_input`
- package loader entrypoint is `package::load_package`

Next planned document after this phase:

- parser and normalized package builder plan
