import type { WasmRuntimeFrame } from '../runtime/wasmBridge';
import type { ResourceIndex } from '../types';
import { makeBackgroundPathMap, makeSpriteFrameMap, ResourceCache } from './resourceCache';

function rgbaToCss([r, g, b, a]: [number, number, number, number]): string {
  return `rgba(${r}, ${g}, ${b}, ${a / 255})`;
}

function resolveAxisPositions(offset: number, imageSize: number, roomSize: number, shouldTile: boolean): number[] {
  if (!shouldTile || imageSize <= 0) {
    return [offset];
  }

  let start = offset;
  while (start > 0) {
    start -= imageSize;
  }
  while (start + imageSize <= 0) {
    start += imageSize;
  }

  const positions: number[] = [];
  for (let position = start; position < roomSize; position += imageSize) {
    positions.push(position);
  }

  return positions.length > 0 ? positions : [offset];
}

function drawBackground(
  context: CanvasRenderingContext2D,
  frame: WasmRuntimeFrame,
  image: HTMLImageElement,
  command: Extract<WasmRuntimeFrame['commands'][number], { kind: 'drawBackground' }>
): void {
  if (command.stretch) {
    context.drawImage(image, 0, 0, frame.width, frame.height);
    return;
  }

  const xPositions = resolveAxisPositions(command.x, image.width, frame.width, command.tileHorz);
  const yPositions = resolveAxisPositions(command.y, image.height, frame.height, command.tileVert);

  for (const x of xPositions) {
    for (const y of yPositions) {
      context.drawImage(image, x, y);
    }
  }
}

function drawTile(
  context: CanvasRenderingContext2D,
  image: HTMLImageElement,
  command: Extract<WasmRuntimeFrame['commands'][number], { kind: 'drawTile' }>
): void {
  const drawWidth = Math.max(1, Math.round(command.width * command.xscale));
  const drawHeight = Math.max(1, Math.round(command.height * command.yscale));
  context.drawImage(
    image,
    command.tileX,
    command.tileY,
    command.width,
    command.height,
    command.x,
    command.y,
    drawWidth,
    drawHeight
  );
}

export async function renderWasmFrame(
  canvas: HTMLCanvasElement,
  frame: WasmRuntimeFrame,
  resources: ResourceIndex,
  basePath: string,
  cache: ResourceCache = new ResourceCache()
): Promise<void> {
  if (canvas.width !== frame.width) {
    canvas.width = frame.width;
  }
  if (canvas.height !== frame.height) {
    canvas.height = frame.height;
  }
  const context = canvas.getContext('2d');
  if (!context) {
    throw new Error('Canvas 2d context unavailable');
  }

  const backgroundPaths = makeBackgroundPathMap(basePath, resources);
  const spritePaths = makeSpriteFrameMap(basePath, resources);

  for (const command of frame.commands) {
    switch (command.kind) {
      case 'clear':
        context.clearRect(0, 0, frame.width, frame.height);
        context.fillStyle = rgbaToCss(command.colour);
        context.fillRect(0, 0, frame.width, frame.height);
        break;
      case 'drawBackground': {
        const path = backgroundPaths.get(command.backgroundId);
        if (!path) {
          continue;
        }

        const image = await cache.getImage(path);
        drawBackground(context, frame, image, command);
        break;
      }
      case 'drawTile': {
        const path = backgroundPaths.get(command.backgroundId);
        if (!path) {
          continue;
        }

        const image = await cache.getImage(path);
        drawTile(context, image, command);
        break;
      }
      case 'drawSprite': {
        const sprite = spritePaths.get(command.spriteId);
        if (!sprite) {
          continue;
        }

        const image = await cache.getImage(sprite.imagePath);
        context.save();
        context.translate(command.x, command.y);
        if (command.angleDegrees !== 0) {
          context.rotate((command.angleDegrees * Math.PI) / 180);
        }
        if (command.xscale !== 1 || command.yscale !== 1) {
          context.scale(command.xscale, command.yscale);
        }
        context.drawImage(image, -command.originX, -command.originY);
        context.restore();
        break;
      }
      case 'fillRect':
        context.fillStyle = rgbaToCss(command.colour);
        context.fillRect(command.x, command.y, command.width, command.height);
        break;
      case 'present':
        break;
    }
  }
}
