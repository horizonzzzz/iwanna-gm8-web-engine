use std::collections::{HashMap, HashSet};

use iwm_runtime_host::RuntimeHost;

use super::context::{RuntimeEvalContext, RuntimeExecutionScope};
use super::eval_functions::{
    evaluate_choose_call, evaluate_collision_line, evaluate_distance_to_object,
    evaluate_file_exists, evaluate_instance_exists, evaluate_instance_number,
    evaluate_instance_place, evaluate_keyboard_query, evaluate_ord_call, evaluate_place_query,
    evaluate_random_call, evaluate_random_range_call,
};
pub(super) use super::eval_values::is_truthy;
use super::eval_values::{eval_binary_expr, runtime_value_to_string_text};
pub(crate) use super::eval_variables::assignable_key;
use super::eval_variables::{evaluate_identifier, evaluate_index_access, evaluate_member_access};
use crate::helpers::{as_number, parse_runtime_value};
use crate::{LoweredLogicExpr, RuntimeInstance, RuntimeValue};

pub(super) fn evaluate_expr(
    expr: &LoweredLogicExpr,
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    match expr {
        LoweredLogicExpr::Identifier(name) => {
            evaluate_identifier(name, instance, globals, scope, eval_context)
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
            "random" => evaluate_random_call(args, instance, globals, scope, eval_context),
            "random_range" => {
                evaluate_random_range_call(args, instance, globals, scope, eval_context)
            }
            "choose" => evaluate_choose_call(args, instance, globals, scope, eval_context),
            "string" => args
                .first()
                .and_then(|arg| evaluate_expr(arg, instance, globals, scope, eval_context))
                .map(runtime_value_to_string_text)
                .map(RuntimeValue::Text),
            "file_exists" => evaluate_file_exists(args, instance, globals, scope, eval_context),
            "instance_exists" => evaluate_instance_exists(args, eval_context),
            "instance_number" => evaluate_instance_number(args, eval_context),
            "instance_place" => {
                evaluate_instance_place(args, instance, globals, scope, eval_context)
            }
            "distance_to_object" => {
                evaluate_distance_to_object(args, instance, globals, scope, eval_context)
            }
            "collision_line" => {
                evaluate_collision_line(args, instance, globals, scope, eval_context)
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
            evaluate_member_access(target, member, instance, globals, scope, eval_context)
        }
        LoweredLogicExpr::IndexAccess { target, index } => {
            evaluate_index_access(target, index, instance, globals, scope)
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

pub(crate) fn sample_known_files<H: RuntimeHost>(host: &H) -> HashSet<String> {
    let mut files = HashSet::new();
    for candidate in ["temp", "DeathTime", "save1", "save2", "save3"] {
        if host.read(std::path::Path::new(candidate)).is_ok() {
            files.insert(candidate.to_string());
        }
    }
    files
}
