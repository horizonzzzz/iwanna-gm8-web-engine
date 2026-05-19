use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompatibilityLevel {
    Supported,
    Partial,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    pub format_version: u32,
    pub source_name: String,
    pub source_hash: String,
    pub engine_family: String,
    pub compatibility: CompatibilityLevel,
    pub room_count: usize,
    pub object_count: usize,
    pub script_count: usize,
    pub sprite_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomSummary {
    pub id: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub speed: u32,
    pub persistent: bool,
    pub instance_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectSummary {
    pub id: usize,
    pub name: String,
    pub sprite_index: i32,
    pub parent_index: i32,
    pub depth: i32,
    pub persistent: bool,
    pub visible: bool,
    pub solid: bool,
    pub event_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptSummary {
    pub id: usize,
    pub name: String,
    pub code_len: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub dlls: Vec<String>,
    pub included_files: Vec<String>,
    pub warnings: Vec<String>,
    pub unsupported_features: Vec<String>,
}
