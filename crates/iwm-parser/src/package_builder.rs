use crate::gm8_adapter::read_gm8_assets;
use crate::gml_lowering::lower_raw_logic_file;
use crate::logic_export::export_rooms_and_logic;
use crate::models::{AnalysisReport, CompatibilityLevel, LogicOp, RawLogicFile, RuntimeManifest};
use crate::raw_logic_export::export_raw_logic;
use crate::resource_export::export_resources;
use crate::LoweredLogicStatement;
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
    let (mut rooms, objects, script_ir) =
        export_rooms_and_logic(&assets.rooms, &assets.objects, &assets.scripts);
    let room_order = normalized_room_order(&assets.room_order, &rooms);
    sort_rooms_by_order(&mut rooms, &room_order);
    let raw_logic: RawLogicFile = export_raw_logic(&assets);
    let lowered_logic = lower_raw_logic_file(&raw_logic);

    // Generate actionable warnings
    let mut warnings = Vec::new();

    let lowered_entry_by_block_id = lowered_logic
        .entries
        .iter()
        .map(|entry| (entry.block_id.as_str(), entry))
        .collect::<std::collections::HashMap<_, _>>();

    // Check for source-only blocks that still rely on raw fallback after lowering
    for block in &script_ir.blocks {
        let still_has_raw = lowered_entry_by_block_id
            .get(block.id.as_str())
            .map(|entry| {
                entry
                    .statements
                    .iter()
                    .any(|statement| matches!(statement, LoweredLogicStatement::Raw { .. }))
            })
            .unwrap_or(false);

        if block.support == "source-only" && still_has_raw {
            warnings.push(format!("runtime-missing-source-lowering:{}", block.id));
        }
    }

    // Check for unsupported event types
    let mut seen_event_types: std::collections::HashSet<String> = std::collections::HashSet::new();
    for obj in &objects {
        for event in &obj.events {
            let tag = &event.event_tag;
            // Group unsupported event types (trigger, timeline, etc.)
            if tag.starts_with("trigger:") || tag.starts_with("other:user") {
                if !seen_event_types.contains(tag) {
                    seen_event_types.insert(tag.clone());
                    warnings.push(format!("runtime-unsupported-event:{}", tag));
                }
            }
        }
    }

    // Check for unsupported actions
    let mut seen_actions: std::collections::HashSet<String> = std::collections::HashSet::new();
    let unsupported_action_prefixes = [
        "game_", "file_", "sound_", "window_", "os_", "http_", "shopify_", "steam_",
    ];
    for block in &script_ir.blocks {
        for op in &block.ops {
            if let LogicOp::ActionCall { fn_name, .. } = op {
                let lower = fn_name.to_lowercase();
                if unsupported_action_prefixes
                    .iter()
                    .any(|p| lower.starts_with(p))
                {
                    if !seen_actions.contains(&lower) {
                        seen_actions.insert(lower.clone());
                        warnings.push(format!("runtime-unsupported-action:{}", fn_name));
                    }
                }
            }
        }
    }

    // Check for raw-fallback statements in lowered logic
    let raw_statement_count = lowered_logic
        .entries
        .iter()
        .flat_map(|entry| entry.statements.iter())
        .filter(|statement| matches!(statement, LoweredLogicStatement::Raw { .. }))
        .count();

    if raw_statement_count > 0 {
        warnings.push(format!(
            "runtime-lowered-raw-fallback-count:{raw_statement_count}"
        ));
    }

    // Add a note about partial execution support
    if !warnings.is_empty() {
        warnings.push("script-ir-partial".to_string());
    }

    let analysis = AnalysisReport {
        dlls: dlls.to_vec(),
        included_files: assets
            .included_files
            .iter()
            .map(|f| f.file_name.to_string())
            .collect(),
        warnings: warnings.clone(),
        unsupported_features: vec!["logic-execution-not-yet-implemented".into()],
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
        default_room_id: room_order
            .first()
            .copied()
            .or_else(|| rooms.first().map(|room| room.id)),
        room_order,
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
    write_json(output_dir.join("logic.raw.json"), &raw_logic)?;
    write_json(output_dir.join("logic.lowered.json"), &lowered_logic)?;
    write_json(output_dir.join("analysis.json"), &analysis)?;
    write_json(
        output_dir.join("resources").join("index.json"),
        &resource_index,
    )?;

    Ok(())
}

fn normalized_room_order(
    source_order: &[i32],
    rooms: &[crate::models::RoomDefinition],
) -> Vec<usize> {
    let room_ids = rooms
        .iter()
        .map(|room| room.id)
        .collect::<std::collections::HashSet<_>>();
    let mut order = source_order
        .iter()
        .filter_map(|room_id| usize::try_from(*room_id).ok())
        .filter(|room_id| room_ids.contains(room_id))
        .collect::<Vec<_>>();

    for room in rooms {
        if !order.contains(&room.id) {
            order.push(room.id);
        }
    }

    order
}

fn sort_rooms_by_order(rooms: &mut [crate::models::RoomDefinition], room_order: &[usize]) {
    let order_index = room_order
        .iter()
        .enumerate()
        .map(|(index, room_id)| (*room_id, index))
        .collect::<std::collections::HashMap<_, _>>();

    rooms.sort_by_key(|room| order_index.get(&room.id).copied().unwrap_or(usize::MAX));
}

fn write_json<T: Serialize>(path: impl AsRef<Path>, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(path.as_ref(), bytes)
        .with_context(|| format!("failed to write {}", path.as_ref().display()))?;
    Ok(())
}
