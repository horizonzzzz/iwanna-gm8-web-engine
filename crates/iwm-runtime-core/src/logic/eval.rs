use std::collections::{HashMap, HashSet};

use iwm_runtime_host::{RuntimeButton, RuntimeHost};

use super::context::{RuntimeEvalContext, RuntimeExecutionScope};
use crate::helpers::{as_number, parse_runtime_value};
use crate::{LoweredLogicExpr, RuntimeInstance, RuntimeValue};

pub(super) fn is_truthy(value: Option<RuntimeValue>) -> bool {
    match value {
        Some(RuntimeValue::Bool(b)) => b,
        Some(RuntimeValue::Number(n)) => n != 0.0,
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

            if let Some(key_code) = gm_key_code(name) {
                return Some(RuntimeValue::Number(key_code as f64));
            }

            globals.get(name).cloned()
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
            "string" => args
                .first()
                .and_then(|arg| evaluate_expr(arg, instance, globals, scope, eval_context))
                .map(runtime_value_to_string_text)
                .map(RuntimeValue::Text),
            "file_exists" => evaluate_file_exists(args, instance, globals, scope, eval_context),
            "instance_exists" => evaluate_instance_exists(args, eval_context),
            "distance_to_object" => {
                evaluate_distance_to_object(args, instance, globals, scope, eval_context)
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
                return match member.as_str() {
                    "x" => Some(RuntimeValue::Number(other.x)),
                    "y" => Some(RuntimeValue::Number(other.y)),
                    "hspeed" => Some(RuntimeValue::Number(other.hspeed)),
                    "vspeed" => Some(RuntimeValue::Number(other.vspeed)),
                    _ => other.vars.get(member).cloned(),
                };
            }
            if let LoweredLogicExpr::Identifier(object_name) = target.as_ref() {
                if object_name != "global" {
                    if let Some(context) = eval_context {
                        if let Some((_, target_instance)) =
                            context.room_instances_iter().find(|(_, candidate)| {
                                candidate.alive
                                    && candidate.object_name.eq_ignore_ascii_case(object_name)
                            })
                        {
                            return match member.as_str() {
                                "x" => Some(RuntimeValue::Number(target_instance.x)),
                                "y" => Some(RuntimeValue::Number(target_instance.y)),
                                "hspeed" => Some(RuntimeValue::Number(target_instance.hspeed)),
                                "vspeed" => Some(RuntimeValue::Number(target_instance.vspeed)),
                                _ => target_instance.vars.get(member).cloned(),
                            };
                        }
                    }
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
        "==" => Some(RuntimeValue::Bool(left == right)),
        "=" => Some(RuntimeValue::Bool(left == right)),
        "!=" => Some(RuntimeValue::Bool(left != right)),
        ">=" => Some(RuntimeValue::Bool(as_number(left)? >= as_number(right)?)),
        "<=" => Some(RuntimeValue::Bool(as_number(left)? <= as_number(right)?)),
        ">" => Some(RuntimeValue::Bool(as_number(left)? > as_number(right)?)),
        "<" => Some(RuntimeValue::Bool(as_number(left)? < as_number(right)?)),
        _ => None,
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
    let object_name = args.get(2).and_then(|arg| match arg {
        LoweredLogicExpr::Identifier(name) => Some(name.as_str()),
        _ => None,
    })?;
    let targets = match context
        .place_target_ids_by_name
        .get(&object_name.to_ascii_lowercase())
    {
        Some(target_object_ids) => context
            .room_instances_iter()
            .filter(|(_, candidate)| {
                candidate.alive && target_object_ids.contains(&candidate.object_id)
            })
            .map(|(_, candidate)| candidate)
            .cloned()
            .collect::<Vec<_>>(),
        None => Vec::new(),
    };
    let collides = !targets.is_empty()
        && crate::helpers::collides_at(
            instance,
            x as f64,
            y as f64,
            &targets,
            Some(instance.runtime_id),
        );
    Some(RuntimeValue::Bool(if want_meeting {
        collides
    } else {
        !collides
    }))
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
    let object_name = args.first().and_then(|arg| match arg {
        LoweredLogicExpr::Identifier(name) => Some(name.as_str()),
        _ => None,
    })?;
    let exists = context.room_instances_iter().any(|(_, instance)| {
        instance.alive && instance.object_name.eq_ignore_ascii_case(object_name)
    });
    Some(RuntimeValue::Bool(exists))
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
        .room_instances_iter()
        .filter(|(_, candidate)| {
            candidate.alive
                && candidate.runtime_id != instance.runtime_id
                && target_object_ids.contains(&candidate.object_id)
        })
        .map(|(_, candidate)| instance_bbox_distance(instance, candidate))
        .fold(1_000_000.0, f64::min);

    Some(RuntimeValue::Number(distance))
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
pub(crate) fn sample_known_files<H: RuntimeHost>(host: &H) -> HashSet<String> {
    let mut files = HashSet::new();
    for candidate in ["temp", "DeathTime", "save1", "save2", "save3"] {
        if host.read(std::path::Path::new(candidate)).is_ok() {
            files.insert(candidate.to_string());
        }
    }
    files
}
