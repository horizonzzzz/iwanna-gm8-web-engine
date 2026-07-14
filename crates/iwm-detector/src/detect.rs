use crate::models::{DetectionReport, DetectionVerdict, EngineFamily};
use crate::package::{load_package, LoadedPackage};
use crate::signatures::{match_inventory_signals, match_signals};
use std::fs;
use std::io::Read;
use std::path::Path;

const SIGNATURE_SCAN_BYTES: u64 = 8_000_000;

pub fn detect_input(path: &Path) -> Result<DetectionReport, String> {
    let package = load_package(path)?;
    detect_package(&package)
}

pub fn detect_package(package: &LoadedPackage) -> Result<DetectionReport, String> {
    let mut signals = Vec::new();
    let mut warnings = package.warnings.clone();

    if package.executables.is_empty() {
        warnings.push("no executable found".into());
    }
    if package.executables.len() > 1 {
        warnings.push(format!(
            "multiple executable candidates found: {}",
            package.executables.len()
        ));
    }

    for exe in &package.executables {
        let mut bytes = Vec::new();
        fs::File::open(exe)
            .map_err(|e| e.to_string())?
            .take(SIGNATURE_SCAN_BYTES)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        signals.extend(match_signals(&bytes));
    }

    let inventory_paths = package
        .files
        .iter()
        .map(|file| file.relative_path.clone())
        .collect::<Vec<_>>();
    signals.extend(match_inventory_signals(&inventory_paths));

    signals.sort_by_key(|family| *family as u8);
    signals.dedup();

    let verdict = classify(&signals, package.executables.len());

    Ok(DetectionReport {
        source_name: package.source_name.clone(),
        input_kind: package.input_kind,
        verdict,
        signals,
        executable_count: package.executables.len(),
        dlls: package.dlls.clone(),
        files: package.files.clone(),
        warnings,
    })
}

fn classify(signals: &[EngineFamily], executable_count: usize) -> DetectionVerdict {
    if executable_count == 0 {
        return DetectionVerdict::Blocked;
    }
    if signals.contains(&EngineFamily::Gms) {
        return DetectionVerdict::GmsLikely;
    }
    if signals.iter().any(|s| {
        matches!(
            s,
            EngineFamily::Unity
                | EngineFamily::RpgMaker
                | EngineFamily::Clickteam
                | EngineFamily::Godot
                | EngineFamily::Nwjs
        )
    }) {
        return DetectionVerdict::Blocked;
    }
    if signals.contains(&EngineFamily::Gm8) {
        return DetectionVerdict::Gm8Likely;
    }
    DetectionVerdict::Unknown
}
