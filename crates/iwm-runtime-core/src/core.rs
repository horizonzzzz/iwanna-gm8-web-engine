use std::collections::HashMap;

use iwm_runtime_host::{ButtonState, RuntimeButton, RuntimeHost};

use crate::event_dispatch::{
    object_event_block_ids, runtime_collision_spatial_index,
    runtime_instance_indices_by_object_id_from_instances, RuntimeCollisionSpatialIndex,
    RuntimeEventSelector,
};
use crate::helpers::{as_number, collides_at, is_player_instance};
use crate::logic::RuntimeBinaryFileState;
use crate::{
    LoweredLogicEntry, RuntimeCoreError, RuntimeInputTraceSnapshot, RuntimeInstance,
    RuntimeJumpSnapshot, RuntimePackage, RuntimePlayerSnapshot, RuntimeRoomState, RuntimeSnapshot,
    RuntimeStatus, RuntimeTickPhaseSnapshot, RuntimeValue,
};

#[derive(Debug)]
pub struct RuntimeCore {
    pub(crate) package: RuntimePackage,
    pub(crate) object_index: HashMap<usize, usize>,
    pub(crate) room_index: HashMap<usize, usize>,
    /// Maps a sprite id to its index in `package.resources.sprites`, so the
    /// render pass can resolve sprites without scanning the sprite list per
    /// instance.
    pub(crate) sprite_index: HashMap<usize, usize>,
    pub(crate) sound_index: HashMap<String, i32>,
    pub(crate) lowered_logic_index: HashMap<String, usize>,
    /// Static-after-load tables used every tick by the step dispatch. Cached
    /// here so `execute_lowered_step_events` does not rebuild and clone them on
    /// each tick.
    pub(crate) cached_script_entries: HashMap<String, LoweredLogicEntry>,
    pub(crate) cached_room_order: Vec<usize>,
    pub(crate) cached_step_event_blocks: HashMap<usize, Vec<String>>,
    pub(crate) cached_create_event_entries: HashMap<usize, Vec<LoweredLogicEntry>>,
    pub(crate) cached_destroy_event_entries: HashMap<usize, Vec<LoweredLogicEntry>>,
    pub(crate) cached_collision_target_ids: HashMap<usize, Vec<usize>>,
    pub(crate) cached_collision_matching_object_ids: HashMap<usize, Vec<usize>>,
    /// Maps a lowercased object name to the full set of object ids that match or
    /// inherit from it, so `place_meeting`/`place_free` skip the per-call
    /// inheritance walk over every object.
    pub(crate) place_target_ids_by_name: HashMap<String, Vec<usize>>,
    pub(crate) room_ids_by_name: HashMap<String, usize>,
    pub(crate) current_room: Option<RuntimeRoomState>,
    pub(crate) status: RuntimeStatus,
    pub(crate) tick: u64,
    pub(crate) diagnostics: Vec<iwm_runtime_host::RuntimeDiagnostic>,
    pub(crate) pending_room_transition: Option<usize>,
    pub(crate) pending_room_reset: bool,
    pub(crate) room_needs_first_render_settle: bool,
    pub(crate) globals: HashMap<String, RuntimeValue>,
    pub(crate) package_bootstrap_globals: HashMap<String, RuntimeValue>,
    pub(crate) last_input_trace: RuntimeInputTraceSnapshot,
    pub(crate) last_tick_phases: RuntimeTickPhaseSnapshot,
    pub(crate) death_waiting_for_restart: bool,
    pub(crate) binary_files: RuntimeBinaryFileState,
}

impl RuntimeCore {
    pub fn load(package: RuntimePackage) -> Result<Self, RuntimeCoreError> {
        if package.rooms.is_empty() {
            return Err(RuntimeCoreError::NoRooms);
        }

        let room_index = package
            .rooms
            .iter()
            .enumerate()
            .map(|(index, room)| (room.id, index))
            .collect::<HashMap<_, _>>();
        let object_index = package
            .objects
            .iter()
            .enumerate()
            .map(|(index, object)| (object.id, index))
            .collect::<HashMap<_, _>>();
        let sprite_index = package
            .resources
            .sprites
            .iter()
            .enumerate()
            .map(|(index, sprite)| (sprite.id, index))
            .collect::<HashMap<_, _>>();
        let sound_index = package
            .resources
            .sounds
            .iter()
            .filter_map(|sound| {
                i32::try_from(sound.id)
                    .ok()
                    .map(|id| (sound.name.to_ascii_lowercase(), id))
            })
            .collect::<HashMap<_, _>>();
        let lowered_logic_index = package
            .lowered_logic
            .as_ref()
            .map(|lowered_logic| {
                lowered_logic
                    .entries
                    .iter()
                    .enumerate()
                    .map(|(index, entry)| (entry.block_id.clone(), index))
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default();
        let room_ids_by_name = package
            .rooms
            .iter()
            .map(|room| (room.name.to_ascii_lowercase(), room.id))
            .collect::<HashMap<_, _>>();

        let mut core = Self {
            package,
            object_index,
            room_index,
            sprite_index,
            sound_index,
            lowered_logic_index,
            cached_script_entries: HashMap::new(),
            cached_room_order: Vec::new(),
            cached_step_event_blocks: HashMap::new(),
            cached_create_event_entries: HashMap::new(),
            cached_destroy_event_entries: HashMap::new(),
            cached_collision_target_ids: HashMap::new(),
            cached_collision_matching_object_ids: HashMap::new(),
            place_target_ids_by_name: HashMap::new(),
            room_ids_by_name,
            current_room: None,
            status: RuntimeStatus::Ready,
            tick: 0,
            diagnostics: Vec::new(),
            pending_room_transition: None,
            pending_room_reset: false,
            room_needs_first_render_settle: false,
            globals: HashMap::new(),
            package_bootstrap_globals: HashMap::new(),
            last_input_trace: RuntimeInputTraceSnapshot {
                jump_button_key: 0x20,
                jump_pressed: false,
                jump_just_pressed: false,
                jump_just_released: false,
                active_keys: Vec::new(),
            },
            last_tick_phases: RuntimeTickPhaseSnapshot::default(),
            death_waiting_for_restart: false,
            binary_files: RuntimeBinaryFileState::default(),
        };

        core.cached_script_entries = core.lowered_script_entries();
        core.cached_room_order = core.runtime_room_order();
        core.cached_step_event_blocks = core.object_event_blocks_by_tag("step");
        core.cached_create_event_entries = core.lowered_event_entries_by_tag_for_runtime("create");
        core.cached_destroy_event_entries =
            core.lowered_event_entries_by_selector(RuntimeEventSelector::Destroy);
        core.cached_collision_target_ids = core.collision_target_ids_by_object_id();
        core.cached_collision_matching_object_ids = core.collision_matching_object_ids_by_target();
        core.place_target_ids_by_name = core.compute_place_target_ids_by_name();
        core.package_bootstrap_globals = core.collect_package_bootstrap_globals();
        core.boot_default_room()?;
        Ok(core)
    }

    pub fn status(&self) -> RuntimeStatus {
        self.status
    }

    pub fn tick_count(&self) -> u64 {
        self.tick
    }

    pub fn current_room(&self) -> Option<&RuntimeRoomState> {
        self.current_room.as_ref()
    }

    pub fn diagnostics(&self) -> &[iwm_runtime_host::RuntimeDiagnostic] {
        &self.diagnostics
    }

    pub fn snapshot(&self) -> RuntimeSnapshot {
        RuntimeSnapshot {
            status: self.status,
            tick: self.tick,
            room_id: self.current_room.as_ref().map(|room| room.room_id),
            room_name: self
                .current_room
                .as_ref()
                .map(|room| room.room_name.clone()),
            instance_count: self
                .current_room
                .as_ref()
                .map(|room| room.instances.len())
                .unwrap_or(0),
            player: self.current_room.as_ref().and_then(|room| {
                let solids = room
                    .instances
                    .iter()
                    .filter(|instance| instance.alive && instance.solid)
                    .cloned()
                    .collect::<Vec<_>>();
                room.instances
                    .iter()
                    .find(|instance| is_player_instance(instance))
                    .map(|instance| RuntimePlayerSnapshot {
                        runtime_id: instance.runtime_id,
                        instance_id: instance.instance_id,
                        object_id: instance.object_id,
                        object_name: instance.object_name.clone(),
                        x: instance.x,
                        y: instance.y,
                        hspeed: instance.hspeed,
                        vspeed: instance.vspeed,
                        facing_left: instance.facing_left,
                        alive: instance.alive,
                        jump: RuntimeJumpSnapshot {
                            grounded: collides_at(
                                instance,
                                instance.x,
                                instance.y + 1.0,
                                &solids,
                                Some(instance.runtime_id),
                            ),
                            active: instance.jump.active,
                            hold_frames: instance.jump.hold_frames,
                            cut_applied: instance.jump.cut_applied,
                        },
                    })
            }),
            input_trace: self.last_input_trace.clone(),
            tick_phases: self.last_tick_phases,
            diagnostics: self.diagnostics.clone(),
        }
    }

    pub fn request_room_transition(&mut self, room_id: usize) {
        self.pending_room_transition = Some(room_id);
    }

    pub fn render<H: RuntimeHost>(&mut self, host: &mut H) -> Result<(), RuntimeCoreError> {
        self.settle_current_room_before_first_render(host)?;
        self.sync_current_room_views_from_globals();
        let frame = self.build_render_frame()?;
        host.submit_frame(frame)?;
        Ok(())
    }

    pub fn tick<H: RuntimeHost>(&mut self, host: &mut H) -> Result<(), RuntimeCoreError> {
        if self.current_room.is_none() {
            self.status = RuntimeStatus::Error;
            return Err(RuntimeCoreError::NoRooms);
        }

        let tick_start = host.diagnostic_now_nanos();
        let mut phase_start = tick_start;
        let mut tick_phases = RuntimeTickPhaseSnapshot::default();

        let left = self.bound_button_state(host, "global.leftbutton", 0x25);
        let right = self.bound_button_state(host, "global.rightbutton", 0x27);
        let mut jump = self.bound_button_state(host, "global.jumpbutton", 0x20);
        let restart = self.bound_restart_button_state(host);
        self.record_jump_input_diagnostic(host, jump);

        self.tick += 1;
        self.status = RuntimeStatus::Running;

        if !left.pressed && !right.pressed && !jump.pressed && !restart.pressed {
            self.record_diagnostic(
                host,
                iwm_runtime_host::RuntimeDiagnosticLevel::Info,
                "runtime-idle",
                format!("tick {} advanced without player input", self.tick),
            );
        }

        tick_phases.input_diag_nanos += mark_phase_elapsed(host, &mut phase_start);

        self.apply_pending_room_change(host)?;
        self.room_needs_first_render_settle = false;
        tick_phases.view_sync_nanos += mark_phase_elapsed(host, &mut phase_start);

        let Some(room) = self.current_room.as_ref() else {
            self.status = RuntimeStatus::Error;
            return Err(RuntimeCoreError::NoRooms);
        };

        if room.instances.is_empty() {
            self.record_diagnostic(
                host,
                iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
                "runtime-empty-room",
                format!("room {} has no live instances", room.room_name),
            );
            tick_phases.step_events_nanos += mark_phase_elapsed(host, &mut phase_start);
        } else {
            let step_result = self.execute_lowered_step_events(host)?;
            tick_phases.step_events_nanos += mark_phase_elapsed(host, &mut phase_start);

            self.sync_current_room_views_from_globals();
            tick_phases.view_sync_nanos += mark_phase_elapsed(host, &mut phase_start);

            if step_result.interrupted {
                self.apply_pending_room_change(host)?;
                tick_phases.view_sync_nanos += mark_phase_elapsed(host, &mut phase_start);

                self.render(host)?;
                tick_phases.render_submit_nanos += mark_phase_elapsed(host, &mut phase_start);
                tick_phases.total_nanos = elapsed_since(host, tick_start);
                self.last_tick_phases = tick_phases;
                return Ok(());
            }

            if !step_result.player_motion_changed || step_result.player_jump_owned_by_script {
                jump = self.bound_button_state(host, "global.jumpbutton", 0x20);
                self.step_player(
                    host,
                    left.pressed,
                    right.pressed,
                    jump,
                    !step_result.player_jump_owned_by_script,
                )?;
            }
            tick_phases.player_movement_nanos += mark_phase_elapsed(host, &mut phase_start);

            self.step_non_player_instances()?;
            tick_phases.player_movement_nanos += mark_phase_elapsed(host, &mut phase_start);
        }

        self.dispatch_collision_events(host)?;
        tick_phases.collision_events_nanos += mark_phase_elapsed(host, &mut phase_start);

        // Dispatch alarm events (countdown alarm state)
        self.process_alarm_countdowns(host)?;
        tick_phases.alarms_nanos += mark_phase_elapsed(host, &mut phase_start);

        // Dispatch held keyboard events for any currently pressed key exposed by the host.
        for (button, state) in host.active_buttons() {
            if let RuntimeButton::Keyboard(key) = button {
                if state.pressed {
                    self.execute_event_blocks(host, RuntimeEventSelector::KeyboardHeld(key))?;
                }
            }
        }

        // Dispatch key press events.
        for (button, state) in host.active_buttons() {
            if let RuntimeButton::Keyboard(key) = button {
                if state.just_pressed {
                    self.execute_event_blocks(host, RuntimeEventSelector::KeyboardPressed(key))?;
                }
            }
        }

        // Dispatch key release events.
        for (button, state) in host.active_buttons() {
            if let RuntimeButton::Keyboard(key) = button {
                if state.just_released {
                    self.execute_event_blocks(host, RuntimeEventSelector::KeyboardReleased(key))?;
                }
            }
        }

        if restart.just_pressed {
            let current_room_id = self
                .current_room
                .as_ref()
                .map(|room| room.room_id)
                .unwrap_or(0);
            self.record_diagnostic(
                host,
                iwm_runtime_host::RuntimeDiagnosticLevel::Info,
                "runtime-room-restart-requested",
                format!("room={} tick={}", current_room_id, self.tick),
            );
            self.pending_room_reset = true;
        }
        tick_phases.keyboard_events_nanos += mark_phase_elapsed(host, &mut phase_start);

        if self.pending_room_reset || self.pending_room_transition.is_some() {
            self.apply_pending_room_change(host)?;
        }
        tick_phases.view_sync_nanos += mark_phase_elapsed(host, &mut phase_start);

        self.render(host)?;
        tick_phases.render_submit_nanos += mark_phase_elapsed(host, &mut phase_start);
        tick_phases.total_nanos = elapsed_since(host, tick_start);
        self.last_tick_phases = tick_phases;
        Ok(())
    }

    fn settle_current_room_before_first_render<H: RuntimeHost>(
        &mut self,
        host: &mut H,
    ) -> Result<(), RuntimeCoreError> {
        if !self.room_needs_first_render_settle {
            return Ok(());
        }
        self.room_needs_first_render_settle = false;

        let Some(room) = self.current_room.as_ref() else {
            return Err(RuntimeCoreError::NoRooms);
        };
        if room.instances.is_empty() {
            return Ok(());
        }

        let left = self.bound_button_state(host, "global.leftbutton", 0x25);
        let right = self.bound_button_state(host, "global.rightbutton", 0x27);
        let jump = self.bound_button_state(host, "global.jumpbutton", 0x20);
        let step_result = self.execute_lowered_step_events(host)?;
        self.sync_current_room_views_from_globals();

        if step_result.interrupted {
            self.apply_pending_room_change(host)?;
            return Ok(());
        }

        if !step_result.player_motion_changed || step_result.player_jump_owned_by_script {
            self.step_player(
                host,
                left.pressed,
                right.pressed,
                jump,
                !step_result.player_jump_owned_by_script,
            )?;
        }

        self.step_non_player_instances()?;

        if self.pending_room_reset || self.pending_room_transition.is_some() {
            self.apply_pending_room_change(host)?;
            return Ok(());
        }

        self.dispatch_collision_events(host)?;
        if self.pending_room_reset || self.pending_room_transition.is_some() {
            self.apply_pending_room_change(host)?;
            return Ok(());
        }

        self.process_alarm_countdowns(host)?;
        if self.pending_room_reset || self.pending_room_transition.is_some() {
            self.apply_pending_room_change(host)?;
        }

        Ok(())
    }

    fn sync_current_room_views_from_globals(&mut self) {
        if let Some(room) = self.current_room.as_mut() {
            crate::logic::apply_view_globals_to_room(room, &self.globals);
        }
    }

    fn bound_button_state<H: RuntimeHost>(
        &self,
        host: &H,
        binding_key: &str,
        fallback_key_code: u16,
    ) -> ButtonState {
        let key_code = self
            .globals
            .get(binding_key)
            .and_then(as_number)
            .map(|value| value.round() as u16)
            .unwrap_or(fallback_key_code);
        host.button_state(RuntimeButton::Keyboard(key_code))
    }

    fn bound_restart_button_state<H: RuntimeHost>(&self, host: &H) -> ButtonState {
        let key_code = self
            .globals
            .get("global.restartbutton")
            .or_else(|| self.globals.get("global.resetbutton"))
            .and_then(as_number)
            .map(|value| value.round() as u16)
            .unwrap_or(0x52);
        host.button_state(RuntimeButton::Keyboard(key_code))
    }

    pub fn reload_room(&mut self, room_id: usize) -> Result<(), RuntimeCoreError> {
        self.hydrate_missing_package_bootstrap_globals();
        self.current_room = Some(self.build_room(room_id)?);
        self.room_needs_first_render_settle = true;
        self.death_waiting_for_restart = false;
        self.status = RuntimeStatus::Ready;
        Ok(())
    }

    pub(crate) fn execute_event_blocks<H: RuntimeHost>(
        &mut self,
        host: &mut H,
        selector: RuntimeEventSelector,
    ) -> Result<(), RuntimeCoreError> {
        let block_lookups: Vec<(usize, Vec<String>)> = {
            let Some(room) = self.current_room.as_ref() else {
                return Err(RuntimeCoreError::NoRooms);
            };
            room.instances
                .iter()
                .enumerate()
                .filter(|(_, i)| i.alive)
                .filter_map(|(idx, instance)| {
                    let block_ids =
                        object_event_block_ids(&self.package, instance.object_id, selector.clone());
                    if block_ids.is_empty() {
                        None
                    } else {
                        Some((idx, block_ids))
                    }
                })
                .collect()
        };

        for (instance_idx, block_ids) in block_lookups {
            self.apply_event_blocks_to_instance(
                host,
                instance_idx,
                &block_ids,
                None,
                selector_event_tag(&selector),
                None,
            );
            if self.pending_room_reset || self.pending_room_transition.is_some() {
                break;
            }
        }

        Ok(())
    }

    fn apply_event_blocks_to_instance<H: RuntimeHost>(
        &mut self,
        host: &mut H,
        instance_idx: usize,
        block_ids: &[String],
        other_instance: Option<RuntimeInstance>,
        event_tag: String,
        collision_spatial_index: Option<&RuntimeCollisionSpatialIndex>,
    ) {
        // Clone entries first to avoid borrow conflicts
        let entries: Vec<LoweredLogicEntry> = block_ids
            .iter()
            .filter_map(|block_id| self.lowered_logic_entry(block_id).cloned())
            .collect();

        let script_entries = &self.cached_script_entries;
        let destroy_event_entries = &self.cached_destroy_event_entries;
        let button_states = host
            .active_buttons()
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        let current_room_id = {
            let Some(room) = self.current_room.as_ref() else {
                return;
            };
            room.room_id
        };
        let known_files = crate::logic::sample_known_files(host);
        let room_order = &self.cached_room_order;

        let mut instance = {
            let Some(room) = self.current_room.as_ref() else {
                return;
            };
            let Some(instance) = room.instances.get(instance_idx) else {
                return;
            };
            instance.clone()
        };

        let mut instance_creates = Vec::new();
        let mut instance_updates = HashMap::new();
        let room_instance_indices_by_object_id = {
            let Some(room) = self.current_room.as_ref() else {
                return;
            };
            runtime_instance_indices_by_object_id_from_instances(&room.instances)
        };

        for entry in &entries {
            crate::diagnostics::record_execution_trace(
                host,
                &mut self.diagnostics,
                current_room_id,
                self.tick,
                &instance,
                &entry.block_id,
                &event_tag,
            );
            let mut scope = crate::logic::RuntimeExecutionScope::default();
            let mut with_updates = Vec::new();
            for statement in &entry.statements {
                let Some(room) = self.current_room.as_ref() else {
                    return;
                };
                let eval_overlay = crate::logic::RuntimeRoomInstanceOverlay::with_current(
                    &instance_updates,
                    &with_updates,
                    instance_idx,
                    &instance,
                );
                let eval_context = crate::logic::RuntimeEvalContext {
                    current_room_id,
                    button_states: &button_states,
                    room_instances: &room.instances,
                    room_instance_indices_by_object_id: &room_instance_indices_by_object_id,
                    collision_spatial_index,
                    room_instance_overlay: eval_overlay,
                    room_order,
                    known_files: &known_files,
                    other_instance: other_instance.as_ref(),
                    other_runtime_id: other_instance.as_ref().map(|instance| instance.runtime_id),
                    place_target_ids_by_name: &self.place_target_ids_by_name,
                    room_ids_by_name: &self.room_ids_by_name,
                };
                let mut statement_env = crate::logic::RuntimeStatementEnvironment {
                    script_entries,
                    sound_index: &self.sound_index,
                    globals: &mut self.globals,
                    pending_room_transition: &mut self.pending_room_transition,
                    pending_room_reset: &mut self.pending_room_reset,
                    binary_files: &mut self.binary_files,
                    host: &mut *host,
                    diagnostics: &mut self.diagnostics,
                    room_instance_updates: &mut with_updates,
                    room_instance_creates: &mut instance_creates,
                    trace: crate::logic::RuntimeExecutionTrace {
                        room_id: current_room_id,
                        tick: self.tick,
                        block_id: entry.block_id.clone(),
                        object_name: instance.object_name.clone(),
                        event_tag: event_tag.clone(),
                    },
                };
                crate::logic::apply_runtime_statement(
                    statement,
                    &mut instance,
                    instance_idx,
                    &mut scope,
                    &destroy_event_entries,
                    Some(&eval_context),
                    &mut statement_env,
                );
                crate::logic::sync_current_instance_from_updates(
                    instance_idx,
                    &mut instance,
                    &mut with_updates,
                );
                if self.pending_room_reset || self.pending_room_transition.is_some() {
                    break;
                }
            }
            crate::logic::commit_instance_updates(&mut instance_updates, with_updates);
            if self.pending_room_reset || self.pending_room_transition.is_some() {
                break;
            }
        }

        if let Some(room) = self.current_room.as_mut() {
            for (update_index, updated_instance) in instance_updates {
                if let Some(slot) = room.instances.get_mut(update_index) {
                    *slot = updated_instance;
                }
            }
        }
        self.apply_runtime_instance_creates(host, &mut instance_creates);

        let Some(room) = self.current_room.as_mut() else {
            return;
        };
        if let Some(slot) = room.instances.get_mut(instance_idx) {
            *slot = instance;
        }
    }

    fn process_alarm_countdowns<H: RuntimeHost>(
        &mut self,
        host: &mut H,
    ) -> Result<(), RuntimeCoreError> {
        // First, collect all alarm slots that will fire on this tick.
        let alarm_triggers: Vec<(usize, u32)> = {
            let Some(room) = self.current_room.as_ref() else {
                return Err(RuntimeCoreError::NoRooms);
            };
            room.instances
                .iter()
                .enumerate()
                .filter(|(_, i)| i.alive)
                .flat_map(|(idx, instance)| {
                    instance.vars.iter().filter_map(move |(key, value)| {
                        match (parse_alarm_slot(key), value) {
                            (Some(slot), RuntimeValue::Number(ticks))
                                if *ticks > 0.0 && *ticks <= 1.0 =>
                            {
                                Some((idx, slot))
                            }
                            _ => None,
                        }
                    })
                })
                .collect()
        };

        // Decrement all active alarm counters.
        {
            let Some(room) = self.current_room.as_mut() else {
                return Err(RuntimeCoreError::NoRooms);
            };
            for instance in &mut room.instances {
                for (key, value) in instance.vars.iter_mut() {
                    if parse_alarm_slot(key).is_some() {
                        if let RuntimeValue::Number(ticks) = value {
                            if *ticks > 0.0 {
                                *ticks -= 1.0;
                            }
                        }
                    }
                }
            }
        }

        for (instance_idx, slot) in alarm_triggers {
            let block_ids = {
                let Some(room) = self.current_room.as_ref() else {
                    continue;
                };
                let Some(instance) = room.instances.get(instance_idx) else {
                    continue;
                };
                object_event_block_ids(
                    &self.package,
                    instance.object_id,
                    RuntimeEventSelector::Alarm(slot),
                )
            };
            self.apply_event_blocks_to_instance(
                host,
                instance_idx,
                &block_ids,
                None,
                format!("alarm:{slot}"),
                None,
            );
        }

        Ok(())
    }

    fn dispatch_collision_events<H: RuntimeHost>(
        &mut self,
        host: &mut H,
    ) -> Result<(), RuntimeCoreError> {
        let (collisions, spatial_index) = {
            let Some(room) = self.current_room.as_ref() else {
                return Err(RuntimeCoreError::NoRooms);
            };
            let mut hits = Vec::new();
            let spatial_index = runtime_collision_spatial_index(room);
            for (instance_index, instance) in room.instances.iter().enumerate() {
                if !instance.alive {
                    continue;
                }
                let Some(target_object_ids) =
                    self.cached_collision_target_ids.get(&instance.object_id)
                else {
                    continue;
                };
                if target_object_ids.is_empty() {
                    continue;
                }
                for &target_object_id in target_object_ids {
                    let Some(matching_object_ids) = self
                        .cached_collision_matching_object_ids
                        .get(&target_object_id)
                    else {
                        continue;
                    };
                    for matching_object_id in matching_object_ids {
                        for other_index in spatial_index.candidate_indices(
                            *matching_object_id,
                            instance,
                            instance.x,
                            instance.y,
                        ) {
                            let Some(other) = room.instances.get(other_index) else {
                                continue;
                            };
                            if instance.runtime_id == other.runtime_id {
                                continue;
                            }
                            if crate::helpers::collides_at(
                                instance,
                                instance.x,
                                instance.y,
                                std::slice::from_ref(other),
                                Some(instance.runtime_id),
                            ) {
                                hits.push((
                                    instance_index,
                                    target_object_id,
                                    other_index,
                                    instance.solid || other.solid,
                                ));
                            }
                        }
                    }
                }
            }
            (hits, spatial_index)
        };

        for (instance_idx, target_object_id, other_idx, solid_collision) in collisions {
            let other_instance = {
                let Some(room) = self.current_room.as_mut() else {
                    continue;
                };
                if instance_idx >= room.instances.len() || other_idx >= room.instances.len() {
                    continue;
                }
                if !room.instances[instance_idx].alive || !room.instances[other_idx].alive {
                    continue;
                }
                if solid_collision {
                    if let Some(instance) = room.instances.get_mut(instance_idx) {
                        instance.x = instance.previous_x;
                        instance.y = instance.previous_y;
                    }
                    if let Some(other) = room.instances.get_mut(other_idx) {
                        other.x = other.previous_x;
                        other.y = other.previous_y;
                    }
                }
                room.instances.get(other_idx).cloned()
            };
            let Some(other_instance) = other_instance else {
                continue;
            };
            let block_ids = {
                let Some(room) = self.current_room.as_ref() else {
                    continue;
                };
                let Some(instance) = room.instances.get(instance_idx) else {
                    continue;
                };
                object_event_block_ids(
                    &self.package,
                    instance.object_id,
                    RuntimeEventSelector::Collision { target_object_id },
                )
            };
            self.apply_event_blocks_to_instance(
                host,
                instance_idx,
                &block_ids,
                Some(other_instance),
                "collision".into(),
                Some(&spatial_index),
            );
        }

        Ok(())
    }
}

fn parse_alarm_slot(key: &str) -> Option<u32> {
    key.strip_prefix("alarm[")?.strip_suffix(']')?.parse().ok()
}

fn selector_event_tag(selector: &RuntimeEventSelector) -> String {
    match selector {
        RuntimeEventSelector::Destroy => "destroy".into(),
        RuntimeEventSelector::Alarm(slot) => format!("alarm:{slot}"),
        RuntimeEventSelector::KeyboardHeld(key) => {
            format!("keyboard:{}", format_selector_key_name(*key))
        }
        RuntimeEventSelector::KeyboardPressed(key) => {
            format!("keypress:{}", format_selector_key_name(*key))
        }
        RuntimeEventSelector::KeyboardReleased(key) => {
            format!("keyrelease:{}", format_selector_key_name(*key))
        }
        RuntimeEventSelector::Collision { .. } => "collision".into(),
    }
}

fn format_selector_key_name(sub_event: u16) -> String {
    let key = sub_event as u8 as char;
    if key.is_ascii_alphanumeric() {
        key.to_ascii_lowercase().to_string()
    } else {
        format!("0x{:02x}", sub_event as u8)
    }
}

fn mark_phase_elapsed<H: RuntimeHost>(host: &H, phase_start: &mut Option<u128>) -> u64 {
    let Some(start) = *phase_start else {
        return 0;
    };
    let Some(end) = host.diagnostic_now_nanos() else {
        *phase_start = None;
        return 0;
    };
    *phase_start = Some(end);
    nanos_between(start, end)
}

fn elapsed_since<H: RuntimeHost>(host: &H, start: Option<u128>) -> u64 {
    let Some(start) = start else {
        return 0;
    };
    let Some(end) = host.diagnostic_now_nanos() else {
        return 0;
    };
    nanos_between(start, end)
}

fn nanos_between(start: u128, end: u128) -> u64 {
    end.saturating_sub(start).min(u128::from(u64::MAX)) as u64
}
