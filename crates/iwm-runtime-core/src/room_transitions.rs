use crate::helpers::{adjusted_spawn_for_player, is_player_instance};
use crate::types::RuntimeJumpState;
use crate::{RuntimeCore, RuntimeCoreError, RuntimeStatus};
use iwm_runtime_host::RuntimeHost;

impl RuntimeCore {
    pub(crate) fn apply_pending_room_change<H: RuntimeHost>(
        &mut self,
        host: &mut H,
    ) -> Result<(), RuntimeCoreError> {
        if self.pending_room_reset {
            let from_room_id = self
                .current_room
                .as_ref()
                .map(|room| room.room_id)
                .ok_or(RuntimeCoreError::NoRooms)?;
            let room_id = self
                .current_room
                .as_ref()
                .map(|room| room.room_id)
                .ok_or(RuntimeCoreError::NoRooms)?;
            self.pending_room_reset = false;
            self.current_room = Some(self.build_room(room_id)?);
            self.room_needs_first_render_settle = true;
            self.death_waiting_for_restart = false;
            self.reset_player_to_spawn();
            self.status = RuntimeStatus::Ready;
            self.record_diagnostic(
                host,
                iwm_runtime_host::RuntimeDiagnosticLevel::Info,
                "runtime-room-changed",
                format!(
                    "from_room={} to_room={} reason=restart tick={}",
                    from_room_id, room_id, self.tick
                ),
            );
        }

        if let Some(room_id) = self.pending_room_transition.take() {
            let from_room_id = self.current_room.as_ref().map(|room| room.room_id);
            self.current_room = Some(self.build_room(room_id)?);
            self.room_needs_first_render_settle = true;
            self.status = RuntimeStatus::Ready;
            self.record_diagnostic(
                host,
                iwm_runtime_host::RuntimeDiagnosticLevel::Info,
                "runtime-room-changed",
                format!(
                    "from_room={} to_room={} reason=transition tick={}",
                    from_room_id.unwrap_or(room_id),
                    room_id,
                    self.tick
                ),
            );
        }

        Ok(())
    }

    pub(crate) fn reset_player_to_spawn(&mut self) {
        let Some(room) = self.current_room.as_mut() else {
            return;
        };

        let Some((spawn_x, spawn_y)) = room.spawn_point else {
            return;
        };

        if let Some(player_index) = room.instances.iter().position(is_player_instance) {
            let (x, y) =
                adjusted_spawn_for_player(&room.instances[player_index], spawn_x, spawn_y, room);
            let player = &mut room.instances[player_index];
            player.x = x as f64;
            player.y = y as f64;
            player.previous_x = x as f64;
            player.previous_y = y as f64;
            player.set_hvspeed(0.0, 0.0);
            player.jump = RuntimeJumpState::default();
        }
    }
}
