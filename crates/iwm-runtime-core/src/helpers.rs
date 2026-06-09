use iwm_runtime_host::{RuntimeDiagnostic, RuntimeDiagnosticLevel, RuntimeHost};

use crate::{RuntimeCollisionMask, RuntimeInstance, RuntimeRoomState, RuntimeValue};

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

pub(crate) fn bounds_at(instance: &RuntimeInstance, x: f64, y: f64) -> (i32, i32, i32, i32) {
    let x = x.round() as i32;
    let y = y.round() as i32;
    let left = x - instance.origin_x + instance.bbox_left;
    let top = y - instance.origin_y + instance.bbox_top;
    let right = x - instance.origin_x + instance.bbox_right + 1;
    let bottom = y - instance.origin_y + instance.bbox_bottom + 1;
    (left, top, right.max(left + 1), bottom.max(top + 1))
}

pub(crate) fn collides_at(
    instance: &RuntimeInstance,
    x: f64,
    y: f64,
    others: &[RuntimeInstance],
    ignore_runtime_id: Option<usize>,
) -> bool {
    let (left, top, right, bottom) = bounds_at(instance, x, y);

    others.iter().any(|other| {
        if !other.alive || ignore_runtime_id == Some(other.runtime_id) {
            return false;
        }

        let (other_left, other_top, other_right, other_bottom) = bounds_at(other, other.x, other.y);
        if !(left < other_right && right > other_left && top < other_bottom && bottom > other_top) {
            return false;
        }

        match (
            active_collision_mask(instance),
            active_collision_mask(other),
        ) {
            (Some(mask), Some(other_mask)) => masks_overlap(
                instance,
                mask,
                x,
                y,
                other,
                other_mask,
                (
                    left.max(other_left),
                    top.max(other_top),
                    right.min(other_right),
                    bottom.min(other_bottom),
                ),
            ),
            _ => true,
        }
    })
}

pub(crate) fn collision_candidates_near<F>(
    instance: &RuntimeInstance,
    x: f64,
    y: f64,
    others: &[RuntimeInstance],
    ignore_runtime_id: Option<usize>,
    padding: f64,
    predicate: F,
) -> Vec<RuntimeInstance>
where
    F: Fn(&RuntimeInstance) -> bool,
{
    collision_candidate_indices_near(
        instance,
        x,
        y,
        others,
        ignore_runtime_id,
        padding,
        predicate,
    )
    .into_iter()
    .filter_map(|index| others.get(index).cloned())
    .collect()
}

pub(crate) fn collision_candidate_indices_near<F>(
    instance: &RuntimeInstance,
    x: f64,
    y: f64,
    others: &[RuntimeInstance],
    ignore_runtime_id: Option<usize>,
    padding: f64,
    predicate: F,
) -> Vec<usize>
where
    F: Fn(&RuntimeInstance) -> bool,
{
    let padding = padding.max(0.0).ceil() as i32;
    let (left, top, right, bottom) = bounds_at(instance, x, y);
    let query_left = left - padding;
    let query_top = top - padding;
    let query_right = right + padding;
    let query_bottom = bottom + padding;

    others
        .iter()
        .enumerate()
        .filter(|(_, other)| ignore_runtime_id != Some(other.runtime_id))
        .filter(|(_, other)| predicate(other))
        .filter_map(|(index, other)| {
            let (other_left, other_top, other_right, other_bottom) =
                bounds_at(other, other.x, other.y);
            (query_left < other_right
                && query_right > other_left
                && query_top < other_bottom
                && query_bottom > other_top)
                .then_some(index)
        })
        .collect()
}

fn active_collision_mask(instance: &RuntimeInstance) -> Option<&RuntimeCollisionMask> {
    instance.collision_masks.first().filter(|mask| {
        let expected_len = mask.width.checked_mul(mask.height).unwrap_or(0) as usize;
        mask.width > 0 && mask.height > 0 && expected_len > 0 && mask.data.len() >= expected_len
    })
}

fn masks_overlap(
    instance: &RuntimeInstance,
    mask: &RuntimeCollisionMask,
    x: f64,
    y: f64,
    other: &RuntimeInstance,
    other_mask: &RuntimeCollisionMask,
    intersection: (i32, i32, i32, i32),
) -> bool {
    let (left, top, right, bottom) = intersection;
    for world_y in top..bottom {
        for world_x in left..right {
            if mask_contains_world_pixel(instance, mask, x, y, world_x, world_y)
                && mask_contains_world_pixel(other, other_mask, other.x, other.y, world_x, world_y)
            {
                return true;
            }
        }
    }
    false
}

fn mask_contains_world_pixel(
    instance: &RuntimeInstance,
    mask: &RuntimeCollisionMask,
    x: f64,
    y: f64,
    world_x: i32,
    world_y: i32,
) -> bool {
    let local_x = world_x - x.round() as i32 + instance.origin_x;
    let local_y = world_y - y.round() as i32 + instance.origin_y;
    if local_x < mask.bbox_left
        || local_x > mask.bbox_right
        || local_y < mask.bbox_top
        || local_y > mask.bbox_bottom
        || local_x < 0
        || local_y < 0
        || local_x >= mask.width as i32
        || local_y >= mask.height as i32
    {
        return false;
    }

    let index = local_y as usize * mask.width as usize + local_x as usize;
    mask.data.get(index).copied().unwrap_or(false)
}

pub(crate) fn move_instance_axis(
    instance: &mut RuntimeInstance,
    solids: &[RuntimeInstance],
    ignore_runtime_id: Option<usize>,
    axis: Axis,
    delta: f64,
) -> bool {
    let step = delta.signum();
    let mut remaining = delta.abs();

    while remaining >= 1.0 {
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
                Axis::Horizontal => instance.hspeed = 0.0,
                Axis::Vertical => instance.vspeed = 0.0,
            }
            return true;
        }

        instance.x = next_x;
        instance.y = next_y;
        remaining -= 1.0;
    }

    if remaining > f64::EPSILON {
        let next_x = match axis {
            Axis::Horizontal => instance.x + step * remaining,
            Axis::Vertical => instance.x,
        };
        let next_y = match axis {
            Axis::Horizontal => instance.y,
            Axis::Vertical => instance.y + step * remaining,
        };

        if collides_at(instance, next_x, next_y, solids, ignore_runtime_id) {
            match axis {
                Axis::Horizontal => instance.hspeed = 0.0,
                Axis::Vertical => instance.vspeed = 0.0,
            }
            return true;
        }

        instance.x = next_x;
        instance.y = next_y;
    }

    false
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

    if !collides_at(
        player,
        spawn_x as f64,
        spawn_y as f64,
        &solids,
        Some(player.runtime_id),
    ) {
        return (spawn_x, spawn_y);
    }

    for distance in 1..=player.height.max(player.width).max(16) {
        for (dx, dy) in [(0, -distance), (0, distance), (-distance, 0), (distance, 0)] {
            let x = spawn_x + dx;
            let y = spawn_y + dy;
            if spawn_candidate_is_inside_room(player, x, y, room)
                && !collides_at(player, x as f64, y as f64, &solids, Some(player.runtime_id))
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
    let (left, top, right, bottom) = bounds_at(player, x as f64, y as f64);
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
    if diagnostics.len() >= MAX_DIAGNOSTICS {
        diagnostics.remove(0);
    }
    diagnostics.push(diagnostic);
}
const MAX_DIAGNOSTICS: usize = 64;
