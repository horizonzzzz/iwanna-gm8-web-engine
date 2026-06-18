import type { RuntimePackage } from '../types';

type RuntimePackageFixtureOptions = {
  roomId?: number;
  roomName?: string;
  roomSpeed?: number;
  width?: number;
  height?: number;
  sourceName?: string;
  sourceHash?: string;
};

export function makeRuntimePackage({
  roomId = 1,
  roomName = 'rTest',
  roomSpeed = 60,
  width = 960,
  height = 540,
  sourceName = 'sample.exe',
  sourceHash = 'hash',
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
    resources: {
      sprites: [],
      backgrounds: [],
      sounds: [],
    },
    analysis: {
      dlls: [],
      included_files: [],
      warnings: [],
      unsupported_features: [],
    },
  };
}
