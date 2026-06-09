use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("iwm-runtime-model")
        .join("tests")
        .join("fixtures")
        .join("sparse-runtime-package")
}

#[test]
fn validate_package_exits_successfully_for_valid_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_iwm-cli"))
        .arg("validate-package")
        .arg("--input")
        .arg(fixture_root())
        .output()
        .expect("cli should run");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    assert!(stdout(&output).contains("\"valid\": true"));
}

#[test]
fn validate_package_exits_nonzero_for_invalid_fixture() {
    let temp_root = copy_fixture_to_temp();
    let rooms_path = temp_root.join("rooms.json");
    let mut rooms: serde_json::Value =
        serde_json::from_slice(&fs::read(&rooms_path).expect("rooms fixture should read"))
            .expect("rooms fixture should parse");
    rooms[0]["instances"][0]["object_id"] = serde_json::json!(999);
    fs::write(
        &rooms_path,
        serde_json::to_vec_pretty(&rooms).expect("rooms fixture should serialize"),
    )
    .expect("rooms fixture should write");

    let output = Command::new(env!("CARGO_BIN_EXE_iwm-cli"))
        .arg("validate-package")
        .arg("--input")
        .arg(&temp_root)
        .output()
        .expect("cli should run");

    assert!(!output.status.success());
    assert!(stdout(&output).contains("missing-room-instance-object"));

    fs::remove_dir_all(temp_root).expect("temporary fixture should be removed");
}

#[test]
fn runtime_diagnostics_trace_player_includes_comparable_summary() {
    let temp_root = copy_fixture_to_temp();
    make_fixture_player_traceable(&temp_root);

    let output = Command::new(env!("CARGO_BIN_EXE_iwm-cli"))
        .arg("runtime-diagnostics")
        .arg("--input")
        .arg(&temp_root)
        .arg("--ticks")
        .arg("3")
        .arg("--trace-player")
        .arg("--trace-every")
        .arg("1")
        .output()
        .expect("cli should run");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    let diagnostics: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("diagnostics output should be json");
    let trace = diagnostics["player_trace"]
        .as_array()
        .expect("player trace should be present");
    let summary = &diagnostics["trace_summary"];

    assert_eq!(summary["sample_count"], trace.len());
    assert_eq!(summary["first"]["tick"], trace[0]["tick"]);
    assert_eq!(summary["last"]["tick"], trace.last().unwrap()["tick"]);
    assert_eq!(summary["last"]["x"], trace.last().unwrap()["x"]);
    assert_eq!(summary["last"]["y"], trace.last().unwrap()["y"]);
    assert!(summary["max_abs_vspeed"].is_number());
    assert_eq!(summary["rooms"][0]["room_id"], trace[0]["room_id"]);

    fs::remove_dir_all(temp_root).expect("temporary fixture should be removed");
}

fn copy_fixture_to_temp() -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    let temp_root = std::env::temp_dir().join(format!("iwm-cli-validate-package-{suffix}"));
    copy_dir(&fixture_root(), &temp_root);
    temp_root
}

fn copy_dir(source: &Path, target: &Path) {
    fs::create_dir_all(target).expect("fixture target directory should be created");

    for entry in fs::read_dir(source).expect("fixture source directory should be readable") {
        let entry = entry.expect("fixture entry should be readable");
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir(&source_path, &target_path);
        } else {
            fs::copy(&source_path, &target_path).expect("fixture file should be copied");
        }
    }
}

fn make_fixture_player_traceable(root: &Path) {
    let objects_path = root.join("objects.json");
    let mut objects: serde_json::Value =
        serde_json::from_slice(&fs::read(&objects_path).expect("objects fixture should read"))
            .expect("objects fixture should parse");
    objects[0]["name"] = serde_json::json!("obj_player");
    fs::write(
        &objects_path,
        serde_json::to_vec_pretty(&objects).expect("objects fixture should serialize"),
    )
    .expect("objects fixture should write");
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
