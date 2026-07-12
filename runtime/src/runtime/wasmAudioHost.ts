import type { RuntimePackage } from '../types';

export type WasmSoundMode = 'once' | 'loop';

export type WasmAudioHost = {
  configurePackage: (pkg: RuntimePackage, basePath: string) => void;
  playSound: (soundId: number, mode: WasmSoundMode) => Promise<void> | void;
  stopSound: (soundId: number) => void;
  stopAllSounds: () => void;
  isSoundPlaying: (soundId: number) => boolean;
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
  const playingSounds = new Set<number>();
  let packageBasePath = '';
  let packageSounds = new Map<number, string>();
  let packageSoundKinds = new Map<number, string>();
  let activeBackgroundMusic: { soundId: number; source: AudioBufferSourceNode } | undefined;

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
      stopSource(activeLoops.get(soundId));
    }
    if (isExclusiveMusicKind(packageSoundKinds.get(soundId))) {
      if (activeBackgroundMusic?.soundId !== soundId) {
        stopSource(activeBackgroundMusic?.source);
        if (activeBackgroundMusic) {
          activeLoops.delete(activeBackgroundMusic.soundId);
          playingSounds.delete(activeBackgroundMusic.soundId);
        }
      }
    }

    const source = context.createBufferSource();
    source.buffer = buffer;
    source.loop = mode === 'loop';
    source.onended = () => {
      if (mode !== 'loop') {
        playingSounds.delete(soundId);
      }
    };
    source.connect(context.destination);
    playingSounds.add(soundId);
    source.start();
    if (mode === 'loop') {
      activeLoops.set(soundId, source);
    }
    if (isExclusiveMusicKind(packageSoundKinds.get(soundId))) {
      activeBackgroundMusic = { soundId, source };
    }
  }

  function stopSource(source: AudioBufferSourceNode | undefined): void {
    try {
      source?.stop();
    } catch (error) {
      if (!(error instanceof DOMException) || error.name !== 'InvalidStateError') {
        throw error;
      }
    }
  }

  return {
    configurePackage(pkg, basePath) {
      for (const source of activeLoops.values()) {
        stopSource(source);
      }
      packageBasePath = basePath.replace(/\/$/, '');
      packageSounds = new Map(pkg.resources.sounds.map((sound) => [sound.id, sound.file_path]));
      packageSoundKinds = new Map(pkg.resources.sounds.map((sound) => [sound.id, sound.kind ?? 'normal']));
      buffers.clear();
      activeLoops.clear();
      playingSounds.clear();
      activeBackgroundMusic = undefined;
    },
    async playSound(soundId, mode) {
      const buffer = await loadBuffer(soundId);
      if (buffer) {
        startSource(soundId, buffer, mode);
      }
    },
    stopSound(soundId) {
      stopSource(activeLoops.get(soundId));
      activeLoops.delete(soundId);
      playingSounds.delete(soundId);
      if (activeBackgroundMusic?.soundId === soundId) {
        activeBackgroundMusic = undefined;
      }
    },
    stopAllSounds() {
      for (const source of activeLoops.values()) {
        stopSource(source);
      }
      activeLoops.clear();
      playingSounds.clear();
      activeBackgroundMusic = undefined;
    },
    isSoundPlaying(soundId) {
      return playingSounds.has(soundId);
    }
  };
}

function isExclusiveMusicKind(kind: string | undefined): boolean {
  return kind === 'background-music' || kind === 'multimedia';
}

function defaultAudioContext(): AudioContext | undefined {
  const AudioContextCtor =
    globalThis.AudioContext
    ?? (globalThis as typeof globalThis & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
  return AudioContextCtor ? new AudioContextCtor() : undefined;
}
