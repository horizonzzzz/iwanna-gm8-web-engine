import type { ObjectDefinition, RoomDefinition } from '../types';
import type { RuntimeBlockRef, RuntimeInstance } from './types';

let nextRuntimeInstanceId = 1;

function getSpriteMetrics(object: ObjectDefinition): {
  width: number;
  height: number;
  originX: number;
  originY: number;
} {
  return {
    width: object.sprite_index >= 0 ? 32 : 16,
    height: object.sprite_index >= 0 ? 32 : 16,
    originX: 0,
    originY: 0
  };
}

export function createRuntimeInstance(
  room: RoomDefinition,
  object: ObjectDefinition,
  placement: RoomDefinition['instances'][number],
  blocks: RuntimeBlockRef[]
): RuntimeInstance {
  const sprite = getSpriteMetrics(object);
  return {
    runtimeId: nextRuntimeInstanceId++,
    instanceId: placement.instance_id,
    objectId: object.id,
    objectName: object.name,
    x: placement.x,
    y: placement.y,
    prevX: placement.x,
    prevY: placement.y,
    hspeed: 0,
    vspeed: 0,
    gravity: 0.4,
    xscale: placement.xscale,
    yscale: placement.yscale,
    angle: placement.angle,
    visible: object.visible,
    persistent: object.persistent || room.persistent,
    solid: placement.is_solid || object.solid,
    hazard: Boolean(placement.is_hazard || object.is_hazard),
    checkpoint: Boolean(placement.is_checkpoint || object.is_checkpoint),
    playerCandidate: object.is_player,
    spriteIndex: object.sprite_index,
    maskIndex: object.mask_index,
    width: sprite.width,
    height: sprite.height,
    originX: sprite.originX,
    originY: sprite.originY,
    alive: true,
    vars: {},
    eventBlocks: blocks
  };
}

export function isPlayerInstance(instance: RuntimeInstance): boolean {
  return instance.playerCandidate && /^player(?:\d+|face)?$/i.test(instance.objectName);
}

export function getRuntimeBounds(instance: RuntimeInstance): {
  left: number;
  top: number;
  right: number;
  bottom: number;
} {
  const halfWidth = Math.max(1, Math.round(instance.width / 2));
  const halfHeight = Math.max(1, Math.round(instance.height / 2));
  return {
    left: instance.x - halfWidth,
    top: instance.y - halfHeight,
    right: instance.x + halfWidth,
    bottom: instance.y + halfHeight
  };
}

export function destroyRuntimeInstance(instance: RuntimeInstance): void {
  instance.alive = false;
}
