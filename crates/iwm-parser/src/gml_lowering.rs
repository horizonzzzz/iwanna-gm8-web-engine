use crate::models::RawLogicFile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoweredLogicFile {
    pub format: String,
    pub entries: Vec<LoweredLogicEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoweredLogicEntry {
    pub block_id: String,
    pub statements: Vec<LoweredLogicStatement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum LoweredLogicStatement {
    Assignment { lhs: String, rhs: String },
    FunctionCall { name: String, args: Vec<String> },
    Conditional {
        condition: String,
        then_branch: Vec<LoweredLogicStatement>,
        else_branch: Vec<LoweredLogicStatement>,
    },
    With {
        target: String,
        body: Vec<LoweredLogicStatement>,
    },
    Repeat {
        count: String,
        body: Vec<LoweredLogicStatement>,
    },
    While {
        condition: String,
        body: Vec<LoweredLogicStatement>,
    },
    For {
        init: String,
        condition: String,
        step: String,
        body: Vec<LoweredLogicStatement>,
    },
    Raw { source: String },
}

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

    if stmt.starts_with("if ") || stmt.starts_with("if(") {
        return lower_if_statement(stmt);
    }

    if stmt.starts_with("with ") || stmt.starts_with("with(") {
        return lower_block_statement(stmt, "with").map(|(head, body)| LoweredLogicStatement::With {
            target: head,
            body: lower_source(&body),
        });
    }

    if stmt.starts_with("repeat ") || stmt.starts_with("repeat(") {
        return lower_block_statement(stmt, "repeat").map(|(head, body)| LoweredLogicStatement::Repeat {
            count: head,
            body: lower_source(&body),
        });
    }

    if stmt.starts_with("while ") || stmt.starts_with("while(") {
        return lower_block_statement(stmt, "while").map(|(head, body)| LoweredLogicStatement::While {
            condition: head,
            body: lower_source(&body),
        });
    }

    if stmt.starts_with("for ") || stmt.starts_with("for(") {
        return lower_for_statement(stmt);
    }

    if let Some((lhs, rhs)) = stmt.split_once('=') {
        if !lhs.contains("==") && !lhs.contains(">=") && !lhs.contains("<=") && !lhs.contains("!=") {
            return Some(LoweredLogicStatement::Assignment {
                lhs: lhs.trim().to_string(),
                rhs: rhs.trim().to_string(),
            });
        }
    }

    if let Some((name, rest)) = stmt.split_once('(') {
        let args = rest
            .trim_end_matches(')')
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        return Some(LoweredLogicStatement::FunctionCall {
            name: name.trim().to_string(),
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
        condition,
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
        init,
        condition,
        step,
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
    head.trim()
        .trim_start_matches('(')
        .trim_end_matches(')')
        .trim()
        .to_string()
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
