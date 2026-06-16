import { describe, expect, it, beforeEach } from 'vitest';
import { ResourceCache } from './resourceCache';

describe('ResourceCache', () => {
  beforeEach(() => {
    // Mock Image constructor for tests
    global.Image = class MockImage {
      private _src = '';
      onload: (() => void) | null = null;
      onerror: (() => void) | null = null;

      get src() {
        return this._src;
      }

      set src(value: string) {
        this._src = value;
        // Immediately trigger onload in tests
        setTimeout(() => {
          if (this.onload) {
            this.onload();
          }
        }, 0);
      }
    } as any;
  });

  describe('getCachedImage', () => {
    it('returns cached image synchronously when already loaded', async () => {
      const cache = new ResourceCache();
      const testSrc = 'test.png';

      // Preload the image
      await cache.getImage(testSrc);

      // Should return synchronously without await
      const image = cache.getCachedImage(testSrc);

      expect(image).toBeDefined();
      expect(image.src).toContain(testSrc);
    });

    it('throws error when requesting non-cached image', () => {
      const cache = new ResourceCache();

      expect(() => {
        cache.getCachedImage('not-loaded.png');
      }).toThrow('Image not preloaded: not-loaded.png');
    });
  });
});
