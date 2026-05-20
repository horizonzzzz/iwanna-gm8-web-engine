import type { ResourceIndex } from '../types';

export type BackgroundPathMap = Map<number, string>;

export type SpriteFrame = {
  imagePath: string;
  originX: number;
  originY: number;
};

export type SpriteFrameMap = Map<number, SpriteFrame>;

export class ResourceCache {
  private readonly images = new Map<string, HTMLImageElement>();

  async getImage(src: string): Promise<HTMLImageElement> {
    const cached = this.images.get(src);
    if (cached) {
      return cached;
    }

    const image = new Image();
    const loaded = new Promise<HTMLImageElement>((resolve, reject) => {
      image.onload = () => resolve(image);
      image.onerror = () => reject(new Error(`Failed to load image: ${src}`));
    });
    image.src = src;
    this.images.set(src, image);
    return loaded;
  }
}

export function makeBackgroundPathMap(basePath: string, resources: ResourceIndex): BackgroundPathMap {
  return new Map(
    resources.backgrounds.map((background) => [background.id, `${basePath}/${background.image_path}`])
  );
}

export function makeSpriteFrameMap(basePath: string, resources: ResourceIndex): SpriteFrameMap {
  return new Map(
    resources.sprites
      .filter((sprite) => sprite.frame_paths[0])
      .map((sprite) => [
        sprite.id,
        {
          imagePath: `${basePath}/${sprite.frame_paths[0]}`,
          originX: sprite.origin_x,
          originY: sprite.origin_y
        }
      ])
  );
}
