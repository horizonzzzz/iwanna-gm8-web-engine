use iwm_runtime_host::HeadlessHost;
use iwm_runtime_model::{
    AnalysisReport, BackgroundResource, CompatibilityLevel, LogicBlock, LogicOp, ObjectDefinition,
    ObjectEventEntry, ResourceIndex, RoomBackgroundLayer, RoomDefinition, RoomInstancePlacement,
    RoomTilePlacement, RoomView, RuntimeManifest, ScriptIrFile, SoundResource, SpriteResource,
};
use std::path::Path;

use crate::helpers::collides_at;
use crate::{
    LoweredLogicEntry, LoweredLogicFile, LoweredLogicStatement, RuntimeCore, RuntimePackage,
};

const PLAYER_OBJECT_INDEX: usize = 0;
const PLAYER_OBJECT_ID: usize = 0;
const DEFAULT_ROOM_ID: usize = 7;
const DEFAULT_ROOM_CREATE_BLOCK_ID: &str = "room:7:create";

#[derive(Debug, Clone, PartialEq)]
pub(super) struct JumpTraceFrame {
    pub tick: u64,
    pub x: f64,
    pub y: f64,
    pub hspeed: f64,
    pub vspeed: f64,
    pub grounded: bool,
    pub jump_active: bool,
    pub jump_hold_frames: u32,
    pub jump_cut_applied: bool,
}

pub(super) fn sample_package() -> RuntimePackage {
    RuntimePackage {
        manifest: RuntimeManifest {
            format_version: 1,
            package_kind: "runtime-v1".into(),
            source_name: "sample.exe".into(),
            source_hash: "abc123".into(),
            engine_family: "gm8".into(),
            compatibility: CompatibilityLevel::Partial,
            default_room_id: Some(7),
            room_order: vec![7, 9],
            room_count: 2,
            object_count: 5,
            script_block_count: 1,
            sprite_count: 2,
            background_count: 1,
            sound_count: 0,
            resource_index_path: "resources/index.json".into(),
            warnings: vec![],
            display_source: None,
            display_width: None,
            display_height: None,
        },
        rooms: vec![
            RoomDefinition {
                id: 7,
                name: "room7".into(),
                width: 320,
                height: 240,
                speed: 60,
                persistent: false,
                backgrounds: vec![RoomBackgroundLayer {
                    visible_on_start: true,
                    is_foreground: false,
                    source_bg: 0,
                    xoffset: 0,
                    yoffset: 0,
                    tile_horz: false,
                    tile_vert: false,
                    hspeed: 0,
                    vspeed: 0,
                    stretch: false,
                }],
                views_enabled: false,
                views: vec![RoomView {
                    visible: true,
                    source_x: 0,
                    source_y: 0,
                    source_w: 320,
                    source_h: 240,
                    port_x: 0,
                    port_y: 0,
                    port_w: 320,
                    port_h: 240,
                    target: -1,
                    hborder: 32,
                    vborder: 32,
                    hspeed: -1,
                    vspeed: -1,
                }],
                tiles: vec![RoomTilePlacement {
                    tile_id: 21,
                    source_bg: 0,
                    x: 64,
                    y: 80,
                    tile_x: 0,
                    tile_y: 0,
                    width: 32,
                    height: 32,
                    depth: 100,
                    xscale: 1.0,
                    yscale: 1.0,
                    blend: 0x00ff_ffff,
                }],
                instances: vec![
                    RoomInstancePlacement {
                        instance_id: 11,
                        object_id: 0,
                        x: 12,
                        y: 24,
                        xscale: 1.0,
                        yscale: 1.0,
                        angle: 0.0,
                        blend: 0x00ff_ffff,
                        creation_block_id: None,
                        is_solid: false,
                        is_hazard: false,
                        is_checkpoint: false,
                    },
                    RoomInstancePlacement {
                        instance_id: 12,
                        object_id: 1,
                        x: 48,
                        y: 64,
                        xscale: 1.0,
                        yscale: 1.0,
                        angle: 0.0,
                        blend: 0x00ff_ffff,
                        creation_block_id: None,
                        is_solid: false,
                        is_hazard: false,
                        is_checkpoint: false,
                    },
                    RoomInstancePlacement {
                        instance_id: 13,
                        object_id: 2,
                        x: 12,
                        y: 40,
                        xscale: 1.0,
                        yscale: 1.0,
                        angle: 0.0,
                        blend: 0x00ff_ffff,
                        creation_block_id: None,
                        is_solid: true,
                        is_hazard: false,
                        is_checkpoint: false,
                    },
                    RoomInstancePlacement {
                        instance_id: 14,
                        object_id: 3,
                        x: 12,
                        y: 24,
                        xscale: 1.0,
                        yscale: 1.0,
                        angle: 0.0,
                        blend: 0x00ff_ffff,
                        creation_block_id: None,
                        is_solid: false,
                        is_hazard: false,
                        is_checkpoint: true,
                    },
                    RoomInstancePlacement {
                        instance_id: 15,
                        object_id: 705,
                        x: 96,
                        y: 96,
                        xscale: 1.0,
                        yscale: 1.0,
                        angle: 0.0,
                        blend: 0x00ff_ffff,
                        creation_block_id: None,
                        is_solid: false,
                        is_hazard: false,
                        is_checkpoint: false,
                    },
                ],
                creation_block_id: None,
                playable: true,
                transition_targets: vec![9],
            },
            RoomDefinition {
                id: 9,
                name: "room9".into(),
                width: 160,
                height: 120,
                speed: 60,
                persistent: false,
                backgrounds: vec![],
                views_enabled: false,
                views: vec![],
                tiles: vec![],
                instances: vec![],
                creation_block_id: None,
                playable: true,
                transition_targets: vec![],
            },
        ],
        objects: vec![
            ObjectDefinition {
                id: 0,
                name: "obj_player".into(),
                sprite_index: 0,
                parent_index: -1,
                depth: 0,
                persistent: false,
                visible: true,
                solid: false,
                mask_index: -1,
                is_hazard: Some(false),
                is_checkpoint: Some(false),
                is_player: true,
                events: vec![ObjectEventEntry {
                    event_type: 0,
                    sub_event: 0,
                    event_tag: "create".into(),
                    block_id: "object:0:event:0:0".into(),
                    action_count: 0,
                }],
            },
            ObjectDefinition {
                id: 1,
                name: "obj_marker".into(),
                sprite_index: -1,
                parent_index: -1,
                depth: 0,
                persistent: false,
                visible: true,
                solid: false,
                mask_index: -1,
                is_hazard: Some(false),
                is_checkpoint: Some(false),
                is_player: false,
                events: vec![],
            },
            ObjectDefinition {
                id: 2,
                name: "obj_block".into(),
                sprite_index: -1,
                parent_index: -1,
                depth: 0,
                persistent: false,
                visible: false,
                solid: true,
                mask_index: -1,
                is_hazard: Some(false),
                is_checkpoint: Some(true),
                is_player: false,
                events: vec![],
            },
            ObjectDefinition {
                id: 3,
                name: "obj_checkpoint".into(),
                sprite_index: -1,
                parent_index: -1,
                depth: 0,
                persistent: false,
                visible: false,
                solid: false,
                mask_index: -1,
                is_hazard: Some(false),
                is_checkpoint: Some(true),
                is_player: false,
                events: vec![],
            },
            ObjectDefinition {
                id: 705,
                name: "obj_sparse_sprite".into(),
                sprite_index: 1,
                parent_index: -1,
                depth: 0,
                persistent: false,
                visible: true,
                solid: false,
                mask_index: -1,
                is_hazard: Some(false),
                is_checkpoint: Some(false),
                is_player: false,
                events: vec![],
            },
        ],
        scripts: ScriptIrFile {
            format: "iwm-script-ir-v1".into(),
            blocks: vec![LogicBlock {
                id: "object:0:event:0:0".into(),
                name: "object event".into(),
                kind: "object-event".into(),
                support: "source-only".into(),
                executable_action_count: 0,
                ops: vec![LogicOp::Unsupported {
                    reason: "placeholder".into(),
                }],
            }],
        },
        lowered_logic: None,
        resources: ResourceIndex {
            sprites: vec![
                SpriteResource {
                    id: 0,
                    name: "spr_player".into(),
                    origin_x: 0,
                    origin_y: 0,
                    frame_paths: vec![],
                    width: 16,
                    height: 16,
                    bbox_left: 0,
                    bbox_right: 15,
                    bbox_top: 0,
                    bbox_bottom: 15,
                    collision_masks: vec![],
                    per_frame_collision_masks: false,
                },
                SpriteResource {
                    id: 1,
                    name: "spr_sparse".into(),
                    origin_x: 0,
                    origin_y: 0,
                    frame_paths: vec![],
                    width: 16,
                    height: 16,
                    bbox_left: 0,
                    bbox_right: 15,
                    bbox_top: 0,
                    bbox_bottom: 15,
                    collision_masks: vec![],
                    per_frame_collision_masks: false,
                },
            ],
            backgrounds: vec![BackgroundResource {
                id: 0,
                name: "bg_room".into(),
                width: 320,
                height: 240,
                image_path: "resources/backgrounds/0.png".into(),
            }],
            sounds: vec![SoundResource {
                id: 0,
                name: "snd_beep".into(),
                file_path: "resources/audio/0.wav".into(),
                extension: "wav".into(),
                preload: false,
            }],
        },
        analysis: AnalysisReport {
            dlls: vec![],
            included_files: vec![],
            warnings: vec![],
            unsupported_features: vec![],
        },
    }
}

pub(super) fn host() -> HeadlessHost {
    HeadlessHost::new("sandbox")
}

pub(super) fn real_sample_package() -> Option<RuntimePackage> {
    let package_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("runtime")
        .join("public")
        .join("packages")
        .join("sample");

    let manifest_path = package_root.join("manifest.json");
    if !manifest_path.exists() {
        return None;
    }

    let manifest = serde_json::from_slice(&std::fs::read(manifest_path).ok()?).ok()?;
    let rooms =
        serde_json::from_slice(&std::fs::read(package_root.join("rooms.json")).ok()?).ok()?;
    let objects =
        serde_json::from_slice(&std::fs::read(package_root.join("objects.json")).ok()?).ok()?;
    let scripts =
        serde_json::from_slice(&std::fs::read(package_root.join("scripts.ir.json")).ok()?).ok()?;
    let analysis =
        serde_json::from_slice(&std::fs::read(package_root.join("analysis.json")).ok()?).ok()?;
    let resources = serde_json::from_slice(
        &std::fs::read(package_root.join("resources").join("index.json")).ok()?,
    )
    .ok()?;
    let lowered_logic =
        serde_json::from_slice(&std::fs::read(package_root.join("logic.lowered.json")).ok()?)
            .ok()?;

    Some(RuntimePackage {
        manifest,
        rooms,
        objects,
        scripts,
        lowered_logic: Some(lowered_logic),
        resources,
        analysis,
    })
}

pub(super) fn capture_jump_trace(core: &RuntimeCore) -> JumpTraceFrame {
    let room = core.current_room().unwrap();
    let player = room
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    let solids = room
        .instances
        .iter()
        .filter(|instance| instance.alive && instance.solid)
        .cloned()
        .collect::<Vec<_>>();

    JumpTraceFrame {
        tick: core.tick_count(),
        x: player.x,
        y: player.y,
        hspeed: player.hspeed,
        vspeed: player.vspeed,
        grounded: collides_at(
            player,
            player.x,
            player.y + 1.0,
            &solids,
            Some(player.runtime_id),
        ),
        jump_active: player.jump.active,
        jump_hold_frames: player.jump.hold_frames,
        jump_cut_applied: player.jump.cut_applied,
    }
}

pub(super) fn add_step_block(package: &mut RuntimePackage, statements: Vec<LoweredLogicStatement>) {
    add_player_event_block(
        package,
        3,
        0,
        "step".into(),
        player_block_id(3, 0),
        statements,
    );
}

pub(super) fn add_destroy_block(
    package: &mut RuntimePackage,
    statements: Vec<LoweredLogicStatement>,
) {
    add_player_event_block(
        package,
        1,
        0,
        "destroy".into(),
        player_block_id(1, 0),
        statements,
    );
}

pub(super) fn add_room_create_block(
    package: &mut RuntimePackage,
    statements: Vec<LoweredLogicStatement>,
) {
    package.rooms[0].creation_block_id = Some(DEFAULT_ROOM_CREATE_BLOCK_ID.into());
    append_lowered_entry(package, DEFAULT_ROOM_CREATE_BLOCK_ID.into(), statements);
}

pub(super) fn append_lowered_entry(
    package: &mut RuntimePackage,
    block_id: String,
    statements: Vec<LoweredLogicStatement>,
) {
    if package.lowered_logic.is_none() {
        package.lowered_logic = Some(LoweredLogicFile {
            format: "iwm-lowered-logic-v1".into(),
            entries: vec![],
        });
    }
    if let Some(ref mut lowered) = package.lowered_logic {
        lowered.entries.push(LoweredLogicEntry {
            block_id,
            statements,
        });
    }
}

pub(super) fn add_script_block(
    package: &mut RuntimePackage,
    script_id: usize,
    script_name: &str,
    statements: Vec<LoweredLogicStatement>,
) {
    package.scripts.blocks.push(LogicBlock {
        id: format!("script:{script_id}"),
        name: script_name.to_string(),
        kind: "script".into(),
        support: "source-only".into(),
        executable_action_count: 0,
        ops: vec![LogicOp::SourceSnippet {
            code: script_name.to_string(),
        }],
    });
    append_lowered_entry(package, format!("script:{script_id}"), statements);
}

pub(super) fn add_keyboard_block(
    package: &mut RuntimePackage,
    key: u8,
    statements: Vec<LoweredLogicStatement>,
) {
    add_player_event_block(
        package,
        5,
        key as u32,
        format!("keyboard:{}", key_name(key)),
        player_block_id(5, key as u32),
        statements,
    );
}

pub(super) fn add_keyboard_press_block(
    package: &mut RuntimePackage,
    key: u8,
    statements: Vec<LoweredLogicStatement>,
) {
    add_player_event_block(
        package,
        9,
        key as u32,
        format!("keypress:{}", key_name(key)),
        player_block_id(9, key as u32),
        statements,
    );
}

pub(super) fn add_keyboard_release_block(
    package: &mut RuntimePackage,
    key: u8,
    statements: Vec<LoweredLogicStatement>,
) {
    add_player_event_block(
        package,
        10,
        key as u32,
        format!("keyrelease:{}", key_name(key)),
        player_block_id(10, key as u32),
        statements,
    );
}

pub(super) fn add_collision_block(
    package: &mut RuntimePackage,
    target_object_id: usize,
    statements: Vec<LoweredLogicStatement>,
) {
    add_player_event_block(
        package,
        4,
        target_object_id as u32,
        "collision".into(),
        player_block_id(4, target_object_id as u32),
        statements,
    );
}

pub(super) fn add_alarm_block(
    package: &mut RuntimePackage,
    slot: u32,
    statements: Vec<LoweredLogicStatement>,
) {
    add_player_event_block(
        package,
        2,
        slot,
        format!("alarm:{slot}"),
        player_block_id(2, slot),
        statements,
    );
}

pub(super) fn add_create_block(
    package: &mut RuntimePackage,
    statements: Vec<LoweredLogicStatement>,
) {
    add_player_event_block(
        package,
        0,
        0,
        "create".into(),
        player_block_id(0, 0),
        statements,
    );
}

fn add_player_event_block(
    package: &mut RuntimePackage,
    event_type: usize,
    sub_event: u32,
    event_tag: String,
    block_id: String,
    statements: Vec<LoweredLogicStatement>,
) {
    package.objects[PLAYER_OBJECT_INDEX]
        .events
        .push(ObjectEventEntry {
            event_type,
            sub_event,
            event_tag,
            block_id: block_id.clone(),
            action_count: 0,
        });
    append_lowered_entry(package, block_id, statements);
}

fn player_block_id(event_type: usize, sub_event: u32) -> String {
    format!("object:{PLAYER_OBJECT_ID}:event:{event_type}:{sub_event}")
}

fn key_name(key: u8) -> String {
    if (key as char).is_ascii_alphanumeric() {
        (key as char).to_ascii_lowercase().to_string()
    } else {
        format!("0x{key:02x}")
    }
}
