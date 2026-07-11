use std::collections::HashMap;

use iwm_runtime_model::SpriteResource;

use super::context::{RuntimeEvalContext, RuntimeExecutionScope};
use crate::helpers::{as_number, parse_room_id};
use crate::{RuntimeInstance, RuntimeValue};

pub(super) fn assign_runtime_value(
    key: String,
    value: RuntimeValue,
    instance: &mut RuntimeInstance,
    globals: &mut HashMap<String, RuntimeValue>,
    scope: &mut RuntimeExecutionScope,
    room_speed: Option<&mut u32>,
    sprites: &[SpriteResource],
    sprite_index: &HashMap<usize, usize>,
) {
    if scope.assign(&key, value.clone()) {
        return;
    }
    assign_instance_or_global(
        key,
        value,
        instance,
        globals,
        room_speed,
        sprites,
        sprite_index,
    );
}

pub(super) fn assign_instance_or_global(
    key: String,
    value: RuntimeValue,
    instance: &mut RuntimeInstance,
    globals: &mut HashMap<String, RuntimeValue>,
    room_speed: Option<&mut u32>,
    sprites: &[SpriteResource],
    sprite_index: &HashMap<usize, usize>,
) {
    if assign_room_speed(&key, &value, room_speed) {
        return;
    }

    if key.starts_with("global.") || is_view_variable_key(&key) {
        globals.insert(key, value);
        return;
    }

    assign_instance_field_or_var(key, value, instance, sprites, sprite_index);
}

pub(super) fn assign_instance_field_or_var(
    key: String,
    value: RuntimeValue,
    instance: &mut RuntimeInstance,
    sprites: &[SpriteResource],
    sprite_index: &HashMap<usize, usize>,
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
        "sprite_index" => {
            let Some(sprite_id) = as_number(&value)
                .filter(|value| value.is_finite())
                .map(|value| value.round() as i32)
            else {
                instance.vars.insert(key, value);
                return;
            };
            let frame_count = usize::try_from(sprite_id)
                .ok()
                .and_then(|id| sprite_index.get(&id))
                .and_then(|index| sprites.get(*index))
                .map(|sprite| sprite.frame_paths.len())
                .unwrap_or(0);
            instance.vars.insert(
                "sprite_index".into(),
                RuntimeValue::Number(sprite_id as f64),
            );

            let image_index = instance
                .vars
                .get("image_index")
                .and_then(as_number)
                .unwrap_or(0.0);
            if image_index.floor() >= frame_count as f64 {
                instance
                    .vars
                    .insert("image_index".into(), RuntimeValue::Number(0.0));
            }
        }
        _ => {
            instance.vars.insert(key, value);
        }
    }
}

pub(super) fn assign_room_speed(
    key: &str,
    value: &RuntimeValue,
    room_speed: Option<&mut u32>,
) -> bool {
    if !key.eq_ignore_ascii_case("room_speed") {
        return false;
    }

    if let Some(room_speed) = room_speed {
        if let Some(speed) = runtime_value_to_room_speed(value) {
            *room_speed = speed;
        }
    }
    true
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

fn runtime_value_to_room_speed(value: &RuntimeValue) -> Option<u32> {
    let number = as_number(value)?;
    if !number.is_finite() {
        return None;
    }
    let speed = number.trunc();
    if speed >= 1.0 && speed <= i32::MAX as f64 {
        Some(speed as u32)
    } else {
        None
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
