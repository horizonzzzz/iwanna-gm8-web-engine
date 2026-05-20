import { describe, expect, it } from 'vitest';
import { GameRuntime } from './gameRuntime';
import type { RuntimePackage } from '../types';

const samplePackage: RuntimePackage = {
  manifest: {
    format_version: 1,
    package_kind: 'runtime-v1',
    source_name: 'sample.exe',
    source_hash: 'abc123',
    engine_family: 'gm8',
    compatibility: 'partial',
    default_room_id: 0,
    room_count: 2,
    object_count: 3,
    script_block_count: 3,
    sprite_count: 1,
    background_count: 0,
    sound_count: 0,
    resource_index_path: 'resources/index.json',
    warnings: []
  },
  analysis: {
    dlls: [],
    included_files: [],
    warnings: [],
    unsupported_features: []
  },
  rooms: [
    {
      id: 0,
      name: 'Init',
      width: 320,
      height: 240,
      speed: 30,
      persistent: false,
      backgrounds: [],
      views_enabled: false,
      views: [],
      instances: [
        {
          instance_id: 1,
          object_id: 2,
          x: 0,
          y: 0,
          xscale: 1,
          yscale: 1,
          angle: 0,
          blend: 0xffffffff,
          creation_block_id: null,
          is_solid: false,
          is_hazard: false,
          is_checkpoint: false
        }
      ],
      creation_block_id: null,
      playable: false,
      transition_targets: []
    },
    {
      id: 1,
      name: 'Stage',
      width: 320,
      height: 240,
      speed: 30,
      persistent: false,
      backgrounds: [],
      views_enabled: false,
      views: [],
      instances: [
        {
          instance_id: 2,
          object_id: 1,
          x: 64,
          y: 64,
          xscale: 1,
          yscale: 1,
          angle: 0,
          blend: 0xffffffff,
          creation_block_id: null,
          is_solid: false,
          is_hazard: false,
          is_checkpoint: true
        },
        {
          instance_id: 3,
          object_id: 0,
          x: 64,
          y: 160,
          xscale: 1,
          yscale: 1,
          angle: 0,
          blend: 0xffffffff,
          creation_block_id: null,
          is_solid: true,
          is_hazard: false,
          is_checkpoint: false
        },
        {
          instance_id: 4,
          object_id: 3,
          x: 128,
          y: 160,
          xscale: 1,
          yscale: 1,
          angle: 0,
          blend: 0xffffffff,
          creation_block_id: null,
          is_solid: false,
          is_hazard: true,
          is_checkpoint: false
        }
      ],
      creation_block_id: null,
      playable: true,
      transition_targets: []
    }
  ],
  objects: [
    {
      id: 0,
      name: 'block',
      sprite_index: -1,
      parent_index: -1,
      depth: 0,
      persistent: false,
      visible: true,
      solid: true,
      mask_index: -1,
      is_hazard: null,
      is_checkpoint: null,
      is_player: false,
      events: []
    },
    {
      id: 1,
      name: 'playerStart',
      sprite_index: -1,
      parent_index: -1,
      depth: 0,
      persistent: false,
      visible: false,
      solid: false,
      mask_index: -1,
      is_hazard: null,
      is_checkpoint: true,
      is_player: false,
      events: [
        {
          event_type: 0,
          sub_event: 0,
          event_tag: 'create',
          block_id: 'object:1:event:0:0',
          action_count: 1
        }
      ]
    },
    {
      id: 2,
      name: 'init',
      sprite_index: -1,
      parent_index: -1,
      depth: 0,
      persistent: false,
      visible: true,
      solid: false,
      mask_index: -1,
      is_hazard: null,
      is_checkpoint: null,
      is_player: false,
      events: [
        {
          event_type: 3,
          sub_event: 0,
          event_tag: 'step',
          block_id: 'object:2:event:3:0',
          action_count: 1
        }
      ]
    },
    {
      id: 3,
      name: 'spikeUp',
      sprite_index: -1,
      parent_index: -1,
      depth: 0,
      persistent: false,
      visible: true,
      solid: false,
      mask_index: -1,
      is_hazard: true,
      is_checkpoint: null,
      is_player: false,
      events: []
    }
  ],
  scripts: {
    format: 'iwm-script-ir-v1',
    blocks: [
      {
        id: 'object:1:event:0:0',
        name: 'object playerStart event 0:0',
        kind: 'object-event',
        support: 'action-list',
        executable_action_count: 1,
        ops: [{ op: 'source-snippet', code: 'x+=17\ny+=23' }]
      },
      {
        id: 'object:2:event:3:0',
        name: 'object init event 3:0',
        kind: 'object-event',
        support: 'action-list',
        executable_action_count: 1,
        ops: [{ op: 'source-snippet', code: 'room_goto_next();' }]
      }
    ]
  },
  resources: {
    sprites: [],
    backgrounds: [],
    sounds: []
  }
};

describe('GameRuntime', () => {
  it('boots package and exposes initial room snapshot', () => {
    const runtime = new GameRuntime();
    runtime.load(samplePackage);
    expect(runtime.snapshot.roomId).toBe(0);
    expect(runtime.snapshot.instanceCount).toBe(1);
  });

  it('can transition to another room and reset', () => {
    const runtime = new GameRuntime();
    runtime.load(samplePackage);
    runtime.queueRoomTransition({ roomId: 1 });
    runtime.tick();
    expect(runtime.snapshot.roomId).toBe(1);
    runtime.reset();
    runtime.tick();
    expect(runtime.snapshot.roomId).toBe(1);
  });
});
