use std::collections::{HashMap, HashSet};

use iwm_runtime_host::{RuntimeButton, RuntimeHost};
use iwm_runtime_model::{ObjectDefinition, RoomDefinition};

use crate::helpers::{as_number, parse_room_id, parse_runtime_value, record_host_diagnostic};
use crate::{
    LoweredLogicEntry, LoweredLogicExpr, LoweredLogicStatement, RuntimeCore, RuntimeCoreError,
    RuntimeInstance, RuntimeRoomState, RuntimeValue,
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

    pub(crate) fn execute_lowered_step_events<H: RuntimeHost>(
        &mut self,
        host: &mut H,
    ) -> Result<StepExecutionResult, RuntimeCoreError> {
        let step_event_blocks = self.object_event_blocks_by_tag("step");
        let script_entries = self.lowered_script_entries();
        let room_order = self.runtime_room_order();
        let button_states = host.active_buttons().into_iter().collect::<HashMap<_, _>>();
        let known_files = sample_known_files(host);
        let (current_room_id, dispatches) = {
            let Some(room) = self.current_room.as_ref() else {
                return Err(RuntimeCoreError::NoRooms);
            };
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
            (room.room_id, dispatches)
        };

        let mut player_motion_changed = false;
        let mut player_jump_owned_by_script = false;
        let mut instance_updates = Vec::new();

        for (index, entries) in dispatches {
            let Some(mut instance) = self
                .current_room
                .as_ref()
                .and_then(|room| room.instances.get(index).cloned())
            else {
                continue;
            };
            if !instance.alive {
                continue;
            }
            let is_player = crate::helpers::is_player_instance(&instance);
            let motion_before = (instance.x, instance.y, instance.hspeed, instance.vspeed);
            if is_player
                && entries
                    .iter()
                    .any(|entry| statements_reference_jump_queries(&entry.statements))
            {
                player_jump_owned_by_script = true;
            }

            {
                let Some(room) = self.current_room.as_ref() else {
                    return Err(RuntimeCoreError::NoRooms);
                };
                let eval_context = RuntimeEvalContext {
                    current_room_id,
                    button_states: &button_states,
                    room_instances: &room.instances,
                    room_order: &room_order,
                    objects: &self.package.objects,
                    known_files: &known_files,
                    other_instance: None,
                };

                for entry in &entries {
                    for statement in &entry.statements {
                        apply_runtime_statement(
                            statement,
                            &mut instance,
                            &script_entries,
                            &mut self.globals,
                            &mut self.pending_room_transition,
                            &mut self.pending_room_reset,
                            host,
                            &mut self.diagnostics,
                            Some(&eval_context),
                        );
                        if self.pending_room_reset || self.pending_room_transition.is_some() {
                            instance_updates.push((index, instance));
                            if let Some(room) = self.current_room.as_mut() {
                                for (update_index, updated_instance) in instance_updates {
                                    if let Some(slot) = room.instances.get_mut(update_index) {
                                        *slot = updated_instance;
                                    }
                                }
                            }
                            return Ok(StepExecutionResult {
                                interrupted: true,
                                player_motion_changed,
                                player_jump_owned_by_script,
                            });
                        }
                    }
                }
            }

            if is_player
                && (instance.x, instance.y, instance.hspeed, instance.vspeed) != motion_before
            {
                player_motion_changed = true;
            }
            instance_updates.push((index, instance));
        }

        if let Some(room) = self.current_room.as_mut() {
            for (index, instance) in instance_updates {
                if let Some(slot) = room.instances.get_mut(index) {
                    *slot = instance;
                }
            }
        }

        Ok(StepExecutionResult {
            interrupted: false,
            player_motion_changed,
            player_jump_owned_by_script,
        })
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

        let script_entries = self.lowered_script_entries();
        for statement in &entry.statements {
            apply_statement_to_globals_map(statement, &script_entries, &mut self.globals);
        }
    }

    pub(crate) fn lowered_logic_entry(&self, block_id: &str) -> Option<&LoweredLogicEntry> {
        let index = self.lowered_logic_index.get(block_id)?;
        self.package
            .lowered_logic
            .as_ref()
            .and_then(|lowered_logic| lowered_logic.entries.get(*index))
    }

    pub(crate) fn lowered_script_entries(&self) -> HashMap<String, LoweredLogicEntry> {
        self.package
            .scripts
            .blocks
            .iter()
            .filter(|block| block.kind == "script")
            .filter_map(|block| {
                self.lowered_logic_entry(&block.id)
                    .cloned()
                    .map(|entry| (block.name.clone(), entry))
            })
            .collect()
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

    pub(crate) fn runtime_room_order(&self) -> Vec<usize> {
        if self.package.manifest.room_order.is_empty() {
            self.package.rooms.iter().map(|room| room.id).collect()
        } else {
            self.package.manifest.room_order.clone()
        }
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
                if let Some(key) = assignable_key(target, Some(&instance_snapshot)) {
                    let button_states = HashMap::new();
                    let known_files = HashSet::new();
                    let eval_context = RuntimeEvalContext {
                        current_room_id: room_state.room_id,
                        button_states: &button_states,
                        room_instances: &room_state.instances,
                        room_order: &[],
                        objects: &self.package.objects,
                        known_files: &known_files,
                        other_instance: None,
                    };
                    if let Some(value) = evaluate_expr(
                        value,
                        Some(&instance_snapshot),
                        &self.globals,
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
                    room_order: &[],
                    objects: &self.package.objects,
                    known_files: &known_files,
                    other_instance: None,
                };
                let condition_value = evaluate_expr(
                    condition,
                    Some(&instance_snapshot),
                    &self.globals,
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
            room_order: &[],
            objects: &self.package.objects,
            known_files: &known_files,
            other_instance: None,
        };
        let x = args
            .first()
            .and_then(|arg| evaluate_expr(arg, source_instance, &self.globals, Some(&eval_context)))
            .and_then(|value| as_number(&value))
            .unwrap_or(0.0);
        let y = args
            .get(1)
            .and_then(|arg| evaluate_expr(arg, source_instance, &self.globals, Some(&eval_context)))
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

pub(crate) fn apply_runtime_statement<H: RuntimeHost>(
    statement: &LoweredLogicStatement,
    instance: &mut RuntimeInstance,
    script_entries: &HashMap<String, LoweredLogicEntry>,
    globals: &mut HashMap<String, RuntimeValue>,
    pending_room_transition: &mut Option<usize>,
    pending_room_reset: &mut bool,
    host: &mut H,
    diagnostics: &mut Vec<iwm_runtime_host::RuntimeDiagnostic>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) {
    match statement {
        LoweredLogicStatement::Assignment { target, value } => {
            if let Some(key) = assignable_key(target, Some(instance)) {
                if let Some(value) = evaluate_expr(value, Some(instance), globals, eval_context) {
                    assign_instance_or_global(key, value, instance, globals);
                }
            }
        }
        LoweredLogicStatement::Conditional {
            condition,
            then_branch,
            else_branch,
        } => {
            let condition_value = evaluate_expr(condition, Some(instance), globals, eval_context);
            let branch = if is_truthy(condition_value) {
                then_branch
            } else {
                else_branch
            };
            for nested in branch {
                apply_runtime_statement(
                    nested,
                    instance,
                    script_entries,
                    globals,
                    pending_room_transition,
                    pending_room_reset,
                    host,
                    diagnostics,
                    eval_context,
                );
            }
        }
        LoweredLogicStatement::VariableDeclaration { .. } => {}
        LoweredLogicStatement::Return { .. } => {}
        LoweredLogicStatement::FunctionCall { name, args } => match name.as_str() {
            "room_goto" => {
                if let Some(room_id) = args
                    .first()
                    .and_then(|arg| evaluate_expr(arg, Some(instance), globals, eval_context))
                    .and_then(|value| runtime_value_to_room_id(&value))
                {
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
            "room_goto_next" => {
                if let Some(room_id) = next_room_id(instance, globals, eval_context) {
                    *pending_room_transition = Some(room_id);
                } else {
                    record_host_diagnostic(
                        host,
                        diagnostics,
                        iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
                        "runtime-step-room-goto-next-unresolved",
                        format!(
                            "could not resolve room_goto_next target for {}",
                            instance.object_name
                        ),
                    );
                }
            }
            "game_restart" => {
                *pending_room_reset = true;
            }
            _ => {
                if let Some(entry) = script_entries.get(name) {
                    for nested in &entry.statements {
                        apply_runtime_statement(
                            nested,
                            instance,
                            script_entries,
                            globals,
                            pending_room_transition,
                            pending_room_reset,
                            host,
                            diagnostics,
                            eval_context,
                        );
                        if *pending_room_reset || pending_room_transition.is_some() {
                            break;
                        }
                    }
                }
            }
        },
        _ => {}
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StepExecutionResult {
    pub interrupted: bool,
    pub player_motion_changed: bool,
    pub player_jump_owned_by_script: bool,
}

fn statements_reference_jump_queries(statements: &[LoweredLogicStatement]) -> bool {
    statements.iter().any(statement_references_jump_queries)
}

fn statement_references_jump_queries(statement: &LoweredLogicStatement) -> bool {
    match statement {
        LoweredLogicStatement::Assignment { value, .. } => expr_references_jump_queries(value),
        LoweredLogicStatement::Conditional {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_references_jump_queries(condition)
                || statements_reference_jump_queries(then_branch)
                || statements_reference_jump_queries(else_branch)
        }
        LoweredLogicStatement::FunctionCall { name, args } => {
            matches!(
                name.as_str(),
                "keyboard_check"
                    | "keyboard_check_direct"
                    | "keyboard_check_pressed"
                    | "keyboard_check_released"
            ) && args.iter().any(expr_is_global_jumpbutton)
                || args.iter().any(expr_references_jump_queries)
        }
        LoweredLogicStatement::With { body, .. }
        | LoweredLogicStatement::Repeat { body, .. }
        | LoweredLogicStatement::While { body, .. }
        | LoweredLogicStatement::For { body, .. } => statements_reference_jump_queries(body),
        LoweredLogicStatement::Return { value } => value
            .as_ref()
            .map(expr_references_jump_queries)
            .unwrap_or(false),
        LoweredLogicStatement::VariableDeclaration { .. } | LoweredLogicStatement::Raw { .. } => {
            false
        }
    }
}

fn expr_references_jump_queries(expr: &LoweredLogicExpr) -> bool {
    match expr {
        LoweredLogicExpr::Call { name, args } => {
            (matches!(
                name.as_str(),
                "keyboard_check"
                    | "keyboard_check_direct"
                    | "keyboard_check_pressed"
                    | "keyboard_check_released"
            ) && args.iter().any(expr_is_global_jumpbutton))
                || args.iter().any(expr_references_jump_queries)
        }
        LoweredLogicExpr::UnaryExpr { child, .. } => expr_references_jump_queries(child),
        LoweredLogicExpr::BinaryExpr { left, right, .. } => {
            expr_references_jump_queries(left) || expr_references_jump_queries(right)
        }
        LoweredLogicExpr::MemberAccess { target, .. } => expr_references_jump_queries(target),
        LoweredLogicExpr::IndexAccess { target, index } => {
            expr_references_jump_queries(target) || expr_references_jump_queries(index)
        }
        LoweredLogicExpr::Identifier(_)
        | LoweredLogicExpr::LiteralNumber(_)
        | LoweredLogicExpr::LiteralBool(_)
        | LoweredLogicExpr::LiteralText(_)
        | LoweredLogicExpr::Raw { .. } => false,
    }
}

fn expr_is_global_jumpbutton(expr: &LoweredLogicExpr) -> bool {
    matches!(
        expr,
        LoweredLogicExpr::MemberAccess { target, member }
            if member == "jumpbutton"
                && matches!(target.as_ref(), LoweredLogicExpr::Identifier(name) if name == "global")
    )
}

fn assign_instance_or_global(
    key: String,
    value: RuntimeValue,
    instance: &mut RuntimeInstance,
    globals: &mut HashMap<String, RuntimeValue>,
) {
    if key.starts_with("global.") || is_view_variable_key(&key) {
        globals.insert(key, value);
        return;
    }

    match key.as_str() {
        "x" => assign_number_field(value, &mut instance.x, &mut instance.vars, key),
        "y" => assign_number_field(value, &mut instance.y, &mut instance.vars, key),
        "previous_x" => {
            assign_number_field(value, &mut instance.previous_x, &mut instance.vars, key)
        }
        "previous_y" => {
            assign_number_field(value, &mut instance.previous_y, &mut instance.vars, key)
        }
        "hspeed" => assign_number_field(value, &mut instance.hspeed, &mut instance.vars, key),
        "vspeed" => assign_number_field(value, &mut instance.vspeed, &mut instance.vars, key),
        _ => {
            instance.vars.insert(key, value);
        }
    }
}

fn is_view_variable_key(key: &str) -> bool {
    matches!(
        key,
        "view_xview"
            | "view_yview"
            | "view_wview"
            | "view_hview"
            | "view_xview[0]"
            | "view_yview[0]"
            | "view_wview[0]"
            | "view_hview[0]"
    )
}

fn assign_number_field(
    value: RuntimeValue,
    field: &mut f64,
    vars: &mut HashMap<String, RuntimeValue>,
    key: String,
) {
    if let Some(number) = as_number(&value) {
        *field = number;
    } else {
        vars.insert(key, value);
    }
}

fn is_truthy(value: Option<RuntimeValue>) -> bool {
    match value {
        Some(RuntimeValue::Bool(b)) => b,
        Some(RuntimeValue::Number(n)) => n != 0.0,
        Some(RuntimeValue::Text(s)) => !s.is_empty(),
        None => false,
    }
}

pub(crate) fn assignable_key(
    expr: &LoweredLogicExpr,
    instance: Option<&RuntimeInstance>,
) -> Option<String> {
    match expr {
        LoweredLogicExpr::Identifier(name) => Some(name.clone()),
        LoweredLogicExpr::MemberAccess { target, member } => {
            let base = assignable_key(target, instance)?;
            Some(format!("{base}.{member}"))
        }
        LoweredLogicExpr::IndexAccess { target, index } => {
            let base = assignable_key(target, instance)?;
            let suffix = expr_key_fragment(index, instance)?;
            Some(format!("{base}[{suffix}]"))
        }
        _ => None,
    }
}

fn expr_key_fragment(
    expr: &LoweredLogicExpr,
    instance: Option<&RuntimeInstance>,
) -> Option<String> {
    match expr {
        LoweredLogicExpr::Identifier(name) => Some(name.clone()),
        LoweredLogicExpr::LiteralNumber(number) => Some(if number.fract() == 0.0 {
            format!("{}", *number as i64)
        } else {
            number.to_string()
        }),
        LoweredLogicExpr::LiteralBool(flag) => Some(flag.to_string()),
        LoweredLogicExpr::LiteralText(text) => Some(text.clone()),
        _ => evaluate_expr(expr, instance, &HashMap::new(), None).map(|value| match value {
            RuntimeValue::Number(number) if number.fract() == 0.0 => format!("{}", number as i64),
            RuntimeValue::Number(number) => number.to_string(),
            RuntimeValue::Bool(flag) => flag.to_string(),
            RuntimeValue::Text(text) => text,
        }),
    }
}

pub(crate) struct RuntimeEvalContext<'a> {
    pub current_room_id: usize,
    pub button_states: &'a HashMap<RuntimeButton, iwm_runtime_host::ButtonState>,
    pub room_instances: &'a [RuntimeInstance],
    pub room_order: &'a [usize],
    pub objects: &'a [ObjectDefinition],
    pub known_files: &'a HashSet<String>,
    pub other_instance: Option<&'a RuntimeInstance>,
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
            if let Some(key) = assignable_key(target, None) {
                if let Some(value) = evaluate_expr(value, None, globals, None) {
                    globals.insert(key, value);
                }
            }
        }
        LoweredLogicStatement::Conditional {
            condition,
            then_branch,
            else_branch,
        } => {
            let condition_value = evaluate_expr(condition, None, globals, None);
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
        LoweredLogicStatement::Assignment { target, .. } => assignable_key(target, None)
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

fn evaluate_expr(
    expr: &LoweredLogicExpr,
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    match expr {
        LoweredLogicExpr::Identifier(name) => {
            if let Some(instance) = instance {
                if let Some(value) = instance.vars.get(name) {
                    return Some(value.clone());
                }
                match name.as_str() {
                    "x" => return Some(RuntimeValue::Number(instance.x as f64)),
                    "y" => return Some(RuntimeValue::Number(instance.y as f64)),
                    "hspeed" => return Some(RuntimeValue::Number(instance.hspeed as f64)),
                    "vspeed" => return Some(RuntimeValue::Number(instance.vspeed as f64)),
                    _ => {}
                }
            }

            if let Some(key_code) = gm_key_code(name) {
                return Some(RuntimeValue::Number(key_code as f64));
            }

            globals
                .get(name)
                .cloned()
                .or_else(|| parse_runtime_value(name))
        }
        LoweredLogicExpr::LiteralNumber(number) => Some(RuntimeValue::Number(*number)),
        LoweredLogicExpr::LiteralBool(flag) => Some(RuntimeValue::Bool(*flag)),
        LoweredLogicExpr::LiteralText(text) => Some(RuntimeValue::Text(text.clone())),
        LoweredLogicExpr::UnaryExpr { op, child } => {
            let value = evaluate_expr(child, instance, globals, eval_context)?;
            match op.as_str() {
                "-" => Some(RuntimeValue::Number(-as_number(&value)?)),
                "+" => Some(RuntimeValue::Number(as_number(&value)?)),
                "!" => Some(RuntimeValue::Bool(!is_truthy(Some(value)))),
                _ => None,
            }
        }
        LoweredLogicExpr::Call { name, args } => match name.as_str() {
            "room_goto" => args
                .first()
                .and_then(|arg| evaluate_expr(arg, instance, globals, eval_context)),
            "ord" => evaluate_ord_call(args),
            "floor" => args
                .first()
                .and_then(|arg| evaluate_expr(arg, instance, globals, eval_context))
                .and_then(|value| as_number(&value))
                .map(|value| RuntimeValue::Number(value.floor())),
            "file_exists" => evaluate_file_exists(args, instance, globals, eval_context),
            "instance_exists" => evaluate_instance_exists(args, eval_context),
            "keyboard_check"
            | "keyboard_check_direct"
            | "keyboard_check_pressed"
            | "keyboard_check_released" => {
                evaluate_keyboard_query(name, args, instance, globals, eval_context)
            }
            "place_meeting" => evaluate_place_query(args, instance, globals, eval_context, true),
            "place_free" => evaluate_place_query(args, instance, globals, eval_context, false),
            _ => None,
        },
        LoweredLogicExpr::MemberAccess { target, member } => {
            if matches!(target.as_ref(), LoweredLogicExpr::Identifier(name) if name == "other") {
                let other = eval_context?.other_instance?;
                return match member.as_str() {
                    "x" => Some(RuntimeValue::Number(other.x)),
                    "y" => Some(RuntimeValue::Number(other.y)),
                    "hspeed" => Some(RuntimeValue::Number(other.hspeed)),
                    "vspeed" => Some(RuntimeValue::Number(other.vspeed)),
                    _ => other.vars.get(member).cloned(),
                };
            }
            if let LoweredLogicExpr::Identifier(object_name) = target.as_ref() {
                if object_name != "global" {
                    if let Some(context) = eval_context {
                        if let Some(target_instance) =
                            context.room_instances.iter().find(|candidate| {
                                candidate.alive
                                    && candidate.object_name.eq_ignore_ascii_case(object_name)
                            })
                        {
                            return match member.as_str() {
                                "x" => Some(RuntimeValue::Number(target_instance.x)),
                                "y" => Some(RuntimeValue::Number(target_instance.y)),
                                "hspeed" => Some(RuntimeValue::Number(target_instance.hspeed)),
                                "vspeed" => Some(RuntimeValue::Number(target_instance.vspeed)),
                                _ => target_instance.vars.get(member).cloned(),
                            };
                        }
                    }
                }
            }
            let base = assignable_key(target, instance)?;
            let key = format!("{base}.{member}");
            globals
                .get(&key)
                .cloned()
                .or_else(|| instance.and_then(|instance| instance.vars.get(&key).cloned()))
        }
        LoweredLogicExpr::IndexAccess { target, index } => {
            let base = assignable_key(target, instance)?;
            let suffix = expr_key_fragment(index, instance)?;
            let key = format!("{base}[{suffix}]");
            globals
                .get(&key)
                .cloned()
                .or_else(|| instance.and_then(|instance| instance.vars.get(&key).cloned()))
        }
        LoweredLogicExpr::BinaryExpr { op, left, right } => {
            if op == "&&" {
                let left = evaluate_expr(left, instance, globals, eval_context)?;
                if !is_truthy(Some(left)) {
                    return Some(RuntimeValue::Bool(false));
                }
                let right = evaluate_expr(right, instance, globals, eval_context)?;
                return Some(RuntimeValue::Bool(is_truthy(Some(right))));
            }

            if op == "||" {
                let left = evaluate_expr(left, instance, globals, eval_context)?;
                if is_truthy(Some(left)) {
                    return Some(RuntimeValue::Bool(true));
                }
                let right = evaluate_expr(right, instance, globals, eval_context)?;
                return Some(RuntimeValue::Bool(is_truthy(Some(right))));
            }

            let left = evaluate_expr(left, instance, globals, eval_context)?;
            let right = evaluate_expr(right, instance, globals, eval_context)?;
            eval_binary_expr(op, &left, &right)
        }
        LoweredLogicExpr::Raw { source } => parse_runtime_value(source),
    }
}

fn eval_binary_expr(op: &str, left: &RuntimeValue, right: &RuntimeValue) -> Option<RuntimeValue> {
    match op {
        "+" => Some(RuntimeValue::Number(as_number(left)? + as_number(right)?)),
        "-" => Some(RuntimeValue::Number(as_number(left)? - as_number(right)?)),
        "*" => Some(RuntimeValue::Number(as_number(left)? * as_number(right)?)),
        "/" => Some(RuntimeValue::Number(as_number(left)? / as_number(right)?)),
        "==" => Some(RuntimeValue::Bool(left == right)),
        "=" => Some(RuntimeValue::Bool(left == right)),
        "!=" => Some(RuntimeValue::Bool(left != right)),
        ">=" => Some(RuntimeValue::Bool(as_number(left)? >= as_number(right)?)),
        "<=" => Some(RuntimeValue::Bool(as_number(left)? <= as_number(right)?)),
        ">" => Some(RuntimeValue::Bool(as_number(left)? > as_number(right)?)),
        "<" => Some(RuntimeValue::Bool(as_number(left)? < as_number(right)?)),
        _ => None,
    }
}

fn gm_key_code(name: &str) -> Option<u16> {
    match name.to_ascii_lowercase().as_str() {
        "vk_left" => Some(0x25),
        "vk_up" => Some(0x26),
        "vk_right" => Some(0x27),
        "vk_down" => Some(0x28),
        "vk_shift" => Some(0x10),
        "vk_space" => Some(0x20),
        "vk_enter" => Some(0x0D),
        "vk_escape" => Some(0x1B),
        "vk_control" => Some(0x11),
        "vk_alt" => Some(0x12),
        _ => None,
    }
}

fn evaluate_ord_call(args: &[LoweredLogicExpr]) -> Option<RuntimeValue> {
    let first = args.first()?;
    match first {
        LoweredLogicExpr::LiteralText(text) => text
            .chars()
            .next()
            .map(|ch| RuntimeValue::Number(ch as u32 as f64)),
        _ => None,
    }
}

fn evaluate_keyboard_query(
    name: &str,
    args: &[LoweredLogicExpr],
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    let context = eval_context?;
    let key_code = args
        .first()
        .and_then(|arg| evaluate_expr(arg, instance, globals, eval_context))
        .and_then(|value| as_number(&value))
        .map(|value| value.round() as u16)?;
    let state = context
        .button_states
        .get(&RuntimeButton::Keyboard(key_code))
        .copied()
        .unwrap_or_default();
    let result = match name {
        "keyboard_check" | "keyboard_check_direct" => state.pressed,
        "keyboard_check_pressed" => state.just_pressed,
        "keyboard_check_released" => state.just_released,
        _ => return None,
    };
    Some(RuntimeValue::Bool(result))
}

fn evaluate_place_query(
    args: &[LoweredLogicExpr],
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    want_meeting: bool,
) -> Option<RuntimeValue> {
    let context = eval_context?;
    let instance = instance?;
    let x = args
        .first()
        .and_then(|arg| evaluate_expr(arg, Some(instance), globals, eval_context))
        .and_then(|value| as_number(&value))
        .map(|value| value.round() as i32)?;
    let y = args
        .get(1)
        .and_then(|arg| evaluate_expr(arg, Some(instance), globals, eval_context))
        .and_then(|value| as_number(&value))
        .map(|value| value.round() as i32)?;
    let object_name = args.get(2).and_then(|arg| match arg {
        LoweredLogicExpr::Identifier(name) => Some(name.as_str()),
        _ => None,
    })?;
    let root_object_ids = context
        .objects
        .iter()
        .filter(|object| object.name.eq_ignore_ascii_case(object_name))
        .map(|object| object.id)
        .collect::<Vec<_>>();
    let target_object_ids = context
        .objects
        .iter()
        .filter(|object| {
            root_object_ids.iter().any(|root_id| {
                object_matches_or_inherits_from(context.objects, object.id, *root_id)
            })
        })
        .map(|object| object.id)
        .collect::<Vec<_>>();
    let targets = context
        .room_instances
        .iter()
        .filter(|candidate| candidate.alive && target_object_ids.contains(&candidate.object_id))
        .cloned()
        .collect::<Vec<_>>();
    let collides = !targets.is_empty()
        && crate::helpers::collides_at(
            instance,
            x as f64,
            y as f64,
            &targets,
            Some(instance.runtime_id),
        );
    Some(RuntimeValue::Bool(if want_meeting {
        collides
    } else {
        !collides
    }))
}

fn object_matches_or_inherits_from(
    objects: &[ObjectDefinition],
    object_id: usize,
    wanted_object_id: usize,
) -> bool {
    let mut current_id = Some(object_id);
    while let Some(id) = current_id {
        if id == wanted_object_id {
            return true;
        }
        current_id = objects
            .iter()
            .find(|object| object.id == id)
            .and_then(|object| usize::try_from(object.parent_index).ok());
    }
    false
}

fn evaluate_file_exists(
    args: &[LoweredLogicExpr],
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    let context = eval_context?;
    let path = args
        .first()
        .and_then(|arg| evaluate_expr(arg, instance, globals, eval_context))
        .and_then(|value| match value {
            RuntimeValue::Text(text) => Some(text),
            _ => None,
        })?;
    Some(RuntimeValue::Bool(context.known_files.contains(&path)))
}

fn evaluate_instance_exists(
    args: &[LoweredLogicExpr],
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<RuntimeValue> {
    let context = eval_context?;
    let object_name = args.first().and_then(|arg| match arg {
        LoweredLogicExpr::Identifier(name) => Some(name.as_str()),
        _ => None,
    })?;
    let exists = context
        .room_instances
        .iter()
        .any(|instance| instance.alive && instance.object_name.eq_ignore_ascii_case(object_name));
    Some(RuntimeValue::Bool(exists))
}

pub(crate) fn sample_known_files<H: RuntimeHost>(host: &H) -> HashSet<String> {
    let mut files = HashSet::new();
    for candidate in ["temp", "DeathTime", "save1", "save2", "save3"] {
        if host.read(std::path::Path::new(candidate)).is_ok() {
            files.insert(candidate.to_string());
        }
    }
    files
}

fn runtime_value_to_room_id(value: &RuntimeValue) -> Option<usize> {
    match value {
        RuntimeValue::Number(number) => {
            if number.is_finite() && *number >= 0.0 {
                Some(number.round() as usize)
            } else {
                None
            }
        }
        RuntimeValue::Bool(flag) => Some(if *flag { 1 } else { 0 }),
        RuntimeValue::Text(text) => parse_room_id(text),
    }
}

fn next_room_id(
    _instance: &RuntimeInstance,
    _globals: &HashMap<String, RuntimeValue>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
) -> Option<usize> {
    let context = eval_context?;
    let current_index = context
        .room_order
        .iter()
        .position(|room_id| *room_id == context.current_room_id)?;
    context.room_order.get(current_index + 1).copied()
}
