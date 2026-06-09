import { describe, expect, it, vi } from 'vitest';
import { makeWasmRuntimeHostImports } from './wasmBridge';

describe('wasm bridge audio imports', () => {
  it('forwards wasm audio callbacks to the configured audio host', async () => {
    const audioHost = {
      playSound: vi.fn(async () => undefined),
      stopSound: vi.fn()
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
});
