import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { loadPackage } from '../../loadPackage';
import { makeBackgroundPathMap, makeSpriteFrameMap, ResourceCache } from '../../render/resourceCache';
import { renderStaticRoom } from '../../render/staticRoomRenderer';
import { renderWasmFrame } from '../../render/wasmFrameRenderer';
import {
  describeWasmBridgeAvailability,
  loadDefaultWasmRuntimeBridge,
  type WasmRuntimeBridge,
  type WasmRuntimeBridgeSnapshot,
} from '../../runtime/wasmBridge';
import { WasmRuntimeSession } from '../../runtime/wasmSession';
import type { RuntimePackage } from '../../types';
import type { RuntimePerformanceStats } from '../traceView';
import type { KeyboardInputState } from './useKeyboardInput';

const AUTO_TICK_MS = 1000 / 60;

type ShellBackend =
  | {
      kind: 'viewer';
      roomId: number | null;
      diagnostics: string[];
    }
  | {
      kind: 'wasm';
      bridge: WasmRuntimeBridge;
      session: WasmRuntimeSession;
    };

type KeyboardInputSource = KeyboardInputState | { current: KeyboardInputState };

function currentKeyboardInput(source: KeyboardInputSource): KeyboardInputState {
  return 'current' in source ? source.current : source;
}

function nowMs(): number {
  return globalThis.performance?.now() ?? Date.now();
}

function defaultInputTrace(): WasmRuntimeBridgeSnapshot['inputTrace'] {
  return {
    jumpButtonKey: 0x20,
    jumpPressed: false,
    jumpJustPressed: false,
    jumpJustReleased: false,
    activeKeys: [],
  };
}

function clearCanvas(canvas: HTMLCanvasElement): void {
  canvas.width = 960;
  canvas.height = 540;
  const context = canvas.getContext('2d');
  if (!context) {
    return;
  }
  context.clearRect(0, 0, canvas.width, canvas.height);
  context.fillStyle = '#0c1118';
  context.fillRect(0, 0, canvas.width, canvas.height);
}

function formatErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

async function renderRuntimeRoom(
  basePath: string,
  canvas: HTMLCanvasElement,
  roomId: number,
  pkg: RuntimePackage,
  cache: ResourceCache
): Promise<void> {
  const room = pkg.rooms.find((candidate) => candidate.id === roomId);
  if (!room) {
    return;
  }
  const backgroundPaths = makeBackgroundPathMap(basePath, pkg.resources);
  const spritePaths = makeSpriteFrameMap(basePath, pkg.resources);
  await renderStaticRoom(canvas, room, pkg.objects, backgroundPaths, spritePaths, cache);
}

export function useRuntimeShell() {
  const [packagePath, setPackagePath] = useState('/packages/sample');
  const [loadedPackage, setLoadedPackage] = useState<RuntimePackage | null>(null);
  const [backendStatus, setBackendStatus] = useState(
    'Execution path: static room viewer until a WASM bridge is configured.'
  );
  const [selectedRoomId, setSelectedRoomId] = useState<number | null>(null);
  const [autoTickRunning, setAutoTickRunning] = useState(false);
  const [snapshot, setSnapshot] = useState<WasmRuntimeBridgeSnapshot | null>(null);
  const [performanceStats, setPerformanceStats] = useState<RuntimePerformanceStats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [runtimeReady, setRuntimeReady] = useState(false);
  const [mode, setMode] = useState<'viewer' | 'wasm'>('viewer');
  const [viewerDiagnostics, setViewerDiagnostics] = useState<string[]>([
    'Static room viewer idle. Load a package to inspect resources.',
  ]);

  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const loadedPackageRef = useRef<RuntimePackage | null>(null);
  const packagePathRef = useRef(packagePath);
  const backendRef = useRef<ShellBackend>({
    kind: 'viewer',
    roomId: null,
    diagnostics: ['Static room viewer idle. Load a package to inspect resources.'],
  });
  const renderCacheRef = useRef(new ResourceCache());
  const autoTickHandleRef = useRef<number | null>(null);
  const autoTickInFlightRef = useRef(false);
  const skippedAutoTickIntervalsRef = useRef(0);

  useEffect(() => {
    packagePathRef.current = packagePath;
  }, [packagePath]);

  const roomOptions = useMemo(
    () => loadedPackage?.rooms.map((room) => ({ id: room.id, name: room.name })) ?? [],
    [loadedPackage]
  );

  const draw = useCallback(
    async (
      pkg: RuntimePackage,
      backend: ShellBackend,
      currentPath: string,
      nextPerformance: RuntimePerformanceStats | null = null
    ): Promise<void> => {
      const canvas = canvasRef.current;
      if (!canvas) {
        return;
      }

      if (backend.kind === 'wasm') {
        const nextSnapshot = await backend.bridge.snapshot();
        const frame = await backend.bridge.frame();
        await renderWasmFrame(canvas, frame, pkg.resources, currentPath, renderCacheRef.current);
        setSnapshot(nextSnapshot);
        setMode('wasm');
        setViewerDiagnostics([]);
        if (nextPerformance) {
          setPerformanceStats(nextPerformance);
        }
        setSelectedRoomId(nextSnapshot.roomId ?? null);
        return;
      }

      setMode('viewer');
      setPerformanceStats(null);
      setViewerDiagnostics(backend.diagnostics);
      const room = backend.roomId != null
        ? pkg.rooms.find((candidate) => candidate.id === backend.roomId)
        : null;
      setSnapshot({
        tick: 0,
        roomId: backend.roomId,
        roomName: room?.name ?? (backend.roomId != null ? 'Static room viewer' : null),
        diagnostics: backend.diagnostics,
        inputTrace: defaultInputTrace(),
        player: null,
      });

      if (backend.roomId != null) {
        await renderRuntimeRoom(currentPath, canvas, backend.roomId, pkg, renderCacheRef.current);
        return;
      }

      clearCanvas(canvas);
    },
    []
  );

  const stopAutoTick = useCallback(() => {
    if (autoTickHandleRef.current != null) {
      globalThis.clearInterval(autoTickHandleRef.current);
    }
    autoTickHandleRef.current = null;
    autoTickInFlightRef.current = false;
    setAutoTickRunning(false);
  }, []);

  const tickRuntimeOnce = useCallback(
    async (keyboard: KeyboardInputState) => {
      const currentPackage = loadedPackageRef.current;
      if (!currentPackage || backendRef.current.kind !== 'wasm') {
        return;
      }

      const backend = backendRef.current;
      backend.session.setInputState({
        left: keyboard.left,
        right: keyboard.right,
        jump: keyboard.jump,
        restart: keyboard.restart,
        keysHeld: keyboard.keysHeld,
        keysPressed: keyboard.keysPressed,
        keysReleased: keyboard.keysReleased,
      });
      const frameStart = nowMs();
      const { snapshot: nextSnapshot, frame, timings } = await backend.session.stepOnce();
      keyboard.clearEdgeKeys();
      const renderStart = nowMs();
      const canvas = canvasRef.current;
      if (canvas) {
        await renderWasmFrame(canvas, frame, currentPackage.resources, packagePathRef.current, renderCacheRef.current);
      }
      const renderMs = nowMs() - renderStart;
      const nextPerformance: RuntimePerformanceStats = {
        inputMs: timings.inputMs,
        tickMs: timings.tickMs,
        snapshotMs: timings.snapshotMs,
        frameMs: timings.frameMs,
        runtimeMs: timings.runtimeMs,
        renderMs,
        totalMs: nowMs() - frameStart,
        commandCount: frame.commands.length,
        skippedIntervals: skippedAutoTickIntervalsRef.current,
      };
      setSnapshot(nextSnapshot);
      setPerformanceStats(nextPerformance);
      setSelectedRoomId(nextSnapshot.roomId ?? null);
    },
    []
  );

  const startAutoTick = useCallback(
    (keyboardSource: KeyboardInputSource) => {
      if (!loadedPackageRef.current || backendRef.current.kind !== 'wasm' || autoTickHandleRef.current != null) {
        return;
      }

      autoTickHandleRef.current = globalThis.setInterval(() => {
        if (autoTickInFlightRef.current) {
          skippedAutoTickIntervalsRef.current += 1;
          return;
        }

        autoTickInFlightRef.current = true;
        void tickRuntimeOnce(currentKeyboardInput(keyboardSource))
          .catch((tickError) => {
            stopAutoTick();
            setError(`Runtime tick failed: ${formatErrorMessage(tickError)}`);
          })
          .finally(() => {
            autoTickInFlightRef.current = false;
          });
      }, AUTO_TICK_MS);
      setAutoTickRunning(true);
    },
    [stopAutoTick, tickRuntimeOnce]
  );

  const loadCurrentPackage = useCallback(async (keyboardSource?: KeyboardInputSource) => {
    setError(null);
    stopAutoTick();
    skippedAutoTickIntervalsRef.current = 0;
    packagePathRef.current = packagePath;

    try {
      const pkg = await loadPackage(packagePath);
      loadedPackageRef.current = pkg;
      setLoadedPackage(pkg);

      const defaultRoomId = pkg.manifest.default_room_id ?? pkg.rooms[0]?.id ?? null;
      let nextBackend: ShellBackend = {
        kind: 'viewer',
        roomId: defaultRoomId,
        diagnostics: ['Static room viewer active. Gameplay execution requires the WASM bridge.'],
      };
      let roomId = defaultRoomId;
      let wasmBridgeError: unknown = null;

      try {
        const wasmBridge = await loadDefaultWasmRuntimeBridge();
        const bootSnapshot = await wasmBridge.boot(pkg, { basePath: packagePath });
        nextBackend = {
          kind: 'wasm',
          bridge: wasmBridge,
          session: new WasmRuntimeSession(wasmBridge),
        };
        roomId = bootSnapshot.roomId ?? roomId;
        setRuntimeReady(true);
      } catch (bootError) {
        wasmBridgeError = bootError;
        setRuntimeReady(false);
        nextBackend = {
          kind: 'viewer',
          roomId: defaultRoomId,
          diagnostics: [
            `WASM runtime unavailable: ${formatErrorMessage(bootError)}`,
            'Static room viewer active. Gameplay execution requires the WASM bridge.',
          ],
        };
      }

      backendRef.current = nextBackend;
      setBackendStatus(
        `Execution path: ${describeWasmBridgeAvailability(
          nextBackend.kind === 'wasm' ? nextBackend.bridge : null,
          wasmBridgeError
        )}`
      );
      setSelectedRoomId(roomId);
      await draw(pkg, nextBackend, packagePath);
      if (nextBackend.kind === 'wasm' && keyboardSource) {
        startAutoTick(keyboardSource);
      }
      return pkg;
    } catch (loadError) {
      loadedPackageRef.current = null;
      setLoadedPackage(null);
      setRuntimeReady(false);
      backendRef.current = {
        kind: 'viewer',
        roomId: null,
        diagnostics: ['Static room viewer idle. Load a package to inspect resources.'],
      };
      setSnapshot({
        tick: 0,
        roomId: null,
        roomName: null,
        diagnostics: [`Load failed: ${formatErrorMessage(loadError)}`],
        inputTrace: defaultInputTrace(),
        player: null,
      });
      setError(`Load failed: ${formatErrorMessage(loadError)}`);
      const canvas = canvasRef.current;
      if (canvas) {
        clearCanvas(canvas);
      }
      throw loadError;
    }
  }, [draw, packagePath, startAutoTick, stopAutoTick]);

  const selectRoom = useCallback(
    async (roomId: number) => {
      setSelectedRoomId(roomId);
      if (!loadedPackage) {
        return;
      }

      if (backendRef.current.kind === 'wasm') {
        await backendRef.current.bridge.selectRoom(roomId);
        await draw(loadedPackage, backendRef.current, packagePath);
        return;
      }

      const nextBackend: ShellBackend = {
        ...backendRef.current,
        roomId,
      };
      backendRef.current = nextBackend;
      await draw(loadedPackage, nextBackend, packagePath);
    },
    [draw, loadedPackage, packagePath]
  );

  const togglePause = useCallback(
    (keyboard: KeyboardInputSource) => {
      if (!loadedPackageRef.current || backendRef.current.kind !== 'wasm') {
        return;
      }
      if (autoTickRunning) {
        stopAutoTick();
        return;
      }
      startAutoTick(keyboard);
    },
    [autoTickRunning, startAutoTick, stopAutoTick]
  );

  const resetRuntime = useCallback(async () => {
    if (!loadedPackage || backendRef.current.kind !== 'wasm') {
      return;
    }
    await backendRef.current.bridge.reset();
    await draw(loadedPackage, backendRef.current, packagePath);
  }, [draw, loadedPackage, packagePath]);

  useEffect(() => {
    return () => stopAutoTick();
  }, [stopAutoTick]);

  return {
    packagePath,
    setPackagePath,
    loadedPackage,
    backendStatus,
    selectedRoomId,
    setSelectedRoomId: selectRoom,
    autoTickRunning,
    snapshot,
    performance: performanceStats,
    error,
    runtimeReady,
    mode,
    viewerDiagnostics,
    roomOptions,
    canvasRef,
    loadCurrentPackage,
    togglePause,
    resetRuntime,
    tickRuntimeOnce,
    startAutoTick,
    stopAutoTick,
  };
}
