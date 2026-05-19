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
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;

    let source_hash = {
        let bytes = fs::read(input_exe)?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        format!("{:x}", hasher.finalize())
    };

    let rooms: Vec<RoomSummary> = assets
        .rooms
        .iter()
        .enumerate()
        .filter_map(|(id, room)| {
            room.as_ref().map(|room| RoomSummary {
                id,
                name: room.name.to_string(),
                width: room.width,
                height: room.height,
                speed: room.speed,
                persistent: room.persistent,
                instance_count: room.instances.len(),
            })
        })
        .collect();

    let objects: Vec<ObjectSummary> = assets
        .objects
        .iter()
        .enumerate()
        .filter_map(|(id, object)| {
            object.as_ref().map(|object| ObjectSummary {
                id,
                name: object.name.to_string(),
                sprite_index: object.sprite_index,
                parent_index: object.parent_index,
                depth: object.depth,
                persistent: object.persistent,
                visible: object.visible,
                solid: object.solid,
                event_count: object.events.len(),
            })
        })
        .collect();

    let scripts: Vec<ScriptSummary> = assets
        .scripts
        .iter()
        .enumerate()
        .filter_map(|(id, script)| {
            script.as_ref().map(|script| ScriptSummary {
                id,
                name: script.name.to_string(),
                code_len: script.source.0.len(),
            })
        })
        .collect();

    let analysis = AnalysisReport {
        dlls: dlls.to_vec(),
        included_files: assets
            .included_files
            .iter()
            .map(|f| f.file_name.to_string())
            .collect(),
        warnings: Vec::new(),
        unsupported_features: vec![
            "resource-export-not-yet-implemented".into(),
            "script-ir-not-yet-implemented".into(),
        ],
    };

    let compatibility = CompatibilityLevel::Partial;

    let manifest = PackageManifest {
        format_version: 0,
        source_name: input_exe
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
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
    fs::write(path.as_ref(), bytes)
        .with_context(|| format!("failed to write {}", path.as_ref().display()))?;
    Ok(())
}
