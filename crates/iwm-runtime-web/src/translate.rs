use iwm_runtime_core::{
    RuntimeCoreError, RuntimeInputTraceSnapshot, RuntimeJumpSnapshot, RuntimePlayerSnapshot,
    RuntimeSnapshot, RuntimeStatus, RuntimeTickPhaseSnapshot,
};
use iwm_runtime_host::RuntimeDiagnostic;

use crate::{
    BridgeInputTraceSnapshot, BridgeJumpSnapshot, BridgePlayerSnapshot, BridgeSnapshot,
    BridgeTickPhaseSnapshot,
};

pub fn bridge_snapshot(snapshot: RuntimeSnapshot) -> BridgeSnapshot {
    BridgeSnapshot {
        status: status_label(snapshot.status).into(),
        tick: snapshot.tick,
        room_id: snapshot.room_id,
        room_name: snapshot.room_name,
        instance_count: snapshot.instance_count,
        player: snapshot.player.map(bridge_player_snapshot),
        input_trace: bridge_input_trace_snapshot(snapshot.input_trace),
        tick_phases: bridge_tick_phase_snapshot(snapshot.tick_phases),
        diagnostics: format_diagnostics(&snapshot.diagnostics),
    }
}

pub fn bridge_player_snapshot(snapshot: RuntimePlayerSnapshot) -> BridgePlayerSnapshot {
    BridgePlayerSnapshot {
        x: snapshot.x,
        y: snapshot.y,
        hspeed: snapshot.hspeed,
        vspeed: snapshot.vspeed,
        facing_left: snapshot.facing_left,
        jump: bridge_jump_snapshot(snapshot.jump),
    }
}

pub fn bridge_jump_snapshot(snapshot: RuntimeJumpSnapshot) -> BridgeJumpSnapshot {
    BridgeJumpSnapshot {
        grounded: snapshot.grounded,
        active: snapshot.active,
        hold_frames: snapshot.hold_frames,
        cut_applied: snapshot.cut_applied,
    }
}

pub fn bridge_input_trace_snapshot(
    snapshot: RuntimeInputTraceSnapshot,
) -> BridgeInputTraceSnapshot {
    BridgeInputTraceSnapshot {
        jump_button_key: snapshot.jump_button_key,
        jump_pressed: snapshot.jump_pressed,
        jump_just_pressed: snapshot.jump_just_pressed,
        jump_just_released: snapshot.jump_just_released,
        active_keys: snapshot.active_keys,
    }
}

pub fn bridge_tick_phase_snapshot(snapshot: RuntimeTickPhaseSnapshot) -> BridgeTickPhaseSnapshot {
    BridgeTickPhaseSnapshot {
        input_diag_nanos: snapshot.input_diag_nanos,
        step_events_nanos: snapshot.step_events_nanos,
        view_sync_nanos: snapshot.view_sync_nanos,
        player_movement_nanos: snapshot.player_movement_nanos,
        collision_events_nanos: snapshot.collision_events_nanos,
        alarms_nanos: snapshot.alarms_nanos,
        keyboard_events_nanos: snapshot.keyboard_events_nanos,
        render_submit_nanos: snapshot.render_submit_nanos,
        total_nanos: snapshot.total_nanos,
    }
}

pub fn format_diagnostics(diagnostics: &[RuntimeDiagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .map(|entry| {
            format!(
                "{}:{}:{}",
                diagnostic_level_label(entry),
                entry.code,
                entry.message
            )
        })
        .collect()
}

pub fn diagnostic_level_label(entry: &RuntimeDiagnostic) -> &'static str {
    match entry.level {
        iwm_runtime_host::RuntimeDiagnosticLevel::Info => "info",
        iwm_runtime_host::RuntimeDiagnosticLevel::Warning => "warning",
        iwm_runtime_host::RuntimeDiagnosticLevel::Error => "error",
    }
}

pub fn status_label(status: RuntimeStatus) -> &'static str {
    match status {
        RuntimeStatus::Idle => "idle",
        RuntimeStatus::Ready => "ready",
        RuntimeStatus::Running => "running",
        RuntimeStatus::Error => "error",
    }
}

pub fn format_core_error(error: RuntimeCoreError) -> String {
    match error {
        RuntimeCoreError::NoRooms => "runtime package does not contain any rooms".into(),
        RuntimeCoreError::RoomMissing(room_id) => {
            format!("runtime package is missing room {}", room_id)
        }
        RuntimeCoreError::Host(host_error) => host_error.to_string(),
    }
}
