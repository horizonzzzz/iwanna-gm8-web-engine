import type { ObjectDefinition, ResourceIndex, RoomDefinition, RuntimePackage, ScriptIrFile } from '../types';

export type RuntimeInputSnapshot = {
  left: boolean;
  right: boolean;
  jump: boolean;
  jumpPressed: boolean;
  jumpReleased: boolean;
  restart: boolean;
};

export type RuntimeDiagnosticLevel = 'info' | 'warning' | 'error';

export type RuntimeDiagnostic = {
  timestamp: number;
  level: RuntimeDiagnosticLevel;
  code: string;
  message: string;
  blockId?: string;
  roomId?: number;
  instanceId?: number;
};

export type RuntimeRect = {
  left: number;
  top: number;
  right: number;
  bottom: number;
};

export type RuntimeBlockRef = {
  objectId: number;
  blockId: string;
  eventTag: string;
  eventType: number;
  subEvent: number;
};

export type RuntimeTransitionTarget = {
  roomId: number;
  x?: number;
  y?: number;
};

export type RuntimeInstance = {
  runtimeId: number;
  instanceId: number;
  objectId: number;
  objectName: string;
  x: number;
  y: number;
  prevX: number;
  prevY: number;
  hspeed: number;
  vspeed: number;
  gravity: number;
  xscale: number;
  yscale: number;
  angle: number;
  visible: boolean;
  persistent: boolean;
  solid: boolean;
  hazard: boolean;
  checkpoint: boolean;
  playerCandidate: boolean;
  spriteIndex: number;
  maskIndex: number;
  width: number;
  height: number;
  originX: number;
  originY: number;
  alive: boolean;
  vars: Record<string, number | string | boolean | null>;
  eventBlocks: RuntimeBlockRef[];
};

export type RuntimeRoomState = {
  roomId: number;
  roomName: string;
  width: number;
  height: number;
  speed: number;
  playable: boolean;
  source: RoomDefinition;
  instances: RuntimeInstance[];
  spawn: {
    x: number;
    y: number;
    objectId: number;
  } | null;
  playerRuntimeId: number | null;
};

export type RuntimeStatus = 'idle' | 'ready' | 'running' | 'paused' | 'error';

export type RuntimeSourceSnippetIntent =
  | {
      type: 'player-create';
      defaults: Record<string, number | string | boolean>;
    }
  | {
      type: 'player-step';
    }
  | {
      type: 'solid-collision';
    }
  | {
      type: 'hazard-kill';
    }
  | {
      type: 'room-boundary-transition';
    }
  | {
      type: 'spawn-offset';
      x: number;
      y: number;
    }
  | {
      type: 'spawn-player-if-missing';
    }
  | {
      type: 'room-goto-next';
    }
  | {
      type: 'room-goto-literal';
      roomId: number;
    }
  | {
      type: 'unknown';
      summary: string;
    };

export type RuntimePreparedBlock = {
  id: string;
  name: string;
  kind: string;
  support: string;
  executableActionCount: number;
  ops: ScriptIrFile['blocks'][number]['ops'];
  source?: string;
  intents: RuntimeSourceSnippetIntent[];
};

export type RuntimePreparedPackage = RuntimePackage & {
  roomMap: Map<number, RoomDefinition>;
  objectMap: Map<number, ObjectDefinition>;
  blockMap: Map<string, RuntimePreparedBlock>;
  spriteSizeMap: Map<number, { width: number; height: number; originX: number; originY: number }>;
  defaultPlayableRoomId: number | null;
};

export type RuntimeStateSnapshot = {
  status: RuntimeStatus;
  roomId: number | null;
  roomName: string | null;
  paused: boolean;
  tick: number;
  player: {
    x: number;
    y: number;
    hspeed: number;
    vspeed: number;
  } | null;
  instanceCount: number;
  diagnostics: RuntimeDiagnostic[];
};

export type RuntimeEventDispatchContext = {
  pkg: RuntimePreparedPackage;
  room: RuntimeRoomState;
  tick: number;
  diagnostics: RuntimeDiagnostic[];
  queueRoomTransition: (target: RuntimeTransitionTarget) => void;
  requestRoomReset: () => void;
};

export type RuntimeRenderFrame = {
  room: RuntimeRoomState | null;
  resources: ResourceIndex;
};
