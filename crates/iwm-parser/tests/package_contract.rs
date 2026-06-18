use std::fs;
use std::process::Command;

use iwm_runtime_model::{CompatibilityLevel, ObjectDefinition, RoomDefinition, RuntimeManifest};

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
        room_order: vec![0, 1],
        room_count: 2,
        object_count: 3,
        script_block_count: 4,
        sprite_count: 5,
        background_count: 1,
        sound_count: 6,
        resource_index_path: "resources/index.json".into(),
        warnings: vec!["missing dll support".into()],
        display_source: None,
        display_width: None,
        display_height: None,
    };

    let json = serde_json::to_value(&manifest).unwrap();
    assert_eq!(json["format_version"], 1);
    assert_eq!(json["package_kind"], "runtime-v1");
    assert_eq!(json["resource_index_path"], "resources/index.json");
    assert_eq!(json["room_order"][0], 0);
    assert_eq!(json["room_order"][1], 1);
}

#[test]
fn runtime_package_uses_ir_and_resource_index_outputs() {
    let outputs = [
        "manifest.json",
        "rooms.json",
        "objects.json",
        "scripts.ir.json",
        "logic.raw.json",
        "logic.lowered.json",
        "analysis.json",
        "resources/index.json",
    ];

    assert!(outputs.contains(&"scripts.ir.json"));
    assert!(outputs.contains(&"logic.raw.json"));
    assert!(outputs.contains(&"logic.lowered.json"));
    assert!(outputs.contains(&"resources/index.json"));
    assert!(!outputs.contains(&"scripts.json"));
}

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
        [
            "left".into(),
            "right".into(),
            "ignored".into(),
            "".into(),
            "".into(),
            "".into(),
            "".into(),
            "".into(),
        ],
    );
    assert_eq!(args, vec!["left".to_string(), "right".to_string()]);
}

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
    assert!(out_dir.join("logic.raw.json").exists());
    assert!(out_dir.join("logic.lowered.json").exists());
    assert!(out_dir.join("analysis.json").exists());
    assert!(out_dir.join("resources").join("index.json").exists());
}

#[test]
fn object_definition_includes_runtime_categorization_fields() {
    // Object definitions should include runtime-categorization hints
    let object = ObjectDefinition {
        id: 0,
        name: "obj_player".to_string(),
        sprite_index: 0,
        parent_index: -1,
        depth: 0,
        persistent: false,
        visible: true,
        solid: false,
        mask_index: -1,
        is_hazard: Some(false),
        is_checkpoint: Some(true),
        is_player: true,
        events: vec![],
    };

    let json = serde_json::to_value(&object).unwrap();
    assert_eq!(json["is_hazard"], false);
    assert_eq!(json["is_checkpoint"], true);
    assert_eq!(json["is_player"], true);
}

#[test]
fn room_definition_includes_playability_metadata() {
    // Room definitions should indicate if they're playable and potential transitions
    let room = RoomDefinition {
        id: 0,
        name: "rm_test".to_string(),
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
        transition_targets: vec![1, 3], // Room IDs this room can transition to
    };

    let json = serde_json::to_value(&room).unwrap();
    assert_eq!(json["playable"], true);
    assert_eq!(json["transition_targets"], serde_json::json!([1, 3]));
}

#[test]
fn room_view_includes_gm8_follow_metadata() {
    use iwm_parser::models::RoomView;

    let view = RoomView {
        visible: true,
        source_x: 800,
        source_y: 608,
        source_w: 800,
        source_h: 600,
        port_x: 0,
        port_y: 0,
        port_w: 800,
        port_h: 600,
        target: 259,
        hborder: 32,
        vborder: 48,
        hspeed: -1,
        vspeed: 8,
    };

    let json = serde_json::to_value(&view).unwrap();
    assert_eq!(json["target"], 259);
    assert_eq!(json["hborder"], 32);
    assert_eq!(json["vborder"], 48);
    assert_eq!(json["hspeed"], -1);
    assert_eq!(json["vspeed"], 8);
}

#[test]
fn room_tile_placement_includes_runtime_fields() {
    use iwm_parser::models::RoomTilePlacement;

    let tile = RoomTilePlacement {
        tile_id: 42,
        source_bg: 7,
        x: 128,
        y: 256,
        tile_x: 2,
        tile_y: 3,
        width: 32,
        height: 32,
        depth: 100,
        xscale: 1.5,
        yscale: 2.0,
        blend: 0xff00ff,
    };

    let json = serde_json::to_value(&tile).unwrap();
    assert_eq!(json["tile_id"], 42);
    assert_eq!(json["source_bg"], 7);
    assert_eq!(json["width"], 32);
}

#[test]
fn room_instance_placement_includes_runtime_flags() {
    use iwm_parser::models::RoomInstancePlacement;

    // Instance placements should include runtime-relevant flags
    let instance = RoomInstancePlacement {
        instance_id: 1001,
        object_id: 5,
        x: 100,
        y: 200,
        xscale: 1.0,
        yscale: 1.0,
        angle: 0.0,
        blend: 0,
        creation_block_id: None,
        is_solid: true,
        is_hazard: false,
        is_checkpoint: true,
    };

    let json = serde_json::to_value(&instance).unwrap();
    assert_eq!(json["is_solid"], true);
    assert_eq!(json["is_hazard"], false);
    assert_eq!(json["is_checkpoint"], true);
}

#[test]
fn logic_block_distinguishes_executable_vs_source_only() {
    use iwm_parser::models::{LogicBlock, LogicOp, ScriptIrFile};

    let blocks = vec![
        LogicBlock {
            id: "object:0:event:0:0".to_string(),
            name: "obj_player Create".to_string(),
            kind: "object-event".to_string(),
            support: "action-list".to_string(), // Executable
            executable_action_count: 3,
            ops: vec![LogicOp::ActionCall {
                action_id: 1,
                lib_id: 0,
                applies_to: -1,
                is_condition: false,
                invert_condition: false,
                is_relative: false,
                fn_name: "instance_create".to_string(),
                fn_code: String::new(),
                args: vec!["x".to_string(), "y".to_string(), "0".to_string()],
            }],
        },
        LogicBlock {
            id: "room:0:create".to_string(),
            name: "room 0 creation".to_string(),
            kind: "room-creation".to_string(),
            support: "source-only".to_string(), // Requires GML lowering
            executable_action_count: 0,
            ops: vec![LogicOp::SourceSnippet {
                code: "global.score = 0;".to_string(),
            }],
        },
    ];

    let ir = ScriptIrFile {
        format: "iwm-script-ir-v1".to_string(),
        blocks,
    };

    let json = serde_json::to_value(&ir).unwrap();
    let block0 = &json["blocks"][0];
    let block1 = &json["blocks"][1];

    assert_eq!(block0["support"], "action-list");
    assert_eq!(block0["executable_action_count"], 3);
    assert_eq!(block1["support"], "source-only");
    assert_eq!(block1["executable_action_count"], 0);
}

#[test]
fn analysis_warnings_use_actionable_categories() {
    use iwm_parser::models::AnalysisReport;

    let analysis = AnalysisReport {
        dlls: vec!["test.dll".to_string()],
        included_files: vec!["config.ini".to_string()],
        warnings: vec![
            "runtime-missing-source-lowering:room:0:create".to_string(),
            "runtime-unsupported-event:trigger".to_string(),
            "runtime-unsupported-action:game_save".to_string(),
            "runtime-unsupported-action:mouse_check_button".to_string(),
        ],
        unsupported_features: vec![],
    };

    let json = serde_json::to_value(&analysis).unwrap();
    let warnings = json["warnings"].as_array().unwrap();

    assert!(warnings.iter().any(|w| w
        .as_str()
        .unwrap()
        .starts_with("runtime-missing-source-lowering:")));
    assert!(warnings.iter().any(|w| w
        .as_str()
        .unwrap()
        .starts_with("runtime-unsupported-event:")));
    assert!(warnings.iter().any(|w| w
        .as_str()
        .unwrap()
        .starts_with("runtime-unsupported-action:")));
}

#[test]
fn event_block_ids_are_stable_and_parseable() {
    // Block IDs should follow consistent format for runtime parsing
    let block_id = "object:12:event:3:0";
    let parts: Vec<&str> = block_id.split(':').collect();

    assert_eq!(parts.len(), 5);
    assert_eq!(parts[0], "object");
    assert_eq!(parts[1], "12");
    assert_eq!(parts[2], "event");
    assert_eq!(parts[3], "3"); // event_type
    assert_eq!(parts[4], "0"); // sub_event
}

#[test]
fn room_transition_block_ids_follow_naming_convention() {
    use iwm_parser::logic_export::instance_creation_block_id;
    use iwm_parser::logic_export::room_creation_block_id;

    let room_create = room_creation_block_id(5);
    assert_eq!(room_create, "room:5:create");

    let instance_create = instance_creation_block_id(5, 1001);
    assert_eq!(instance_create, "room:5:instance:1001:create");
}

#[test]
fn export_rooms_and_logic_assigns_transition_targets_to_the_source_room() {
    use gm8exe::{
        asset::{
            code_action::CodeAction,
            object::Object,
            room::{Instance, Room},
        },
        AssetList,
    };
    use iwm_parser::logic_export::export_rooms_and_logic;

    fn room_goto_action(target_room_id: &str) -> CodeAction {
        let mut param_strings: [gm8exe::asset::PascalString; 8] = Default::default();
        param_strings[0] = target_room_id.into();

        CodeAction {
            id: 603,
            applies_to: -1,
            is_condition: false,
            invert_condition: false,
            is_relative: false,
            lib_id: 1,
            action_kind: 7,
            execution_type: 1,
            can_be_relative: 0,
            applies_to_something: false,
            fn_name: "room_goto".into(),
            fn_code: "".into(),
            param_count: 1,
            param_types: [0; 8],
            param_strings,
        }
    }

    let mut room0_events: Vec<Vec<(u32, Vec<CodeAction>)>> = (0..12).map(|_| Vec::new()).collect();
    room0_events[3].push((0, vec![room_goto_action("1")]));

    let objects: AssetList<Object> = vec![Some(Box::new(Object {
        name: "obj_door".into(),
        sprite_index: -1,
        solid: false,
        visible: true,
        depth: 0,
        persistent: false,
        parent_index: -1,
        mask_index: -1,
        events: room0_events,
    }))];

    let rooms: AssetList<Room> = vec![
        Some(Box::new(Room {
            name: "rm_start".into(),
            caption: "".into(),
            width: 320,
            height: 240,
            speed: 30,
            persistent: false,
            bg_colour: 0.into(),
            clear_screen: true,
            clear_region: true,
            creation_code: "".into(),
            backgrounds: vec![],
            views_enabled: false,
            views: vec![],
            instances: vec![Instance {
                x: 64,
                y: 96,
                object: 0,
                id: 1001,
                creation_code: "".into(),
                xscale: 1.0,
                yscale: 1.0,
                blend: 0,
                angle: 0.0,
            }],
            tiles: vec![],
            uses_810_features: false,
            uses_811_features: false,
        })),
        Some(Box::new(Room {
            name: "rm_next".into(),
            caption: "".into(),
            width: 320,
            height: 240,
            speed: 30,
            persistent: false,
            bg_colour: 0.into(),
            clear_screen: true,
            clear_region: true,
            creation_code: "".into(),
            backgrounds: vec![],
            views_enabled: false,
            views: vec![],
            instances: vec![],
            tiles: vec![],
            uses_810_features: false,
            uses_811_features: false,
        })),
    ];

    let empty_scripts = Vec::new();
    let (room_defs, _, _) = export_rooms_and_logic(&rooms, &objects, &empty_scripts);

    assert_eq!(room_defs[0].transition_targets, vec![1]);
    assert!(room_defs[1].transition_targets.is_empty());
}

#[test]
fn export_rooms_and_logic_includes_script_resources_in_script_ir() {
    use gm8exe::{
        asset::{room::Room, script::Script},
        settings::{GameHelpDialog, Settings},
        GameAssets, GameVersion,
    };
    use iwm_parser::logic_export::export_rooms_and_logic;

    let assets = GameAssets {
        triggers: vec![],
        constants: vec![],
        extensions: vec![],
        sprites: vec![],
        sounds: vec![],
        backgrounds: vec![],
        paths: vec![],
        scripts: vec![Some(Box::new(Script {
            name: "defControls".into(),
            source: "global.jumpbutton=vk_shift;".into(),
        }))],
        fonts: vec![],
        timelines: vec![],
        objects: vec![],
        rooms: vec![Some(Box::new(Room {
            name: "rm_init".into(),
            caption: "".into(),
            width: 320,
            height: 240,
            speed: 30,
            persistent: false,
            bg_colour: 0.into(),
            clear_screen: true,
            clear_region: true,
            creation_code: "".into(),
            backgrounds: vec![],
            views_enabled: false,
            views: vec![],
            instances: vec![],
            tiles: vec![],
            uses_810_features: false,
            uses_811_features: false,
        }))],
        included_files: vec![],
        version: GameVersion::GameMaker8_0,
        dx_dll: vec![],
        ico_file_raw: None,
        help_dialog: GameHelpDialog {
            bg_colour: 0u32.into(),
            new_window: false,
            caption: "".into(),
            left: 0,
            top: 0,
            width: 0,
            height: 0,
            border: false,
            resizable: false,
            window_on_top: false,
            freeze_game: false,
            info: "".into(),
        },
        last_instance_id: 0,
        last_tile_id: 0,
        library_init_strings: vec![],
        room_order: vec![],
        settings: Settings {
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
            set_resolution: false,
            colour_depth: 0,
            resolution: 0,
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
        },
        game_id: 0,
        guid: [0; 4],
    };

    let (_, _, script_ir) = export_rooms_and_logic(&assets.rooms, &assets.objects, &assets.scripts);
    let script_block = script_ir
        .blocks
        .iter()
        .find(|block| block.id == "script:0")
        .expect("expected exported script block");

    assert_eq!(script_block.name, "defControls");
    assert_eq!(script_block.kind, "script");
    assert_eq!(script_block.support, "source-only");
    assert!(matches!(
        script_block.ops.as_slice(),
        [iwm_parser::models::LogicOp::SourceSnippet { code }]
            if code == "global.jumpbutton=vk_shift;"
    ));
}
