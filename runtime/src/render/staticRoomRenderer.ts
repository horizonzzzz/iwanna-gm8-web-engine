import type { ObjectDefinition, RoomDefinition } from '../types';
import type { BackgroundPathMap, SpriteFrame, SpriteFrameMap } from './resourceCache';
import { ResourceCache } from './resourceCache';

export type BackgroundDraw = {
  imagePath: string;
  x: number;
  y: number;
  stretch: boolean;
  tileHorz: boolean;
  tileVert: boolean;
  isForeground: boolean;
};

export type TileDraw = {
  imagePath: string;
  x: number;
  y: number;
  tileX: number;
  tileY: number;
  width: number;
  height: number;
  depth: number;
  xscale: number;
  yscale: number;
};

function getObjectMap(objects: ObjectDefinition[]): Map<number, ObjectDefinition> {
  return new Map(objects.map((object) => [object.id, object]));
}

export function resolveBackgroundDraws(
  room: RoomDefinition,
  backgroundPaths: BackgroundPathMap
): BackgroundDraw[] {
  return room.backgrounds
    .filter((layer) => layer.visible_on_start && layer.source_bg >= 0)
    .flatMap((layer) => {
      const imagePath = backgroundPaths.get(layer.source_bg);
      return imagePath
        ? [
            {
              imagePath,
              x: layer.xoffset,
              y: layer.yoffset,
              stretch: layer.stretch,
              tileHorz: layer.tile_horz,
              tileVert: layer.tile_vert,
              isForeground: layer.is_foreground
            }
          ]
        : [];
    });
}

export function resolveTileDraws(
  room: RoomDefinition,
  backgroundPaths: BackgroundPathMap
): TileDraw[] {
  return [...room.tiles]
    .filter((tile) => tile.source_bg >= 0)
    .sort((left, right) => left.depth - right.depth)
    .flatMap((tile) => {
      const imagePath = backgroundPaths.get(tile.source_bg);
      return imagePath
        ? [{
            imagePath,
            x: tile.x,
            y: tile.y,
            tileX: tile.tile_x,
            tileY: tile.tile_y,
            width: tile.width,
            height: tile.height,
            depth: tile.depth,
            xscale: tile.xscale,
            yscale: tile.yscale
          }]
        : [];
    });
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

function drawBackgroundLayer(
  context: CanvasRenderingContext2D,
  room: RoomDefinition,
  draw: BackgroundDraw,
  image: HTMLImageElement
): void {
  if (draw.stretch) {
    context.drawImage(image, 0, 0, room.width, room.height);
    return;
  }

  const xPositions = resolveAxisPositions(draw.x, image.width, room.width, draw.tileHorz);
  const yPositions = resolveAxisPositions(draw.y, image.height, room.height, draw.tileVert);

  for (const x of xPositions) {
    for (const y of yPositions) {
      context.drawImage(image, x, y);
    }
  }
}

function drawSpriteInstance(
  context: CanvasRenderingContext2D,
  image: HTMLImageElement,
  instance: RoomDefinition['instances'][number],
  sprite: SpriteFrame
): void {
  context.save();
  context.translate(instance.x, instance.y);
  if (instance.angle !== 0) {
    context.rotate((instance.angle * Math.PI) / 180);
  }
  if (instance.xscale !== 1 || instance.yscale !== 1) {
    context.scale(instance.xscale, instance.yscale);
  }
  context.drawImage(image, -sprite.originX, -sprite.originY);
  context.restore();
}

function drawTileInstance(
  context: CanvasRenderingContext2D,
  image: HTMLImageElement,
  tile: TileDraw
): void {
  const drawWidth = Math.max(1, Math.round(tile.width * tile.xscale));
  const drawHeight = Math.max(1, Math.round(tile.height * tile.yscale));
  context.drawImage(
    image,
    tile.tileX,
    tile.tileY,
    tile.width,
    tile.height,
    tile.x,
    tile.y,
    drawWidth,
    drawHeight
  );
}

export async function renderStaticRoom(
  canvas: HTMLCanvasElement,
  room: RoomDefinition,
  objects: ObjectDefinition[],
  backgroundPaths: BackgroundPathMap,
  spritePaths: SpriteFrameMap,
  cache: ResourceCache = new ResourceCache()
): Promise<void> {
  if (canvas.width !== room.width) canvas.width = room.width;
  if (canvas.height !== room.height) canvas.height = room.height;
  const context = canvas.getContext('2d');
  if (!context) {
    throw new Error('Canvas 2d context unavailable');
  }

  if (room.clear_screen !== false) {
    const colour = room.background_colour ?? 0;
    context.clearRect(0, 0, room.width, room.height);
    context.fillStyle = `rgb(${colour & 0xff}, ${(colour >>> 8) & 0xff}, ${(colour >>> 16) & 0xff})`;
    context.fillRect(0, 0, room.width, room.height);
  }

  const backgroundDraws = resolveBackgroundDraws(room, backgroundPaths);
  const tileDraws = resolveTileDraws(room, backgroundPaths);

  for (const background of backgroundDraws.filter((draw) => !draw.isForeground)) {
    const image = await cache.getImage(background.imagePath);
    drawBackgroundLayer(context, room, background, image);
  }

  for (const tile of tileDraws) {
    const image = await cache.getImage(tile.imagePath);
    drawTileInstance(context, image, tile);
  }

  const objectMap = getObjectMap(objects);
  for (const instance of room.instances) {
    const objectDefinition = objectMap.get(instance.object_id);
    if (objectDefinition && !objectDefinition.visible) {
      continue;
    }

    const sprite =
      objectDefinition?.sprite_index != null && objectDefinition.sprite_index >= 0
        ? spritePaths.get(objectDefinition.sprite_index)
        : null;

    if (sprite && objectDefinition) {
      const firstFrame = sprite.frames[0];
      if (!firstFrame) {
        continue;
      }
      const image = await cache.getImage(firstFrame.imagePath);
      drawSpriteInstance(context, image, instance, firstFrame);
      continue;
    }

    context.fillStyle = '#60708a';
    context.fillRect(instance.x - 4, instance.y - 4, 8, 8);
  }

  for (const background of backgroundDraws.filter((draw) => draw.isForeground)) {
    const image = await cache.getImage(background.imagePath);
    drawBackgroundLayer(context, room, background, image);
  }
}
