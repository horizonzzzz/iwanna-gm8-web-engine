use std::collections::HashMap;

use iwm_runtime_host::RuntimeHost;
use iwm_runtime_model::RoomDefinition;

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
    ) -> Result<(bool, bool), RuntimeCoreError> {
        let Some(room) = self.current_room.as_ref() else {
            return Err(RuntimeCoreError::NoRooms);
        };

        let step_event_blocks = self.object_event_blocks_by_tag("step");
        let script_entries = self.lowered_script_entries();
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
            let is_player = crate::helpers::is_player_instance(instance);
            let motion_before = (instance.x, instance.y, instance.hspeed, instance.vspeed);

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
                    );
                    if self.pending_room_reset || self.pending_room_transition.is_some() {
                        return Ok((true, player_motion_changed));
                    }
                }
            }

            if is_player && (instance.x, instance.y, instance.hspeed, instance.vspeed) != motion_before {
                player_motion_changed = true;
            }
        }

        Ok((false, player_motion_changed))
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
                    if let Some(value) = evaluate_expr(value, None, &self.globals) {
                        self.globals.insert(key, value);
                    }
                }
            }
            LoweredLogicStatement::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition_value = evaluate_expr(condition, None, &self.globals);
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
                    if let Some(value) = evaluate_expr(value, Some(instance), &self.globals) {
                        assign_instance_or_global(key, value, instance, &mut self.globals);
                    }
                }
            }
            LoweredLogicStatement::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition_value = evaluate_expr(condition, Some(instance), &self.globals);
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
) {
    match statement {
        LoweredLogicStatement::Assignment { target, value } => {
            if let Some(key) = assignable_key(target, Some(instance)) {
                if let Some(value) = evaluate_expr(value, Some(instance), globals) {
                    assign_instance_or_global(key, value, instance, globals);
                }
            }
        }
        LoweredLogicStatement::Conditional { condition, then_branch, else_branch } => {
            let condition_value = evaluate_expr(condition, Some(instance), globals);
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
                );
            }
        }
        LoweredLogicStatement::VariableDeclaration { .. } => {}
        LoweredLogicStatement::Return { .. } => {}
        LoweredLogicStatement::FunctionCall { name, args } => match name.as_str() {
            "room_goto" => {
                if let Some(room_id) = args
                    .first()
                    .and_then(|arg| evaluate_expr(arg, Some(instance), globals))
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
    field: &mut i32,
    vars: &mut HashMap<String, RuntimeValue>,
    key: String,
) {
    if let Some(number) = as_number(&value) {
        *field = number.round() as i32;
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
        _ => evaluate_expr(expr, instance, &HashMap::new()).map(|value| match value {
            RuntimeValue::Number(number) if number.fract() == 0.0 => format!("{}", number as i64),
            RuntimeValue::Number(number) => number.to_string(),
            RuntimeValue::Bool(flag) => flag.to_string(),
            RuntimeValue::Text(text) => text,
        }),
    }
}

fn evaluate_expr(
    expr: &LoweredLogicExpr,
    instance: Option<&RuntimeInstance>,
    globals: &HashMap<String, RuntimeValue>,
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

            globals
                .get(name)
                .cloned()
                .or_else(|| parse_runtime_value(name))
        }
        LoweredLogicExpr::LiteralNumber(number) => Some(RuntimeValue::Number(*number)),
        LoweredLogicExpr::LiteralBool(flag) => Some(RuntimeValue::Bool(*flag)),
        LoweredLogicExpr::LiteralText(text) => Some(RuntimeValue::Text(text.clone())),
        LoweredLogicExpr::UnaryExpr { op, child } => {
            let value = evaluate_expr(child, instance, globals)?;
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
                .and_then(|arg| evaluate_expr(arg, instance, globals)),
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
            let left = evaluate_expr(left, instance, globals)?;
            let right = evaluate_expr(right, instance, globals)?;
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
        "!=" => Some(RuntimeValue::Bool(left != right)),
        ">=" => Some(RuntimeValue::Bool(as_number(left)? >= as_number(right)?)),
        "<=" => Some(RuntimeValue::Bool(as_number(left)? <= as_number(right)?)),
        ">" => Some(RuntimeValue::Bool(as_number(left)? > as_number(right)?)),
        "<" => Some(RuntimeValue::Bool(as_number(left)? < as_number(right)?)),
        _ => None,
    }
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
