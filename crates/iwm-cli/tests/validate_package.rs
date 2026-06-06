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

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
