use iwm_runtime_core::{
    RuntimeCoreError, RuntimeInputTraceSnapshot, RuntimeJumpSnapshot, RuntimePlayerSnapshot,
    RuntimeSnapshot, RuntimeStatus,
};
use iwm_runtime_host::{RuntimeDiagnostic, RuntimeDrawCommand};

use crate::{
    BridgeDrawCommand, BridgeFrameSnapshot, BridgeInputTraceSnapshot, BridgeJumpSnapshot,
    BridgePlayerSnapshot, BridgeSnapshot,
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

pub fn bridge_input_trace_snapshot(snapshot: RuntimeInputTraceSnapshot) -> BridgeInputTraceSnapshot {
    BridgeInputTraceSnapshot {
        jump_button_key: snapshot.jump_button_key,
        jump_pressed: snapshot.jump_pressed,
        jump_just_pressed: snapshot.jump_just_pressed,
        jump_just_released: snapshot.jump_just_released,
        active_keys: snapshot.active_keys,
    }
}

pub fn bridge_draw_command(command: &RuntimeDrawCommand) -> BridgeDrawCommand {
    match command {
        RuntimeDrawCommand::Clear { colour } => BridgeDrawCommand::Clear {
            colour: [colour.r, colour.g, colour.b, colour.a],
        },
        RuntimeDrawCommand::DrawBackground {
            background_id,
            x,
            y,
            stretch,
            tile_horz,
            tile_vert,
            is_foreground,
        } => BridgeDrawCommand::DrawBackground {
            background_id: *background_id,
            x: *x,
            y: *y,
            stretch: *stretch,
            tile_horz: *tile_horz,
            tile_vert: *tile_vert,
            is_foreground: *is_foreground,
        },
        RuntimeDrawCommand::DrawTile {
            background_id,
            x,
            y,
            tile_x,
            tile_y,
            width,
            height,
            xscale,
            yscale,
        } => BridgeDrawCommand::DrawTile {
            background_id: *background_id,
            x: *x,
            y: *y,
            tile_x: *tile_x,
            tile_y: *tile_y,
            width: *width,
            height: *height,
            xscale: *xscale,
            yscale: *yscale,
        },
        RuntimeDrawCommand::DrawSprite {
            sprite_id,
            frame_index,
            x,
            y,
            origin_x,
            origin_y,
            xscale,
            yscale,
            angle_degrees,
        } => BridgeDrawCommand::DrawSprite {
            sprite_id: *sprite_id,
            frame_index: *frame_index,
            x: *x,
            y: *y,
            origin_x: *origin_x,
            origin_y: *origin_y,
            xscale: *xscale,
            yscale: *yscale,
            angle_degrees: *angle_degrees,
        },
        RuntimeDrawCommand::FillRect {
            x,
            y,
            width,
            height,
            colour,
        } => BridgeDrawCommand::FillRect {
            x: *x,
            y: *y,
            width: *width,
            height: *height,
            colour: [colour.r, colour.g, colour.b, colour.a],
        },
        RuntimeDrawCommand::Present => BridgeDrawCommand::Present,
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

pub fn bridge_frame_snapshot(
    tick: u64,
    room_id: Option<usize>,
    width: u32,
    height: u32,
    commands: Vec<BridgeDrawCommand>,
) -> BridgeFrameSnapshot {
    BridgeFrameSnapshot {
        tick,
        room_id,
        width,
        height,
        commands,
    }
}
