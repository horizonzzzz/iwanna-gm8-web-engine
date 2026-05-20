use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompatibilityLevel {
    Supported,
    Partial,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeManifest {
    pub format_version: u32,
    pub package_kind: String,
    pub source_name: String,
    pub source_hash: String,
    pub engine_family: String,
    pub compatibility: CompatibilityLevel,
    pub default_room_id: Option<usize>,
    pub room_count: usize,
    pub object_count: usize,
    pub script_block_count: usize,
    pub sprite_count: usize,
    pub background_count: usize,
    pub sound_count: usize,
    pub resource_index_path: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceIndex {
    pub sprites: Vec<SpriteResource>,
    pub backgrounds: Vec<BackgroundResource>,
    pub sounds: Vec<SoundResource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteResource {
    pub id: usize,
    pub name: String,
    pub origin_x: i32,
    pub origin_y: i32,
    pub frame_paths: Vec<String>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundResource {
    pub id: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub image_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundResource {
    pub id: usize,
    pub name: String,
    pub file_path: String,
    pub extension: String,
    pub preload: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomDefinition {
    pub id: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub speed: u32,
    pub persistent: bool,
    pub backgrounds: Vec<RoomBackgroundLayer>,
    pub views_enabled: bool,
    pub views: Vec<RoomView>,
    pub instances: Vec<RoomInstancePlacement>,
    pub creation_block_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomBackgroundLayer {
    pub visible_on_start: bool,
    pub is_foreground: bool,
    pub source_bg: i32,
    pub xoffset: i32,
    pub yoffset: i32,
    pub tile_horz: bool,
    pub tile_vert: bool,
    pub hspeed: i32,
    pub vspeed: i32,
    pub stretch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomView {
    pub visible: bool,
    pub source_x: i32,
    pub source_y: i32,
    pub source_w: u32,
    pub source_h: u32,
    pub port_x: i32,
    pub port_y: i32,
    pub port_w: u32,
    pub port_h: u32,
    pub target: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInstancePlacement {
    pub instance_id: i32,
    pub object_id: i32,
    pub x: i32,
    pub y: i32,
    pub xscale: f64,
    pub yscale: f64,
    pub angle: f64,
    pub blend: u32,
    pub creation_block_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectDefinition {
    pub id: usize,
    pub name: String,
    pub sprite_index: i32,
    pub parent_index: i32,
    pub depth: i32,
    pub persistent: bool,
    pub visible: bool,
    pub solid: bool,
    pub mask_index: i32,
    pub events: Vec<ObjectEventEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectEventEntry {
    pub event_type: usize,
    pub sub_event: u32,
    pub block_id: String,
    pub action_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptIrFile {
    pub format: String,
    pub blocks: Vec<LogicBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicBlock {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub support: String,
    pub ops: Vec<LogicOp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub enum LogicOp {
    ActionCall {
        action_id: u32,
        lib_id: u32,
        applies_to: i32,
        is_condition: bool,
        invert_condition: bool,
        is_relative: bool,
        fn_name: String,
        fn_code: String,
        args: Vec<String>,
    },
    SourceSnippet {
        code: String,
    },
    Unsupported {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub dlls: Vec<String>,
    pub included_files: Vec<String>,
    pub warnings: Vec<String>,
    pub unsupported_features: Vec<String>,
}
