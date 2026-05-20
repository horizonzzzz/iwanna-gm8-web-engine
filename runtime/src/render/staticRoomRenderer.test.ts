import { describe, expect, it, vi } from 'vitest';
import { renderStaticRoom, resolveBackgroundDraws } from './staticRoomRenderer';
import type { ObjectDefinition, RoomDefinition } from '../types';

describe('resolveBackgroundDraws', () => {
  it('returns visible background layers with their draw flags', () => {
    const room: RoomDefinition = {
      id: 1,
      name: 'Room',
      width: 320,
      height: 240,
      speed: 30,
      persistent: false,
      backgrounds: [
        {
          visible_on_start: false,
          is_foreground: false,
          source_bg: 0,
          xoffset: 0,
          yoffset: 0,
          tile_horz: false,
          tile_vert: false,
          hspeed: 0,
          vspeed: 0,
          stretch: false
        },
        {
          visible_on_start: true,
          is_foreground: false,
          source_bg: 1,
          xoffset: 4,
          yoffset: 8,
          tile_horz: false,
          tile_vert: false,
          hspeed: 0,
          vspeed: 0,
          stretch: false
        }
      ],
      views_enabled: false,
      views: [],
      instances: [],
      creation_block_id: null,
      playable: true,
      transition_targets: []
    };

    const backgroundPaths = new Map([[1, '/pkg/resources/backgrounds/1.png']]);

    expect(resolveBackgroundDraws(room, backgroundPaths)).toEqual([
      {
        imagePath: '/pkg/resources/backgrounds/1.png',
        x: 4,
        y: 8,
        stretch: false,
        tileHorz: false,
        tileVert: false,
        isForeground: false
      }
    ]);
  });
});

describe('renderStaticRoom', () => {
  it('draws stretched and tiled backgrounds', async () => {
    const clearRect = vi.fn();
    const fillRect = vi.fn();
    const drawImage = vi.fn();
    const context = {
      fillStyle: '',
      clearRect,
      fillRect,
      drawImage,
      save: vi.fn(),
      restore: vi.fn(),
      translate: vi.fn(),
      rotate: vi.fn(),
      scale: vi.fn()
    };

    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => context)
    } as unknown as HTMLCanvasElement;

    const room: RoomDefinition = {
      id: 1,
      name: 'Room',
      width: 640,
      height: 480,
      speed: 30,
      persistent: false,
      backgrounds: [
        {
          visible_on_start: true,
          is_foreground: false,
          source_bg: 1,
          xoffset: 0,
          yoffset: 0,
          tile_horz: false,
          tile_vert: false,
          hspeed: 0,
          vspeed: 0,
          stretch: true
        },
        {
          visible_on_start: true,
          is_foreground: false,
          source_bg: 2,
          xoffset: 50,
          yoffset: 20,
          tile_horz: true,
          tile_vert: false,
          hspeed: 0,
          vspeed: 0,
          stretch: false
        }
      ],
      views_enabled: false,
      views: [],
      instances: [],
      creation_block_id: null,
      playable: true,
      transition_targets: []
    };

    const backgroundPaths = new Map([
      [1, '/pkg/resources/backgrounds/1.png'],
      [2, '/pkg/resources/backgrounds/2.png']
    ]);
    const spritePaths = new Map();

    const stretchedImage = { id: 'stretch', width: 64, height: 64 } as unknown as HTMLImageElement;
    const tiledImage = { id: 'tile', width: 100, height: 40 } as unknown as HTMLImageElement;
    const cache = {
      getImage: vi.fn(async (src: string) => {
        if (src.endsWith('/1.png')) {
          return stretchedImage;
        }
        return tiledImage;
      })
    };

    await renderStaticRoom(canvas, room, [], backgroundPaths, spritePaths, cache as never);

    expect(canvas.width).toBe(640);
    expect(canvas.height).toBe(480);
    expect(clearRect).toHaveBeenCalledWith(0, 0, 640, 480);
    expect(fillRect).toHaveBeenNthCalledWith(1, 0, 0, 640, 480);
    expect(drawImage).toHaveBeenNthCalledWith(1, stretchedImage, 0, 0, 640, 480);
    expect(drawImage).toHaveBeenNthCalledWith(2, tiledImage, -50, 20);
    expect(drawImage).toHaveBeenNthCalledWith(3, tiledImage, 50, 20);
    expect(drawImage).toHaveBeenNthCalledWith(4, tiledImage, 150, 20);
  });

  it('draws sprite instances with origin-aware transforms and skips invisible objects', async () => {
    const clearRect = vi.fn();
    const fillRect = vi.fn();
    const drawImage = vi.fn();
    const save = vi.fn();
    const restore = vi.fn();
    const translate = vi.fn();
    const rotate = vi.fn();
    const scale = vi.fn();
    const context = {
      fillStyle: '',
      clearRect,
      fillRect,
      drawImage,
      save,
      restore,
      translate,
      rotate,
      scale
    };

    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => context)
    } as unknown as HTMLCanvasElement;

    const room: RoomDefinition = {
      id: 1,
      name: 'Room',
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
          object_id: 0,
          x: 10,
          y: 20,
          xscale: 2,
          yscale: 3,
          angle: 90,
          blend: 0xffffffff,
          creation_block_id: null,
          is_solid: false,
          is_hazard: false,
          is_checkpoint: false
        },
        {
          instance_id: 2,
          object_id: 1,
          x: 50,
          y: 60,
          xscale: 1,
          yscale: 1,
          angle: 0,
          blend: 0xffffffff,
          creation_block_id: null,
          is_solid: false,
          is_hazard: false,
          is_checkpoint: false
        },
        {
          instance_id: 3,
          object_id: 2,
          x: 70,
          y: 80,
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
      playable: true,
      transition_targets: []
    };

    const objects: ObjectDefinition[] = [
      {
        id: 0,
        name: 'Player',
        sprite_index: 0,
        parent_index: -1,
        depth: 0,
        persistent: false,
        visible: true,
        solid: false,
        mask_index: -1,
        is_hazard: null,
        is_checkpoint: null,
        is_player: true,
        events: []
      },
      {
        id: 1,
        name: 'Hidden',
        sprite_index: 1,
        parent_index: -1,
        depth: 0,
        persistent: false,
        visible: false,
        solid: false,
        mask_index: -1,
        is_hazard: null,
        is_checkpoint: null,
        is_player: false,
        events: []
      },
      {
        id: 2,
        name: 'Missing',
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
        events: []
      }
    ];

    const spritePaths = new Map([
      [
        0,
        {
          imagePath: '/pkg/resources/sprites/0-0.png',
          originX: 5,
          originY: 6
        }
      ]
    ]);
    const spriteImage = { id: 'sprite', width: 30, height: 40 } as unknown as HTMLImageElement;
    const cache = {
      getImage: vi.fn(async () => spriteImage)
    };

    await renderStaticRoom(canvas, room, objects, new Map(), spritePaths, cache as never);

    expect(save).toHaveBeenCalledTimes(1);
    expect(translate).toHaveBeenCalledWith(10, 20);
    expect(rotate).toHaveBeenCalledWith(Math.PI / 2);
    expect(scale).toHaveBeenCalledWith(2, 3);
    expect(drawImage).toHaveBeenNthCalledWith(1, spriteImage, -5, -6);
    expect(restore).toHaveBeenCalledTimes(1);
    expect(fillRect).toHaveBeenNthCalledWith(2, 66, 76, 8, 8);
  });
});
