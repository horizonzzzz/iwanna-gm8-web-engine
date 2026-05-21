use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebInputState {
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub jump_pressed: bool,
    pub jump_released: bool,
    pub restart: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgePlayerSnapshot {
    pub x: i32,
    pub y: i32,
    pub hspeed: i32,
    pub vspeed: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeSnapshot {
    pub status: String,
    pub tick: u64,
    pub room_id: Option<usize>,
    pub room_name: Option<String>,
    pub instance_count: usize,
    pub player: Option<BridgePlayerSnapshot>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "kind")]
pub enum BridgeDrawCommand {
    Clear {
        colour: [u8; 4],
    },
    DrawBackground {
        background_id: usize,
        x: i32,
        y: i32,
        stretch: bool,
        tile_horz: bool,
        tile_vert: bool,
        is_foreground: bool,
    },
    DrawTile {
        background_id: usize,
        x: i32,
        y: i32,
        tile_x: u32,
        tile_y: u32,
        width: u32,
        height: u32,
        xscale: f64,
        yscale: f64,
    },
    DrawSprite {
        sprite_id: usize,
        frame_index: usize,
        x: i32,
        y: i32,
        origin_x: i32,
        origin_y: i32,
        xscale: f64,
        yscale: f64,
        angle_degrees: f64,
    },
    FillRect {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        colour: [u8; 4],
    },
    Present,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeFrameSnapshot {
    pub tick: u64,
    pub room_id: Option<usize>,
    pub width: u32,
    pub height: u32,
    pub commands: Vec<BridgeDrawCommand>,
}
