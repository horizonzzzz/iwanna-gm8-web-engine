import { describe, expect, it } from 'vitest';
import {
  describeWasmBridgeAvailability,
  isWasmRuntimeBridge,
  loadWasmRuntimeBridge,
  type WasmRuntimeBridge,
} from './wasmBridge';

function makeBridge(): WasmRuntimeBridge {
  return {
    backend: 'opengmk-wasm',
    boot: async () => ({ tick: 0, roomId: 0, diagnostics: [] }),
    tick: async (frames = 1) => ({ tick: frames, roomId: 0, diagnostics: [] }),
    diagnostics: async () => [],
  };
}

describe('wasm bridge loader', () => {
  it('accepts a valid bridge module', async () => {
    const bridge = await loadWasmRuntimeBridge(async () => ({
      initRuntimeHost: async () => makeBridge(),
    }));

    expect(isWasmRuntimeBridge(bridge)).toBe(true);
    expect(bridge.backend).toBe('opengmk-wasm');
  });

  it('rejects modules without the expected initializer', async () => {
    await expect(loadWasmRuntimeBridge(async () => ({}))).rejects.toThrow(
      'WASM bridge module is missing initRuntimeHost()'
    );
  });

  it('describes configured and missing bridge states clearly', () => {
    expect(describeWasmBridgeAvailability(makeBridge(), null)).toContain('WASM bridge available');
    expect(describeWasmBridgeAvailability(null, new Error('module fetch failed'))).toContain(
      'module fetch failed'
    );
    expect(describeWasmBridgeAvailability(null, null)).toContain('No WASM bridge configured');
  });
});
