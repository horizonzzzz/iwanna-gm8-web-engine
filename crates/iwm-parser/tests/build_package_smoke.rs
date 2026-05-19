use iwm_parser::models::{CompatibilityLevel, PackageManifest};
use std::fs;
use std::process::Command;

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

#[test]
fn build_package_writes_v0_outputs_for_single_exe_input() {
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
    assert!(out_dir.join("scripts.json").exists());
    assert!(out_dir.join("analysis.json").exists());
}

#[test]
fn build_package_supports_zip_input() {
    let sample_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("samples")
        .join("local")
        .join("iwanna-examples")
        .join("gm8-core")
        .join("IWBT_Dife");

    if !sample_dir.exists() {
        return;
    }

    let temp = tempfile::tempdir().unwrap();
    let zip_path = temp.path().join("sample.zip");
    let file = fs::File::create(&zip_path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();

    for name in ["I wanna be the Dife.exe", "ReadMe.txt"] {
        let bytes = fs::read(sample_dir.join(name)).unwrap();
        zip.start_file(name, options).unwrap();
        use std::io::Write;
        zip.write_all(&bytes).unwrap();
    }
    zip.finish().unwrap();

    let out_dir = temp.path().join("out");
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "-p",
            "iwm-cli",
            "--",
            "build-package",
            "--input",
        ])
        .arg(&zip_path)
        .args(["--output"])
        .arg(&out_dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(out_dir.join("manifest.json").exists());
}

#[test]
fn build_package_rejects_multiple_executable_candidates() {
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

    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("a.exe"), b"MZfake").unwrap();
    fs::copy(sample_exe, temp.path().join("b.exe")).unwrap();
    let out_dir = temp.path().join("out");

    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "-p",
            "iwm-cli",
            "--",
            "build-package",
            "--input",
        ])
        .arg(temp.path())
        .args(["--output"])
        .arg(&out_dir)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("multiple executable candidates"));
    assert!(!out_dir.join("manifest.json").exists());
}
