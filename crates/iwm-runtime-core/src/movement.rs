use iwm_runtime_host::{ButtonState, RuntimeHost};

use crate::helpers::{
    as_number, collides_at, is_player_instance, move_instance_axis, player_out_of_bounds, Axis,
};
use crate::{RuntimeCore, RuntimeCoreError};

const RUN_SPEED: f64 = 4.0;
const JUMP_SPEED: f64 = 8.0;
const GRAVITY: f64 = 1.0;
const MAX_FALL_SPEED: f64 = 8.0;

impl RuntimeCore {
    pub(crate) fn step_player<H: RuntimeHost>(
        &mut self,
        host: &mut H,
        left_pressed: bool,
        right_pressed: bool,
        jump: ButtonState,
        enable_builtin_jump: bool,
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

        let room = self
            .current_room
            .as_mut()
            .ok_or(RuntimeCoreError::NoRooms)?;
        let player = room
            .instances
            .get_mut(player_index)
            .ok_or(RuntimeCoreError::NoRooms)?;

        let run_speed = player
            .vars
            .get("moveSpeed")
            .and_then(as_number)
            .or_else(|| player.vars.get("maxSpeed").and_then(as_number))
            .unwrap_or(RUN_SPEED);
        let jump_speed = player
            .vars
            .get("jump")
            .and_then(as_number)
            .unwrap_or(JUMP_SPEED);
        let jump_cut_speed = player
            .vars
            .get("jump2")
            .and_then(as_number)
            .unwrap_or((jump_speed - 1.0).max(1.0));
        let jump_hold_frames = player
            .vars
            .get("jumpHoldFrames")
            .and_then(as_number)
            .unwrap_or(4.0)
            .round() as u32;
        let gravity = player
            .vars
            .get("gravity")
            .and_then(as_number)
            .unwrap_or(GRAVITY);
        let max_fall_speed = player
            .vars
            .get("maxFallSpeed")
            .and_then(as_number)
            .unwrap_or(MAX_FALL_SPEED);

        player.previous_x = player.x;
        player.previous_y = player.y;

        player.hspeed = match (left_pressed, right_pressed) {
            (true, false) => {
                player.facing_left = true;
                -run_speed
            }
            (false, true) => {
                player.facing_left = false;
                run_speed
            }
            _ => 0.0,
        };

        let standing_on_solid = collides_at(
            player,
            player.x,
            player.y + 1.0,
            &solids,
            Some(player.runtime_id),
        );
        player.jump.grounded_last_tick = standing_on_solid;

        if enable_builtin_jump {
            if jump.just_pressed && standing_on_solid {
                player.jump.active = true;
                player.jump.hold_frames = 0;
                player.jump.cut_applied = false;
                player.vspeed = -jump_speed;
            }

            if player.jump.active
                && jump.pressed
                && player.jump.hold_frames < jump_hold_frames
                && player.vspeed < 0.0
            {
                player.vspeed = player.vspeed.min(-jump_speed);
                player.jump.hold_frames += 1;
            }

            if jump.just_released && player.jump.active && player.vspeed < 0.0 && !player.jump.cut_applied {
                player.vspeed = player.vspeed.max(-jump_cut_speed);
                player.jump.cut_applied = true;
            }
        }

        player.vspeed = (player.vspeed + gravity).min(max_fall_speed);

        move_instance_axis(
            player,
            &solids,
            Some(player.runtime_id),
            Axis::Horizontal,
            player.hspeed,
        );
        let vertical_blocked = move_instance_axis(
            player,
            &solids,
            Some(player.runtime_id),
            Axis::Vertical,
            player.vspeed,
        );
        if vertical_blocked {
            player.jump.active = false;
            player.jump.cut_applied = true;
        }

        let grounded_after = collides_at(
            player,
            player.x,
            player.y + 1.0,
            &solids,
            Some(player.runtime_id),
        );
        if grounded_after {
            player.jump.active = false;
            player.jump.hold_frames = 0;
            player.jump.cut_applied = false;
        }
        player.jump.grounded_last_tick = grounded_after;

        if collides_at(
            player,
            player.x,
            player.y,
            &hazards,
            Some(player.runtime_id),
        ) {
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
