use std::fs::File;
use std::io::Write;

#[test]
fn detect_zip_reports_gm8_likely() {
    let temp = tempfile::tempdir().unwrap();
    let zip_path = temp.path().join("sample.zip");
    let file = File::create(&zip_path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();

    zip.start_file("game.exe", options).unwrap();
    zip.write_all(b"Game Maker Version 8 D3DX8.dll").unwrap();
    zip.finish().unwrap();

    let report = iwm_detector::detect_input(&zip_path).unwrap();

    assert_eq!(report.verdict, iwm_detector::DetectionVerdict::Gm8Likely);
    assert_eq!(report.input_kind, iwm_detector::PackageInputKind::Zip);
}
