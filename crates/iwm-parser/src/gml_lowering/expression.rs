use iwm_runtime_model::LoweredLogicExpr;

use super::syntax::{
    extract_parenthesized_block, find_top_level_dot, split_top_level_csv, split_top_level_operator,
    split_top_level_trailing_index, strip_balanced_outer_parens,
};

pub(super) fn lower_expr(expr: &str) -> LoweredLogicExpr {
    let expr = expr.trim();
    if expr.is_empty() {
        return LoweredLogicExpr::Raw {
            source: String::new(),
        };
    }

    let expr = strip_balanced_outer_parens(expr);

    if let Some(binary) = lower_binary_expr(expr) {
        return binary;
    }

    if let Some(unary) = lower_unary_expr(expr) {
        return unary;
    }

    if let Some(call) = lower_call_expr(expr) {
        return call;
    }

    if let Some(index) = lower_index_expr(expr) {
        return index;
    }

    if let Ok(number) = expr.parse::<f64>() {
        return LoweredLogicExpr::LiteralNumber(number);
    }

    if let Some(member) = lower_member_expr(expr) {
        return member;
    }

    if expr.eq_ignore_ascii_case("true") {
        return LoweredLogicExpr::LiteralBool(true);
    }

    if expr.eq_ignore_ascii_case("false") {
        return LoweredLogicExpr::LiteralBool(false);
    }

    if expr.starts_with('"') && expr.ends_with('"') && expr.len() >= 2 {
        return LoweredLogicExpr::LiteralText(expr.trim_matches('"').to_string());
    }

    LoweredLogicExpr::Identifier(expr.to_string())
}

fn lower_binary_expr(expr: &str) -> Option<LoweredLogicExpr> {
    // Lower-precedence boolean operators must split first so the right-hand side
    // can still contain tighter expressions such as `b && c`.
    for op in ["||", "&&"] {
        if let Some((left, right)) = split_top_level_operator(expr, op) {
            return Some(LoweredLogicExpr::BinaryExpr {
                op: op.to_string(),
                left: Box::new(lower_expr(&left)),
                right: Box::new(lower_expr(&right)),
            });
        }
    }

    for ops in [
        &["==", "!=", ">=", "<=", "=", ">", "<"][..],
        &["+", "-"],
        &["*", "/", "div", "mod"],
    ] {
        for op in ops {
            if let Some((left, right)) = split_top_level_operator(expr, op) {
                return Some(LoweredLogicExpr::BinaryExpr {
                    op: op.to_string(),
                    left: Box::new(lower_expr(&left)),
                    right: Box::new(lower_expr(&right)),
                });
            }
        }
    }

    None
}

fn lower_call_expr(expr: &str) -> Option<LoweredLogicExpr> {
    let open_paren = expr.find('(')?;
    if !expr.ends_with(')') {
        return None;
    }
    let name = expr[..open_paren].trim();
    if name.is_empty() {
        return None;
    }
    let call_suffix = &expr[open_paren..];
    let (args_source, rest) = extract_parenthesized_block(call_suffix)?;
    if !rest.trim().is_empty() {
        return None;
    }
    let args = split_top_level_csv(&args_source)
        .into_iter()
        .map(|arg| lower_expr(&arg))
        .collect();
    Some(LoweredLogicExpr::Call {
        name: name.to_string(),
        args,
    })
}

fn lower_index_expr(expr: &str) -> Option<LoweredLogicExpr> {
    let (target, index) = split_top_level_trailing_index(expr)?;
    Some(LoweredLogicExpr::IndexAccess {
        target: Box::new(lower_expr(&target)),
        index: Box::new(lower_expr(&index)),
    })
}

fn lower_member_expr(expr: &str) -> Option<LoweredLogicExpr> {
    let dot_index = find_top_level_dot(expr)?;
    let left = expr[..dot_index].trim();
    let right = expr[dot_index + 1..].trim();
    if left.is_empty() || right.is_empty() {
        return None;
    }
    Some(LoweredLogicExpr::MemberAccess {
        target: Box::new(lower_expr(left)),
        member: right.to_string(),
    })
}

fn lower_unary_expr(expr: &str) -> Option<LoweredLogicExpr> {
    let expr = expr.trim();
    let (op, rest) = if let Some(rest) = expr.strip_prefix('!') {
        ("!", rest)
    } else if let Some(rest) = expr.strip_prefix('-') {
        if rest.starts_with('-') {
            return None;
        }
        ("-", rest)
    } else if let Some(rest) = expr.strip_prefix('+') {
        if rest.starts_with('+') {
            return None;
        }
        ("+", rest)
    } else {
        return None;
    };

    let child = rest.trim();
    if child.is_empty() {
        return None;
    }

    Some(LoweredLogicExpr::UnaryExpr {
        op: op.to_string(),
        child: Box::new(lower_expr(child)),
    })
}
