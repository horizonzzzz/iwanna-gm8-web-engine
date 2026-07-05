use std::collections::HashMap;

use super::context::{RuntimeEvalContext, RuntimeExecutionScope};
use super::eval::evaluate_expr;
use super::eval_values::runtime_value_to_string_text;
use crate::{LoweredLogicExpr, RuntimeInstance, RuntimeValue};

pub(crate) fn assignable_key(
    expr: &LoweredLogicExpr,
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<String> {
    match expr {
        LoweredLogicExpr::Identifier(name) => Some(name.clone()),
        LoweredLogicExpr::MemberAccess { target, member } => {
            let base = assignable_key(target, instance, globals, scope, eval_context)?;
            Some(format!("{base}.{member}"))
        }
        LoweredLogicExpr::IndexAccess { target, index } => {
            let base = assignable_key(target, instance, globals, scope, eval_context)?;
            let suffix = expr_key_fragment(index, instance, globals, scope, eval_context)?;
            Some(format!("{base}[{suffix}]"))
        }
        _ => None,
    }
}

pub(super) fn evaluate_identifier(
    name: &str,
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    if let Some(scope) = scope {
        if scope.is_local_key(name) {
            return scope.get(name);
        }
    }

    if let Some(context) = eval_context {
        if name.eq_ignore_ascii_case("room") {
            return Some(RuntimeValue::Number(context.current_room_id as f64));
        }
        if name.eq_ignore_ascii_case("room_speed") {
            return Some(RuntimeValue::Number(context.room_speed as f64));
        }
    }

    if let Some(instance) = instance {
        if let Some(value) = instance.vars.get(name) {
            return Some(value.clone());
        }
        match name {
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

pub(super) fn evaluate_member_access(
    target: &LoweredLogicExpr,
    member: &str,
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    if matches!(target, LoweredLogicExpr::Identifier(name) if name == "other") {
        let other = eval_context?.other_instance()?;
        return runtime_instance_member_value(other, member);
    }
    if let LoweredLogicExpr::Identifier(object_name) = target {
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
        if let Some(target_instance) = resolve_instance_reference(instance_ref, eval_context?) {
            return runtime_instance_member_value(target_instance, member);
        }
    }
    let base = assignable_key(target, instance, globals, scope, eval_context)?;
    let key = format!("{base}.{member}");
    scope
        .and_then(|scope| scope.get(&key))
        .or_else(|| globals.get(&key).cloned())
        .or_else(|| instance.and_then(|instance| instance.vars.get(&key).cloned()))
}

pub(super) fn evaluate_index_access(
    target: &LoweredLogicExpr,
    index: &LoweredLogicExpr,
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    let base = assignable_key(target, instance, globals, scope, eval_context)?;
    let suffix = expr_key_fragment(index, instance, globals, scope, eval_context)?;
    let key = format!("{base}[{suffix}]");
    scope
        .and_then(|scope| scope.get(&key))
        .or_else(|| globals.get(&key).cloned())
        .or_else(|| instance.and_then(|instance| instance.vars.get(&key).cloned()))
}

fn expr_key_fragment(
    expr: &LoweredLogicExpr,
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<String> {
    match expr {
        LoweredLogicExpr::Identifier(name) => {
            evaluate_expr(expr, instance, globals, scope, eval_context)
                .map(runtime_value_to_string_text)
                .or_else(|| Some(name.clone()))
        }
        LoweredLogicExpr::LiteralNumber(number) => Some(if number.fract() == 0.0 {
            format!("{}", *number as i64)
        } else {
            number.to_string()
        }),
        LoweredLogicExpr::LiteralBool(flag) => Some(flag.to_string()),
        LoweredLogicExpr::LiteralText(text) => Some(text.clone()),
        _ => evaluate_expr(expr, instance, globals, scope, eval_context)
            .map(runtime_value_to_string_text),
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
