use std::collections::HashMap;

use iwm_runtime_host::{RuntimeHost, RuntimeSoundMode};

use super::context::{
    RuntimeBinaryFileState, RuntimeEvalContext, RuntimeExecutionScope,
    RuntimeInstanceCreateRequest, RuntimeRoomInstanceOverlay,
};
use super::eval::{assignable_key, evaluate_expr, is_truthy};
use crate::helpers::{as_number, collides_with_instance_at, parse_room_id, record_host_diagnostic};
use crate::{
    LoweredLogicEntry, LoweredLogicExpr, LoweredLogicStatement, RuntimeInstance, RuntimeValue,
};

#[derive(Debug, Clone)]
pub(crate) struct RuntimeExecutionTrace {
    pub(crate) room_id: usize,
    pub(crate) tick: u64,
    pub(crate) block_id: String,
    pub(crate) object_name: String,
    pub(crate) event_tag: String,
}

pub(crate) struct RuntimeStatementEnvironment<'a, H: RuntimeHost> {
    pub(crate) script_entries: &'a HashMap<String, LoweredLogicEntry>,
    pub(crate) sound_index: &'a HashMap<String, i32>,
    pub(crate) globals: &'a mut HashMap<String, RuntimeValue>,
    pub(crate) pending_room_transition: &'a mut Option<usize>,
    pub(crate) pending_room_reset: &'a mut bool,
    pub(crate) binary_files: &'a mut RuntimeBinaryFileState,
    pub(crate) host: &'a mut H,
    pub(crate) diagnostics: &'a mut Vec<iwm_runtime_host::RuntimeDiagnostic>,
    pub(crate) room_instance_updates: &'a mut Vec<(usize, RuntimeInstance)>,
    pub(crate) room_instance_creates: &'a mut Vec<RuntimeInstanceCreateRequest>,
    pub(crate) trace: RuntimeExecutionTrace,
}

pub(crate) fn apply_runtime_statement<H: RuntimeHost>(
    statement: &LoweredLogicStatement,
    instance: &mut RuntimeInstance,
    instance_index: usize,
    scope: &mut RuntimeExecutionScope,
    destroy_event_entries: &HashMap<usize, Vec<LoweredLogicEntry>>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
) {
    match statement {
        LoweredLogicStatement::Assignment { target, value } => {
            if let Some(value) = evaluate_with_diagnostics(
                value,
                Some(instance),
                Some(scope),
                eval_context,
                env,
                instance,
            ) {
                if assign_runtime_member_reference(
                    target,
                    value.clone(),
                    instance,
                    instance_index,
                    scope,
                    eval_context,
                    env,
                ) {
                    return;
                }
                if let Some(key) = assignable_key(target, Some(instance), Some(scope)) {
                    assign_runtime_value(key, value, instance, env.globals, scope);
                }
            }
        }
        LoweredLogicStatement::Conditional {
            condition,
            then_branch,
            else_branch,
        } => {
            let condition_value = evaluate_with_diagnostics(
                condition,
                Some(instance),
                Some(scope),
                eval_context,
                env,
                instance,
            );
            let branch = if is_truthy(condition_value) {
                then_branch
            } else {
                else_branch
            };
            for nested in branch {
                apply_runtime_statement(
                    nested,
                    instance,
                    instance_index,
                    scope,
                    destroy_event_entries,
                    eval_context,
                    env,
                );
            }
        }
        LoweredLogicStatement::VariableDeclaration { names } => {
            for name in names {
                scope.declare(name);
            }
        }
        LoweredLogicStatement::Return { .. } => {}
        LoweredLogicStatement::FunctionCall { name, args } => match name.as_str() {
            "room_goto" => {
                if let Some(room_id) = args
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
                    .and_then(|value| runtime_value_to_room_id(&value))
                {
                    *env.pending_room_transition = Some(room_id);
                } else {
                    record_host_diagnostic(
                        env.host,
                        env.diagnostics,
                        iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
                        "runtime-step-room-goto-unresolved",
                        format!(
                            "could not resolve room_goto target for {}",
                            instance.object_name
                        ),
                    );
                }
            }
            "room_goto_next" => {
                if let Some(room_id) = next_room_id(instance, env.globals, eval_context) {
                    *env.pending_room_transition = Some(room_id);
                } else {
                    record_host_diagnostic(
                        env.host,
                        env.diagnostics,
                        iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
                        "runtime-step-room-goto-next-unresolved",
                        format!(
                            "could not resolve room_goto_next target for {}",
                            instance.object_name
                        ),
                    );
                }
            }
            "game_restart" => {
                *env.pending_room_reset = true;
            }
            "sound_play" => {
                dispatch_runtime_sound_call(
                    env,
                    name,
                    args,
                    Some(RuntimeSoundMode::Once),
                    instance,
                    scope,
                    eval_context,
                );
            }
            "sound_loop" => {
                dispatch_runtime_sound_call(
                    env,
                    name,
                    args,
                    Some(RuntimeSoundMode::Loop),
                    instance,
                    scope,
                    eval_context,
                );
            }
            "sound_stop" => {
                dispatch_runtime_sound_call(env, name, args, None, instance, scope, eval_context);
            }
            "sound_stop_all" => {
                if let Err(error) = env.host.stop_all_sounds() {
                    record_host_diagnostic(
                        env.host,
                        env.diagnostics,
                        iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
                        "runtime-audio-host-error",
                        format!(
                            "{} function=sound_stop_all error={}",
                            trace_message(&env.trace, instance),
                            error
                        ),
                    );
                }
            }
            "keyboard_set_numlock" => {
                if let Some(value) = args.first().and_then(|arg| {
                    evaluate_with_diagnostics(
                        arg,
                        Some(instance),
                        Some(scope),
                        eval_context,
                        env,
                        instance,
                    )
                }) {
                    env.host.set_keyboard_numlock(is_truthy(Some(value)));
                }
            }
            "move_contact_solid" => {
                dispatch_move_contact_solid(env, args, instance, scope, eval_context);
            }
            "file_bin_write_byte" => {
                let Some(handle) =
                    evaluate_file_bin_handle(args.first(), instance, scope, eval_context, env)
                else {
                    return;
                };
                let Some(byte) =
                    evaluate_file_bin_byte(args.get(1), instance, scope, eval_context, env)
                else {
                    return;
                };
                env.binary_files.write_byte(handle, byte);
            }
            "file_bin_close" => {
                let Some(handle) =
                    evaluate_file_bin_handle(args.first(), instance, scope, eval_context, env)
                else {
                    return;
                };
                if let Err(error) = env.binary_files.close(env.host, handle) {
                    record_host_diagnostic(
                        env.host,
                        env.diagnostics,
                        iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
                        "runtime-file-host-error",
                        format!(
                            "{} function=file_bin_close handle={} error={}",
                            trace_message(&env.trace, instance),
                            handle,
                            error
                        ),
                    );
                }
            }
            "instance_destroy" => {
                if instance.alive {
                    let entries = destroy_event_entries
                        .get(&instance.object_id)
                        .cloned()
                        .unwrap_or_default();
                    for entry in &entries {
                        let mut destroy_scope = RuntimeExecutionScope::default();
                        let nested_destroy_entries = HashMap::new();
                        for nested in &entry.statements {
                            apply_runtime_statement(
                                nested,
                                instance,
                                instance_index,
                                &mut destroy_scope,
                                &nested_destroy_entries,
                                eval_context,
                                env,
                            );
                            if *env.pending_room_reset || env.pending_room_transition.is_some() {
                                break;
                            }
                        }
                        if *env.pending_room_reset || env.pending_room_transition.is_some() {
                            break;
                        }
                    }
                    instance.alive = false;
                    record_host_diagnostic(
                        env.host,
                        env.diagnostics,
                        iwm_runtime_host::RuntimeDiagnosticLevel::Info,
                        "runtime-instance-destroyed",
                        format!(
                            "{} object={} runtime_id={}",
                            trace_message(&env.trace, instance),
                            instance.object_name,
                            instance.runtime_id
                        ),
                    );
                }
            }
            "instance_create" => {
                if let Some(create) = runtime_instance_create_request(
                    args,
                    instance,
                    env.globals,
                    scope,
                    eval_context,
                    env.room_instance_creates.len(),
                ) {
                    env.room_instance_creates.push(create);
                }
            }
            _ => {
                if let Some(entry) = env.script_entries.get(name) {
                    let mut script_scope = RuntimeExecutionScope::default();
                    let previous_trace = env.trace.clone();
                    env.trace.block_id.clone_from(&entry.block_id);
                    env.trace.event_tag = "script".into();
                    for nested in &entry.statements {
                        apply_runtime_statement(
                            nested,
                            instance,
                            instance_index,
                            &mut script_scope,
                            destroy_event_entries,
                            eval_context,
                            env,
                        );
                        if *env.pending_room_reset || env.pending_room_transition.is_some() {
                            break;
                        }
                    }
                    env.trace = previous_trace;
                } else {
                    record_unsupported_function(env, name, instance);
                }
            }
        },
        LoweredLogicStatement::With { target, body } => {
            let Some(context) = eval_context else {
                return;
            };
            let target_indices = with_target_indices(target, instance_index, context);
            let other_snapshot = instance.clone();
            for target_index in target_indices {
                if target_index == instance_index {
                    for nested in body {
                        let overlay = merged_statement_overlay(
                            &context.room_instance_overlay,
                            env.room_instance_updates,
                            instance_index,
                            instance,
                        );
                        let with_context = context.with_other_and_overlay(&other_snapshot, overlay);
                        apply_runtime_statement(
                            nested,
                            instance,
                            instance_index,
                            scope,
                            destroy_event_entries,
                            Some(&with_context),
                            env,
                        );
                        sync_instance_from_updates(
                            instance_index,
                            instance,
                            env.room_instance_updates,
                        );
                        if *env.pending_room_reset || env.pending_room_transition.is_some() {
                            break;
                        }
                    }
                    if *env.pending_room_reset || env.pending_room_transition.is_some() {
                        break;
                    }
                    continue;
                }

                let Some(mut target_instance) = context.room_instance(target_index).cloned() else {
                    continue;
                };
                if !target_instance.alive {
                    continue;
                }
                for nested in body {
                    let overlay = merged_statement_overlay(
                        &context.room_instance_overlay,
                        env.room_instance_updates,
                        target_index,
                        &target_instance,
                    );
                    let with_context = context.with_other_and_overlay(&other_snapshot, overlay);
                    apply_runtime_statement(
                        nested,
                        &mut target_instance,
                        target_index,
                        scope,
                        destroy_event_entries,
                        Some(&with_context),
                        env,
                    );
                    sync_instance_from_updates(
                        target_index,
                        &mut target_instance,
                        env.room_instance_updates,
                    );
                    if *env.pending_room_reset || env.pending_room_transition.is_some() {
                        break;
                    }
                }
                env.room_instance_updates
                    .push((target_index, target_instance));
                if *env.pending_room_reset || env.pending_room_transition.is_some() {
                    break;
                }
            }
        }
        LoweredLogicStatement::For {
            init,
            condition,
            step,
            body,
        } => {
            execute_assignment_expression(init, instance, scope, eval_context, env);
            let mut iteration_count = 0usize;
            while is_truthy(evaluate_with_diagnostics(
                condition,
                Some(instance),
                Some(scope),
                eval_context,
                env,
                instance,
            )) {
                for nested in body {
                    apply_runtime_statement(
                        nested,
                        instance,
                        instance_index,
                        scope,
                        destroy_event_entries,
                        eval_context,
                        env,
                    );
                    if *env.pending_room_reset || env.pending_room_transition.is_some() {
                        break;
                    }
                }
                if *env.pending_room_reset || env.pending_room_transition.is_some() {
                    break;
                }
                execute_assignment_expression(step, instance, scope, eval_context, env);
                iteration_count += 1;
                if iteration_count >= 10_000 {
                    record_host_diagnostic(
                        env.host,
                        env.diagnostics,
                        iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
                        "runtime-for-iteration-limit",
                        format!(
                            "{} iteration_limit=10000",
                            trace_message(&env.trace, instance)
                        ),
                    );
                    break;
                }
            }
        }
        LoweredLogicStatement::Repeat { count, body } => {
            let repeat_count = evaluate_with_diagnostics(
                count,
                Some(instance),
                Some(scope),
                eval_context,
                env,
                instance,
            )
            .and_then(|value| as_number(&value))
            .filter(|value| value.is_finite() && *value > 0.0)
            .map(|value| value.floor() as usize)
            .unwrap_or(0);
            let capped_count = repeat_count.min(10_000);
            for _ in 0..capped_count {
                for nested in body {
                    apply_runtime_statement(
                        nested,
                        instance,
                        instance_index,
                        scope,
                        destroy_event_entries,
                        eval_context,
                        env,
                    );
                    if *env.pending_room_reset || env.pending_room_transition.is_some() {
                        break;
                    }
                }
                if *env.pending_room_reset || env.pending_room_transition.is_some() {
                    break;
                }
            }
            if repeat_count > capped_count {
                record_host_diagnostic(
                    env.host,
                    env.diagnostics,
                    iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
                    "runtime-repeat-iteration-limit",
                    format!(
                        "{} iteration_limit=10000",
                        trace_message(&env.trace, instance)
                    ),
                );
            }
        }
        _ => {
            record_unsupported_statement(env, statement, instance);
        }
    }
}

fn record_unsupported_function<H: RuntimeHost>(
    env: &mut RuntimeStatementEnvironment<'_, H>,
    name: &str,
    instance: &RuntimeInstance,
) {
    record_host_diagnostic(
        env.host,
        env.diagnostics,
        iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
        "runtime-unsupported-function",
        format!("{} function={}", trace_message(&env.trace, instance), name),
    );
}

fn dispatch_runtime_sound_call<H: RuntimeHost>(
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

pub(crate) fn resolve_runtime_sound_id(
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

fn dispatch_move_contact_solid<H: RuntimeHost>(
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

fn assign_runtime_member_reference<H: RuntimeHost>(
    target: &LoweredLogicExpr,
    value: RuntimeValue,
    instance: &mut RuntimeInstance,
    instance_index: usize,
    scope: &RuntimeExecutionScope,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
) -> bool {
    let LoweredLogicExpr::MemberAccess { target, member } = target else {
        return false;
    };

    if matches!(target.as_ref(), LoweredLogicExpr::Identifier(name) if name == "global") {
        return false;
    }

    if matches!(target.as_ref(), LoweredLogicExpr::Identifier(name) if name == "self") {
        assign_instance_field_or_var(member.clone(), value, instance);
        return true;
    }

    if matches!(target.as_ref(), LoweredLogicExpr::Identifier(name) if name == "other") {
        let Some(context) = eval_context else {
            return false;
        };
        let Some(other) = context.other_instance() else {
            return false;
        };
        return assign_runtime_member_by_ref(
            other.instance_id as f64,
            member,
            value,
            instance,
            instance_index,
            eval_context,
            env,
        );
    }

    if let Some((target_index, _)) = object_member_assignment_target(target, scope, eval_context) {
        return assign_runtime_member_by_index(
            target_index,
            member,
            value,
            instance,
            instance_index,
            eval_context,
            env,
        );
    }

    let Some(RuntimeValue::Number(instance_ref)) = evaluate_with_diagnostics(
        target,
        Some(instance),
        Some(scope),
        eval_context,
        env,
        instance,
    ) else {
        return false;
    };

    assign_runtime_member_by_ref(
        instance_ref,
        member,
        value,
        instance,
        instance_index,
        eval_context,
        env,
    )
}

fn object_member_assignment_target<'a>(
    target: &LoweredLogicExpr,
    scope: &RuntimeExecutionScope,
    eval_context: Option<&'a RuntimeEvalContext<'_>>,
) -> Option<(usize, &'a RuntimeInstance)> {
    let LoweredLogicExpr::Identifier(name) = target else {
        return None;
    };
    if scope.is_local_key(name) {
        return None;
    }
    let context = eval_context?;
    let target_object_ids = context
        .place_target_ids_by_name
        .get(&name.to_ascii_lowercase())?;
    context
        .room_instances_matching_object_ids(target_object_ids)
        .find(|(_, candidate)| candidate.alive)
}

fn assign_runtime_member_by_ref<H: RuntimeHost>(
    instance_ref: f64,
    member: &str,
    value: RuntimeValue,
    instance: &mut RuntimeInstance,
    instance_index: usize,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
) -> bool {
    if assign_pending_create_member(
        env.room_instance_creates,
        instance_ref,
        member,
        value.clone(),
    ) {
        return true;
    }

    let Some(context) = eval_context else {
        return false;
    };
    let Some((target_index, _)) = context
        .room_instances_iter()
        .find(|(_, candidate)| runtime_instance_ref_matches(instance_ref, candidate))
    else {
        return false;
    };

    assign_runtime_member_by_index(
        target_index,
        member,
        value,
        instance,
        instance_index,
        Some(context),
        env,
    )
}

fn assign_runtime_member_by_index<H: RuntimeHost>(
    target_index: usize,
    member: &str,
    value: RuntimeValue,
    instance: &mut RuntimeInstance,
    instance_index: usize,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
) -> bool {
    if target_index == instance_index {
        assign_instance_field_or_var(member.to_string(), value, instance);
        return true;
    }

    let Some(mut target_instance) = env
        .room_instance_updates
        .iter()
        .rev()
        .find(|(index, _)| *index == target_index)
        .map(|(_, instance)| instance.clone())
        .or_else(|| eval_context.and_then(|context| context.room_instance(target_index).cloned()))
    else {
        return false;
    };
    assign_instance_field_or_var(member.to_string(), value, &mut target_instance);
    env.room_instance_updates
        .push((target_index, target_instance));
    true
}

fn assign_pending_create_member(
    creates: &mut [RuntimeInstanceCreateRequest],
    instance_ref: f64,
    member: &str,
    value: RuntimeValue,
) -> bool {
    let Some(create) = creates
        .iter_mut()
        .find(|create| create_request_ref_matches(instance_ref, create))
    else {
        return false;
    };
    create.post_create_vars.insert(member.to_string(), value);
    true
}

fn pending_create_member_value(
    creates: &[RuntimeInstanceCreateRequest],
    instance_ref: f64,
    member: &str,
) -> Option<RuntimeValue> {
    creates
        .iter()
        .find(|create| create_request_ref_matches(instance_ref, create))
        .and_then(|create| create.post_create_vars.get(member).cloned())
}

fn create_request_ref_matches(instance_ref: f64, create: &RuntimeInstanceCreateRequest) -> bool {
    if !instance_ref.is_finite() {
        return false;
    }
    let rounded = instance_ref.round();
    create.instance_id as f64 == rounded || create.runtime_id as f64 == rounded
}

fn runtime_instance_ref_matches(instance_ref: f64, instance: &RuntimeInstance) -> bool {
    if !instance_ref.is_finite() {
        return false;
    }
    let rounded = instance_ref.round();
    instance.instance_id as f64 == rounded || instance.runtime_id as f64 == rounded
}

fn execute_assignment_expression<H: RuntimeHost>(
    expr: &LoweredLogicExpr,
    instance: &mut RuntimeInstance,
    scope: &mut RuntimeExecutionScope,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
) {
    if let LoweredLogicExpr::BinaryExpr { op, left, right } = expr {
        if op == "=" {
            if let Some(key) = assignable_key(left, Some(instance), Some(scope)) {
                if let Some(value) = evaluate_with_diagnostics(
                    right,
                    Some(instance),
                    Some(scope),
                    eval_context,
                    env,
                    instance,
                ) {
                    assign_runtime_value(key, value, instance, env.globals, scope);
                }
            }
        }
    }
}

fn evaluate_with_diagnostics<H: RuntimeHost>(
    expr: &LoweredLogicExpr,
    instance: Option<&RuntimeInstance>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
    trace_instance: &RuntimeInstance,
) -> Option<RuntimeValue> {
    let value = evaluate_runtime_expr(expr, instance, scope, eval_context, env, trace_instance);
    if value.is_none() {
        record_unsupported_expr_functions(env, expr, trace_instance);
    }
    value
}

fn evaluate_runtime_expr<H: RuntimeHost>(
    expr: &LoweredLogicExpr,
    instance: Option<&RuntimeInstance>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
    trace_instance: &RuntimeInstance,
) -> Option<RuntimeValue> {
    if let LoweredLogicExpr::Call { name, args } = expr {
        if name == "instance_create" {
            let instance = instance?;
            let scope = scope?;
            return runtime_instance_create_request(
                args,
                instance,
                env.globals,
                scope,
                eval_context,
                env.room_instance_creates.len(),
            )
            .map(|create| {
                let instance_id = create.instance_id;
                env.room_instance_creates.push(create);
                RuntimeValue::Number(instance_id as f64)
            });
        }
        if name == "sound_isplaying" {
            let sound_id = args.first().and_then(|arg| {
                resolve_runtime_sound_id(
                    arg,
                    trace_instance,
                    scope,
                    eval_context,
                    env.globals,
                    env.sound_index,
                )
            })?;
            return env
                .host
                .is_sound_playing(sound_id)
                .ok()
                .map(RuntimeValue::Bool);
        }
        if name == "keyboard_get_numlock" {
            return Some(RuntimeValue::Bool(env.host.keyboard_numlock()));
        }
        if name == "file_bin_open" {
            let path = args.first().and_then(|arg| {
                evaluate_runtime_expr(arg, instance, scope, eval_context, env, trace_instance)
            })?;
            let RuntimeValue::Text(path) = path else {
                return None;
            };
            let mode = args
                .get(1)
                .and_then(|arg| {
                    evaluate_runtime_expr(arg, instance, scope, eval_context, env, trace_instance)
                })
                .and_then(|value| as_number(&value))
                .map(|value| value.round() as i32)
                .unwrap_or(0);
            let handle = env.binary_files.open(&*env.host, path, mode);
            return Some(RuntimeValue::Number(handle as f64));
        }
        if name == "file_bin_read_byte" {
            let handle = args
                .first()
                .and_then(|arg| {
                    evaluate_runtime_expr(arg, instance, scope, eval_context, env, trace_instance)
                })
                .and_then(|value| runtime_value_to_i32(&value))?;
            let byte = env.binary_files.read_byte(handle);
            return Some(RuntimeValue::Number(byte as f64));
        }
    }
    if let LoweredLogicExpr::MemberAccess { target, member } = expr {
        if let Some(RuntimeValue::Number(instance_ref)) =
            evaluate_runtime_expr(target, instance, scope, eval_context, env, trace_instance)
        {
            if let Some(value) =
                pending_create_member_value(env.room_instance_creates, instance_ref, member)
            {
                return Some(value);
            }
        }
    }

    evaluate_expr(expr, instance, env.globals, scope, eval_context)
}

fn evaluate_file_bin_handle<H: RuntimeHost>(
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

fn evaluate_file_bin_byte<H: RuntimeHost>(
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

fn runtime_value_to_i32(value: &RuntimeValue) -> Option<i32> {
    let number = as_number(value)?;
    if number.is_finite() && number >= i32::MIN as f64 && number <= i32::MAX as f64 {
        Some(number.round() as i32)
    } else {
        None
    }
}

fn record_unsupported_expr_functions<H: RuntimeHost>(
    env: &mut RuntimeStatementEnvironment<'_, H>,
    expr: &LoweredLogicExpr,
    instance: &RuntimeInstance,
) {
    match expr {
        LoweredLogicExpr::Call { name, args } => {
            if !is_supported_eval_function(name) {
                record_unsupported_function(env, name, instance);
            }
            for arg in args {
                record_unsupported_expr_functions(env, arg, instance);
            }
        }
        LoweredLogicExpr::UnaryExpr { child, .. } => {
            record_unsupported_expr_functions(env, child, instance);
        }
        LoweredLogicExpr::BinaryExpr { left, right, .. } => {
            record_unsupported_expr_functions(env, left, instance);
            record_unsupported_expr_functions(env, right, instance);
        }
        LoweredLogicExpr::MemberAccess { target, .. } => {
            record_unsupported_expr_functions(env, target, instance);
        }
        LoweredLogicExpr::IndexAccess { target, index } => {
            record_unsupported_expr_functions(env, target, instance);
            record_unsupported_expr_functions(env, index, instance);
        }
        LoweredLogicExpr::Identifier(_)
        | LoweredLogicExpr::LiteralNumber(_)
        | LoweredLogicExpr::LiteralBool(_)
        | LoweredLogicExpr::LiteralText(_)
        | LoweredLogicExpr::Raw { .. } => {}
    }
}

fn is_supported_eval_function(name: &str) -> bool {
    matches!(
        name,
        "room_goto"
            | "ord"
            | "abs"
            | "floor"
            | "random"
            | "random_range"
            | "choose"
            | "string"
            | "file_bin_open"
            | "file_bin_read_byte"
            | "file_bin_write_byte"
            | "file_bin_close"
            | "file_exists"
            | "instance_exists"
            | "distance_to_object"
            | "collision_line"
            | "keyboard_check"
            | "keyboard_check_direct"
            | "keyboard_check_pressed"
            | "keyboard_check_released"
            | "keyboard_get_numlock"
            | "place_meeting"
            | "place_free"
            | "sound_isplaying"
    )
}

fn record_unsupported_statement<H: RuntimeHost>(
    env: &mut RuntimeStatementEnvironment<'_, H>,
    statement: &LoweredLogicStatement,
    instance: &RuntimeInstance,
) {
    record_host_diagnostic(
        env.host,
        env.diagnostics,
        iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
        "runtime-unsupported-statement",
        format!(
            "{} statement_kind={}",
            trace_message(&env.trace, instance),
            statement_kind(statement)
        ),
    );
}

fn trace_message(trace: &RuntimeExecutionTrace, instance: &RuntimeInstance) -> String {
    format!(
        "room={} tick={} block_id={} object={} event_tag={} runtime_id={}",
        trace.room_id,
        trace.tick,
        trace.block_id,
        trace.object_name,
        trace.event_tag,
        instance.runtime_id
    )
}

fn statement_kind(statement: &LoweredLogicStatement) -> &'static str {
    match statement {
        LoweredLogicStatement::Assignment { .. } => "assignment",
        LoweredLogicStatement::VariableDeclaration { .. } => "variable-declaration",
        LoweredLogicStatement::Return { .. } => "return",
        LoweredLogicStatement::FunctionCall { .. } => "function-call",
        LoweredLogicStatement::Conditional { .. } => "conditional",
        LoweredLogicStatement::With { .. } => "with",
        LoweredLogicStatement::Repeat { .. } => "repeat",
        LoweredLogicStatement::While { .. } => "while",
        LoweredLogicStatement::For { .. } => "for",
        LoweredLogicStatement::Raw { .. } => "raw",
    }
}

fn merged_statement_overlay<'a>(
    base_overlay: &RuntimeRoomInstanceOverlay<'a>,
    pending_updates: &[(usize, RuntimeInstance)],
    current_index: usize,
    current_instance: &RuntimeInstance,
) -> RuntimeRoomInstanceOverlay<'a> {
    base_overlay.merge_pending_current(pending_updates, current_index, current_instance)
}

fn sync_instance_from_updates(
    current_index: usize,
    current_instance: &mut RuntimeInstance,
    pending_updates: &mut Vec<(usize, RuntimeInstance)>,
) {
    let Some(last_update_index) = pending_updates
        .iter()
        .rposition(|(index, _)| *index == current_index)
    else {
        return;
    };
    *current_instance = pending_updates[last_update_index].1.clone();
    pending_updates.retain(|(index, _)| *index != current_index);
}

fn with_target_indices(
    target: &LoweredLogicExpr,
    instance_index: usize,
    context: &RuntimeEvalContext<'_>,
) -> Vec<usize> {
    match target {
        LoweredLogicExpr::Identifier(name) if name.eq_ignore_ascii_case("self") => {
            vec![instance_index]
        }
        LoweredLogicExpr::Identifier(name) if name.eq_ignore_ascii_case("other") => context
            .other_instance()
            .and_then(|other| {
                context
                    .room_instances_iter()
                    .find(|(_, instance)| instance.runtime_id == other.runtime_id)
                    .map(|(index, _)| index)
            })
            .into_iter()
            .collect(),
        LoweredLogicExpr::Identifier(name) if name.eq_ignore_ascii_case("all") => context
            .room_instances_iter()
            .filter(|(_, instance)| instance.alive)
            .map(|(index, _)| index)
            .collect(),
        LoweredLogicExpr::Identifier(name) => {
            let wanted_object_ids = context
                .place_target_ids_by_name
                .get(&name.to_ascii_lowercase())
                .cloned()
                .unwrap_or_default();
            context
                .room_instances_matching_object_ids(&wanted_object_ids)
                .filter(|(_, instance)| instance.alive)
                .map(|(index, _)| index)
                .collect()
        }
        _ => Vec::new(),
    }
}
fn runtime_instance_create_request(
    args: &[LoweredLogicExpr],
    instance: &RuntimeInstance,
    globals: &HashMap<String, RuntimeValue>,
    scope: &RuntimeExecutionScope,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    pending_create_count: usize,
) -> Option<RuntimeInstanceCreateRequest> {
    let context = eval_context?;
    let x = args
        .first()
        .and_then(|arg| evaluate_expr(arg, Some(instance), globals, Some(scope), eval_context))
        .and_then(|value| as_number(&value))
        .unwrap_or(0.0);
    let y = args
        .get(1)
        .and_then(|arg| evaluate_expr(arg, Some(instance), globals, Some(scope), eval_context))
        .and_then(|value| as_number(&value))
        .unwrap_or(0.0);
    let object_name = args.get(2).and_then(|arg| match arg {
        LoweredLogicExpr::Identifier(name) => Some(name.as_str()),
        _ => None,
    })?;
    let object_id = context
        .place_target_ids_by_name
        .get(&object_name.to_ascii_lowercase())
        .and_then(|ids| ids.first().copied())?;
    let runtime_id = context
        .room_instances
        .len()
        .saturating_add(pending_create_count);
    let instance_id = -1 - runtime_id as i32;
    Some(RuntimeInstanceCreateRequest {
        object_id,
        runtime_id,
        instance_id,
        x,
        y,
        post_create_vars: HashMap::new(),
    })
}

fn assign_runtime_value(
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
            if let Some(n) = crate::helpers::as_number(&value) {
                instance.set_hspeed(n);
            } else {
                instance.vars.insert(key, value);
            }
        }
        "vspeed" => {
            if let Some(n) = crate::helpers::as_number(&value) {
                instance.set_vspeed(n);
            } else {
                instance.vars.insert(key, value);
            }
        }
        "speed" => {
            if let Some(n) = crate::helpers::as_number(&value) {
                instance.set_speed(n);
            } else {
                instance.vars.insert(key, value);
            }
        }
        "direction" => {
            if let Some(n) = crate::helpers::as_number(&value) {
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
fn runtime_value_to_room_id(value: &RuntimeValue) -> Option<usize> {
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

fn next_room_id(
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
