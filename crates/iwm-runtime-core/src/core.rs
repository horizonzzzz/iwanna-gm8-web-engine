//! RuntimeCore state, package loading, tick orchestration, and snapshots.

use std::cell::Cell;
use std::collections::{HashMap, HashSet};

use iwm_runtime_host::{ButtonState, RuntimeButton, RuntimeHost};
use iwm_runtime_model::RoomDefinition;

use crate::event_dispatch::{
    event_owner_id_for_block_id, object_event_block_ids, runtime_event_dispatch_tables,
    runtime_instance_indices_by_object_id_from_instances, RuntimeCollisionSpatialIndex,
    RuntimeEventDispatchTables, RuntimeEventSelector,
};
use crate::helpers::{
    as_number, bounds_at, collides_at, collides_with_instances_at, is_player_instance,
};
use crate::logic::{RuntimeBinaryFileState, RuntimeSparseInstanceOverlay};
use crate::tick_context::{RuntimeCollisionHit, RuntimeTickContext};
use crate::{
    LoweredLogicEntry, LoweredLogicExpr, LoweredLogicStatement, RuntimeCoreError,
    RuntimeInputTraceSnapshot, RuntimeInstance, RuntimeJumpSnapshot, RuntimePackage,
    RuntimePlayerSnapshot, RuntimeRoomState, RuntimeSnapshot, RuntimeStatus,
    RuntimeTickPhaseSnapshot, RuntimeValue,
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
    pub(crate) sprite_ids_by_name: HashMap<String, usize>,
    pub(crate) font_index_by_name: HashMap<String, usize>,
    pub(crate) sound_index: HashMap<String, i32>,
    pub(crate) lowered_logic_index: HashMap<String, usize>,
    /// Static-after-load tables used every tick by the step dispatch. Cached
    /// here so `execute_lowered_step_events` does not rebuild and clone them on
    /// each tick.
    pub(crate) cached_script_entries: HashMap<String, LoweredLogicEntry>,
    pub(crate) cached_room_order: Vec<usize>,
    pub(crate) cached_step_event_blocks: HashMap<usize, Vec<String>>,
    pub(crate) cached_dispatch_tables: RuntimeEventDispatchTables,
    pub(crate) cached_create_event_entries: HashMap<usize, Vec<LoweredLogicEntry>>,
    pub(crate) cached_destroy_event_entries: HashMap<usize, Vec<LoweredLogicEntry>>,
    pub(crate) cached_timeline_entries: HashMap<usize, Vec<(u32, usize)>>,
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
    pub(crate) pending_game_restart: bool,
    pub(crate) room_needs_first_render_settle: bool,
    pub(crate) globals: HashMap<String, RuntimeValue>,
    pub(crate) package_bootstrap_globals: HashMap<String, RuntimeValue>,
    pub(crate) last_input_trace: RuntimeInputTraceSnapshot,
    pub(crate) last_tick_phases: RuntimeTickPhaseSnapshot,
    pub(crate) death_waiting_for_restart: bool,
    pub(crate) host_bootstrap_scripts_applied: bool,
    pub(crate) binary_files: RuntimeBinaryFileState,
    pub(crate) tick_context: RuntimeTickContext,
    pub(crate) random_state: Cell<u64>,
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
        let sprite_ids_by_name = package
            .resources
            .sprites
            .iter()
            .map(|sprite| (sprite.name.to_ascii_lowercase(), sprite.id))
            .collect::<HashMap<_, _>>();
        let font_index_by_name = package
            .resources
            .fonts
            .iter()
            .enumerate()
            .map(|(index, font)| (font.name.to_ascii_lowercase(), index))
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
            sprite_ids_by_name,
            font_index_by_name,
            sound_index,
            lowered_logic_index: HashMap::new(),
            cached_script_entries: HashMap::new(),
            cached_room_order: Vec::new(),
            cached_step_event_blocks: HashMap::new(),
            cached_dispatch_tables: RuntimeEventDispatchTables::default(),
            cached_create_event_entries: HashMap::new(),
            cached_destroy_event_entries: HashMap::new(),
            cached_timeline_entries: HashMap::new(),
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
            pending_game_restart: false,
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
            host_bootstrap_scripts_applied: false,
            binary_files: RuntimeBinaryFileState::default(),
            tick_context: RuntimeTickContext::default(),
            random_state: Cell::new(0x4d59_5df4_d0f3_3173),
        };

        core.cached_room_order = core.runtime_room_order();
        core.rebuild_lowered_logic_caches();
        core.boot_default_room()?;
        core.package_bootstrap_globals = core.globals.clone();
        Ok(core)
    }

    pub(crate) fn rebuild_lowered_logic_caches(&mut self) {
        self.lowered_logic_index = self
            .package
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
        self.cached_script_entries = self.lowered_script_entries();
        self.cached_step_event_blocks = self.object_event_blocks_by_tag("step");
        self.cached_dispatch_tables =
            runtime_event_dispatch_tables(&self.package, &self.lowered_logic_index);
        self.cached_create_event_entries = self.lowered_event_entries_by_tag_for_runtime("create");
        self.cached_destroy_event_entries =
            self.lowered_event_entries_by_selector(RuntimeEventSelector::Destroy);
        self.cached_timeline_entries = self
            .lowered_logic_index
            .iter()
            .filter_map(|(block_id, entry_index)| {
                let suffix = block_id.strip_prefix("timeline:")?;
                let (timeline_id, moment) = suffix.split_once(':')?;
                Some((
                    timeline_id.parse::<usize>().ok()?,
                    moment.parse::<u32>().ok()?,
                    *entry_index,
                ))
            })
            .fold(
                HashMap::new(),
                |mut timelines, (timeline_id, moment, entry_index)| {
                    timelines
                        .entry(timeline_id)
                        .or_insert_with(Vec::new)
                        .push((moment, entry_index));
                    timelines
                },
            );
        for entries in self.cached_timeline_entries.values_mut() {
            entries.sort_by_key(|(moment, _)| *moment);
        }
        self.cached_collision_target_ids = self.collision_target_ids_by_object_id();
        self.cached_collision_matching_object_ids = self.collision_matching_object_ids_by_target();
        self.place_target_ids_by_name = self.compute_place_target_ids_by_name();
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

    pub fn current_room_speed(&self) -> Option<u32> {
        self.current_room.as_ref().map(|room| room.speed)
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
            room_speed: self.current_room_speed(),
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

    pub(crate) fn has_pending_scene_change(&self) -> bool {
        self.pending_game_restart
            || self.pending_room_reset
            || self.pending_room_transition.is_some()
    }

    fn apply_deferred_host_bootstrap_scripts<H: RuntimeHost>(&mut self, host: &mut H) {
        if self.host_bootstrap_scripts_applied {
            return;
        }
        self.host_bootstrap_scripts_applied = true;

        let script_calls = self.deferred_host_bootstrap_script_entry_indices();
        for (instance_idx, entry_index) in script_calls {
            self.apply_event_entry_indices_to_instance(
                host,
                instance_idx,
                &[entry_index],
                None,
                RuntimeEventSelector::Create,
                "deferred-create-script".into(),
                None,
            );
            if self.has_pending_scene_change() {
                break;
            }
        }
    }

    fn deferred_host_bootstrap_script_entry_indices(&self) -> Vec<(usize, usize)> {
        let Some(room) = self.current_room.as_ref() else {
            return Vec::new();
        };

        let mut calls = Vec::new();
        for (instance_idx, instance) in room.instances.iter().enumerate() {
            if !instance.alive {
                continue;
            }
            let Some(create_entries) = self.cached_create_event_entries.get(&instance.object_id)
            else {
                continue;
            };
            for create_entry in create_entries {
                for statement in &create_entry.statements {
                    let LoweredLogicStatement::FunctionCall { name, .. } = statement else {
                        continue;
                    };
                    let Some(script_entry) = self.cached_script_entries.get(name) else {
                        continue;
                    };
                    let mut seen_scripts = HashSet::new();
                    if !statements_reference_host_file_functions(
                        &script_entry.statements,
                        &self.cached_script_entries,
                        &mut seen_scripts,
                    ) {
                        continue;
                    }
                    let Some(script_entry_index) = self
                        .lowered_logic_index
                        .get(&script_entry.block_id)
                        .copied()
                    else {
                        continue;
                    };
                    let call = (instance_idx, script_entry_index);
                    if !calls.contains(&call) {
                        calls.push(call);
                    }
                }
            }
        }

        calls
    }

    pub fn render<H: RuntimeHost>(&mut self, host: &mut H) -> Result<(), RuntimeCoreError> {
        self.settle_current_room_before_first_render(host)?;
        self.sync_current_room_views_from_globals();
        let draw_commands = self.execute_draw_events(host)?;
        let frame = self.build_render_frame(draw_commands)?;
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
        self.update_jump_input_trace(host, jump);

        self.tick += 1;
        self.status = RuntimeStatus::Running;

        tick_phases.input_diag_nanos += mark_phase_elapsed(host, &mut phase_start);

        self.apply_pending_room_change(host)?;
        self.apply_deferred_host_bootstrap_scripts(host);
        if self.has_pending_scene_change() {
            self.apply_pending_room_change(host)?;
        }
        self.room_needs_first_render_settle = false;
        tick_phases.view_sync_nanos += mark_phase_elapsed(host, &mut phase_start);

        self.process_timelines(host)?;
        if self.has_pending_scene_change() {
            self.apply_pending_room_change(host)?;
        }

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

        self.dispatch_outside_room_events(host)?;

        self.dispatch_collision_events(host)?;
        tick_phases.collision_events_nanos += mark_phase_elapsed(host, &mut phase_start);

        self.detect_player_hazard_after_collision_events(host)?;

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

        if restart.just_pressed && !self.has_pending_scene_change() {
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

        if self.has_pending_scene_change() {
            self.apply_pending_room_change(host)?;
        }
        tick_phases.view_sync_nanos += mark_phase_elapsed(host, &mut phase_start);

        let animation_end_indices = self.advance_instance_sprite_animations();
        self.dispatch_animation_end_events(host, animation_end_indices)?;
        if self.has_pending_scene_change() {
            self.apply_pending_room_change(host)?;
        }

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

        self.apply_deferred_host_bootstrap_scripts(host);
        if self.has_pending_scene_change() {
            self.apply_pending_room_change(host)?;
            return Ok(());
        }

        let Some(room) = self.current_room.as_ref() else {
            return Err(RuntimeCoreError::NoRooms);
        };
        if room.instances.is_empty() {
            return Ok(());
        }

        let button_states = button_states_without_transitions(host);
        let step_result =
            self.execute_lowered_step_events_with_button_states(host, &button_states)?;
        self.sync_current_room_views_from_globals();

        if step_result.interrupted {
            self.apply_pending_room_change(host)?;
            return Ok(());
        }

        if self.has_pending_scene_change() {
            self.apply_pending_room_change(host)?;
            return Ok(());
        }

        self.dispatch_collision_events(host)?;
        if self.has_pending_scene_change() {
            self.apply_pending_room_change(host)?;
            return Ok(());
        }

        self.process_alarm_countdowns(host)?;
        if self.has_pending_scene_change() {
            self.apply_pending_room_change(host)?;
        }

        Ok(())
    }

    fn sync_current_room_views_from_globals(&mut self) {
        if let Some(room) = self.current_room.as_mut() {
            crate::logic::apply_view_globals_to_room(room, &self.globals);
        }
    }

    fn advance_instance_sprite_animations(&mut self) -> Vec<usize> {
        let Some(room) = self.current_room.as_mut() else {
            return Vec::new();
        };

        let mut animation_end_indices = Vec::new();
        for (instance_index, instance) in room
            .instances
            .iter_mut()
            .enumerate()
            .filter(|(_, instance)| instance.alive)
        {
            let sprite_id = instance
                .vars
                .get("sprite_index")
                .and_then(as_number)
                .map(|value| value.round() as i32)
                .unwrap_or_else(|| {
                    self.object_index
                        .get(&instance.object_id)
                        .and_then(|index| self.package.objects.get(*index))
                        .map(|object| object.sprite_index)
                        .unwrap_or(-1)
                });
            if sprite_id < 0 {
                continue;
            }

            let Some(frame_count) = self
                .sprite_index
                .get(&(sprite_id as usize))
                .and_then(|index| self.package.resources.sprites.get(*index))
                .map(|sprite| sprite.frame_paths.len())
                .filter(|count| *count > 0)
            else {
                continue;
            };

            let image_speed = instance
                .vars
                .get("image_speed")
                .and_then(as_number)
                .unwrap_or(1.0);
            if !image_speed.is_finite() || image_speed == 0.0 {
                continue;
            }

            let image_index = instance
                .vars
                .get("image_index")
                .and_then(as_number)
                .unwrap_or(0.0);
            let advanced_index = image_index + image_speed;
            let next_index = advanced_index.rem_euclid(frame_count as f64);
            instance
                .vars
                .insert("image_index".into(), RuntimeValue::Number(next_index));
            if (image_speed > 0.0 && advanced_index >= frame_count as f64)
                || (image_speed < 0.0 && advanced_index < 0.0)
            {
                animation_end_indices.push(instance_index);
            }
        }

        animation_end_indices
    }

    fn dispatch_animation_end_events<H: RuntimeHost>(
        &mut self,
        host: &mut H,
        instance_indices: Vec<usize>,
    ) -> Result<(), RuntimeCoreError> {
        for instance_idx in instance_indices {
            let Some(instance) = self
                .current_room
                .as_ref()
                .and_then(|room| room.instances.get(instance_idx))
                .filter(|instance| instance.alive)
            else {
                continue;
            };
            let block_ids = object_event_block_ids(
                &self.package,
                instance.object_id,
                RuntimeEventSelector::OtherAnimationEnd,
            );
            if block_ids.is_empty() {
                continue;
            }
            self.apply_event_blocks_to_instance(
                host,
                instance_idx,
                &block_ids,
                None,
                RuntimeEventSelector::OtherAnimationEnd,
                selector_event_tag(&RuntimeEventSelector::OtherAnimationEnd),
                None,
            );
            if self.has_pending_scene_change() {
                break;
            }
        }

        Ok(())
    }

    fn bound_button_state<H: RuntimeHost>(
        &self,
        host: &H,
        binding_key: &str,
        fallback_key_code: u16,
    ) -> ButtonState {
        let key_code = self.bound_key_code(binding_key, fallback_key_code);
        host.button_state(RuntimeButton::Keyboard(key_code))
    }

    fn bound_key_code(&self, binding_key: &str, fallback_key_code: u16) -> u16 {
        let key_code = self
            .globals
            .get(binding_key)
            .and_then(as_number)
            .map(|value| value.round() as u16)
            .unwrap_or(fallback_key_code);
        key_code
    }

    fn bound_restart_button_state<H: RuntimeHost>(&self, host: &H) -> ButtonState {
        let semantic_restart = host.button_state(RuntimeButton::Restart);
        if semantic_restart.pressed
            || semantic_restart.just_pressed
            || semantic_restart.just_released
        {
            return semantic_restart;
        }

        let Some(key_code) = self
            .globals
            .get("global.restartbutton")
            .or_else(|| self.globals.get("global.resetbutton"))
            .and_then(as_number)
            .map(|value| value.round() as u16)
        else {
            return ButtonState::default();
        };
        host.button_state(RuntimeButton::Keyboard(key_code))
    }

    pub fn reload_room(&mut self, room_id: usize) -> Result<(), RuntimeCoreError> {
        let persistent_instances = self
            .persistent_instances_for_room_transition()
            .into_iter()
            .filter(|instance| !is_player_instance(instance))
            .collect::<Vec<_>>();
        self.hydrate_missing_package_bootstrap_globals(room_id);
        let (mut room, source_room) = self.build_room_layout(room_id)?;
        self.apply_create_logic_with_visible_instances(
            &mut room,
            &source_room,
            &persistent_instances,
        );
        crate::room_transitions::add_persistent_instances(&mut room, persistent_instances);
        self.apply_room_start_logic_with_visible_instances(&mut room, &[]);
        self.current_room = Some(room);
        self.room_needs_first_render_settle = true;
        self.death_waiting_for_restart = false;
        self.status = RuntimeStatus::Ready;
        Ok(())
    }

    pub(crate) fn apply_current_room_startup_events<H: RuntimeHost>(
        &mut self,
        host: &mut H,
        source_room: &RoomDefinition,
    ) -> Result<(), RuntimeCoreError> {
        for placement in &source_room.instances {
            let Some(instance_idx) = self.current_room.as_ref().and_then(|room| {
                room.instances
                    .iter()
                    .position(|instance| instance.instance_id == placement.instance_id)
            }) else {
                continue;
            };
            if let Some(block_id) = placement.creation_block_id.clone() {
                self.apply_event_blocks_to_instance(
                    host,
                    instance_idx,
                    &[block_id],
                    None,
                    RuntimeEventSelector::Create,
                    "instance-create".into(),
                    None,
                );
                if self.has_pending_scene_change() {
                    return Ok(());
                }
            }

            let block_ids = {
                let Some(room) = self.current_room.as_ref() else {
                    return Err(RuntimeCoreError::NoRooms);
                };
                let Some(instance) = room.instances.get(instance_idx) else {
                    continue;
                };
                object_event_block_ids(
                    &self.package,
                    instance.object_id,
                    RuntimeEventSelector::Create,
                )
            };
            if !block_ids.is_empty() {
                self.apply_event_blocks_to_instance(
                    host,
                    instance_idx,
                    &block_ids,
                    None,
                    RuntimeEventSelector::Create,
                    "create".into(),
                    None,
                );
                if self.has_pending_scene_change() {
                    return Ok(());
                }
            }
        }

        let room_start_indices = self
            .current_room
            .as_ref()
            .map(|room| {
                room.instances
                    .iter()
                    .enumerate()
                    .filter_map(|(index, instance)| instance.alive.then_some(index))
                    .collect::<Vec<_>>()
            })
            .ok_or(RuntimeCoreError::NoRooms)?;
        for instance_idx in room_start_indices {
            let block_ids = {
                let Some(room) = self.current_room.as_ref() else {
                    return Err(RuntimeCoreError::NoRooms);
                };
                let Some(instance) = room.instances.get(instance_idx) else {
                    continue;
                };
                object_event_block_ids(
                    &self.package,
                    instance.object_id,
                    RuntimeEventSelector::OtherRoomStart,
                )
            };
            if block_ids.is_empty() {
                continue;
            }
            self.apply_event_blocks_to_instance(
                host,
                instance_idx,
                &block_ids,
                None,
                RuntimeEventSelector::OtherRoomStart,
                "other:room-start".into(),
                None,
            );
            if self.has_pending_scene_change() {
                return Ok(());
            }
        }

        Ok(())
    }

    pub(crate) fn execute_event_blocks<H: RuntimeHost>(
        &mut self,
        host: &mut H,
        selector: RuntimeEventSelector,
    ) -> Result<(), RuntimeCoreError> {
        let event_tag = selector_event_tag(&selector);
        let entry_lookups: Vec<(usize, Vec<usize>)> = {
            let Some(room) = self.current_room.as_ref() else {
                return Err(RuntimeCoreError::NoRooms);
            };
            room.instances
                .iter()
                .enumerate()
                .filter(|(_, i)| i.alive)
                .filter_map(|(idx, instance)| {
                    self.cached_entry_indices_for_selector(instance.object_id, &selector)
                        .map(|entry_indices| (idx, entry_indices.to_vec()))
                })
                .collect()
        };

        for (instance_idx, entry_indices) in entry_lookups {
            self.apply_event_entry_indices_to_instance(
                host,
                instance_idx,
                &entry_indices,
                None,
                selector.clone(),
                event_tag.clone(),
                None,
            );
            if self.has_pending_scene_change() {
                break;
            }
        }

        Ok(())
    }

    fn process_timelines<H: RuntimeHost>(&mut self, host: &mut H) -> Result<(), RuntimeCoreError> {
        let plans = {
            let Some(room) = self.current_room.as_ref() else {
                return Err(RuntimeCoreError::NoRooms);
            };
            room.instances
                .iter()
                .enumerate()
                .filter(|(_, instance)| instance.alive)
                .filter_map(|(instance_index, instance)| {
                    let running = instance
                        .vars
                        .get("timeline_running")
                        .and_then(as_number)
                        .is_some_and(|value| value != 0.0);
                    if !running {
                        return None;
                    }
                    let timeline_id = instance
                        .vars
                        .get("timeline_index")
                        .and_then(as_number)
                        .filter(|value| value.is_finite() && *value >= 0.0)?
                        .round() as usize;
                    let moments = self.cached_timeline_entries.get(&timeline_id)?;
                    let old_position = instance
                        .vars
                        .get("timeline_position")
                        .and_then(as_number)
                        .unwrap_or(0.0);
                    let speed = instance
                        .vars
                        .get("timeline_speed")
                        .and_then(as_number)
                        .unwrap_or(1.0);
                    let new_position = old_position + speed;
                    let timeline_length = moments
                        .last()
                        .map(|(moment, _)| *moment as f64)
                        .unwrap_or(0.0);
                    let looping = instance
                        .vars
                        .get("timeline_loop")
                        .and_then(as_number)
                        .is_some_and(|value| value != 0.0);
                    let stored_position = if new_position > timeline_length && looping {
                        if speed >= 0.0 {
                            0.0
                        } else {
                            timeline_length
                        }
                    } else {
                        new_position
                    };
                    let mut entry_indices = moments
                        .iter()
                        .filter(|(moment, _)| {
                            let moment = *moment as f64;
                            if speed > 0.0 {
                                moment >= old_position && moment < new_position
                            } else if speed < 0.0 {
                                moment <= old_position && moment > new_position
                            } else {
                                false
                            }
                        })
                        .map(|(_, entry_index)| *entry_index)
                        .collect::<Vec<_>>();
                    if speed < 0.0 {
                        entry_indices.reverse();
                    }
                    Some((instance_index, timeline_id, stored_position, entry_indices))
                })
                .collect::<Vec<_>>()
        };

        for (instance_index, timeline_id, stored_position, entry_indices) in plans {
            if let Some(instance) = self
                .current_room
                .as_mut()
                .and_then(|room| room.instances.get_mut(instance_index))
            {
                instance.vars.insert(
                    "timeline_position".into(),
                    RuntimeValue::Number(stored_position),
                );
            }
            if entry_indices.is_empty() {
                continue;
            }
            self.apply_event_entry_indices_to_instance(
                host,
                instance_index,
                &entry_indices,
                None,
                RuntimeEventSelector::Timeline,
                format!("timeline:{timeline_id}"),
                None,
            );
            if self.has_pending_scene_change() {
                break;
            }
        }
        Ok(())
    }

    fn dispatch_outside_room_events<H: RuntimeHost>(
        &mut self,
        host: &mut H,
    ) -> Result<(), RuntimeCoreError> {
        let selector = RuntimeEventSelector::OtherOutside;
        let lookups = {
            let Some(room) = self.current_room.as_ref() else {
                return Err(RuntimeCoreError::NoRooms);
            };
            room.instances
                .iter()
                .enumerate()
                .filter(|(_, instance)| instance.alive)
                .filter_map(|(instance_index, instance)| {
                    let (left, top, right, bottom) = bounds_at(instance, instance.x, instance.y);
                    let outside = right < 0
                        || bottom < 0
                        || left > room.width as i32
                        || top > room.height as i32;
                    if !outside {
                        return None;
                    }
                    self.cached_entry_indices_for_selector(instance.object_id, &selector)
                        .map(|entries| (instance_index, entries.to_vec()))
                })
                .collect::<Vec<_>>()
        };
        for (instance_index, entry_indices) in lookups {
            self.apply_event_entry_indices_to_instance(
                host,
                instance_index,
                &entry_indices,
                None,
                selector.clone(),
                "other:outside".into(),
                None,
            );
            if self.has_pending_scene_change() {
                break;
            }
        }
        Ok(())
    }

    fn cached_entry_indices_for_selector(
        &self,
        object_id: usize,
        selector: &RuntimeEventSelector,
    ) -> Option<&[usize]> {
        if let RuntimeEventSelector::Collision { target_object_id } = selector {
            return self
                .cached_dispatch_tables
                .collision_entry_indices_by_owner_and_target
                .get(&(object_id, *target_object_id))
                .map(Vec::as_slice);
        }

        let event_tag = selector_event_tag(selector);
        self.cached_dispatch_tables
            .entry_indices_by_event_tag_and_object_id
            .get(&event_tag)
            .and_then(|entries_by_object| entries_by_object.get(&object_id))
            .map(Vec::as_slice)
    }

    fn apply_event_blocks_to_instance<H: RuntimeHost>(
        &mut self,
        host: &mut H,
        instance_idx: usize,
        block_ids: &[String],
        other_instance: Option<RuntimeInstance>,
        event_selector: RuntimeEventSelector,
        event_tag: String,
        collision_spatial_index: Option<&RuntimeCollisionSpatialIndex>,
    ) {
        let entry_indices: Vec<usize> = block_ids
            .iter()
            .filter_map(|block_id| self.lowered_logic_index.get(block_id).copied())
            .collect();
        self.apply_event_entry_indices_to_instance(
            host,
            instance_idx,
            &entry_indices,
            other_instance,
            event_selector,
            event_tag,
            collision_spatial_index,
        );
    }

    fn apply_event_entry_indices_to_instance<H: RuntimeHost>(
        &mut self,
        host: &mut H,
        instance_idx: usize,
        entry_indices: &[usize],
        other_instance: Option<RuntimeInstance>,
        event_selector: RuntimeEventSelector,
        event_tag: String,
        collision_spatial_index: Option<&RuntimeCollisionSpatialIndex>,
    ) {
        let script_entries = &self.cached_script_entries;
        let destroy_event_entries = &self.cached_destroy_event_entries;
        let objects = &self.package.objects;
        let lowered_entries = self
            .package
            .lowered_logic
            .as_ref()
            .map(|logic| logic.entries.as_slice())
            .unwrap_or(&[]);
        let button_states = host
            .active_buttons()
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        let (current_room_id, mut current_room_speed) = {
            let Some(room) = self.current_room.as_ref() else {
                return;
            };
            (room.room_id, room.speed)
        };
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
        let mut instance_updates = RuntimeSparseInstanceOverlay::default();
        let room_instance_indices_by_object_id = {
            let Some(room) = self.current_room.as_ref() else {
                return;
            };
            runtime_instance_indices_by_object_id_from_instances(&room.instances)
        };

        for entry_index in entry_indices {
            let Some(entry) = lowered_entries.get(*entry_index) else {
                continue;
            };
            let event_owner_id =
                event_owner_id_for_block_id(objects, &entry.block_id).unwrap_or(instance.object_id);
            let mut scope = crate::logic::RuntimeExecutionScope::default();
            let mut with_updates = RuntimeSparseInstanceOverlay::default();
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
                    room_speed: current_room_speed,
                    room_width: room.width,
                    room_height: room.height,
                    random_state: &self.random_state,
                    button_states: &button_states,
                    room_instances: &room.instances,
                    room_instance_indices_by_object_id: &room_instance_indices_by_object_id,
                    object_index: None,
                    collision_spatial_index,
                    room_instance_overlay: eval_overlay,
                    room_order,
                    other_instance: other_instance.as_ref(),
                    other_runtime_id: other_instance.as_ref().map(|instance| instance.runtime_id),
                    place_target_ids_by_name: &self.place_target_ids_by_name,
                    room_ids_by_name: &self.room_ids_by_name,
                    view_zero: crate::logic::RuntimeViewValues::from_room(room),
                };
                let mut with_target_indices = Vec::new();
                let mut statement_env = crate::logic::RuntimeStatementEnvironment {
                    script_entries,
                    sound_index: &self.sound_index,
                    globals: &mut self.globals,
                    room_speed: &mut current_room_speed,
                    pending_room_transition: &mut self.pending_room_transition,
                    pending_room_reset: &mut self.pending_room_reset,
                    pending_game_restart: &mut self.pending_game_restart,
                    binary_files: &mut self.binary_files,
                    host: &mut *host,
                    diagnostics: &mut self.diagnostics,
                    object_query_scratch: None,
                    with_target_indices: &mut with_target_indices,
                    room_instance_updates: &mut with_updates,
                    room_instance_creates: &mut instance_creates,
                    objects,
                    sprites: &self.package.resources.sprites,
                    sprite_index: &self.sprite_index,
                    sprite_ids_by_name: &self.sprite_ids_by_name,
                    fonts: &self.package.resources.fonts,
                    font_index_by_name: &self.font_index_by_name,
                    lowered_entries,
                    event_selector: Some(event_selector.clone()),
                    event_owner_id: Some(event_owner_id),
                    draw: None,
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
                if self.has_pending_scene_change() {
                    break;
                }
            }
            crate::logic::commit_instance_updates(&mut instance_updates, &mut with_updates);
            if self.has_pending_scene_change() {
                break;
            }
        }

        if let Some(room) = self.current_room.as_mut() {
            room.speed = current_room_speed;
            for (update_index, updated_instance) in instance_updates.drain_dirty_updates() {
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
                RuntimeEventSelector::Alarm(slot),
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
        if self.current_room.is_none() {
            return Err(RuntimeCoreError::NoRooms);
        }

        let mut tick_context = std::mem::take(&mut self.tick_context);
        {
            let Some(room) = self.current_room.as_ref() else {
                return Err(RuntimeCoreError::NoRooms);
            };
            tick_context.clear_collision_hits();
            tick_context.rebuild_collision_spatial_index(room);
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
                        tick_context
                            .collision_spatial_index
                            .motion_candidate_indices_into(
                                *matching_object_id,
                                instance,
                                instance.x,
                                instance.y,
                                &mut tick_context.collision_scratch,
                            );
                        for &other_index in tick_context.collision_scratch.candidates() {
                            let Some(other) = room.instances.get(other_index) else {
                                continue;
                            };
                            if instance.runtime_id == other.runtime_id {
                                continue;
                            }
                            let swept_contact_y = (!other.hazard)
                                .then(|| swept_top_contact_y(instance, other))
                                .flatten();
                            if collides_at(
                                instance,
                                instance.x,
                                instance.y,
                                std::slice::from_ref(other),
                                Some(instance.runtime_id),
                            ) || swept_contact_y.is_some()
                            {
                                tick_context.collision_hits.push(RuntimeCollisionHit {
                                    instance_idx: instance_index,
                                    target_object_id,
                                    other_idx: other_index,
                                    solid_collision: instance.solid || other.solid,
                                    contact_y: swept_contact_y,
                                });
                            }
                        }
                    }
                }
            }
        };

        tick_context
            .collision_hits
            .sort_by_key(|hit| !hit.solid_collision);

        for hit in tick_context.collision_hits.iter().copied() {
            let mut collision_trace = Vec::new();
            let other_instance = {
                let Some(room) = self.current_room.as_mut() else {
                    continue;
                };
                if hit.instance_idx >= room.instances.len() || hit.other_idx >= room.instances.len()
                {
                    continue;
                }
                if !room.instances[hit.instance_idx].alive || !room.instances[hit.other_idx].alive {
                    continue;
                }
                {
                    // GM8 re-checks each pair live: an earlier hit's event may have
                    // already separated the instances (e.g. move_contact_solid), and
                    // rolling back on a stale hit would teleport the mover into the air.
                    let instance = &room.instances[hit.instance_idx];
                    let other = &room.instances[hit.other_idx];
                    let still_touching = hit.contact_y.is_some()
                        || collides_at(
                            instance,
                            instance.x,
                            instance.y,
                            std::slice::from_ref(other),
                            Some(instance.runtime_id),
                        );
                    if !still_touching {
                        continue;
                    }
                }
                // Resolve solidness from the live instances at dispatch time; earlier
                // collision events may have changed state since this hit was collected.
                let solid_collision =
                    room.instances[hit.instance_idx].solid || room.instances[hit.other_idx].solid;
                record_player_collision_trace(
                    &mut collision_trace,
                    self.tick,
                    "pre",
                    hit,
                    &room.instances[hit.instance_idx],
                    &room.instances[hit.other_idx],
                    solid_collision,
                    hit.contact_y,
                );
                if solid_collision {
                    if let Some(instance) = room.instances.get_mut(hit.instance_idx) {
                        instance.x = instance.previous_x;
                        instance.y = instance.previous_y;
                    }
                    if let Some(other) = room.instances.get_mut(hit.other_idx) {
                        other.x = other.previous_x;
                        other.y = other.previous_y;
                    }
                } else if let Some(contact_y) = hit.contact_y {
                    if let Some(instance) = room.instances.get_mut(hit.instance_idx) {
                        instance.y = contact_y as f64;
                    }
                }
                record_player_collision_trace(
                    &mut collision_trace,
                    self.tick,
                    if solid_collision {
                        "after-rollback"
                    } else {
                        "after-contact"
                    },
                    hit,
                    &room.instances[hit.instance_idx],
                    &room.instances[hit.other_idx],
                    solid_collision,
                    hit.contact_y,
                );
                room.instances.get(hit.other_idx).cloned()
            };
            let Some(other_instance) = other_instance else {
                continue;
            };
            let other_was_hazard = other_instance.hazard;
            let block_ids = {
                let Some(room) = self.current_room.as_ref() else {
                    continue;
                };
                let Some(instance) = room.instances.get(hit.instance_idx) else {
                    continue;
                };
                object_event_block_ids(
                    &self.package,
                    instance.object_id,
                    RuntimeEventSelector::Collision {
                        target_object_id: hit.target_object_id,
                    },
                )
            };
            self.apply_event_blocks_to_instance(
                host,
                hit.instance_idx,
                &block_ids,
                Some(other_instance),
                RuntimeEventSelector::Collision {
                    target_object_id: hit.target_object_id,
                },
                "collision".into(),
                Some(&tick_context.collision_spatial_index),
            );
            let scripted_hazard_death = self.current_room.as_ref().and_then(|room| {
                let instance = room.instances.get(hit.instance_idx)?;
                (other_was_hazard && instance.player_candidate && !instance.alive).then(|| {
                    format!(
                        "room={} tick={} object={} runtime_id={} x={} y={} reason=hazard message=player-hit-hazard-in-{}",
                        room.room_id,
                        self.tick,
                        instance.object_name,
                        instance.runtime_id,
                        instance.x,
                        instance.y,
                        room.room_name
                    )
                })
            });
            if let Some(message) = scripted_hazard_death {
                self.record_diagnostic(
                    host,
                    iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
                    "runtime-player-died",
                    message,
                );
            }
            let solid_collision = {
                let Some(room) = self.current_room.as_ref() else {
                    continue;
                };
                let solid_collision = room
                    .instances
                    .get(hit.instance_idx)
                    .zip(room.instances.get(hit.other_idx))
                    .map(|(instance, other)| instance.solid || other.solid)
                    .unwrap_or(hit.solid_collision);
                if let (Some(instance), Some(other)) = (
                    room.instances.get(hit.instance_idx),
                    room.instances.get(hit.other_idx),
                ) {
                    record_player_collision_trace(
                        &mut collision_trace,
                        self.tick,
                        "after-event",
                        hit,
                        instance,
                        other,
                        solid_collision,
                        hit.contact_y,
                    );
                }
                solid_collision
            };
            if solid_collision {
                // GM8 re-applies both instances' speeds after the event and only
                // rolls back again if they still overlap; this is how horizontal
                // motion survives a vertical solid collision.
                let mut still_overlapping = false;
                if let Some(room) = self.current_room.as_mut() {
                    if let Some(instance) = room.instances.get_mut(hit.instance_idx) {
                        if instance.alive {
                            instance.x += instance.hspeed;
                            instance.y += instance.vspeed;
                        }
                    }
                    if let Some(other) = room.instances.get_mut(hit.other_idx) {
                        if other.alive {
                            other.x += other.hspeed;
                            other.y += other.vspeed;
                        }
                    }
                    still_overlapping = match (
                        room.instances.get(hit.instance_idx),
                        room.instances.get(hit.other_idx),
                    ) {
                        (Some(instance), Some(other)) if instance.alive && other.alive => {
                            collides_at(
                                instance,
                                instance.x,
                                instance.y,
                                std::slice::from_ref(other),
                                Some(instance.runtime_id),
                            )
                        }
                        _ => false,
                    };
                    if let (Some(instance), Some(other)) = (
                        room.instances.get(hit.instance_idx),
                        room.instances.get(hit.other_idx),
                    ) {
                        record_player_collision_trace(
                            &mut collision_trace,
                            self.tick,
                            "after-reapply",
                            hit,
                            instance,
                            other,
                            solid_collision,
                            hit.contact_y,
                        );
                    }
                    if still_overlapping {
                        let resolution = room
                            .instances
                            .get(hit.instance_idx)
                            .zip(room.instances.get(hit.other_idx))
                            .map(|(instance, other)| {
                                final_solid_overlap_resolution(instance, other)
                            })
                            .unwrap_or(FinalSolidOverlapResolution::FullRollback);
                        if let Some(instance) = room.instances.get_mut(hit.instance_idx) {
                            match resolution {
                                FinalSolidOverlapResolution::KeepMotion => {}
                                FinalSolidOverlapResolution::PreserveHorizontal => {
                                    instance.y = instance.previous_y;
                                }
                                FinalSolidOverlapResolution::PreserveVertical => {
                                    instance.x = instance.previous_x;
                                }
                                FinalSolidOverlapResolution::FullRollback => {
                                    instance.x = instance.previous_x;
                                    instance.y = instance.previous_y;
                                }
                            }
                        }
                        if let Some(other) = room.instances.get_mut(hit.other_idx) {
                            match resolution {
                                FinalSolidOverlapResolution::KeepMotion => {}
                                FinalSolidOverlapResolution::PreserveHorizontal => {
                                    other.y = other.previous_y;
                                }
                                FinalSolidOverlapResolution::PreserveVertical => {
                                    other.x = other.previous_x;
                                }
                                FinalSolidOverlapResolution::FullRollback => {
                                    other.x = other.previous_x;
                                    other.y = other.previous_y;
                                }
                            }
                        }
                    }
                }
                if still_overlapping {
                    if let Some(room) = self.current_room.as_ref() {
                        if let (Some(instance), Some(other)) = (
                            room.instances.get(hit.instance_idx),
                            room.instances.get(hit.other_idx),
                        ) {
                            record_player_collision_trace(
                                &mut collision_trace,
                                self.tick,
                                "after-final-rollback",
                                hit,
                                instance,
                                other,
                                solid_collision,
                                hit.contact_y,
                            );
                        }
                    }
                }
            }
            for message in collision_trace {
                self.record_diagnostic(
                    host,
                    iwm_runtime_host::RuntimeDiagnosticLevel::Info,
                    "runtime-collision-trace",
                    message,
                );
            }
        }

        self.tick_context = tick_context;
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum FinalSolidOverlapResolution {
    KeepMotion,
    PreserveHorizontal,
    PreserveVertical,
    FullRollback,
}

fn final_solid_overlap_resolution(
    instance: &RuntimeInstance,
    other: &RuntimeInstance,
) -> FinalSolidOverlapResolution {
    let ignore_runtime_id = Some(instance.runtime_id);
    let (_, _, _, instance_bottom) = bounds_at(instance, instance.x, instance.y);
    let (_, other_top, _, _) = bounds_at(other, other.x, other.y);
    if instance.vspeed <= 0.0
        && instance_bottom - other_top == 1
        && !collides_with_instances_at(
            instance,
            (instance.previous_x, instance.previous_y),
            other,
            (other.previous_x, other.previous_y),
            ignore_runtime_id,
            |_| true,
        )
        && !collides_with_instances_at(
            instance,
            (instance.x, instance.y - 1.0),
            other,
            (other.x, other.y),
            ignore_runtime_id,
            |_| true,
        )
    {
        return FinalSolidOverlapResolution::KeepMotion;
    }

    if !collides_with_instances_at(
        instance,
        (instance.x, instance.previous_y),
        other,
        (other.x, other.previous_y),
        ignore_runtime_id,
        |_| true,
    ) {
        return FinalSolidOverlapResolution::PreserveHorizontal;
    }

    if !collides_with_instances_at(
        instance,
        (instance.previous_x, instance.y),
        other,
        (other.previous_x, other.y),
        ignore_runtime_id,
        |_| true,
    ) {
        return FinalSolidOverlapResolution::PreserveVertical;
    }

    FinalSolidOverlapResolution::FullRollback
}

fn record_player_collision_trace(
    messages: &mut Vec<String>,
    tick: u64,
    phase: &'static str,
    hit: RuntimeCollisionHit,
    instance: &RuntimeInstance,
    other: &RuntimeInstance,
    solid_collision: bool,
    contact_y: Option<i32>,
) {
    let relevant = (instance.player_candidate || other.player_candidate)
        && (solid_collision || instance.hazard || other.hazard)
        && (instance.vspeed < 0.0 || other.vspeed < 0.0 || instance.hazard || other.hazard);
    if messages.is_empty() && !relevant {
        return;
    }

    messages.push(format!(
        "tick={tick} phase={phase} owner={}#{} target={} other={}#{} solid={} contact_y={} pos=({:.3},{:.3}) prev=({:.3},{:.3}) speed=({:.3},{:.3}) other_pos=({:.3},{:.3}) other_speed=({:.3},{:.3}) flags=inst_solid:{} inst_hazard:{} other_solid:{} other_hazard:{}",
        instance.object_name,
        instance.runtime_id,
        hit.target_object_id,
        other.object_name,
        other.runtime_id,
        solid_collision,
        contact_y
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".into()),
        instance.x,
        instance.y,
        instance.previous_x,
        instance.previous_y,
        instance.hspeed,
        instance.vspeed,
        other.x,
        other.y,
        other.hspeed,
        other.vspeed,
        instance.solid,
        instance.hazard,
        other.solid,
        other.hazard,
    ));
}

fn parse_alarm_slot(key: &str) -> Option<u32> {
    key.strip_prefix("alarm[")?.strip_suffix(']')?.parse().ok()
}

fn swept_top_contact_y(instance: &RuntimeInstance, other: &RuntimeInstance) -> Option<i32> {
    if other.solid || instance.y <= instance.previous_y {
        return None;
    }

    let (left, _, right, bottom) = bounds_at(instance, instance.x, instance.y);
    let (_, _, _, previous_bottom) = bounds_at(instance, instance.previous_x, instance.previous_y);
    let (other_left, other_top, other_right, _) = bounds_at(other, other.x, other.y);

    (left < other_right
        && right > other_left
        && previous_bottom <= other_top
        && bottom >= other_top)
        .then_some(other_top + instance.origin_y - instance.bbox_bottom - 1)
}

fn statements_reference_host_file_functions(
    statements: &[LoweredLogicStatement],
    script_entries: &HashMap<String, LoweredLogicEntry>,
    seen_scripts: &mut HashSet<String>,
) -> bool {
    statements.iter().any(|statement| {
        statement_references_host_file_functions(statement, script_entries, seen_scripts)
    })
}

fn statement_references_host_file_functions(
    statement: &LoweredLogicStatement,
    script_entries: &HashMap<String, LoweredLogicEntry>,
    seen_scripts: &mut HashSet<String>,
) -> bool {
    match statement {
        LoweredLogicStatement::Assignment { target, value } => {
            expr_references_host_file_functions(target, script_entries, seen_scripts)
                || expr_references_host_file_functions(value, script_entries, seen_scripts)
        }
        LoweredLogicStatement::Conditional {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_references_host_file_functions(condition, script_entries, seen_scripts)
                || statements_reference_host_file_functions(
                    then_branch,
                    script_entries,
                    seen_scripts,
                )
                || statements_reference_host_file_functions(
                    else_branch,
                    script_entries,
                    seen_scripts,
                )
        }
        LoweredLogicStatement::FunctionCall { name, args } => {
            is_host_file_function(name)
                || args.iter().any(|arg| {
                    expr_references_host_file_functions(arg, script_entries, seen_scripts)
                })
                || script_entries.get(name).is_some_and(|entry| {
                    if !seen_scripts.insert(name.clone()) {
                        return false;
                    }
                    statements_reference_host_file_functions(
                        &entry.statements,
                        script_entries,
                        seen_scripts,
                    )
                })
        }
        LoweredLogicStatement::With { target, body } => {
            expr_references_host_file_functions(target, script_entries, seen_scripts)
                || statements_reference_host_file_functions(body, script_entries, seen_scripts)
        }
        LoweredLogicStatement::For {
            init,
            condition,
            step,
            body,
        } => {
            expr_references_host_file_functions(init, script_entries, seen_scripts)
                || expr_references_host_file_functions(condition, script_entries, seen_scripts)
                || expr_references_host_file_functions(step, script_entries, seen_scripts)
                || statements_reference_host_file_functions(body, script_entries, seen_scripts)
        }
        LoweredLogicStatement::Repeat { count, body } => {
            expr_references_host_file_functions(count, script_entries, seen_scripts)
                || statements_reference_host_file_functions(body, script_entries, seen_scripts)
        }
        LoweredLogicStatement::While { condition, body } => {
            expr_references_host_file_functions(condition, script_entries, seen_scripts)
                || statements_reference_host_file_functions(body, script_entries, seen_scripts)
        }
        LoweredLogicStatement::VariableDeclaration { .. }
        | LoweredLogicStatement::Return { .. }
        | LoweredLogicStatement::Raw { .. } => false,
    }
}

fn expr_references_host_file_functions(
    expr: &LoweredLogicExpr,
    script_entries: &HashMap<String, LoweredLogicEntry>,
    seen_scripts: &mut HashSet<String>,
) -> bool {
    match expr {
        LoweredLogicExpr::Call { name, args } => {
            is_host_file_function(name)
                || args.iter().any(|arg| {
                    expr_references_host_file_functions(arg, script_entries, seen_scripts)
                })
                || script_entries.get(name).is_some_and(|entry| {
                    if !seen_scripts.insert(name.clone()) {
                        return false;
                    }
                    statements_reference_host_file_functions(
                        &entry.statements,
                        script_entries,
                        seen_scripts,
                    )
                })
        }
        LoweredLogicExpr::UnaryExpr { child, .. } => {
            expr_references_host_file_functions(child, script_entries, seen_scripts)
        }
        LoweredLogicExpr::BinaryExpr { left, right, .. } => {
            expr_references_host_file_functions(left, script_entries, seen_scripts)
                || expr_references_host_file_functions(right, script_entries, seen_scripts)
        }
        LoweredLogicExpr::MemberAccess { target, .. } => {
            expr_references_host_file_functions(target, script_entries, seen_scripts)
        }
        LoweredLogicExpr::IndexAccess { target, index } => {
            expr_references_host_file_functions(target, script_entries, seen_scripts)
                || expr_references_host_file_functions(index, script_entries, seen_scripts)
        }
        LoweredLogicExpr::Identifier(_)
        | LoweredLogicExpr::LiteralNumber(_)
        | LoweredLogicExpr::LiteralBool(_)
        | LoweredLogicExpr::LiteralText(_)
        | LoweredLogicExpr::Raw { .. } => false,
    }
}

fn is_host_file_function(name: &str) -> bool {
    matches!(
        name,
        "file_exists"
            | "file_bin_open"
            | "file_bin_read_byte"
            | "file_bin_write_byte"
            | "file_bin_close"
            | "file_delete"
    )
}

fn button_states_without_transitions<H: RuntimeHost>(
    host: &H,
) -> HashMap<RuntimeButton, ButtonState> {
    let mut button_states = host.active_buttons().into_iter().collect::<HashMap<_, _>>();
    for state in button_states.values_mut() {
        state.just_pressed = false;
        state.just_released = false;
    }
    button_states
}

fn selector_event_tag(selector: &RuntimeEventSelector) -> String {
    match selector {
        RuntimeEventSelector::Create => "create".into(),
        RuntimeEventSelector::Step => "step".into(),
        RuntimeEventSelector::Draw => "draw".into(),
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
        RuntimeEventSelector::OtherAnimationEnd => "other:animation-end".into(),
        RuntimeEventSelector::OtherRoomStart => "other:room-start".into(),
        RuntimeEventSelector::OtherOutside => "other:outside".into(),
        RuntimeEventSelector::Timeline => "timeline".into(),
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
