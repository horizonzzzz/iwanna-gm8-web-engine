use std::collections::HashMap;

use iwm_runtime_host::{RuntimeHost, RuntimeSoundMode};

use super::context::{RuntimeEvalContext, RuntimeExecutionScope};
use super::diagnostics::trace_message;
use super::eval::{evaluate_expr, is_truthy};
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
) -> Option<RuntimeValue> {
    let Some(sound_id) = evaluate_runtime_sound_id(env, args, instance, scope, eval_context) else {
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
        return None;
    };

    let result = if let Some(mode) = mode {
        env.host.play_sound(sound_id, mode)
    } else {
        env.host.stop_sound(sound_id)
    };
    if result.is_ok() {
        match mode {
            Some(RuntimeSoundMode::Once) => {
                env.active_one_shot_sounds.insert(sound_id);
            }
            Some(RuntimeSoundMode::Loop) | None => {
                env.active_one_shot_sounds.remove(&sound_id);
            }
        }
    }

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
        return None;
    }

    Some(RuntimeValue::Number(sound_id as f64))
}

pub(super) fn dispatch_legacy_audio_call<H: RuntimeHost>(
    env: &mut RuntimeStatementEnvironment<'_, H>,
    function_name: &str,
    args: &[LoweredLogicExpr],
    instance: &RuntimeInstance,
    scope: &RuntimeExecutionScope,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    let name = function_name.to_ascii_lowercase();
    match name.as_str() {
        "ss_loadsound" | "fmodsoundadd" | "fmodsoundaddasyncstream" => Some(
            evaluate_runtime_sound_id(env, args, instance, scope, eval_context)
                .map(|id| RuntimeValue::Number(id as f64))
                .unwrap_or(RuntimeValue::Number(0.0)),
        ),
        "ss_playsound" | "fmodsoundplay" | "fmodsoundplay3d" => Some(
            dispatch_runtime_sound_call(
                env,
                function_name,
                args,
                Some(RuntimeSoundMode::Once),
                instance,
                scope,
                eval_context,
            )
            .unwrap_or(RuntimeValue::Number(0.0)),
        ),
        "ss_loopsound" | "fmodsoundloop" | "fmodsoundloop3d" => Some(
            dispatch_runtime_sound_call(
                env,
                function_name,
                args,
                Some(RuntimeSoundMode::Loop),
                instance,
                scope,
                eval_context,
            )
            .unwrap_or(RuntimeValue::Number(0.0)),
        ),
        "ss_stopsound" | "ss_freesound" | "fmodinstancestop" | "fmodsoundfree" => Some(
            dispatch_runtime_sound_call(
                env,
                function_name,
                args,
                None,
                instance,
                scope,
                eval_context,
            )
            .unwrap_or(RuntimeValue::Number(0.0)),
        ),
        "ss_issoundplaying" | "ss_ishandlevalid" | "fmodinstanceisplaying" => {
            let playing = evaluate_runtime_sound_id(env, args, instance, scope, eval_context)
                .and_then(|id| env.host.is_sound_playing(id).ok())
                .unwrap_or(false);
            Some(RuntimeValue::Bool(playing))
        }
        "ss_issoundlooping" | "ss_issoundpaused" => Some(RuntimeValue::Bool(false)),
        "ss_unload" | "fmodfree" | "fmodallstop" => {
            env.active_one_shot_sounds.clear();
            let _ = env.host.stop_all_sounds();
            Some(RuntimeValue::Number(1.0))
        }
        "cleanmem"
        | "cleanmem_init"
        | "cleanmem_get_mem"
        | "ss_init"
        | "ss_setsoundfreq"
        | "ss_setsoundpan"
        | "ss_setsoundvol"
        | "ss_setsoundposition"
        | "ss_pausesound"
        | "ss_resumesound"
        | "ss_getsoundbytespersecond"
        | "ss_getsoundfreq"
        | "ss_getsoundlength"
        | "ss_getsoundpan"
        | "ss_getsoundposition"
        | "ss_getsoundvol"
        | "loadfmod"
        | "fmodinit"
        | "fmodupdate"
        | "fmodupdate3dpositions"
        | "fmodgroupsetvolume"
        | "fmodinstanceset3dposition"
        | "fmodinstancesetpaused"
        | "fmodinstancesetvolume"
        | "fmodlistenerset3dposition"
        | "fmodsetpassword"
        | "fmodsetworldscale"
        | "fmodsoundgetlength"
        | "fmodsoundgetmaxdist"
        | "fmodsoundset3dminmaxdistance"
        | "fmodsoundsetgroup"
        | "fmodsoundsetlooppoints"
        | "fmodsoundsetmaxvolume"
        | "fmodgetlasterror" => Some(RuntimeValue::Number(1.0)),
        "fmoderrorstr" => Some(RuntimeValue::Text(String::new())),
        _ => None,
    }
}

fn evaluate_runtime_sound_id<H: RuntimeHost>(
    env: &mut RuntimeStatementEnvironment<'_, H>,
    args: &[LoweredLogicExpr],
    instance: &RuntimeInstance,
    scope: &RuntimeExecutionScope,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<i32> {
    let arg = args.first()?;
    evaluate_with_diagnostics(
        arg,
        Some(instance),
        Some(scope),
        eval_context,
        env,
        instance,
    )
    .and_then(|value| runtime_value_to_sound_id(value, env.sound_index))
    .or_else(|| {
        resolve_runtime_sound_id(
            arg,
            instance,
            Some(scope),
            eval_context,
            env.globals,
            env.sound_index,
            env.zero_uninitialized_vars,
        )
    })
}

pub(crate) fn normalize_runtime_sound_key(value: &str) -> String {
    let mut key = value.trim().replace('\\', "/");
    while let Some(stripped) = key.strip_prefix("./") {
        key = stripped.to_string();
    }
    key.trim_start_matches('/').to_ascii_lowercase()
}

pub(super) fn resolve_runtime_sound_id(
    expr: &LoweredLogicExpr,
    instance: &RuntimeInstance,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    globals: &HashMap<String, RuntimeValue>,
    sound_index: &HashMap<String, i32>,
    zero_uninitialized_vars: bool,
) -> Option<i32> {
    match expr {
        LoweredLogicExpr::Identifier(name) | LoweredLogicExpr::LiteralText(name) => {
            evaluate_expr(expr, Some(instance), globals, scope, eval_context)
                .and_then(|value| runtime_value_to_sound_id(value, sound_index))
                .or_else(|| sound_index.get(&normalize_runtime_sound_key(name)).copied())
                // GM treats uninitialized identifiers as 0 when zero_uninitialized_vars is set.
                .or_else(|| zero_uninitialized_vars.then_some(0))
        }
        LoweredLogicExpr::LiteralNumber(number) => finite_sound_number_to_id(*number),
        _ => evaluate_expr(expr, Some(instance), globals, scope, eval_context)
            .and_then(|value| runtime_value_to_sound_id(value, sound_index))
            .or((zero_uninitialized_vars
                && matches!(
                    expr,
                    LoweredLogicExpr::MemberAccess { .. } | LoweredLogicExpr::IndexAccess { .. }
                ))
            .then_some(0)),
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

pub(super) fn dispatch_move_towards_point<H: RuntimeHost>(
    env: &mut RuntimeStatementEnvironment<'_, H>,
    args: &[LoweredLogicExpr],
    instance: &mut RuntimeInstance,
    scope: &RuntimeExecutionScope,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) {
    let Some(target_x) = evaluate_number_arg(args.first(), instance, scope, eval_context, env)
    else {
        return;
    };
    let Some(target_y) = evaluate_number_arg(args.get(1), instance, scope, eval_context, env)
    else {
        return;
    };
    let Some(speed) = evaluate_number_arg(args.get(2), instance, scope, eval_context, env) else {
        return;
    };
    instance.set_direction(
        (instance.y - target_y)
            .atan2(target_x - instance.x)
            .to_degrees(),
    );
    instance.set_speed(speed);
}

pub(super) fn dispatch_motion_add<H: RuntimeHost>(
    env: &mut RuntimeStatementEnvironment<'_, H>,
    args: &[LoweredLogicExpr],
    instance: &mut RuntimeInstance,
    scope: &RuntimeExecutionScope,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) {
    let Some(direction) = evaluate_number_arg(args.first(), instance, scope, eval_context, env)
    else {
        return;
    };
    let Some(speed) = evaluate_number_arg(args.get(1), instance, scope, eval_context, env) else {
        return;
    };
    let radians = direction.to_radians();
    instance.set_hvspeed(
        instance.hspeed + radians.cos() * speed,
        instance.vspeed - radians.sin() * speed,
    );
}

pub(super) fn dispatch_move_wrap<H: RuntimeHost>(
    env: &mut RuntimeStatementEnvironment<'_, H>,
    args: &[LoweredLogicExpr],
    instance: &mut RuntimeInstance,
    scope: &RuntimeExecutionScope,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) {
    let Some(context) = eval_context else {
        return;
    };
    let horizontal = evaluate_truthy_arg(args.first(), instance, scope, eval_context, env);
    let vertical = evaluate_truthy_arg(args.get(1), instance, scope, eval_context, env);
    let Some(margin) = evaluate_number_arg(args.get(2), instance, scope, eval_context, env) else {
        return;
    };
    if horizontal {
        if instance.x < -margin {
            instance.x = context.room_width as f64 + margin;
        } else if instance.x > context.room_width as f64 + margin {
            instance.x = -margin;
        }
    }
    if vertical {
        if instance.y < -margin {
            instance.y = context.room_height as f64 + margin;
        } else if instance.y > context.room_height as f64 + margin {
            instance.y = -margin;
        }
    }
}

pub(super) fn dispatch_move_bounce_solid<H: RuntimeHost>(
    env: &mut RuntimeStatementEnvironment<'_, H>,
    args: &[LoweredLogicExpr],
    instance: &mut RuntimeInstance,
    scope: &RuntimeExecutionScope,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) {
    let Some(context) = eval_context else {
        return;
    };
    let advanced = evaluate_number_arg(args.first(), instance, scope, eval_context, env)
        .map(|value| value.round() == 1.0)
        .unwrap_or(false);
    if advanced {
        bounce_advanced(context, instance);
    } else {
        bounce_simple(context, instance);
    }
}

fn bounce_simple(context: &RuntimeEvalContext<'_>, instance: &mut RuntimeInstance) {
    if collides_with_solid_at(context, instance, instance.x, instance.y) {
        instance.x = instance.previous_x;
        instance.y = instance.previous_y;
    }
    let (old_x, old_y) = (instance.x, instance.y);
    let x_bounce = collides_with_solid_at(context, instance, old_x + instance.hspeed, old_y);
    let y_bounce = collides_with_solid_at(context, instance, old_x, old_y + instance.vspeed);
    if x_bounce {
        instance.set_hspeed(-instance.hspeed);
    }
    if y_bounce {
        instance.set_vspeed(-instance.vspeed);
    }
    if !x_bounce
        && !y_bounce
        && collides_with_solid_at(
            context,
            instance,
            old_x + instance.hspeed,
            old_y + instance.vspeed,
        )
    {
        instance.set_hvspeed(-instance.hspeed, -instance.vspeed);
    }
}

fn bounce_advanced(context: &RuntimeEvalContext<'_>, instance: &mut RuntimeInstance) {
    let mut bounced = collides_with_solid_at(context, instance, instance.x, instance.y);
    if bounced {
        instance.x = instance.previous_x;
        instance.y = instance.previous_y;
    }
    let direction = instance
        .vars
        .get("direction")
        .and_then(as_number)
        .unwrap_or_else(|| (-instance.vspeed).atan2(instance.hspeed).to_degrees());
    let start_angle = (direction / 10.0).round() * 10.0;
    let (clockwise, clockwise_collided) = clear_side_angle(context, instance, start_angle, -10.0);
    let (counter_clockwise, counter_clockwise_collided) =
        clear_side_angle(context, instance, start_angle, 10.0);
    bounced |= clockwise_collided || counter_clockwise_collided;
    if bounced {
        instance.set_direction(clockwise + counter_clockwise + 180.0 - start_angle);
    }
}

fn clear_side_angle(
    context: &RuntimeEvalContext<'_>,
    instance: &RuntimeInstance,
    start_angle: f64,
    step: f64,
) -> (f64, bool) {
    let mut angle = start_angle;
    let mut collided = false;
    for _ in 0..36 {
        angle += step;
        let radians = angle.to_radians();
        let speed = instance
            .vars
            .get("speed")
            .and_then(as_number)
            .unwrap_or_else(|| instance.hspeed.hypot(instance.vspeed));
        if !collides_with_solid_at(
            context,
            instance,
            instance.x + speed * radians.cos(),
            instance.y - speed * radians.sin(),
        ) {
            break;
        }
        collided = true;
    }
    (angle, collided)
}

fn collides_with_solid_at(
    context: &RuntimeEvalContext<'_>,
    instance: &RuntimeInstance,
    x: f64,
    y: f64,
) -> bool {
    context
        .solid_room_instances_near(instance, x, y)
        .any(|(_, candidate)| {
            collides_with_instance_at(
                instance,
                x,
                y,
                candidate,
                Some(instance.runtime_id),
                |candidate| candidate.solid,
            )
        })
}

fn evaluate_number_arg<H: RuntimeHost>(
    expr: Option<&LoweredLogicExpr>,
    instance: &RuntimeInstance,
    scope: &RuntimeExecutionScope,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
) -> Option<f64> {
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
    .and_then(|value| as_number(&value))
    .filter(|value| value.is_finite())
}

fn evaluate_truthy_arg<H: RuntimeHost>(
    expr: Option<&LoweredLogicExpr>,
    instance: &RuntimeInstance,
    scope: &RuntimeExecutionScope,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
) -> bool {
    is_truthy(expr.and_then(|arg| {
        evaluate_with_diagnostics(
            arg,
            Some(instance),
            Some(scope),
            eval_context,
            env,
            instance,
        )
    }))
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
        RuntimeValue::Text(name) => sound_index
            .get(&normalize_runtime_sound_key(&name))
            .copied(),
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
