use std::collections::HashMap;

use iwm_runtime_host::RuntimeButton;

use super::context::{RuntimeEvalContext, RuntimeExecutionScope};
use super::eval::evaluate_expr;
use super::eval_values::is_truthy;
use crate::helpers::as_number;
use crate::tick_context::RuntimeObjectQueryScratch;
use crate::{LoweredLogicExpr, RuntimeInstance, RuntimeValue};

pub(super) fn evaluate_ord_call(args: &[LoweredLogicExpr]) -> Option<RuntimeValue> {
    let first = args.first()?;
    match first {
        LoweredLogicExpr::LiteralText(text) => text
            .chars()
            .next()
            .map(|ch| RuntimeValue::Number(ch as u32 as f64)),
        _ => None,
    }
}

pub(super) fn evaluate_random_call(
    args: &[LoweredLogicExpr],
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    let max = args
        .first()
        .and_then(|arg| evaluate_expr(arg, instance, globals, scope, eval_context))
        .and_then(|value| as_number(&value))?;
    if max == 0.0 {
        return Some(RuntimeValue::Number(0.0));
    }
    let unit = deterministic_random_unit(instance, scope, max.to_bits());
    Some(RuntimeValue::Number(unit * max))
}

pub(super) fn evaluate_random_range_call(
    args: &[LoweredLogicExpr],
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    let min = args
        .first()
        .and_then(|arg| evaluate_expr(arg, instance, globals, scope, eval_context))
        .and_then(|value| as_number(&value))?;
    let max = args
        .get(1)
        .and_then(|arg| evaluate_expr(arg, instance, globals, scope, eval_context))
        .and_then(|value| as_number(&value))?;
    if min == max {
        return Some(RuntimeValue::Number(min));
    }
    let unit = deterministic_random_unit(instance, scope, min.to_bits() ^ max.to_bits());
    Some(RuntimeValue::Number(min + unit * (max - min)))
}

pub(super) fn evaluate_choose_call(
    args: &[LoweredLogicExpr],
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    if args.is_empty() {
        return None;
    }
    let values = args
        .iter()
        .filter_map(|arg| evaluate_expr(arg, instance, globals, scope, eval_context))
        .collect::<Vec<_>>();
    if values.is_empty() {
        return None;
    }
    let unit = deterministic_random_unit(instance, scope, values.len() as u64);
    let index = ((unit * values.len() as f64).floor() as usize).min(values.len() - 1);
    values.get(index).cloned()
}

pub(super) fn evaluate_keyboard_query(
    name: &str,
    args: &[LoweredLogicExpr],
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    let context = eval_context?;
    let key_code = args
        .first()
        .and_then(|arg| evaluate_expr(arg, instance, globals, scope, eval_context))
        .and_then(|value| as_number(&value))
        .map(|value| value.round() as u16)?;
    let state = context
        .button_states
        .get(&RuntimeButton::Keyboard(key_code))
        .copied()
        .unwrap_or_default();
    let result = match name {
        "keyboard_check" | "keyboard_check_direct" => state.pressed,
        "keyboard_check_pressed" => state.just_pressed,
        "keyboard_check_released" => state.just_released,
        _ => return None,
    };
    Some(RuntimeValue::Bool(result))
}

pub(super) fn evaluate_place_query(
    args: &[LoweredLogicExpr],
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    want_meeting: bool,
) -> Option<RuntimeValue> {
    let context = eval_context?;
    let instance = instance?;
    let query_x = args
        .first()
        .and_then(|arg| evaluate_expr(arg, Some(instance), globals, scope, eval_context))
        .and_then(|value| as_number(&value))?;
    let query_y = args
        .get(1)
        .and_then(|arg| evaluate_expr(arg, Some(instance), globals, scope, eval_context))
        .and_then(|value| as_number(&value))?;
    let collides = if let Some(target_expr) = args.get(2) {
        let target_object_ids = instance_target_object_ids(target_expr, context)?;
        context
            .room_instances_matching_object_ids_near(&target_object_ids, instance, query_x, query_y)
            .any(|(_, candidate)| {
                crate::helpers::collides_with_instance_at(
                    instance,
                    query_x,
                    query_y,
                    candidate,
                    Some(instance.runtime_id),
                    |_| true,
                )
            })
    } else if !want_meeting {
        let horizontal_query = (query_x - instance.x).abs() > f64::EPSILON
            && (query_y - instance.y).abs() <= f64::EPSILON;
        context
            .solid_room_instances_near(instance, query_x, query_y)
            .any(|(_, candidate)| {
                let collides = crate::helpers::collides_with_instance_at(
                    instance,
                    query_x,
                    query_y,
                    candidate,
                    Some(instance.runtime_id),
                    |candidate| candidate.solid,
                );
                collides
                    && !(horizontal_query
                        && horizontal_query_only_touches_supporting_top(
                            instance, query_x, query_y, candidate,
                        ))
            })
    } else {
        return None;
    };
    Some(RuntimeValue::Bool(if want_meeting {
        collides
    } else {
        !collides
    }))
}

pub(super) fn evaluate_instance_place(
    args: &[LoweredLogicExpr],
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    let context = eval_context?;
    let instance = instance?;
    let x = args
        .first()
        .and_then(|arg| evaluate_expr(arg, Some(instance), globals, scope, eval_context))
        .and_then(|value| as_number(&value))?;
    let y = args
        .get(1)
        .and_then(|arg| evaluate_expr(arg, Some(instance), globals, scope, eval_context))
        .and_then(|value| as_number(&value))?;
    let target_object_ids = instance_target_object_ids(args.get(2)?, context)?;
    let hit = context
        .room_instances_matching_object_ids_near(&target_object_ids, instance, x, y)
        .find(|(_, candidate)| {
            crate::helpers::collides_with_instance_at(
                instance,
                x,
                y,
                candidate,
                Some(instance.runtime_id),
                |_| true,
            )
        })
        .map(|(_, candidate)| candidate.instance_id as f64)
        .unwrap_or(-4.0);
    Some(RuntimeValue::Number(hit))
}

pub(super) fn evaluate_instance_exists(
    args: &[LoweredLogicExpr],
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    let context = eval_context?;
    let target_object_ids = instance_target_object_ids(args.first()?, context)?;
    let exists = context
        .room_instances_matching_object_ids(&target_object_ids)
        .any(|(_, instance)| instance.alive);
    Some(RuntimeValue::Bool(exists))
}

pub(super) fn evaluate_instance_number(
    args: &[LoweredLogicExpr],
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    let mut scratch = RuntimeObjectQueryScratch::default();
    evaluate_instance_number_with_scratch(args, eval_context, &mut scratch)
}

fn horizontal_query_only_touches_supporting_top(
    instance: &RuntimeInstance,
    query_x: f64,
    query_y: f64,
    candidate: &RuntimeInstance,
) -> bool {
    if crate::helpers::collides_with_instance_at(
        instance,
        instance.x,
        instance.y,
        candidate,
        Some(instance.runtime_id),
        |candidate| candidate.solid,
    ) {
        return false;
    }

    let (_, _, _, query_bottom) = crate::helpers::bounds_at(instance, query_x, query_y);
    let (_, candidate_top, _, _) = crate::helpers::bounds_at(candidate, candidate.x, candidate.y);
    query_bottom - candidate_top == 1
        && !crate::helpers::collides_with_instance_at(
            instance,
            query_x,
            query_y - 1.0,
            candidate,
            Some(instance.runtime_id),
            |candidate| candidate.solid,
        )
}

pub(super) fn evaluate_instance_number_with_scratch(
    args: &[LoweredLogicExpr],
    eval_context: Option<&RuntimeEvalContext<'_>>,
    scratch: &mut RuntimeObjectQueryScratch,
) -> Option<RuntimeValue> {
    let context = eval_context?;
    let target_object_ids = instance_target_object_ids(args.first()?, context)?;
    context.write_room_instance_indices_matching_object_ids(&target_object_ids, scratch);
    let count = scratch
        .candidates()
        .iter()
        .filter_map(|&index| context.room_instance(index))
        .filter(|instance| instance.alive)
        .count();
    Some(RuntimeValue::Number(count as f64))
}

pub(super) fn evaluate_distance_to_object(
    args: &[LoweredLogicExpr],
    instance: Option<&RuntimeInstance>,
    _globals: &HashMap<String, RuntimeValue>,
    _scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    let mut scratch = RuntimeObjectQueryScratch::default();
    evaluate_distance_to_object_with_scratch(args, instance, eval_context, &mut scratch)
}

pub(super) fn evaluate_distance_to_object_with_scratch(
    args: &[LoweredLogicExpr],
    instance: Option<&RuntimeInstance>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    scratch: &mut RuntimeObjectQueryScratch,
) -> Option<RuntimeValue> {
    let context = eval_context?;
    let instance = instance?;
    let object_name = args.first().and_then(|arg| match arg {
        LoweredLogicExpr::Identifier(name) => Some(name.as_str()),
        _ => None,
    })?;
    let target_object_ids = context
        .place_target_ids_by_name
        .get(&object_name.to_ascii_lowercase())?;
    context.write_room_instance_indices_matching_object_ids(target_object_ids, scratch);
    let distance = scratch
        .candidates()
        .iter()
        .filter_map(|&index| context.room_instance(index))
        .filter(|candidate| candidate.alive && candidate.runtime_id != instance.runtime_id)
        .map(|candidate| instance_bbox_distance(instance, candidate))
        .fold(1_000_000.0, f64::min);

    Some(RuntimeValue::Number(distance))
}

pub(super) fn evaluate_collision_line(
    args: &[LoweredLogicExpr],
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    let mut scratch = RuntimeObjectQueryScratch::default();
    evaluate_collision_line_with_scratch(args, instance, globals, scope, eval_context, &mut scratch)
}

pub(super) fn evaluate_collision_line_with_scratch(
    args: &[LoweredLogicExpr],
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    scratch: &mut RuntimeObjectQueryScratch,
) -> Option<RuntimeValue> {
    let context = eval_context?;
    let x1 = args
        .first()
        .and_then(|arg| evaluate_expr(arg, instance, globals, scope, eval_context))
        .and_then(|value| as_number(&value))?;
    let y1 = args
        .get(1)
        .and_then(|arg| evaluate_expr(arg, instance, globals, scope, eval_context))
        .and_then(|value| as_number(&value))?;
    let x2 = args
        .get(2)
        .and_then(|arg| evaluate_expr(arg, instance, globals, scope, eval_context))
        .and_then(|value| as_number(&value))?;
    let y2 = args
        .get(3)
        .and_then(|arg| evaluate_expr(arg, instance, globals, scope, eval_context))
        .and_then(|value| as_number(&value))?;
    let object_name = args.get(4).and_then(|arg| match arg {
        LoweredLogicExpr::Identifier(name) => Some(name.as_str()),
        _ => None,
    })?;
    let precise = args
        .get(5)
        .and_then(|arg| evaluate_expr(arg, instance, globals, scope, eval_context))
        .map(|value| is_truthy(Some(value)))
        .unwrap_or(false);
    let exclude_self = args
        .get(6)
        .and_then(|arg| evaluate_expr(arg, instance, globals, scope, eval_context))
        .map(|value| is_truthy(Some(value)))
        .unwrap_or(false);
    let current_runtime_id = instance.map(|instance| instance.runtime_id);
    let target_object_ids = context
        .place_target_ids_by_name
        .get(&object_name.to_ascii_lowercase())?;
    context.write_room_instance_indices_matching_object_ids(target_object_ids, scratch);
    let hit = scratch
        .candidates()
        .iter()
        .filter_map(|&index| context.room_instance(index))
        .find(|candidate| {
            candidate.alive
                && (!exclude_self || current_runtime_id != Some(candidate.runtime_id))
                && line_intersects_instance(candidate, x1, y1, x2, y2, precise)
        })
        .map(|candidate| candidate.instance_id as f64)
        .unwrap_or(-4.0);

    Some(RuntimeValue::Number(hit))
}

fn deterministic_random_unit(
    instance: Option<&RuntimeInstance>,
    scope: Option<&RuntimeExecutionScope>,
    salt: u64,
) -> f64 {
    let mut seed = salt ^ 0x9e37_79b9_7f4a_7c15;
    if let Some(instance) = instance {
        seed ^= (instance.runtime_id as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        seed ^= instance.x.to_bits().rotate_left(17);
        seed ^= instance.y.to_bits().rotate_left(29);
    }
    let loop_index = scope
        .and_then(|scope| scope.get("i"))
        .or_else(|| instance.and_then(|instance| instance.vars.get("i").cloned()));
    if let Some(RuntimeValue::Number(i)) = loop_index {
        seed ^= i.to_bits().rotate_left(7);
    }
    seed = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
    seed = (seed ^ (seed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    seed = (seed ^ (seed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    let mixed = seed ^ (seed >> 31);
    (mixed as f64) / (u64::MAX as f64)
}

fn instance_target_object_ids(
    expr: &LoweredLogicExpr,
    context: &RuntimeEvalContext<'_>,
) -> Option<Vec<usize>> {
    match expr {
        LoweredLogicExpr::Identifier(name) => context
            .place_target_ids_by_name
            .get(&name.to_ascii_lowercase())
            .cloned(),
        LoweredLogicExpr::LiteralNumber(number) if number.is_finite() && *number >= 0.0 => {
            Some(vec![number.round() as usize])
        }
        _ => None,
    }
}

fn instance_bbox_distance(
    left_instance: &RuntimeInstance,
    right_instance: &RuntimeInstance,
) -> f64 {
    let (left, top, right, bottom) = inclusive_bounds_at(left_instance);
    let (other_left, other_top, other_right, other_bottom) = inclusive_bounds_at(right_instance);
    let dx = if left > other_right {
        left - other_right
    } else if other_left > right {
        other_left - right
    } else {
        0
    };
    let dy = if top > other_bottom {
        top - other_bottom
    } else if other_top > bottom {
        other_top - bottom
    } else {
        0
    };

    match (dx, dy) {
        (0, 0) => 0.0,
        (x, 0) => x as f64,
        (0, y) => y as f64,
        (x, y) => (x as f64).hypot(y as f64),
    }
}

fn inclusive_bounds_at(instance: &RuntimeInstance) -> (i32, i32, i32, i32) {
    let x = instance.x.round() as i32;
    let y = instance.y.round() as i32;
    (
        x - instance.origin_x + instance.bbox_left,
        y - instance.origin_y + instance.bbox_top,
        x - instance.origin_x + instance.bbox_right,
        y - instance.origin_y + instance.bbox_bottom,
    )
}

fn line_intersects_instance(
    instance: &RuntimeInstance,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    precise: bool,
) -> bool {
    let (left, top, right, bottom) = inclusive_bounds_at(instance);
    if !line_intersects_inclusive_rect(x1, y1, x2, y2, left, top, right, bottom) {
        return false;
    }
    if !precise || instance.collision_masks.is_empty() {
        return true;
    }
    line_points(x1, y1, x2, y2).any(|(x, y)| instance_mask_contains_point(instance, x, y))
}

fn line_intersects_inclusive_rect(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
) -> bool {
    let rect_left = left as f64;
    let rect_top = top as f64;
    let rect_right = (right + 1) as f64;
    let rect_bottom = (bottom + 1) as f64;
    let mut t0 = 0.0;
    let mut t1 = 1.0;
    clip_line_axis(-(x2 - x1), x1 - rect_left, &mut t0, &mut t1)
        && clip_line_axis(x2 - x1, rect_right - x1, &mut t0, &mut t1)
        && clip_line_axis(-(y2 - y1), y1 - rect_top, &mut t0, &mut t1)
        && clip_line_axis(y2 - y1, rect_bottom - y1, &mut t0, &mut t1)
}

fn clip_line_axis(p: f64, q: f64, t0: &mut f64, t1: &mut f64) -> bool {
    if p == 0.0 {
        return q >= 0.0;
    }
    let r = q / p;
    if p < 0.0 {
        if r > *t1 {
            return false;
        }
        if r > *t0 {
            *t0 = r;
        }
    } else {
        if r < *t0 {
            return false;
        }
        if r < *t1 {
            *t1 = r;
        }
    }
    true
}

fn line_points(x1: f64, y1: f64, x2: f64, y2: f64) -> impl Iterator<Item = (i32, i32)> {
    let x1 = x1.round() as i32;
    let y1 = y1.round() as i32;
    let x2 = x2.round() as i32;
    let y2 = y2.round() as i32;
    let steps = (x2 - x1).abs().max((y2 - y1).abs());
    (0..=steps).map(move |step| {
        if steps == 0 {
            (x1, y1)
        } else {
            let t = step as f64 / steps as f64;
            (
                (x1 as f64 + (x2 - x1) as f64 * t).round() as i32,
                (y1 as f64 + (y2 - y1) as f64 * t).round() as i32,
            )
        }
    })
}

fn instance_mask_contains_point(instance: &RuntimeInstance, world_x: i32, world_y: i32) -> bool {
    let Some(mask) = instance.collision_masks.first() else {
        return false;
    };
    let expected_len = mask.width.checked_mul(mask.height).unwrap_or(0) as usize;
    if mask.width == 0 || mask.height == 0 || mask.data.len() < expected_len {
        return false;
    }
    let local_x = world_x - instance.x.round() as i32 + instance.origin_x;
    let local_y = world_y - instance.y.round() as i32 + instance.origin_y;
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
