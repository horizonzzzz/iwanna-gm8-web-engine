use std::collections::HashMap;

use iwm_runtime_host::{RuntimeHost, RuntimeSoundMode};

use super::context::{RuntimeEvalContext, RuntimeExecutionScope};
use super::diagnostics::trace_message;
use super::eval::evaluate_expr;
use super::statement::{evaluate_with_diagnostics, RuntimeStatementEnvironment};
use crate::helpers::{as_number, collides_with_instance_at, record_host_diagnostic};
use crate::{LoweredLogicExpr, RuntimeInstance, RuntimeValue};

pub(super) fn dispatch_runtime_sound_call<H: RuntimeHost>(
    env: &mut RuntimeStatementEnvironment<'_, H>,
    function_name: &str,
    args: &[LoweredLogicExpr],
    mode: Option<RuntimeSoundMode>,
    instance: &RuntimeInstance,
    scope: &RuntimeExecutionScope,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) {
    let Some(sound_id) = args.first().and_then(|arg| {
        resolve_runtime_sound_id(
            arg,
            instance,
            Some(scope),
            eval_context,
            env.globals,
            env.sound_index,
        )
    }) else {
        record_host_diagnostic(
            env.host,
            env.diagnostics,
            iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
            "runtime-sound-unresolved",
            format!(
                "{} function={} arg_count={}",
                trace_message(&env.trace, instance),
                function_name,
                args.len()
            ),
        );
        return;
    };

    let result = if let Some(mode) = mode {
        env.host.play_sound(sound_id, mode)
    } else {
        env.host.stop_sound(sound_id)
    };

    if let Err(error) = result {
        record_host_diagnostic(
            env.host,
            env.diagnostics,
            iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
            "runtime-audio-host-error",
            format!(
                "{} function={} sound_id={} error={}",
                trace_message(&env.trace, instance),
                function_name,
                sound_id,
                error
            ),
        );
    }
}

pub(super) fn resolve_runtime_sound_id(
    expr: &LoweredLogicExpr,
    instance: &RuntimeInstance,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    globals: &HashMap<String, RuntimeValue>,
    sound_index: &HashMap<String, i32>,
) -> Option<i32> {
    match expr {
        LoweredLogicExpr::Identifier(name) | LoweredLogicExpr::LiteralText(name) => {
            sound_index.get(&name.to_ascii_lowercase()).copied()
        }
        LoweredLogicExpr::LiteralNumber(number) => finite_sound_number_to_id(*number),
        _ => evaluate_expr(expr, Some(instance), globals, scope, eval_context)
            .and_then(|value| runtime_value_to_sound_id(value, sound_index)),
    }
}

pub(super) fn dispatch_move_contact_solid<H: RuntimeHost>(
    env: &mut RuntimeStatementEnvironment<'_, H>,
    args: &[LoweredLogicExpr],
    instance: &mut RuntimeInstance,
    scope: &RuntimeExecutionScope,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) {
    let Some(context) = eval_context else {
        return;
    };
    let Some(direction) = args
        .first()
        .and_then(|arg| {
            evaluate_with_diagnostics(
                arg,
                Some(instance),
                Some(scope),
                eval_context,
                env,
                instance,
            )
        })
        .and_then(|value| as_number(&value))
        .filter(|value| value.is_finite())
    else {
        return;
    };
    let max_distance = args
        .get(1)
        .and_then(|arg| {
            evaluate_with_diagnostics(
                arg,
                Some(instance),
                Some(scope),
                eval_context,
                env,
                instance,
            )
        })
        .and_then(|value| as_number(&value))
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| value.round().clamp(0.0, 1000.0) as usize)
        .unwrap_or(1000);

    if context
        .solid_room_instances_near(instance, instance.x, instance.y)
        .any(|(_, candidate)| {
            collides_with_instance_at(
                instance,
                instance.x,
                instance.y,
                candidate,
                Some(instance.runtime_id),
                |candidate| candidate.solid,
            )
        })
    {
        return;
    }

    let radians = direction.to_radians();
    let step_x = radians.cos();
    let step_y = -radians.sin();
    for _ in 0..max_distance {
        let old_x = instance.x;
        let old_y = instance.y;
        instance.x += step_x;
        instance.y += step_y;

        if context
            .solid_room_instances_near(instance, instance.x, instance.y)
            .any(|(_, candidate)| {
                collides_with_instance_at(
                    instance,
                    instance.x,
                    instance.y,
                    candidate,
                    Some(instance.runtime_id),
                    |candidate| candidate.solid,
                )
            })
        {
            instance.x = old_x;
            instance.y = old_y;
            break;
        }
    }
}

pub(super) fn evaluate_file_bin_handle<H: RuntimeHost>(
    expr: Option<&LoweredLogicExpr>,
    instance: &RuntimeInstance,
    scope: &RuntimeExecutionScope,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
) -> Option<i32> {
    expr.and_then(|arg| {
        evaluate_with_diagnostics(
            arg,
            Some(instance),
            Some(scope),
            eval_context,
            env,
            instance,
        )
    })
    .and_then(|value| runtime_value_to_i32(&value))
}

pub(super) fn evaluate_file_bin_byte<H: RuntimeHost>(
    expr: Option<&LoweredLogicExpr>,
    instance: &RuntimeInstance,
    scope: &RuntimeExecutionScope,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
) -> Option<u8> {
    let number = expr
        .and_then(|arg| {
            evaluate_with_diagnostics(
                arg,
                Some(instance),
                Some(scope),
                eval_context,
                env,
                instance,
            )
        })
        .and_then(|value| as_number(&value))?;
    if !number.is_finite() {
        return None;
    }
    Some((number.round() as i64).clamp(0, u8::MAX as i64) as u8)
}

pub(super) fn runtime_value_to_i32(value: &RuntimeValue) -> Option<i32> {
    let number = as_number(value)?;
    if number.is_finite() && number >= i32::MIN as f64 && number <= i32::MAX as f64 {
        Some(number.round() as i32)
    } else {
        None
    }
}

fn runtime_value_to_sound_id(
    value: RuntimeValue,
    sound_index: &HashMap<String, i32>,
) -> Option<i32> {
    match value {
        RuntimeValue::Number(number) => finite_sound_number_to_id(number),
        RuntimeValue::Text(name) => sound_index.get(&name.to_ascii_lowercase()).copied(),
        RuntimeValue::Bool(_) => None,
    }
}

fn finite_sound_number_to_id(number: f64) -> Option<i32> {
    if number.is_finite() && number >= 0.0 && number <= f64::from(i32::MAX) {
        Some(number.round() as i32)
    } else {
        None
    }
}
