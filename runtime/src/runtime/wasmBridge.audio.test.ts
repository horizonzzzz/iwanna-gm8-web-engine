import { describe, expect, it, vi } from 'vitest';
import { makeWasmRuntimeHostImports } from './wasmBridge';

describe('wasm bridge audio imports', () => {
  it('forwards wasm audio callbacks to the configured audio host', async () => {
    const audioHost = {
      playSound: vi.fn(async () => undefined),
      stopSound: vi.fn(),
      stopAllSounds: vi.fn(),
      isSoundPlaying: vi.fn(() => false)
    };

    const imports = makeWasmRuntimeHostImports({
      now: () => 1,
      audioHost
    });
    const env = imports.env as Record<string, (soundId: number, mode?: number) => void>;

    env.iwm_host_play_sound(6, 0);
    env.iwm_host_play_sound(7, 1);
    env.iwm_host_stop_sound(6);

    expect(audioHost.playSound).toHaveBeenNthCalledWith(1, 6, 'once');
    expect(audioHost.playSound).toHaveBeenNthCalledWith(2, 7, 'loop');
    expect(audioHost.stopSound).toHaveBeenCalledWith(6);
  });

  it('forwards wasm audio query and stop-all callbacks', () => {
    const audioHost = {
      playSound: vi.fn(),
      stopSound: vi.fn(),
      stopAllSounds: vi.fn(),
      isSoundPlaying: vi.fn(() => true)
    };

    const imports = makeWasmRuntimeHostImports({
      now: () => 1,
      audioHost
    });
    const env = imports.env as Record<string, (...args: number[]) => number | void>;

    expect(env.iwm_host_is_sound_playing(7)).toBe(1);
    env.iwm_host_stop_all_sounds();

    expect(audioHost.isSoundPlaying).toHaveBeenCalledWith(7);
    expect(audioHost.stopAllSounds).toHaveBeenCalledTimes(1);
  });
});
