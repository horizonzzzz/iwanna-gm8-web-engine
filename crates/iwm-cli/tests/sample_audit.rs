use std::path::PathBuf;
use std::process::Command;

#[test]
fn sample_audit_reports_a_missing_source_as_an_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_iwm-cli"))
        .args([
            "sample-audit",
            "--input",
            "definitely-missing-sample",
            "--package-output",
            temp_path("package").to_str().unwrap(),
            "--report-output",
            temp_path("audit.json").to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        !String::from_utf8_lossy(&output.stderr).contains("unrecognized subcommand"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("iwm-audit-{}-{name}", std::process::id()))
}
