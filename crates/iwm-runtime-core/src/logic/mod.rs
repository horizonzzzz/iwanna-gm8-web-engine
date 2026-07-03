//! Lowered-logic orchestration for runtime events and GM helper execution.

mod assignment;
mod bootstrap;
mod calls;
mod context;
mod control_flow;
mod diagnostics;
mod eval;
mod eval_functions;
mod eval_values;
mod eval_variables;
mod instances;
mod statement;

use std::collections::{HashMap, VecDeque};

use iwm_runtime_host::{ButtonState, RuntimeButton, RuntimeHost};
use iwm_runtime_model::ObjectDefinition;

use crate::event_dispatch::{
    collision_event_target_object_ids, event_owner_id_for_block_id, object_event_block_ids,
    object_ids_matching_or_inheriting_from, runtime_instance_indices_by_object_id,
    runtime_instance_indices_by_object_id_from_instances, RuntimeEventSelector,
};
use context::RuntimeInstanceCreateRequest;

use crate::{
    LoweredLogicEntry, LoweredLogicExpr, LoweredLogicStatement, RuntimeCore, RuntimeCoreError,
    RuntimeInstance,
};

pub(crate) use bootstrap::apply_view_globals_to_room;
pub(crate) use context::{
    RuntimeBinaryFileState, RuntimeEvalContext, RuntimeExecutionScope, RuntimeRoomInstanceOverlay,
    StepExecutionResult,
};
pub(crate) use statement::{
    apply_runtime_statement, RuntimeDrawContext, RuntimeExecutionTrace, RuntimeStatementEnvironment,
};

impl RuntimeCore {
    pub(crate) fn execute_lowered_step_events<H: RuntimeHost>(
        &mut self,
        host: &mut H,
    ) -> Result<StepExecutionResult, RuntimeCoreError> {
        let button_states = host.active_buttons().into_iter().collect::<HashMap<_, _>>();
        self.execute_lowered_step_events_with_button_states(host, &button_states)
    }

    pub(crate) fn execute_lowered_step_events_with_button_states<H: RuntimeHost>(
        &mut self,
        host: &mut H,
        button_states: &HashMap<RuntimeButton, ButtonState>,
    ) -> Result<StepExecutionResult, RuntimeCoreError> {
        let script_entries = &self.cached_script_entries;
        let objects = &self.package.objects;
        let lowered_entries = self
            .package
            .lowered_logic
            .as_ref()
            .map(|logic| logic.entries.as_slice())
            .unwrap_or(&[]);
        let room_order = &self.cached_room_order;
        let destroy_event_entries = &self.cached_destroy_event_entries;
        let (current_room_id, dispatches, room_instance_indices_by_object_id) = {
            let Some(room) = self.current_room.as_ref() else {
                return Err(RuntimeCoreError::NoRooms);
            };
            let dispatches = room
                .instances
                .iter()
                .enumerate()
                .filter(|(_, instance)| instance.alive)
                .filter_map(|(index, instance)| {
                    self.cached_dispatch_tables
                        .step_entry_indices_by_object_id
                        .contains_key(&instance.object_id)
                        .then_some((index, instance.object_id))
                })
                .collect::<Vec<_>>();
            (
                room.room_id,
                dispatches,
                runtime_instance_indices_by_object_id(room),
            )
        };

        let mut player_motion_changed = false;
        let mut player_jump_owned_by_script = false;
        let mut instance_updates: HashMap<usize, RuntimeInstance> = HashMap::new();
        let mut instance_creates: Vec<RuntimeInstanceCreateRequest> = Vec::new();

        for (index, object_id) in dispatches {
            let Some(mut instance) = instance_updates.get(&index).cloned().or_else(|| {
                self.current_room
                    .as_ref()
                    .and_then(|room| room.instances.get(index).cloned())
            }) else {
                continue;
            };
            if !instance.alive {
                continue;
            }
            let is_player = crate::helpers::is_player_instance(&instance);
            let motion_before = (instance.x, instance.y, instance.hspeed, instance.vspeed);
            let Some(entry_indices) = self
                .cached_dispatch_tables
                .step_entry_indices_by_object_id
                .get(&object_id)
            else {
                continue;
            };
            if is_player
                && entry_indices
                    .iter()
                    .filter_map(|entry_index| lowered_entries.get(*entry_index))
                    .any(|entry| statements_reference_jump_queries(&entry.statements))
            {
                player_jump_owned_by_script = true;
            }

            for entry_index in entry_indices {
                let Some(entry) = lowered_entries.get(*entry_index) else {
                    continue;
                };
                let event_owner_id = event_owner_id_for_block_id(objects, &entry.block_id)
                    .unwrap_or(instance.object_id);
                crate::diagnostics::record_execution_trace(
                    host,
                    &mut self.diagnostics,
                    current_room_id,
                    self.tick,
                    &instance,
                    &entry.block_id,
                    "step",
                );
                let mut scope = RuntimeExecutionScope::default();
                let mut with_updates = Vec::new();
                for statement in &entry.statements {
                    let Some(room) = self.current_room.as_ref() else {
                        return Err(RuntimeCoreError::NoRooms);
                    };
                    let eval_overlay = RuntimeRoomInstanceOverlay::with_current(
                        &instance_updates,
                        &with_updates,
                        index,
                        &instance,
                    );
                    let eval_context = RuntimeEvalContext {
                        current_room_id,
                        button_states,
                        room_instances: &room.instances,
                        room_instance_indices_by_object_id: &room_instance_indices_by_object_id,
                        collision_spatial_index: None,
                        room_instance_overlay: eval_overlay,
                        room_order: room_order.as_slice(),
                        other_instance: None,
                        other_runtime_id: None,
                        place_target_ids_by_name: &self.place_target_ids_by_name,
                        room_ids_by_name: &self.room_ids_by_name,
                    };
                    let mut statement_env = RuntimeStatementEnvironment {
                        script_entries,
                        sound_index: &self.sound_index,
                        globals: &mut self.globals,
                        pending_room_transition: &mut self.pending_room_transition,
                        pending_room_reset: &mut self.pending_room_reset,
                        pending_game_restart: &mut self.pending_game_restart,
                        binary_files: &mut self.binary_files,
                        host: &mut *host,
                        diagnostics: &mut self.diagnostics,
                        room_instance_updates: &mut with_updates,
                        room_instance_creates: &mut instance_creates,
                        objects,
                        sprites: &self.package.resources.sprites,
                        sprite_index: &self.sprite_index,
                        sprite_ids_by_name: &self.sprite_ids_by_name,
                        fonts: &self.package.resources.fonts,
                        font_index_by_name: &self.font_index_by_name,
                        lowered_entries,
                        event_selector: Some(RuntimeEventSelector::Step),
                        event_owner_id: Some(event_owner_id),
                        draw: None,
                        trace: RuntimeExecutionTrace {
                            room_id: current_room_id,
                            tick: self.tick,
                            block_id: entry.block_id.clone(),
                            object_name: instance.object_name.clone(),
                            event_tag: "step".into(),
                        },
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
                    sync_current_instance_from_updates(index, &mut instance, &mut with_updates);
                    if self.has_pending_scene_change() {
                        instance_updates.insert(index, instance);
                        commit_instance_updates(&mut instance_updates, with_updates);
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
                commit_instance_updates(&mut instance_updates, with_updates);
            }

            if is_player
                && (instance.x, instance.y, instance.hspeed, instance.vspeed) != motion_before
            {
                player_motion_changed = true;
            }
            instance_updates.insert(index, instance);
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
        let mut pending_creates: VecDeque<RuntimeInstanceCreateRequest> =
            std::mem::take(creates).into();
        let mut room_instance_indices_by_object_id = self
            .current_room
            .as_ref()
            .map(|room| runtime_instance_indices_by_object_id_from_instances(&room.instances));
        while let Some(create) = pending_creates.pop_front() {
            let scene_change_before_create = self.pending_scene_change_state();
            let Some(current_room_id) = self.current_room.as_ref().map(|room| room.room_id) else {
                return;
            };
            let Some(mut instance) = self.instantiate_runtime_object(
                create.object_id,
                create.runtime_id,
                create.x,
                create.y,
            ) else {
                continue;
            };
            let Some(room_instances) = self.current_room.as_ref().map(|room| &room.instances)
            else {
                return;
            };
            let create_index = room_instances.len();
            let Some(room_instance_indices_by_object_id) =
                room_instance_indices_by_object_id.as_mut()
            else {
                return;
            };
            room_instance_indices_by_object_id
                .entry(instance.object_id)
                .or_default()
                .push(create_index);

            let create_entries = self
                .cached_create_event_entries
                .get(&instance.object_id)
                .cloned();
            if let Some(entries) = create_entries {
                let script_entries = &self.cached_script_entries;
                let objects = &self.package.objects;
                let lowered_entries = self
                    .package
                    .lowered_logic
                    .as_ref()
                    .map(|logic| logic.entries.as_slice())
                    .unwrap_or(&[]);
                let button_states = HashMap::new();
                let room_order = &self.cached_room_order;
                let committed_updates = HashMap::new();
                let eval_context = RuntimeEvalContext {
                    current_room_id,
                    button_states: &button_states,
                    room_instances,
                    room_instance_indices_by_object_id: &room_instance_indices_by_object_id,
                    collision_spatial_index: None,
                    room_instance_overlay: RuntimeRoomInstanceOverlay::with_current(
                        &committed_updates,
                        &[],
                        create_index,
                        &instance,
                    ),
                    room_order,
                    other_instance: None,
                    other_runtime_id: None,
                    place_target_ids_by_name: &self.place_target_ids_by_name,
                    room_ids_by_name: &self.room_ids_by_name,
                };
                let destroy_event_entries = &self.cached_destroy_event_entries;
                let mut room_instance_updates = Vec::new();
                for entry in &entries {
                    let event_owner_id = event_owner_id_for_block_id(objects, &entry.block_id)
                        .unwrap_or(instance.object_id);
                    crate::diagnostics::record_execution_trace(
                        host,
                        &mut self.diagnostics,
                        current_room_id,
                        self.tick,
                        &instance,
                        &entry.block_id,
                        "create",
                    );
                    let mut scope = RuntimeExecutionScope::default();
                    for statement in &entry.statements {
                        let mut statement_env = RuntimeStatementEnvironment {
                            script_entries,
                            sound_index: &self.sound_index,
                            globals: &mut self.globals,
                            pending_room_transition: &mut self.pending_room_transition,
                            pending_room_reset: &mut self.pending_room_reset,
                            pending_game_restart: &mut self.pending_game_restart,
                            binary_files: &mut self.binary_files,
                            host: &mut *host,
                            diagnostics: &mut self.diagnostics,
                            room_instance_updates: &mut room_instance_updates,
                            room_instance_creates: creates,
                            objects,
                            sprites: &self.package.resources.sprites,
                            sprite_index: &self.sprite_index,
                            sprite_ids_by_name: &self.sprite_ids_by_name,
                            fonts: &self.package.resources.fonts,
                            font_index_by_name: &self.font_index_by_name,
                            lowered_entries,
                            event_selector: None,
                            event_owner_id: Some(event_owner_id),
                            draw: None,
                            trace: RuntimeExecutionTrace {
                                room_id: current_room_id,
                                tick: self.tick,
                                block_id: entry.block_id.clone(),
                                object_name: instance.object_name.clone(),
                                event_tag: "create".into(),
                            },
                        };
                        apply_runtime_statement(
                            statement,
                            &mut instance,
                            create.runtime_id,
                            &mut scope,
                            &destroy_event_entries,
                            Some(&eval_context),
                            &mut statement_env,
                        );
                        if self.pending_scene_change_state() != scene_change_before_create {
                            break;
                        }
                    }
                    if self.pending_scene_change_state() != scene_change_before_create {
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

            for (key, value) in create.post_create_vars {
                assignment::assign_instance_field_or_var(key, value, &mut instance);
            }

            let created_object_name = instance.object_name.clone();
            let created_x = instance.x;
            let created_y = instance.y;
            let created_object_id = instance.object_id;
            if let Some(room) = self.current_room.as_mut() {
                room.instances.push(instance);
            }
            self.record_diagnostic(
                host,
                iwm_runtime_host::RuntimeDiagnosticLevel::Info,
                "runtime-instance-created",
                format!(
                    "room={} tick={} object={} runtime_id={} x={} y={}",
                    current_room_id,
                    self.tick,
                    created_object_name,
                    create.runtime_id,
                    created_x,
                    created_y
                ),
            );
            room_instance_indices_by_object_id
                .entry(created_object_id)
                .or_default();

            if self.has_pending_scene_change() {
                break;
            }

            if !creates.is_empty() {
                pending_creates.extend(std::mem::take(creates));
            }
        }
        while let Some(create) = pending_creates.pop_front() {
            creates.push(create);
        }
    }

    pub(crate) fn lowered_event_entries_by_tag_for_runtime(
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

    fn pending_scene_change_state(&self) -> (bool, bool, Option<usize>) {
        (
            self.pending_game_restart,
            self.pending_room_reset,
            self.pending_room_transition,
        )
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

    pub(crate) fn collision_target_ids_by_object_id(&self) -> HashMap<usize, Vec<usize>> {
        self.package
            .objects
            .iter()
            .filter_map(|object| {
                let target_ids = collision_event_target_object_ids(&self.package, object.id);
                (!target_ids.is_empty()).then_some((object.id, target_ids))
            })
            .collect()
    }

    pub(crate) fn collision_matching_object_ids_by_target(&self) -> HashMap<usize, Vec<usize>> {
        self.cached_collision_target_ids
            .values()
            .flatten()
            .copied()
            .fold(HashMap::new(), |mut targets, target_object_id| {
                targets.entry(target_object_id).or_insert_with(|| {
                    object_ids_matching_or_inheriting_from(&self.package, target_object_id)
                });
                targets
            })
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

pub(crate) fn sync_current_instance_from_updates(
    current_index: usize,
    current_instance: &mut RuntimeInstance,
    pending_updates: &mut Vec<(usize, RuntimeInstance)>,
) {
    let Some(last_update_index) = pending_updates
        .iter()
        .rposition(|(index, _)| *index == current_index)
    else {
        return;
    };
    *current_instance = pending_updates[last_update_index].1.clone();
    pending_updates.retain(|(index, _)| *index != current_index);
}

pub(crate) fn commit_instance_updates(
    committed_updates: &mut HashMap<usize, RuntimeInstance>,
    updates: Vec<(usize, RuntimeInstance)>,
) {
    for (index, instance) in updates {
        committed_updates.insert(index, instance);
    }
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
