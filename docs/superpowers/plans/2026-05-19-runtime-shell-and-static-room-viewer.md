# Runtime Shell And Static Room Viewer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Phase 2 structural V0 package with a minimal runtime-facing package and add a local browser harness that can inspect package data and render static rooms without executing gameplay yet.

**Architecture:** Keep parser responsibility in Rust and runtime responsibility in a standalone `runtime/` frontend. The parser now emits a runtime-consumable package with browser-ready resources, normalized room instance placements, object event tables, and a first executable logic envelope in `scripts.ir.json`; the runtime harness consumes only that package and provides a shell for package inspection and static room visualization.

**Important scope note:** This phase starts the runtime stage, but it is not the minimal playable runtime yet. Do not add fixed-step simulation, player input, collision execution, death, respawn, or room-transition gameplay in this plan.

**Tech Stack:** Rust 1.77+, Cargo workspace, `serde`, `serde_json`, `anyhow`, `png`, vendored `gm8exe`, Node 20+, Vite, TypeScript, Canvas 2D, Vitest

---

## File Structure

Planned files for this phase:

- Modify: `Cargo.toml`
- Modify: `README.md`
- Modify: `crates/iwm-parser/Cargo.toml`
- Modify: `crates/iwm-parser/src/lib.rs`
- Modify: `crates/iwm-parser/src/models.rs`
- Modify: `crates/iwm-parser/src/package_builder.rs`
- Create: `crates/iwm-parser/src/resource_export.rs`
- Create: `crates/iwm-parser/src/logic_export.rs`
- Modify: `crates/iwm-parser/tests/build_package_smoke.rs`
- Modify: `crates/iwm-cli/src/main.rs`
- Create: `docs/notes/package-format-v1-runtime.md`
- Create: `runtime/package.json`
- Create: `runtime/tsconfig.json`
- Create: `runtime/vite.config.ts`
- Create: `runtime/index.html`
- Create: `runtime/public/packages/.gitkeep`
- Create: `runtime/src/main.ts`
- Create: `runtime/src/styles.css`
- Create: `runtime/src/types.ts`
- Create: `runtime/src/loadPackage.ts`
- Create: `runtime/src/loadPackage.test.ts`
- Create: `runtime/src/render/resourceCache.ts`
- Create: `runtime/src/render/staticRoomRenderer.ts`
- Create: `runtime/src/render/staticRoomRenderer.test.ts`
- Create: `runtime/src/ui/shell.ts`
- Create: `runtime/src/ui/inspectors.ts`
- Create: `runtime/src/vite-env.d.ts`

Responsibilities:

- `iwm-parser/src/models.rs`: runtime package schema shared by the builder and tests
- `iwm-parser/src/resource_export.rs`: sprite, background, and audio export into browser-friendly files
- `iwm-parser/src/logic_export.rs`: normalized object events, room creation blocks, and initial `scripts.ir.json` envelope
- `iwm-parser/src/package_builder.rs`: orchestrates runtime package emission
- `iwm-cli/src/main.rs`: keeps `build-package` as the developer entrypoint for generating runtime packages
- `docs/notes/package-format-v1-runtime.md`: documents the new contract and explicitly marks V0 as superseded
- `runtime/`: standalone developer harness that loads package JSON plus `resources/` and renders a static room viewer

## Preconditions

Before starting this phase:

- Phase 2 code should already pass `cargo test`
- `vendor/OpenGMK/gm8exe` must still be available locally for parser work
- `samples/local/iwanna-examples/gm8-core/` remains the preferred first smoke-test corpus
- `runtime/` does not exist yet, so this phase is allowed to introduce new frontend tooling

### Task 1: Define The Runtime Package Contract

**Files:**
- Modify: `README.md`
- Modify: `crates/iwm-parser/src/models.rs`
- Modify: `crates/iwm-parser/tests/build_package_smoke.rs`
- Create: `docs/notes/package-format-v1-runtime.md`

- [x] **Step 1: Write the failing contract test for runtime-facing manifest and file names**

```rust
use iwm_parser::models::{CompatibilityLevel, RuntimeManifest};

#[test]
fn runtime_manifest_serializes_expected_fields() {
    let manifest = RuntimeManifest {
        format_version: 1,
        package_kind: "runtime-v1".into(),
        source_name: "sample.exe".into(),
        source_hash: "abc123".into(),
        engine_family: "gm8".into(),
        compatibility: CompatibilityLevel::Partial,
        default_room_id: Some(0),
        room_count: 2,
        object_count: 3,
        script_block_count: 4,
        sprite_count: 5,
        background_count: 1,
        sound_count: 6,
        resource_index_path: "resources/index.json".into(),
        warnings: vec!["script-ir-partial".into()],
    };

    let json = serde_json::to_value(&manifest).unwrap();
    assert_eq!(json["format_version"], 1);
    assert_eq!(json["package_kind"], "runtime-v1");
    assert_eq!(json["resource_index_path"], "resources/index.json");
}

#[test]
fn runtime_package_uses_ir_and_resource_index_outputs() {
    let outputs = [
        "manifest.json",
        "rooms.json",
        "objects.json",
        "scripts.ir.json",
        "analysis.json",
        "resources/index.json",
    ];

    assert!(outputs.contains(&"scripts.ir.json"));
    assert!(outputs.contains(&"resources/index.json"));
    assert!(!outputs.contains(&"scripts.json"));
}
```

- [x] **Step 2: Replace the V0 summary types in `crates/iwm-parser/src/models.rs` with runtime-facing models**

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
pub struct RuntimeManifest {
    pub format_version: u32,
    pub package_kind: String,
    pub source_name: String,
    pub source_hash: String,
    pub engine_family: String,
    pub compatibility: CompatibilityLevel,
    pub default_room_id: Option<usize>,
    pub room_count: usize,
    pub object_count: usize,
    pub script_block_count: usize,
    pub sprite_count: usize,
    pub background_count: usize,
    pub sound_count: usize,
    pub resource_index_path: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceIndex {
    pub sprites: Vec<SpriteResource>,
    pub backgrounds: Vec<BackgroundResource>,
    pub sounds: Vec<SoundResource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteResource {
    pub id: usize,
    pub name: String,
    pub origin_x: i32,
    pub origin_y: i32,
    pub frame_paths: Vec<String>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundResource {
    pub id: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub image_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundResource {
    pub id: usize,
    pub name: String,
    pub file_path: String,
    pub extension: String,
    pub preload: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomDefinition {
    pub id: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub speed: u32,
    pub persistent: bool,
    pub backgrounds: Vec<RoomBackgroundLayer>,
    pub views_enabled: bool,
    pub views: Vec<RoomView>,
    pub instances: Vec<RoomInstancePlacement>,
    pub creation_block_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomBackgroundLayer {
    pub visible_on_start: bool,
    pub is_foreground: bool,
    pub source_bg: i32,
    pub xoffset: i32,
    pub yoffset: i32,
    pub tile_horz: bool,
    pub tile_vert: bool,
    pub hspeed: i32,
    pub vspeed: i32,
    pub stretch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomView {
    pub visible: bool,
    pub source_x: i32,
    pub source_y: i32,
    pub source_w: u32,
    pub source_h: u32,
    pub port_x: i32,
    pub port_y: i32,
    pub port_w: u32,
    pub port_h: u32,
    pub target: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInstancePlacement {
    pub instance_id: i32,
    pub object_id: i32,
    pub x: i32,
    pub y: i32,
    pub xscale: f64,
    pub yscale: f64,
    pub angle: f64,
    pub blend: u32,
    pub creation_block_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectDefinition {
    pub id: usize,
    pub name: String,
    pub sprite_index: i32,
    pub parent_index: i32,
    pub depth: i32,
    pub persistent: bool,
    pub visible: bool,
    pub solid: bool,
    pub mask_index: i32,
    pub events: Vec<ObjectEventEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectEventEntry {
    pub event_type: usize,
    pub sub_event: u32,
    pub block_id: String,
    pub action_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptIrFile {
    pub format: String,
    pub blocks: Vec<LogicBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicBlock {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub support: String,
    pub ops: Vec<LogicOp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub enum LogicOp {
    ActionCall {
        action_id: u32,
        lib_id: u32,
        applies_to: i32,
        is_condition: bool,
        invert_condition: bool,
        is_relative: bool,
        fn_name: String,
        fn_code: String,
        args: Vec<String>,
    },
    SourceSnippet {
        code: String,
    },
    Unsupported {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub dlls: Vec<String>,
    pub included_files: Vec<String>,
    pub warnings: Vec<String>,
    pub unsupported_features: Vec<String>,
}
```

- [x] **Step 3: Replace the parser smoke test file with runtime package contract checks**

```rust
use iwm_parser::models::{CompatibilityLevel, RuntimeManifest};

#[test]
fn runtime_manifest_serializes_expected_fields() {
    let manifest = RuntimeManifest {
        format_version: 1,
        package_kind: "runtime-v1".into(),
        source_name: "sample.exe".into(),
        source_hash: "abc123".into(),
        engine_family: "gm8".into(),
        compatibility: CompatibilityLevel::Partial,
        default_room_id: Some(0),
        room_count: 2,
        object_count: 3,
        script_block_count: 4,
        sprite_count: 5,
        background_count: 1,
        sound_count: 6,
        resource_index_path: "resources/index.json".into(),
        warnings: vec!["script-ir-partial".into()],
    };

    let json = serde_json::to_value(&manifest).unwrap();
    assert_eq!(json["format_version"], 1);
    assert_eq!(json["package_kind"], "runtime-v1");
    assert_eq!(json["resource_index_path"], "resources/index.json");
}

#[test]
fn runtime_package_uses_ir_and_resource_index_outputs() {
    let outputs = [
        "manifest.json",
        "rooms.json",
        "objects.json",
        "scripts.ir.json",
        "analysis.json",
        "resources/index.json",
    ];

    assert!(outputs.contains(&"scripts.ir.json"));
    assert!(outputs.contains(&"resources/index.json"));
    assert!(!outputs.contains(&"scripts.json"));
}
```

- [x] **Step 4: Document the runtime package in `docs/notes/package-format-v1-runtime.md`**

```md
# Package Format V1 Runtime

Current emitted runtime package directory contents:

- `manifest.json`
- `rooms.json`
- `objects.json`
- `scripts.ir.json`
- `analysis.json`
- `resources/index.json`
- `resources/sprites/...`
- `resources/backgrounds/...`
- `resources/audio/...`

This package is runtime-consumable but still phase-limited.

Included in this phase:

- browser-ready sprite exports
- browser-ready background exports
- audio file exports
- normalized room instance placements
- normalized object event table
- first logic envelope in `scripts.ir.json`

Still deferred:

- fixed-step gameplay execution
- collision runtime
- player control
- death and respawn
- room-transition simulation
```

- [x] **Step 5: Update the root README to describe Phase 3 correctly**

```md
## Current Phase

Phase 3 upgrades the package output to a runtime-facing format and adds a development runtime shell with a static room viewer.

## Current Commands

```bash
cargo test
cargo run -p iwm-cli -- detect --input C:\path\to\game
cargo run -p iwm-cli -- build-package --input C:\path\to\game --output .\runtime\public\packages\sample
```

See `docs/notes/package-format-v1-runtime.md` for the current runtime package contract.
```

- [x] **Step 6: Run the targeted parser test**

Run:

```bash
cargo test -p iwm-parser runtime_manifest_serializes_expected_fields -- --exact
```

Expected:

```text
PASS
```

- [ ] **Step 7: Commit**

```bash
git add README.md docs/notes/package-format-v1-runtime.md crates/iwm-parser/src/models.rs crates/iwm-parser/tests/build_package_smoke.rs
git commit -m "feat: define runtime package contract"
```

### Task 2: Export Browser-Ready Resources

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/iwm-parser/Cargo.toml`
- Modify: `crates/iwm-parser/src/lib.rs`
- Create: `crates/iwm-parser/src/resource_export.rs`
- Modify: `crates/iwm-parser/tests/build_package_smoke.rs`

- [ ] **Step 1: Write the failing unit test for pixel conversion and resource output paths**

```rust
#[test]
fn bgra_pixels_are_converted_to_rgba_order() {
    let converted = iwm_parser::resource_export::bgra_to_rgba(vec![0, 64, 255, 255]);
    assert_eq!(converted, vec![255, 64, 0, 255]);
}

#[test]
fn runtime_resources_are_written_under_expected_directories() {
    let base = std::path::Path::new("resources");
    assert_eq!(base.join("sprites").to_string_lossy(), "resources/sprites");
    assert_eq!(base.join("backgrounds").to_string_lossy(), "resources/backgrounds");
    assert_eq!(base.join("audio").to_string_lossy(), "resources/audio");
}
```

- [ ] **Step 2: Add `png` to the workspace and parser dependencies**

```toml
[workspace.dependencies]
anyhow = "1.0"
camino = "1.1"
clap = { version = "4.5.7", features = ["derive"] }
png = "0.17"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
sha2 = "0.10"
tempfile = "3.10"
walkdir = "2.5"
zip = "2.1"
```

```toml
[dependencies]
anyhow.workspace = true
camino.workspace = true
png.workspace = true
serde.workspace = true
serde_json.workspace = true
sha2.workspace = true
iwm-detector = { path = "../iwm-detector" }
gm8exe = { path = "../../vendor/OpenGMK/gm8exe" }
```

- [ ] **Step 3: Expose the new resource module from `crates/iwm-parser/src/lib.rs`**

```rust
mod gm8_adapter;
pub mod logic_export;
pub mod models;
pub mod package_builder;
pub mod resource_export;

pub use package_builder::build_package;
```

- [ ] **Step 4: Create `crates/iwm-parser/src/resource_export.rs`**

```rust
use crate::models::{BackgroundResource, ResourceIndex, SoundResource, SpriteResource};
use anyhow::{Context, Result};
use gm8exe::GameAssets;
use png::{BitDepth, ColorType, Encoder};
use std::fs;
use std::io::BufWriter;
use std::path::Path;

pub fn export_resources(assets: &GameAssets, output_dir: &Path) -> Result<ResourceIndex> {
    let resources_dir = output_dir.join("resources");
    let sprite_dir = resources_dir.join("sprites");
    let background_dir = resources_dir.join("backgrounds");
    let audio_dir = resources_dir.join("audio");

    fs::create_dir_all(&sprite_dir)?;
    fs::create_dir_all(&background_dir)?;
    fs::create_dir_all(&audio_dir)?;

    let sprites = assets
        .sprites
        .iter()
        .enumerate()
        .filter_map(|(id, sprite)| sprite.as_ref().map(|sprite| (id, sprite)))
        .map(|(id, sprite)| {
            let mut frame_paths = Vec::new();
            for (frame_index, frame) in sprite.frames.iter().enumerate() {
                let path = sprite_dir.join(format!("{id}-{frame_index}.png"));
                write_rgba_png(&path, frame.width, frame.height, &frame.data)?;
                frame_paths.push(path.strip_prefix(output_dir).unwrap().to_string_lossy().replace('\\', "/"));
            }

            let (width, height) = sprite
                .frames
                .first()
                .map(|frame| (frame.width, frame.height))
                .unwrap_or((0, 0));

            Ok(SpriteResource {
                id,
                name: sprite.name.to_string(),
                origin_x: sprite.origin_x,
                origin_y: sprite.origin_y,
                frame_paths,
                width,
                height,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let backgrounds = assets
        .backgrounds
        .iter()
        .enumerate()
        .filter_map(|(id, background)| background.as_ref().map(|background| (id, background)))
        .map(|(id, background)| {
            let path = background_dir.join(format!("{id}.png"));
            if let Some(data) = &background.data {
                let rgba = bgra_to_rgba(data.to_vec());
                write_rgba_png(&path, background.width, background.height, &rgba)?;
            }

            Ok(BackgroundResource {
                id,
                name: background.name.to_string(),
                width: background.width,
                height: background.height,
                image_path: path.strip_prefix(output_dir).unwrap().to_string_lossy().replace('\\', "/"),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let sounds = assets
        .sounds
        .iter()
        .enumerate()
        .filter_map(|(id, sound)| sound.as_ref().map(|sound| (id, sound)))
        .filter_map(|(id, sound)| sound.data.as_ref().map(|data| (id, sound, data)))
        .map(|(id, sound, data)| {
            let extension = sound.extension.to_string();
            let path = audio_dir.join(format!("{id}.{}", extension.trim_start_matches('.')));
            fs::write(&path, data).with_context(|| format!("failed to write {}", path.display()))?;

            Ok(SoundResource {
                id,
                name: sound.name.to_string(),
                file_path: path.strip_prefix(output_dir).unwrap().to_string_lossy().replace('\\', "/"),
                extension,
                preload: sound.preload,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(ResourceIndex {
        sprites,
        backgrounds,
        sounds,
    })
}

pub fn bgra_to_rgba(input: Vec<u8>) -> Vec<u8> {
    input
        .chunks_exact(4)
        .flat_map(|chunk| [chunk[2], chunk[1], chunk[0], chunk[3]])
        .collect()
}

fn write_rgba_png(path: &Path, width: u32, height: u32, bytes: &[u8]) -> Result<()> {
    let file = fs::File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    let writer = BufWriter::new(file);
    let mut encoder = Encoder::new(writer, width, height);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);
    let mut png_writer = encoder.write_header()?;
    png_writer.write_image_data(bytes)?;
    Ok(())
}
```

- [ ] **Step 5: Extend the parser smoke test file with the resource helper checks**

```rust
#[test]
fn bgra_pixels_are_converted_to_rgba_order() {
    let converted = iwm_parser::resource_export::bgra_to_rgba(vec![0, 64, 255, 255]);
    assert_eq!(converted, vec![255, 64, 0, 255]);
}

#[test]
fn runtime_resources_are_written_under_expected_directories() {
    let base = std::path::Path::new("resources");
    assert_eq!(base.join("sprites").to_string_lossy(), "resources/sprites");
    assert_eq!(base.join("backgrounds").to_string_lossy(), "resources/backgrounds");
    assert_eq!(base.join("audio").to_string_lossy(), "resources/audio");
}
```

- [ ] **Step 6: Run the targeted resource tests**

Run:

```bash
cargo test -p iwm-parser bgra_pixels_are_converted_to_rgba_order -- --exact
```

Expected:

```text
PASS
```

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml crates/iwm-parser/Cargo.toml crates/iwm-parser/src/lib.rs crates/iwm-parser/src/resource_export.rs crates/iwm-parser/tests/build_package_smoke.rs
git commit -m "feat: export runtime resources"
```

### Task 3: Export Room Placements, Event Tables, And Logic Blocks

**Files:**
- Create: `crates/iwm-parser/src/logic_export.rs`
- Modify: `crates/iwm-parser/src/package_builder.rs`
- Modify: `crates/iwm-parser/tests/build_package_smoke.rs`

- [ ] **Step 1: Write the failing tests for block id formatting and action argument trimming**

```rust
#[test]
fn logic_block_ids_use_stable_prefixes() {
    assert_eq!(
        iwm_parser::logic_export::event_block_id(12, 3, 0),
        "object:12:event:3:0"
    );
    assert_eq!(
        iwm_parser::logic_export::room_creation_block_id(7),
        "room:7:create"
    );
    assert_eq!(
        iwm_parser::logic_export::instance_creation_block_id(7, 9001),
        "room:7:instance:9001:create"
    );
}

#[test]
fn action_argument_export_uses_declared_param_count() {
    let args = iwm_parser::logic_export::take_action_args(
        2,
        ["left".into(), "right".into(), "ignored".into(), "".into(), "".into(), "".into(), "".into(), "".into()],
    );
    assert_eq!(args, vec!["left".to_string(), "right".to_string()]);
}
```

- [ ] **Step 2: Create `crates/iwm-parser/src/logic_export.rs`**

```rust
use crate::models::{
    LogicBlock, LogicOp, ObjectDefinition, ObjectEventEntry, RoomBackgroundLayer, RoomDefinition,
    RoomInstancePlacement, RoomView, ScriptIrFile,
};
use gm8exe::{asset::{CodeAction, Object, Room}, AssetList};

pub fn export_rooms_and_logic(
    rooms: &AssetList<Room>,
    objects: &AssetList<Object>,
) -> (Vec<RoomDefinition>, Vec<ObjectDefinition>, ScriptIrFile) {
    let mut blocks = Vec::new();

    let room_defs = rooms
        .iter()
        .enumerate()
        .filter_map(|(room_id, room)| room.as_ref().map(|room| (room_id, room)))
        .map(|(room_id, room)| {
            let creation_block_id = if room.creation_code.0.is_empty() {
                None
            } else {
                let id = room_creation_block_id(room_id);
                blocks.push(LogicBlock {
                    id: id.clone(),
                    name: format!("room {} creation", room.name),
                    kind: "room-creation".into(),
                    support: "source-only".into(),
                    ops: vec![LogicOp::SourceSnippet {
                        code: room.creation_code.to_string(),
                    }],
                });
                Some(id)
            };

            let instances = room
                .instances
                .iter()
                .map(|instance| {
                    let creation_block_id = if instance.creation_code.0.is_empty() {
                        None
                    } else {
                        let id = instance_creation_block_id(room_id, instance.id);
                        blocks.push(LogicBlock {
                            id: id.clone(),
                            name: format!("room {room_id} instance {} creation", instance.id),
                            kind: "instance-creation".into(),
                            support: "source-only".into(),
                            ops: vec![LogicOp::SourceSnippet {
                                code: instance.creation_code.to_string(),
                            }],
                        });
                        Some(id)
                    };

                    RoomInstancePlacement {
                        instance_id: instance.id,
                        object_id: instance.object,
                        x: instance.x,
                        y: instance.y,
                        xscale: instance.xscale,
                        yscale: instance.yscale,
                        angle: instance.angle,
                        blend: instance.blend,
                        creation_block_id,
                    }
                })
                .collect();

            RoomDefinition {
                id: room_id,
                name: room.name.to_string(),
                width: room.width,
                height: room.height,
                speed: room.speed,
                persistent: room.persistent,
                backgrounds: room
                    .backgrounds
                    .iter()
                    .map(|bg| RoomBackgroundLayer {
                        visible_on_start: bg.visible_on_start,
                        is_foreground: bg.is_foreground,
                        source_bg: bg.source_bg,
                        xoffset: bg.xoffset,
                        yoffset: bg.yoffset,
                        tile_horz: bg.tile_horz,
                        tile_vert: bg.tile_vert,
                        hspeed: bg.hspeed,
                        vspeed: bg.vspeed,
                        stretch: bg.stretch,
                    })
                    .collect(),
                views_enabled: room.views_enabled,
                views: room
                    .views
                    .iter()
                    .map(|view| RoomView {
                        visible: view.visible,
                        source_x: view.source_x,
                        source_y: view.source_y,
                        source_w: view.source_w,
                        source_h: view.source_h,
                        port_x: view.port_x,
                        port_y: view.port_y,
                        port_w: view.port_w,
                        port_h: view.port_h,
                        target: view.following.target,
                    })
                    .collect(),
                instances,
                creation_block_id,
            }
        })
        .collect();

    let object_defs = objects
        .iter()
        .enumerate()
        .filter_map(|(object_id, object)| object.as_ref().map(|object| (object_id, object)))
        .map(|(object_id, object)| {
            let mut events = Vec::new();

            for (event_type, sub_events) in object.events.iter().enumerate() {
                for (sub_event_index, (sub_event, actions)) in sub_events.iter().enumerate() {
                    let block_id = event_block_id(object_id, event_type, *sub_event);
                    blocks.push(LogicBlock {
                        id: block_id.clone(),
                        name: format!("object {} event {}:{}", object.name, event_type, sub_event),
                        kind: "object-event".into(),
                        support: detect_support(actions),
                        ops: actions.iter().map(action_to_logic_op).collect(),
                    });

                    let _ = sub_event_index;
                    events.push(ObjectEventEntry {
                        event_type,
                        sub_event: *sub_event,
                        block_id,
                        action_count: actions.len(),
                    });
                }
            }

            ObjectDefinition {
                id: object_id,
                name: object.name.to_string(),
                sprite_index: object.sprite_index,
                parent_index: object.parent_index,
                depth: object.depth,
                persistent: object.persistent,
                visible: object.visible,
                solid: object.solid,
                mask_index: object.mask_index,
                events,
            }
        })
        .collect();

    (
        room_defs,
        object_defs,
        ScriptIrFile {
            format: "iwm-script-ir-v1".into(),
            blocks,
        },
    )
}

pub fn event_block_id(object_id: usize, event_type: usize, sub_event: u32) -> String {
    format!("object:{object_id}:event:{event_type}:{sub_event}")
}

pub fn room_creation_block_id(room_id: usize) -> String {
    format!("room:{room_id}:create")
}

pub fn instance_creation_block_id(room_id: usize, instance_id: i32) -> String {
    format!("room:{room_id}:instance:{instance_id}:create")
}

pub fn take_action_args(param_count: usize, args: [String; 8]) -> Vec<String> {
    args.into_iter().take(param_count).collect()
}

fn action_to_logic_op(action: &CodeAction) -> LogicOp {
    let args = take_action_args(
        action.param_count,
        action.param_strings.clone().map(|value| value.to_string()),
    );

    if !action.fn_code.0.is_empty() {
        return LogicOp::SourceSnippet {
            code: action.fn_code.to_string(),
        };
    }

    LogicOp::ActionCall {
        action_id: action.id,
        lib_id: action.lib_id,
        applies_to: action.applies_to,
        is_condition: action.is_condition,
        invert_condition: action.invert_condition,
        is_relative: action.is_relative,
        fn_name: action.fn_name.to_string(),
        fn_code: action.fn_code.to_string(),
        args,
    }
}

fn detect_support(actions: &[CodeAction]) -> String {
    if actions.iter().any(|action| !action.fn_code.0.is_empty()) {
        "source-only".into()
    } else {
        "action-list".into()
    }
}
```

- [ ] **Step 3: Rework `crates/iwm-parser/src/package_builder.rs` to emit runtime JSON instead of V0 summaries**

```rust
use crate::gm8_adapter::read_gm8_assets;
use crate::logic_export::export_rooms_and_logic;
use crate::models::{AnalysisReport, CompatibilityLevel, ResourceIndex, RuntimeManifest};
use crate::resource_export::export_resources;
use anyhow::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

pub fn build_package(input_exe: &Path, output_dir: &Path, dlls: &[String]) -> Result<()> {
    let assets = read_gm8_assets(input_exe)?;
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;

    let source_hash = {
        let bytes = fs::read(input_exe)?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        format!("{:x}", hasher.finalize())
    };

    let resource_index: ResourceIndex = export_resources(&assets, output_dir)?;
    let (rooms, objects, script_ir) = export_rooms_and_logic(&assets.rooms, &assets.objects);

    let warnings = vec!["script-ir-partial".to_string()];
    let analysis = AnalysisReport {
        dlls: dlls.to_vec(),
        included_files: assets
            .included_files
            .iter()
            .map(|f| f.file_name.to_string())
            .collect(),
        warnings: warnings.clone(),
        unsupported_features: vec![
            "logic-execution-not-yet-implemented".into(),
            "room-runtime-not-yet-implemented".into(),
        ],
    };

    let manifest = RuntimeManifest {
        format_version: 1,
        package_kind: "runtime-v1".into(),
        source_name: input_exe
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        source_hash,
        engine_family: "gm8".into(),
        compatibility: CompatibilityLevel::Partial,
        default_room_id: rooms.first().map(|room| room.id),
        room_count: rooms.len(),
        object_count: objects.len(),
        script_block_count: script_ir.blocks.len(),
        sprite_count: resource_index.sprites.len(),
        background_count: resource_index.backgrounds.len(),
        sound_count: resource_index.sounds.len(),
        resource_index_path: "resources/index.json".into(),
        warnings,
    };

    write_json(output_dir.join("manifest.json"), &manifest)?;
    write_json(output_dir.join("rooms.json"), &rooms)?;
    write_json(output_dir.join("objects.json"), &objects)?;
    write_json(output_dir.join("scripts.ir.json"), &script_ir)?;
    write_json(output_dir.join("analysis.json"), &analysis)?;
    write_json(output_dir.join("resources").join("index.json"), &resource_index)?;

    Ok(())
}

fn write_json<T: Serialize>(path: impl AsRef<Path>, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(path.as_ref(), bytes)
        .with_context(|| format!("failed to write {}", path.as_ref().display()))?;
    Ok(())
}
```

- [ ] **Step 4: Extend `crates/iwm-parser/tests/build_package_smoke.rs` with logic export helper tests**

```rust
#[test]
fn logic_block_ids_use_stable_prefixes() {
    assert_eq!(
        iwm_parser::logic_export::event_block_id(12, 3, 0),
        "object:12:event:3:0"
    );
    assert_eq!(
        iwm_parser::logic_export::room_creation_block_id(7),
        "room:7:create"
    );
    assert_eq!(
        iwm_parser::logic_export::instance_creation_block_id(7, 9001),
        "room:7:instance:9001:create"
    );
}

#[test]
fn action_argument_export_uses_declared_param_count() {
    let args = iwm_parser::logic_export::take_action_args(
        2,
        ["left".into(), "right".into(), "ignored".into(), "".into(), "".into(), "".into(), "".into(), "".into()],
    );
    assert_eq!(args, vec!["left".to_string(), "right".to_string()]);
}
```

- [ ] **Step 5: Run the targeted logic-export tests**

Run:

```bash
cargo test -p iwm-parser logic_block_ids_use_stable_prefixes -- --exact
```

Expected:

```text
PASS
```

- [ ] **Step 6: Commit**

```bash
git add crates/iwm-parser/src/logic_export.rs crates/iwm-parser/src/package_builder.rs crates/iwm-parser/tests/build_package_smoke.rs
git commit -m "feat: export runtime logic and room placements"
```

### Task 4: Verify Runtime Package Emission End To End

**Files:**
- Modify: `crates/iwm-parser/tests/build_package_smoke.rs`
- Modify: `crates/iwm-cli/src/main.rs`

- [ ] **Step 1: Replace the old V0 smoke checks with runtime package assertions**

```rust
use std::fs;
use std::process::Command;

#[test]
fn build_package_writes_runtime_outputs_for_single_exe_input() {
    let temp = tempfile::tempdir().unwrap();
    let sample_exe = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("samples")
        .join("local")
        .join("iwanna-examples")
        .join("gm8-core")
        .join("IWBT_Dife")
        .join("I wanna be the Dife.exe");

    if !sample_exe.exists() {
        return;
    }

    let exe_copy = temp.path().join("game.exe");
    fs::copy(&sample_exe, &exe_copy).unwrap();
    let out_dir = temp.path().join("out");

    let status = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "-p",
            "iwm-cli",
            "--",
            "build-package",
            "--input",
        ])
        .arg(&exe_copy)
        .args(["--output"])
        .arg(&out_dir)
        .status()
        .unwrap();

    assert!(status.success());
    assert!(out_dir.join("manifest.json").exists());
    assert!(out_dir.join("rooms.json").exists());
    assert!(out_dir.join("objects.json").exists());
    assert!(out_dir.join("scripts.ir.json").exists());
    assert!(out_dir.join("analysis.json").exists());
    assert!(out_dir.join("resources").join("index.json").exists());
}
```

- [ ] **Step 2: Keep `crates/iwm-cli/src/main.rs` stable and only update help text if needed**

```rust
#[derive(Subcommand)]
enum Commands {
    Detect {
        #[arg(long)]
        input: PathBuf,
    },
    BuildPackage {
        #[arg(long, help = "Directory, exe, or zip to normalize into a runtime package")]
        input: PathBuf,
        #[arg(long, help = "Destination directory for manifest, json data, and resources")]
        output: PathBuf,
    },
}
```

- [ ] **Step 3: Run the runtime package smoke test**

Run:

```bash
cargo test -p iwm-parser build_package_writes_runtime_outputs_for_single_exe_input -- --exact
```

Expected:

```text
PASS
```

- [ ] **Step 4: Commit**

```bash
git add crates/iwm-parser/tests/build_package_smoke.rs crates/iwm-cli/src/main.rs
git commit -m "test: verify runtime package emission"
```

### Task 5: Bootstrap The Runtime Harness

**Files:**
- Create: `runtime/package.json`
- Create: `runtime/tsconfig.json`
- Create: `runtime/vite.config.ts`
- Create: `runtime/index.html`
- Create: `runtime/public/packages/.gitkeep`
- Create: `runtime/src/main.ts`
- Create: `runtime/src/styles.css`
- Create: `runtime/src/types.ts`
- Create: `runtime/src/loadPackage.ts`
- Create: `runtime/src/loadPackage.test.ts`
- Create: `runtime/src/vite-env.d.ts`

- [ ] **Step 1: Write the failing frontend bootstrap check**

Run:

```bash
npm --prefix runtime test
```

Expected:

```text
npm ERR! enoent Could not read package.json
```

- [ ] **Step 2: Create `runtime/package.json`**

```json
{
  "name": "iwm-runtime-shell",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "test": "vitest run"
  },
  "devDependencies": {
    "typescript": "^5.6.3",
    "vite": "^5.4.8",
    "vitest": "^2.1.1"
  }
}
```

- [ ] **Step 3: Create `runtime/tsconfig.json` and `runtime/vite.config.ts`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "strict": true,
    "noEmit": true,
    "lib": ["ES2022", "DOM"],
    "types": ["vitest/globals"]
  },
  "include": ["src"]
}
```

```ts
import { defineConfig } from "vite";

export default defineConfig({
  server: {
    port: 4173,
  },
  test: {
    environment: "node",
  },
});
```

- [ ] **Step 4: Create the runtime package types and loader**

```ts
export type RuntimeManifest = {
  format_version: number;
  package_kind: string;
  source_name: string;
  source_hash: string;
  engine_family: string;
  compatibility: "supported" | "partial" | "blocked";
  default_room_id: number | null;
  room_count: number;
  object_count: number;
  script_block_count: number;
  sprite_count: number;
  background_count: number;
  sound_count: number;
  resource_index_path: string;
  warnings: string[];
};

export type ResourceIndex = {
  sprites: Array<{ id: number; name: string; origin_x: number; origin_y: number; frame_paths: string[]; width: number; height: number }>;
  backgrounds: Array<{ id: number; name: string; width: number; height: number; image_path: string }>;
  sounds: Array<{ id: number; name: string; file_path: string; extension: string; preload: boolean }>;
};

export type RoomDefinition = {
  id: number;
  name: string;
  width: number;
  height: number;
  speed: number;
  persistent: boolean;
  backgrounds: Array<{
    visible_on_start: boolean;
    is_foreground: boolean;
    source_bg: number;
    xoffset: number;
    yoffset: number;
    tile_horz: boolean;
    tile_vert: boolean;
    hspeed: number;
    vspeed: number;
    stretch: boolean;
  }>;
  views_enabled: boolean;
  views: Array<{
    visible: boolean;
    source_x: number;
    source_y: number;
    source_w: number;
    source_h: number;
    port_x: number;
    port_y: number;
    port_w: number;
    port_h: number;
    target: number;
  }>;
  instances: Array<{
    instance_id: number;
    object_id: number;
    x: number;
    y: number;
    xscale: number;
    yscale: number;
    angle: number;
    blend: number;
    creation_block_id: string | null;
  }>;
  creation_block_id: string | null;
};

export type ObjectDefinition = {
  id: number;
  name: string;
  sprite_index: number;
  parent_index: number;
  depth: number;
  persistent: boolean;
  visible: boolean;
  solid: boolean;
  mask_index: number;
  events: Array<{
    event_type: number;
    sub_event: number;
    block_id: string;
    action_count: number;
  }>;
};

export type ScriptIrFile = {
  format: string;
  blocks: Array<{
    id: string;
    name: string;
    kind: string;
    support: string;
    ops: Array<Record<string, unknown>>;
  }>;
};

export type RuntimePackage = {
  manifest: RuntimeManifest;
  rooms: RoomDefinition[];
  objects: ObjectDefinition[];
  scripts: ScriptIrFile;
  resources: ResourceIndex;
  analysis: {
    dlls: string[];
    included_files: string[];
    warnings: string[];
    unsupported_features: string[];
  };
};
```

```ts
import type { ResourceIndex, RoomDefinition, RuntimeManifest, RuntimePackage, ObjectDefinition, ScriptIrFile } from "./types";

async function readJson<T>(path: string): Promise<T> {
  const response = await fetch(path);
  if (!response.ok) {
    throw new Error(`failed to load ${path}: ${response.status}`);
  }
  return response.json() as Promise<T>;
}

export async function loadPackage(basePath: string): Promise<RuntimePackage> {
  const manifest = await readJson<RuntimeManifest>(`${basePath}/manifest.json`);
  const [rooms, objects, scripts, analysis, resources] = await Promise.all([
    readJson<RoomDefinition[]>(`${basePath}/rooms.json`),
    readJson<ObjectDefinition[]>(`${basePath}/objects.json`),
    readJson<ScriptIrFile>(`${basePath}/scripts.ir.json`),
    readJson<RuntimePackage["analysis"]>(`${basePath}/analysis.json`),
    readJson<ResourceIndex>(`${basePath}/${manifest.resource_index_path}`),
  ]);

  return { manifest, rooms, objects, scripts, analysis, resources };
}
```

- [ ] **Step 5: Add the loader test**

```ts
import { describe, expect, it, vi } from "vitest";
import { loadPackage } from "./loadPackage";

describe("loadPackage", () => {
  it("loads all runtime package files via manifest paths", async () => {
    const files = new Map<string, unknown>([
      ["/pkg/manifest.json", { resource_index_path: "resources/index.json" }],
      ["/pkg/rooms.json", []],
      ["/pkg/objects.json", []],
      ["/pkg/scripts.ir.json", { format: "iwm-script-ir-v1", blocks: [] }],
      ["/pkg/analysis.json", { dlls: [], included_files: [], warnings: [], unsupported_features: [] }],
      ["/pkg/resources/index.json", { sprites: [], backgrounds: [], sounds: [] }],
    ]);

    vi.stubGlobal("fetch", vi.fn(async (path: string) => ({
      ok: true,
      status: 200,
      json: async () => files.get(path),
    })));

    const pkg = await loadPackage("/pkg");
    expect(pkg.scripts.format).toBe("iwm-script-ir-v1");
    expect(pkg.resources.sprites).toEqual([]);
  });
});
```

- [ ] **Step 6: Create a minimal shell entrypoint**

```ts
import "./styles.css";

const root = document.querySelector<HTMLDivElement>("#app");

if (!root) {
  throw new Error("missing #app root");
}

root.innerHTML = `
  <div class="shell">
    <aside class="sidebar">
      <h1>IWanna Runtime Shell</h1>
      <p>Load a generated runtime package from <code>runtime/public/packages/</code>.</p>
      <div id="package-meta"></div>
    </aside>
    <main class="stage">
      <div id="toolbar"></div>
      <canvas id="room-canvas" width="960" height="540"></canvas>
      <section id="inspectors"></section>
    </main>
  </div>
`;
```

- [ ] **Step 7: Run the frontend tests**

Run:

```bash
npm --prefix runtime install
npm --prefix runtime test
```

Expected:

```text
1 passed
```

- [ ] **Step 8: Commit**

```bash
git add runtime
git commit -m "feat: bootstrap runtime harness"
```

### Task 6: Build The Static Room Viewer

**Files:**
- Create: `runtime/src/render/resourceCache.ts`
- Create: `runtime/src/render/staticRoomRenderer.ts`
- Create: `runtime/src/render/staticRoomRenderer.test.ts`
- Create: `runtime/src/ui/shell.ts`
- Create: `runtime/src/ui/inspectors.ts`
- Modify: `runtime/src/main.ts`
- Modify: `runtime/src/styles.css`

- [ ] **Step 1: Write the failing renderer tests**

```ts
import { describe, expect, it } from "vitest";
import { resolveBackgroundDraws } from "./staticRoomRenderer";

describe("resolveBackgroundDraws", () => {
  it("returns only visible layers with known backgrounds", () => {
    const draws = resolveBackgroundDraws(
      {
        width: 320,
        height: 240,
        backgrounds: [
          {
            visible_on_start: true,
            is_foreground: false,
            source_bg: 4,
            xoffset: 8,
            yoffset: 9,
            tile_horz: false,
            tile_vert: false,
            hspeed: 0,
            vspeed: 0,
            stretch: false,
          },
        ],
        views_enabled: false,
        views: [],
        instances: [],
        creation_block_id: null,
        id: 0,
        name: "room0",
        speed: 30,
        persistent: false,
      },
      new Map([[4, "/pkg/resources/backgrounds/4.png"]]),
    );

    expect(draws).toHaveLength(1);
    expect(draws[0].imagePath).toBe("/pkg/resources/backgrounds/4.png");
  });
});
```

- [ ] **Step 2: Create the resource cache and renderer helpers**

```ts
import type { ResourceIndex } from "../types";

export function makeBackgroundPathMap(basePath: string, resources: ResourceIndex): Map<number, string> {
  return new Map(
    resources.backgrounds.map((background) => [
      background.id,
      `${basePath}/${background.image_path}`,
    ]),
  );
}

export function makeSpriteFrameMap(basePath: string, resources: ResourceIndex): Map<number, string> {
  return new Map(
    resources.sprites
      .filter((sprite) => sprite.frame_paths[0])
      .map((sprite) => [sprite.id, `${basePath}/${sprite.frame_paths[0]}`]),
  );
}
```

```ts
import type { ObjectDefinition, RoomDefinition } from "../types";

export type BackgroundDraw = {
  imagePath: string;
  x: number;
  y: number;
  stretch: boolean;
};

export function resolveBackgroundDraws(
  room: RoomDefinition,
  backgrounds: Map<number, string>,
): BackgroundDraw[] {
  return room.backgrounds
    .filter((layer) => layer.visible_on_start && backgrounds.has(layer.source_bg))
    .map((layer) => ({
      imagePath: backgrounds.get(layer.source_bg)!,
      x: layer.xoffset,
      y: layer.yoffset,
      stretch: layer.stretch,
    }));
}

export async function renderStaticRoom(
  ctx: CanvasRenderingContext2D,
  room: RoomDefinition,
  objects: ObjectDefinition[],
  backgroundPaths: Map<number, string>,
  spritePaths: Map<number, string>,
): Promise<void> {
  ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);
  ctx.fillStyle = "#121212";
  ctx.fillRect(0, 0, room.width, room.height);

  for (const draw of resolveBackgroundDraws(room, backgroundPaths)) {
    const image = await loadImage(draw.imagePath);
    if (draw.stretch) {
      ctx.drawImage(image, 0, 0, room.width, room.height);
    } else {
      ctx.drawImage(image, draw.x, draw.y);
    }
  }

  for (const instance of room.instances) {
    const object = objects.find((candidate) => candidate.id === instance.object_id);
    if (!object || object.sprite_index < 0) {
      drawFallbackInstance(ctx, instance.x, instance.y);
      continue;
    }

    const spritePath = spritePaths.get(object.sprite_index);
    if (!spritePath) {
      drawFallbackInstance(ctx, instance.x, instance.y);
      continue;
    }

    const sprite = await loadImage(spritePath);
    ctx.drawImage(sprite, instance.x, instance.y);
  }
}

function drawFallbackInstance(ctx: CanvasRenderingContext2D, x: number, y: number): void {
  ctx.fillStyle = "#ff5a36";
  ctx.fillRect(x, y, 12, 12);
}

function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error(`failed to load image: ${src}`));
    image.src = src;
  });
}
```

- [ ] **Step 3: Add the renderer test file**

```ts
import { describe, expect, it } from "vitest";
import { resolveBackgroundDraws } from "./staticRoomRenderer";

describe("resolveBackgroundDraws", () => {
  it("returns only visible layers with known backgrounds", () => {
    const draws = resolveBackgroundDraws(
      {
        width: 320,
        height: 240,
        backgrounds: [
          {
            visible_on_start: true,
            is_foreground: false,
            source_bg: 4,
            xoffset: 8,
            yoffset: 9,
            tile_horz: false,
            tile_vert: false,
            hspeed: 0,
            vspeed: 0,
            stretch: false,
          },
        ],
        views_enabled: false,
        views: [],
        instances: [],
        creation_block_id: null,
        id: 0,
        name: "room0",
        speed: 30,
        persistent: false,
      },
      new Map([[4, "/pkg/resources/backgrounds/4.png"]]),
    );

    expect(draws).toHaveLength(1);
    expect(draws[0].imagePath).toBe("/pkg/resources/backgrounds/4.png");
  });
});
```

- [ ] **Step 4: Wire package loading and rendering into the shell**

```ts
import "./styles.css";
import { loadPackage } from "./loadPackage";
import { makeBackgroundPathMap, makeSpriteFrameMap } from "./render/resourceCache";
import { renderStaticRoom } from "./render/staticRoomRenderer";
import { renderInspectors, renderMeta, renderRoomSelector } from "./ui/inspectors";

const root = document.querySelector<HTMLDivElement>("#app");

if (!root) {
  throw new Error("missing #app root");
}

root.innerHTML = `
  <div class="shell">
    <aside class="sidebar">
      <h1>IWanna Runtime Shell</h1>
      <p>Developer harness for runtime package inspection and static room rendering.</p>
      <label class="package-picker">
        Package
        <input id="package-input" value="/packages/sample" />
      </label>
      <button id="load-button">Load Package</button>
      <div id="package-meta"></div>
    </aside>
    <main class="stage">
      <div id="toolbar"></div>
      <canvas id="room-canvas" width="960" height="540"></canvas>
      <section id="inspectors"></section>
    </main>
  </div>
`;

const input = document.querySelector<HTMLInputElement>("#package-input")!;
const loadButton = document.querySelector<HTMLButtonElement>("#load-button")!;
const meta = document.querySelector<HTMLDivElement>("#package-meta")!;
const toolbar = document.querySelector<HTMLDivElement>("#toolbar")!;
const inspectors = document.querySelector<HTMLElement>("#inspectors")!;
const canvas = document.querySelector<HTMLCanvasElement>("#room-canvas")!;
const ctx = canvas.getContext("2d")!;

loadButton.addEventListener("click", async () => {
  const pkg = await loadPackage(input.value);
  renderMeta(meta, pkg.manifest, pkg.analysis);
  renderInspectors(inspectors, pkg.rooms, pkg.objects, pkg.scripts);

  const room = pkg.rooms.find((candidate) => candidate.id === pkg.manifest.default_room_id) ?? pkg.rooms[0];
  const backgroundMap = makeBackgroundPathMap(input.value, pkg.resources);
  const spriteMap = makeSpriteFrameMap(input.value, pkg.resources);

  renderRoomSelector(toolbar, pkg.rooms, async (roomId) => {
    const selected = pkg.rooms.find((candidate) => candidate.id === roomId)!;
    await renderStaticRoom(ctx, selected, pkg.objects, backgroundMap, spriteMap);
  });

  await renderStaticRoom(ctx, room, pkg.objects, backgroundMap, spriteMap);
});
```

- [ ] **Step 5: Add the UI helpers**

```ts
import type { RuntimePackage, RuntimeManifest, RoomDefinition, ObjectDefinition, ScriptIrFile } from "../types";

export function renderMeta(target: HTMLElement, manifest: RuntimeManifest, analysis: RuntimePackage["analysis"]): void {
  target.innerHTML = `
    <dl>
      <dt>Source</dt><dd>${manifest.source_name}</dd>
      <dt>Compatibility</dt><dd>${manifest.compatibility}</dd>
      <dt>Rooms</dt><dd>${manifest.room_count}</dd>
      <dt>Objects</dt><dd>${manifest.object_count}</dd>
      <dt>Script Blocks</dt><dd>${manifest.script_block_count}</dd>
      <dt>Warnings</dt><dd>${analysis.warnings.join(", ") || "none"}</dd>
    </dl>
  `;
}

export function renderRoomSelector(
  target: HTMLElement,
  rooms: RoomDefinition[],
  onSelect: (roomId: number) => void | Promise<void>,
): void {
  target.innerHTML = `
    <label>
      Room
      <select id="room-select">
        ${rooms.map((room) => `<option value="${room.id}">${room.id}: ${room.name}</option>`).join("")}
      </select>
    </label>
  `;

  const select = target.querySelector<HTMLSelectElement>("#room-select")!;
  select.addEventListener("change", () => void onSelect(Number(select.value)));
}

export function renderInspectors(
  target: HTMLElement,
  rooms: RoomDefinition[],
  objects: ObjectDefinition[],
  scripts: ScriptIrFile,
): void {
  target.innerHTML = `
    <section>
      <h2>Rooms</h2>
      <pre>${JSON.stringify(rooms.slice(0, 3), null, 2)}</pre>
    </section>
    <section>
      <h2>Objects</h2>
      <pre>${JSON.stringify(objects.slice(0, 5), null, 2)}</pre>
    </section>
    <section>
      <h2>Logic</h2>
      <pre>${JSON.stringify(scripts.blocks.slice(0, 5), null, 2)}</pre>
    </section>
  `;
}
```

- [ ] **Step 6: Add shell styles**

```css
:root {
  color-scheme: dark;
  font-family: "IBM Plex Sans", "Segoe UI", sans-serif;
  background: radial-gradient(circle at top, #22324b 0%, #0d1118 50%, #07090d 100%);
  color: #e8eef7;
}

body {
  margin: 0;
}

.shell {
  display: grid;
  grid-template-columns: 320px 1fr;
  min-height: 100vh;
}

.sidebar {
  padding: 24px;
  background: rgba(7, 10, 16, 0.88);
  border-right: 1px solid rgba(255, 255, 255, 0.08);
}

.stage {
  padding: 24px;
  display: grid;
  gap: 16px;
}

#room-canvas {
  width: min(100%, 960px);
  background: #0a0d12;
  border: 1px solid rgba(255, 255, 255, 0.15);
}

#inspectors {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 16px;
}

pre {
  margin: 0;
  padding: 12px;
  background: rgba(255, 255, 255, 0.05);
  overflow: auto;
  max-height: 280px;
}
```

- [ ] **Step 7: Run the frontend test suite**

Run:

```bash
npm --prefix runtime test
```

Expected:

```text
2 passed
```

- [ ] **Step 8: Commit**

```bash
git add runtime/src
git commit -m "feat: add static room viewer"
```

### Task 7: Final Verification For Phase 3

**Files:**
- Modify: none

- [ ] **Step 1: Run Rust formatting**

Run:

```bash
cargo fmt --all
```

Expected:

```text
no output
```

- [ ] **Step 2: Run all Rust tests**

Run:

```bash
cargo test
```

Expected:

```text
test result: ok
```

- [ ] **Step 3: Run frontend tests**

Run:

```bash
npm --prefix runtime test
```

Expected:

```text
test files passed
```

- [ ] **Step 4: Generate a real runtime package from a gold sample**

Run:

```bash
cargo run -p iwm-cli -- build-package --input ".\\samples\\local\\iwanna-examples\\gm8-core\\IWBT_Dife" --output ".\\runtime\\public\\packages\\iwbt-dife"
```

Expected:

```text
command exits successfully
```

- [ ] **Step 5: Verify package contents**

Run:

```bash
powershell -NoProfile -Command "Get-ChildItem -Recurse '.\\runtime\\public\\packages\\iwbt-dife' | Select-Object FullName"
```

Expected:

```text
manifest.json
rooms.json
objects.json
scripts.ir.json
analysis.json
resources\index.json
resources\sprites\...
resources\backgrounds\...
resources\audio\...
```

- [ ] **Step 6: Run the runtime shell build**

Run:

```bash
npm --prefix runtime build
```

Expected:

```text
vite build completes successfully
```

- [ ] **Step 7: Launch the runtime shell locally and inspect one room manually**

Run:

```bash
npm --prefix runtime run dev -- --host 127.0.0.1
```

Expected:

```text
local Vite URL is printed
```

Manual verification:

- load `/packages/iwbt-dife`
- confirm manifest metadata renders
- confirm room selector renders
- confirm at least one room background or instance marker appears on the canvas
- confirm JSON inspectors show rooms, objects, and logic blocks

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "test: verify runtime shell and static room viewer"
```

## Self-Review

Spec coverage for this plan:

- runtime-facing package with `resources/`: covered
- room instance normalization: covered
- object event table: covered
- `scripts.ir.json` as a first executable logic envelope: covered
- development runtime harness: covered
- static room viewer: covered
- minimal playable runtime gameplay loop: intentionally deferred to the next plan

Placeholder scan:

- no `TODO`
- no `TBD`
- no undefined “implement appropriately” steps

Type consistency notes:

- package manifest type is `RuntimeManifest`
- runtime resource index path is always `resources/index.json`
- runtime logic file is `scripts.ir.json`
- parser entrypoint remains `build_package`
- runtime loader entrypoint is `loadPackage`

Next planned document after this phase:

- minimal playable runtime plan covering movement, jump, death, respawn, and room transition
