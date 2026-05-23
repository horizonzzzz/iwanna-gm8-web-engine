use iwm_runtime_host::{RuntimeDiagnostic, RuntimeDiagnosticLevel, RuntimeHost};

use crate::{RuntimeInstance, RuntimeRoomState, RuntimeValue};

#[derive(Clone, Copy)]
pub(crate) enum Axis {
    Horizontal,
    Vertical,
}

pub(crate) fn is_preferred_player_name(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "player" | "player2" | "playerface" | "obj_player" | "obj_player2" | "obj_playerface"
    )
}

pub(crate) fn is_player_instance(instance: &RuntimeInstance) -> bool {
    instance.player_candidate && instance.alive && is_preferred_player_name(&instance.object_name)
}

pub(crate) fn bounds_at(instance: &RuntimeInstance, x: i32, y: i32) -> (i32, i32, i32, i32) {
    let left = x - instance.origin_x + instance.bbox_left;
    let top = y - instance.origin_y + instance.bbox_top;
    let right = x - instance.origin_x + instance.bbox_right + 1;
    let bottom = y - instance.origin_y + instance.bbox_bottom + 1;
    (left, top, right.max(left + 1), bottom.max(top + 1))
}

pub(crate) fn collides_at(
    instance: &RuntimeInstance,
    x: i32,
    y: i32,
    others: &[RuntimeInstance],
    ignore_runtime_id: Option<usize>,
) -> bool {
    let (left, top, right, bottom) = bounds_at(instance, x, y);

    others.iter().any(|other| {
        if !other.alive || ignore_runtime_id == Some(other.runtime_id) {
            return false;
        }

        let (other_left, other_top, other_right, other_bottom) = bounds_at(other, other.x, other.y);
        left < other_right && right > other_left && top < other_bottom && bottom > other_top
    })
}

pub(crate) fn move_instance_axis(
    instance: &mut RuntimeInstance,
    solids: &[RuntimeInstance],
    ignore_runtime_id: Option<usize>,
    axis: Axis,
    delta: i32,
) {
    let step = delta.signum();
    let mut remaining = delta.abs();

    while remaining > 0 {
        let next_x = match axis {
            Axis::Horizontal => instance.x + step,
            Axis::Vertical => instance.x,
        };
        let next_y = match axis {
            Axis::Horizontal => instance.y,
            Axis::Vertical => instance.y + step,
        };

        if collides_at(instance, next_x, next_y, solids, ignore_runtime_id) {
            match axis {
                Axis::Horizontal => instance.hspeed = 0,
                Axis::Vertical => instance.vspeed = 0,
            }
            break;
        }

        instance.x = next_x;
        instance.y = next_y;
        remaining -= 1;
    }
}

pub(crate) fn player_out_of_bounds(
    instance: &RuntimeInstance,
    room_width: u32,
    room_height: u32,
) -> bool {
    let (left, top, right, bottom) = bounds_at(instance, instance.x, instance.y);
    right < 0 || bottom < 0 || left > room_width as i32 || top > room_height as i32
}

pub(crate) fn adjusted_spawn_for_player(
    player: &RuntimeInstance,
    spawn_x: i32,
    spawn_y: i32,
    room: &RuntimeRoomState,
) -> (i32, i32) {
    let solids = room
        .instances
        .iter()
        .filter(|instance| instance.alive && instance.solid)
        .cloned()
        .collect::<Vec<_>>();

    if !collides_at(player, spawn_x, spawn_y, &solids, Some(player.runtime_id)) {
        return (spawn_x, spawn_y);
    }

    for distance in 1..=player.height.max(player.width).max(16) {
        for (dx, dy) in [(0, -distance), (0, distance), (-distance, 0), (distance, 0)] {
            let x = spawn_x + dx;
            let y = spawn_y + dy;
            if spawn_candidate_is_inside_room(player, x, y, room)
                && !collides_at(player, x, y, &solids, Some(player.runtime_id))
            {
                return (x, y);
            }
        }
    }

    (spawn_x, spawn_y)
}

fn spawn_candidate_is_inside_room(
    player: &RuntimeInstance,
    x: i32,
    y: i32,
    room: &RuntimeRoomState,
) -> bool {
    let (left, top, right, bottom) = bounds_at(player, x, y);
    left >= 0 && top >= 0 && right <= room.width as i32 && bottom <= room.height as i32
}

pub(crate) fn parse_runtime_value(raw: &str) -> Option<RuntimeValue> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(number) = trimmed.parse::<f64>() {
        return Some(RuntimeValue::Number(number));
    }

    if trimmed.eq_ignore_ascii_case("true") {
        return Some(RuntimeValue::Bool(true));
    }

    if trimmed.eq_ignore_ascii_case("false") {
        return Some(RuntimeValue::Bool(false));
    }

    Some(RuntimeValue::Text(trimmed.trim_matches('"').to_string()))
}

pub(crate) fn as_number(value: &RuntimeValue) -> Option<f64> {
    match value {
        RuntimeValue::Number(number) => Some(*number),
        RuntimeValue::Bool(flag) => Some(if *flag { 1.0 } else { 0.0 }),
        RuntimeValue::Text(text) => text.parse().ok(),
    }
}

pub(crate) fn parse_room_id(raw: &str) -> Option<usize> {
    let value = parse_runtime_value(raw)?;
    let number = as_number(&value)?;
    if number.is_finite() && number >= 0.0 {
        Some(number.round() as usize)
    } else {
        None
    }
}

pub(crate) fn record_host_diagnostic<H: RuntimeHost>(
    host: &mut H,
    diagnostics: &mut Vec<RuntimeDiagnostic>,
    level: RuntimeDiagnosticLevel,
    code: impl Into<String>,
    message: impl Into<String>,
) {
    let diagnostic = RuntimeDiagnostic {
        level,
        code: code.into(),
        message: message.into(),
    };
    host.record(diagnostic.clone());
    diagnostics.push(diagnostic);
}
