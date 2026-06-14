use iwm_runtime_model::{LoweredLogicExpr, LoweredLogicStatement};

use super::expression::lower_expr;
use super::source::lower_source;
use super::syntax::{
    extract_braced_block, extract_parenthesized_block, split_head_and_body,
    split_top_level_commas_or_semicolons, split_top_level_csv, split_top_level_operator,
};

pub(super) fn lower_statement(stmt: &str) -> Option<LoweredLogicStatement> {
    let stmt = stmt.trim();
    if stmt.is_empty() {
        return None;
    }

    if let Some(names) = lower_variable_declaration(stmt) {
        return Some(LoweredLogicStatement::VariableDeclaration { names });
    }

    if let Some(expr) = stmt.strip_prefix("return ") {
        let expr = expr.trim().trim_end_matches(';').trim();
        let value = if expr.is_empty() {
            None
        } else {
            Some(lower_expr(expr))
        };
        return Some(LoweredLogicStatement::Return { value });
    }

    if stmt.ends_with("++") && !stmt.ends_with("+++") {
        return lower_step_assignment(&stmt[..stmt.len() - 2], "+");
    }

    if stmt.ends_with("--") && !stmt.ends_with("---") {
        return lower_step_assignment(&stmt[..stmt.len() - 2], "-");
    }

    if let Some(target) = stmt.strip_prefix("++").map(str::trim) {
        return lower_step_assignment(target, "+");
    }

    if stmt.starts_with("--") && !stmt.starts_with("---") {
        return lower_step_assignment(&stmt[2..], "-");
    }

    if stmt.starts_with("if ") || stmt.starts_with("if(") {
        return lower_if_statement(stmt);
    }

    if stmt.starts_with("with ") || stmt.starts_with("with(") {
        return lower_block_statement(stmt, "with").map(|(head, body)| {
            LoweredLogicStatement::With {
                target: lower_expr(&head),
                body: lower_source(&body),
            }
        });
    }

    if stmt.starts_with("repeat ") || stmt.starts_with("repeat(") {
        return lower_block_statement(stmt, "repeat").map(|(head, body)| {
            LoweredLogicStatement::Repeat {
                count: lower_expr(&head),
                body: lower_source(&body),
            }
        });
    }

    if stmt.starts_with("while ") || stmt.starts_with("while(") {
        return lower_block_statement(stmt, "while").map(|(head, body)| {
            LoweredLogicStatement::While {
                condition: lower_expr(&head),
                body: lower_source(&body),
            }
        });
    }

    if stmt.starts_with("for ") || stmt.starts_with("for(") {
        return lower_for_statement(stmt);
    }

    for (compound_op, binary_op) in [("+=", "+"), ("-=", "-"), ("*=", "*"), ("/=", "/")] {
        if let Some((lhs, rhs)) = split_top_level_operator(stmt, compound_op) {
            return Some(LoweredLogicStatement::Assignment {
                target: lower_expr(&lhs),
                value: LoweredLogicExpr::BinaryExpr {
                    op: binary_op.to_string(),
                    left: Box::new(lower_expr(&lhs)),
                    right: Box::new(lower_expr(&rhs)),
                },
            });
        }
    }

    if let Some((lhs, rhs)) = stmt.split_once('=') {
        if !lhs.contains("==") && !lhs.contains(">=") && !lhs.contains("<=") && !lhs.contains("!=")
        {
            return Some(LoweredLogicStatement::Assignment {
                target: lower_expr(lhs.trim()),
                value: lower_expr(rhs.trim()),
            });
        }
    }

    if let Some(open_paren) = stmt.find('(') {
        let name = stmt[..open_paren].trim();
        let call_suffix = &stmt[open_paren..];
        let Some((args_source, _rest)) = extract_parenthesized_block(call_suffix) else {
            return Some(LoweredLogicStatement::Raw {
                source: stmt.to_string(),
            });
        };
        let args = split_top_level_csv(&args_source)
            .into_iter()
            .map(|arg| lower_expr(&arg))
            .collect();
        return Some(LoweredLogicStatement::FunctionCall {
            name: name.to_string(),
            args,
        });
    }

    Some(LoweredLogicStatement::Raw {
        source: stmt.to_string(),
    })
}

fn lower_variable_declaration(stmt: &str) -> Option<Vec<String>> {
    let rest = stmt.strip_prefix("var ")?;
    if rest.contains('=') {
        return None;
    }
    let names = split_top_level_csv(rest)
        .into_iter()
        .map(|name| name.trim().trim_end_matches(';').to_string())
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>();
    if names.is_empty() {
        return None;
    }

    Some(names)
}

fn lower_step_assignment(target: &str, op: &str) -> Option<LoweredLogicStatement> {
    let target = target.trim();
    if target.is_empty() {
        return None;
    }

    Some(LoweredLogicStatement::Assignment {
        target: lower_expr(target),
        value: LoweredLogicExpr::BinaryExpr {
            op: op.to_string(),
            left: Box::new(lower_expr(target)),
            right: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
        },
    })
}

fn lower_if_statement(stmt: &str) -> Option<LoweredLogicStatement> {
    let (condition, body, rest) = lower_conditional_parts(stmt, "if")?;
    let then_branch = lower_branch_body(&body);
    let else_branch = lower_else_branch(&rest);
    Some(LoweredLogicStatement::Conditional {
        condition: lower_expr(&condition),
        then_branch,
        else_branch,
    })
}

fn lower_else_branch(rest: &str) -> Vec<LoweredLogicStatement> {
    let rest = rest.trim();
    if rest.is_empty() {
        return Vec::new();
    }

    if let Some(after_else) = rest.strip_prefix("else") {
        let after_else = after_else.trim_start();
        if after_else.starts_with('{') {
            if let Some((body, tail)) = extract_braced_block(after_else) {
                let mut lowered = lower_source(&body);
                lowered.extend(lower_else_branch(&tail));
                return lowered;
            }
        }

        if after_else.starts_with("if") {
            if let Some(stmt) = lower_if_statement(after_else) {
                return vec![stmt];
            }
        }

        if let Some((stmt, tail)) = split_inline_branch_statement(after_else) {
            let mut lowered = lower_branch_body(&stmt);
            lowered.extend(lower_else_branch(&tail));
            return lowered;
        }
    }

    lower_source(rest)
}

fn lower_conditional_parts(stmt: &str, keyword: &str) -> Option<(String, String, String)> {
    if let Some(parts) = lower_block_statement_parts(stmt, keyword) {
        return Some(parts);
    }

    lower_inline_conditional_parts(stmt, keyword)
}

fn lower_inline_conditional_parts(stmt: &str, keyword: &str) -> Option<(String, String, String)> {
    let trimmed = stmt.trim_start();
    let rest = trimmed.strip_prefix(keyword)?.trim_start();
    let (head, tail) = if rest.starts_with('(') {
        extract_parenthesized_block(rest)?
    } else {
        let boundary = rest.find(char::is_whitespace).unwrap_or(rest.len());
        (
            rest[..boundary].trim().to_string(),
            rest[boundary..].to_string(),
        )
    };

    let tail = tail.trim_start();
    let (body, after_body) = split_inline_branch_statement(tail)?;
    Some((head.trim().to_string(), body, after_body))
}

fn split_inline_branch_statement(source: &str) -> Option<(String, String)> {
    let trimmed = source.trim_start();
    if trimmed.is_empty() {
        return None;
    }

    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;

    for (index, ch) in trimmed.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            ';' if paren_depth == 0 && brace_depth == 0 => {
                let body = trimmed[..index].trim();
                let rest = trimmed[index + ch.len_utf8()..].trim_start();
                if !body.is_empty() {
                    return Some((body.to_string(), rest.to_string()));
                }
                return None;
            }
            _ => {}
        }

        if paren_depth == 0 && brace_depth == 0 {
            let tail = trimmed[index..].trim_start();
            if index > 0 && tail.starts_with("else") {
                let body = trimmed[..index].trim();
                if !body.is_empty() {
                    return Some((body.to_string(), tail.to_string()));
                }
                return None;
            }
        }
    }

    Some((trimmed.trim().to_string(), String::new()))
}

fn lower_branch_body(body: &str) -> Vec<LoweredLogicStatement> {
    let lowered = lower_source(body);
    if lowered.is_empty() {
        lower_statement(body).into_iter().collect()
    } else {
        lowered
    }
}

fn lower_block_statement(stmt: &str, keyword: &str) -> Option<(String, String)> {
    let (head, body, _) = lower_block_statement_parts(stmt, keyword)?;
    Some((head, body))
}

fn lower_block_statement_parts(stmt: &str, keyword: &str) -> Option<(String, String, String)> {
    let trimmed = stmt.trim_start();
    let rest = trimmed.strip_prefix(keyword)?.trim_start();
    let (head, body, tail) = split_head_and_body(rest)?;
    Some((head, body, tail))
}

fn lower_for_statement(stmt: &str) -> Option<LoweredLogicStatement> {
    let (head, body, _) = lower_block_statement_parts(stmt, "for")?;
    let mut parts = split_top_level_commas_or_semicolons(&head);
    if parts.len() != 3 {
        return Some(LoweredLogicStatement::Raw {
            source: stmt.trim().to_string(),
        });
    }

    let init = parts.remove(0);
    let condition = parts.remove(0);
    let step = parts.remove(0);
    Some(LoweredLogicStatement::For {
        init: lower_expr(&init),
        condition: lower_expr(&condition),
        step: lower_for_step_expr(&step),
        body: lower_source(&body),
    })
}

fn lower_for_step_expr(step: &str) -> LoweredLogicExpr {
    match lower_statement(step) {
        Some(LoweredLogicStatement::Assignment { target, value }) => LoweredLogicExpr::BinaryExpr {
            op: "=".to_string(),
            left: Box::new(target),
            right: Box::new(value),
        },
        _ => lower_expr(step),
    }
}
