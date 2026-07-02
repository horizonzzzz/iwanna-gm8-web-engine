mod audio_host;
mod bridge_buffer;
mod bridge_types;
mod ffi;
mod file_host;
mod result_store;
mod translate;
mod web_runtime_host;

pub use audio_host::WebAudioHost;
pub use bridge_buffer::{decode_web_input_state_from_buffer, encode_bridge_step_result_to_buffer};
pub use bridge_types::{
    BridgeDrawCommand, BridgeFrameSnapshot, BridgeInputTraceSnapshot, BridgeJumpSnapshot,
    BridgePlayerSnapshot, BridgeRgba8, BridgeSnapshot, BridgeStepResult, BridgeTickPhaseSnapshot,
    WebInputState,
};
pub use ffi::{
    iwm_alloc, iwm_boot_json, iwm_diagnostics_json, iwm_frame_json, iwm_free, iwm_last_result_len,
    iwm_reset, iwm_select_room, iwm_set_input_json, iwm_snapshot_json, iwm_step_buffer,
    iwm_step_json, iwm_tick,
};
pub use web_runtime_host::WebRuntimeHost;
