import type { RuntimeEventDispatchContext, RuntimeInstance, RuntimePreparedBlock, RuntimeSourceSnippetIntent } from './types';
import { destroyRuntimeInstance } from './instanceState';

function getSourceFromBlock(block: RuntimePreparedBlock): string[] {
  const source: string[] = [];
  for (const op of block.ops) {
    if (op && typeof op === 'object' && 'op' in op) {
      const value = op as { op: string; code?: string; args?: unknown[] };
      if (value.op === 'source-snippet' && typeof value.code === 'string') {
        source.push(value.code);
      } else if (value.op === 'action-call' && Array.isArray(value.args)) {
        for (const arg of value.args) {
          if (typeof arg === 'string') {
            source.push(arg);
          }
        }
      }
    }
  }
  return source;
}

export function prepareSnippetIntent(source: string): RuntimeSourceSnippetIntent {
  const lower = source.toLowerCase();
  if (lower.includes('room_goto_next')) {
    return { type: 'room-goto-next' };
  }
  const gotoLiteral = source.match(/room_goto\((\d+)\)/i);
  if (gotoLiteral) {
    return { type: 'room-goto-literal', roomId: Number(gotoLiteral[1]) };
  }
  if (lower.includes('instance_destroy')) {
    return { type: 'hazard-kill' };
  }
  if (lower.includes('playerjump') || lower.includes('hspeed') || lower.includes('vspeed') || lower.includes('gravity')) {
    return { type: 'player-step' };
  }
  if (lower.includes('x+=17') && lower.includes('y+=23')) {
    return { type: 'spawn-offset', x: 17, y: 23 };
  }
  if (lower.includes('instance_create') && lower.includes('player)')) {
    return { type: 'spawn-player-if-missing' };
  }
  if (lower.includes('room_caption') || lower.includes('sound_stop_all') || lower.includes('defcontrols')) {
    return { type: 'player-create', defaults: {} };
  }
  return { type: 'unknown', summary: source.slice(0, 120) };
}

export function interpretBlockSource(
  context: RuntimeEventDispatchContext,
  instance: RuntimeInstance,
  block: RuntimePreparedBlock
): void {
  const sources = getSourceFromBlock(block);
  for (const source of sources) {
    const intent = prepareSnippetIntent(source);
    switch (intent.type) {
      case 'player-create':
        instance.vars.frozen = false;
        instance.vars.jump = 8.5;
        instance.vars.jump2 = 7;
        instance.vars.djump = true;
        instance.vars.maxSpeed = 3;
        instance.vars.gravity = 0.4;
        instance.vars.maxFallSpeed = 8;
        instance.vars.maxVspeed = 9;
        instance.vars.image_speed = 0.2;
        instance.vars.ladder = false;
        instance.vars.moveSpeed = 3;
        break;
      case 'spawn-offset':
        instance.x += intent.x;
        instance.y += intent.y;
        break;
      case 'room-goto-next':
        context.queueRoomTransition({
          roomId: context.pkg.rooms[(context.pkg.rooms.findIndex((room) => room.id === context.room.roomId) + 1) % context.pkg.rooms.length]?.id ?? context.room.roomId
        });
        break;
      case 'room-goto-literal':
        context.queueRoomTransition({ roomId: intent.roomId });
        break;
      case 'spawn-player-if-missing':
        if (!context.room.instances.some((candidate) => candidate.playerCandidate && candidate.alive)) {
          context.room.playerRuntimeId = instance.runtimeId;
        }
        break;
      case 'hazard-kill':
        destroyRuntimeInstance(instance);
        break;
      case 'player-step':
      case 'solid-collision':
      case 'room-boundary-transition':
        break;
      case 'unknown':
        context.diagnostics.push({
          timestamp: context.tick,
          level: 'info',
          code: 'runtime-source-snippet-unhandled',
          message: `Unhandled source snippet in ${block.id}`,
          blockId: block.id,
          roomId: context.room.roomId,
          instanceId: instance.instanceId
        });
        break;
    }
  }
}
