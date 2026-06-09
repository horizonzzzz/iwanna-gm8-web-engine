use std::collections::HashMap;

use iwm_runtime_host::RuntimeHost;

use super::context::{RuntimeEvalContext, RuntimeExecutionScope, RuntimeInstanceCreateRequest};
use super::eval::{assignable_key, evaluate_expr, is_truthy};
use crate::helpers::{as_number, parse_room_id, record_host_diagnostic};
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
    pub(crate) globals: &'a mut HashMap<String, RuntimeValue>,
    pub(crate) pending_room_transition: &'a mut Option<usize>,
    pub(crate) pending_room_reset: &'a mut bool,
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
            if let Some(key) = assignable_key(target, Some(instance), Some(scope)) {
                if let Some(value) = evaluate_with_diagnostics(
                    value,
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
                }
            }
            "instance_create" => {
                if let Some(create) = runtime_instance_create_request(
                    args,
                    instance,
                    env.globals,
                    scope,
                    eval_context,
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
                            context.room_instance_overlay,
                            env.room_instance_updates,
                            instance_index,
                            instance,
                        );
                        let with_context =
                            context.with_other_and_overlay(&other_snapshot, &overlay);
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
                        context.room_instance_overlay,
                        env.room_instance_updates,
                        target_index,
                        &target_instance,
                    );
                    let with_context = context.with_other_and_overlay(&other_snapshot, &overlay);
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

fn evaluate_with_diagnostics<H: RuntimeHost>(
    expr: &LoweredLogicExpr,
    instance: Option<&RuntimeInstance>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
    trace_instance: &RuntimeInstance,
) -> Option<RuntimeValue> {
    let value = evaluate_expr(expr, instance, env.globals, scope, eval_context);
    if value.is_none() {
        record_unsupported_expr_functions(env, expr, trace_instance);
    }
    value
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
            | "string"
            | "file_exists"
            | "instance_exists"
            | "distance_to_object"
            | "keyboard_check"
            | "keyboard_check_direct"
            | "keyboard_check_pressed"
            | "keyboard_check_released"
            | "place_meeting"
            | "place_free"
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

fn merged_statement_overlay(
    base_overlay: &[(usize, RuntimeInstance)],
    pending_updates: &[(usize, RuntimeInstance)],
    current_index: usize,
    current_instance: &RuntimeInstance,
) -> Vec<(usize, RuntimeInstance)> {
    base_overlay
        .iter()
        .chain(pending_updates.iter())
        .chain(std::iter::once(&(current_index, current_instance.clone())))
        .map(|(index, instance)| (*index, instance.clone()))
        .collect()
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
                .room_instances_iter()
                .filter(|(_, instance)| {
                    instance.alive && wanted_object_ids.contains(&instance.object_id)
                })
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
    Some(RuntimeInstanceCreateRequest { object_id, x, y })
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

    match key.as_str() {
        "x" => assign_number_field(value, &mut instance.x, &mut instance.vars, key),
        "y" => assign_number_field(value, &mut instance.y, &mut instance.vars, key),
        "previous_x" => {
            assign_number_field(value, &mut instance.previous_x, &mut instance.vars, key)
        }
        "previous_y" => {
            assign_number_field(value, &mut instance.previous_y, &mut instance.vars, key)
        }
        "hspeed" => assign_number_field(value, &mut instance.hspeed, &mut instance.vars, key),
        "vspeed" => assign_number_field(value, &mut instance.vspeed, &mut instance.vars, key),
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
