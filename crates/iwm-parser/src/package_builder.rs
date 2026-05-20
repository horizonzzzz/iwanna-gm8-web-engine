use crate::gm8_adapter::read_gm8_assets;
use crate::logic_export::export_rooms_and_logic;
use crate::models::{AnalysisReport, CompatibilityLevel, RuntimeManifest};
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

    let resource_index = export_resources(&assets, output_dir)?;
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
    write_json(
        output_dir.join("resources").join("index.json"),
        &resource_index,
    )?;

    Ok(())
}

fn write_json<T: Serialize>(path: impl AsRef<Path>, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(path.as_ref(), bytes)
        .with_context(|| format!("failed to write {}", path.as_ref().display()))?;
    Ok(())
}
