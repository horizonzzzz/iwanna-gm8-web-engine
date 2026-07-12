use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn runtime_scenario_accepts_existing_input_script_with_assertions() {
    let fixture = fixture_root();
    let scenario_path =
        std::env::temp_dir().join(format!("iwm-runtime-scenario-{}.json", std::process::id()));
    fs::write(
        &scenario_path,
        r#"{
          "ticks": [],
          "assertions": {
            "no_runtime_blockers": true,
            "final_room_id": 300,
            "visited_room_ids": [300]
          }
        }"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_iwm-cli"))
        .args([
            "runtime-scenario",
            "--input",
            fixture.to_str().unwrap(),
            "--scenario",
            scenario_path.to_str().unwrap(),
            "--ticks",
            "1",
        ])
        .output()
        .unwrap();

    let _ = fs::remove_file(scenario_path);
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("iwm-runtime-model")
        .join("tests")
        .join("fixtures")
        .join("sparse-runtime-package")
}
