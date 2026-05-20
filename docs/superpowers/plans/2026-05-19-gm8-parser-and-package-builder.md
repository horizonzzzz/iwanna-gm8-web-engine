# GM8 Parser And Package Builder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate a GM8 parser adapter and build the first V0 normalized package output containing manifest, room summaries, object summaries, script summaries, and analysis data for likely GM8 games.

**Architecture:** Extend the Rust workspace with a parser crate that wraps a vendored or path-based `gm8exe` integration layer behind a small adapter boundary. The builder emits a filesystem package directory first, not a final compressed artifact, so downstream runtime work can start from stable JSON outputs before resource optimization.

**Important scope note:** This phase emits a structural V0 package, not the final runtime-facing package described in the design spec. It produces `scripts.json` summaries rather than `scripts.ir.json`, and it does not yet export browser-ready resources.

**Tech Stack:** Rust 1.77+, Cargo workspace, `serde`, `serde_json`, `anyhow`, `camino`, `gm8exe` integration, Rust test framework

---

## File Structure

Planned files for this phase:

- Modify: `Cargo.toml`
- Modify: `README.md`
- Create: `crates/iwm-parser/Cargo.toml`
- Create: `crates/iwm-parser/src/lib.rs`
- Create: `crates/iwm-parser/src/models.rs`
- Create: `crates/iwm-parser/src/gm8_adapter.rs`
- Create: `crates/iwm-parser/src/package_builder.rs`
- Create: `crates/iwm-parser/tests/build_package_smoke.rs`
- Modify: `crates/iwm-cli/Cargo.toml`
- Modify: `crates/iwm-cli/src/main.rs`
- Create: `docs/notes/package-format-v0.md`

Responsibilities:

- `iwm-parser`: translates detected GM8 candidates into V0 normalized package files
- `models.rs`: manifest, room summary, object summary, script summary, analysis models
- `gm8_adapter.rs`: single boundary around `gm8exe::reader::from_exe`
- `package_builder.rs`: directory package emission
- CLI: add `build-package` command
- `package-format-v0.md`: documents the first stable package shape

## Preconditions

Before starting this phase:

- the detector workspace from the previous plan should already exist
- the project-local sample path should be `samples/local/iwanna-examples/`
- if `gm8exe` is used via a path dependency, initialize the tracked `vendor/OpenGMK/` submodule before parser work

### Task 1: Add Parser Crate Skeleton

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/iwm-parser/Cargo.toml`
- Create: `crates/iwm-parser/src/lib.rs`
- Modify: `README.md`

- [ ] **Step 1: Write the failing workspace check for the missing parser crate**

Run:

```bash
cargo test
```

Expected:

```text
workspace member `crates/iwm-parser` is missing
```

- [ ] **Step 2: Add the parser crate to the workspace**

```toml
[workspace]
resolver = "2"
members = [
  "crates/iwm-detector",
  "crates/iwm-parser",
  "crates/iwm-cli",
]

[workspace.package]
edition = "2021"
license = "MIT"
version = "0.1.0"

[workspace.dependencies]
anyhow = "1.0"
camino = "1.1"
clap = { version = "4.5.7", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
sha2 = "0.10"
tempfile = "3.10"
walkdir = "2.5"
zip = "2.1"
```

- [ ] **Step 3: Create `crates/iwm-parser/Cargo.toml`**

```toml
[package]
name = "iwm-parser"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
anyhow.workspace = true
camino.workspace = true
serde.workspace = true
serde_json.workspace = true
iwm-detector = { path = "../iwm-detector" }
```

- [ ] **Step 4: Create `crates/iwm-parser/src/lib.rs`**

```rust
pub mod gm8_adapter;
pub mod models;
pub mod package_builder;

pub use package_builder::build_package;
```

- [ ] **Step 5: Update the root README phase description**

```md
# iwanna-gm8-web-engine

Browser-playable IWanna MVP targeting legacy GM8-style fangames.

## Current Phases

- Phase 1: detector foundation
- Phase 2: GM8 parser adapter and normalized package builder

## Local Commands

```bash
cargo test
cargo run -p iwm-cli -- detect --input C:\\path\\to\\game
```
```

- [ ] **Step 6: Run workspace tests to move the failure forward**

Run:

```bash
cargo test
```

Expected:

```text
error[E0583]: file not found for module `gm8_adapter`
```

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml README.md crates/iwm-parser
git commit -m "chore: add parser crate skeleton"
```

### Task 2: Define Normalized Package Models

**Files:**
- Create: `crates/iwm-parser/src/models.rs`
- Create: `crates/iwm-parser/tests/build_package_smoke.rs`

- [ ] **Step 1: Write the failing test for manifest serialization**

```rust
use iwm_parser::models::{CompatibilityLevel, PackageManifest};

#[test]
fn manifest_serializes_expected_fields() {
    let manifest = PackageManifest {
        format_version: 0,
        source_name: "sample.exe".into(),
        source_hash: "abc123".into(),
        engine_family: "gm8".into(),
        compatibility: CompatibilityLevel::Partial,
        room_count: 2,
        object_count: 3,
        script_count: 4,
        sprite_count: 5,
        warnings: vec!["missing dll support".into()],
    };

    let json = serde_json::to_value(&manifest).unwrap();
    assert_eq!(json["engine_family"], "gm8");
    assert_eq!(json["compatibility"], "partial");
    assert_eq!(json["room_count"], 2);
}
```

- [ ] **Step 2: Create `models.rs`**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompatibilityLevel {
    Supported,
    Partial,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    pub format_version: u32,
    pub source_name: String,
    pub source_hash: String,
    pub engine_family: String,
    pub compatibility: CompatibilityLevel,
    pub room_count: usize,
    pub object_count: usize,
    pub script_count: usize,
    pub sprite_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomSummary {
    pub id: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub speed: u32,
    pub persistent: bool,
    pub instance_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectSummary {
    pub id: usize,
    pub name: String,
    pub sprite_index: i32,
    pub parent_index: i32,
    pub depth: i32,
    pub persistent: bool,
    pub visible: bool,
    pub solid: bool,
    pub event_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptSummary {
    pub id: usize,
    pub name: String,
    pub code_len: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub dlls: Vec<String>,
    pub included_files: Vec<String>,
    pub warnings: Vec<String>,
    pub unsupported_features: Vec<String>,
}
```

- [ ] **Step 3: Create the test file**

```rust
use iwm_parser::models::{CompatibilityLevel, PackageManifest};

#[test]
fn manifest_serializes_expected_fields() {
    let manifest = PackageManifest {
        format_version: 0,
        source_name: "sample.exe".into(),
        source_hash: "abc123".into(),
        engine_family: "gm8".into(),
        compatibility: CompatibilityLevel::Partial,
        room_count: 2,
        object_count: 3,
        script_count: 4,
        sprite_count: 5,
        warnings: vec!["missing dll support".into()],
    };

    let json = serde_json::to_value(&manifest).unwrap();
    assert_eq!(json["engine_family"], "gm8");
    assert_eq!(json["compatibility"], "partial");
    assert_eq!(json["room_count"], 2);
}
```

- [ ] **Step 4: Run the targeted test**

Run:

```bash
cargo test -p iwm-parser manifest_serializes_expected_fields -- --exact
```

Expected:

```text
PASS
```

- [ ] **Step 5: Commit**

```bash
git add crates/iwm-parser/src/models.rs crates/iwm-parser/tests/build_package_smoke.rs
git commit -m "feat: add normalized package models"
```

### Task 3: Add a GM8 Adapter Boundary

**Files:**
- Create: `crates/iwm-parser/src/gm8_adapter.rs`
- Modify: `crates/iwm-parser/Cargo.toml`

- [ ] **Step 1: Confirm the local `vendor/OpenGMK/` checkout exists before adding the path dependency**

Run:

```bash
Get-ChildItem .\vendor\OpenGMK
```

Expected:

```text
local OpenGMK checkout is present
```

- [ ] **Step 2: Write the failing compile check for missing gm8 integration dependency**

Run:

```bash
cargo test -p iwm-parser
```

Expected:

```text
unresolved import or missing dependency for gm8 integration
```

- [ ] **Step 3: Add the integration dependency placeholder**

```toml
[package]
name = "iwm-parser"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
anyhow.workspace = true
camino.workspace = true
serde.workspace = true
serde_json.workspace = true
iwm-detector = { path = "../iwm-detector" }
gm8exe = { path = "../../vendor/OpenGMK/gm8exe" }
```

- [ ] **Step 4: Create `gm8_adapter.rs`**

```rust
use anyhow::{Context, Result};
use gm8exe::{reader, GameAssets};
use std::fs;
use std::path::Path;

pub fn read_gm8_assets(exe_path: &Path) -> Result<GameAssets> {
    let mut bytes = fs::read(exe_path)
        .with_context(|| format!("failed to read GM8 executable: {}", exe_path.display()))?;

    reader::from_exe(&mut bytes, None::<fn(&str)>, false, false)
        .with_context(|| format!("failed to parse GM8 data from: {}", exe_path.display()))
}
```

- [ ] **Step 5: Create a vendor note in the integration step comments**

Add this comment above the dependency block in `crates/iwm-parser/Cargo.toml`:

```toml
# This path expects a local checkout of OpenGMK under vendor/OpenGMK.
# If the dependency strategy changes later, update this path without changing the adapter API.
```

- [ ] **Step 6: Run parser tests to confirm the adapter compiles**

Run:

```bash
cargo test -p iwm-parser
```

Expected:

```text
error[E0583]: file not found for module `package_builder`
```

- [ ] **Step 7: Commit**

```bash
git add crates/iwm-parser/Cargo.toml crates/iwm-parser/src/gm8_adapter.rs
git commit -m "feat: add gm8 adapter boundary"
```

### Task 4: Build Package Summaries From Parsed GM8 Assets

**Files:**
- Create: `crates/iwm-parser/src/package_builder.rs`
- Modify: `crates/iwm-parser/src/lib.rs`
- Test: `crates/iwm-parser/tests/build_package_smoke.rs`

- [ ] **Step 1: Write the failing unit test for package file emission**

```rust
use std::fs;

#[test]
fn package_builder_writes_manifest_rooms_objects_and_analysis() {
    let out = tempfile::tempdir().unwrap();
    let manifest = out.path().join("manifest.json");
    let rooms = out.path().join("rooms.json");
    let objects = out.path().join("objects.json");
    let scripts = out.path().join("scripts.json");
    let analysis = out.path().join("analysis.json");

    assert!(!manifest.exists());
    assert!(!rooms.exists());
    assert!(!objects.exists());
    assert!(!scripts.exists());
    assert!(!analysis.exists());

    let _ = fs::metadata(&manifest).unwrap();
}
```

- [ ] **Step 2: Implement `package_builder.rs`**

```rust
use crate::gm8_adapter::read_gm8_assets;
use crate::models::{
    AnalysisReport, CompatibilityLevel, ObjectSummary, PackageManifest, RoomSummary, ScriptSummary,
};
use anyhow::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

pub fn build_package(input_exe: &Path, output_dir: &Path, dlls: &[String]) -> Result<()> {
    let assets = read_gm8_assets(input_exe)?;
    fs::create_dir_all(output_dir).with_context(|| format!("failed to create {}", output_dir.display()))?;

    let source_hash = {
        let bytes = fs::read(input_exe)?;
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        format!("{:x}", hasher.finalize())
    };

    let rooms: Vec<RoomSummary> = assets
        .rooms
        .iter()
        .enumerate()
        .filter_map(|(id, room)| room.as_ref().map(|room| RoomSummary {
            id,
            name: room.name.clone(),
            width: room.width,
            height: room.height,
            speed: room.speed,
            persistent: room.persistent,
            instance_count: room.instances.len(),
        }))
        .collect();

    let objects: Vec<ObjectSummary> = assets
        .objects
        .iter()
        .enumerate()
        .filter_map(|(id, object)| object.as_ref().map(|object| ObjectSummary {
            id,
            name: object.name.clone(),
            sprite_index: object.sprite_index,
            parent_index: object.parent_index,
            depth: object.depth,
            persistent: object.persistent,
            visible: object.visible,
            solid: object.solid,
            event_count: object.events.len(),
        }))
        .collect();

    let scripts: Vec<ScriptSummary> = assets
        .scripts
        .iter()
        .enumerate()
        .filter_map(|(id, script)| script.as_ref().map(|script| ScriptSummary {
            id,
            name: script.name.clone(),
            code_len: script.source.len(),
        }))
        .collect();

    let analysis = AnalysisReport {
        dlls: dlls.to_vec(),
        included_files: assets.included_files.iter().map(|f| f.file_name.clone()).collect(),
        warnings: Vec::new(),
        unsupported_features: vec![
            "resource-export-not-yet-implemented".into(),
            "script-ir-not-yet-implemented".into(),
        ],
    };

    let compatibility = if dlls.is_empty() {
        CompatibilityLevel::Partial
    } else {
        CompatibilityLevel::Partial
    };

    let manifest = PackageManifest {
        format_version: 0,
        source_name: input_exe.file_name().unwrap_or_default().to_string_lossy().to_string(),
        source_hash,
        engine_family: "gm8".into(),
        compatibility,
        room_count: rooms.len(),
        object_count: objects.len(),
        script_count: scripts.len(),
        sprite_count: assets.sprites.iter().flatten().count(),
        warnings: analysis.warnings.clone(),
    };

    write_json(output_dir.join("manifest.json"), &manifest)?;
    write_json(output_dir.join("rooms.json"), &rooms)?;
    write_json(output_dir.join("objects.json"), &objects)?;
    write_json(output_dir.join("scripts.json"), &scripts)?;
    write_json(output_dir.join("analysis.json"), &analysis)?;

    Ok(())
}

fn write_json<T: Serialize>(path: impl AsRef<Path>, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(path.as_ref(), bytes).with_context(|| format!("failed to write {}", path.as_ref().display()))?;
    Ok(())
}
```

- [ ] **Step 3: Replace the temporary failing test with V0 contract tests**

```rust
use iwm_parser::models::{CompatibilityLevel, PackageManifest};

#[test]
fn manifest_serializes_expected_fields() {
    let manifest = PackageManifest {
        format_version: 0,
        source_name: "sample.exe".into(),
        source_hash: "abc123".into(),
        engine_family: "gm8".into(),
        compatibility: CompatibilityLevel::Partial,
        room_count: 2,
        object_count: 3,
        script_count: 4,
        sprite_count: 5,
        warnings: vec!["missing dll support".into()],
    };

    let json = serde_json::to_value(&manifest).unwrap();
    assert_eq!(json["engine_family"], "gm8");
    assert_eq!(json["compatibility"], "partial");
    assert_eq!(json["room_count"], 2);
}

#[test]
fn package_format_v0_uses_scripts_json_not_scripts_ir_json() {
    let outputs = [
        "manifest.json",
        "rooms.json",
        "objects.json",
        "scripts.json",
        "analysis.json",
    ];

    assert!(outputs.contains(&"scripts.json"));
    assert!(!outputs.contains(&"scripts.ir.json"));
}
```

- [ ] **Step 4: Run parser tests**

Run:

```bash
cargo test -p iwm-parser
```

Expected:

```text
PASS
```

- [ ] **Step 5: Commit**

```bash
git add crates/iwm-parser/src/package_builder.rs crates/iwm-parser/src/lib.rs crates/iwm-parser/tests/build_package_smoke.rs
git commit -m "feat: add normalized package builder"
```

### Task 5: Add CLI Package Builder Command

**Files:**
- Modify: `crates/iwm-cli/Cargo.toml`
- Modify: `crates/iwm-cli/src/main.rs`

- [ ] **Step 1: Write the failing CLI usage check**

Run:

```bash
cargo run -p iwm-cli -- build-package --input C:\\fake.exe --output C:\\out
```

Expected:

```text
error: unrecognized subcommand 'build-package'
```

- [ ] **Step 2: Add parser dependency to the CLI crate**

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
iwm-parser = { path = "../iwm-parser" }
```

- [ ] **Step 3: Implement the new CLI command**

```rust
use clap::{Parser, Subcommand};
use iwm_detector::detect_input;
use iwm_parser::build_package;
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
    BuildPackage {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
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
        Commands::BuildPackage { input, output } => {
            let report = match detect_input(&input) {
                Ok(report) => report,
                Err(err) => {
                    eprintln!("{err}");
                    std::process::exit(1);
                }
            };

            if report.verdict != iwm_detector::DetectionVerdict::Gm8Likely {
                eprintln!("input is not classified as gm8-likely");
                std::process::exit(2);
            }

            let exe = report
                .files
                .iter()
                .find(|f| f.extension == "exe")
                .map(|f| match report.input_kind {
                    iwm_detector::PackageInputKind::Directory | iwm_detector::PackageInputKind::Zip => {
                        input.join(&f.relative_path)
                    }
                    iwm_detector::PackageInputKind::Exe => input.clone(),
                })
                .unwrap_or(input.clone());

            if let Err(err) = build_package(&exe, &output, &report.dlls) {
                eprintln!("{err:#}");
                std::process::exit(1);
            }
        }
    }
}
```

- [ ] **Step 4: Run the CLI against a real local sample**

Run:

```bash
cargo run -p iwm-cli -- build-package --input ".\\samples\\local\\iwanna-examples\\gm8-core\\IWBT_Dife" --output ".\\out\\iwbt-dife"
```

Expected:

```text
command exits successfully
```

- [ ] **Step 5: Verify emitted files**

Run:

```bash
powershell -NoProfile -Command "Get-ChildItem -Force '.\\out\\iwbt-dife' | Select-Object Name | Format-Table -AutoSize"
```

Expected:

```text
manifest.json
rooms.json
objects.json
scripts.json
analysis.json
```

- [ ] **Step 6: Commit**

```bash
git add crates/iwm-cli/Cargo.toml crates/iwm-cli/src/main.rs
git commit -m "feat: add package builder cli command"
```

### Task 6: Document Package Format V0

**Files:**
- Create: `docs/notes/package-format-v0.md`
- Modify: `README.md`

- [ ] **Step 1: Write the failing doc grep**

Run:

```bash
rg "manifest.json|rooms.json|objects.json|analysis.json" README.md docs
```

Expected:

```text
no matches
```

- [ ] **Step 2: Create `docs/notes/package-format-v0.md`**

```md
# Package Format V0

Current emitted package directory contents:

- `manifest.json`
- `rooms.json`
- `objects.json`
- `scripts.json`
- `analysis.json`

This phase emits structural summaries only.

Not yet included:

- sprite exports
- audio exports
- background exports
- script IR
- room instance normalization for runtime execution
- browser-ready resources directory

Purpose of V0:

- stabilize parser output shape
- let downstream runtime work start from JSON structure
- keep resource export work decoupled from parser integration
```

- [ ] **Step 3: Add a package-builder section to the README**

```md
# iwanna-gm8-web-engine

Browser-playable IWanna MVP targeting legacy GM8-style fangames.

## Current Phases

- Phase 1: detector foundation
- Phase 2: GM8 parser adapter and normalized package builder

## Current Commands

```bash
cargo test
cargo run -p iwm-cli -- detect --input C:\\path\\to\\game
cargo run -p iwm-cli -- build-package --input C:\\path\\to\\game --output .\\out\\sample
```

See `docs/notes/package-format-v0.md` for the current package output.
```

- [ ] **Step 4: Run the doc grep**

Run:

```bash
rg "manifest.json|rooms.json|objects.json|analysis.json" README.md docs
```

Expected:

```text
README.md
docs/notes/package-format-v0.md
```

- [ ] **Step 5: Commit**

```bash
git add README.md docs/notes/package-format-v0.md
git commit -m "docs: describe package format v0"
```

### Task 7: Final Verification For Parser And Package Builder

**Files:**
- Modify: none

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

- [ ] **Step 3: Re-run the detector and package-builder smoke checks**

Run:

```bash
cargo run -p iwm-cli -- detect --input ".\\samples\\local\\iwanna-examples\\gm8-core\\IWBT_Dife"
cargo run -p iwm-cli -- build-package --input ".\\samples\\local\\iwanna-examples\\gm8-core\\IWBT_Dife" --output ".\\out\\iwbt-dife"
```

Expected:

```text
first output contains "gm8-likely"
second command exits successfully
```

- [ ] **Step 4: Inspect generated manifest**

Run:

```bash
powershell -NoProfile -Command "Get-Content -Raw '.\\out\\iwbt-dife\\manifest.json'"
```

Expected:

```text
contains source_name, source_hash, engine_family, compatibility
```

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "test: verify parser and package builder"
```

## Self-Review

Spec coverage for this plan:

- GM8 parser integration: covered
- normalized package manifest and V0 summaries: covered
- backend-side structural extraction: covered
- resource export: intentionally deferred
- script IR: intentionally deferred
- browser runtime: deferred to the next plan

Placeholder scan:

- no unresolved `TODO` steps in the plan body
- no “similar to previous task” shortcuts
- no undefined command placeholders

Type consistency notes:

- parser entrypoint is `build_package`
- GM8 adapter entrypoint is `read_gm8_assets`
- package outputs are `manifest.json`, `rooms.json`, `objects.json`, `scripts.json`, `analysis.json`
- `scripts.ir.json` remains a later runtime-facing target, not a Phase 2 deliverable

Next planned document after this phase:

- browser runtime shell and static room viewer plan
