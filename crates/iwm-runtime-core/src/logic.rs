use std::collections::HashMap;

use iwm_runtime_host::RuntimeHost;
use iwm_runtime_model::RoomDefinition;

use crate::helpers::{parse_room_id, parse_runtime_value, record_host_diagnostic};
use crate::{
    LoweredLogicEntry, LoweredLogicStatement, RuntimeCore, RuntimeCoreError, RuntimeInstance,
    RuntimeRoomState, RuntimeValue,
};

impl RuntimeCore {
    pub(crate) fn apply_create_logic(
        &mut self,
        room_state: &mut RuntimeRoomState,
        source_room: &RoomDefinition,
    ) {
        if let Some(block_id) = source_room.creation_block_id.as_deref() {
            self.apply_lowered_block_to_globals(block_id);
        }

        let create_event_blocks = self.object_event_blocks_by_tag("create");

        for instance in &mut room_state.instances {
            if let Some(source_instance) = source_room
                .instances
                .iter()
                .find(|candidate| candidate.instance_id == instance.instance_id)
            {
                if let Some(block_id) = source_instance.creation_block_id.as_deref() {
                    self.apply_lowered_block_to_instance(block_id, instance);
                }
            }

            if let Some(block_ids) = create_event_blocks.get(&instance.object_id) {
                for block_id in block_ids {
                    self.apply_lowered_block_to_instance(block_id, instance);
                }
            }
        }
    }

    pub(crate) fn execute_lowered_step_events<H: RuntimeHost>(
        &mut self,
        host: &mut H,
    ) -> Result<bool, RuntimeCoreError> {
        let Some(room) = self.current_room.as_ref() else {
            return Err(RuntimeCoreError::NoRooms);
        };

        let step_event_blocks = self.object_event_blocks_by_tag("step");
        let dispatches = room
            .instances
            .iter()
            .enumerate()
            .filter(|(_, instance)| instance.alive)
            .filter_map(|(index, instance)| {
                let entries = step_event_blocks
                    .get(&instance.object_id)
                    .into_iter()
                    .flat_map(|block_ids| block_ids.iter())
                    .filter_map(|block_id| self.lowered_logic_entry(block_id).cloned())
                    .collect::<Vec<_>>();

                if entries.is_empty() {
                    None
                } else {
                    Some((index, entries))
                }
            })
            .collect::<Vec<_>>();

        for (index, entries) in dispatches {
            let Some(room) = self.current_room.as_mut() else {
                return Err(RuntimeCoreError::NoRooms);
            };
            let Some(instance) = room.instances.get_mut(index) else {
                continue;
            };
            if !instance.alive {
                continue;
            }

            for entry in &entries {
                for statement in &entry.statements {
                    apply_step_statement(
                        statement,
                        instance,
                        &mut self.globals,
                        &mut self.pending_room_transition,
                        &mut self.pending_room_reset,
                        host,
                        &mut self.diagnostics,
                    );
                    if self.pending_room_reset || self.pending_room_transition.is_some() {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    fn object_event_blocks_by_tag(&self, event_tag: &str) -> HashMap<usize, Vec<String>> {
        self.package
            .objects
            .iter()
            .map(|object| {
                let block_ids = object
                    .events
                    .iter()
                    .filter(|event| event.event_tag == event_tag)
                    .map(|event| event.block_id.clone())
                    .collect::<Vec<_>>();
                (object.id, block_ids)
            })
            .collect::<HashMap<_, _>>()
    }

    fn apply_lowered_block_to_globals(&mut self, block_id: &str) {
        let Some(entry) = self.lowered_logic_entry(block_id).cloned() else {
            return;
        };

        for statement in &entry.statements {
            self.apply_statement_to_globals(statement);
        }
    }

    fn apply_lowered_block_to_instance(&mut self, block_id: &str, instance: &mut RuntimeInstance) {
        let Some(entry) = self.lowered_logic_entry(block_id).cloned() else {
            return;
        };

        for statement in &entry.statements {
            self.apply_statement_to_instance(statement, instance);
        }
    }

    fn lowered_logic_entry(&self, block_id: &str) -> Option<&LoweredLogicEntry> {
        let index = self.lowered_logic_index.get(block_id)?;
        self.package
            .lowered_logic
            .as_ref()
            .and_then(|lowered_logic| lowered_logic.entries.get(*index))
    }

    fn apply_statement_to_globals(&mut self, statement: &LoweredLogicStatement) {
        match statement {
            LoweredLogicStatement::Assignment { lhs, rhs } => {
                if let Some(value) = parse_runtime_value(rhs) {
                    self.globals.insert(lhs.clone(), value);
                }
            }
            LoweredLogicStatement::Conditional {
                then_branch,
                else_branch,
                ..
            } => {
                for nested in then_branch {
                    self.apply_statement_to_globals(nested);
                }
                for nested in else_branch {
                    self.apply_statement_to_globals(nested);
                }
            }
            LoweredLogicStatement::With { body, .. }
            | LoweredLogicStatement::Repeat { body, .. }
            | LoweredLogicStatement::While { body, .. }
            | LoweredLogicStatement::For { body, .. } => {
                for nested in body {
                    self.apply_statement_to_globals(nested);
                }
            }
            LoweredLogicStatement::FunctionCall { .. } | LoweredLogicStatement::Raw { .. } => {}
        }
    }

    fn apply_statement_to_instance(
        &mut self,
        statement: &LoweredLogicStatement,
        instance: &mut RuntimeInstance,
    ) {
        match statement {
            LoweredLogicStatement::Assignment { lhs, rhs } => {
                if let Some(value) = parse_runtime_value(rhs) {
                    instance.vars.insert(lhs.clone(), value);
                }
            }
            LoweredLogicStatement::Conditional {
                then_branch,
                else_branch,
                ..
            } => {
                for nested in then_branch {
                    self.apply_statement_to_instance(nested, instance);
                }
                for nested in else_branch {
                    self.apply_statement_to_instance(nested, instance);
                }
            }
            LoweredLogicStatement::With { body, .. }
            | LoweredLogicStatement::Repeat { body, .. }
            | LoweredLogicStatement::While { body, .. }
            | LoweredLogicStatement::For { body, .. } => {
                for nested in body {
                    self.apply_statement_to_instance(nested, instance);
                }
            }
            LoweredLogicStatement::FunctionCall { .. } | LoweredLogicStatement::Raw { .. } => {}
        }
    }
}

fn apply_step_statement<H: RuntimeHost>(
    statement: &LoweredLogicStatement,
    instance: &mut RuntimeInstance,
    globals: &mut HashMap<String, RuntimeValue>,
    pending_room_transition: &mut Option<usize>,
    pending_room_reset: &mut bool,
    host: &mut H,
    diagnostics: &mut Vec<iwm_runtime_host::RuntimeDiagnostic>,
) {
    match statement {
        LoweredLogicStatement::Assignment { lhs, rhs } => {
            if let Some(value) = parse_runtime_value(rhs) {
                if lhs.starts_with("global.") {
                    globals.insert(lhs.clone(), value);
                } else {
                    instance.vars.insert(lhs.clone(), value);
                }
            }
        }
        LoweredLogicStatement::FunctionCall { name, args } => match name.as_str() {
            "room_goto" => {
                if let Some(room_id) = args.first().and_then(|arg| parse_room_id(arg)) {
                    *pending_room_transition = Some(room_id);
                } else {
                    record_host_diagnostic(
                        host,
                        diagnostics,
                        iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
                        "runtime-step-room-goto-unresolved",
                        format!(
                            "could not resolve room_goto target for {}",
                            instance.object_name
                        ),
                    );
                }
            }
            "game_restart" => {
                *pending_room_reset = true;
            }
            _ => {}
        },
        _ => {}
    }
}
