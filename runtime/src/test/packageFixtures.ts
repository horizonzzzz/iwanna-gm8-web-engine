import type { WasmRuntimeBridgeSnapshot, WasmRuntimeFrame } from '../runtime/wasmBridge';
import type { ResourceIndex, RoomDefinition, RuntimePackage } from '../types';

type RuntimePackageFixtureOptions = {
  roomId?: number;
  roomName?: string;
  roomSpeed?: number;
  width?: number;
  height?: number;
  sourceName?: string;
  sourceHash?: string;
  resources?: ResourceIndex;
};

type WasmRuntimeBridgeSnapshotFixtureOptions = Omit<Partial<WasmRuntimeBridgeSnapshot>, 'inputTrace'> & {
  inputTrace?: Partial<WasmRuntimeBridgeSnapshot['inputTrace']>;
};

export function makeRuntimePackage({
  roomId = 1,
  roomName = 'rTest',
  roomSpeed = 60,
  width = 960,
  height = 540,
  sourceName = 'sample.exe',
  sourceHash = 'hash',
  resources = makeResourceIndex(),
}: RuntimePackageFixtureOptions = {}): RuntimePackage {
  return {
    manifest: {
      format_version: 1,
      package_kind: 'runtime-v1',
      source_name: sourceName,
      source_hash: sourceHash,
      engine_family: 'gm8',
      compatibility: 'partial',
      default_room_id: roomId,
      room_count: 1,
      object_count: 0,
      script_block_count: 0,
      sprite_count: 0,
      background_count: 0,
      sound_count: 0,
      resource_index_path: 'resources/index.json',
      warnings: [],
    },
    rooms: [
      {
        id: roomId,
        name: roomName,
        width,
        height,
        speed: roomSpeed,
        persistent: false,
        backgrounds: [],
        views_enabled: false,
        views: [],
        tiles: [],
        instances: [],
        creation_block_id: null,
        playable: true,
        transition_targets: [],
      },
    ],
    objects: [],
    scripts: {
      format: 'iwm-script-ir-v1',
      blocks: [],
    },
    rawLogic: {
      format: 'iwm-raw-logic-v1',
      room_creation_codes: [],
      instance_creation_codes: [],
      object_events: [],
      scripts: [],
      triggers: [],
      timelines: [],
    },
    loweredLogic: {
      format: 'iwm-lowered-logic-v1',
      entries: [],
    },
    resources,
    analysis: {
      dlls: [],
      included_files: [],
      warnings: [],
      unsupported_features: [],
    },
  };
}

export function makeRoomDefinition(overrides: Partial<RoomDefinition> = {}): RoomDefinition {
  return {
    id: 1,
    name: 'Room',
    width: 320,
    height: 240,
    speed: 30,
    persistent: false,
    backgrounds: [],
    views_enabled: false,
    views: [],
    tiles: [],
    instances: [],
    creation_block_id: null,
    playable: true,
    transition_targets: [],
    ...overrides,
  };
}

export function makeResourceIndex(overrides: Partial<ResourceIndex> = {}): ResourceIndex {
  return {
    sprites: [],
    backgrounds: [],
    sounds: [],
    fonts: [],
    paths: [],
    ...overrides,
  };
}

export function makeWasmSnapshot({
  inputTrace,
  ...overrides
}: WasmRuntimeBridgeSnapshotFixtureOptions = {}): WasmRuntimeBridgeSnapshot {
  return {
    tick: 0,
    roomId: 1,
    roomName: 'rTest',
    diagnostics: [],
    inputTrace: {
      jumpButtonKey: 0x20,
      jumpPressed: false,
      jumpJustPressed: false,
      jumpJustReleased: false,
      activeKeys: [],
      ...inputTrace,
    },
    player: null,
    ...overrides,
  };
}

export function makeWasmFrame(overrides: Partial<WasmRuntimeFrame> = {}): WasmRuntimeFrame {
  return {
    tick: 0,
    roomId: 1,
    width: 320,
    height: 240,
    commands: [{ kind: 'present' }],
    ...overrides,
  };
}
