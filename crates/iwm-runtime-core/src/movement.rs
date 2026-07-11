use iwm_runtime_host::{ButtonState, RuntimeHost};

use crate::helpers::{
    as_number, collides_at, collision_candidates_near, is_player_instance, move_instance_axis,
    player_out_of_bounds, Axis,
};
use crate::{RuntimeCore, RuntimeCoreError};

const RUN_SPEED: f64 = 4.0;
const JUMP_SPEED: f64 = 8.0;
const GRAVITY: f64 = 1.0;
const MAX_FALL_SPEED: f64 = 8.0;

impl RuntimeCore {
    pub(crate) fn step_non_player_instances(&mut self) -> Result<(), RuntimeCoreError> {
        let Some(room) = self.current_room.as_mut() else {
            return Err(RuntimeCoreError::NoRooms);
        };

        for instance in &mut room.instances {
            if !instance.alive || is_player_instance(instance) {
                continue;
            }

            instance.previous_x = instance.x;
            instance.previous_y = instance.y;
            apply_gm_motion_vars(instance);
            instance.x += instance.hspeed;
            instance.y += instance.vspeed;
        }

        Ok(())
    }

    pub(crate) fn step_player<H: RuntimeHost>(
        &mut self,
        _host: &mut H,
        left_pressed: bool,
        right_pressed: bool,
        jump: ButtonState,
        enable_builtin_jump: bool,
    ) -> Result<(), RuntimeCoreError> {
        if self.death_waiting_for_restart {
            return Ok(());
        }

        let Some(room) = self.current_room.as_ref() else {
            return Err(RuntimeCoreError::NoRooms);
        };

        let player_index = room.instances.iter().position(is_player_instance);
        let Some(player_index) = player_index else {
            return Ok(());
        };

        let room_width = room.width;
        let room_height = room.height;
        let player_snapshot = room
            .instances
            .get(player_index)
            .ok_or(RuntimeCoreError::NoRooms)?
            .clone();

        let run_speed = player_snapshot
            .vars
            .get("moveSpeed")
            .and_then(as_number)
            .or_else(|| player_snapshot.vars.get("maxSpeed").and_then(as_number))
            .unwrap_or(RUN_SPEED);
        let jump_speed = player_snapshot
            .vars
            .get("jump")
            .and_then(as_number)
            .unwrap_or(JUMP_SPEED);
        let jump_cut_speed = player_snapshot
            .vars
            .get("jump2")
            .and_then(as_number)
            .unwrap_or((jump_speed - 1.0).max(1.0));
        let jump_hold_frames = player_snapshot
            .vars
            .get("jumpHoldFrames")
            .and_then(as_number)
            .unwrap_or(4.0)
            .round() as u32;
        let gravity = player_snapshot
            .vars
            .get("gravity")
            .and_then(as_number)
            .unwrap_or(GRAVITY);
        let max_fall_speed = player_snapshot
            .vars
            .get("maxFallSpeed")
            .and_then(as_number)
            .unwrap_or(MAX_FALL_SPEED);

        let next_hspeed = match (left_pressed, right_pressed) {
            (true, false) => -run_speed,
            (false, true) => run_speed,
            _ => 0.0,
        };
        let movement_padding = next_hspeed
            .abs()
            .max(player_snapshot.vspeed.abs())
            .max(jump_speed + gravity.abs())
            .max(max_fall_speed)
            .ceil()
            + 2.0;
        let solids = collision_candidates_near(
            &player_snapshot,
            player_snapshot.x,
            player_snapshot.y,
            &room.instances,
            Some(player_snapshot.runtime_id),
            movement_padding,
            |instance| instance.alive && instance.solid,
        );
        let room = self
            .current_room
            .as_mut()
            .ok_or(RuntimeCoreError::NoRooms)?;
        let player = room
            .instances
            .get_mut(player_index)
            .ok_or(RuntimeCoreError::NoRooms)?;

        player.previous_x = player.x;
        player.previous_y = player.y;

        player.hspeed = match (left_pressed, right_pressed) {
            (true, false) => {
                player.facing_left = true;
                next_hspeed
            }
            (false, true) => {
                player.facing_left = false;
                next_hspeed
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

            if jump.just_released
                && player.jump.active
                && player.vspeed < 0.0
                && !player.jump.cut_applied
            {
                player.vspeed = player.vspeed.max(-jump_cut_speed);
                player.jump.cut_applied = true;
            }
        }

        player.vspeed = (player.vspeed + gravity).min(max_fall_speed);

        if enable_builtin_jump {
            let horizontal_delta = player.hspeed;
            move_instance_axis(
                player,
                &solids,
                Some(player.runtime_id),
                Axis::Horizontal,
                horizontal_delta,
            );
            let vertical_delta = player.vspeed;
            let vertical_blocked = move_instance_axis(
                player,
                &solids,
                Some(player.runtime_id),
                Axis::Vertical,
                vertical_delta,
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
        } else {
            // GM8 never clamps motion against solids: the collision event pipeline
            // (previous-position rollback + the game's own move_contact_solid GML)
            // resolves them, and it must keep firing while gravity presses a resting
            // player into the floor so per-frame GML like `djump = 1` stays live.
            player.x += player.hspeed;
            player.y += player.vspeed;
        }

        if player_out_of_bounds(player, room_width, room_height)
            && !room.transition_targets.is_empty()
        {
            self.pending_room_transition = room.transition_targets.first().copied();
        }

        Ok(())
    }

    pub(crate) fn detect_player_hazard_after_collision_events<H: RuntimeHost>(
        &mut self,
        host: &mut H,
    ) -> Result<(), RuntimeCoreError> {
        if self.death_waiting_for_restart {
            return Ok(());
        }
        let Some(room) = self.current_room.as_ref() else {
            return Err(RuntimeCoreError::NoRooms);
        };
        let Some(player) = room
            .instances
            .iter()
            .find(|instance| instance.alive && is_player_instance(instance))
        else {
            return Ok(());
        };
        let hazards = collision_candidates_near(
            player,
            player.x,
            player.y,
            &room.instances,
            Some(player.runtime_id),
            2.0,
            |instance| instance.alive && instance.hazard,
        );
        if !collides_at(
            player,
            player.x,
            player.y,
            &hazards,
            Some(player.runtime_id),
        ) {
            return Ok(());
        }
        let solids = collision_candidates_near(
            player,
            player.x,
            player.y,
            &room.instances,
            Some(player.runtime_id),
            2.0,
            |instance| instance.alive && instance.solid,
        );
        if final_solid_contact_shadows_hazard(player, &hazards, &solids) {
            return Ok(());
        }

        let death_message = format!(
            "room={} tick={} object={} runtime_id={} x={} y={} reason=hazard message=player-hit-hazard-in-{}",
            room.room_id,
            self.tick,
            player.object_name,
            player.runtime_id,
            player.x,
            player.y,
            room.room_name
        );
        self.record_diagnostic(
            host,
            iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
            "runtime-player-died",
            death_message,
        );
        self.death_waiting_for_restart = true;
        Ok(())
    }
}

fn final_solid_contact_shadows_hazard(
    player: &crate::RuntimeInstance,
    hazards: &[crate::RuntimeInstance],
    solids: &[crate::RuntimeInstance],
) -> bool {
    let ignore_runtime_id = Some(player.runtime_id);
    let related_contact_planes = hazards.iter().any(|hazard| {
        solids.iter().any(|solid| {
            let (hazard_left, _, hazard_right, _) =
                crate::helpers::bounds_at(hazard, hazard.x, hazard.y);
            let (solid_left, _, solid_right, _) =
                crate::helpers::bounds_at(solid, solid.x, solid.y);
            hazard_left == solid_right
                || hazard_right == solid_left
                || crate::helpers::collides_with_instance_at(
                    hazard,
                    hazard.x,
                    hazard.y,
                    solid,
                    Some(hazard.runtime_id),
                    |_| true,
                )
        })
    });
    if !related_contact_planes {
        return false;
    }
    [(0.0, -1.0), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0)]
        .into_iter()
        .any(|(separate_x, separate_y)| {
            !collides_at(
                player,
                player.x + separate_x,
                player.y + separate_y,
                hazards,
                ignore_runtime_id,
            ) && collides_at(
                player,
                player.x - separate_x,
                player.y - separate_y,
                solids,
                ignore_runtime_id,
            )
        })
}

pub(crate) fn apply_gm_motion_vars(instance: &mut crate::RuntimeInstance) {
    if instance.hspeed == 0.0 && instance.vspeed == 0.0 {
        if instance.vars.contains_key("speed") || instance.vars.contains_key("direction") {
            instance.sync_hvspeed_from_speed_direction();
        }
    }

    if let Some(friction) = instance.vars.get("friction").and_then(as_number) {
        if friction != 0.0 {
            let speed = instance
                .vars
                .get("speed")
                .and_then(as_number)
                .unwrap_or(0.0);
            if speed > 0.0 {
                if friction > speed {
                    instance.set_speed(0.0);
                } else {
                    instance.set_speed(speed - friction);
                }
            } else if speed < 0.0 {
                if friction > -speed {
                    instance.set_speed(0.0);
                } else {
                    instance.set_speed(speed + friction);
                }
            }
        }
    }

    if let Some(gravity) = instance.vars.get("gravity").and_then(as_number) {
        if gravity != 0.0 {
            let gravity_direction = instance
                .vars
                .get("gravity_direction")
                .and_then(as_number)
                .unwrap_or(270.0);
            let radians = gravity_direction.to_radians();
            let new_hspeed = instance.hspeed + radians.cos() * gravity;
            let new_vspeed = instance.vspeed - radians.sin() * gravity;
            instance.set_hvspeed(new_hspeed, new_vspeed);
        }
    }
}
