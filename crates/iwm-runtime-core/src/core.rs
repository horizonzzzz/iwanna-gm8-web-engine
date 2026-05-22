use std::collections::HashMap;

use iwm_runtime_host::{RuntimeButton, RuntimeHost};

use crate::helpers::is_player_instance;
use crate::{
    RuntimeCoreError, RuntimePackage, RuntimePlayerSnapshot, RuntimeRoomState, RuntimeSnapshot,
    RuntimeStatus, RuntimeValue,
};

#[derive(Debug)]
pub struct RuntimeCore {
    pub(crate) package: RuntimePackage,
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
}
