use std::collections::HashMap;

use iwm_runtime_host::{RuntimeButton, RuntimeHost};

use crate::event_dispatch::{object_event_block_ids, RuntimeEventSelector};
use crate::helpers::is_player_instance;
use crate::{
    LoweredLogicEntry, LoweredLogicStatement, RuntimeCoreError, RuntimePackage,
    RuntimePlayerSnapshot, RuntimeRoomState, RuntimeSnapshot, RuntimeStatus, RuntimeValue,
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
        };

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
                room.instances
                    .iter()
                    .find(|instance| is_player_instance(instance))
                    .map(|instance| RuntimePlayerSnapshot {
                        x: instance.x,
                        y: instance.y,
                        hspeed: instance.hspeed,
                        vspeed: instance.vspeed,
                    })
            }),
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

        let left = host.button_state(RuntimeButton::Keyboard(0x25));
        let right = host.button_state(RuntimeButton::Keyboard(0x27));
        let jump = host.button_state(RuntimeButton::Keyboard(0x20));
        let restart = host.button_state(RuntimeButton::Keyboard(0x52));

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

        if restart.just_pressed {
            self.pending_room_reset = true;
            self.apply_pending_room_change()?;
            self.render(host)?;
            return Ok(());
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
            if self.execute_lowered_step_events(host)? {
                self.apply_pending_room_change()?;
                self.render(host)?;
                return Ok(());
            }
            self.step_player(host, left.pressed, right.pressed, jump.just_pressed)?;
        }

        // Dispatch alarm events (countdown alarm state)
        self.process_alarm_countdowns(host)?;

        // Dispatch keyboard events for any currently pressed key exposed by the host.
        for (button, state) in host.active_buttons() {
            if let RuntimeButton::Keyboard(key) = button {
                if state.pressed {
                    self.execute_event_blocks(host, RuntimeEventSelector::Keyboard(key))?;
                }
            }
        }

        if self.pending_room_reset || self.pending_room_transition.is_some() {
            self.apply_pending_room_change()?;
        }

        self.render(host)?;
        Ok(())
    }

    pub fn reload_room(&mut self, room_id: usize) -> Result<(), RuntimeCoreError> {
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
            self.apply_event_blocks_to_instance(host, instance_idx, &block_ids);
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
                &mut self.globals,
                &mut self.pending_room_transition,
                &mut self.pending_room_reset,
                host,
                &mut self.diagnostics,
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
            self.apply_event_blocks_to_instance(host, instance_idx, &block_ids);
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
