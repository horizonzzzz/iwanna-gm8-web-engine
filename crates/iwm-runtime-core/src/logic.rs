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
    ) -> Result<StepExecutionResult, RuntimeCoreError> {
        let Some(room) = self.current_room.as_ref() else {
            return Err(RuntimeCoreError::NoRooms);
        };

        let step_event_blocks = self.object_event_blocks_by_tag("step");
        let script_entries = self.lowered_script_entries();
        let room_order = self.package.rooms.iter().map(|room| room.id).collect::<Vec<_>>();
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

        let mut player_motion_changed = false;
        let mut player_jump_owned_by_script = false;

        for (index, entries) in dispatches {
            let button_states = host
                .active_buttons()
                .into_iter()
                .collect::<HashMap<_, _>>();
            let room_instances = self
                .current_room
                .as_ref()
                .map(|room| room.instances.clone())
                .ok_or(RuntimeCoreError::NoRooms)?;
            let Some(room) = self.current_room.as_mut() else {
                return Err(RuntimeCoreError::NoRooms);
            };
            let Some(instance) = room.instances.get_mut(index) else {
                continue;
            };
            if !instance.alive {
                continue;
            }
            let is_player = crate::helpers::is_player_instance(instance);
            let motion_before = (instance.x, instance.y, instance.hspeed, instance.vspeed);
            if is_player
                && entries
                    .iter()
                    .any(|entry| statements_reference_jump_queries(&entry.statements))
            {
                player_jump_owned_by_script = true;
            }
            let known_files = sample_known_files(host);

            let eval_context = RuntimeEvalContext {
                current_room_id: room.room_id,
                button_states: &button_states,
                room_instances: &room_instances,
                room_order: &room_order,
                objects: &self.package.objects,
                known_files: &known_files,
            };

            for entry in &entries {
                for statement in &entry.statements {
                    apply_runtime_statement(
                        statement,
                        instance,
                        &script_entries,
                        &mut self.globals,
                        &mut self.pending_room_transition,
                        &mut self.pending_room_reset,
                        host,
                        &mut self.diagnostics,
                        Some(&eval_context),
                    );
                    if self.pending_room_reset || self.pending_room_transition.is_some() {
                        return Ok(StepExecutionResult {
                            interrupted: true,
                            player_motion_changed,
                            player_jump_owned_by_script,
                        });
                    }
                }
            }

            if is_player && (instance.x, instance.y, instance.hspeed, instance.vspeed) != motion_before {
                player_motion_changed = true;
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

    pub(crate) fn apply_statement_to_globals(&mut self, statement: &LoweredLogicStatement) {
        match statement {
            LoweredLogicStatement::Assignment { target, value } => {
                if let Some(key) = assignable_key(target, None) {
                    if let Some(value) = evaluate_expr(value, None, &self.globals, None) {
                        self.globals.insert(key, value);
                    }
                }
            }
            LoweredLogicStatement::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition_value = evaluate_expr(condition, None, &self.globals, None);
                let branch = if is_truthy(condition_value) { then_branch } else { else_branch };
                for nested in branch {
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
            LoweredLogicStatement::VariableDeclaration { .. }
            | LoweredLogicStatement::Return { .. }
            | LoweredLogicStatement::FunctionCall { .. }
            | LoweredLogicStatement::Raw { .. } => {}
        }
    }

    pub(crate) fn apply_statement_to_instance(
        &mut self,
        statement: &LoweredLogicStatement,
        instance: &mut RuntimeInstance,
    ) {
        match statement {
            LoweredLogicStatement::Assignment { target, value } => {
                if let Some(key) = assignable_key(target, Some(instance)) {
                    if let Some(value) = evaluate_expr(value, Some(instance), &self.globals, None) {
                        assign_instance_or_global(key, value, instance, &mut self.globals);
                    }
                }
            }
            LoweredLogicStatement::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition_value = evaluate_expr(condition, Some(instance), &self.globals, None);
                let branch = if is_truthy(condition_value) { then_branch } else { else_branch };
                for nested in branch {
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
            LoweredLogicStatement::VariableDeclaration { .. }
            | LoweredLogicStatement::Return { .. }
            | LoweredLogicStatement::FunctionCall { .. }
            | LoweredLogicStatement::Raw { .. } => {}
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
        LoweredLogicStatement::Conditional { condition, then_branch, else_branch } => {
            let condition_value = evaluate_expr(condition, Some(instance), globals, eval_context);
            let branch = if is_truthy(condition_value) { then_branch } else { else_branch };
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
    statements
        .iter()
        .any(statement_references_jump_queries)
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
                "keyboard_check" | "keyboard_check_direct" | "keyboard_check_pressed" | "keyboard_check_released"
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
        LoweredLogicStatement::VariableDeclaration { .. }
        | LoweredLogicStatement::Raw { .. } => false,
    }
}

fn expr_references_jump_queries(expr: &LoweredLogicExpr) -> bool {
    match expr {
        LoweredLogicExpr::Call { name, args } => {
            (matches!(
                name.as_str(),
                "keyboard_check" | "keyboard_check_direct" | "keyboard_check_pressed" | "keyboard_check_released"
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
    if key.starts_with("global.") {
        globals.insert(key, value);
        return;
    }

    match key.as_str() {
        "x" => assign_number_field(value, &mut instance.x, &mut instance.vars, key),
        "y" => assign_number_field(value, &mut instance.y, &mut instance.vars, key),
        "previous_x" => assign_number_field(value, &mut instance.previous_x, &mut instance.vars, key),
        "previous_y" => assign_number_field(value, &mut instance.previous_y, &mut instance.vars, key),
        "hspeed" => assign_number_field(value, &mut instance.hspeed, &mut instance.vars, key),
        "vspeed" => assign_number_field(value, &mut instance.vspeed, &mut instance.vars, key),
        _ => {
            instance.vars.insert(key, value);
        }
    }
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

pub(crate) fn assignable_key(expr: &LoweredLogicExpr, instance: Option<&RuntimeInstance>) -> Option<String> {
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

fn expr_key_fragment(expr: &LoweredLogicExpr, instance: Option<&RuntimeInstance>) -> Option<String> {
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
            "file_exists" => evaluate_file_exists(args, instance, globals, eval_context),
            "keyboard_check" | "keyboard_check_direct" | "keyboard_check_pressed" | "keyboard_check_released" => {
                evaluate_keyboard_query(name, args, instance, globals, eval_context)
            }
            "place_meeting" => evaluate_place_query(args, instance, globals, eval_context, true),
            "place_free" => evaluate_place_query(args, instance, globals, eval_context, false),
            _ => None,
        },
        LoweredLogicExpr::MemberAccess { target, member } => {
            let base = assignable_key(target, instance)?;
            let key = format!("{base}.{member}");
            globals.get(&key).cloned().or_else(|| {
                instance.and_then(|instance| instance.vars.get(&key).cloned())
            })
        }
        LoweredLogicExpr::IndexAccess { target, index } => {
            let base = assignable_key(target, instance)?;
            let suffix = expr_key_fragment(index, instance)?;
            let key = format!("{base}[{suffix}]");
            globals.get(&key).cloned().or_else(|| {
                instance.and_then(|instance| instance.vars.get(&key).cloned())
            })
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
        LoweredLogicExpr::LiteralText(text) => text.chars().next().map(|ch| RuntimeValue::Number(ch as u32 as f64)),
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
    let target_object_ids = context
        .objects
        .iter()
        .filter(|object| object.name.eq_ignore_ascii_case(object_name))
        .map(|object| object.id)
        .collect::<Vec<_>>();
    let targets = context
        .room_instances
        .iter()
        .filter(|candidate| candidate.alive && target_object_ids.contains(&candidate.object_id))
        .cloned()
        .collect::<Vec<_>>();
    let collides = !targets.is_empty()
        && crate::helpers::collides_at(instance, x as f64, y as f64, &targets, Some(instance.runtime_id));
    Some(RuntimeValue::Bool(if want_meeting { collides } else { !collides }))
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
