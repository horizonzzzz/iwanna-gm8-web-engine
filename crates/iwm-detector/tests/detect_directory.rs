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
