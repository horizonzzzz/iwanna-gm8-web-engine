use std::collections::HashMap;

use iwm_runtime_host::{ButtonState, RuntimeButton, RuntimeHost};

use crate::event_dispatch::{
    collision_event_target_object_ids, object_event_block_ids, RuntimeEventSelector,
};
use crate::helpers::{as_number, collides_at, is_player_instance};
use crate::{
    LoweredLogicEntry, LoweredLogicStatement, RuntimeCoreError, RuntimePackage,
    RuntimeInputTraceSnapshot, RuntimeInstance, RuntimeJumpSnapshot, RuntimePlayerSnapshot, RuntimeRoomState,
    RuntimeSnapshot, RuntimeStatus, RuntimeValue,
};

#[derive(Debug)]
pub struct RuntimeCore {
    pub(crate) package: RuntimePackage,
    pub(crate) object_index: HashMap<usize, usize>,
    pub(crate) room_index: HashMap<usize, usize>,
    pub(crate) lowered_logic_index: HashMap<String, usize>,
    pub(crate) current_room: Option<RuntimeRoomState>,
    pub(crate) status: RuntimeStatus,
    pub(crate) tick: u64,
    pub(crate) diagnostics: Vec<iwm_runtime_host::RuntimeDiagnostic>,
    pub(crate) pending_room_transition: Option<usize>,
    pub(crate) pending_room_reset: bool,
    pub(crate) globals: HashMap<String, RuntimeValue>,
    pub(crate) package_bootstrap_globals: HashMap<String, RuntimeValue>,
    pub(crate) last_input_trace: RuntimeInputTraceSnapshot,
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

        let mut core = Self {
            package,
            object_index,
            room_index,
            lowered_logic_index,
            current_room: None,
            status: RuntimeStatus::Ready,
            tick: 0,
            diagnostics: Vec::new(),
            pending_room_transition: None,
            pending_room_reset: false,
            globals: HashMap::new(),
            package_bootstrap_globals: HashMap::new(),
            last_input_trace: RuntimeInputTraceSnapshot {
                jump_button_key: 0x20,
                jump_pressed: false,
                jump_just_pressed: false,
                jump_just_released: false,
                active_keys: Vec::new(),
            },
        };

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
                        x: instance.x,
                        y: instance.y,
                        hspeed: instance.hspeed,
                        vspeed: instance.vspeed,
                        facing_left: instance.facing_left,
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
            diagnostics: self.diagnostics.clone(),
        }
    }

    pub fn request_room_transition(&mut self, room_id: usize) {
        self.pending_room_transition = Some(room_id);
    }

    pub fn render<H: RuntimeHost>(&mut self, host: &mut H) -> Result<(), RuntimeCoreError> {
        let frame = self.build_render_frame()?;
        host.submit_frame(frame)?;
        Ok(())
    }

    pub fn tick<H: RuntimeHost>(&mut self, host: &mut H) -> Result<(), RuntimeCoreError> {
        if self.current_room.is_none() {
            self.status = RuntimeStatus::Error;
            return Err(RuntimeCoreError::NoRooms);
        }

        let left = self.bound_button_state(host, "global.leftbutton", 0x25);
        let right = self.bound_button_state(host, "global.rightbutton", 0x27);
        let mut jump = self.bound_button_state(host, "global.jumpbutton", 0x20);
        let restart = host.button_state(RuntimeButton::Keyboard(0x52));
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

        self.apply_pending_room_change()?;

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
        } else {
            let step_result = self.execute_lowered_step_events(host)?;
            if step_result.interrupted {
                self.apply_pending_room_change()?;
                self.render(host)?;
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
        }

        self.dispatch_collision_events(host)?;

        // Dispatch alarm events (countdown alarm state)
        self.process_alarm_countdowns(host)?;

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
            self.pending_room_reset = true;
        }

        if self.pending_room_reset || self.pending_room_transition.is_some() {
            self.apply_pending_room_change()?;
        }

        self.render(host)?;
        Ok(())
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

    pub fn reload_room(&mut self, room_id: usize) -> Result<(), RuntimeCoreError> {
        self.hydrate_missing_package_bootstrap_globals();
        self.current_room = Some(self.build_room(room_id)?);
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
                    let block_ids = object_event_block_ids(
                        &self.package,
                        instance.object_id,
                        selector.clone(),
                    );
                    if block_ids.is_empty() {
                        None
                    } else {
                        Some((idx, block_ids))
                    }
                })
                .collect()
        };

        for (instance_idx, block_ids) in block_lookups {
            self.apply_event_blocks_to_instance(host, instance_idx, &block_ids, None);
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
    ) {
        // Clone entries first to avoid borrow conflicts
        let entries: Vec<LoweredLogicEntry> = block_ids
            .iter()
            .filter_map(|block_id| self.lowered_logic_entry(block_id).cloned())
            .collect();

        let statements: Vec<LoweredLogicStatement> = entries
            .iter()
            .flat_map(|entry| entry.statements.clone())
            .collect();
        let script_entries = self.lowered_script_entries();
        let button_states = host
            .active_buttons()
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        let room_instances = {
            let Some(room) = self.current_room.as_ref() else {
                return;
            };
            room.instances.clone()
        };
        let room_order = self.package.rooms.iter().map(|room| room.id).collect::<Vec<_>>();
        let current_room_id = {
            let Some(room) = self.current_room.as_ref() else {
                return;
            };
            room.room_id
        };
        let known_files = crate::logic::sample_known_files(host);
        let eval_context = crate::logic::RuntimeEvalContext {
            current_room_id,
            button_states: &button_states,
            room_instances: &room_instances,
            room_order: &room_order,
            objects: &self.package.objects,
            known_files: &known_files,
            other_instance: other_instance.as_ref(),
        };

        let mut instance = {
            let Some(room) = self.current_room.as_ref() else {
                return;
            };
            let Some(instance) = room.instances.get(instance_idx) else {
                return;
            };
            instance.clone()
        };

        for statement in &statements {
            crate::logic::apply_runtime_statement(
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
                break;
            }
        }

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
                    instance
                        .vars
                        .iter()
                        .filter_map(move |(key, value)| match (parse_alarm_slot(key), value) {
                            (Some(slot), RuntimeValue::Number(ticks)) if *ticks > 0.0 && *ticks <= 1.0 => {
                                Some((idx, slot))
                            }
                            _ => None,
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
            self.apply_event_blocks_to_instance(host, instance_idx, &block_ids, None);
        }

        Ok(())
    }

    fn dispatch_collision_events<H: RuntimeHost>(
        &mut self,
        host: &mut H,
    ) -> Result<(), RuntimeCoreError> {
        let collisions = {
            let Some(room) = self.current_room.as_ref() else {
                return Err(RuntimeCoreError::NoRooms);
            };
            let mut hits = Vec::new();
            for instance in &room.instances {
                if !instance.alive {
                    continue;
                }
                let target_object_ids = collision_event_target_object_ids(&self.package, instance.object_id);
                if target_object_ids.is_empty() {
                    continue;
                }
                for target_object_id in target_object_ids {
                    for other in &room.instances {
                        if !other.alive
                            || instance.runtime_id == other.runtime_id
                            || other.object_id != target_object_id
                        {
                            continue;
                        }
                        if crate::helpers::collides_at(
                            instance,
                            instance.x,
                            instance.y,
                            std::slice::from_ref(other),
                            Some(instance.runtime_id),
                        ) {
                            hits.push((instance.runtime_id, target_object_id, other.clone()));
                        }
                    }
                }
            }
            hits
        };

        for (instance_idx, target_object_id, other_instance) in collisions {
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
            self.apply_event_blocks_to_instance(host, instance_idx, &block_ids, Some(other_instance));
        }

        Ok(())
    }
}

fn parse_alarm_slot(key: &str) -> Option<u32> {
    key.strip_prefix("alarm[")?
        .strip_suffix(']')?
        .parse()
        .ok()
}
