use iwm_runtime_core::{
    RuntimeCollisionParticipantTrace, RuntimeCollisionTraceEntry, RuntimeCoreError,
    RuntimeDeathTraceEntry, RuntimeInputTraceSnapshot, RuntimeJumpSnapshot, RuntimePlayerSnapshot,
    RuntimeSnapshot, RuntimeStatus, RuntimeTickPhaseSnapshot,
};
use iwm_runtime_host::RuntimeDiagnostic;

use crate::{
    BridgeCollisionParticipantTrace, BridgeCollisionTraceEntry, BridgeDeathTraceEntry,
    BridgeInputTraceSnapshot, BridgeJumpSnapshot, BridgePlayerSnapshot, BridgeSnapshot,
    BridgeTickPhaseSnapshot,
};

pub fn bridge_snapshot(snapshot: RuntimeSnapshot) -> BridgeSnapshot {
    BridgeSnapshot {
        status: status_label(snapshot.status).into(),
        tick: snapshot.tick,
        room_id: snapshot.room_id,
        room_name: snapshot.room_name,
        room_speed: snapshot.room_speed,
        instance_count: snapshot.instance_count,
        deaths: snapshot.deaths,
        player: snapshot.player.map(bridge_player_snapshot),
        input_trace: bridge_input_trace_snapshot(snapshot.input_trace),
        tick_phases: bridge_tick_phase_snapshot(snapshot.tick_phases),
        diagnostics: format_diagnostics(&snapshot.diagnostics),
        collision_trace: snapshot
            .collision_trace
            .into_iter()
            .map(bridge_collision_trace_entry)
            .collect(),
        death_trace: snapshot
            .death_trace
            .into_iter()
            .map(bridge_death_trace_entry)
            .collect(),
    }
}

pub fn bridge_collision_participant_trace(
    trace: RuntimeCollisionParticipantTrace,
) -> BridgeCollisionParticipantTrace {
    BridgeCollisionParticipantTrace {
        runtime_id: trace.runtime_id,
        instance_id: trace.instance_id,
        object_id: trace.object_id,
        object_name: trace.object_name,
        x: trace.x,
        y: trace.y,
        previous_x: trace.previous_x,
        previous_y: trace.previous_y,
        hspeed: trace.hspeed,
        vspeed: trace.vspeed,
        bounds: trace.bounds,
        previous_bounds: trace.previous_bounds,
        solid: trace.solid,
        hazard: trace.hazard,
        has_collision_mask: trace.has_collision_mask,
        collision_mask_size: trace.collision_mask_size,
    }
}

pub fn bridge_collision_trace_entry(
    trace: RuntimeCollisionTraceEntry,
) -> BridgeCollisionTraceEntry {
    BridgeCollisionTraceEntry {
        tick: trace.tick,
        phase: trace.phase,
        target_object_id: trace.target_object_id,
        solid_collision: trace.solid_collision,
        contact_y: trace.contact_y,
        event_blocks: trace.event_blocks,
        owner: bridge_collision_participant_trace(trace.owner),
        other: bridge_collision_participant_trace(trace.other),
    }
}

pub fn bridge_death_trace_entry(trace: RuntimeDeathTraceEntry) -> BridgeDeathTraceEntry {
    BridgeDeathTraceEntry {
        tick: trace.tick,
        room_id: trace.room_id,
        room_name: trace.room_name,
        reason: trace.reason,
        player: bridge_collision_participant_trace(trace.player),
        hazard: trace.hazard.map(bridge_collision_participant_trace),
        collision_window: trace
            .collision_window
            .into_iter()
            .map(bridge_collision_trace_entry)
            .collect(),
    }
}

pub fn bridge_player_snapshot(snapshot: RuntimePlayerSnapshot) -> BridgePlayerSnapshot {
    BridgePlayerSnapshot {
        runtime_id: snapshot.runtime_id,
        instance_id: snapshot.instance_id,
        object_id: snapshot.object_id,
        object_name: snapshot.object_name,
        x: snapshot.x,
        y: snapshot.y,
        hspeed: snapshot.hspeed,
        vspeed: snapshot.vspeed,
        facing_left: snapshot.facing_left,
        alive: snapshot.alive,
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
