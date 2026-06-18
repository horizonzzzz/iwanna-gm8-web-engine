use std::collections::HashMap;

use super::context::{RuntimeEvalContext, RuntimeExecutionScope};
use crate::helpers::{as_number, parse_room_id};
use crate::{RuntimeInstance, RuntimeValue};

pub(super) fn assign_runtime_value(
    key: String,
    value: RuntimeValue,
    instance: &mut RuntimeInstance,
    globals: &mut HashMap<String, RuntimeValue>,
    scope: &mut RuntimeExecutionScope,
) {
    if scope.assign(&key, value.clone()) {
        return;
    }
    assign_instance_or_global(key, value, instance, globals);
}

pub(super) fn assign_instance_or_global(
    key: String,
    value: RuntimeValue,
    instance: &mut RuntimeInstance,
    globals: &mut HashMap<String, RuntimeValue>,
) {
    if key.starts_with("global.") || is_view_variable_key(&key) {
        globals.insert(key, value);
        return;
    }

    assign_instance_field_or_var(key, value, instance);
}

pub(super) fn assign_instance_field_or_var(
    key: String,
    value: RuntimeValue,
    instance: &mut RuntimeInstance,
) {
    match key.as_str() {
        "x" => assign_number_field(value, &mut instance.x, &mut instance.vars, key),
        "y" => assign_number_field(value, &mut instance.y, &mut instance.vars, key),
        "previous_x" => {
            assign_number_field(value, &mut instance.previous_x, &mut instance.vars, key)
        }
        "previous_y" => {
            assign_number_field(value, &mut instance.previous_y, &mut instance.vars, key)
        }
        "hspeed" => {
            if let Some(n) = as_number(&value) {
                instance.set_hspeed(n);
            } else {
                instance.vars.insert(key, value);
            }
        }
        "vspeed" => {
            if let Some(n) = as_number(&value) {
                instance.set_vspeed(n);
            } else {
                instance.vars.insert(key, value);
            }
        }
        "speed" => {
            if let Some(n) = as_number(&value) {
                instance.set_speed(n);
            } else {
                instance.vars.insert(key, value);
            }
        }
        "direction" => {
            if let Some(n) = as_number(&value) {
                instance.set_direction(n);
            } else {
                instance.vars.insert(key, value);
            }
        }
        _ => {
            instance.vars.insert(key, value);
        }
    }
}

pub(super) fn runtime_value_to_room_id(value: &RuntimeValue) -> Option<usize> {
    match value {
        RuntimeValue::Number(number) => {
            if number.is_finite() && *number >= 0.0 {
                Some(number.round() as usize)
            } else {
                None
            }
        }
        RuntimeValue::Bool(flag) => Some(if *flag { 1 } else { 0 }),
        RuntimeValue::Text(text) => parse_room_id(text),
    }
}

pub(super) fn next_room_id(
    _instance: &RuntimeInstance,
    _globals: &HashMap<String, RuntimeValue>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<usize> {
    let context = eval_context?;
    let current_index = context
        .room_order
        .iter()
        .position(|room_id| *room_id == context.current_room_id)?;
    context.room_order.get(current_index + 1).copied()
}

fn is_view_variable_key(key: &str) -> bool {
    matches!(
        key,
        "view_xview"
            | "view_yview"
            | "view_wview"
            | "view_hview"
            | "view_xview[0]"
            | "view_yview[0]"
            | "view_wview[0]"
            | "view_hview[0]"
    )
}

fn assign_number_field(
    value: RuntimeValue,
    field: &mut f64,
    vars: &mut HashMap<String, RuntimeValue>,
    key: String,
) {
    if let Some(number) = as_number(&value) {
        *field = number;
    } else {
        vars.insert(key, value);
    }
}
