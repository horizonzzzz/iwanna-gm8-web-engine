use std::fs;

use iwm_detector::{DetectionVerdict, PackageInputKind};

#[test]
fn detect_directory_reports_gm8_likely() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("game.exe"),
        b"Game Maker Version 8 D3DX8.dll room_goto keyboard_check",
    )
    .unwrap();

    let report = iwm_detector::detect_input(temp.path()).unwrap();

    assert_eq!(report.input_kind, PackageInputKind::Directory);
    assert_eq!(report.verdict, DetectionVerdict::Gm8Likely);
    assert_eq!(report.executable_count, 1);
}

#[test]
fn detect_directory_reports_blocked_for_unity() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("game.exe"), b"UnityPlayer.dll UnityEngine").unwrap();

    let report = iwm_detector::detect_input(temp.path()).unwrap();

    assert_eq!(report.verdict, DetectionVerdict::Blocked);
}

#[test]
fn detect_exe_input_uses_package_inventory_for_sidecar_files() {
    let temp = tempfile::tempdir().unwrap();
    let exe_path = temp.path().join("game.exe");
    fs::write(&exe_path, b"MZfake").unwrap();
    fs::write(temp.path().join("data.win"), b"asset data").unwrap();

    let report = iwm_detector::detect_input(&exe_path).unwrap();

    assert_eq!(report.input_kind, PackageInputKind::Exe);
    assert_eq!(report.verdict, DetectionVerdict::GmsLikely);
    assert_eq!(report.executable_count, 1);
    assert_eq!(report.files.len(), 2);
}

#[test]
fn detect_directory_ignores_partial_inventory_signature_matches() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("game.exe"),
        b"Game Maker Version 8 D3DX8.dll room_goto keyboard_check",
    )
    .unwrap();
    fs::write(temp.path().join("notdata.win.txt"), b"x").unwrap();
    fs::write(temp.path().join("UnityPlayer.dll.backup"), b"x").unwrap();

    let report = iwm_detector::detect_input(temp.path()).unwrap();

    assert_eq!(report.verdict, DetectionVerdict::Gm8Likely);
    assert_eq!(report.signals, vec![iwm_detector::EngineFamily::Gm8]);
}

#[test]
fn detect_directory_scans_all_executables_and_warns_on_multiple_candidates() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("a.exe"), b"MZfake").unwrap();
    fs::write(temp.path().join("b.exe"), b"MZfake").unwrap();
    fs::write(
        temp.path().join("c.exe"),
        b"Game Maker Version 8 D3DX8.dll room_goto keyboard_check",
    )
    .unwrap();

    let report = iwm_detector::detect_input(temp.path()).unwrap();

    assert_eq!(report.verdict, DetectionVerdict::Gm8Likely);
    assert_eq!(report.executable_count, 3);
    assert!(report
        .warnings
        .iter()
        .any(|warning| warning.contains("multiple executable candidates")));
}
