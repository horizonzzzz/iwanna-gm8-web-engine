mod expression;
mod source;
mod statement;
mod syntax;

use iwm_runtime_model::{LoweredLogicEntry, LoweredLogicFile, LoweredLogicStatement};

use crate::models::{RawCodeAction, RawLogicFile};

use self::source::{looks_like_gml_source, lower_source};

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
            statements.extend(lower_action_source(action));
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
            statements.extend(lower_action_source(action));
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

fn lower_action_source(action: &RawCodeAction) -> Vec<LoweredLogicStatement> {
    let primary = action.fn_code.trim();
    if !primary.is_empty() {
        return lower_source(primary);
    }

    action
        .args
        .iter()
        .filter(|arg| looks_like_gml_source(arg))
        .flat_map(|arg| lower_source(arg))
        .collect()
}
