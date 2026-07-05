use std::{borrow::Cow, collections::HashMap};

use iwm_runtime_model::RoomDefinition;

use super::assignment::{assign_instance_or_global, assign_room_speed};
use super::context::RuntimeEvalContext;
use super::eval::{assignable_key, evaluate_expr, is_truthy};
use crate::helpers::as_number;
use crate::{
    LoweredLogicEntry, LoweredLogicExpr, LoweredLogicStatement, RuntimeCore, RuntimeInstance,
    RuntimeRoomState, RuntimeValue,
};

impl RuntimeCore {
    pub(crate) fn apply_create_logic_with_visible_instances(
        &mut self,
        room_state: &mut RuntimeRoomState,
        source_room: &RoomDefinition,
        visible_instances: &[RuntimeInstance],
    ) {
        if let Some(block_id) = source_room.creation_block_id.as_deref() {
            self.apply_lowered_block_to_globals(block_id, Some(&mut room_state.speed));
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
                        self.apply_create_statement_to_instance(
                            statement,
                            room_state,
                            index,
                            visible_instances,
                        );
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
                        self.apply_create_statement_to_instance(
                            statement,
                            room_state,
                            index,
                            visible_instances,
                        );
                    }
                }
            }

            index += 1;
        }
    }
    fn apply_lowered_block_to_globals(&mut self, block_id: &str, mut room_speed: Option<&mut u32>) {
        let Some(entry) = self.lowered_logic_entry(block_id).cloned() else {
            return;
        };

        let script_entries = self.lowered_script_entries();
        for statement in &entry.statements {
            apply_statement_to_globals_map(
                statement,
                &script_entries,
                &mut self.globals,
                room_speed.as_deref_mut(),
            );
        }
    }
    fn collect_package_bootstrap_globals_until_room(
        &self,
        target_room_id: usize,
    ) -> HashMap<String, RuntimeValue> {
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
                if !block_references_global_assignments(&entry.statements, &script_entries) {
                    continue;
                }
                for statement in &entry.statements {
                    apply_statement_to_globals_map(statement, &script_entries, &mut globals, None);
                }
            }
            if room.id == target_room_id {
                break;
            }
        }

        globals
    }

    pub(crate) fn hydrate_missing_package_bootstrap_globals(&mut self, target_room_id: usize) {
        for (key, value) in self.package_bootstrap_globals.clone() {
            self.globals.entry(key).or_insert(value);
        }
        for (key, value) in self.collect_package_bootstrap_globals_until_room(target_room_id) {
            self.globals.entry(key).or_insert(value);
        }
    }

    fn apply_create_statement_to_instance(
        &mut self,
        statement: &LoweredLogicStatement,
        room_state: &mut RuntimeRoomState,
        instance_index: usize,
        visible_instances: &[RuntimeInstance],
    ) {
        let Some(instance_snapshot) = room_state.instances.get(instance_index).cloned() else {
            return;
        };

        match statement {
            LoweredLogicStatement::Assignment { target, value } => {
                let button_states = HashMap::new();
                let room_instance_indices_by_object_id = HashMap::new();
                let room_instances = create_visible_instances(room_state, visible_instances);
                let eval_context = RuntimeEvalContext {
                    current_room_id: room_state.room_id,
                    room_speed: room_state.speed,
                    button_states: &button_states,
                    room_instances: room_instances.as_ref(),
                    room_instance_indices_by_object_id: &room_instance_indices_by_object_id,
                    object_index: None,
                    collision_spatial_index: None,
                    room_instance_overlay: super::RuntimeRoomInstanceOverlay::empty(),
                    room_order: &[],
                    other_instance: None,
                    other_runtime_id: None,
                    place_target_ids_by_name: &self.place_target_ids_by_name,
                    room_ids_by_name: &self.room_ids_by_name,
                };
                if let Some(key) = assignable_key(
                    target,
                    Some(&instance_snapshot),
                    &self.globals,
                    None,
                    Some(&eval_context),
                ) {
                    if let Some(value) = evaluate_expr(
                        value,
                        Some(&instance_snapshot),
                        &self.globals,
                        None,
                        Some(&eval_context),
                    ) {
                        if let Some(instance) = room_state.instances.get_mut(instance_index) {
                            assign_instance_or_global(
                                key,
                                value,
                                instance,
                                &mut self.globals,
                                Some(&mut room_state.speed),
                            );
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
                let room_instance_indices_by_object_id = HashMap::new();
                let room_instances = create_visible_instances(room_state, visible_instances);
                let eval_context = RuntimeEvalContext {
                    current_room_id: room_state.room_id,
                    room_speed: room_state.speed,
                    button_states: &button_states,
                    room_instances: room_instances.as_ref(),
                    room_instance_indices_by_object_id: &room_instance_indices_by_object_id,
                    object_index: None,
                    collision_spatial_index: None,
                    room_instance_overlay: super::RuntimeRoomInstanceOverlay::empty(),
                    room_order: &[],
                    other_instance: None,
                    other_runtime_id: None,
                    place_target_ids_by_name: &self.place_target_ids_by_name,
                    room_ids_by_name: &self.room_ids_by_name,
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
                    self.apply_create_statement_to_instance(
                        nested,
                        room_state,
                        instance_index,
                        visible_instances,
                    );
                }
            }
            LoweredLogicStatement::FunctionCall { name, args } => match name.as_str() {
                "instance_destroy" => {
                    if let Some(instance) = room_state.instances.get_mut(instance_index) {
                        instance.alive = false;
                    }
                }
                "instance_create" => {
                    self.apply_create_instance_create(
                        args,
                        room_state,
                        Some(&instance_snapshot),
                        visible_instances,
                    );
                }
                _ => {
                    let script_entries = self.lowered_script_entries();
                    if let Some(entry) = script_entries.get(name) {
                        for nested in &entry.statements {
                            self.apply_create_statement_to_instance(
                                nested,
                                room_state,
                                instance_index,
                                visible_instances,
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
                    self.apply_create_statement_to_instance(
                        nested,
                        room_state,
                        instance_index,
                        visible_instances,
                    );
                }
            }
            LoweredLogicStatement::VariableDeclaration { .. }
            | LoweredLogicStatement::Return { .. }
            | LoweredLogicStatement::Raw { .. } => {}
        }
    }

    pub(crate) fn apply_room_start_logic_with_visible_instances(
        &mut self,
        room_state: &mut RuntimeRoomState,
        visible_instances: &[RuntimeInstance],
    ) {
        let room_start_event_blocks = self.object_event_blocks_by_tag("other:room-start");
        let initial_instance_count = room_state.instances.len();
        let mut index = 0usize;

        while index < initial_instance_count {
            if !room_state
                .instances
                .get(index)
                .map(|instance| instance.alive)
                .unwrap_or(false)
            {
                index += 1;
                continue;
            }

            let object_id = room_state.instances[index].object_id;
            if let Some(block_ids) = room_start_event_blocks.get(&object_id).cloned() {
                for block_id in block_ids {
                    let Some(entry) = self.lowered_logic_entry(&block_id).cloned() else {
                        continue;
                    };
                    for statement in &entry.statements {
                        self.apply_create_statement_to_instance(
                            statement,
                            room_state,
                            index,
                            visible_instances,
                        );
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
        visible_instances: &[RuntimeInstance],
    ) {
        let button_states = HashMap::new();
        let room_instance_indices_by_object_id = HashMap::new();
        let room_instances = create_visible_instances(room_state, visible_instances);
        let eval_context = RuntimeEvalContext {
            current_room_id: room_state.room_id,
            room_speed: room_state.speed,
            button_states: &button_states,
            room_instances: room_instances.as_ref(),
            room_instance_indices_by_object_id: &room_instance_indices_by_object_id,
            object_index: None,
            collision_spatial_index: None,
            room_instance_overlay: super::RuntimeRoomInstanceOverlay::empty(),
            room_order: &[],
            other_instance: None,
            other_runtime_id: None,
            place_target_ids_by_name: &self.place_target_ids_by_name,
            room_ids_by_name: &self.room_ids_by_name,
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
        let Some(object_id) = args.get(2).and_then(|arg| {
            self.create_instance_object_id(arg, source_instance, Some(&eval_context))
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
                    self.apply_create_statement_to_instance(
                        nested,
                        room_state,
                        runtime_id,
                        visible_instances,
                    );
                }
            }
        }
    }

    fn create_instance_object_id(
        &self,
        expr: &LoweredLogicExpr,
        source_instance: Option<&RuntimeInstance>,
        eval_context: Option<&RuntimeEvalContext<'_>>,
    ) -> Option<usize> {
        if let LoweredLogicExpr::Identifier(name) = expr {
            if let Some(object_id) = self
                .package
                .objects
                .iter()
                .find(|object| object.name.eq_ignore_ascii_case(name))
                .map(|object| object.id)
            {
                return Some(object_id);
            }
        }

        evaluate_expr(expr, source_instance, &self.globals, None, eval_context)
            .and_then(|value| as_number(&value))
            .and_then(non_negative_integer_usize)
    }
}

fn create_visible_instances<'a>(
    room_state: &'a RuntimeRoomState,
    visible_instances: &'a [RuntimeInstance],
) -> Cow<'a, [RuntimeInstance]> {
    if visible_instances.is_empty() {
        return Cow::Borrowed(&room_state.instances);
    }

    let mut instances = room_state.instances.clone();
    instances.extend_from_slice(visible_instances);
    Cow::Owned(instances)
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

fn non_negative_integer_usize(value: f64) -> Option<usize> {
    if value.is_finite() && value >= 0.0 && value.fract() == 0.0 {
        Some(value as usize)
    } else {
        None
    }
}

fn apply_statement_to_globals_map(
    statement: &LoweredLogicStatement,
    script_entries: &HashMap<String, LoweredLogicEntry>,
    globals: &mut HashMap<String, RuntimeValue>,
    mut room_speed: Option<&mut u32>,
) {
    match statement {
        LoweredLogicStatement::Assignment { target, value } => {
            if let Some(key) = assignable_key(target, None, globals, None, None) {
                if let Some(value) = evaluate_expr(value, None, globals, None, None) {
                    if assign_room_speed(&key, &value, room_speed.as_deref_mut()) {
                        return;
                    }
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
                apply_statement_to_globals_map(
                    nested,
                    script_entries,
                    globals,
                    room_speed.as_deref_mut(),
                );
            }
        }
        LoweredLogicStatement::With { body, .. }
        | LoweredLogicStatement::Repeat { body, .. }
        | LoweredLogicStatement::While { body, .. }
        | LoweredLogicStatement::For { body, .. } => {
            for nested in body {
                apply_statement_to_globals_map(
                    nested,
                    script_entries,
                    globals,
                    room_speed.as_deref_mut(),
                );
            }
        }
        LoweredLogicStatement::FunctionCall { name, .. } => {
            if let Some(entry) = script_entries.get(name) {
                for nested in &entry.statements {
                    apply_statement_to_globals_map(
                        nested,
                        script_entries,
                        globals,
                        room_speed.as_deref_mut(),
                    );
                }
            }
        }
        LoweredLogicStatement::VariableDeclaration { .. }
        | LoweredLogicStatement::Return { .. }
        | LoweredLogicStatement::Raw { .. } => {}
    }
}

fn block_references_global_assignments(
    statements: &[LoweredLogicStatement],
    script_entries: &HashMap<String, LoweredLogicEntry>,
) -> bool {
    statements
        .iter()
        .any(|statement| statement_references_global_assignment(statement, script_entries))
}

fn statement_references_global_assignment(
    statement: &LoweredLogicStatement,
    script_entries: &HashMap<String, LoweredLogicEntry>,
) -> bool {
    match statement {
        LoweredLogicStatement::Assignment { target, .. } => {
            let globals = HashMap::new();
            assignable_key(target, None, &globals, None, None)
                .map(|key| key.starts_with("global."))
                .unwrap_or(false)
        }
        LoweredLogicStatement::Conditional {
            then_branch,
            else_branch,
            ..
        } => {
            block_references_global_assignments(then_branch, script_entries)
                || block_references_global_assignments(else_branch, script_entries)
        }
        LoweredLogicStatement::With { body, .. }
        | LoweredLogicStatement::Repeat { body, .. }
        | LoweredLogicStatement::While { body, .. }
        | LoweredLogicStatement::For { body, .. } => {
            block_references_global_assignments(body, script_entries)
        }
        LoweredLogicStatement::FunctionCall { name, .. } => script_entries
            .get(name)
            .map(|entry| block_references_global_assignments(&entry.statements, script_entries))
            .unwrap_or(false),
        LoweredLogicStatement::VariableDeclaration { .. }
        | LoweredLogicStatement::Return { .. }
        | LoweredLogicStatement::Raw { .. } => false,
    }
}
