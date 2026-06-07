use serde::{Deserialize, Serialize};

pub use iwm_runtime_host::{
    Rgba8 as BridgeRgba8, RuntimeDrawCommand as BridgeDrawCommand,
    RuntimeRenderFrame as BridgeFrameSnapshot,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebInputState {
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub jump_pressed: bool,
    pub jump_released: bool,
    pub restart: bool,
    #[serde(default)]
    pub keys_held: Vec<u16>,
    #[serde(default)]
    pub keys_pressed: Vec<u16>,
    #[serde(default)]
    pub keys_released: Vec<u16>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeJumpSnapshot {
    pub grounded: bool,
    pub active: bool,
    pub hold_frames: u32,
    pub cut_applied: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgePlayerSnapshot {
    pub x: f64,
    pub y: f64,
    pub hspeed: f64,
    pub vspeed: f64,
    pub facing_left: bool,
    pub jump: BridgeJumpSnapshot,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeInputTraceSnapshot {
    pub jump_button_key: u16,
    pub jump_pressed: bool,
    pub jump_just_pressed: bool,
    pub jump_just_released: bool,
    pub active_keys: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeTickPhaseSnapshot {
    pub input_diag_nanos: u64,
    pub step_events_nanos: u64,
    pub view_sync_nanos: u64,
    pub player_movement_nanos: u64,
    pub collision_events_nanos: u64,
    pub alarms_nanos: u64,
    pub keyboard_events_nanos: u64,
    pub render_submit_nanos: u64,
    pub total_nanos: u64,
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
    pub input_trace: BridgeInputTraceSnapshot,
    pub tick_phases: BridgeTickPhaseSnapshot,
    pub diagnostics: Vec<String>,
}
