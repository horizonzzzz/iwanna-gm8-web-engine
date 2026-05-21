import type { RuntimePackage } from '../types';

export type WasmRuntimeBridgeSnapshot = {
  status?: string;
  tick: number;
  roomId: number | null;
  roomName?: string | null;
  instanceCount?: number;
  diagnostics: string[];
};

export type WasmRuntimeInputState = {
  left: boolean;
  right: boolean;
  jump: boolean;
  jumpPressed: boolean;
  jumpReleased: boolean;
  restart: boolean;
};

export type WasmRuntimeFrame = {
  tick: number;
  roomId: number | null;
  width: number;
  height: number;
  commands: Array<
    | { kind: 'clear'; colour: [number, number, number, number] }
    | { kind: 'drawBackground'; backgroundId: number; x: number; y: number; stretch: boolean; tileHorz: boolean; tileVert: boolean; isForeground: boolean }
    | { kind: 'drawTile'; backgroundId: number; x: number; y: number; tileX: number; tileY: number; width: number; height: number; xscale: number; yscale: number }
    | { kind: 'drawSprite'; spriteId: number; frameIndex: number; x: number; y: number; originX: number; originY: number; xscale: number; yscale: number; angleDegrees: number }
    | { kind: 'fillRect'; x: number; y: number; width: number; height: number; colour: [number, number, number, number] }
    | { kind: 'present' }
  >;
};

export type WasmRuntimeBridge = {
  backend: 'opengmk-wasm';
  boot: (pkg: RuntimePackage) => Promise<WasmRuntimeBridgeSnapshot> | WasmRuntimeBridgeSnapshot;
  snapshot: () => Promise<WasmRuntimeBridgeSnapshot> | WasmRuntimeBridgeSnapshot;
  frame: () => Promise<WasmRuntimeFrame> | WasmRuntimeFrame;
  setInput: (input: WasmRuntimeInputState) => Promise<WasmRuntimeBridgeSnapshot> | WasmRuntimeBridgeSnapshot;
  tick: (frames?: number) => Promise<WasmRuntimeBridgeSnapshot> | WasmRuntimeBridgeSnapshot;
  reset: () => Promise<WasmRuntimeBridgeSnapshot> | WasmRuntimeBridgeSnapshot;
  selectRoom: (roomId: number) => Promise<WasmRuntimeBridgeSnapshot> | WasmRuntimeBridgeSnapshot;
  diagnostics: () => Promise<string[]> | string[];
};

export type WasmRuntimeBridgeModule = {
  initRuntimeHost: () => Promise<WasmRuntimeBridge> | WasmRuntimeBridge;
};

type WasmRuntimeExports = {
  memory: { buffer: ArrayBufferLike };
  iwm_alloc: (size: number) => number;
  iwm_free: (pointer: number, size: number) => void;
  iwm_boot_json: (pointer: number, size: number) => number;
  iwm_set_input_json: (pointer: number, size: number) => number;
  iwm_tick: (frames: number) => number;
  iwm_reset: () => number;
  iwm_select_room: (roomId: number) => number;
  iwm_snapshot_json: () => number;
  iwm_frame_json: () => number;
  iwm_diagnostics_json: () => number;
  iwm_last_result_len: () => number;
};

function isFunction(value: unknown): value is (...args: unknown[]) => unknown {
  return typeof value === 'function';
}

export function isWasmRuntimeBridge(value: unknown): value is WasmRuntimeBridge {
  if (!value || typeof value !== 'object') {
    return false;
  }

  const candidate = value as Partial<WasmRuntimeBridge>;
  return candidate.backend === 'opengmk-wasm'
    && isFunction(candidate.boot)
    && isFunction(candidate.snapshot)
    && isFunction(candidate.frame)
    && isFunction(candidate.setInput)
    && isFunction(candidate.tick)
    && isFunction(candidate.reset)
    && isFunction(candidate.selectRoom)
    && isFunction(candidate.diagnostics);
}

function isWasmRuntimeExports(value: unknown): value is WasmRuntimeExports {
  if (!value || typeof value !== 'object') {
    return false;
  }

  const candidate = value as Partial<WasmRuntimeExports>;
  return !!candidate.memory
    && isFunction(candidate.iwm_alloc)
    && isFunction(candidate.iwm_free)
    && isFunction(candidate.iwm_boot_json)
    && isFunction(candidate.iwm_set_input_json)
    && isFunction(candidate.iwm_tick)
    && isFunction(candidate.iwm_reset)
    && isFunction(candidate.iwm_select_room)
    && isFunction(candidate.iwm_snapshot_json)
    && isFunction(candidate.iwm_frame_json)
    && isFunction(candidate.iwm_diagnostics_json)
    && isFunction(candidate.iwm_last_result_len);
}

function readJsonResult<T>(exports: WasmRuntimeExports, pointer: number): T {
  const byteLength = exports.iwm_last_result_len();
  const bytes = new Uint8Array(exports.memory.buffer, pointer, byteLength);
  const decoded = new TextDecoder().decode(bytes);
  const parsed = JSON.parse(decoded) as T & { error?: string };
  if (typeof parsed === 'object' && parsed && typeof parsed.error === 'string') {
    throw new Error(parsed.error);
  }
  return parsed;
}

function writeJsonInput(exports: WasmRuntimeExports, value: unknown): { pointer: number; byteLength: number } {
  const bytes = new TextEncoder().encode(JSON.stringify(value));
  const pointer = exports.iwm_alloc(bytes.byteLength);
  new Uint8Array(exports.memory.buffer, pointer, bytes.byteLength).set(bytes);
  return { pointer, byteLength: bytes.byteLength };
}

export function makeWasmRuntimeBridge(exports: WasmRuntimeExports): WasmRuntimeBridge {
  return {
    backend: 'opengmk-wasm',
    boot: async (pkg) => {
      const { pointer, byteLength } = writeJsonInput(exports, pkg);
      try {
        return readJsonResult<WasmRuntimeBridgeSnapshot>(exports, exports.iwm_boot_json(pointer, byteLength));
      } finally {
        exports.iwm_free(pointer, byteLength);
      }
    },
    snapshot: async () => {
      return readJsonResult<WasmRuntimeBridgeSnapshot>(exports, exports.iwm_snapshot_json());
    },
    frame: async () => {
      return readJsonResult<WasmRuntimeFrame>(exports, exports.iwm_frame_json());
    },
    setInput: async (input) => {
      const { pointer, byteLength } = writeJsonInput(exports, input);
      try {
        return readJsonResult<WasmRuntimeBridgeSnapshot>(exports, exports.iwm_set_input_json(pointer, byteLength));
      } finally {
        exports.iwm_free(pointer, byteLength);
      }
    },
    tick: async (frames = 1) => {
      return readJsonResult<WasmRuntimeBridgeSnapshot>(exports, exports.iwm_tick(Math.max(1, frames)));
    },
    reset: async () => {
      return readJsonResult<WasmRuntimeBridgeSnapshot>(exports, exports.iwm_reset());
    },
    selectRoom: async (roomId: number) => {
      return readJsonResult<WasmRuntimeBridgeSnapshot>(exports, exports.iwm_select_room(roomId));
    },
    diagnostics: async () => {
      return readJsonResult<string[]>(exports, exports.iwm_diagnostics_json());
    }
  };
}

export async function loadWasmRuntimeBridge(
  loader: () => Promise<unknown>
): Promise<WasmRuntimeBridge> {
  const loaded = await loader();
  if (isWasmRuntimeBridge(loaded)) {
    return loaded;
  }

  if (!loaded || typeof loaded !== 'object' || !isFunction((loaded as Partial<WasmRuntimeBridgeModule>).initRuntimeHost)) {
    throw new Error('WASM bridge module is missing initRuntimeHost()');
  }

  const bridge = await (loaded as WasmRuntimeBridgeModule).initRuntimeHost();
  if (!isWasmRuntimeBridge(bridge)) {
    throw new Error('WASM bridge initRuntimeHost() returned an invalid bridge');
  }

  return bridge;
}

export async function instantiateWasmRuntimeBridge(
  source: RequestInfo | URL,
  imports: WebAssembly.Imports = {}
): Promise<WasmRuntimeBridge> {
  const response = await fetch(source);
  if (!response.ok) {
    throw new Error(`failed to fetch wasm module: ${response.status} ${response.statusText}`);
  }

  const bytes = await response.arrayBuffer();
  const instantiated = await WebAssembly.instantiate(bytes, imports);
  const exported = instantiated.instance.exports;
  if (!isWasmRuntimeExports(exported)) {
    throw new Error('WASM module does not expose the expected iwm runtime bridge exports');
  }

  return makeWasmRuntimeBridge(exported);
}

export async function loadDefaultWasmRuntimeBridge(): Promise<WasmRuntimeBridge> {
  return instantiateWasmRuntimeBridge('/wasm/iwm_runtime_web.wasm');
}

export function describeWasmBridgeAvailability(bridge: WasmRuntimeBridge | null, error: unknown): string {
  if (bridge) {
    return 'WASM bridge available; shell can drive the OpenGMK-facing runtime host through the browser bridge.';
  }

  if (error instanceof Error) {
    return `WASM bridge unavailable: ${error.message}. Shell is using the static room viewer.`;
  }

  if (error != null) {
    return `WASM bridge unavailable: ${String(error)}. Shell is using the static room viewer.`;
  }

  return 'No WASM bridge configured; shell is using the static room viewer.';
}
