import { describe, expect, it, vi } from 'vitest';
import { renderStaticRoom, resolveBackgroundDraws } from './staticRoomRenderer';
import type { ObjectDefinition, RoomDefinition } from '../types';

describe('resolveBackgroundDraws', () => {
  it('returns visible background layers with known image paths', () => {
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
      creation_block_id: null
    };

    const backgroundPaths = new Map([[1, '/pkg/resources/backgrounds/1.png']]);

    expect(resolveBackgroundDraws(room, backgroundPaths)).toEqual([
      { imagePath: '/pkg/resources/backgrounds/1.png', x: 4, y: 8 }
    ]);
  });
});

describe('renderStaticRoom', () => {
  it('draws the dark background, background images, sprites, and fallback markers', async () => {
    const clearRect = vi.fn();
    const fillRect = vi.fn();
    const drawImage = vi.fn();
    const context = {
      fillStyle: '',
      clearRect,
      fillRect,
      drawImage
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
          stretch: false
        }
      ],
      views_enabled: false,
      views: [],
      instances: [
        {
          instance_id: 1,
          object_id: 0,
          x: 10,
          y: 20,
          xscale: 1,
          yscale: 1,
          angle: 0,
          blend: 0xffffffff,
          creation_block_id: null
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
          creation_block_id: null
        }
      ],
      creation_block_id: null
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
        events: []
      },
      {
        id: 1,
        name: 'Missing',
        sprite_index: -1,
        parent_index: -1,
        depth: 0,
        persistent: false,
        visible: true,
        solid: false,
        mask_index: -1,
        events: []
      }
    ];

    const backgroundPaths = new Map([[1, '/pkg/resources/backgrounds/1.png']]);
    const spritePaths = new Map([[0, '/pkg/resources/sprites/0-0.png']]);

    const bgImage = { id: 'bg' } as unknown as HTMLImageElement;
    const spriteImage = { id: 'sprite' } as unknown as HTMLImageElement;
    const cache = {
      getImage: vi.fn(async (src: string) => {
        if (src.includes('backgrounds')) {
          return bgImage;
        }
        return spriteImage;
      })
    };

    await renderStaticRoom(canvas, room, objects, backgroundPaths, spritePaths, cache as never);

    expect(canvas.width).toBe(640);
    expect(canvas.height).toBe(480);
    expect(clearRect).toHaveBeenCalledWith(0, 0, 640, 480);
    expect(fillRect).toHaveBeenNthCalledWith(1, 0, 0, 640, 480);
    expect(drawImage).toHaveBeenNthCalledWith(1, bgImage, 0, 0);
    expect(drawImage).toHaveBeenNthCalledWith(2, spriteImage, 10, 20);
    expect(fillRect).toHaveBeenNthCalledWith(2, 46, 56, 8, 8);
  });
});
