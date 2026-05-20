import { describe, expect, it } from 'vitest';
import { getInstanceRect, rectsOverlap } from './collision';
import type { RuntimeInstance } from './types';

function makeInstance(overrides: Partial<RuntimeInstance> = {}): RuntimeInstance {
  return {
    runtimeId: 1,
    instanceId: 1,
    objectId: 1,
    objectName: 'test',
    x: 10,
    y: 10,
    prevX: 10,
    prevY: 10,
    hspeed: 0,
    vspeed: 0,
    gravity: 0,
    xscale: 1,
    yscale: 1,
    angle: 0,
    visible: true,
    persistent: false,
    solid: true,
    hazard: false,
    checkpoint: false,
    playerCandidate: false,
    spriteIndex: -1,
    maskIndex: -1,
    width: 32,
    height: 32,
    originX: 0,
    originY: 0,
    alive: true,
    vars: {},
    eventBlocks: [],
    ...overrides
  };
}

describe('collision', () => {
  it('builds runtime rects from instance position and size', () => {
    expect(getInstanceRect(makeInstance())).toEqual({
      left: 10,
      top: 10,
      right: 42,
      bottom: 42
    });
  });

  it('detects overlapping rects', () => {
    const a = getInstanceRect(makeInstance());
    const b = getInstanceRect(makeInstance({ x: 20, y: 20 }));
    expect(rectsOverlap(a, b)).toBe(true);
  });
});
