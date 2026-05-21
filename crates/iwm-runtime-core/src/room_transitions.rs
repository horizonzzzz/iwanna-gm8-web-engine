use crate::helpers::is_player_instance;
use crate::{RuntimeCore, RuntimeCoreError, RuntimeStatus};

impl RuntimeCore {
    pub(crate) fn apply_pending_room_change(&mut self) -> Result<(), RuntimeCoreError> {
        if self.pending_room_reset {
            let room_id = self
                .current_room
                .as_ref()
                .map(|room| room.room_id)
                .ok_or(RuntimeCoreError::NoRooms)?;
            self.pending_room_reset = false;
            self.current_room = Some(self.build_room(room_id)?);
            self.reset_player_to_spawn();
            self.status = RuntimeStatus::Ready;
        }

        if let Some(room_id) = self.pending_room_transition.take() {
            self.current_room = Some(self.build_room(room_id)?);
            self.status = RuntimeStatus::Ready;
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

        if let Some(player) = room
            .instances
            .iter_mut()
            .find(|instance| is_player_instance(instance))
        {
            player.x = spawn_x;
            player.y = spawn_y;
            player.previous_x = spawn_x;
            player.previous_y = spawn_y;
            player.hspeed = 0;
            player.vspeed = 0;
        }
    }
}
