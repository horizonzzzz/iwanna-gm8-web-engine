use iwm_runtime_host::RuntimeHost;

use crate::helpers::{
    as_number, collides_at, is_player_instance, move_instance_axis, player_out_of_bounds, Axis,
};
use crate::{RuntimeCore, RuntimeCoreError};

const RUN_SPEED: i32 = 4;
const JUMP_SPEED: i32 = 8;
const GRAVITY: i32 = 1;
const MAX_FALL_SPEED: i32 = 8;

impl RuntimeCore {
    pub(crate) fn step_player<H: RuntimeHost>(
        &mut self,
        host: &mut H,
        left_pressed: bool,
        right_pressed: bool,
        jump_just_pressed: bool,
    ) -> Result<(), RuntimeCoreError> {
        let Some(room) = self.current_room.as_ref() else {
            return Err(RuntimeCoreError::NoRooms);
        };

        let player_index = room.instances.iter().position(is_player_instance);
        let Some(player_index) = player_index else {
            return Ok(());
        };

        let room_width = room.width;
        let room_height = room.height;
        let room_name = room.room_name.clone();
        let solids = room
            .instances
            .iter()
            .filter(|instance| instance.alive && instance.solid)
            .cloned()
            .collect::<Vec<_>>();
        let hazards = room
            .instances
            .iter()
            .filter(|instance| instance.alive && instance.hazard)
            .cloned()
            .collect::<Vec<_>>();

        let room = self.current_room.as_mut().ok_or(RuntimeCoreError::NoRooms)?;
        let player = room
            .instances
            .get_mut(player_index)
            .ok_or(RuntimeCoreError::NoRooms)?;

        let run_speed = player
            .vars
            .get("moveSpeed")
            .and_then(as_number)
            .or_else(|| player.vars.get("maxSpeed").and_then(as_number))
            .unwrap_or(RUN_SPEED as f64)
            .round() as i32;
        let jump_speed = player
            .vars
            .get("jump")
            .and_then(as_number)
            .unwrap_or(JUMP_SPEED as f64)
            .round() as i32;
        let gravity = player
            .vars
            .get("gravity")
            .and_then(as_number)
            .unwrap_or(GRAVITY as f64)
            .round() as i32;
        let max_fall_speed = player
            .vars
            .get("maxFallSpeed")
            .and_then(as_number)
            .unwrap_or(MAX_FALL_SPEED as f64)
            .round() as i32;

        player.previous_x = player.x;
        player.previous_y = player.y;

        player.hspeed = match (left_pressed, right_pressed) {
            (true, false) => -run_speed,
            (false, true) => run_speed,
            _ => 0,
        };

        let standing_on_solid =
            collides_at(player, player.x, player.y + 1, &solids, Some(player.runtime_id));
        if jump_just_pressed && standing_on_solid {
            player.vspeed = -jump_speed;
        }

        player.vspeed = (player.vspeed + gravity).min(max_fall_speed);

        move_instance_axis(
            player,
            &solids,
            Some(player.runtime_id),
            Axis::Horizontal,
            player.hspeed,
        );
        move_instance_axis(
            player,
            &solids,
            Some(player.runtime_id),
            Axis::Vertical,
            player.vspeed,
        );

        if collides_at(player, player.x, player.y, &hazards, Some(player.runtime_id)) {
            self.record_diagnostic(
                host,
                iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
                "runtime-player-died",
                format!("player hit a hazard in {}", room_name),
            );
            self.pending_room_reset = true;
        } else if player_out_of_bounds(player, room_width, room_height)
            && !room.transition_targets.is_empty()
        {
            self.pending_room_transition = room.transition_targets.first().copied();
        }

        Ok(())
    }
}
