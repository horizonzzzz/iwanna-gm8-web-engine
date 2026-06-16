use std::collections::HashMap;

use iwm_runtime_model::{ObjectDefinition, SpriteResource};

use crate::helpers::{adjusted_spawn_for_player, is_preferred_player_name};
use crate::types::{RuntimeCollisionMask, RuntimeJumpState, RuntimeRoomView};
use crate::{RuntimeCore, RuntimeCoreError, RuntimeInstance, RuntimeRoomState};

impl RuntimeCore {
    pub fn boot_default_room(&mut self) -> Result<(), RuntimeCoreError> {
        let room_id = self
            .package
            .manifest
            .default_room_id
            .or_else(|| self.package.rooms.first().map(|room| room.id))
            .ok_or(RuntimeCoreError::NoRooms)?;

        self.current_room = Some(self.build_room(room_id)?);
        self.room_needs_first_render_settle = true;
        self.status = crate::RuntimeStatus::Ready;
        Ok(())
    }

    pub(crate) fn build_room(
        &mut self,
        room_id: usize,
    ) -> Result<RuntimeRoomState, RuntimeCoreError> {
        let room = self
            .room_index
            .get(&room_id)
            .and_then(|index| self.package.rooms.get(*index))
            .cloned()
            .ok_or(RuntimeCoreError::RoomMissing(room_id))?;

        let spawn_point = room
            .instances
            .iter()
            .find(|instance| instance.is_checkpoint)
            .or_else(|| room.instances.first())
            .map(|instance| (instance.x, instance.y));

        let instances = room
            .instances
            .iter()
            .enumerate()
            .filter_map(|(runtime_id, instance)| {
                let object = self
                    .object_index
                    .get(&(instance.object_id as usize))
                    .and_then(|index| self.package.objects.get(*index))?;
                let metrics = self.sprite_metrics(object);
                let mut vars = HashMap::new();
                vars.insert(
                    "image_xscale".into(),
                    crate::RuntimeValue::Number(instance.xscale),
                );
                vars.insert(
                    "image_yscale".into(),
                    crate::RuntimeValue::Number(instance.yscale),
                );
                vars.insert("image_index".into(), crate::RuntimeValue::Number(0.0));
                vars.insert("image_speed".into(), crate::RuntimeValue::Number(1.0));
                Some(RuntimeInstance {
                    runtime_id,
                    instance_id: instance.instance_id,
                    object_id: instance.object_id as usize,
                    object_name: object.name.clone(),
                    x: instance.x as f64,
                    y: instance.y as f64,
                    previous_x: instance.x as f64,
                    previous_y: instance.y as f64,
                    hspeed: 0.0,
                    vspeed: 0.0,
                    width: metrics.width,
                    height: metrics.height,
                    origin_x: metrics.origin_x,
                    origin_y: metrics.origin_y,
                    bbox_left: metrics.bbox_left,
                    bbox_right: metrics.bbox_right,
                    bbox_top: metrics.bbox_top,
                    bbox_bottom: metrics.bbox_bottom,
                    collision_masks: metrics.collision_masks.clone(),
                    per_frame_collision_masks: metrics.per_frame_collision_masks,
                    facing_left: false,
                    visible: object.visible,
                    alive: true,
                    solid: instance.is_solid || object.solid,
                    hazard: instance.is_hazard || object.is_hazard.unwrap_or(false),
                    checkpoint: instance.is_checkpoint || object.is_checkpoint.unwrap_or(false),
                    player_candidate: object.is_player,
                    jump: RuntimeJumpState::default(),
                    vars,
                })
            })
            .collect::<Vec<_>>();

        let mut room_state = RuntimeRoomState {
            room_id: room.id,
            room_name: room.name.clone(),
            width: room.width,
            height: room.height,
            speed: room.speed,
            playable: room.playable,
            transition_targets: room.transition_targets.clone(),
            spawn_point,
            views_enabled: room.views_enabled,
            views: room.views.iter().map(RuntimeRoomView::from).collect(),
            instances,
        };
        if let Some((spawn_x, spawn_y)) = room_state.spawn_point {
            if let Some(player_index) = room_state.instances.iter().position(|instance| {
                instance.player_candidate
                    && instance.alive
                    && is_preferred_player_name(&instance.object_name)
            }) {
                let adjusted = adjusted_spawn_for_player(
                    &room_state.instances[player_index],
                    spawn_x,
                    spawn_y,
                    &room_state,
                );
                let player = &mut room_state.instances[player_index];
                player.x = adjusted.0 as f64;
                player.y = adjusted.1 as f64;
                player.previous_x = adjusted.0 as f64;
                player.previous_y = adjusted.1 as f64;
            }
        }
        self.apply_create_logic(&mut room_state, &room);
        self.apply_room_start_logic(&mut room_state);
        Ok(room_state)
    }

    pub(crate) fn sprite_metrics(&self, object: &ObjectDefinition) -> RuntimeSpriteMetrics {
        let sprite = self.sprite_for_index(object.mask_index).or_else(|| {
            if object.mask_index < 0 {
                self.sprite_for_index(object.sprite_index)
            } else {
                None
            }
        });

        sprite.map(RuntimeSpriteMetrics::from).unwrap_or_default()
    }

    pub(crate) fn instantiate_runtime_object(
        &self,
        object_id: usize,
        runtime_id: usize,
        x: f64,
        y: f64,
    ) -> Option<RuntimeInstance> {
        let object = self
            .object_index
            .get(&object_id)
            .and_then(|index| self.package.objects.get(*index))?;
        let metrics = self.sprite_metrics(object);
        let mut vars = HashMap::new();
        vars.insert("image_xscale".into(), crate::RuntimeValue::Number(1.0));
        vars.insert("image_yscale".into(), crate::RuntimeValue::Number(1.0));
        vars.insert("image_index".into(), crate::RuntimeValue::Number(0.0));
        vars.insert("image_speed".into(), crate::RuntimeValue::Number(1.0));
        Some(RuntimeInstance {
            runtime_id,
            instance_id: -1 - runtime_id as i32,
            object_id: object.id,
            object_name: object.name.clone(),
            x,
            y,
            previous_x: x,
            previous_y: y,
            hspeed: 0.0,
            vspeed: 0.0,
            width: metrics.width,
            height: metrics.height,
            origin_x: metrics.origin_x,
            origin_y: metrics.origin_y,
            bbox_left: metrics.bbox_left,
            bbox_right: metrics.bbox_right,
            bbox_top: metrics.bbox_top,
            bbox_bottom: metrics.bbox_bottom,
            collision_masks: metrics.collision_masks,
            per_frame_collision_masks: metrics.per_frame_collision_masks,
            facing_left: false,
            visible: object.visible,
            alive: true,
            solid: object.solid,
            hazard: object.is_hazard.unwrap_or(false),
            checkpoint: object.is_checkpoint.unwrap_or(false),
            player_candidate: object.is_player,
            jump: RuntimeJumpState::default(),
            vars,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeSpriteMetrics {
    pub width: i32,
    pub height: i32,
    pub origin_x: i32,
    pub origin_y: i32,
    pub bbox_left: i32,
    pub bbox_right: i32,
    pub bbox_top: i32,
    pub bbox_bottom: i32,
    pub collision_masks: Vec<RuntimeCollisionMask>,
    pub per_frame_collision_masks: bool,
}

impl Default for RuntimeSpriteMetrics {
    fn default() -> Self {
        Self {
            width: 16,
            height: 16,
            origin_x: 0,
            origin_y: 0,
            bbox_left: 0,
            bbox_right: 15,
            bbox_top: 0,
            bbox_bottom: 15,
            collision_masks: Vec::new(),
            per_frame_collision_masks: false,
        }
    }
}

impl From<&SpriteResource> for RuntimeSpriteMetrics {
    fn from(sprite: &SpriteResource) -> Self {
        let width = sprite.width.max(1) as i32;
        let height = sprite.height.max(1) as i32;
        Self {
            width,
            height,
            origin_x: sprite.origin_x,
            origin_y: sprite.origin_y,
            bbox_left: sprite.bbox_left as i32,
            bbox_right: sprite.bbox_right as i32,
            bbox_top: sprite.bbox_top as i32,
            bbox_bottom: sprite.bbox_bottom as i32,
            collision_masks: sprite
                .collision_masks
                .iter()
                .map(|mask| RuntimeCollisionMask {
                    width: mask.width,
                    height: mask.height,
                    bbox_left: mask.bbox_left as i32,
                    bbox_right: mask.bbox_right as i32,
                    bbox_top: mask.bbox_top as i32,
                    bbox_bottom: mask.bbox_bottom as i32,
                    data: mask.data.clone(),
                })
                .collect(),
            per_frame_collision_masks: sprite.per_frame_collision_masks,
        }
    }
}

impl RuntimeCore {
    fn sprite_for_index(&self, sprite_index: i32) -> Option<&SpriteResource> {
        if sprite_index < 0 {
            return None;
        }

        self.package
            .resources
            .sprites
            .iter()
            .find(|sprite| sprite.id == sprite_index as usize)
    }
}
