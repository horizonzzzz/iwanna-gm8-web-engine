use std::collections::HashMap;

use iwm_runtime_model::ObjectDefinition;

use crate::helpers::is_preferred_player_name;
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

        let mut instances = room
            .instances
            .iter()
            .enumerate()
            .filter_map(|(runtime_id, instance)| {
                let object = self.package.objects.get(instance.object_id as usize)?;
                let (width, height, origin_x, origin_y) = self.sprite_metrics(object);
                Some(RuntimeInstance {
                    runtime_id,
                    instance_id: instance.instance_id,
                    object_id: instance.object_id as usize,
                    object_name: object.name.clone(),
                    x: instance.x,
                    y: instance.y,
                    previous_x: instance.x,
                    previous_y: instance.y,
                    hspeed: 0,
                    vspeed: 0,
                    width,
                    height,
                    origin_x,
                    origin_y,
                    alive: true,
                    solid: instance.is_solid || object.solid,
                    hazard: instance.is_hazard || object.is_hazard.unwrap_or(false),
                    checkpoint: instance.is_checkpoint || object.is_checkpoint.unwrap_or(false),
                    player_candidate: object.is_player,
                    vars: HashMap::new(),
                })
            })
            .collect::<Vec<_>>();

        let has_player = instances.iter().any(|instance| {
            instance.player_candidate
                && instance.alive
                && is_preferred_player_name(&instance.object_name)
        });
        if !has_player {
            let preferred_player = self
                .package
                .objects
                .iter()
                .find(|object| object.is_player && is_preferred_player_name(&object.name))
                .or_else(|| self.package.objects.iter().find(|object| object.is_player));

            if let Some(player_object) = preferred_player {
                let (width, height, origin_x, origin_y) = self.sprite_metrics(player_object);
                let (x, y) = spawn_point.unwrap_or((0, 0));
                instances.push(RuntimeInstance {
                    runtime_id: instances.len(),
                    instance_id: -1,
                    object_id: player_object.id,
                    object_name: player_object.name.clone(),
                    x,
                    y,
                    previous_x: x,
                    previous_y: y,
                    hspeed: 0,
                    vspeed: 0,
                    width,
                    height,
                    origin_x,
                    origin_y,
                    alive: true,
                    solid: player_object.solid,
                    hazard: player_object.is_hazard.unwrap_or(false),
                    checkpoint: player_object.is_checkpoint.unwrap_or(false),
                    player_candidate: true,
                    vars: HashMap::new(),
                });
            }
        }

        let mut room_state = RuntimeRoomState {
            room_id: room.id,
            room_name: room.name.clone(),
            width: room.width,
            height: room.height,
            speed: room.speed,
            playable: room.playable,
            transition_targets: room.transition_targets.clone(),
            spawn_point,
            instances,
        };
        self.apply_create_logic(&mut room_state, &room);
        Ok(room_state)
    }

    pub(crate) fn sprite_metrics(&self, object: &ObjectDefinition) -> (i32, i32, i32, i32) {
        if object.sprite_index >= 0 {
            if let Some(sprite) = self
                .package
                .resources
                .sprites
                .iter()
                .find(|sprite| sprite.id == object.sprite_index as usize)
            {
                return (
                    sprite.width.max(1) as i32,
                    sprite.height.max(1) as i32,
                    sprite.origin_x,
                    sprite.origin_y,
                );
            }
        }

        (16, 16, 0, 0)
    }
}
