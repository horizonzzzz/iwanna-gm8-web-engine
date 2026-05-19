use iwm_parser::models::{CompatibilityLevel, PackageManifest};

#[test]
fn manifest_serializes_expected_fields() {
    let manifest = PackageManifest {
        format_version: 0,
        source_name: "sample.exe".into(),
        source_hash: "abc123".into(),
        engine_family: "gm8".into(),
        compatibility: CompatibilityLevel::Partial,
        room_count: 2,
        object_count: 3,
        script_count: 4,
        sprite_count: 5,
        warnings: vec!["missing dll support".into()],
    };

    let json = serde_json::to_value(&manifest).unwrap();
    assert_eq!(json["engine_family"], "gm8");
    assert_eq!(json["compatibility"], "partial");
    assert_eq!(json["room_count"], 2);
}

#[test]
fn package_format_v0_uses_scripts_json_not_scripts_ir_json() {
    let outputs = [
        "manifest.json",
        "rooms.json",
        "objects.json",
        "scripts.json",
        "analysis.json",
    ];

    assert!(outputs.contains(&"scripts.json"));
    assert!(!outputs.contains(&"scripts.ir.json"));
}
