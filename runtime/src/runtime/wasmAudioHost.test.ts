import { describe, expect, it, vi } from 'vitest';
import { createWebAudioHost } from './wasmAudioHost';
import type { RuntimePackage } from '../types';

class FakeBufferSource {
  buffer: unknown = null;
  loop = false;
  connect = vi.fn();
  start = vi.fn();
  stop = vi.fn();
}

class FakeAudioContext {
  readonly destination = {};
  readonly createdSources: FakeBufferSource[] = [];
  decodeAudioData = vi.fn(async (bytes: ArrayBuffer) => ({ byteLength: bytes.byteLength }));

  createBufferSource(): FakeBufferSource {
    const source = new FakeBufferSource();
    this.createdSources.push(source);
    return source;
  }
}

const packageWithSound = {
  resources: {
    sounds: [
      {
        id: 42,
        name: 'sndJump',
        file_path: 'resources/audio/42.wav',
        extension: 'wav',
        preload: false
      }
    ]
  }
} as RuntimePackage;

describe('wasm web audio host', () => {
  it('plays package sounds through Web Audio and stops active loops', async () => {
    const context = new FakeAudioContext();
    const fetchSound = vi.fn(async () => new Response(new Uint8Array([1, 2, 3])));
    const host = createWebAudioHost({
      audioContext: context as unknown as AudioContext,
      fetch: fetchSound
    });

    host.configurePackage(packageWithSound, '/packages/sample');
    await host.playSound(42, 'loop');
    await host.playSound(42, 'once');
    host.stopSound(42);

    expect(fetchSound).toHaveBeenCalledTimes(1);
    expect(fetchSound).toHaveBeenCalledWith('/packages/sample/resources/audio/42.wav');
    expect(context.createdSources).toHaveLength(2);
    expect(context.createdSources[0].loop).toBe(true);
    expect(context.createdSources[0].start).toHaveBeenCalledTimes(1);
    expect(context.createdSources[0].stop).toHaveBeenCalledTimes(1);
    expect(context.createdSources[1].loop).toBe(false);
    expect(context.createdSources[1].start).toHaveBeenCalledTimes(1);
  });
});
