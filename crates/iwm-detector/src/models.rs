use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DetectionVerdict {
    Gm8Likely,
    GmsLikely,
    Unknown,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EngineFamily {
    Gm8,
    Gms,
    Unity,
    RpgMaker,
    Clickteam,
    Godot,
    Nwjs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageInputKind {
    Directory,
    Exe,
    Zip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub relative_path: String,
    pub extension: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionReport {
    pub source_name: String,
    pub input_kind: PackageInputKind,
    pub verdict: DetectionVerdict,
    pub signals: Vec<EngineFamily>,
    pub executable_count: usize,
    pub dlls: Vec<String>,
    pub files: Vec<FileEntry>,
    pub warnings: Vec<String>,
}

impl DetectionReport {
    pub fn minimal(
        source_name: String,
        input_kind: PackageInputKind,
        verdict: DetectionVerdict,
        signals: Vec<EngineFamily>,
    ) -> Self {
        Self {
            source_name,
            input_kind,
            verdict,
            signals,
            executable_count: 0,
            dlls: Vec::new(),
            files: Vec::new(),
            warnings: Vec::new(),
        }
    }
}
