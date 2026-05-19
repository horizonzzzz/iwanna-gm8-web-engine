use crate::models::{DetectionReport, DetectionVerdict, EngineFamily};
use crate::package::load_package;
use crate::signatures::{match_inventory_signals, match_signals};
use std::fs;
use std::path::Path;

pub fn detect_input(path: &Path) -> Result<DetectionReport, String> {
    let package = load_package(path)?;
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
        let bytes = fs::read(exe).map_err(|e| e.to_string())?;
        signals.extend(match_signals(&bytes[..bytes.len().min(8_000_000)]));
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
        source_name: package.source_name,
        input_kind: package.input_kind,
        verdict,
        signals,
        executable_count: package.executables.len(),
        dlls: package.dlls,
        files: package.files,
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
