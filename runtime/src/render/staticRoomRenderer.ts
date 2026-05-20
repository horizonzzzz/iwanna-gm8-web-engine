import type { ObjectDefinition, RoomDefinition } from '../types';
import { ResourceCache } from './resourceCache';

export type BackgroundDraw = {
  imagePath: string;
  x: number;
  y: number;
};

function getObjectMap(objects: ObjectDefinition[]): Map<number, ObjectDefinition> {
  return new Map(objects.map((object) => [object.id, object]));
}

export function resolveBackgroundDraws(
  room: RoomDefinition,
  backgroundPaths: Map<number, string>
): BackgroundDraw[] {
  return room.backgrounds
    .filter((layer) => layer.visible_on_start && layer.source_bg >= 0)
    .flatMap((layer) => {
      const imagePath = backgroundPaths.get(layer.source_bg);
      return imagePath ? [{ imagePath, x: layer.xoffset, y: layer.yoffset }] : [];
    });
}

export async function renderStaticRoom(
  canvas: HTMLCanvasElement,
  room: RoomDefinition,
  objects: ObjectDefinition[],
  backgroundPaths: Map<number, string>,
  spritePaths: Map<number, string>,
  cache: ResourceCache = new ResourceCache()
): Promise<void> {
  canvas.width = room.width;
  canvas.height = room.height;
  const context = canvas.getContext('2d');
  if (!context) {
    throw new Error('Canvas 2d context unavailable');
  }

  context.clearRect(0, 0, room.width, room.height);
  context.fillStyle = '#0c1118';
  context.fillRect(0, 0, room.width, room.height);

  for (const background of resolveBackgroundDraws(room, backgroundPaths)) {
    const image = await cache.getImage(background.imagePath);
    context.drawImage(image, background.x, background.y);
  }

  const objectMap = getObjectMap(objects);
  for (const instance of room.instances) {
    const objectDefinition = objectMap.get(instance.object_id);
    const spritePath =
      objectDefinition?.sprite_index != null && objectDefinition.sprite_index >= 0
        ? spritePaths.get(objectDefinition.sprite_index)
        : null;

    if (spritePath && objectDefinition) {
      const image = await cache.getImage(spritePath);
      context.drawImage(image, instance.x, instance.y);
      continue;
    }

    context.fillStyle = '#60708a';
    context.fillRect(instance.x - 4, instance.y - 4, 8, 8);
  }
}
