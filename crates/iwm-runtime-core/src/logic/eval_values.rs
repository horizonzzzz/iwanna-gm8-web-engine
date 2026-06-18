use crate::helpers::as_number;
use crate::RuntimeValue;

pub(super) fn is_truthy(value: Option<RuntimeValue>) -> bool {
    match value {
        Some(RuntimeValue::Bool(b)) => b,
        Some(RuntimeValue::Number(n)) => n >= 0.5,
        Some(RuntimeValue::Text(s)) => !s.is_empty(),
        None => false,
    }
}

pub(super) fn runtime_value_to_string_text(value: RuntimeValue) -> String {
    match value {
        RuntimeValue::Number(number) if number.fract() == 0.0 => format!("{}", number as i64),
        RuntimeValue::Number(number) => number.to_string(),
        RuntimeValue::Bool(flag) => flag.to_string(),
        RuntimeValue::Text(text) => text,
    }
}

pub(super) fn eval_binary_expr(
    op: &str,
    left: &RuntimeValue,
    right: &RuntimeValue,
) -> Option<RuntimeValue> {
    match op {
        "+" => match (left, right) {
            (RuntimeValue::Text(_), _) | (_, RuntimeValue::Text(_)) => {
                Some(RuntimeValue::Text(format!(
                    "{}{}",
                    runtime_value_to_string_text(left.clone()),
                    runtime_value_to_string_text(right.clone())
                )))
            }
            _ => Some(RuntimeValue::Number(as_number(left)? + as_number(right)?)),
        },
        "-" => Some(RuntimeValue::Number(as_number(left)? - as_number(right)?)),
        "*" => Some(RuntimeValue::Number(as_number(left)? * as_number(right)?)),
        "/" => Some(RuntimeValue::Number(as_number(left)? / as_number(right)?)),
        "==" => Some(RuntimeValue::Bool(runtime_values_equal(left, right))),
        "=" => Some(RuntimeValue::Bool(runtime_values_equal(left, right))),
        "!=" => Some(RuntimeValue::Bool(!runtime_values_equal(left, right))),
        ">=" => Some(RuntimeValue::Bool(as_number(left)? >= as_number(right)?)),
        "<=" => Some(RuntimeValue::Bool(as_number(left)? <= as_number(right)?)),
        ">" => Some(RuntimeValue::Bool(as_number(left)? > as_number(right)?)),
        "<" => Some(RuntimeValue::Bool(as_number(left)? < as_number(right)?)),
        _ => None,
    }
}

fn runtime_values_equal(left: &RuntimeValue, right: &RuntimeValue) -> bool {
    match (as_number(left), as_number(right)) {
        (Some(left), Some(right)) => left == right,
        _ => left == right,
    }
}
