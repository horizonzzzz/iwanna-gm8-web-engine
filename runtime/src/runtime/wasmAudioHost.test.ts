import { describe, expect, it, vi } from 'vitest';
import { createWebAudioHost } from './wasmAudioHost';
import { makeResourceIndex, makeRuntimePackage } from '../test/packageFixtures';

class FakeBufferSource {
  buffer: unknown = null;
  loop = false;
  onended: (() => void) | null = null;
  connect = vi.fn();
  start = vi.fn();
  stop = vi.fn();
}

class FakeAudioContext {
  readonly destination = {};
  readonly createdSources: FakeBufferSource[] = [];
  state: AudioContextState = 'running';
  resume = vi.fn(async () => {
    this.state = 'running';
  });
  decodeAudioData = vi.fn(async (bytes: ArrayBuffer) => ({ byteLength: bytes.byteLength }));

  createBufferSource(): FakeBufferSource {
    const source = new FakeBufferSource();
    this.createdSources.push(source);
    return source;
  }
}

const packageWithSound = makeRuntimePackage({
  resources: makeResourceIndex({
    sounds: [
      {
        id: 42,
        name: 'sndJump',
        file_path: 'resources/audio/42.wav',
        extension: 'wav',
        preload: false
      }
    ]
  })
});

describe('wasm web audio host', () => {
  it('resumes a suspended browser audio context before starting music', async () => {
    const context = new FakeAudioContext();
    context.state = 'suspended';
    const host = createWebAudioHost({
      audioContext: context as unknown as AudioContext,
      fetch: vi.fn(async () => new Response(new Uint8Array([1, 2, 3])))
    });

    host.configurePackage(packageWithSound, '/packages/sample');
    await host.playSound(42, 'loop');

    expect(context.resume).toHaveBeenCalledTimes(1);
    expect(context.createdSources[0].start).toHaveBeenCalledTimes(1);
  });

  it('queues music on the suspended context when autoplay resume is blocked', async () => {
    const context = new FakeAudioContext();
    context.state = 'suspended';
    context.resume.mockRejectedValueOnce(new DOMException('blocked', 'NotAllowedError'));
    const host = createWebAudioHost({
      audioContext: context as unknown as AudioContext,
      fetch: vi.fn(async () => new Response(new Uint8Array([1, 2, 3])))
    });

    host.configurePackage(packageWithSound, '/packages/sample');
    await host.playSound(42, 'loop');

    expect(context.createdSources[0].start).toHaveBeenCalledTimes(1);
  });

  it('replaces the active GM exclusive music channel', async () => {
    const context = new FakeAudioContext();
    const fetchSound = vi.fn(async () => new Response(new Uint8Array([1, 2, 3])));
    const host = createWebAudioHost({
      audioContext: context as unknown as AudioContext,
      fetch: fetchSound
    });
    const pkg = makeRuntimePackage({
      resources: makeResourceIndex({
        sounds: [
          { id: 1, name: 'first', file_path: '1.mp3', extension: '.mp3', preload: true, kind: 'multimedia' },
          { id: 2, name: 'second', file_path: '2.mp3', extension: '.mp3', preload: true, kind: 'multimedia' }
        ]
      })
    });

    host.configurePackage(pkg, '/packages/sample');
    await host.playSound(1, 'loop');
    await host.playSound(2, 'loop');

    expect(context.createdSources[0].stop).toHaveBeenCalledTimes(1);
    expect(host.isSoundPlaying(1)).toBe(false);
    expect(host.isSoundPlaying(2)).toBe(true);
  });

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

  it('reports active loop state and stops all sounds', async () => {
    const context = new FakeAudioContext();
    const fetchSound = vi.fn(async () => new Response(new Uint8Array([1, 2, 3])));
    const host = createWebAudioHost({
      audioContext: context as unknown as AudioContext,
      fetch: fetchSound
    });

    host.configurePackage(packageWithSound, '/packages/sample');
    expect(host.isSoundPlaying(42)).toBe(false);

    await host.playSound(42, 'loop');
    expect(host.isSoundPlaying(42)).toBe(true);

    host.stopAllSounds();
    expect(host.isSoundPlaying(42)).toBe(false);
    expect(context.createdSources[0].stop).toHaveBeenCalledTimes(1);
  });

  it('marks a loading loop active and starts it only once', async () => {
    const context = new FakeAudioContext();
    let resolveFetch: (response: Response) => void;
    const fetchSound = vi.fn(() => new Promise<Response>((resolve) => {
      resolveFetch = resolve;
    }));
    const host = createWebAudioHost({
      audioContext: context as unknown as AudioContext,
      fetch: fetchSound
    });

    host.configurePackage(packageWithSound, '/packages/sample');
    const first = host.playSound(42, 'loop');
    const duplicate = host.playSound(42, 'loop');

    expect(host.isSoundPlaying(42)).toBe(true);
    resolveFetch!(new Response(new Uint8Array([1, 2, 3])));
    await Promise.all([first, duplicate]);

    expect(context.createdSources).toHaveLength(1);
  });
});
