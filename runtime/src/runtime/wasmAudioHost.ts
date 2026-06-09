import type { RuntimePackage } from '../types';

export type WasmSoundMode = 'once' | 'loop';

export type WasmAudioHost = {
  configurePackage: (pkg: RuntimePackage, basePath: string) => void;
  playSound: (soundId: number, mode: WasmSoundMode) => Promise<void> | void;
  stopSound: (soundId: number) => void;
};

type WebAudioHostOptions = {
  audioContext?: AudioContext;
  fetch?: typeof fetch;
};

export function createWebAudioHost(options: WebAudioHostOptions = {}): WasmAudioHost {
  const context = options.audioContext ?? defaultAudioContext();
  const fetchSound = options.fetch ?? fetch.bind(globalThis);
  const buffers = new Map<number, Promise<AudioBuffer>>();
  const activeLoops = new Map<number, AudioBufferSourceNode>();
  let packageBasePath = '';
  let packageSounds = new Map<number, string>();

  async function loadBuffer(soundId: number): Promise<AudioBuffer | null> {
    const path = packageSounds.get(soundId);
    if (!path || !context) {
      return null;
    }

    const cached = buffers.get(soundId);
    if (cached) {
      return cached;
    }

    const loaded = fetchSound(`${packageBasePath}/${path}`)
      .then((response) => {
        if (!response.ok) {
          throw new Error(`Failed to load sound ${soundId}: ${response.status}`);
        }
        return response.arrayBuffer();
      })
      .then((bytes) => context.decodeAudioData(bytes));
    buffers.set(soundId, loaded);
    return loaded;
  }

  function startSource(soundId: number, buffer: AudioBuffer, mode: WasmSoundMode): void {
    if (!context) {
      return;
    }
    if (mode === 'loop') {
      activeLoops.get(soundId)?.stop();
    }

    const source = context.createBufferSource();
    source.buffer = buffer;
    source.loop = mode === 'loop';
    source.connect(context.destination);
    source.start();
    if (mode === 'loop') {
      activeLoops.set(soundId, source);
    }
  }

  return {
    configurePackage(pkg, basePath) {
      for (const source of activeLoops.values()) {
        source.stop();
      }
      packageBasePath = basePath.replace(/\/$/, '');
      packageSounds = new Map(pkg.resources.sounds.map((sound) => [sound.id, sound.file_path]));
      buffers.clear();
      activeLoops.clear();
    },
    async playSound(soundId, mode) {
      const buffer = await loadBuffer(soundId);
      if (buffer) {
        startSource(soundId, buffer, mode);
      }
    },
    stopSound(soundId) {
      activeLoops.get(soundId)?.stop();
      activeLoops.delete(soundId);
    }
  };
}

function defaultAudioContext(): AudioContext | undefined {
  const AudioContextCtor =
    globalThis.AudioContext
    ?? (globalThis as typeof globalThis & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
  return AudioContextCtor ? new AudioContextCtor() : undefined;
}
