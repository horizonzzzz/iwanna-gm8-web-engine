use iwm_runtime_host::RuntimeHost;

use super::statement::{RuntimeExecutionTrace, RuntimeStatementEnvironment};
use crate::helpers::record_host_diagnostic;
use crate::{LoweredLogicExpr, LoweredLogicStatement, RuntimeInstance};

pub(super) fn record_unsupported_function<H: RuntimeHost>(
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

pub(super) fn record_unsupported_expr_functions<H: RuntimeHost>(
    env: &mut RuntimeStatementEnvironment<'_, H>,
    expr: &LoweredLogicExpr,
    instance: &RuntimeInstance,
) {
    match expr {
        LoweredLogicExpr::Call { name, args } => {
            if !is_supported_eval_function(name) && !env.script_entries.contains_key(name) {
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

pub(super) fn record_unsupported_statement<H: RuntimeHost>(
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

pub(super) fn trace_message(trace: &RuntimeExecutionTrace, instance: &RuntimeInstance) -> String {
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
