import { describe, expect, it, vi } from 'vitest';
import { WasmRuntimeSession } from './wasmSession';
import type { WasmRuntimeBridge } from './wasmBridge';

function makeBridge(): WasmRuntimeBridge {
  return {
    backend: 'opengmk-wasm',
    boot: vi.fn(async () => ({ tick: 0, roomId: 0, diagnostics: [] })),
    snapshot: vi.fn(async () => ({ tick: 0, roomId: 0, diagnostics: [] })),
    frame: vi.fn(async () => ({ tick: 1, roomId: 0, width: 320, height: 240, commands: [{ kind: 'present' as const }] })),
    setInput: vi.fn(async () => ({ tick: 0, roomId: 0, diagnostics: [] })),
    tick: vi.fn(async () => ({ tick: 1, roomId: 0, diagnostics: [] })),
    reset: vi.fn(async () => ({ tick: 0, roomId: 0, diagnostics: [] })),
    selectRoom: vi.fn(async () => ({ tick: 0, roomId: 0, diagnostics: [] })),
    diagnostics: vi.fn(async () => []),
  };
}

describe('WasmRuntimeSession', () => {
  it('submits input and fetches a frame for each manual step', async () => {
    const bridge = makeBridge();
    const session = new WasmRuntimeSession(bridge);
    session.setInputState({ left: true, right: false, jump: true, restart: false });

    const frame = await session.stepOnce();

    expect(bridge.setInput).toHaveBeenCalledWith({
      left: true,
      right: false,
      jump: true,
      jumpPressed: true,
      jumpReleased: false,
      restart: false,
    });
    expect(bridge.tick).toHaveBeenCalledWith(1);
    expect(bridge.frame).toHaveBeenCalledTimes(1);
    expect(frame.width).toBe(320);
  });

  it('clears jump edge transitions after a step', async () => {
    const bridge = makeBridge();
    const session = new WasmRuntimeSession(bridge);
    session.setInputState({ left: false, right: false, jump: true, restart: false });

    await session.stepOnce();
    await session.stepOnce();

    expect(bridge.setInput).toHaveBeenNthCalledWith(2, {
      left: false,
      right: false,
      jump: true,
      jumpPressed: false,
      jumpReleased: false,
      restart: false,
    });
  });
});
