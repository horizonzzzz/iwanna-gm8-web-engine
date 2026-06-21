import { describe, expect, it, vi } from 'vitest';
import { renderWasmFrame } from './wasmFrameRenderer';
import { makeResourceIndex, makeWasmFrame } from '../test/packageFixtures';

const sampleResources = makeResourceIndex({
  sprites: [
    {
      id: 0,
      name: 'Player',
      origin_x: 5,
      origin_y: 6,
      frame_paths: ['resources/sprites/0-0.png', 'resources/sprites/0-1.png'],
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
});

const sampleFrame = makeWasmFrame({
  tick: 1,
  roomId: 0,
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
    {
      kind: 'drawText',
      text: 'GAME OVER',
      x: 160,
      y: 88,
      size: 32,
      colour: [232, 36, 48, 220],
      align: 'center',
    },
    { kind: 'present' },
  ],
});

function makeMockContext(overrides: Record<string, unknown> = {}) {
  return {
    clearRect: vi.fn(),
    fillRect: vi.fn(),
    drawImage: vi.fn(),
    fillText: vi.fn(),
    save: vi.fn(),
    restore: vi.fn(),
    translate: vi.fn(),
    rotate: vi.fn(),
    scale: vi.fn(),
    fillStyle: '',
    font: '',
    textAlign: '',
    textBaseline: '',
    ...overrides,
  };
}

function makeMockCanvas(context: object): HTMLCanvasElement {
  return {
    width: 0,
    height: 0,
    getContext: vi.fn(() => context),
  } as unknown as HTMLCanvasElement;
}

function bitmapFontResources(offset: number) {
  return makeResourceIndex({
    fonts: [
      {
        id: 0,
        name: 'font12',
        system_name: 'System',
        size: 12,
        bold: false,
        italic: false,
        range_start: 32,
        range_end: 127,
        map_width: 16,
        map_height: 8,
        image_path: 'resources/fonts/0.png',
        glyphs: [
          { code: 65, x: 2, y: 3, width: 4, height: 5, offset, advance: 6 },
        ],
      },
    ],
  });
}

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
    const fillText = vi.fn();
    const context = {
      clearRect,
      fillRect,
      drawImage,
      fillText,
      save,
      restore,
      translate,
      rotate,
      scale,
      fillStyle: '',
      font: '',
      textAlign: '',
      textBaseline: '',
    };

    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => context),
    } as unknown as HTMLCanvasElement;

    const backgroundImage = { id: 'bg', width: 320, height: 240 } as unknown as HTMLImageElement;
    const spriteImage0 = { id: 'sprite-0', width: 30, height: 40 } as unknown as HTMLImageElement;
    const spriteImage1 = { id: 'sprite-1', width: 30, height: 40 } as unknown as HTMLImageElement;
    const cache = {
      getImage: vi.fn(async (src: string) => {
        if (src.includes('backgrounds')) {
          return backgroundImage;
        }
        return src.includes('0-1') ? spriteImage1 : spriteImage0;
      }),
      getCachedImage: vi.fn((src: string) => {
        if (src.includes('backgrounds')) {
          return backgroundImage;
        }
        return src.includes('0-1') ? spriteImage1 : spriteImage0;
      }),
    };

    await renderWasmFrame(canvas, sampleFrame, sampleResources, '/packages/sample', cache as never);

    expect(canvas.width).toBe(320);
    expect(canvas.height).toBe(240);
    expect(clearRect).toHaveBeenCalledWith(0, 0, 320, 240);
    expect(fillRect).toHaveBeenNthCalledWith(1, 0, 0, 320, 240);
    expect(drawImage).toHaveBeenNthCalledWith(1, backgroundImage, 3, 4);
    expect(drawImage).toHaveBeenNthCalledWith(2, backgroundImage, 1, 2, 16, 18, 12, 14, 32, 27);
    expect(save).toHaveBeenCalled();
    expect(translate).toHaveBeenCalledWith(10, 20);
    expect(rotate).toHaveBeenCalledWith(Math.PI / 2);
    expect(scale).toHaveBeenCalledWith(2, 3);
    expect(drawImage).toHaveBeenNthCalledWith(3, spriteImage0, -5, -6);
    expect(fillRect).toHaveBeenNthCalledWith(2, 30, 40, 8, 9);
    expect(fillText).toHaveBeenCalledWith('GAME OVER', 160, 88);
    expect(context.font).toBe('32px sans-serif');
    expect(context.textAlign).toBe('center');
    expect(context.textBaseline).toBe('top');
    expect(save).toHaveBeenCalledTimes(2);
    expect(restore).toHaveBeenCalledTimes(2);
  });

  it('uses package font metadata for text commands', async () => {
    const fillText = vi.fn();
    const context = {
      clearRect: vi.fn(),
      fillRect: vi.fn(),
      drawImage: vi.fn(),
      fillText,
      save: vi.fn(),
      restore: vi.fn(),
      translate: vi.fn(),
      rotate: vi.fn(),
      scale: vi.fn(),
      fillStyle: '',
      font: '',
      textAlign: '',
      textBaseline: '',
    };
    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => context),
    } as unknown as HTMLCanvasElement;
    const resources = makeResourceIndex({
      fonts: [
        {
          id: 0,
          name: 'font12',
          system_name: 'MS Gothic',
          size: 12,
          bold: false,
          italic: false,
        },
      ],
    } as never);
    const frame = makeWasmFrame({
      commands: [
        {
          kind: 'drawText',
          text: 'Medium',
          x: 64,
          y: 96,
          size: 12,
          fontName: 'font12',
          fontBold: false,
          fontItalic: false,
          colour: [255, 0, 0, 255],
          align: 'left',
        } as never,
      ],
    });

    await renderWasmFrame(canvas, frame, resources, '/packages/sample');

    expect(fillText).toHaveBeenCalledWith('Medium', 64, 96);
    expect(context.font).toBe('12px "MS Gothic", sans-serif');
  });

  it('renders drawText through parsed GM font atlas when glyph data is available', async () => {
    const finalDrawImage = vi.fn();
    const finalContext = makeMockContext({ drawImage: finalDrawImage });
    const maskContext = makeMockContext({
      globalCompositeOperation: '',
      fillStyle: '',
    });
    const maskCanvas = makeMockCanvas(maskContext);
    const createElement = vi.spyOn(document, 'createElement').mockReturnValue(maskCanvas);
    const canvas = makeMockCanvas(finalContext);
    const fontAtlas = { id: 'font-atlas', width: 16, height: 8 } as unknown as HTMLImageElement;
    const resources = bitmapFontResources(1);
    const frame = makeWasmFrame({
      commands: [
        {
          kind: 'drawText',
          text: 'A',
          x: 64,
          y: 96,
          size: 12,
          fontName: 'font12',
          fontBold: false,
          fontItalic: false,
          colour: [255, 0, 0, 255],
          align: 'left',
        },
      ],
    });
    const cache = {
      getImage: vi.fn(async () => fontAtlas),
      getCachedImage: vi.fn(() => fontAtlas),
    };

    await renderWasmFrame(canvas, frame, resources, '/packages/sample', cache as never);

    expect(cache.getImage).toHaveBeenCalledWith('/packages/sample/resources/fonts/0.png');
    expect(createElement).toHaveBeenCalledWith('canvas');
    expect(maskCanvas.width).toBe(4);
    expect(maskCanvas.height).toBe(5);
    expect(maskContext.drawImage).toHaveBeenCalledWith(fontAtlas, 2, 3, 4, 5, 0, 0, 4, 5);
    expect(maskContext.globalCompositeOperation).toBe('source-in');
    expect(maskContext.fillStyle).toBe('rgba(255, 0, 0, 1)');
    expect(maskContext.fillRect).toHaveBeenCalledWith(0, 0, 4, 5);
    expect(finalDrawImage).toHaveBeenCalledWith(maskCanvas, 65, 96);
    expect(finalContext.fillText).not.toHaveBeenCalled();

    createElement.mockRestore();
  });

  it('preserves GM font glyph overhangs when bitmap text has negative offsets', async () => {
    const finalDrawImage = vi.fn();
    const finalContext = makeMockContext({ drawImage: finalDrawImage });
    const maskContext = makeMockContext({
      globalCompositeOperation: '',
      fillStyle: '',
    });
    const maskCanvas = makeMockCanvas(maskContext);
    const createElement = vi.spyOn(document, 'createElement').mockReturnValue(maskCanvas);
    const canvas = makeMockCanvas(finalContext);
    const fontAtlas = { id: 'font-atlas', width: 16, height: 8 } as unknown as HTMLImageElement;
    const resources = bitmapFontResources(-2);
    const frame = makeWasmFrame({
      commands: [
        {
          kind: 'drawText',
          text: 'A',
          x: 64,
          y: 96,
          size: 12,
          fontName: 'font12',
          fontBold: false,
          fontItalic: false,
          colour: [255, 0, 0, 255],
          align: 'left',
        },
      ],
    });
    const cache = {
      getImage: vi.fn(async () => fontAtlas),
      getCachedImage: vi.fn(() => fontAtlas),
    };

    await renderWasmFrame(canvas, frame, resources, '/packages/sample', cache as never);

    expect(maskCanvas.width).toBe(4);
    expect(maskCanvas.height).toBe(5);
    expect(maskContext.drawImage).toHaveBeenCalledWith(fontAtlas, 2, 3, 4, 5, 0, 0, 4, 5);
    expect(finalDrawImage).toHaveBeenCalledWith(maskCanvas, 62, 96);

    createElement.mockRestore();
  });

  it('applies sprite alpha only while drawing the sprite', async () => {
    const alphaDuringDraw: number[] = [];
    const alphaStack: number[] = [];
    const context = {
      clearRect: vi.fn(),
      fillRect: vi.fn(),
      drawImage: vi.fn(() => {
        alphaDuringDraw.push(context.globalAlpha);
      }),
      fillText: vi.fn(),
      save: vi.fn(() => {
        alphaStack.push(context.globalAlpha);
      }),
      restore: vi.fn(() => {
        context.globalAlpha = alphaStack.pop() ?? 1;
      }),
      translate: vi.fn(),
      rotate: vi.fn(),
      scale: vi.fn(),
      globalAlpha: 1,
      fillStyle: '',
      font: '',
      textAlign: '',
      textBaseline: '',
    };
    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => context),
    } as unknown as HTMLCanvasElement;
    const cache = {
      getImage: vi.fn(async () => ({ width: 30, height: 40 }) as unknown as HTMLImageElement),
      getCachedImage: vi.fn(() => ({ width: 30, height: 40 }) as unknown as HTMLImageElement),
    };
    const frame = makeWasmFrame({
      tick: 1,
      roomId: 0,
      commands: [
        {
          kind: 'drawSprite',
          spriteId: 0,
          frameIndex: 0,
          x: 10,
          y: 20,
          originX: 5,
          originY: 6,
          xscale: 1,
          yscale: 1,
          alpha: 0.7,
          angleDegrees: 0,
        },
        { kind: 'present' },
      ],
    });

    await renderWasmFrame(canvas, frame, sampleResources, '/packages/sample', cache as never);

    expect(alphaDuringDraw).toEqual([0.7]);
    expect(context.globalAlpha).toBe(1);
  });

  it('does not reset canvas dimensions when the frame size is unchanged', async () => {
    const clearRect = vi.fn();
    const fillRect = vi.fn();
    const drawImage = vi.fn();
    const fillText = vi.fn();
    const context = {
      clearRect,
      fillRect,
      drawImage,
      fillText,
      save: vi.fn(),
      restore: vi.fn(),
      translate: vi.fn(),
      rotate: vi.fn(),
      scale: vi.fn(),
      fillStyle: '',
      font: '',
      textAlign: '',
      textBaseline: '',
    };

    let width = 320;
    let height = 240;
    let widthSetCount = 0;
    let heightSetCount = 0;
    const canvas = {
      get width() {
        return width;
      },
      set width(value: number) {
        widthSetCount++;
        width = value;
      },
      get height() {
        return height;
      },
      set height(value: number) {
        heightSetCount++;
        height = value;
      },
      getContext: vi.fn(() => context),
    } as unknown as HTMLCanvasElement;

    const backgroundImage = { id: 'bg', width: 320, height: 240 } as unknown as HTMLImageElement;
    const spriteImage0 = { id: 'sprite-0', width: 30, height: 40 } as unknown as HTMLImageElement;
    const spriteImage1 = { id: 'sprite-1', width: 30, height: 40 } as unknown as HTMLImageElement;
    const cache = {
      getImage: vi.fn(async (src: string) => {
        if (src.includes('backgrounds')) {
          return backgroundImage;
        }
        return src.includes('0-1') ? spriteImage1 : spriteImage0;
      }),
      getCachedImage: vi.fn((src: string) => {
        if (src.includes('backgrounds')) {
          return backgroundImage;
        }
        return src.includes('0-1') ? spriteImage1 : spriteImage0;
      }),
    };

    await renderWasmFrame(canvas, sampleFrame, sampleResources, '/packages/sample', cache as never);
    await renderWasmFrame(canvas, sampleFrame, sampleResources, '/packages/sample', cache as never);

    expect(width).toBe(320);
    expect(height).toBe(240);
    expect(widthSetCount).toBe(0);
    expect(heightSetCount).toBe(0);
    expect(clearRect).toHaveBeenCalledTimes(2);
    expect(fillRect).toHaveBeenNthCalledWith(1, 0, 0, 320, 240);
  });
});

describe('renderWasmFrame - preloading', () => {
  it('preloads all unique images before rendering', async () => {
    const context = {
      clearRect: vi.fn(),
      fillRect: vi.fn(),
      drawImage: vi.fn(),
      save: vi.fn(),
      restore: vi.fn(),
      translate: vi.fn(),
      rotate: vi.fn(),
      scale: vi.fn(),
      fillStyle: '',
      font: '',
      textAlign: '',
      textBaseline: '',
    };

    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => context),
    } as unknown as HTMLCanvasElement;

    const getImageCalls: string[] = [];
    const getCachedImageCalls: string[] = [];

    const backgroundImage = { id: 'bg' } as unknown as HTMLImageElement;
    const spriteImage0 = { id: 'sprite-0' } as unknown as HTMLImageElement;
    const spriteImage1 = { id: 'sprite-1' } as unknown as HTMLImageElement;

    const cache = {
      getImage: vi.fn(async (src: string) => {
        getImageCalls.push(src);
        if (src.includes('backgrounds')) {
          return backgroundImage;
        }
        return src.includes('0-1') ? spriteImage1 : spriteImage0;
      }),
      getCachedImage: vi.fn((src: string) => {
        getCachedImageCalls.push(src);
        if (src.includes('backgrounds')) {
          return backgroundImage;
        }
        return src.includes('0-1') ? spriteImage1 : spriteImage0;
      }),
    };

    const frameWithDuplicates = makeWasmFrame({
      tick: 1,
      roomId: 0,
      commands: [
        { kind: 'clear', colour: [0, 0, 0, 255] },
        { kind: 'drawSprite', spriteId: 0, frameIndex: 0, x: 10, y: 20, originX: 5, originY: 6, xscale: 1, yscale: 1, angleDegrees: 0 },
        { kind: 'drawSprite', spriteId: 0, frameIndex: 0, x: 30, y: 40, originX: 5, originY: 6, xscale: 1, yscale: 1, angleDegrees: 0 },
        { kind: 'drawSprite', spriteId: 0, frameIndex: 0, x: 50, y: 60, originX: 5, originY: 6, xscale: 1, yscale: 1, angleDegrees: 0 },
        { kind: 'present' },
      ],
    });

    await renderWasmFrame(canvas, frameWithDuplicates, sampleResources, '/packages/sample', cache as never);

    // Should call getImage once per unique image during preload
    expect(getImageCalls.length).toBe(1);
    expect(getImageCalls[0]).toContain('sprites/0-0.png');

    // Should call getCachedImage for each draw command during render
    expect(getCachedImageCalls.length).toBe(3);
    expect(getCachedImageCalls.every(call => call.includes('sprites/0-0.png'))).toBe(true);
  });

  it('uses frameIndex to select the requested sprite frame', async () => {
    const context = {
      clearRect: vi.fn(),
      fillRect: vi.fn(),
      drawImage: vi.fn(),
      save: vi.fn(),
      restore: vi.fn(),
      translate: vi.fn(),
      rotate: vi.fn(),
      scale: vi.fn(),
      fillStyle: '',
      font: '',
      textAlign: '',
      textBaseline: '',
    };

    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => context),
    } as unknown as HTMLCanvasElement;

    const spriteImage0 = { id: 'sprite-0' } as unknown as HTMLImageElement;
    const spriteImage1 = { id: 'sprite-1' } as unknown as HTMLImageElement;
    const cache = {
      getImage: vi.fn(async (src: string) => src.includes('0-1') ? spriteImage1 : spriteImage0),
      getCachedImage: vi.fn((src: string) => src.includes('0-1') ? spriteImage1 : spriteImage0),
    };

    await renderWasmFrame(
      canvas,
      makeWasmFrame({
        tick: 1,
        roomId: 0,
        commands: [
          { kind: 'clear', colour: [0, 0, 0, 255] },
          { kind: 'drawSprite', spriteId: 0, frameIndex: 1, x: 10, y: 20, originX: 5, originY: 6, xscale: 1, yscale: 1, angleDegrees: 0 },
          { kind: 'present' },
        ],
      }),
      sampleResources,
      '/packages/sample',
      cache as never
    );

    expect(cache.getImage).toHaveBeenCalledWith('/packages/sample/resources/sprites/0-1.png');
    expect(cache.getCachedImage).toHaveBeenCalledWith('/packages/sample/resources/sprites/0-1.png');
    expect(context.drawImage).toHaveBeenCalledWith(spriteImage1, -5, -6);
  });
});
