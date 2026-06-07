use std::path::Path;

use iwm_runtime_core::RuntimePackage;
use iwm_runtime_model::{
    AnalysisReport, ObjectDefinition, ResourceIndex, RoomDefinition, RuntimeManifest, ScriptIrFile,
};
use iwm_runtime_web::{BridgeDrawCommand, WebRuntimeHost};

#[test]
fn real_mashikaku_package_emits_tile_commands_when_local_package_exists() {
    let package_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("runtime")
        .join("public")
        .join("packages")
        .join("mashikaku");

    let rooms_path = package_root.join("rooms.json");
    if !rooms_path.exists() {
        return;
    }

    let manifest: RuntimeManifest =
        serde_json::from_slice(&std::fs::read(package_root.join("manifest.json")).unwrap())
            .unwrap();
    let rooms: Vec<RoomDefinition> =
        serde_json::from_slice(&std::fs::read(&rooms_path).unwrap()).unwrap();
    let objects: Vec<ObjectDefinition> =
        serde_json::from_slice(&std::fs::read(package_root.join("objects.json")).unwrap()).unwrap();
    let scripts: ScriptIrFile =
        serde_json::from_slice(&std::fs::read(package_root.join("scripts.ir.json")).unwrap())
            .unwrap();
    let analysis: AnalysisReport =
        serde_json::from_slice(&std::fs::read(package_root.join("analysis.json")).unwrap())
            .unwrap();
    let resources: ResourceIndex = serde_json::from_slice(
        &std::fs::read(package_root.join("resources").join("index.json")).unwrap(),
    )
    .unwrap();

    let package = RuntimePackage {
        manifest,
        rooms,
        objects,
        scripts,
        lowered_logic: None,
        resources,
        analysis,
    };

    let mut host = WebRuntimeHost::new();
    host.boot(package).unwrap();
    host.select_room(87).unwrap();
    let frame = host.frame_snapshot().unwrap();

    assert!(frame
        .commands
        .iter()
        .any(|command| matches!(command, BridgeDrawCommand::DrawTile { .. })));
}

#[test]
fn gold_sample_package_has_lowered_logic_file() {
    let package_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("runtime")
        .join("public")
        .join("packages")
        .join("sample");

    // Verify the sample package has lowered logic file
    let lowered_path = package_root.join("logic.lowered.json");
    assert!(
        lowered_path.exists(),
        "sample package should have logic.lowered.json"
    );

    // Verify the file can be read as valid JSON
    let lowered_json = std::fs::read(&lowered_path).unwrap();
    let _: serde_json::Value = serde_json::from_slice(&lowered_json).unwrap();
}
