mod bootstrap;
mod context;
mod eval;
mod statement;

use std::collections::{HashMap, HashSet};

use iwm_runtime_host::RuntimeHost;
use iwm_runtime_model::ObjectDefinition;

use crate::event_dispatch::{object_event_block_ids, RuntimeEventSelector};
use context::RuntimeInstanceCreateRequest;

use crate::{
    LoweredLogicEntry, LoweredLogicExpr, LoweredLogicStatement, RuntimeCore, RuntimeCoreError,
    RuntimeInstance,
};

pub(crate) use bootstrap::apply_view_globals_to_room;
pub(crate) use context::{RuntimeEvalContext, RuntimeExecutionScope, StepExecutionResult};
pub(crate) use eval::sample_known_files;
pub(crate) use statement::{apply_runtime_statement, RuntimeStatementEnvironment};

impl RuntimeCore {
    pub(crate) fn execute_lowered_step_events<H: RuntimeHost>(
        &mut self,
        host: &mut H,
    ) -> Result<StepExecutionResult, RuntimeCoreError> {
        let step_event_blocks = &self.cached_step_event_blocks;
        let script_entries = &self.cached_script_entries;
        let room_order = &self.cached_room_order;
        let destroy_event_entries =
            self.lowered_event_entries_by_selector(RuntimeEventSelector::Destroy);
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
        let mut instance_updates: Vec<(usize, RuntimeInstance)> = Vec::new();
        let mut instance_creates: Vec<RuntimeInstanceCreateRequest> = Vec::new();

        for (index, entries) in dispatches {
            let Some(mut instance) = instance_updates
                .iter()
                .rev()
                .find(|(update_index, _)| *update_index == index)
                .map(|(_, instance)| instance.clone())
                .or_else(|| {
                    self.current_room
                        .as_ref()
                        .and_then(|room| room.instances.get(index).cloned())
                })
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
                    room_order: room_order.as_slice(),
                    known_files: &known_files,
                    other_instance: None,
                    place_target_ids_by_name: &self.place_target_ids_by_name,
                };

                for entry in &entries {
                    let mut scope = RuntimeExecutionScope::default();
                    let mut with_updates = Vec::new();
                    for statement in &entry.statements {
                        let mut statement_env = RuntimeStatementEnvironment {
                            script_entries,
                            globals: &mut self.globals,
                            pending_room_transition: &mut self.pending_room_transition,
                            pending_room_reset: &mut self.pending_room_reset,
                            host: &mut *host,
                            diagnostics: &mut self.diagnostics,
                            room_instance_updates: &mut with_updates,
                            room_instance_creates: &mut instance_creates,
                        };
                        apply_runtime_statement(
                            statement,
                            &mut instance,
                            index,
                            &mut scope,
                            &destroy_event_entries,
                            Some(&eval_context),
                            &mut statement_env,
                        );
                        if self.pending_room_reset || self.pending_room_transition.is_some() {
                            instance_updates.push((index, instance));
                            instance_updates.append(&mut with_updates);
                            if let Some(room) = self.current_room.as_mut() {
                                for (update_index, updated_instance) in instance_updates {
                                    if let Some(slot) = room.instances.get_mut(update_index) {
                                        *slot = updated_instance;
                                    }
                                }
                            }
                            self.apply_runtime_instance_creates(host, &mut instance_creates);
                            return Ok(StepExecutionResult {
                                interrupted: true,
                                player_motion_changed,
                                player_jump_owned_by_script,
                            });
                        }
                    }
                    instance_updates.append(&mut with_updates);
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
        self.apply_runtime_instance_creates(host, &mut instance_creates);

        Ok(StepExecutionResult {
            interrupted: false,
            player_motion_changed,
            player_jump_owned_by_script,
        })
    }

    pub(crate) fn object_event_blocks_by_tag(
        &self,
        event_tag: &str,
    ) -> HashMap<usize, Vec<String>> {
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

    pub(crate) fn lowered_event_entries_by_selector(
        &self,
        selector: RuntimeEventSelector,
    ) -> HashMap<usize, Vec<LoweredLogicEntry>> {
        self.package
            .objects
            .iter()
            .filter_map(|object| {
                let entries = object_event_block_ids(&self.package, object.id, selector.clone())
                    .iter()
                    .filter_map(|block_id| self.lowered_logic_entry(block_id).cloned())
                    .collect::<Vec<_>>();
                if entries.is_empty() {
                    None
                } else {
                    Some((object.id, entries))
                }
            })
            .collect()
    }

    pub(crate) fn apply_runtime_instance_creates<H: RuntimeHost>(
        &mut self,
        host: &mut H,
        creates: &mut Vec<RuntimeInstanceCreateRequest>,
    ) {
        let create_event_entries = self.lowered_event_entries_by_tag_for_runtime("create");
        while let Some(create) = creates.first().cloned() {
            creates.remove(0);
            let Some((runtime_id, current_room_id)) = self
                .current_room
                .as_ref()
                .map(|room| (room.instances.len(), room.room_id))
            else {
                return;
            };
            let Some(mut instance) =
                self.instantiate_runtime_object(create.object_id, runtime_id, create.x, create.y)
            else {
                continue;
            };
            let Some(mut room_instances) = self
                .current_room
                .as_ref()
                .map(|room| room.instances.clone())
            else {
                return;
            };
            room_instances.push(instance.clone());

            if let Some(entries) = create_event_entries.get(&instance.object_id) {
                let script_entries = &self.cached_script_entries;
                let button_states = HashMap::new();
                let known_files = HashSet::new();
                let room_order = self.cached_room_order.clone();
                let eval_context = RuntimeEvalContext {
                    current_room_id,
                    button_states: &button_states,
                    room_instances: &room_instances,
                    room_order: &room_order,
                    known_files: &known_files,
                    other_instance: None,
                    place_target_ids_by_name: &self.place_target_ids_by_name,
                };
                let destroy_event_entries =
                    self.lowered_event_entries_by_selector(RuntimeEventSelector::Destroy);
                let mut room_instance_updates = Vec::new();
                for entry in entries {
                    let mut scope = RuntimeExecutionScope::default();
                    for statement in &entry.statements {
                        let mut statement_env = RuntimeStatementEnvironment {
                            script_entries,
                            globals: &mut self.globals,
                            pending_room_transition: &mut self.pending_room_transition,
                            pending_room_reset: &mut self.pending_room_reset,
                            host: &mut *host,
                            diagnostics: &mut self.diagnostics,
                            room_instance_updates: &mut room_instance_updates,
                            room_instance_creates: &mut *creates,
                        };
                        apply_runtime_statement(
                            statement,
                            &mut instance,
                            runtime_id,
                            &mut scope,
                            &destroy_event_entries,
                            Some(&eval_context),
                            &mut statement_env,
                        );
                        if self.pending_room_reset || self.pending_room_transition.is_some() {
                            break;
                        }
                    }
                    if self.pending_room_reset || self.pending_room_transition.is_some() {
                        break;
                    }
                }
                if let Some(room) = self.current_room.as_mut() {
                    for (update_index, updated_instance) in room_instance_updates {
                        if let Some(slot) = room.instances.get_mut(update_index) {
                            *slot = updated_instance;
                        }
                    }
                }
            }

            if let Some(room) = self.current_room.as_mut() {
                room.instances.push(instance);
            }

            if self.pending_room_reset || self.pending_room_transition.is_some() {
                break;
            }
        }
    }

    fn lowered_event_entries_by_tag_for_runtime(
        &self,
        event_tag: &str,
    ) -> HashMap<usize, Vec<LoweredLogicEntry>> {
        self.package
            .objects
            .iter()
            .filter_map(|object| {
                let entries = object
                    .events
                    .iter()
                    .filter(|event| event.event_tag == event_tag)
                    .filter_map(|event| self.lowered_logic_entry(&event.block_id).cloned())
                    .collect::<Vec<_>>();
                if entries.is_empty() {
                    None
                } else {
                    Some((object.id, entries))
                }
            })
            .collect()
    }

    /// Precomputes, for each lowercased object name, the set of object ids that
    /// match or inherit from an object with that name. `place_meeting` and
    /// `place_free` use this to avoid walking the inheritance chain of every
    /// object on each call.
    pub(crate) fn compute_place_target_ids_by_name(&self) -> HashMap<String, Vec<usize>> {
        let objects = &self.package.objects;
        let mut map: HashMap<String, Vec<usize>> = HashMap::new();
        for name_owner in objects {
            let name = name_owner.name.to_ascii_lowercase();
            if map.contains_key(&name) {
                continue;
            }
            let roots = objects
                .iter()
                .filter(|object| object.name.eq_ignore_ascii_case(&name))
                .map(|object| object.id)
                .collect::<Vec<_>>();
            let targets = objects
                .iter()
                .filter(|object| {
                    roots.iter().any(|root_id| {
                        object_matches_or_inherits_from(objects, object.id, *root_id)
                    })
                })
                .map(|object| object.id)
                .collect::<Vec<_>>();
            map.insert(name, targets);
        }
        map
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
    pub(crate) fn runtime_room_order(&self) -> Vec<usize> {
        if self.package.manifest.room_order.is_empty() {
            self.package.rooms.iter().map(|room| room.id).collect()
        } else {
            self.package.manifest.room_order.clone()
        }
    }
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
