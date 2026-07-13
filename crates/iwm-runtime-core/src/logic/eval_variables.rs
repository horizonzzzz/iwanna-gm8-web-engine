use std::collections::HashMap;

use super::context::{RuntimeEvalContext, RuntimeExecutionScope};
use super::eval::evaluate_expr;
use super::eval_values::runtime_value_to_string_text;
use crate::{LoweredLogicExpr, RuntimeInstance, RuntimeValue};

pub(super) fn evaluate_expr_with_sprite_constants(
    expr: &LoweredLogicExpr,
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    sprite_ids_by_name: &HashMap<String, usize>,
) -> Option<RuntimeValue> {
    evaluate_expr(expr, instance, globals, scope, eval_context).or_else(|| {
        let LoweredLogicExpr::Identifier(name) = expr else {
            return None;
        };
        sprite_ids_by_name
            .get(&name.to_ascii_lowercase())
            .map(|sprite_id| RuntimeValue::Number(*sprite_id as f64))
    })
}

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
        if let Some(value) = default_instance_variable(name) {
            return Some(value);
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

    globals
        .get(name)
        .cloned()
        .or_else(|| eval_context.and_then(|context| view_variable_value(name, context)))
        .or_else(|| {
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

fn default_instance_variable(name: &str) -> Option<RuntimeValue> {
    let number = match name {
        "image_alpha" | "image_xscale" | "image_yscale" | "image_speed" => 1.0,
        "image_index" => 0.0,
        "visible" => return Some(RuntimeValue::Bool(true)),
        _ => return None,
    };
    Some(RuntimeValue::Number(number))
}

pub(super) fn evaluate_member_access(
    target: &LoweredLogicExpr,
    member: &str,
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    if !matches!(target, LoweredLogicExpr::Identifier(name) if name == "global") {
        if let Some(target_instance) =
            resolve_instance_receiver(target, instance, globals, scope, eval_context)
        {
            return runtime_instance_member_value(target_instance, member);
        }
    }
    let base = assignable_key(target, instance, globals, scope, eval_context)?;
    let key = format!("{base}.{member}");
    scope
        .and_then(|scope| scope.get(&key))
        .or_else(|| globals.get(&key).cloned())
        .or_else(|| instance.and_then(|instance| instance.vars.get(&key).cloned()))
        .or_else(|| (base == "global").then_some(RuntimeValue::Number(0.0)))
}

pub(super) fn evaluate_index_access(
    target: &LoweredLogicExpr,
    index: &LoweredLogicExpr,
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    if let LoweredLogicExpr::MemberAccess { target, member } = target {
        if !matches!(target.as_ref(), LoweredLogicExpr::Identifier(name) if name == "global") {
            let suffix = expr_key_fragment(index, instance, globals, scope, eval_context)?;
            if let Some(target_instance) =
                resolve_instance_receiver(target, instance, globals, scope, eval_context)
            {
                return runtime_instance_member_value(
                    target_instance,
                    &format!("{member}[{suffix}]"),
                );
            }
        }
    }
    let base = assignable_key(target, instance, globals, scope, eval_context)?;
    let suffix = expr_key_fragment(index, instance, globals, scope, eval_context)?;
    let key = format!("{base}[{suffix}]");
    scope
        .and_then(|scope| scope.get(&key))
        .or_else(|| globals.get(&key).cloned())
        .or_else(|| instance.and_then(|instance| instance.vars.get(&key).cloned()))
        .or_else(|| {
            (suffix == "0" && is_view_variable_name(&base))
                .then(|| globals.get(&base).cloned())
                .flatten()
        })
        .or_else(|| {
            (suffix == "0" && is_view_variable_name(&base))
                .then(|| eval_context.and_then(|context| view_variable_value(&base, context)))
                .flatten()
        })
}

fn view_variable_value(name: &str, context: &RuntimeEvalContext<'_>) -> Option<RuntimeValue> {
    let view = context.view_zero?;
    let value = match name {
        "view_xview" => view.x as f64,
        "view_yview" => view.y as f64,
        "view_wview" => view.width as f64,
        "view_hview" => view.height as f64,
        _ => return None,
    };
    Some(RuntimeValue::Number(value))
}

fn is_view_variable_name(name: &str) -> bool {
    matches!(
        name,
        "view_xview" | "view_yview" | "view_wview" | "view_hview"
    )
}

pub(super) fn instance_member_access<'a>(
    expr: &'a LoweredLogicExpr,
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<(&'a LoweredLogicExpr, String)> {
    match expr {
        LoweredLogicExpr::MemberAccess { target, member } => {
            Some((target.as_ref(), member.clone()))
        }
        LoweredLogicExpr::IndexAccess { target, index } => {
            let LoweredLogicExpr::MemberAccess { target, member } = target.as_ref() else {
                return None;
            };
            let suffix = expr_key_fragment(index, instance, globals, scope, eval_context)?;
            Some((target.as_ref(), format!("{member}[{suffix}]")))
        }
        _ => None,
    }
}

pub(super) fn expr_key_fragment(
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

fn resolve_instance_receiver<'a>(
    receiver: &LoweredLogicExpr,
    instance: Option<&'a RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&'a RuntimeEvalContext<'_>>,
) -> Option<&'a RuntimeInstance> {
    if matches!(receiver, LoweredLogicExpr::Identifier(name) if name == "self") {
        return instance;
    }
    if matches!(receiver, LoweredLogicExpr::Identifier(name) if name == "other") {
        return eval_context?.other_instance();
    }
    if let LoweredLogicExpr::Identifier(object_name) = receiver {
        let is_local = scope
            .map(|scope| scope.is_local_key(object_name))
            .unwrap_or(false);
        if !is_local {
            let context = eval_context?;
            if let Some(target_object_ids) = context
                .place_target_ids_by_name
                .get(&object_name.to_ascii_lowercase())
            {
                return context
                    .room_instances_matching_object_ids(target_object_ids)
                    .find(|(_, candidate)| candidate.alive)
                    .map(|(_, candidate)| candidate);
            }
        }
    }
    let RuntimeValue::Number(instance_ref) =
        evaluate_expr(receiver, instance, globals, scope, eval_context)?
    else {
        return None;
    };
    resolve_instance_reference(instance_ref, eval_context?)
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
