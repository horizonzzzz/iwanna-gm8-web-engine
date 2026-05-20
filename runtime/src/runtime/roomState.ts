import type { ObjectDefinition, RoomDefinition } from '../types';
import { createRuntimeInstance, isPlayerInstance } from './instanceState';
import type { RuntimeBlockRef, RuntimeInstance, RuntimePreparedPackage, RuntimeRoomState } from './types';

function getObjectBlocks(pkg: RuntimePreparedPackage, objectId: number): RuntimeBlockRef[] {
  const object = pkg.objectMap.get(objectId);
  if (!object) {
    return [];
  }

  return object.events.map((event) => ({
      objectId,
      blockId: event.block_id,
      eventTag: event.event_tag,
      eventType: event.event_type,
      subEvent: event.sub_event
    }));
}

function getPreferredPlayerObject(pkg: RuntimePreparedPackage): ObjectDefinition | null {
  return (
    pkg.objects.find((object) => /^player(?:\d+|face)?$/i.test(object.name) && object.is_player) ??
    null
  );
}

export function buildRoomState(pkg: RuntimePreparedPackage, room: RoomDefinition): RuntimeRoomState {
  const instances: RuntimeInstance[] = room.instances.map((placement) => {
    const object = pkg.objectMap.get(placement.object_id);
    const blocks = object ? getObjectBlocks(pkg, object.id) : [];
    return createRuntimeInstance(room, object ?? fallbackObject(placement.object_id), placement, blocks);
  });

  const spawn = room.instances
    .filter((instance) => instance.is_checkpoint)
    .slice(0, 1)
    .map((instance) => ({ x: instance.x, y: instance.y, objectId: instance.object_id }))
    [0] ?? null;

  const player = instances.find((instance) => isPlayerInstance(instance) && instance.alive) ?? null;
  if (!player) {
    const playerObject = getPreferredPlayerObject(pkg);
    if (playerObject) {
      const spawnX = spawn?.x ?? room.instances[0]?.x ?? 0;
      const spawnY = spawn?.y ?? room.instances[0]?.y ?? 0;
      const fallbackPlacement = {
        instance_id: -1,
        object_id: playerObject.id,
        x: spawnX,
        y: spawnY,
        xscale: 1,
        yscale: 1,
        angle: 0,
        blend: 0xffffffff,
        creation_block_id: null,
        is_solid: false,
        is_hazard: false,
        is_checkpoint: false
      };
      const spawnInstance = createRuntimeInstance(room, playerObject, fallbackPlacement, getObjectBlocks(pkg, playerObject.id));
      instances.push(spawnInstance);
    }
  }

  return {
    roomId: room.id,
    roomName: room.name,
    width: room.width,
    height: room.height,
    speed: room.speed,
    playable: room.playable,
    source: room,
    instances,
    spawn,
    playerRuntimeId: instances.find((instance) => isPlayerInstance(instance) && instance.alive)?.runtimeId ?? null
  };
}

function fallbackObject(objectId: number): ObjectDefinition {
  return {
    id: objectId,
    name: `object_${objectId}`,
    sprite_index: -1,
    parent_index: -1,
    depth: 0,
    persistent: false,
    visible: true,
    solid: false,
    mask_index: -1,
    is_hazard: null,
    is_checkpoint: null,
    is_player: false,
    events: []
  };
}

export function cloneRoomState(room: RuntimeRoomState): RuntimeRoomState {
  return {
    ...room,
    source: room.source,
    instances: room.instances.map((instance) => ({
      ...instance,
      vars: { ...instance.vars },
      eventBlocks: [...instance.eventBlocks]
    })),
    spawn: room.spawn ? { ...room.spawn } : null
  };
}
