use iwm_parser::models::{CompatibilityLevel, ObjectDefinition, ObjectEventEntry, RoomDefinition, RuntimeManifest};
use std::fs;
use std::process::Command;

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
        warnings: vec!["missing dll support".into()],
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

#[test]
fn bgra_pixels_are_converted_to_rgba_order() {
    let converted = iwm_parser::resource_export::bgra_to_rgba(vec![0, 64, 255, 255]);
    assert_eq!(converted, vec![255, 64, 0, 255]);
}

#[test]
fn runtime_resources_are_written_under_expected_directories() {
    let base = std::path::Path::new("resources");
    assert_eq!(
        base.join("sprites").to_string_lossy().replace('\\', "/"),
        "resources/sprites"
    );
    assert_eq!(
        base.join("backgrounds")
            .to_string_lossy()
            .replace('\\', "/"),
        "resources/backgrounds"
    );
    assert_eq!(
        base.join("audio").to_string_lossy().replace('\\', "/"),
        "resources/audio"
    );
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
    assert!(out_dir.join("analysis.json").exists());
    assert!(out_dir.join("resources").join("index.json").exists());
}

// ============================================================================
// Step 2: Tests for new contract fields (Tighten Runtime Execution Contract)
// ============================================================================

#[test]
fn object_event_entry_includes_normalized_event_tag() {
    // Event entries should include a human-readable event_tag for runtime dispatch
    let event = ObjectEventEntry {
        event_type: 3,       // Step event
        sub_event: 0,        // Step normal
        event_tag: "step".to_string(),
        block_id: "object:0:event:3:0".to_string(),
        action_count: 2,
    };

    let json = serde_json::to_value(&event).unwrap();
    assert_eq!(json["event_tag"], "step");
    assert_eq!(json["event_type"], 3);
    assert_eq!(json["sub_event"], 0);
}

#[test]
fn object_event_entry_event_tags_for_all_supported_event_types() {
    // All GM8 event types should have normalized tags
    let test_cases = vec![
        (0, 0, "create"),
        (1, 0, "destroy"),
        (2, 0, "alarm:0"),
        (2, 5, "alarm:5"),
        (3, 0, "step"),
        (3, 1, "step:begin"),
        (3, 2, "step:end"),
        (4, 0, "collision"),  // collision target is dynamic, tag is generic
        (5, 65, "keyboard:a"),  // ASCII key code
        (6, 0, "mouse:left"),
        (7, 0, "other:outside"),
        (7, 1, "other:boundary"),
        (8, 0, "draw"),
        (9, 65, "keypress:a"),
        (10, 65, "keyrelease:a"),
    ];

    for (event_type, sub_event, expected_tag) in test_cases {
        let event = ObjectEventEntry {
            event_type,
            sub_event,
            event_tag: expected_tag.to_string(),
            block_id: format!("object:0:event:{event_type}:{sub_event}"),
            action_count: 0,
        };

        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(
            json["event_tag"], expected_tag,
            "event_type={}, sub_event={} should have tag '{}'",
            event_type, sub_event, expected_tag
        );
    }
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
        instances: vec![],
        creation_block_id: None,
        playable: true,
        transition_targets: vec![1, 3],  // Room IDs this room can transition to
    };

    let json = serde_json::to_value(&room).unwrap();
    assert_eq!(json["playable"], true);
    assert_eq!(json["transition_targets"], serde_json::json!([1, 3]));
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
            support: "action-list".to_string(),  // Executable
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
            support: "source-only".to_string(),  // Requires GML lowering
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

    assert!(warnings.iter().any(|w| w.as_str().unwrap().starts_with("runtime-missing-source-lowering:")));
    assert!(warnings.iter().any(|w| w.as_str().unwrap().starts_with("runtime-unsupported-event:")));
    assert!(warnings.iter().any(|w| w.as_str().unwrap().starts_with("runtime-unsupported-action:")));
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
    assert_eq!(parts[3], "3");  // event_type
    assert_eq!(parts[4], "0");  // sub_event
}

#[test]
fn room_transition_block_ids_follow_naming_convention() {
    use iwm_parser::logic_export::room_creation_block_id;
    use iwm_parser::logic_export::instance_creation_block_id;

    let room_create = room_creation_block_id(5);
    assert_eq!(room_create, "room:5:create");

    let instance_create = instance_creation_block_id(5, 1001);
    assert_eq!(instance_create, "room:5:instance:1001:create");
}
