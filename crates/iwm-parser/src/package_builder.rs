use crate::gm8_adapter::read_gm8_assets;
use crate::gml_lowering::lower_raw_logic_file;
use crate::logic_export::export_rooms_and_logic;
use crate::models::{
    AnalysisReport, CompatibilityLevel, LogicOp, RawLogicFile, RuntimeDisplaySource,
    RuntimeManifest,
};
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
    let exported_background_ids = resource_index
        .backgrounds
        .iter()
        .map(|background| background.id)
        .collect::<std::collections::HashSet<_>>();
    let mut warnings = Vec::new();
    normalize_missing_background_references(&mut rooms, &exported_background_ids, &mut warnings);
    let normalization_warning_count = warnings.len();
    let room_order = normalized_room_order(&assets.room_order, &rooms);
    sort_rooms_by_order(&mut rooms, &room_order);
    let raw_logic: RawLogicFile = export_raw_logic(&assets);
    let lowered_logic = lower_raw_logic_file(&raw_logic);

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
            if (tag.starts_with("trigger:") || tag.starts_with("other:user"))
                && seen_event_types.insert(tag.clone())
            {
                warnings.push(format!("runtime-unsupported-event:{}", tag));
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
                    && seen_actions.insert(lower.clone())
                {
                    warnings.push(format!("runtime-unsupported-action:{}", fn_name));
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
    if warnings.len() > normalization_warning_count {
        warnings.push("script-ir-partial".to_string());
    }

    let mut unsupported_features = Vec::new();
    if raw_statement_count > 0 {
        unsupported_features.push("lowered-logic-raw-fallback".into());
    }
    if !dlls.is_empty() {
        unsupported_features.push("external-dll-execution".into());
    }

    let analysis = AnalysisReport {
        dlls: dlls.to_vec(),
        included_files: assets
            .included_files
            .iter()
            .map(|f| f.file_name.to_string())
            .collect(),
        warnings: warnings.clone(),
        unsupported_features,
    };

    let display = manifest_display_metadata(&assets.settings, &room_order, &rooms);

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
        display_source: display.source,
        display_width: display.dimensions.map(|(w, _)| w),
        display_height: display.dimensions.map(|(_, h)| h),
        zero_uninitialized_vars: assets.settings.zero_uninitialized_vars,
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

fn normalize_missing_background_references(
    rooms: &mut [crate::models::RoomDefinition],
    exported_background_ids: &std::collections::HashSet<usize>,
    warnings: &mut Vec<String>,
) {
    for room in rooms {
        for background in &mut room.backgrounds {
            let Ok(source_bg) = usize::try_from(background.source_bg) else {
                continue;
            };
            if background.visible_on_start && !exported_background_ids.contains(&source_bg) {
                warnings.push(format!(
                    "normalized-missing-room-background:{}:{}",
                    room.id, background.source_bg
                ));
                background.source_bg = -1;
            }
        }

        for tile in &mut room.tiles {
            let Ok(source_bg) = usize::try_from(tile.source_bg) else {
                continue;
            };
            if !exported_background_ids.contains(&source_bg) {
                warnings.push(format!(
                    "normalized-missing-room-tile-background:{}:{}:{}",
                    room.id, tile.tile_id, tile.source_bg
                ));
                tile.source_bg = -1;
            }
        }
    }
}

// Manifest display metadata source and dimensions.
//
/// Index 0 = No Change → returns None.
/// Indices 1-6 map to standard resolutions defined in gm8exe settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ManifestDisplayMetadata {
    source: Option<RuntimeDisplaySource>,
    dimensions: Option<(u32, u32)>,
}

fn manifest_display_metadata(
    settings: &gm8exe::settings::Settings,
    room_order: &[usize],
    rooms: &[crate::models::RoomDefinition],
) -> ManifestDisplayMetadata {
    if settings.set_resolution {
        if let Some(dimensions) = resolution_to_dimensions(settings.resolution) {
            return ManifestDisplayMetadata {
                source: Some(RuntimeDisplaySource::ExeResolution),
                dimensions: Some(dimensions),
            };
        }
    }

    let dimensions = default_room_dimensions(room_order, rooms);
    ManifestDisplayMetadata {
        source: dimensions.map(|_| RuntimeDisplaySource::DefaultRoom),
        dimensions,
    }
}

fn default_room_dimensions(
    room_order: &[usize],
    rooms: &[crate::models::RoomDefinition],
) -> Option<(u32, u32)> {
    let default_id = room_order
        .first()
        .copied()
        .or_else(|| rooms.first().map(|room| room.id))?;
    rooms
        .iter()
        .find(|room| room.id == default_id)
        .map(|room| (room.width, room.height))
}

fn resolution_to_dimensions(index: u32) -> Option<(u32, u32)> {
    match index {
        1 => Some((320, 240)),
        2 => Some((640, 480)),
        3 => Some((800, 600)),
        4 => Some((1024, 768)),
        5 => Some((1280, 1024)),
        6 => Some((1600, 1200)),
        _ => None,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        RoomBackgroundLayer, RoomDefinition, RoomTilePlacement, RuntimeDisplaySource,
    };
    use gm8exe::settings::Settings;

    fn settings_with_resolution(set_resolution: bool, resolution: u32) -> Settings {
        Settings {
            fullscreen: false,
            scaling: 0,
            interpolate_pixels: false,
            clear_colour: 0,
            allow_resize: false,
            window_on_top: false,
            dont_draw_border: false,
            dont_show_buttons: false,
            display_cursor: false,
            freeze_on_lose_focus: false,
            disable_screensaver: false,
            force_cpu_render: false,
            set_resolution,
            colour_depth: 0,
            resolution,
            frequency: 0,
            vsync: false,
            esc_close_game: false,
            treat_close_as_esc: false,
            f1_help_menu: false,
            f4_fullscreen_toggle: false,
            f5_save_f6_load: false,
            f9_screenshot: false,
            priority: 0,
            custom_load_image: None,
            transparent: false,
            translucency: 0,
            loading_bar: 0,
            backdata: None,
            frontdata: None,
            scale_progress_bar: false,
            show_error_messages: false,
            log_errors: false,
            always_abort: false,
            zero_uninitialized_vars: false,
            error_on_uninitialized_args: false,
            swap_creation_events: false,
        }
    }

    fn room(id: usize, width: u32, height: u32) -> RoomDefinition {
        RoomDefinition {
            id,
            name: format!("room{id}"),
            width,
            height,
            speed: 60,
            persistent: false,
            background_colour: 0,
            clear_screen: true,
            backgrounds: vec![],
            views_enabled: false,
            views: vec![],
            tiles: vec![],
            instances: vec![],
            creation_block_id: None,
            playable: false,
            transition_targets: vec![],
        }
    }

    fn background(source_bg: i32, visible_on_start: bool) -> RoomBackgroundLayer {
        RoomBackgroundLayer {
            visible_on_start,
            is_foreground: false,
            source_bg,
            xoffset: 0,
            yoffset: 0,
            tile_horz: false,
            tile_vert: false,
            hspeed: 0,
            vspeed: 0,
            stretch: false,
        }
    }

    fn tile(tile_id: i32, source_bg: i32) -> RoomTilePlacement {
        RoomTilePlacement {
            tile_id,
            source_bg,
            x: 0,
            y: 0,
            tile_x: 0,
            tile_y: 0,
            width: 16,
            height: 16,
            depth: 0,
            xscale: 1.0,
            yscale: 1.0,
            blend: 0,
        }
    }

    #[test]
    fn manifest_display_metadata_uses_exe_resolution_when_enabled() {
        let settings = settings_with_resolution(true, 2);
        let rooms = vec![room(7, 320, 240)];

        let display = manifest_display_metadata(&settings, &[7], &rooms);

        assert_eq!(display.source, Some(RuntimeDisplaySource::ExeResolution));
        assert_eq!(display.dimensions, Some((640, 480)));
    }

    #[test]
    fn manifest_display_metadata_falls_back_to_default_room_dimensions() {
        let settings = settings_with_resolution(false, 0);
        let rooms = vec![room(7, 320, 240), room(9, 800, 600)];

        let display = manifest_display_metadata(&settings, &[9, 7], &rooms);

        assert_eq!(display.source, Some(RuntimeDisplaySource::DefaultRoom));
        assert_eq!(display.dimensions, Some((800, 600)));
    }

    #[test]
    fn background_normalization_preserves_valid_and_hidden_references() {
        let mut rooms = vec![room(7, 320, 240)];
        rooms[0].backgrounds = vec![background(3, true), background(9, false)];
        let exported_background_ids = std::collections::HashSet::from([3]);
        let mut warnings = Vec::new();

        normalize_missing_background_references(
            &mut rooms,
            &exported_background_ids,
            &mut warnings,
        );

        assert_eq!(rooms[0].backgrounds[0].source_bg, 3);
        assert_eq!(rooms[0].backgrounds[1].source_bg, 9);
        assert!(warnings.is_empty());
    }

    #[test]
    fn background_normalization_replaces_missing_visible_reference() {
        let mut rooms = vec![room(65, 320, 240)];
        rooms[0].backgrounds = vec![background(47, true)];
        let mut warnings = Vec::new();

        normalize_missing_background_references(
            &mut rooms,
            &std::collections::HashSet::new(),
            &mut warnings,
        );

        assert_eq!(rooms[0].backgrounds[0].source_bg, -1);
        assert_eq!(warnings, ["normalized-missing-room-background:65:47"]);
    }

    #[test]
    fn background_normalization_replaces_missing_tile_reference() {
        let mut rooms = vec![room(151, 320, 240)];
        rooms[0].tiles = vec![tile(12, 62)];
        let mut warnings = Vec::new();

        normalize_missing_background_references(
            &mut rooms,
            &std::collections::HashSet::new(),
            &mut warnings,
        );

        assert_eq!(rooms[0].tiles[0].source_bg, -1);
        assert_eq!(
            warnings,
            ["normalized-missing-room-tile-background:151:12:62"]
        );
    }
}
