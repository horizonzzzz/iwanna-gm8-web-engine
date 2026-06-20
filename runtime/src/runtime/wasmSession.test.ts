import { describe, expect, it, vi } from 'vitest';
import { WasmRuntimeSession } from './wasmSession';
import type { WasmRuntimeBridge } from './wasmBridge';
import { makeWasmFrame, makeWasmSnapshot } from '../test/packageFixtures';

function makeBridge(): WasmRuntimeBridge {
  return {
    backend: 'opengmk-wasm',
    boot: vi.fn(async () => makeWasmSnapshot({ roomId: 0 })),
    snapshot: vi.fn(async () => makeWasmSnapshot({ roomId: 0 })),
    frame: vi.fn(async () => makeWasmFrame({ tick: 1, roomId: 0 })),
    setInput: vi.fn(async () => makeWasmSnapshot({ roomId: 0 })),
    step: vi.fn(async () => ({
      snapshot: makeWasmSnapshot({ tick: 1, roomId: 0 }),
      frame: makeWasmFrame({ tick: 1, roomId: 0 })
    })),
    tick: vi.fn(async () => makeWasmSnapshot({ tick: 1, roomId: 0 })),
    reset: vi.fn(async () => makeWasmSnapshot({ roomId: 0 })),
    selectRoom: vi.fn(async () => makeWasmSnapshot({ roomId: 0 })),
    diagnostics: vi.fn(async () => []),
  };
}

describe('WasmRuntimeSession', () => {
  it('submits input and fetches a frame for each manual step', async () => {
    const bridge = makeBridge();
    const session = new WasmRuntimeSession(bridge);
    session.setInputState({ left: true, right: false, jump: true, restart: false });

    const result = await session.stepOnce();

    expect(bridge.step).toHaveBeenCalledWith({
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
    expect(bridge.setInput).not.toHaveBeenCalled();
    expect(bridge.tick).not.toHaveBeenCalled();
    expect(bridge.snapshot).not.toHaveBeenCalled();
    expect(bridge.frame).not.toHaveBeenCalled();
    expect(result.frame.width).toBe(320);
    expect(result.snapshot.tick).toBe(1);
  });

  it('clears jump edge transitions after a step', async () => {
    const bridge = makeBridge();
    const session = new WasmRuntimeSession(bridge);
    session.setInputState({ left: false, right: false, jump: true, restart: false });

    await session.stepOnce();
    await session.stepOnce();

    expect(bridge.step).toHaveBeenNthCalledWith(2, {
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

    expect(bridge.step).toHaveBeenNthCalledWith(1, {
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

    expect(bridge.step).toHaveBeenNthCalledWith(2, {
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

    expect(bridge.step).toHaveBeenNthCalledWith(1, {
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

    expect(bridge.step).toHaveBeenNthCalledWith(3, {
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

  it('falls back to legacy setInput/tick/snapshot/frame bridge calls when step is unavailable', async () => {
    const bridge = makeBridge();
    delete bridge.step;
    const session = new WasmRuntimeSession(bridge);

    session.setInputState({ left: true, right: false, jump: false, restart: false });
    const result = await session.stepOnce();

    expect(bridge.setInput).toHaveBeenCalledTimes(1);
    expect(bridge.tick).toHaveBeenCalledWith(1);
    expect(bridge.snapshot).toHaveBeenCalledTimes(1);
    expect(bridge.frame).toHaveBeenCalledTimes(1);
    expect(result.frame.tick).toBe(1);
  });
});
