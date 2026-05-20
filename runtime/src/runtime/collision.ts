import type { ObjectDefinition } from '../types';
import type { RuntimeInstance, RuntimeRect } from './types';

export function getInstanceRect(instance: RuntimeInstance): RuntimeRect {
  const width = Math.max(1, Math.abs(instance.width * instance.xscale));
  const height = Math.max(1, Math.abs(instance.height * instance.yscale));
  const left = instance.x - instance.originX * instance.xscale;
  const top = instance.y - instance.originY * instance.yscale;
  return {
    left,
    top,
    right: left + width,
    bottom: top + height
  };
}

export function rectsOverlap(a: RuntimeRect, b: RuntimeRect): boolean {
  return a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top;
}

export function getRuntimeCollisionMask(instance: RuntimeInstance): RuntimeRect {
  return getInstanceRect(instance);
}

export function findColliders(
  instance: RuntimeInstance,
  objects: ObjectDefinition[],
  candidates: RuntimeInstance[]
): RuntimeInstance[] {
  const instanceRect = getRuntimeCollisionMask(instance);
  return candidates.filter((candidate) => {
    if (!candidate.alive || candidate.runtimeId === instance.runtimeId) {
      return false;
    }

    const object = objects.find((item) => item.id === candidate.objectId);
    if (!object) {
      return false;
    }

    if (!candidate.solid && !candidate.hazard && !candidate.checkpoint && !object.solid) {
      return false;
    }

    return rectsOverlap(instanceRect, getRuntimeCollisionMask(candidate));
  });
}

export function resolveAxisCollision(
  instance: RuntimeInstance,
  candidates: RuntimeInstance[],
  axis: 'x' | 'y'
): void {
  if (axis === 'x') {
    for (const collider of candidates) {
      const colliderRect = getRuntimeCollisionMask(collider);
      const instanceRect = getRuntimeCollisionMask(instance);
      if (!rectsOverlap(instanceRect, colliderRect)) {
        continue;
      }

      if (instance.x < collider.x) {
        instance.x = colliderRect.left - instance.originX * instance.xscale - 0.01;
      } else {
        instance.x = colliderRect.right + instance.originX * instance.xscale + 0.01;
      }
      instance.hspeed = 0;
    }
  } else {
    for (const collider of candidates) {
      const colliderRect = getRuntimeCollisionMask(collider);
      const instanceRect = getRuntimeCollisionMask(instance);
      if (!rectsOverlap(instanceRect, colliderRect)) {
        continue;
      }

      if (instance.y < collider.y) {
        instance.y = colliderRect.top - instance.originY * instance.yscale - 0.01;
      } else {
        instance.y = colliderRect.bottom + instance.originY * instance.yscale + 0.01;
      }
      instance.vspeed = 0;
    }
  }
}
