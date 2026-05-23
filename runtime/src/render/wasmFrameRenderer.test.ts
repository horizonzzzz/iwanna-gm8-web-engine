import { describe, expect, it, vi } from 'vitest';
import { renderWasmFrame } from './wasmFrameRenderer';
import type { ResourceIndex } from '../types';
import type { WasmRuntimeFrame } from '../runtime/wasmBridge';

const sampleResources: ResourceIndex = {
  sprites: [
    {
      id: 0,
      name: 'Player',
      origin_x: 5,
      origin_y: 6,
      frame_paths: ['resources/sprites/0-0.png'],
      width: 30,
      height: 40,
      bbox_left: 2,
      bbox_right: 27,
      bbox_top: 3,
      bbox_bottom: 36,
    }
  ],
  backgrounds: [
    {
      id: 0,
      name: 'Bg',
      width: 320,
      height: 240,
      image_path: 'resources/backgrounds/0.png',
    }
  ],
  sounds: []
};

const sampleFrame: WasmRuntimeFrame = {
  tick: 1,
  roomId: 0,
  width: 320,
  height: 240,
  commands: [
    { kind: 'clear', colour: [12, 16, 22, 255] },
    {
      kind: 'drawBackground',
      backgroundId: 0,
      x: 3,
      y: 4,
      stretch: false,
      tileHorz: false,
      tileVert: false,
      isForeground: false,
    },
    {
      kind: 'drawTile',
      backgroundId: 0,
      x: 12,
      y: 14,
      tileX: 1,
      tileY: 2,
      width: 16,
      height: 18,
      xscale: 2,
      yscale: 1.5,
    },
    {
      kind: 'drawSprite',
      spriteId: 0,
      frameIndex: 0,
      x: 10,
      y: 20,
      originX: 5,
      originY: 6,
      xscale: 2,
      yscale: 3,
      angleDegrees: 90,
    },
    {
      kind: 'fillRect',
      x: 30,
      y: 40,
      width: 8,
      height: 9,
      colour: [96, 112, 138, 255],
    },
    { kind: 'present' },
  ],
};

describe('renderWasmFrame', () => {
  it('draws bridge frame commands onto the canvas', async () => {
    const clearRect = vi.fn();
    const fillRect = vi.fn();
    const drawImage = vi.fn();
    const save = vi.fn();
    const restore = vi.fn();
    const translate = vi.fn();
    const rotate = vi.fn();
    const scale = vi.fn();
    const context = {
      clearRect,
      fillRect,
      drawImage,
      save,
      restore,
      translate,
      rotate,
      scale,
      fillStyle: '',
    };

    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => context),
    } as unknown as HTMLCanvasElement;

    const backgroundImage = { id: 'bg', width: 320, height: 240 } as unknown as HTMLImageElement;
    const spriteImage = { id: 'sprite', width: 30, height: 40 } as unknown as HTMLImageElement;
    const cache = {
      getImage: vi.fn(async (src: string) => src.includes('backgrounds') ? backgroundImage : spriteImage),
    };

    await renderWasmFrame(canvas, sampleFrame, sampleResources, '/packages/sample', cache as never);

    expect(canvas.width).toBe(320);
    expect(canvas.height).toBe(240);
    expect(clearRect).toHaveBeenCalledWith(0, 0, 320, 240);
    expect(fillRect).toHaveBeenNthCalledWith(1, 0, 0, 320, 240);
    expect(drawImage).toHaveBeenNthCalledWith(1, backgroundImage, 3, 4);
    expect(drawImage).toHaveBeenNthCalledWith(2, backgroundImage, 1, 2, 16, 18, 12, 14, 32, 27);
    expect(save).toHaveBeenCalledTimes(1);
    expect(translate).toHaveBeenCalledWith(10, 20);
    expect(rotate).toHaveBeenCalledWith(Math.PI / 2);
    expect(scale).toHaveBeenCalledWith(2, 3);
    expect(drawImage).toHaveBeenNthCalledWith(3, spriteImage, -5, -6);
    expect(fillRect).toHaveBeenNthCalledWith(2, 30, 40, 8, 9);
    expect(restore).toHaveBeenCalledTimes(1);
  });
});
