import { describe, expect, it, vi } from 'vitest';
import { WasmRuntimeSession } from './wasmSession';
import type { WasmRuntimeBridge } from './wasmBridge';

function makeBridge(): WasmRuntimeBridge {
  const inputTrace = {
    jumpButtonKey: 0x20,
    jumpPressed: false,
    jumpJustPressed: false,
    jumpJustReleased: false,
    activeKeys: []
  };
  return {
    backend: 'opengmk-wasm',
    boot: vi.fn(async () => ({ tick: 0, roomId: 0, diagnostics: [], inputTrace })),
    snapshot: vi.fn(async () => ({ tick: 0, roomId: 0, diagnostics: [], inputTrace })),
    frame: vi.fn(async () => ({ tick: 1, roomId: 0, width: 320, height: 240, commands: [{ kind: 'present' as const }] })),
    setInput: vi.fn(async () => ({ tick: 0, roomId: 0, diagnostics: [], inputTrace })),
    tick: vi.fn(async () => ({ tick: 1, roomId: 0, diagnostics: [], inputTrace })),
    reset: vi.fn(async () => ({ tick: 0, roomId: 0, diagnostics: [], inputTrace })),
    selectRoom: vi.fn(async () => ({ tick: 0, roomId: 0, diagnostics: [], inputTrace })),
    diagnostics: vi.fn(async () => []),
  };
}

describe('WasmRuntimeSession', () => {
  it('submits input and fetches a frame for each manual step', async () => {
    const bridge = makeBridge();
    const session = new WasmRuntimeSession(bridge);
    session.setInputState({ left: true, right: false, jump: true, restart: false });

    const result = await session.stepOnce();

    expect(bridge.setInput).toHaveBeenCalledWith({
      left: true,
      right: false,
      jump: true,
      jumpPressed: true,
      jumpReleased: false,
      restart: false,
      keysHeld: [],
      keysPressed: [],
      keysReleased: [],
    });
    expect(bridge.tick).toHaveBeenCalledWith(1);
    expect(bridge.snapshot).toHaveBeenCalledTimes(1);
    expect(bridge.frame).toHaveBeenCalledTimes(1);
    expect(result.frame.width).toBe(320);
    expect(result.snapshot.tick).toBe(0);
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
      keysHeld: [],
      keysPressed: [],
      keysReleased: [],
    });
  });

  it('tracks raw virtual-key hold and edge transitions across steps', async () => {
    const bridge = makeBridge();
    const session = new WasmRuntimeSession(bridge);
    session.setInputState({ left: false, right: false, jump: false, restart: false, keysHeld: [0x10, 0x5A] });

    await session.stepOnce();

    expect(bridge.setInput).toHaveBeenNthCalledWith(1, {
      left: false,
      right: false,
      jump: false,
      jumpPressed: false,
      jumpReleased: false,
      restart: false,
      keysHeld: [0x10, 0x5A],
      keysPressed: [0x10, 0x5A],
      keysReleased: [],
    });

    session.setInputState({ left: false, right: false, jump: false, restart: false, keysHeld: [0x10] });
    await session.stepOnce();

    expect(bridge.setInput).toHaveBeenNthCalledWith(2, {
      left: false,
      right: false,
      jump: false,
      jumpPressed: false,
      jumpReleased: false,
      restart: false,
      keysHeld: [0x10],
      keysPressed: [],
      keysReleased: [0x5A],
    });
  });

  it('preserves very short raw key tap edges within a single tick', async () => {
    const bridge = makeBridge();
    const session = new WasmRuntimeSession(bridge);

    session.setInputState({ left: false, right: false, jump: false, restart: false, keysHeld: [0x10] });
    session.setInputState({ left: false, right: false, jump: false, restart: false, keysHeld: [] });

    await session.stepOnce();

    expect(bridge.setInput).toHaveBeenNthCalledWith(1, {
      left: false,
      right: false,
      jump: false,
      jumpPressed: false,
      jumpReleased: false,
      restart: false,
      keysHeld: [],
      keysPressed: [0x10],
      keysReleased: [0x10],
    });
  });

  it('re-emits raw key press edges after a full release cycle', async () => {
    const bridge = makeBridge();
    const session = new WasmRuntimeSession(bridge);

    session.setInputState({ left: false, right: false, jump: false, restart: false, keysHeld: [0x10] });
    await session.stepOnce();

    session.setInputState({ left: false, right: false, jump: false, restart: false, keysHeld: [] });
    await session.stepOnce();

    session.setInputState({ left: false, right: false, jump: false, restart: false, keysHeld: [0x10] });
    await session.stepOnce();

    expect(bridge.setInput).toHaveBeenNthCalledWith(3, {
      left: false,
      right: false,
      jump: false,
      jumpPressed: false,
      jumpReleased: false,
      restart: false,
      keysHeld: [0x10],
      keysPressed: [0x10],
      keysReleased: [],
    });
  });
});
