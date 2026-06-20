use iwm_runtime_core::RuntimePackage;
use iwm_runtime_model::{
    AnalysisReport, BackgroundResource, CompatibilityLevel, LogicBlock, LogicOp, ObjectDefinition,
    ObjectEventEntry, ResourceIndex, RoomBackgroundLayer, RoomDefinition, RoomInstancePlacement,
    RoomTilePlacement, RoomView, RuntimeManifest, ScriptIrFile, SoundResource, SpriteResource,
};

pub fn sample_package() -> RuntimePackage {
    RuntimePackage {
        manifest: sample_manifest(),
        rooms: vec![primary_room(), secondary_room()],
        objects: vec![player_object(), block_object()],
        scripts: sample_scripts(),
        lowered_logic: None,
        resources: sample_resources(),
        analysis: empty_analysis(),
    }
}

fn sample_manifest() -> RuntimeManifest {
    RuntimeManifest {
        format_version: 1,
        package_kind: "runtime-v1".into(),
        source_name: "sample.exe".into(),
        source_hash: "abc123".into(),
        engine_family: "gm8".into(),
        compatibility: CompatibilityLevel::Partial,
        default_room_id: Some(0),
        room_order: vec![0, 1],
        room_count: 2,
        object_count: 2,
        script_block_count: 1,
        sprite_count: 0,
        background_count: 0,
        sound_count: 0,
        resource_index_path: "resources/index.json".into(),
        warnings: vec![],
        display_source: None,
        display_width: None,
        display_height: None,
    }
}

fn primary_room() -> RoomDefinition {
    RoomDefinition {
        id: 0,
        name: "room0".into(),
        width: 320,
        height: 240,
        speed: 60,
        persistent: false,
        backgrounds: vec![visible_background_layer(0)],
        views_enabled: false,
        views: vec![full_room_view(320, 240)],
        tiles: vec![RoomTilePlacement {
            tile_id: 10,
            source_bg: 0,
            x: 16,
            y: 32,
            tile_x: 4,
            tile_y: 8,
            width: 16,
            height: 16,
            depth: 0,
            xscale: 1.0,
            yscale: 1.0,
            blend: 0x00ff_ffff,
        }],
        instances: vec![
            room_instance(1, 0, 32, 64, false),
            room_instance(2, 1, 32, 80, true),
        ],
        creation_block_id: None,
        playable: true,
        transition_targets: vec![],
    }
}

fn secondary_room() -> RoomDefinition {
    RoomDefinition {
        id: 1,
        name: "room1".into(),
        width: 640,
        height: 480,
        speed: 30,
        persistent: false,
        backgrounds: vec![],
        views_enabled: false,
        views: vec![],
        tiles: vec![],
        instances: vec![],
        creation_block_id: None,
        playable: true,
        transition_targets: vec![],
    }
}

fn visible_background_layer(source_bg: i32) -> RoomBackgroundLayer {
    RoomBackgroundLayer {
        visible_on_start: true,
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

fn full_room_view(width: u32, height: u32) -> RoomView {
    RoomView {
        visible: true,
        source_x: 0,
        source_y: 0,
        source_w: width,
        source_h: height,
        port_x: 0,
        port_y: 0,
        port_w: width,
        port_h: height,
        target: -1,
        hborder: 32,
        vborder: 32,
        hspeed: -1,
        vspeed: -1,
    }
}

fn room_instance(
    instance_id: i32,
    object_id: i32,
    x: i32,
    y: i32,
    is_solid: bool,
) -> RoomInstancePlacement {
    RoomInstancePlacement {
        instance_id,
        object_id,
        x,
        y,
        xscale: 1.0,
        yscale: 1.0,
        angle: 0.0,
        blend: 0x00ff_ffff,
        creation_block_id: None,
        is_solid,
        is_hazard: false,
        is_checkpoint: false,
    }
}

fn player_object() -> ObjectDefinition {
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
        events: vec![object_event(0, 0, "create", "object:0:event:0:0")],
    }
}

fn block_object() -> ObjectDefinition {
    ObjectDefinition {
        id: 1,
        name: "obj_block".into(),
        sprite_index: -1,
        parent_index: -1,
        depth: 0,
        persistent: false,
        visible: false,
        solid: true,
        mask_index: -1,
        is_hazard: Some(false),
        is_checkpoint: Some(false),
        is_player: false,
        events: vec![],
    }
}

fn object_event(
    event_type: usize,
    sub_event: u32,
    event_tag: &str,
    block_id: &str,
) -> ObjectEventEntry {
    ObjectEventEntry {
        event_type,
        sub_event,
        event_tag: event_tag.into(),
        block_id: block_id.into(),
        action_count: 0,
    }
}

fn sample_scripts() -> ScriptIrFile {
    ScriptIrFile {
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
    }
}

fn sample_resources() -> ResourceIndex {
    ResourceIndex {
        sprites: vec![SpriteResource {
            id: 0,
            name: "spr_player".into(),
            origin_x: 0,
            origin_y: 0,
            frame_paths: vec!["resources/sprites/0-0.png".into()],
            width: 16,
            height: 16,
            bbox_left: 0,
            bbox_right: 15,
            bbox_top: 0,
            bbox_bottom: 15,
            collision_masks: vec![],
            per_frame_collision_masks: false,
        }],
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
        fonts: vec![],
    }
}

fn empty_analysis() -> AnalysisReport {
    AnalysisReport {
        dlls: vec![],
        included_files: vec![],
        warnings: vec![],
        unsupported_features: vec![],
    }
}
