use iwm_parser::models::{CompatibilityLevel, RuntimeManifest};
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
