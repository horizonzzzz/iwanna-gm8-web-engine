use iwm_runtime_model::{
    LoweredLogicEntry, LoweredLogicExpr, LoweredLogicFile, LoweredLogicStatement,
};

use crate::models::RawLogicFile;

pub fn lower_raw_logic_file(raw: &RawLogicFile) -> LoweredLogicFile {
    let mut entries = Vec::new();

    for owner in &raw.room_creation_codes {
        entries.push(LoweredLogicEntry {
            block_id: owner.block_id.clone(),
            statements: lower_source(&owner.gml_source),
        });
    }

    for owner in &raw.instance_creation_codes {
        entries.push(LoweredLogicEntry {
            block_id: owner.block_id.clone(),
            statements: lower_source(&owner.gml_source),
        });
    }

    for event in &raw.object_events {
        let mut statements = Vec::new();
        for action in &event.actions {
            statements.extend(lower_source(&action.fn_code));
        }
        entries.push(LoweredLogicEntry {
            block_id: event.block_id.clone(),
            statements,
        });
    }

    for script in &raw.scripts {
        entries.push(LoweredLogicEntry {
            block_id: format!("script:{}", script.script_id),
            statements: lower_source(&script.gml_source),
        });
    }

    for trigger in &raw.triggers {
        entries.push(LoweredLogicEntry {
            block_id: format!("trigger:{}", trigger.trigger_id),
            statements: lower_source(&trigger.condition_gml),
        });
    }

    for moment in &raw.timelines {
        let mut statements = Vec::new();
        for action in &moment.actions {
            statements.extend(lower_source(&action.fn_code));
        }
        entries.push(LoweredLogicEntry {
            block_id: format!("timeline:{}:{}", moment.timeline_id, moment.moment),
            statements,
        });
    }

    LoweredLogicFile {
        format: "iwm-lowered-logic-v1".into(),
        entries,
    }
}

fn lower_source(source: &str) -> Vec<LoweredLogicStatement> {
    split_top_level_statements(source)
        .into_iter()
        .filter_map(|stmt| lower_statement(&stmt))
        .collect()
}

fn lower_statement(stmt: &str) -> Option<LoweredLogicStatement> {
    let stmt = stmt.trim();
    if stmt.is_empty() {
        return None;
    }

    if stmt.ends_with("++") && !stmt.ends_with("+++") {
        let target = stmt[..stmt.len() - 2].trim();
        if !target.is_empty() {
            return Some(LoweredLogicStatement::Assignment {
                target: lower_expr(target),
                value: LoweredLogicExpr::BinaryExpr {
                    op: "+".to_string(),
                    left: Box::new(lower_expr(target)),
                    right: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
                },
            });
        }
    }

    if stmt.ends_with("--") && !stmt.ends_with("---") {
        let target = stmt[..stmt.len() - 2].trim();
        if !target.is_empty() {
            return Some(LoweredLogicStatement::Assignment {
                target: lower_expr(target),
                value: LoweredLogicExpr::BinaryExpr {
                    op: "-".to_string(),
                    left: Box::new(lower_expr(target)),
                    right: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
                },
            });
        }
    }

    if let Some(target) = stmt.strip_prefix("++").map(str::trim) {
        if !target.is_empty() {
            return Some(LoweredLogicStatement::Assignment {
                target: lower_expr(target),
                value: LoweredLogicExpr::BinaryExpr {
                    op: "+".to_string(),
                    left: Box::new(lower_expr(target)),
                    right: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
                },
            });
        }
    }

    if stmt.starts_with("--") && !stmt.starts_with("---") {
        let target = stmt[2..].trim();
        if !target.is_empty() {
            return Some(LoweredLogicStatement::Assignment {
                target: lower_expr(target),
                value: LoweredLogicExpr::BinaryExpr {
                    op: "-".to_string(),
                    left: Box::new(lower_expr(target)),
                    right: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
                },
            });
        }
    }

    if stmt.starts_with("if ") || stmt.starts_with("if(") {
        return lower_if_statement(stmt);
    }

    if stmt.starts_with("with ") || stmt.starts_with("with(") {
        return lower_block_statement(stmt, "with").map(|(head, body)| LoweredLogicStatement::With {
            target: lower_expr(&head),
            body: lower_source(&body),
        });
    }

    if stmt.starts_with("repeat ") || stmt.starts_with("repeat(") {
        return lower_block_statement(stmt, "repeat").map(|(head, body)| LoweredLogicStatement::Repeat {
            count: lower_expr(&head),
            body: lower_source(&body),
        });
    }

    if stmt.starts_with("while ") || stmt.starts_with("while(") {
        return lower_block_statement(stmt, "while").map(|(head, body)| LoweredLogicStatement::While {
            condition: lower_expr(&head),
            body: lower_source(&body),
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
        if !lhs.contains("==") && !lhs.contains(">=") && !lhs.contains("<=") && !lhs.contains("!=") {
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
        let args = split_top_level_csv(args_source)
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

fn lower_if_statement(stmt: &str) -> Option<LoweredLogicStatement> {
    let (condition, body, rest) = lower_block_statement_parts(stmt, "if")?;
    let then_branch = lower_source(&body);
    let else_branch = lower_else_branch(&rest);
    Some(LoweredLogicStatement::Conditional {
        condition: lower_expr(&condition),
        then_branch,
        else_branch,
    })
}

fn lower_expr(expr: &str) -> LoweredLogicExpr {
    let expr = expr.trim();
    if expr.is_empty() {
        return LoweredLogicExpr::Raw {
            source: String::new(),
        };
    }

    // Strip balanced outer parentheses first
    let expr = strip_balanced_outer_parens(expr);

    if let Some(binary) = lower_binary_expr(expr) {
        return binary;
    }

    if let Some(call) = lower_call_expr(expr) {
        return call;
    }

    if let Some(index) = lower_index_expr(expr) {
        return index;
    }

    if let Some(member) = lower_member_expr(expr) {
        return member;
    }

    if let Ok(number) = expr.parse::<f64>() {
        return LoweredLogicExpr::LiteralNumber(number);
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
    // Comparison and arithmetic operators
    for op in ["==", "!=", ">=", "<=", "+", "-", "*", "/", ">", "<"] {
        if let Some((left, right)) = split_top_level_operator(expr, op) {
            return Some(LoweredLogicExpr::BinaryExpr {
                op: op.to_string(),
                left: Box::new(lower_expr(&left)),
                right: Box::new(lower_expr(&right)),
            });
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
    let args = split_top_level_csv(args_source)
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
    }

    lower_source(rest)
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
        step: lower_expr(&step),
        body: lower_source(&body),
    })
}

fn split_head_and_body(rest: &str) -> Option<(String, String, String)> {
    let rest = rest.trim_start();
    let (head, tail) = if rest.starts_with('(') {
        extract_parenthesized_block(rest)?
    } else {
        let brace_index = rest.find('{')?;
        (rest[..brace_index].trim().to_string(), rest[brace_index..].to_string())
    };

    let tail = tail.trim_start();
    let (body, after_body) = extract_braced_block(tail)?;
    Some((normalize_group_head(&head), body, after_body.to_string()))
}

fn normalize_group_head(head: &str) -> String {
    let trimmed = head.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        let inner = &trimmed[1..trimmed.len() - 1];
        if extract_parenthesized_block(trimmed)
            .map(|(_, rest)| rest.trim().is_empty())
            .unwrap_or(false)
        {
            return inner.trim().to_string();
        }
    }

    trimmed.to_string()
}

fn extract_parenthesized_block(input: &str) -> Option<(String, String)> {
    let mut depth = 0usize;
    let mut start = None;
    for (index, ch) in input.char_indices() {
        match ch {
            '(' => {
                depth += 1;
                if start.is_none() {
                    start = Some(index + ch.len_utf8());
                }
            }
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let begin = start?;
                    let inside = input[begin..index].to_string();
                    let rest = input[index + ch.len_utf8()..].to_string();
                    return Some((inside, rest));
                }
            }
            _ => {}
        }
    }
    None
}

fn extract_braced_block(input: &str) -> Option<(String, String)> {
    let mut depth = 0usize;
    let mut start = None;
    for (index, ch) in input.char_indices() {
        match ch {
            '{' => {
                depth += 1;
                if start.is_none() {
                    start = Some(index + ch.len_utf8());
                }
            }
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let begin = start?;
                    let inside = input[begin..index].to_string();
                    let rest = input[index + ch.len_utf8()..].to_string();
                    return Some((inside, rest));
                }
            }
            _ => {}
        }
    }
    None
}

fn strip_balanced_outer_parens(s: &str) -> &str {
    let trimmed = s.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        let inner = &trimmed[1..trimmed.len() - 1];
        let mut depth = 0usize;
        let mut valid = true;
        for ch in inner.chars() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    if depth == 0 {
                        valid = false;
                        break;
                    }
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }
        }
        if valid && depth == 0 {
            return strip_balanced_outer_parens(inner);
        }
    }
    trimmed
}

fn split_top_level_statements(source: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;

    for (index, ch) in source.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' => {
                brace_depth += 1;
            }
            '}' => {
                brace_depth = brace_depth.saturating_sub(1);
                current.push(ch);
                if paren_depth == 0 && brace_depth == 0 {
                    let next = source[index + ch.len_utf8()..].trim_start();
                    if !next.starts_with("else") {
                        let stmt = current.trim();
                        if !stmt.is_empty() {
                            statements.push(stmt.to_string());
                        }
                        current.clear();
                        continue;
                    }
                }
                continue;
            }
            ';' if paren_depth == 0 && brace_depth == 0 => {
                let stmt = current.trim();
                if !stmt.is_empty() {
                    statements.push(stmt.to_string());
                }
                current.clear();
                continue;
            }
            _ => {}
        }
        current.push(ch);
    }

    let stmt = current.trim();
    if !stmt.is_empty() {
        statements.push(stmt.to_string());
    }

    statements
}

fn split_top_level_commas_or_semicolons(source: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;

    for ch in source.chars() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            ';' if paren_depth == 0 && brace_depth == 0 => {
                let part = current.trim();
                if !part.is_empty() {
                    parts.push(part.to_string());
                }
                current.clear();
                continue;
            }
            _ => {}
        }
        current.push(ch);
    }

    let part = current.trim();
    if !part.is_empty() {
        parts.push(part.to_string());
    }

    parts
}

fn split_top_level_csv(source: String) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;

    for ch in source.chars() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            ',' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                let part = current.trim();
                if !part.is_empty() {
                    parts.push(part.to_string());
                }
                current.clear();
                continue;
            }
            _ => {}
        }

        current.push(ch);
    }

    let part = current.trim();
    if !part.is_empty() {
        parts.push(part.to_string());
    }

    parts
}

fn split_top_level_operator(source: &str, operator: &str) -> Option<(String, String)> {
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let chars: Vec<(usize, char)> = source.char_indices().collect();
    let op_len = operator.len();
    let mut i = 0usize;

    while i < chars.len() {
        let (byte_index, ch) = chars[i];
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
            let tail = &source[byte_index..];
            if tail.starts_with(operator) {
                let left = source[..byte_index].trim();
                let right = source[byte_index + op_len..].trim();
                if !left.is_empty() && !right.is_empty() {
                    return Some((left.to_string(), right.to_string()));
                }
            }
        }

        i += 1;
    }

    None
}

fn split_top_level_trailing_index(source: &str) -> Option<(String, String)> {
    if !source.ends_with(']') {
        return None;
    }

    let mut bracket_depth = 0usize;
    for (index, ch) in source.char_indices().rev() {
        match ch {
            ']' => bracket_depth += 1,
            '[' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                if bracket_depth == 0 {
                    let target = source[..index].trim();
                    let index_expr = source[index + 1..source.len() - 1].trim();
                    if !target.is_empty() && !index_expr.is_empty() {
                        return Some((target.to_string(), index_expr.to_string()));
                    }
                    return None;
                }
            }
            _ => {}
        }
    }

    None
}

fn find_top_level_dot(source: &str) -> Option<usize> {
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;

    for (index, ch) in source.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '.' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => return Some(index),
            _ => {}
        }
    }

    None
}
