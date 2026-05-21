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
    Conditional { condition: String, then_branch: Vec<String>, else_branch: Vec<String> },
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
    source
        .split(';')
        .filter_map(|raw| {
            let stmt = raw.trim();
            if stmt.is_empty() {
                return None;
            }

            if stmt.starts_with("if ") || stmt.starts_with("if(") {
                let condition = stmt
                    .strip_prefix("if")
                    .unwrap_or(stmt)
                    .trim()
                    .trim_start_matches('(')
                    .trim_end_matches(')')
                    .trim()
                    .to_string();
                return Some(LoweredLogicStatement::Conditional {
                    condition,
                    then_branch: Vec::new(),
                    else_branch: Vec::new(),
                });
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
                let args = rest.trim_end_matches(')').split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                return Some(LoweredLogicStatement::FunctionCall {
                    name: name.trim().to_string(),
                    args,
                });
            }

            Some(LoweredLogicStatement::Raw {
                source: stmt.to_string(),
            })
        })
        .collect()
}
