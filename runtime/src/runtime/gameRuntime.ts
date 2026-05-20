import type { ObjectDefinition, RoomDefinition, RuntimePackage } from '../types';
import { FixedStepLoop } from './fixedStepLoop';
import { RuntimeInputController } from './input';
import { createRuntimeInstance, destroyRuntimeInstance, isPlayerInstance } from './instanceState';
import { buildRoomState, cloneRoomState } from './roomState';
import { dispatchEventBlocks } from './eventDispatch';
import type {
  RuntimeDiagnostic,
  RuntimePreparedBlock,
  RuntimePreparedPackage,
  RuntimeRoomState,
  RuntimeStateSnapshot,
  RuntimeTransitionTarget
} from './types';
import { findColliders, getRuntimeCollisionMask, resolveAxisCollision } from './collision';

function createDiagnostic(level: RuntimeDiagnostic['level'], code: string, message: string): RuntimeDiagnostic {
  return {
    timestamp: Date.now(),
    level,
    code,
    message
  };
}

export function prepareRuntimePackage(pkg: RuntimePackage): RuntimePreparedPackage {
  const roomMap = new Map(pkg.rooms.map((room) => [room.id, room]));
  const objectMap = new Map(pkg.objects.map((object) => [object.id, object]));
  const spriteSizeMap = new Map(
    pkg.resources.sprites.map((sprite) => [
      sprite.id,
      {
        width: sprite.width,
        height: sprite.height,
        originX: sprite.origin_x,
        originY: sprite.origin_y
      }
    ])
  );
  const blockMap = new Map<string, RuntimePreparedBlock>();

  for (const block of pkg.scripts.blocks) {
    const source = block.ops
      .flatMap((op) => {
        if (op.op === 'source-snippet' && typeof op.code === 'string') {
          return [op.code];
        }
        if (op.op === 'action-call' && Array.isArray(op.args)) {
          return op.args.filter((arg): arg is string => typeof arg === 'string');
        }
        return [];
      })
      .join('\n');

    blockMap.set(block.id, {
      id: block.id,
      name: block.name,
      kind: block.kind,
      support: block.support,
      executableActionCount: block.executable_action_count,
      ops: block.ops,
      source,
      intents: []
    });
  }

  const prepared = {
    ...pkg,
    roomMap,
    objectMap,
    blockMap,
    spriteSizeMap,
    defaultPlayableRoomId: pkg.manifest.default_room_id
  };

  for (const [blockId, block] of prepared.blockMap) {
    prepared.blockMap.set(blockId, {
      ...block,
      intents: block.source
        ? block.source.split(/\r?\n+/).filter(Boolean).map((line) => {
            const lower = line.toLowerCase();
            if (lower.includes('room_goto_next')) {
              return { type: 'room-goto-next' };
            }
            if (lower.includes('instance_destroy')) {
              return { type: 'hazard-kill' };
            }
            if (lower.includes('player')) {
              return { type: 'player-step' };
            }
            return { type: 'unknown', summary: line.slice(0, 120) };
          })
        : []
    });
  }

  return prepared;
}

export class GameRuntime {
  private readonly input = new RuntimeInputController();
  private readonly loop: FixedStepLoop;
  private readonly diagnostics: RuntimeDiagnostic[] = [];
  private pkg: RuntimePreparedPackage | null = null;
  private room: RuntimeRoomState | null = null;
  private status: 'idle' | 'ready' | 'running' | 'paused' | 'error' = 'idle';
  private tickCount = 0;
  private pendingRoomTransition: RuntimeTransitionTarget | null = null;
  private pendingReset = false;

  constructor() {
    this.loop = new FixedStepLoop({
      onStep: () => {
        this.step();
      }
    });
  }

  get snapshot(): RuntimeStateSnapshot {
    return {
      status: this.status,
      roomId: this.room?.roomId ?? null,
      roomName: this.room?.roomName ?? null,
      paused: this.loop.isPaused,
      tick: this.tickCount,
      player: this.getPlayerSnapshot(),
      instanceCount: this.room?.instances.filter((instance) => instance.alive).length ?? 0,
      diagnostics: [...this.diagnostics]
    };
  }

  load(pkg: RuntimePackage): void {
    this.pkg = prepareRuntimePackage(pkg);
    this.diagnostics.length = 0;
    this.tickCount = 0;
    this.pendingRoomTransition = null;
    this.pendingReset = false;
    this.status = 'ready';
    const roomId = this.pkg.defaultPlayableRoomId ?? this.pkg.rooms[0]?.id ?? null;
    this.room = roomId == null ? null : buildRoomState(this.pkg, this.requireRoom(roomId));
    if (this.room) {
      this.dispatchRoomStart();
      this.dispatchRoomEvent('other:room-start');
    }
  }

  resume(): void {
    this.loop.resume();
    this.status = 'running';
  }

  pause(): void {
    this.loop.pause();
    this.status = this.pkg ? 'paused' : 'idle';
  }

  reset(): void {
    this.pendingReset = true;
  }

  setInput(snapshot: { left: boolean; right: boolean; jump: boolean; restart: boolean }): void {
    this.input.setSnapshot(snapshot);
  }

  tick(): void {
    this.loop.tick(1);
  }

  step(): void {
    if (!this.pkg || !this.room) {
      return;
    }

    this.tickCount += 1;

    if (this.pendingReset) {
      this.pendingReset = false;
      this.room = buildRoomState(this.pkg, this.requireRoom(this.room.roomId));
      this.dispatchRoomStart();
    }

    const input = this.input.sample();
    if (input.restart) {
      this.pendingReset = true;
    }

    const room = this.room;
    this.dispatchRoomEvent('step');
    const player = this.getPlayer();

    if (player) {
      if (input.left && !input.right) {
        player.hspeed = -Math.abs(player.vars.maxSpeed instanceof Number ? Number(player.vars.maxSpeed) : 3);
      } else if (input.right && !input.left) {
        player.hspeed = Math.abs(player.vars.maxSpeed instanceof Number ? Number(player.vars.maxSpeed) : 3);
      } else {
        player.hspeed *= 0.85;
      }

      if (input.jumpPressed && Math.abs(player.vspeed) < 0.01) {
        player.vspeed = -(typeof player.vars.jump === 'number' ? player.vars.jump : 8.5);
      }

      player.vspeed += typeof player.vars.gravity === 'number' ? player.vars.gravity : 0.4;
      player.vspeed = Math.min(player.vspeed, typeof player.vars.maxFallSpeed === 'number' ? player.vars.maxFallSpeed : 8);

      player.prevX = player.x;
      player.prevY = player.y;
      player.x += player.hspeed;
      const solids = room.instances.filter((instance) => instance.alive && instance.solid);
      resolveAxisCollision(player, solids, 'x');
      player.y += player.vspeed;
      resolveAxisCollision(player, solids, 'y');

      for (const hazard of room.instances.filter((instance) => instance.alive && instance.hazard)) {
        if (hazard.runtimeId === player.runtimeId) {
          continue;
        }
        if (getRuntimeCollisionMask(player).left < getRuntimeCollisionMask(hazard).right &&
            getRuntimeCollisionMask(player).right > getRuntimeCollisionMask(hazard).left &&
            getRuntimeCollisionMask(player).top < getRuntimeCollisionMask(hazard).bottom &&
            getRuntimeCollisionMask(player).bottom > getRuntimeCollisionMask(hazard).top) {
          destroyRuntimeInstance(player);
          this.diagnostics.push(createDiagnostic('warning', 'runtime-player-died', `Player died in room ${room.roomName}`));
          this.pendingReset = true;
        }
      }

      if (player.x < 0 || player.x > room.width || player.y < 0 || player.y > room.height) {
        this.queueRoomTransition({ roomId: room.roomId });
      }
    }

    if (this.pendingRoomTransition) {
      const target = this.pendingRoomTransition;
      this.pendingRoomTransition = null;
      const nextRoom = this.requireRoom(target.roomId);
      this.room = buildRoomState(this.pkg, nextRoom);
      if (typeof target.x === 'number') {
        const nextPlayer = this.getPlayer();
        if (nextPlayer) {
          nextPlayer.x = target.x;
        }
      }
      if (typeof target.y === 'number') {
        const nextPlayer = this.getPlayer();
        if (nextPlayer) {
          nextPlayer.y = target.y;
        }
      }
      this.dispatchRoomStart();
      this.dispatchRoomEvent('other:room-start');
    }

    if (this.status === 'ready') {
      this.status = 'running';
    }
  }

  queueRoomTransition(target: RuntimeTransitionTarget): void {
    this.pendingRoomTransition = target;
    this.diagnostics.push({
      timestamp: Date.now(),
      level: 'info',
      code: 'runtime-room-transition-requested',
      message: `Requested transition to room ${target.roomId}`,
      roomId: this.room?.roomId
    });
  }

  getRoom(): RuntimeRoomState | null {
    return this.room ? cloneRoomState(this.room) : null;
  }

  getDiagnostics(): RuntimeDiagnostic[] {
    return [...this.diagnostics];
  }

  private dispatchRoomStart(): void {
    if (!this.pkg || !this.room) {
      return;
    }

    for (const instance of this.room.instances.filter((candidate) => candidate.alive)) {
      dispatchEventBlocks(
        {
          pkg: this.pkg,
          room: this.room,
          tick: this.tickCount,
          diagnostics: this.diagnostics,
          queueRoomTransition: (target) => {
            this.queueRoomTransition(target);
          },
          requestRoomReset: () => {
            this.pendingReset = true;
          }
        },
        instance,
        'create'
      );
    }
  }

  private dispatchRoomEvent(eventTag: string): void {
    if (!this.pkg || !this.room) {
      return;
    }

    for (const instance of this.room.instances.filter((candidate) => candidate.alive)) {
      dispatchEventBlocks(
        {
          pkg: this.pkg,
          room: this.room,
          tick: this.tickCount,
          diagnostics: this.diagnostics,
          queueRoomTransition: (target) => {
            this.queueRoomTransition(target);
          },
          requestRoomReset: () => {
            this.pendingReset = true;
          }
        },
        instance,
        eventTag
      );
    }
  }

  private getPlayer(): RuntimeInstance | null {
    return this.room?.instances.find((instance) => instance.alive && isPlayerInstance(instance)) ?? null;
  }

  private getPlayerSnapshot(): RuntimeStateSnapshot['player'] {
    const player = this.getPlayer();
    if (!player) {
      return null;
    }

    return {
      x: player.x,
      y: player.y,
      hspeed: player.hspeed,
      vspeed: player.vspeed
    };
  }

  private requireRoom(roomId: number): RoomDefinition {
    if (!this.pkg) {
      throw new Error('runtime package is not loaded');
    }
    const room = this.pkg.roomMap.get(roomId);
    if (!room) {
      throw new Error(`room ${roomId} is not available`);
    }
    return room;
  }
}
