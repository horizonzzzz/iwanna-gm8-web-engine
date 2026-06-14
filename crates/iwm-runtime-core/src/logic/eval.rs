use std::collections::{HashMap, HashSet};

use iwm_runtime_host::{RuntimeButton, RuntimeHost};

use super::context::{RuntimeEvalContext, RuntimeExecutionScope};
use crate::helpers::{as_number, parse_runtime_value};
use crate::{LoweredLogicExpr, RuntimeInstance, RuntimeValue};

pub(super) fn is_truthy(value: Option<RuntimeValue>) -> bool {
    match value {
        Some(RuntimeValue::Bool(b)) => b,
        Some(RuntimeValue::Number(n)) => n >= 0.5,
        Some(RuntimeValue::Text(s)) => !s.is_empty(),
        None => false,
    }
}

pub(crate) fn assignable_key(
    expr: &LoweredLogicExpr,
    instance: Option<&RuntimeInstance>,
    scope: Option<&RuntimeExecutionScope>,
) -> Option<String> {
    match expr {
        LoweredLogicExpr::Identifier(name) => Some(name.clone()),
        LoweredLogicExpr::MemberAccess { target, member } => {
            let base = assignable_key(target, instance, scope)?;
            Some(format!("{base}.{member}"))
        }
        LoweredLogicExpr::IndexAccess { target, index } => {
            let base = assignable_key(target, instance, scope)?;
            let suffix = expr_key_fragment(index, instance, scope)?;
            Some(format!("{base}[{suffix}]"))
        }
        _ => None,
    }
}

fn expr_key_fragment(
    expr: &LoweredLogicExpr,
    instance: Option<&RuntimeInstance>,
    scope: Option<&RuntimeExecutionScope>,
) -> Option<String> {
    match expr {
        LoweredLogicExpr::Identifier(name) => Some(name.clone()),
        LoweredLogicExpr::LiteralNumber(number) => Some(if number.fract() == 0.0 {
            format!("{}", *number as i64)
        } else {
            number.to_string()
        }),
        LoweredLogicExpr::LiteralBool(flag) => Some(flag.to_string()),
        LoweredLogicExpr::LiteralText(text) => Some(text.clone()),
        _ => evaluate_expr(expr, instance, &HashMap::new(), scope, None)
            .map(runtime_value_to_string_text),
    }
}

fn runtime_value_to_string_text(value: RuntimeValue) -> String {
    match value {
        RuntimeValue::Number(number) if number.fract() == 0.0 => format!("{}", number as i64),
        RuntimeValue::Number(number) => number.to_string(),
        RuntimeValue::Bool(flag) => flag.to_string(),
        RuntimeValue::Text(text) => text,
    }
}

pub(super) fn evaluate_expr(
    expr: &LoweredLogicExpr,
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    match expr {
        LoweredLogicExpr::Identifier(name) => {
            if let Some(scope) = scope {
                if scope.is_local_key(name) {
                    return scope.get(name);
                }
            }
            if let Some(instance) = instance {
                if let Some(value) = instance.vars.get(name) {
                    return Some(value.clone());
                }
                match name.as_str() {
                    "x" => return Some(RuntimeValue::Number(instance.x as f64)),
                    "y" => return Some(RuntimeValue::Number(instance.y as f64)),
                    "hspeed" => return Some(RuntimeValue::Number(instance.hspeed as f64)),
                    "vspeed" => return Some(RuntimeValue::Number(instance.vspeed as f64)),
                    _ => {}
                }
            }

            if name.eq_ignore_ascii_case("off") {
                return Some(RuntimeValue::Bool(false));
            }

            if let Some(key_code) = gm_key_code(name) {
                return Some(RuntimeValue::Number(key_code as f64));
            }

            if let Some(context) = eval_context {
                if name.eq_ignore_ascii_case("room") {
                    return Some(RuntimeValue::Number(context.current_room_id as f64));
                }
                if let Some(room_id) = context.room_ids_by_name.get(&name.to_ascii_lowercase()) {
                    return Some(RuntimeValue::Number(*room_id as f64));
                }
            }

            globals.get(name).cloned().or_else(|| {
                eval_context
                    .and_then(|context| {
                        context
                            .place_target_ids_by_name
                            .get(&name.to_ascii_lowercase())
                            .and_then(|ids| ids.first().copied())
                    })
                    .map(|object_id| RuntimeValue::Number(object_id as f64))
            })
        }
        LoweredLogicExpr::LiteralNumber(number) => Some(RuntimeValue::Number(*number)),
        LoweredLogicExpr::LiteralBool(flag) => Some(RuntimeValue::Bool(*flag)),
        LoweredLogicExpr::LiteralText(text) => Some(RuntimeValue::Text(text.clone())),
        LoweredLogicExpr::UnaryExpr { op, child } => {
            let value = evaluate_expr(child, instance, globals, scope, eval_context)?;
            match op.as_str() {
                "-" => Some(RuntimeValue::Number(-as_number(&value)?)),
                "+" => Some(RuntimeValue::Number(as_number(&value)?)),
                "!" => Some(RuntimeValue::Bool(!is_truthy(Some(value)))),
                _ => None,
            }
        }
        LoweredLogicExpr::Call { name, args } => match name.as_str() {
            "room_goto" => args
                .first()
                .and_then(|arg| evaluate_expr(arg, instance, globals, scope, eval_context)),
            "ord" => evaluate_ord_call(args),
            "abs" => args
                .first()
                .and_then(|arg| evaluate_expr(arg, instance, globals, scope, eval_context))
                .and_then(|value| as_number(&value))
                .map(|value| RuntimeValue::Number(value.abs())),
            "floor" => args
                .first()
                .and_then(|arg| evaluate_expr(arg, instance, globals, scope, eval_context))
                .and_then(|value| as_number(&value))
                .map(|value| RuntimeValue::Number(value.floor())),
            "random" => evaluate_random_call(args, instance, globals, scope, eval_context),
            "choose" => evaluate_choose_call(args, instance, globals, scope, eval_context),
            "string" => args
                .first()
                .and_then(|arg| evaluate_expr(arg, instance, globals, scope, eval_context))
                .map(runtime_value_to_string_text)
                .map(RuntimeValue::Text),
            "file_exists" => evaluate_file_exists(args, instance, globals, scope, eval_context),
            "instance_exists" => evaluate_instance_exists(args, eval_context),
            "instance_number" => evaluate_instance_number(args, eval_context),
            "instance_place" => {
                evaluate_instance_place(args, instance, globals, scope, eval_context)
            }
            "distance_to_object" => {
                evaluate_distance_to_object(args, instance, globals, scope, eval_context)
            }
            "collision_line" => {
                evaluate_collision_line(args, instance, globals, scope, eval_context)
            }
            "keyboard_check"
            | "keyboard_check_direct"
            | "keyboard_check_pressed"
            | "keyboard_check_released" => {
                evaluate_keyboard_query(name, args, instance, globals, scope, eval_context)
            }
            "place_meeting" => {
                evaluate_place_query(args, instance, globals, scope, eval_context, true)
            }
            "place_free" => {
                evaluate_place_query(args, instance, globals, scope, eval_context, false)
            }
            _ => None,
        },
        LoweredLogicExpr::MemberAccess { target, member } => {
            if matches!(target.as_ref(), LoweredLogicExpr::Identifier(name) if name == "other") {
                let other = eval_context?.other_instance()?;
                return runtime_instance_member_value(other, member);
            }
            if let LoweredLogicExpr::Identifier(object_name) = target.as_ref() {
                if object_name != "global" {
                    if let Some(context) = eval_context {
                        let target_object_ids = context
                            .place_target_ids_by_name
                            .get(&object_name.to_ascii_lowercase());
                        if let Some((_, target_instance)) = target_object_ids.and_then(|ids| {
                            context
                                .room_instances_matching_object_ids(ids)
                                .find(|(_, candidate)| candidate.alive)
                        }) {
                            return runtime_instance_member_value(target_instance, member);
                        }
                    }
                }
            }
            if let Some(RuntimeValue::Number(instance_ref)) =
                evaluate_expr(target, instance, globals, scope, eval_context)
            {
                if let Some(target_instance) =
                    resolve_instance_reference(instance_ref, eval_context?)
                {
                    return runtime_instance_member_value(target_instance, member);
                }
            }
            let base = assignable_key(target, instance, scope)?;
            let key = format!("{base}.{member}");
            scope
                .and_then(|scope| scope.get(&key))
                .or_else(|| globals.get(&key).cloned())
                .or_else(|| instance.and_then(|instance| instance.vars.get(&key).cloned()))
        }
        LoweredLogicExpr::IndexAccess { target, index } => {
            let base = assignable_key(target, instance, scope)?;
            let suffix = expr_key_fragment(index, instance, scope)?;
            let key = format!("{base}[{suffix}]");
            scope
                .and_then(|scope| scope.get(&key))
                .or_else(|| globals.get(&key).cloned())
                .or_else(|| instance.and_then(|instance| instance.vars.get(&key).cloned()))
        }
        LoweredLogicExpr::BinaryExpr { op, left, right } => {
            if op == "&&" {
                let left = evaluate_expr(left, instance, globals, scope, eval_context)?;
                if !is_truthy(Some(left)) {
                    return Some(RuntimeValue::Bool(false));
                }
                let right = evaluate_expr(right, instance, globals, scope, eval_context)?;
                return Some(RuntimeValue::Bool(is_truthy(Some(right))));
            }

            if op == "||" {
                let left = evaluate_expr(left, instance, globals, scope, eval_context)?;
                if is_truthy(Some(left)) {
                    return Some(RuntimeValue::Bool(true));
                }
                let right = evaluate_expr(right, instance, globals, scope, eval_context)?;
                return Some(RuntimeValue::Bool(is_truthy(Some(right))));
            }

            let left = evaluate_expr(left, instance, globals, scope, eval_context)?;
            let right = evaluate_expr(right, instance, globals, scope, eval_context)?;
            eval_binary_expr(op, &left, &right)
        }
        LoweredLogicExpr::Raw { source } => parse_runtime_value(source),
    }
}

fn eval_binary_expr(op: &str, left: &RuntimeValue, right: &RuntimeValue) -> Option<RuntimeValue> {
    match op {
        "+" => Some(RuntimeValue::Number(as_number(left)? + as_number(right)?)),
        "-" => Some(RuntimeValue::Number(as_number(left)? - as_number(right)?)),
        "*" => Some(RuntimeValue::Number(as_number(left)? * as_number(right)?)),
        "/" => Some(RuntimeValue::Number(as_number(left)? / as_number(right)?)),
        "==" => Some(RuntimeValue::Bool(runtime_values_equal(left, right))),
        "=" => Some(RuntimeValue::Bool(runtime_values_equal(left, right))),
        "!=" => Some(RuntimeValue::Bool(!runtime_values_equal(left, right))),
        ">=" => Some(RuntimeValue::Bool(as_number(left)? >= as_number(right)?)),
        "<=" => Some(RuntimeValue::Bool(as_number(left)? <= as_number(right)?)),
        ">" => Some(RuntimeValue::Bool(as_number(left)? > as_number(right)?)),
        "<" => Some(RuntimeValue::Bool(as_number(left)? < as_number(right)?)),
        _ => None,
    }
}

fn runtime_values_equal(left: &RuntimeValue, right: &RuntimeValue) -> bool {
    match (as_number(left), as_number(right)) {
        (Some(left), Some(right)) => left == right,
        _ => left == right,
    }
}

fn gm_key_code(name: &str) -> Option<u16> {
    match name.to_ascii_lowercase().as_str() {
        "vk_left" => Some(0x25),
        "vk_up" => Some(0x26),
        "vk_right" => Some(0x27),
        "vk_down" => Some(0x28),
        "vk_shift" => Some(0x10),
        "vk_space" => Some(0x20),
        "vk_enter" => Some(0x0D),
        "vk_escape" => Some(0x1B),
        "vk_control" => Some(0x11),
        "vk_alt" => Some(0x12),
        _ => None,
    }
}

fn evaluate_ord_call(args: &[LoweredLogicExpr]) -> Option<RuntimeValue> {
    let first = args.first()?;
    match first {
        LoweredLogicExpr::LiteralText(text) => text
            .chars()
            .next()
            .map(|ch| RuntimeValue::Number(ch as u32 as f64)),
        _ => None,
    }
}

fn evaluate_random_call(
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

fn evaluate_choose_call(
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

fn evaluate_keyboard_query(
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

fn evaluate_place_query(
    args: &[LoweredLogicExpr],
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    want_meeting: bool,
) -> Option<RuntimeValue> {
    let context = eval_context?;
    let instance = instance?;
    let x = args
        .first()
        .and_then(|arg| evaluate_expr(arg, Some(instance), globals, scope, eval_context))
        .and_then(|value| as_number(&value))
        .map(|value| value.round() as i32)?;
    let y = args
        .get(1)
        .and_then(|arg| evaluate_expr(arg, Some(instance), globals, scope, eval_context))
        .and_then(|value| as_number(&value))
        .map(|value| value.round() as i32)?;
    let query_x = x as f64;
    let query_y = y as f64;
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
        context
            .solid_room_instances_near(instance, query_x, query_y)
            .any(|(_, candidate)| {
                crate::helpers::collides_with_instance_at(
                    instance,
                    query_x,
                    query_y,
                    candidate,
                    Some(instance.runtime_id),
                    |candidate| candidate.solid,
                )
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

fn evaluate_instance_place(
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

fn evaluate_file_exists(
    args: &[LoweredLogicExpr],
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    let context = eval_context?;
    let path = args
        .first()
        .and_then(|arg| evaluate_expr(arg, instance, globals, scope, eval_context))
        .and_then(|value| match value {
            RuntimeValue::Text(text) => Some(text),
            _ => None,
        })?;
    Some(RuntimeValue::Bool(context.known_files.contains(&path)))
}

fn evaluate_instance_exists(
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

fn evaluate_instance_number(
    args: &[LoweredLogicExpr],
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    let context = eval_context?;
    let target_object_ids = instance_target_object_ids(args.first()?, context)?;
    let count = context
        .room_instances_matching_object_ids(&target_object_ids)
        .filter(|(_, instance)| instance.alive)
        .count();
    Some(RuntimeValue::Number(count as f64))
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

fn resolve_instance_reference<'a>(
    instance_ref: f64,
    context: &'a RuntimeEvalContext<'_>,
) -> Option<&'a RuntimeInstance> {
    if !instance_ref.is_finite() {
        return None;
    }
    let rounded = instance_ref.round();
    context
        .room_instances_iter()
        .find(|(_, candidate)| {
            candidate.instance_id as f64 == rounded || candidate.runtime_id as f64 == rounded
        })
        .map(|(_, candidate)| candidate)
}

fn runtime_instance_member_value(instance: &RuntimeInstance, member: &str) -> Option<RuntimeValue> {
    match member {
        "x" => Some(RuntimeValue::Number(instance.x)),
        "y" => Some(RuntimeValue::Number(instance.y)),
        "hspeed" => Some(RuntimeValue::Number(instance.hspeed)),
        "vspeed" => Some(RuntimeValue::Number(instance.vspeed)),
        "object_index" => Some(RuntimeValue::Number(instance.object_id as f64)),
        _ => instance.vars.get(member).cloned(),
    }
}

fn evaluate_distance_to_object(
    args: &[LoweredLogicExpr],
    instance: Option<&RuntimeInstance>,
    _globals: &HashMap<String, RuntimeValue>,
    _scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
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
    let distance = context
        .room_instances_matching_object_ids(target_object_ids)
        .filter(|(_, candidate)| candidate.alive && candidate.runtime_id != instance.runtime_id)
        .map(|(_, candidate)| instance_bbox_distance(instance, candidate))
        .fold(1_000_000.0, f64::min);

    Some(RuntimeValue::Number(distance))
}

fn evaluate_collision_line(
    args: &[LoweredLogicExpr],
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
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
    let hit = context
        .room_instances_matching_object_ids(target_object_ids)
        .find(|(_, candidate)| {
            candidate.alive
                && (!exclude_self || current_runtime_id != Some(candidate.runtime_id))
                && line_intersects_instance(candidate, x1, y1, x2, y2, precise)
        })
        .map(|(_, candidate)| candidate.instance_id as f64)
        .unwrap_or(-4.0);

    Some(RuntimeValue::Number(hit))
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
pub(crate) fn sample_known_files<H: RuntimeHost>(host: &H) -> HashSet<String> {
    let mut files = HashSet::new();
    for candidate in ["temp", "DeathTime", "save1", "save2", "save3"] {
        if host.read(std::path::Path::new(candidate)).is_ok() {
            files.insert(candidate.to_string());
        }
    }
    files
}
