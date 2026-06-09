use std::collections::{HashMap, HashSet};

use iwm_runtime_model::RoomDefinition;

use super::context::RuntimeEvalContext;
use super::eval::{assignable_key, evaluate_expr, is_truthy};
use super::statement::assign_instance_or_global;
use crate::helpers::as_number;
use crate::{
    LoweredLogicEntry, LoweredLogicExpr, LoweredLogicStatement, RuntimeCore, RuntimeInstance,
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
        let mut index = 0usize;
        while index < room_state.instances.len() {
            let source_instance_block = room_state
                .instances
                .get(index)
                .and_then(|instance| {
                    source_room
                        .instances
                        .iter()
                        .find(|candidate| candidate.instance_id == instance.instance_id)
                })
                .and_then(|instance| instance.creation_block_id.clone());
            if let Some(block_id) = source_instance_block.as_deref() {
                if let Some(entry) = self.lowered_logic_entry(block_id).cloned() {
                    for statement in &entry.statements {
                        self.apply_create_statement_to_instance(statement, room_state, index);
                    }
                }
            }

            let object_id = room_state.instances[index].object_id;
            if let Some(block_ids) = create_event_blocks.get(&object_id).cloned() {
                for block_id in block_ids {
                    let Some(entry) = self.lowered_logic_entry(&block_id).cloned() else {
                        continue;
                    };
                    for statement in &entry.statements {
                        self.apply_create_statement_to_instance(statement, room_state, index);
                    }
                }
            }

            index += 1;
        }
    }
    fn apply_lowered_block_to_globals(&mut self, block_id: &str) {
        let Some(entry) = self.lowered_logic_entry(block_id).cloned() else {
            return;
        };

        let script_entries = self.lowered_script_entries();
        for statement in &entry.statements {
            apply_statement_to_globals_map(statement, &script_entries, &mut self.globals);
        }
    }
    pub(crate) fn collect_package_bootstrap_globals(&self) -> HashMap<String, RuntimeValue> {
        let script_entries = self.lowered_script_entries();
        let mut globals = HashMap::new();

        for room in &self.package.rooms {
            for instance in &room.instances {
                let Some(block_id) = instance.creation_block_id.as_deref() else {
                    continue;
                };
                let Some(entry) = self.lowered_logic_entry(block_id) else {
                    continue;
                };
                if !block_references_global_assignments(&entry.statements) {
                    continue;
                }
                for statement in &entry.statements {
                    apply_statement_to_globals_map(statement, &script_entries, &mut globals);
                }
            }
        }

        globals
    }
    pub(crate) fn hydrate_missing_package_bootstrap_globals(&mut self) {
        for (key, value) in self.package_bootstrap_globals.clone() {
            self.globals.entry(key).or_insert(value);
        }
    }

    fn apply_create_statement_to_instance(
        &mut self,
        statement: &LoweredLogicStatement,
        room_state: &mut RuntimeRoomState,
        instance_index: usize,
    ) {
        let Some(instance_snapshot) = room_state.instances.get(instance_index).cloned() else {
            return;
        };

        match statement {
            LoweredLogicStatement::Assignment { target, value } => {
                if let Some(key) = assignable_key(target, Some(&instance_snapshot), None) {
                    let button_states = HashMap::new();
                    let known_files = HashSet::new();
                    let eval_context = RuntimeEvalContext {
                        current_room_id: room_state.room_id,
                        button_states: &button_states,
                        room_instances: &room_state.instances,
                        room_instance_overlay: &[],
                        room_order: &[],
                        known_files: &known_files,
                        other_instance: None,
                        other_runtime_id: None,
                        place_target_ids_by_name: &self.place_target_ids_by_name,
                    };
                    if let Some(value) = evaluate_expr(
                        value,
                        Some(&instance_snapshot),
                        &self.globals,
                        None,
                        Some(&eval_context),
                    ) {
                        if let Some(instance) = room_state.instances.get_mut(instance_index) {
                            assign_instance_or_global(key, value, instance, &mut self.globals);
                        }
                    }
                }
            }
            LoweredLogicStatement::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                let button_states = HashMap::new();
                let known_files = HashSet::new();
                let eval_context = RuntimeEvalContext {
                    current_room_id: room_state.room_id,
                    button_states: &button_states,
                    room_instances: &room_state.instances,
                    room_instance_overlay: &[],
                    room_order: &[],
                    known_files: &known_files,
                    other_instance: None,
                    other_runtime_id: None,
                    place_target_ids_by_name: &self.place_target_ids_by_name,
                };
                let condition_value = evaluate_expr(
                    condition,
                    Some(&instance_snapshot),
                    &self.globals,
                    None,
                    Some(&eval_context),
                );
                let branch = if is_truthy(condition_value) {
                    then_branch
                } else {
                    else_branch
                };
                for nested in branch {
                    self.apply_create_statement_to_instance(nested, room_state, instance_index);
                }
            }
            LoweredLogicStatement::FunctionCall { name, args } => match name.as_str() {
                "instance_create" => {
                    self.apply_create_instance_create(args, room_state, Some(&instance_snapshot));
                }
                _ => {
                    let script_entries = self.lowered_script_entries();
                    if let Some(entry) = script_entries.get(name) {
                        for nested in &entry.statements {
                            self.apply_create_statement_to_instance(
                                nested,
                                room_state,
                                instance_index,
                            );
                        }
                    }
                }
            },
            LoweredLogicStatement::With { body, .. }
            | LoweredLogicStatement::Repeat { body, .. }
            | LoweredLogicStatement::While { body, .. }
            | LoweredLogicStatement::For { body, .. } => {
                for nested in body {
                    self.apply_create_statement_to_instance(nested, room_state, instance_index);
                }
            }
            LoweredLogicStatement::VariableDeclaration { .. }
            | LoweredLogicStatement::Return { .. }
            | LoweredLogicStatement::Raw { .. } => {}
        }
    }

    pub(crate) fn apply_room_start_logic(&mut self, room_state: &mut RuntimeRoomState) {
        let room_start_event_blocks = self.object_event_blocks_by_tag("other:room-start");
        let initial_instance_count = room_state.instances.len();
        let mut index = 0usize;

        while index < initial_instance_count {
            let object_id = room_state.instances[index].object_id;
            if let Some(block_ids) = room_start_event_blocks.get(&object_id).cloned() {
                for block_id in block_ids {
                    let Some(entry) = self.lowered_logic_entry(&block_id).cloned() else {
                        continue;
                    };
                    for statement in &entry.statements {
                        self.apply_create_statement_to_instance(statement, room_state, index);
                    }
                }
            }

            index += 1;
        }
    }

    fn apply_create_instance_create(
        &mut self,
        args: &[LoweredLogicExpr],
        room_state: &mut RuntimeRoomState,
        source_instance: Option<&RuntimeInstance>,
    ) {
        let button_states = HashMap::new();
        let known_files = HashSet::new();
        let eval_context = RuntimeEvalContext {
            current_room_id: room_state.room_id,
            button_states: &button_states,
            room_instances: &room_state.instances,
            room_instance_overlay: &[],
            room_order: &[],
            known_files: &known_files,
            other_instance: None,
            other_runtime_id: None,
            place_target_ids_by_name: &self.place_target_ids_by_name,
        };
        let x = args
            .first()
            .and_then(|arg| {
                evaluate_expr(
                    arg,
                    source_instance,
                    &self.globals,
                    None,
                    Some(&eval_context),
                )
            })
            .and_then(|value| as_number(&value))
            .unwrap_or(0.0);
        let y = args
            .get(1)
            .and_then(|arg| {
                evaluate_expr(
                    arg,
                    source_instance,
                    &self.globals,
                    None,
                    Some(&eval_context),
                )
            })
            .and_then(|value| as_number(&value))
            .unwrap_or(0.0);
        let object_name = args.get(2).and_then(|arg| match arg {
            LoweredLogicExpr::Identifier(name) => Some(name.as_str()),
            _ => None,
        });
        let Some(object_id) = object_name.and_then(|name| {
            self.package
                .objects
                .iter()
                .find(|object| object.name.eq_ignore_ascii_case(name))
                .map(|object| object.id)
        }) else {
            return;
        };

        if room_state
            .instances
            .iter()
            .any(|instance| instance.alive && instance.object_id == object_id)
        {
            return;
        }

        let runtime_id = room_state.instances.len();
        let Some(new_instance) = self.instantiate_runtime_object(object_id, runtime_id, x, y)
        else {
            return;
        };
        room_state.instances.push(new_instance);

        let create_event_blocks = self.object_event_blocks_by_tag("create");
        if let Some(block_ids) = create_event_blocks.get(&object_id).cloned() {
            for block_id in block_ids {
                let Some(entry) = self.lowered_logic_entry(&block_id).cloned() else {
                    continue;
                };
                for nested in &entry.statements {
                    self.apply_create_statement_to_instance(nested, room_state, runtime_id);
                }
            }
        }
    }
}

pub(crate) fn apply_view_globals_to_room(
    room: &mut RuntimeRoomState,
    globals: &HashMap<String, RuntimeValue>,
) {
    let Some(view) = room.views.iter_mut().find(|view| view.visible) else {
        return;
    };

    if let Some(value) = globals
        .get("view_xview[0]")
        .or_else(|| globals.get("view_xview"))
        .and_then(as_number)
    {
        view.source_x = value.round() as i32;
    }
    if let Some(value) = globals
        .get("view_yview[0]")
        .or_else(|| globals.get("view_yview"))
        .and_then(as_number)
    {
        view.source_y = value.round() as i32;
    }
    if let Some(value) = globals
        .get("view_wview[0]")
        .or_else(|| globals.get("view_wview"))
        .and_then(as_number)
    {
        if value > 0.0 {
            view.source_w = value.round() as u32;
        }
    }
    if let Some(value) = globals
        .get("view_hview[0]")
        .or_else(|| globals.get("view_hview"))
        .and_then(as_number)
    {
        if value > 0.0 {
            view.source_h = value.round() as u32;
        }
    }
}

fn apply_statement_to_globals_map(
    statement: &LoweredLogicStatement,
    script_entries: &HashMap<String, LoweredLogicEntry>,
    globals: &mut HashMap<String, RuntimeValue>,
) {
    match statement {
        LoweredLogicStatement::Assignment { target, value } => {
            if let Some(key) = assignable_key(target, None, None) {
                if let Some(value) = evaluate_expr(value, None, globals, None, None) {
                    globals.insert(key, value);
                }
            }
        }
        LoweredLogicStatement::Conditional {
            condition,
            then_branch,
            else_branch,
        } => {
            let condition_value = evaluate_expr(condition, None, globals, None, None);
            let branch = if is_truthy(condition_value) {
                then_branch
            } else {
                else_branch
            };
            for nested in branch {
                apply_statement_to_globals_map(nested, script_entries, globals);
            }
        }
        LoweredLogicStatement::With { body, .. }
        | LoweredLogicStatement::Repeat { body, .. }
        | LoweredLogicStatement::While { body, .. }
        | LoweredLogicStatement::For { body, .. } => {
            for nested in body {
                apply_statement_to_globals_map(nested, script_entries, globals);
            }
        }
        LoweredLogicStatement::FunctionCall { name, .. } => {
            if let Some(entry) = script_entries.get(name) {
                for nested in &entry.statements {
                    apply_statement_to_globals_map(nested, script_entries, globals);
                }
            }
        }
        LoweredLogicStatement::VariableDeclaration { .. }
        | LoweredLogicStatement::Return { .. }
        | LoweredLogicStatement::Raw { .. } => {}
    }
}

fn block_references_global_assignments(statements: &[LoweredLogicStatement]) -> bool {
    statements
        .iter()
        .any(statement_references_global_assignment)
}

fn statement_references_global_assignment(statement: &LoweredLogicStatement) -> bool {
    match statement {
        LoweredLogicStatement::Assignment { target, .. } => assignable_key(target, None, None)
            .map(|key| key.starts_with("global."))
            .unwrap_or(false),
        LoweredLogicStatement::Conditional {
            then_branch,
            else_branch,
            ..
        } => {
            block_references_global_assignments(then_branch)
                || block_references_global_assignments(else_branch)
        }
        LoweredLogicStatement::With { body, .. }
        | LoweredLogicStatement::Repeat { body, .. }
        | LoweredLogicStatement::While { body, .. }
        | LoweredLogicStatement::For { body, .. } => block_references_global_assignments(body),
        LoweredLogicStatement::FunctionCall { .. }
        | LoweredLogicStatement::VariableDeclaration { .. }
        | LoweredLogicStatement::Return { .. }
        | LoweredLogicStatement::Raw { .. } => false,
    }
}
