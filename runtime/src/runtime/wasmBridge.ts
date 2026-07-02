import type { RuntimePackage } from '../types';
import { createWebAudioHost, type WasmAudioHost, type WasmSoundMode } from './wasmAudioHost';

const BRIDGE_BUFFER_MAGIC = 0x424d5749;
const BRIDGE_BUFFER_VERSION = 1;
const BRIDGE_INPUT_KIND = 1;
const BRIDGE_STEP_RESULT_KIND = 2;
const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();

export type WasmFileHost = {
  readFile: (path: string) => Uint8Array | null | undefined;
  writeFile: (path: string, bytes: Uint8Array) => void;
  removeFile: (path: string) => boolean | void;
  configurePackage?: (pkg: RuntimePackage, basePath: string) => void;
};

export type WasmRuntimeTickPhases = {
  inputDiagNanos: number;
  stepEventsNanos: number;
  viewSyncNanos: number;
  playerMovementNanos: number;
  collisionEventsNanos: number;
  alarmsNanos: number;
  keyboardEventsNanos: number;
  renderSubmitNanos: number;
  totalNanos: number;
};

export type WasmRuntimeBridgeSnapshot = {
  status?: string;
  tick: number;
  roomId: number | null;
  roomName?: string | null;
  roomSpeed?: number | null;
  instanceCount?: number;
  player?: {
    runtimeId?: number;
    instanceId?: number;
    objectId?: number;
    objectName?: string;
    x: number;
    y: number;
    hspeed: number;
    vspeed: number;
    facingLeft?: boolean;
    facing_left?: boolean;
    alive?: boolean;
    jump: {
      grounded: boolean;
      active: boolean;
      holdFrames: number;
      cutApplied: boolean;
    };
  } | null;
  inputTrace: {
    jumpButtonKey: number;
    jumpPressed: boolean;
    jumpJustPressed: boolean;
    jumpJustReleased: boolean;
    activeKeys: string[];
  };
  tickPhases?: WasmRuntimeTickPhases;
  diagnostics: string[];
};

export type WasmRuntimeInputState = {
  left: boolean;
  right: boolean;
  jump: boolean;
  jumpPressed: boolean;
  jumpReleased: boolean;
  restart: boolean;
  keysHeld?: number[];
  keysPressed?: number[];
  keysReleased?: number[];
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
    | { kind: 'drawSprite'; spriteId: number; frameIndex: number; x: number; y: number; originX: number; originY: number; xscale: number; yscale: number; alpha?: number; angleDegrees: number }
    | { kind: 'fillRect'; x: number; y: number; width: number; height: number; colour: [number, number, number, number] }
    | { kind: 'drawText'; text: string; x: number; y: number; size: number; fontName?: string | null; fontBold?: boolean; fontItalic?: boolean; colour: [number, number, number, number]; align: CanvasTextAlign }
    | { kind: 'present' }
  >;
};

export type WasmRuntimeBridgeStepResult = {
  snapshot: WasmRuntimeBridgeSnapshot;
  frame: WasmRuntimeFrame;
};

export type WasmRuntimeBridge = {
  backend: 'opengmk-wasm';
  boot: (pkg: RuntimePackage, options?: { basePath?: string }) => Promise<WasmRuntimeBridgeSnapshot> | WasmRuntimeBridgeSnapshot;
  snapshot: () => Promise<WasmRuntimeBridgeSnapshot> | WasmRuntimeBridgeSnapshot;
  frame: () => Promise<WasmRuntimeFrame> | WasmRuntimeFrame;
  setInput: (input: WasmRuntimeInputState) => Promise<WasmRuntimeBridgeSnapshot> | WasmRuntimeBridgeSnapshot;
  step?: (input: WasmRuntimeInputState) => Promise<WasmRuntimeBridgeStepResult> | WasmRuntimeBridgeStepResult;
  tick: (frames?: number) => Promise<WasmRuntimeBridgeSnapshot> | WasmRuntimeBridgeSnapshot;
  reset: () => Promise<WasmRuntimeBridgeSnapshot> | WasmRuntimeBridgeSnapshot;
  selectRoom: (roomId: number) => Promise<WasmRuntimeBridgeSnapshot> | WasmRuntimeBridgeSnapshot;
  diagnostics: () => Promise<string[]> | string[];
};

export type WasmRuntimeBridgeModule = {
  initRuntimeHost: () => Promise<WasmRuntimeBridge> | WasmRuntimeBridge;
};

class LocalStorageWasmFileHost implements WasmFileHost {
  private packageKey = 'unconfigured';
  private readonly memoryFallback = new Map<string, string>();

  constructor(private readonly namespace = 'iwm-runtime-save') {}

  configurePackage(pkg: RuntimePackage, basePath: string): void {
    const hash = pkg.manifest.source_hash || pkg.manifest.source_name || 'package';
    this.packageKey = `${basePath || 'default'}:${hash}`;
    this.removeFile('temp');
  }

  readFile(path: string): Uint8Array | null {
    const encoded = this.storageGet(this.key(path));
    if (!encoded) {
      return null;
    }
    return Uint8Array.from(atob(encoded), (char) => char.charCodeAt(0));
  }

  writeFile(path: string, bytes: Uint8Array): void {
    let binary = '';
    for (const byte of bytes) {
      binary += String.fromCharCode(byte);
    }
    this.storageSet(this.key(path), btoa(binary));
  }

  removeFile(path: string): boolean {
    const key = this.key(path);
    const existed = this.storageGet(key) != null;
    this.storageRemove(key);
    return existed;
  }

  private key(path: string): string {
    return `${this.namespace}:${this.packageKey}:${path}`;
  }

  private storageGet(key: string): string | null {
    try {
      return globalThis.localStorage?.getItem(key) ?? this.memoryFallback.get(key) ?? null;
    } catch {
      return this.memoryFallback.get(key) ?? null;
    }
  }

  private storageSet(key: string, value: string): void {
    try {
      globalThis.localStorage?.setItem(key, value);
      return;
    } catch {
      this.memoryFallback.set(key, value);
    }
  }

  private storageRemove(key: string): void {
    try {
      globalThis.localStorage?.removeItem(key);
    } catch {
      this.memoryFallback.delete(key);
    }
  }
}

export function createLocalStorageWasmFileHost(namespace?: string): WasmFileHost {
  return new LocalStorageWasmFileHost(namespace);
}

type WasmRuntimeExports = {
  memory: { buffer: ArrayBufferLike };
  iwm_alloc: (size: number) => number;
  iwm_free: (pointer: number, size: number) => void;
  iwm_boot_json: (pointer: number, size: number) => number;
  iwm_set_input_json: (pointer: number, size: number) => number;
  iwm_step_json?: (pointer: number, size: number) => number;
  iwm_step_buffer?: (pointer: number, size: number) => number;
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
  const decoded = textDecoder.decode(bytes);
  const parsed = JSON.parse(decoded) as T & { error?: string };
  if (typeof parsed === 'object' && parsed && typeof parsed.error === 'string') {
    throw new Error(parsed.error);
  }
  return parsed;
}

function writeJsonInput(exports: WasmRuntimeExports, value: unknown): { pointer: number; byteLength: number } {
  const bytes = textEncoder.encode(JSON.stringify(value));
  const pointer = exports.iwm_alloc(bytes.byteLength);
  new Uint8Array(exports.memory.buffer, pointer, bytes.byteLength).set(bytes);
  return { pointer, byteLength: bytes.byteLength };
}

class BridgeBufferWriter {
  private readonly bytes: number[] = [];

  intoBytes(): Uint8Array {
    return Uint8Array.from(this.bytes);
  }

  writeHeader(kind: number): void {
    this.writeU32(BRIDGE_BUFFER_MAGIC);
    this.writeU16(BRIDGE_BUFFER_VERSION);
    this.writeU16(kind);
  }

  writeBool(value: boolean): void {
    this.writeU8(value ? 1 : 0);
  }

  writeU8(value: number): void {
    this.bytes.push(value & 0xff);
  }

  writeU16(value: number): void {
    this.writeU8(value);
    this.writeU8(value >> 8);
  }

  writeU32(value: number): void {
    this.writeU8(value);
    this.writeU8(value >> 8);
    this.writeU8(value >> 16);
    this.writeU8(value >> 24);
  }

  writeU16Array(values: number[] | undefined): void {
    const array = values ?? [];
    this.writeU32(array.length);
    for (const value of array) {
      this.writeU16(value);
    }
  }
}

class BridgeBufferReader {
  private readonly view: DataView;
  private offset = 0;

  constructor(private readonly bytes: Uint8Array) {
    this.view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  }

  expectHeader(kind: number): void {
    const magic = this.readU32();
    if (magic !== BRIDGE_BUFFER_MAGIC) {
      throw new Error('invalid bridge buffer magic');
    }
    const version = this.readU16();
    if (version !== BRIDGE_BUFFER_VERSION) {
      throw new Error(`unsupported bridge buffer version: ${version}`);
    }
    const actualKind = this.readU16();
    if (actualKind !== kind) {
      throw new Error(`unexpected bridge buffer kind: ${actualKind}`);
    }
  }

  readBool(): boolean {
    return this.readU8() !== 0;
  }

  readU8(): number {
    this.ensureAvailable(1);
    const value = this.view.getUint8(this.offset);
    this.offset += 1;
    return value;
  }

  readU16(): number {
    this.ensureAvailable(2);
    const value = this.view.getUint16(this.offset, true);
    this.offset += 2;
    return value;
  }

  readU32(): number {
    this.ensureAvailable(4);
    const value = this.view.getUint32(this.offset, true);
    this.offset += 4;
    return value;
  }

  readI32(): number {
    this.ensureAvailable(4);
    const value = this.view.getInt32(this.offset, true);
    this.offset += 4;
    return value;
  }

  readU64(): number {
    this.ensureAvailable(8);
    const value = Number(this.view.getBigUint64(this.offset, true));
    this.offset += 8;
    return value;
  }

  readF64(): number {
    this.ensureAvailable(8);
    const value = this.view.getFloat64(this.offset, true);
    this.offset += 8;
    return value;
  }

  readString(): string {
    const byteLength = this.readU32();
    this.ensureAvailable(byteLength);
    const value = textDecoder.decode(this.bytes.subarray(this.offset, this.offset + byteLength));
    this.offset += byteLength;
    return value;
  }

  readOptionalU32(): number | null {
    return this.readBool() ? this.readU32() : null;
  }

  readOptionalString(): string | null {
    return this.readBool() ? this.readString() : null;
  }

  readStringArray(): string[] {
    const count = this.readU32();
    const values: string[] = [];
    for (let index = 0; index < count; index += 1) {
      values.push(this.readString());
    }
    return values;
  }

  private ensureAvailable(byteLength: number): void {
    if (this.offset + byteLength > this.bytes.byteLength) {
      throw new Error('bridge buffer ended unexpectedly');
    }
  }
}

function writeBinaryInput(exports: WasmRuntimeExports, input: WasmRuntimeInputState): { pointer: number; byteLength: number } {
  const writer = new BridgeBufferWriter();
  let flags = 0;
  flags |= input.left ? 0b0000_0001 : 0;
  flags |= input.right ? 0b0000_0010 : 0;
  flags |= input.jump ? 0b0000_0100 : 0;
  flags |= input.jumpPressed ? 0b0000_1000 : 0;
  flags |= input.jumpReleased ? 0b0001_0000 : 0;
  flags |= input.restart ? 0b0010_0000 : 0;
  writer.writeHeader(BRIDGE_INPUT_KIND);
  writer.writeU16(flags);
  writer.writeU16(0);
  writer.writeU16Array(input.keysHeld);
  writer.writeU16Array(input.keysPressed);
  writer.writeU16Array(input.keysReleased);
  const bytes = writer.intoBytes();
  const pointer = exports.iwm_alloc(bytes.byteLength);
  new Uint8Array(exports.memory.buffer, pointer, bytes.byteLength).set(bytes);
  return { pointer, byteLength: bytes.byteLength };
}

function readBinaryStepResult(exports: WasmRuntimeExports, pointer: number): WasmRuntimeBridgeStepResult {
  const byteLength = exports.iwm_last_result_len();
  const bytes = new Uint8Array(exports.memory.buffer, pointer, byteLength);
  if (byteLength < 4 || new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getUint32(0, true) !== BRIDGE_BUFFER_MAGIC) {
    return readJsonResult<WasmRuntimeBridgeStepResult>(exports, pointer);
  }
  const reader = new BridgeBufferReader(bytes);
  reader.expectHeader(BRIDGE_STEP_RESULT_KIND);
  return {
    snapshot: readBinarySnapshot(reader),
    frame: readBinaryFrame(reader)
  };
}

function readBinarySnapshot(reader: BridgeBufferReader): WasmRuntimeBridgeSnapshot {
  return {
    status: reader.readString(),
    tick: reader.readU64(),
    roomId: reader.readOptionalU32(),
    roomName: reader.readOptionalString(),
    roomSpeed: reader.readOptionalU32(),
    instanceCount: reader.readU32(),
    player: readBinaryPlayer(reader),
    inputTrace: {
      jumpButtonKey: reader.readU16(),
      jumpPressed: reader.readBool(),
      jumpJustPressed: reader.readBool(),
      jumpJustReleased: reader.readBool(),
      activeKeys: reader.readStringArray()
    },
    tickPhases: {
      inputDiagNanos: reader.readU64(),
      stepEventsNanos: reader.readU64(),
      viewSyncNanos: reader.readU64(),
      playerMovementNanos: reader.readU64(),
      collisionEventsNanos: reader.readU64(),
      alarmsNanos: reader.readU64(),
      keyboardEventsNanos: reader.readU64(),
      renderSubmitNanos: reader.readU64(),
      totalNanos: reader.readU64()
    },
    diagnostics: reader.readStringArray()
  };
}

function readBinaryPlayer(reader: BridgeBufferReader): WasmRuntimeBridgeSnapshot['player'] {
  if (!reader.readBool()) {
    return null;
  }
  return {
    runtimeId: reader.readU32(),
    instanceId: reader.readI32(),
    objectId: reader.readU32(),
    objectName: reader.readString(),
    x: reader.readF64(),
    y: reader.readF64(),
    hspeed: reader.readF64(),
    vspeed: reader.readF64(),
    facingLeft: reader.readBool(),
    alive: reader.readBool(),
    jump: {
      grounded: reader.readBool(),
      active: reader.readBool(),
      holdFrames: reader.readU32(),
      cutApplied: reader.readBool()
    }
  };
}

function readBinaryFrame(reader: BridgeBufferReader): WasmRuntimeFrame {
  const frame: WasmRuntimeFrame = {
    tick: reader.readU64(),
    roomId: reader.readOptionalU32(),
    width: reader.readU32(),
    height: reader.readU32(),
    commands: []
  };
  const commandCount = reader.readU32();
  for (let index = 0; index < commandCount; index += 1) {
    frame.commands.push(readBinaryCommand(reader));
  }
  return frame;
}

function readRgba(reader: BridgeBufferReader): [number, number, number, number] {
  return [reader.readU8(), reader.readU8(), reader.readU8(), reader.readU8()];
}

function readBinaryCommand(reader: BridgeBufferReader): WasmRuntimeFrame['commands'][number] {
  const kind = reader.readU8();
  switch (kind) {
    case 0:
      return { kind: 'clear', colour: readRgba(reader) };
    case 1:
      return {
        kind: 'drawBackground',
        backgroundId: reader.readU32(),
        x: reader.readI32(),
        y: reader.readI32(),
        stretch: reader.readBool(),
        tileHorz: reader.readBool(),
        tileVert: reader.readBool(),
        isForeground: reader.readBool()
      };
    case 2:
      return {
        kind: 'drawTile',
        backgroundId: reader.readU32(),
        x: reader.readI32(),
        y: reader.readI32(),
        tileX: reader.readU32(),
        tileY: reader.readU32(),
        width: reader.readU32(),
        height: reader.readU32(),
        xscale: reader.readF64(),
        yscale: reader.readF64()
      };
    case 3:
      return {
        kind: 'drawSprite',
        spriteId: reader.readU32(),
        frameIndex: reader.readU32(),
        x: reader.readI32(),
        y: reader.readI32(),
        originX: reader.readI32(),
        originY: reader.readI32(),
        xscale: reader.readF64(),
        yscale: reader.readF64(),
        alpha: reader.readF64(),
        angleDegrees: reader.readF64()
      };
    case 4:
      return {
        kind: 'fillRect',
        x: reader.readI32(),
        y: reader.readI32(),
        width: reader.readU32(),
        height: reader.readU32(),
        colour: readRgba(reader)
      };
    case 5:
      return {
        kind: 'drawText',
        text: reader.readString(),
        x: reader.readI32(),
        y: reader.readI32(),
        size: reader.readU32(),
        fontName: reader.readOptionalString(),
        fontBold: reader.readBool(),
        fontItalic: reader.readBool(),
        colour: readRgba(reader),
        align: reader.readString() as CanvasTextAlign
      };
    case 6:
      return { kind: 'present' };
    default:
      throw new Error(`unknown bridge draw command kind: ${kind}`);
  }
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
    step: exports.iwm_step_buffer
      ? async (input) => {
          const { pointer, byteLength } = writeBinaryInput(exports, input);
          try {
            return readBinaryStepResult(
              exports,
              exports.iwm_step_buffer!(pointer, byteLength)
            );
          } finally {
            exports.iwm_free(pointer, byteLength);
          }
        }
      : exports.iwm_step_json
      ? async (input) => {
          const { pointer, byteLength } = writeJsonInput(exports, input);
          try {
            return readJsonResult<WasmRuntimeBridgeStepResult>(
              exports,
              exports.iwm_step_json!(pointer, byteLength)
            );
          } finally {
            exports.iwm_free(pointer, byteLength);
          }
        }
      : undefined,
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
  imports: WebAssembly.Imports = {},
  options: WasmRuntimeHostImportOptions = {}
): Promise<WasmRuntimeBridge> {
  const response = await fetch(source);
  if (!response.ok) {
    throw new Error(`failed to fetch wasm module: ${response.status} ${response.statusText}`);
  }

  const bytes = await response.arrayBuffer();
  const mergedImports = mergeWasmRuntimeImports(imports, options);
  const instantiated = await WebAssembly.instantiate(bytes, mergedImports);
  const exported = instantiated.instance.exports;
  if (!isWasmRuntimeExports(exported)) {
    throw new Error('WASM module does not expose the expected iwm runtime bridge exports');
  }
  const bindMemory = (mergedImports.env as { __iwm_bind_memory?: (memory: WebAssembly.Memory) => void } | undefined)
    ?.__iwm_bind_memory;
  if (bindMemory && exported.memory instanceof WebAssembly.Memory) {
    bindMemory(exported.memory);
  }

  return makeWasmRuntimeBridge(exported);
}

export type WasmRuntimeHostImportOptions = {
  now?: () => number;
  audioHost?: Pick<WasmAudioHost, 'playSound' | 'stopSound' | 'stopAllSounds' | 'isSoundPlaying'>;
  fileHost?: WasmFileHost;
};

export function makeWasmRuntimeHostImports(
  options: WasmRuntimeHostImportOptions | (() => number) = {}
): WebAssembly.Imports {
  const now = typeof options === 'function'
    ? options
    : options.now ?? (() => globalThis.performance?.now() ?? Date.now());
  const audioHost = typeof options === 'function' ? undefined : options.audioHost;
  const fileHost = typeof options === 'function' ? undefined : options.fileHost ?? createLocalStorageWasmFileHost();
  let memory: WebAssembly.Memory | undefined;
  const readBytes = (pointer: number, byteLength: number): Uint8Array => {
    if (!memory) {
      return new Uint8Array();
    }
    return new Uint8Array(memory.buffer, pointer, byteLength);
  };
  const readHostPath = (pointer: number, byteLength: number): string => {
    return new TextDecoder().decode(readBytes(pointer, byteLength));
  };
  return {
    env: {
      __iwm_bind_memory: (boundMemory: WebAssembly.Memory) => {
        memory = boundMemory;
      },
      iwm_host_now_nanos: () => Math.max(0, now() * 1_000_000),
      iwm_host_play_sound: (soundId: number, mode: number) => {
        const result = audioHost?.playSound(soundId, wasmSoundMode(mode));
        if (result instanceof Promise) {
          void result.catch(() => undefined);
        }
      },
      iwm_host_stop_sound: (soundId: number) => {
        audioHost?.stopSound(soundId);
      },
      iwm_host_stop_all_sounds: () => {
        audioHost?.stopAllSounds();
      },
      iwm_host_is_sound_playing: (soundId: number) => {
        return audioHost?.isSoundPlaying(soundId) ? 1 : 0;
      },
      iwm_host_read_file: (pathPtr: number, pathLen: number, outPtr: number, outLen: number) => {
        const bytes = fileHost?.readFile(readHostPath(pathPtr, pathLen));
        if (!bytes) {
          return -1;
        }
        if (!outPtr || outLen === 0) {
          return bytes.byteLength;
        }
        const copyLen = Math.min(outLen, bytes.byteLength);
        readBytes(outPtr, copyLen).set(bytes.subarray(0, copyLen));
        return copyLen;
      },
      iwm_host_write_file: (pathPtr: number, pathLen: number, bytesPtr: number, bytesLen: number) => {
        fileHost?.writeFile(readHostPath(pathPtr, pathLen), new Uint8Array(readBytes(bytesPtr, bytesLen)));
        return 1;
      },
      iwm_host_remove_file: (pathPtr: number, pathLen: number) => {
        return fileHost?.removeFile(readHostPath(pathPtr, pathLen)) ? 1 : 0;
      }
    }
  };
}

function wasmSoundMode(mode: number): WasmSoundMode {
  return mode === 1 ? 'loop' : 'once';
}

function mergeWasmRuntimeImports(
  overrides: WebAssembly.Imports,
  options: WasmRuntimeHostImportOptions = {}
): WebAssembly.Imports {
  const defaults = makeWasmRuntimeHostImports(options);
  return {
    ...defaults,
    ...overrides,
    env: {
      ...(defaults.env ?? {}),
      ...((overrides.env as WebAssembly.ModuleImports | undefined) ?? {})
    }
  };
}

export async function loadDefaultWasmRuntimeBridge(): Promise<WasmRuntimeBridge> {
  const audioHost = createWebAudioHost();
  const fileHost = createLocalStorageWasmFileHost();
  const bridge = await instantiateWasmRuntimeBridge(
    '/wasm/iwm_runtime_web.wasm',
    {},
    { audioHost, fileHost }
  );
  return {
    ...bridge,
    boot: async (pkg, options) => {
      audioHost.configurePackage(pkg, options?.basePath ?? '');
      fileHost.configurePackage?.(pkg, options?.basePath ?? '');
      return bridge.boot(pkg);
    }
  };
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
