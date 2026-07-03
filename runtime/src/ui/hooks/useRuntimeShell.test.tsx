import { act, cleanup, fireEvent, render, renderHook, screen, waitFor } from '@testing-library/react';
import { StrictMode } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { App } from '../../app/App';
import type { WasmRuntimeBridge } from '../../runtime/wasmBridge';
import { makeRuntimePackage, makeWasmFrame, makeWasmSnapshot } from '../../test/packageFixtures';
import type { RuntimePackage } from '../../types';
import type { KeyboardInputState } from './useKeyboardInput';
import { useRuntimeShell } from './useRuntimeShell';

const mocks = vi.hoisted(() => ({
  loadPackage: vi.fn(),
  loadDefaultWasmRuntimeBridge: vi.fn(),
  renderWasmFrame: vi.fn(),
}));

vi.mock('../../loadPackage', () => ({
  loadPackage: mocks.loadPackage,
}));

vi.mock('../../render/wasmFrameRenderer', () => ({
  renderWasmFrame: mocks.renderWasmFrame,
}));

vi.mock('../../runtime/wasmBridge', () => ({
  describeWasmBridgeAvailability: (bridge: WasmRuntimeBridge | null, error: unknown) => {
    if (bridge) {
      return 'WASM bridge available';
    }
    return error instanceof Error ? error.message : 'WASM bridge unavailable';
  },
  loadDefaultWasmRuntimeBridge: mocks.loadDefaultWasmRuntimeBridge,
}));

function makeRuntimeShellFrame(tick: number) {
  return makeWasmFrame({
    tick,
    roomId: 1,
    width: 960,
    height: 540,
    commands: [
      { kind: 'clear', colour: [12, 17, 24, 255] },
      { kind: 'present' },
    ],
  });
}

function makeBridge(): WasmRuntimeBridge {
  let tick = 0;
  return {
    backend: 'opengmk-wasm',
    boot: vi.fn(async () => makeWasmSnapshot({ tick })),
    snapshot: vi.fn(async () => makeWasmSnapshot({ tick })),
    frame: vi.fn(async () => makeRuntimeShellFrame(tick)),
    setInput: vi.fn(async () => makeWasmSnapshot({ tick })),
    tick: vi.fn(async (frames = 1) => {
      tick += frames;
      return makeWasmSnapshot({ tick });
    }),
    reset: vi.fn(async () => {
      tick = 0;
      return makeWasmSnapshot({ tick });
    }),
    selectRoom: vi.fn(async () => makeWasmSnapshot({ tick })),
    diagnostics: vi.fn(async () => []),
  };
}

function makeKeyboard(): KeyboardInputState {
  return {
    left: false,
    right: false,
    jump: false,
    restart: false,
    keysHeld: [],
    keysPressed: [],
    keysReleased: [],
    clearEdgeKeys: vi.fn(),
  };
}

function arrangeWasmPackage(pkg: RuntimePackage = makeRuntimePackage()): WasmRuntimeBridge {
  const bridge = makeBridge();
  mocks.loadPackage.mockResolvedValue(pkg);
  mocks.loadDefaultWasmRuntimeBridge.mockResolvedValue(bridge);
  mocks.renderWasmFrame.mockResolvedValue(undefined);
  return bridge;
}

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

describe('useRuntimeShell', () => {
  it('ticks a loaded wasm runtime without shadowing the browser performance clock', async () => {
    const bridge = arrangeWasmPackage();
    const { result } = renderHook(() => useRuntimeShell());

    await act(async () => {
      await result.current.loadCurrentPackage();
    });
    await waitFor(() => expect(result.current.loadedPackage).not.toBeNull());

    await act(async () => {
      await result.current.tickRuntimeOnce(makeKeyboard());
    });

    expect(bridge.tick).toHaveBeenCalledWith(1);
    expect(result.current.snapshot?.tick).toBe(1);
    expect(result.current.performance?.commandCount).toBe(2);
  });

  it('starts automatic ticking after a wasm package loads', async () => {
    arrangeWasmPackage();

    render(<App />);
    fireEvent.click(screen.getByRole('button', { name: 'Load Package' }));

    await waitFor(() => expect(screen.getByRole('button', { name: 'Pause' })).toBeEnabled());
    await waitFor(() => expect(screen.getByText(/^Tick: [1-9]\d*/)).toBeInTheDocument());

    fireEvent.click(screen.getByRole('button', { name: 'Pause' }));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Resume' })).toBeEnabled());
  });

  it('keeps automatic ticking active after StrictMode replays effects', async () => {
    arrangeWasmPackage();

    render(
      <StrictMode>
        <App />
      </StrictMode>
    );
    fireEvent.click(screen.getByRole('button', { name: 'Load Package' }));

    await waitFor(() => expect(screen.getByRole('button', { name: 'Pause' })).toBeEnabled());
    await waitFor(() => expect(screen.getByText(/^Tick: [1-9]\d*/)).toBeInTheDocument());
    await waitFor(() => expect(screen.getByText(/^Frame: \d/)).toBeInTheDocument());

    fireEvent.click(screen.getByRole('button', { name: 'Pause' }));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Resume' })).toBeEnabled());
  });

  it('schedules automatic ticks from the selected room speed', async () => {
    arrangeWasmPackage(makeRuntimePackage({ roomSpeed: 30 }));
    const setIntervalSpy = vi.fn(() => 1);
    vi.stubGlobal('setInterval', setIntervalSpy);
    vi.stubGlobal('clearInterval', vi.fn());
    const { result } = renderHook(() => useRuntimeShell());

    await act(async () => {
      await result.current.loadCurrentPackage(makeKeyboard());
    });

    expect(setIntervalSpy).toHaveBeenCalledWith(expect.any(Function), 1000 / 30);
  });

  it('selects a wasm room directly without overriding package globals', async () => {
    const bridge = arrangeWasmPackage();
    const { result } = renderHook(() => useRuntimeShell());

    await act(async () => {
      await result.current.loadCurrentPackage();
    });

    await act(async () => {
      await result.current.setSelectedRoomId(1);
    });

    expect(bridge.selectRoom).toHaveBeenCalledWith(1);
  });
});
