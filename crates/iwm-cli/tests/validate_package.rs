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

#[test]
fn runtime_diagnostics_input_script_drives_player_trace_edges() {
    let temp_root = copy_fixture_to_temp();
    make_fixture_player_traceable(&temp_root);
    let input_script_path = temp_root.join("input-script.json");
    fs::write(
        &input_script_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "ticks": [
                {
                    "tick": 0,
                    "press_keys": [32]
                }
            ]
        }))
        .expect("input script should serialize"),
    )
    .expect("input script should write");

    let output = Command::new(env!("CARGO_BIN_EXE_iwm-cli"))
        .arg("runtime-diagnostics")
        .arg("--input")
        .arg(&temp_root)
        .arg("--ticks")
        .arg("3")
        .arg("--trace-player")
        .arg("--trace-every")
        .arg("1")
        .arg("--input-script")
        .arg(&input_script_path)
        .output()
        .expect("cli should run");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    let diagnostics: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("diagnostics output should be json");
    let trace = diagnostics["player_trace"]
        .as_array()
        .expect("player trace should be present");

    assert!(trace.iter().any(|entry| {
        entry["jump_button_key"] == serde_json::json!(32)
            && entry["jump_pressed"] == serde_json::json!(true)
            && entry["jump_just_pressed"] == serde_json::json!(true)
    }));

    fs::remove_dir_all(temp_root).expect("temporary fixture should be removed");
}

#[test]
fn runtime_diagnostics_input_script_ticks_start_after_preselect() {
    let temp_root = copy_fixture_to_temp();
    make_fixture_player_traceable(&temp_root);
    let input_script_path = temp_root.join("input-script.json");
    fs::write(
        &input_script_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "ticks": [
                {
                    "tick": 0,
                    "press_keys": [32]
                }
            ]
        }))
        .expect("input script should serialize"),
    )
    .expect("input script should write");

    let output = Command::new(env!("CARGO_BIN_EXE_iwm-cli"))
        .arg("runtime-diagnostics")
        .arg("--input")
        .arg(&temp_root)
        .arg("--preselect-ticks")
        .arg("2")
        .arg("--ticks")
        .arg("1")
        .arg("--trace-player")
        .arg("--trace-every")
        .arg("1")
        .arg("--input-script")
        .arg(&input_script_path)
        .output()
        .expect("cli should run");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    let diagnostics: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("diagnostics output should be json");
    let trace = diagnostics["player_trace"]
        .as_array()
        .expect("player trace should be present");

    assert!(trace.iter().any(|entry| {
        entry["tick"] == serde_json::json!(3)
            && entry["jump_pressed"] == serde_json::json!(true)
            && entry["jump_just_pressed"] == serde_json::json!(true)
    }));

    fs::remove_dir_all(temp_root).expect("temporary fixture should be removed");
}

#[test]
fn runtime_diagnostics_outputs_runtime_events() {
    let temp_root = copy_fixture_to_temp();

    let output = Command::new(env!("CARGO_BIN_EXE_iwm-cli"))
        .arg("runtime-diagnostics")
        .arg("--input")
        .arg(&temp_root)
        .arg("--ticks")
        .arg("2")
        .arg("--press-restart")
        .output()
        .expect("cli should run");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    let diagnostics: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("diagnostics output should be json");
    let events = diagnostics["runtime_events"]
        .as_array()
        .expect("runtime events should be present");

    assert!(events
        .iter()
        .any(|entry| { entry["code"] == serde_json::json!("runtime-room-restart-requested") }));

    fs::remove_dir_all(temp_root).expect("temporary fixture should be removed");
}

#[test]
fn runtime_diagnostics_runtime_events_include_structured_fields() {
    let temp_root = copy_fixture_to_temp();
    make_fixture_spawn_sparse_player_each_step(&temp_root);

    let output = Command::new(env!("CARGO_BIN_EXE_iwm-cli"))
        .arg("runtime-diagnostics")
        .arg("--input")
        .arg(&temp_root)
        .arg("--ticks")
        .arg("1")
        .output()
        .expect("cli should run");

    assert!(output.status.success(), "stderr={}", stderr(&output));
    let diagnostics: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("diagnostics output should be json");
    let events = diagnostics["runtime_events"]
        .as_array()
        .expect("runtime events should be present");
    let created = events
        .iter()
        .find(|entry| entry["code"] == serde_json::json!("runtime-instance-created"))
        .expect("instance create event should be present");

    assert_eq!(created["room"], serde_json::json!(300));
    assert_eq!(created["tick"], serde_json::json!(1));
    assert_eq!(created["object"], serde_json::json!("obj_sparse_player"));
    assert!(created["runtime_id"].is_number());
    assert_eq!(created["x"], serde_json::json!(32.0));
    assert_eq!(created["y"], serde_json::json!(64.0));
    assert!(created["message"]
        .as_str()
        .unwrap()
        .contains("object=obj_sparse_player"));

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

fn make_fixture_spawn_sparse_player_each_step(root: &Path) {
    let lowered_path = root.join("logic.lowered.json");
    let mut lowered: serde_json::Value =
        serde_json::from_slice(&fs::read(&lowered_path).expect("lowered fixture should read"))
            .expect("lowered fixture should parse");
    let entries = lowered["entries"]
        .as_array_mut()
        .expect("lowered fixture entries should be an array");
    let step_entry = entries
        .iter_mut()
        .find(|entry| entry["block_id"] == serde_json::json!("object:705:event:3:0"))
        .expect("step entry should exist");
    step_entry["statements"] = serde_json::json!([
        {
            "kind": "function-call",
            "name": "instance_create",
            "args": [
                { "kind": "literal-number", "value": 32.0 },
                { "kind": "literal-number", "value": 64.0 },
                { "kind": "identifier", "value": "obj_sparse_player" }
            ]
        }
    ]);
    fs::write(
        &lowered_path,
        serde_json::to_vec_pretty(&lowered).expect("lowered fixture should serialize"),
    )
    .expect("lowered fixture should write");
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
