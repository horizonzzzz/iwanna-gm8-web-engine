use iwm_runtime_model::{
    LoweredLogicConditionalBranch, LoweredLogicExpr, LoweredLogicStatement, LoweredLogicSwitchCase,
};

use crate::expression::lower_expr;
use crate::source::lower_source;
use crate::syntax::{
    extract_braced_block, extract_parenthesized_block, split_head_and_body,
    split_top_level_commas_or_semicolons, split_top_level_csv, split_top_level_operator,
};

pub fn lower_statement(stmt: &str) -> Option<LoweredLogicStatement> {
    let stmt = stmt.trim();
    if stmt.is_empty() {
        return None;
    }

    if stmt == "exit" || stmt == "exit;" {
        return Some(LoweredLogicStatement::Return { value: None });
    }

    if stmt.starts_with("switch ") || stmt.starts_with("switch(") {
        return Some(
            lower_switch_statement(stmt).unwrap_or_else(|| LoweredLogicStatement::Raw {
                source: stmt.to_string(),
            }),
        );
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
        let (head, body) = lower_block_statement(stmt, "with")
            .map(|(head, body)| (head, lower_source(&body)))
            .or_else(|| {
                lower_inline_conditional_parts(stmt, "with")
                    .map(|(head, body, _)| (head, lower_branch_body(&body)))
            })?;
        return Some(LoweredLogicStatement::With {
            target: lower_expr(&head),
            body,
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

    if let Some((lhs, rhs)) = split_top_level_operator(stmt, "=") {
        return Some(LoweredLogicStatement::Assignment {
            target: lower_expr(&lhs),
            value: lower_expr(&rhs),
        });
    }

    if let Some(open_paren) = stmt.find('(') {
        let name = stmt[..open_paren].trim();
        let call_suffix = &stmt[open_paren..];
        let Some((args_source, rest)) = extract_parenthesized_block(call_suffix) else {
            return Some(LoweredLogicStatement::Raw {
                source: stmt.to_string(),
            });
        };
        if !is_identifier(name) || !rest.trim().trim_end_matches(';').trim().is_empty() {
            return Some(LoweredLogicStatement::Raw {
                source: stmt.to_string(),
            });
        }
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

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
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

fn lower_switch_statement(stmt: &str) -> Option<LoweredLogicStatement> {
    let (expression, body, _) = lower_block_statement_parts(stmt, "switch")?;
    let cases = split_switch_clauses(&body)?
        .into_iter()
        .map(|(value, source)| {
            let mut body = lower_source(&source);
            let break_after = body
                .iter()
                .position(|statement| {
                    matches!(
                        statement,
                        LoweredLogicStatement::Raw { source }
                            if source.trim().eq_ignore_ascii_case("break")
                                || source.trim().eq_ignore_ascii_case("break;")
                    )
                })
                .is_some_and(|index| {
                    body.truncate(index);
                    true
                });
            LoweredLogicSwitchCase {
                value: value.map(|value| lower_expr(&value)),
                body,
                break_after,
            }
        })
        .collect();

    Some(LoweredLogicStatement::Switch {
        expression: lower_expr(&expression),
        cases,
    })
}

fn split_switch_clauses(source: &str) -> Option<Vec<(Option<String>, String)>> {
    let mut clauses = Vec::new();
    let mut current_value = None;
    let mut body_start = 0usize;
    let mut found_label = false;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut quote = None;
    let mut index = 0usize;

    while index < source.len() {
        let ch = source[index..].chars().next()?;
        if let Some(open) = quote {
            if ch == open {
                quote = None;
            }
            index += ch.len_utf8();
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            index += ch.len_utf8();
            continue;
        }

        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            _ => {}
        }

        if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
            if let Some((value, next_index)) = parse_switch_label(source, index) {
                if found_label {
                    clauses.push((
                        current_value.take(),
                        source[body_start..index].trim().to_string(),
                    ));
                } else if !source[..index].trim().is_empty() {
                    return None;
                }
                found_label = true;
                current_value = value;
                body_start = next_index;
                index = next_index;
                continue;
            }
        }

        index += ch.len_utf8();
    }

    if !found_label {
        return None;
    }
    clauses.push((current_value, source[body_start..].trim().to_string()));
    Some(clauses)
}

fn parse_switch_label(source: &str, index: usize) -> Option<(Option<String>, usize)> {
    if keyword_at(source, index, "default") {
        let colon = source[index + "default".len()..].find(|ch: char| !ch.is_whitespace())?
            + index
            + "default".len();
        return (source.as_bytes().get(colon) == Some(&b':')).then_some((None, colon + 1));
    }
    if !keyword_at(source, index, "case") {
        return None;
    }

    let value_start = index + "case".len();
    let colon = find_switch_label_colon(source, value_start)?;
    let value = source[value_start..colon].trim();
    (!value.is_empty()).then_some((Some(value.to_string()), colon + 1))
}

fn keyword_at(source: &str, index: usize, keyword: &str) -> bool {
    let Some(candidate) = source.get(index..index + keyword.len()) else {
        return false;
    };
    if !candidate.eq_ignore_ascii_case(keyword) {
        return false;
    }
    let before = source[..index].chars().next_back();
    let after = source[index + keyword.len()..].chars().next();
    !is_identifier_char(before) && !is_identifier_char(after)
}

fn find_switch_label_colon(source: &str, start: usize) -> Option<usize> {
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut quote = None;

    for (offset, ch) in source[start..].char_indices() {
        if let Some(open) = quote {
            if ch == open {
                quote = None;
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            continue;
        }
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            ':' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                return Some(start + offset);
            }
            _ => {}
        }
    }
    None
}

fn is_identifier_char(ch: Option<char>) -> bool {
    matches!(ch, Some(ch) if ch == '_' || ch.is_ascii_alphanumeric())
}

fn lower_if_statement(stmt: &str) -> Option<LoweredLogicStatement> {
    let (condition, body, rest) = lower_conditional_parts(stmt, "if")?;
    let mut branches = vec![LoweredLogicConditionalBranch {
        condition: lower_expr(&condition),
        body: lower_branch_body(&body),
    }];
    let mut rest = rest;
    let else_branch = loop {
        let trimmed = rest.trim().to_string();
        if trimmed.is_empty() {
            break Vec::new();
        }
        let Some(after_else) = trimmed.strip_prefix("else").map(str::trim_start) else {
            break lower_source(&trimmed);
        };
        if after_else.starts_with("if ") || after_else.starts_with("if(") {
            let (condition, body, tail) = lower_conditional_parts(after_else, "if")?;
            branches.push(LoweredLogicConditionalBranch {
                condition: lower_expr(&condition),
                body: lower_branch_body(&body),
            });
            rest = tail;
            continue;
        }
        break lower_else_body(after_else);
    };

    if branches.len() == 1 {
        let branch = branches.pop().unwrap();
        Some(LoweredLogicStatement::Conditional {
            condition: branch.condition,
            then_branch: branch.body,
            else_branch,
        })
    } else {
        Some(LoweredLogicStatement::ConditionalChain {
            branches,
            else_branch,
        })
    }
}

fn lower_else_body(source: &str) -> Vec<LoweredLogicStatement> {
    if source.starts_with('{') {
        if let Some((body, tail)) = extract_braced_block(source) {
            let mut lowered = lower_source(&body);
            lowered.extend(lower_source(&tail));
            return lowered;
        }
    }

    if let Some((stmt, tail)) = split_inline_branch_statement(source) {
        let mut lowered = lower_branch_body(&stmt);
        lowered.extend(lower_source(&tail));
        return lowered;
    }

    lower_source(source)
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
