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

const DEFAULT_ROOM_SPEED_HZ = 30;
const SHELL_TELEMETRY_INTERVAL_MS = 1000;

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
type RuntimeTelemetryMode = 'immediate' | 'throttled';

type RuntimeShellOptions = {
  allowStaticFallback?: boolean;
  initialPackagePath?: string;
};

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

function clearCanvas(canvas: HTMLCanvasElement, width = 800, height = 600): void {
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext('2d');
  if (!context) {
    return;
  }
  context.clearRect(0, 0, canvas.width, canvas.height);
  context.fillStyle = '#0c1118';
  context.fillRect(0, 0, canvas.width, canvas.height);
}

function validRoomSpeedHz(speed: number | null | undefined): number | null {
  return Number.isFinite(speed) && speed != null && speed > 0
    ? speed
    : null;
}

function snapshotRoomSpeedHz(snapshot: WasmRuntimeBridgeSnapshot): number | null {
  return validRoomSpeedHz(snapshot.roomSpeed);
}

function roomTickRateHz(
  pkg: RuntimePackage | null,
  roomId: number | null,
  runtimeRoomSpeed: number | null
): number {
  const runtimeSpeed = validRoomSpeedHz(runtimeRoomSpeed);
  if (runtimeSpeed != null) {
    return runtimeSpeed;
  }
  const speed = roomId == null
    ? undefined
    : pkg?.rooms.find((room) => room.id === roomId)?.speed;
  return validRoomSpeedHz(speed) ?? DEFAULT_ROOM_SPEED_HZ;
}

function autoTickIntervalMs(
  pkg: RuntimePackage | null,
  roomId: number | null,
  runtimeRoomSpeed: number | null
): number {
  return 1000 / roomTickRateHz(pkg, roomId, runtimeRoomSpeed);
}

function formatErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function minimalRuntimeSnapshot(snapshot: WasmRuntimeBridgeSnapshot): WasmRuntimeBridgeSnapshot {
  return {
    ...snapshot,
    tickPhases: undefined,
    diagnostics: [],
  };
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

export function useRuntimeShell(options: RuntimeShellOptions = {}) {
  const {
    allowStaticFallback = true,
    initialPackagePath = '/packages/sample',
  } = options;
  const [packagePath, setPackagePath] = useState(initialPackagePath);
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
  const currentRoomIdRef = useRef<number | null>(null);
  const currentRoomSpeedRef = useRef<number | null>(null);
  const backendRef = useRef<ShellBackend>({
    kind: 'viewer',
    roomId: null,
    diagnostics: ['Static room viewer idle. Load a package to inspect resources.'],
  });
  const renderCacheRef = useRef(new ResourceCache());
  const autoTickHandleRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const autoTickInFlightRef = useRef(false);
  const autoTickInFlightPromiseRef = useRef<Promise<void> | null>(null);
  const autoTickKeyboardSourceRef = useRef<KeyboardInputSource | null>(null);
  const autoTickDelayMsRef = useRef<number | null>(null);
  const exclusiveOpRef = useRef(false);
  const skippedAutoTickIntervalsRef = useRef(0);
  const mountedRef = useRef(true);
  const lastTelemetryCommitMsRef = useRef<number | null>(null);
  const pendingSnapshotRef = useRef<WasmRuntimeBridgeSnapshot | null>(null);
  const pendingPerformanceRef = useRef<RuntimePerformanceStats | null>(null);
  const pendingRoomIdRef = useRef<number | null>(null);

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
        const nextRoomId = nextSnapshot.roomId ?? null;
        currentRoomIdRef.current = nextRoomId;
        currentRoomSpeedRef.current = snapshotRoomSpeedHz(nextSnapshot);
        setSnapshot(nextSnapshot);
        setMode('wasm');
        setViewerDiagnostics([]);
        if (nextPerformance) {
          setPerformanceStats(nextPerformance);
        }
        setSelectedRoomId(nextRoomId);
        return;
      }

      setMode('viewer');
      setPerformanceStats(null);
      setViewerDiagnostics(backend.diagnostics);
      const room = backend.roomId != null
        ? pkg.rooms.find((candidate) => candidate.id === backend.roomId)
        : null;
      currentRoomIdRef.current = backend.roomId;
      currentRoomSpeedRef.current = validRoomSpeedHz(room?.speed);
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

      clearCanvas(canvas, pkg.manifest.display_width, pkg.manifest.display_height);
    },
    []
  );

  const commitRuntimeTelemetry = useCallback((
    nextSnapshot: WasmRuntimeBridgeSnapshot,
    nextPerformance: RuntimePerformanceStats | null,
    nextRoomId: number | null,
    commitTimeMs = nowMs(),
    clearPending = true
  ) => {
    if (clearPending) {
      pendingSnapshotRef.current = null;
      pendingPerformanceRef.current = null;
      pendingRoomIdRef.current = null;
    }
    lastTelemetryCommitMsRef.current = commitTimeMs;
    setSnapshot(nextSnapshot);
    setPerformanceStats(nextPerformance);
    currentRoomIdRef.current = nextRoomId;
    currentRoomSpeedRef.current = snapshotRoomSpeedHz(nextSnapshot);
    setSelectedRoomId(nextRoomId);
  }, []);

  const flushPendingRuntimeTelemetry = useCallback(() => {
    if (!pendingSnapshotRef.current) {
      return;
    }
    commitRuntimeTelemetry(
      pendingSnapshotRef.current,
      pendingPerformanceRef.current,
      pendingRoomIdRef.current,
    );
  }, [commitRuntimeTelemetry]);

  const publishRuntimeTelemetry = useCallback((
    nextSnapshot: WasmRuntimeBridgeSnapshot,
    nextPerformance: RuntimePerformanceStats | null,
    nextRoomId: number | null,
    mode: RuntimeTelemetryMode
  ) => {
    const previousRoomId = currentRoomIdRef.current;
    currentRoomIdRef.current = nextRoomId;
    currentRoomSpeedRef.current = snapshotRoomSpeedHz(nextSnapshot);

    if (mode === 'immediate') {
      commitRuntimeTelemetry(nextSnapshot, nextPerformance, nextRoomId);
      return;
    }

    pendingSnapshotRef.current = nextSnapshot;
    pendingPerformanceRef.current = nextPerformance;
    pendingRoomIdRef.current = nextRoomId;

    const commitTimeMs = nowMs();
    const lastCommitMs = lastTelemetryCommitMsRef.current;
    const shouldCommit = lastCommitMs == null
      || commitTimeMs - lastCommitMs >= SHELL_TELEMETRY_INTERVAL_MS
      || previousRoomId !== nextRoomId;

    if (shouldCommit) {
      commitRuntimeTelemetry(
        minimalRuntimeSnapshot(nextSnapshot),
        null,
        nextRoomId,
        commitTimeMs,
        false
      );
    }
  }, [commitRuntimeTelemetry]);

  const stopAutoTick = useCallback(() => {
    if (autoTickHandleRef.current != null) {
      globalThis.clearInterval(autoTickHandleRef.current);
    }
    autoTickHandleRef.current = null;
    autoTickInFlightRef.current = false;
    autoTickKeyboardSourceRef.current = null;
    autoTickDelayMsRef.current = null;
    if (mountedRef.current) {
      flushPendingRuntimeTelemetry();
      setAutoTickRunning(false);
    }
  }, [flushPendingRuntimeTelemetry]);

  const tickRuntimeOnce = useCallback(
    async (keyboard: KeyboardInputState, telemetryMode: RuntimeTelemetryMode = 'immediate') => {
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
      const collectPerformance = telemetryMode === 'immediate';
      const frameStart = collectPerformance ? nowMs() : 0;
      const { snapshot: nextSnapshot, frame, timings } = await backend.session.stepOnce();
      keyboard.clearEdgeKeys();
      const renderStart = collectPerformance ? nowMs() : 0;
      const canvas = canvasRef.current;
      if (canvas) {
        await renderWasmFrame(canvas, frame, currentPackage.resources, packagePathRef.current, renderCacheRef.current);
      }
      if (!mountedRef.current) {
        return;
      }
      const nextPerformance: RuntimePerformanceStats | null = collectPerformance
        ? {
            inputMs: timings.inputMs,
            tickMs: timings.tickMs,
            snapshotMs: timings.snapshotMs,
            frameMs: timings.frameMs,
            runtimeMs: timings.runtimeMs,
            renderMs: nowMs() - renderStart,
            totalMs: nowMs() - frameStart,
            commandCount: frame.commands.length,
            skippedIntervals: skippedAutoTickIntervalsRef.current,
          }
        : null;
      const nextRoomId = nextSnapshot.roomId ?? null;
      publishRuntimeTelemetry(nextSnapshot, nextPerformance, nextRoomId, telemetryMode);
    },
    [publishRuntimeTelemetry]
  );

  const scheduleAutoTickInterval = useCallback(
    (keyboardSource: KeyboardInputSource) => {
      if (!loadedPackageRef.current || backendRef.current.kind !== 'wasm') {
        return;
      }

      if (autoTickHandleRef.current != null) {
        globalThis.clearInterval(autoTickHandleRef.current);
      }
      const delayMs = autoTickIntervalMs(
        loadedPackageRef.current,
        currentRoomIdRef.current,
        currentRoomSpeedRef.current
      );
      autoTickDelayMsRef.current = delayMs;
      autoTickHandleRef.current = globalThis.setInterval(() => {
        if (exclusiveOpRef.current || autoTickInFlightRef.current) {
          skippedAutoTickIntervalsRef.current += 1;
          return;
        }

        autoTickInFlightRef.current = true;
        autoTickInFlightPromiseRef.current = tickRuntimeOnce(currentKeyboardInput(keyboardSource), 'throttled')
          .catch((tickError) => {
            stopAutoTick();
            if (mountedRef.current) {
              setError(`Runtime tick failed: ${formatErrorMessage(tickError)}`);
            }
          })
          .finally(() => {
            autoTickInFlightRef.current = false;
            autoTickInFlightPromiseRef.current = null;
        });
        void autoTickInFlightPromiseRef.current;
      }, delayMs);
    },
    [stopAutoTick, tickRuntimeOnce]
  );

  const startAutoTick = useCallback(
    (keyboardSource: KeyboardInputSource) => {
      if (!loadedPackageRef.current || backendRef.current.kind !== 'wasm' || autoTickHandleRef.current != null) {
        return;
      }

      autoTickKeyboardSourceRef.current = keyboardSource;
      scheduleAutoTickInterval(keyboardSource);
      setAutoTickRunning(true);
    },
    [scheduleAutoTickInterval]
  );

  useEffect(() => {
    if (!autoTickRunning || !autoTickKeyboardSourceRef.current || backendRef.current.kind !== 'wasm') {
      return;
    }
    const nextDelayMs = autoTickIntervalMs(
      loadedPackageRef.current,
      currentRoomIdRef.current,
      currentRoomSpeedRef.current
    );
    if (autoTickDelayMsRef.current == null || Math.abs(autoTickDelayMsRef.current - nextDelayMs) > 0.001) {
      scheduleAutoTickInterval(autoTickKeyboardSourceRef.current);
    }
  }, [autoTickRunning, scheduleAutoTickInterval, selectedRoomId, snapshot?.roomSpeed]);

  const loadCurrentPackage = useCallback(async (
    keyboardSource?: KeyboardInputSource,
    requestedPath = packagePath
  ) => {
    setError(null);
    setRuntimeReady(false);
    stopAutoTick();
    pendingSnapshotRef.current = null;
    pendingPerformanceRef.current = null;
    pendingRoomIdRef.current = null;
    currentRoomSpeedRef.current = null;
    lastTelemetryCommitMsRef.current = null;
    skippedAutoTickIntervalsRef.current = 0;
    packagePathRef.current = requestedPath;
    setPackagePath(requestedPath);

    try {
      const pkg = await loadPackage(requestedPath);
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
        const bootSnapshot = await wasmBridge.boot(pkg, { basePath: requestedPath });
        nextBackend = {
          kind: 'wasm',
          bridge: wasmBridge,
          session: new WasmRuntimeSession(wasmBridge),
        };
        roomId = bootSnapshot.roomId ?? roomId;
        setRuntimeReady(true);
      } catch (bootError) {
        if (!allowStaticFallback) {
          throw new Error(`WASM runtime unavailable: ${formatErrorMessage(bootError)}`);
        }
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
      await draw(pkg, nextBackend, requestedPath);
      if (nextBackend.kind === 'wasm' && keyboardSource) {
        startAutoTick(keyboardSource);
      }
      return pkg;
    } catch (loadError) {
      loadedPackageRef.current = null;
      currentRoomSpeedRef.current = null;
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
  }, [allowStaticFallback, draw, packagePath, startAutoTick, stopAutoTick]);

  const runExclusiveWithAutoTick = useCallback(async (operation: () => Promise<void>): Promise<void> => {
    // Hold a guard the auto-tick interval also honours, then wait for any
    // in-flight tick to settle before touching the shared wasm instance.
    exclusiveOpRef.current = true;
    try {
      if (autoTickInFlightPromiseRef.current) {
        await autoTickInFlightPromiseRef.current.catch(() => undefined);
      }
      await operation();
    } finally {
      exclusiveOpRef.current = false;
    }
  }, []);

  const selectRoom = useCallback(
    async (roomId: number) => {
      setSelectedRoomId(roomId);
      if (!loadedPackage) {
        return;
      }

      if (backendRef.current.kind === 'wasm') {
        await runExclusiveWithAutoTick(async () => {
          if (backendRef.current.kind !== 'wasm') {
            return;
          }
          await backendRef.current.bridge.selectRoom(roomId);
          await draw(loadedPackage, backendRef.current, packagePath);
        });
        return;
      }

      const nextBackend: ShellBackend = {
        ...backendRef.current,
        roomId,
      };
      backendRef.current = nextBackend;
      await draw(loadedPackage, nextBackend, packagePath);
    },
    [draw, loadedPackage, packagePath, runExclusiveWithAutoTick]
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
    await runExclusiveWithAutoTick(async () => {
      if (backendRef.current.kind !== 'wasm') {
        return;
      }
      await backendRef.current.bridge.reset();
      await draw(loadedPackage, backendRef.current, packagePath);
    });
  }, [draw, loadedPackage, packagePath, runExclusiveWithAutoTick]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      stopAutoTick();
    };
  }, [stopAutoTick]);

  const displayWidth = loadedPackage?.manifest.display_width ?? 800;
  const displayHeight = loadedPackage?.manifest.display_height ?? 600;

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
    displayWidth,
    displayHeight,
  };
}
