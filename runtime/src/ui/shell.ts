import { loadPackage } from '../loadPackage';
import { makeBackgroundPathMap, makeSpriteFrameMap, ResourceCache } from '../render/resourceCache';
import { renderStaticRoom } from '../render/staticRoomRenderer';
import { renderWasmFrame } from '../render/wasmFrameRenderer';
import { WasmRuntimeSession } from '../runtime/wasmSession';
import {
  describeWasmBridgeAvailability,
  type WasmRuntimeBridge,
  type WasmRuntimeBridgeSnapshot,
  type WasmRuntimeInputState,
} from '../runtime/wasmBridge';
import type { WasmRuntimeStepResult } from '../runtime/wasmSession';
import type { RuntimePackage } from '../types';
import {
  createPreBlock,
  formatPerformanceDetails,
  formatTickPhaseDetails,
  renderDebugPanels,
  type DebugPanel,
} from './debugPanels';
import { renderManualTestHud } from './hud';
import { renderManifestSummary, renderObjectsSlice, renderRoomsSlice, renderScriptsSlice } from './inspectors';
import {
  buildManualTestSnapshot,
  type RuntimePerformanceStats,
} from './traceView';

type IntervalHandle = ReturnType<typeof globalThis.setInterval>;

type ShellDependencies = {
  loadPackage: typeof loadPackage;
  renderStaticRoom: typeof renderStaticRoom;
  renderWasmFrame: typeof renderWasmFrame;
  loadWasmBridge?: () => Promise<WasmRuntimeBridge>;
  setInterval?: (handler: () => void, timeout: number) => IntervalHandle;
  clearInterval?: (handle: IntervalHandle) => void;
  now?: () => number;
};

const defaultDependencies: ShellDependencies = {
  loadPackage,
  renderStaticRoom,
  renderWasmFrame,
  setInterval: (handler, timeout) => globalThis.setInterval(handler, timeout),
  clearInterval: (handle) => globalThis.clearInterval(handle),
  now: () => globalThis.performance?.now() ?? Date.now()
};

const AUTO_TICK_MS = 1000 / 60;
const MAX_VISIBLE_DIAGNOSTICS = 8;

type ShellElements = {
  input: HTMLInputElement;
  button: HTMLButtonElement;
  select: HTMLSelectElement;
  pauseButton: HTMLButtonElement;
  resetButton: HTMLButtonElement;
  hud: HTMLElement;
  debugPanels: HTMLElement;
  backendStatus: HTMLElement;
  metaRoot: HTMLElement;
  toolbar: HTMLElement;
  inspectors: HTMLElement;
  canvas: HTMLCanvasElement;
};

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

function resetRoomOptions(doc: Document, select: HTMLSelectElement, message: string): void {
  select.innerHTML = '';
  const option = doc.createElement('option');
  option.value = '';
  option.textContent = message;
  select.append(option);
  select.disabled = true;
}

function clearCanvas(canvas: HTMLCanvasElement): void {
  canvas.width = 960;
  canvas.height = 540;
  const getContext = (canvas as Partial<HTMLCanvasElement>).getContext;
  if (typeof getContext !== 'function') {
    return;
  }
  const context = getContext.call(canvas, '2d');
  if (
    !context
    || !('clearRect' in context)
    || !('fillRect' in context)
    || !('fillStyle' in context)
  ) {
    return;
  }
  context.clearRect(0, 0, canvas.width, canvas.height);
  context.fillStyle = '#0c1118';
  context.fillRect(0, 0, canvas.width, canvas.height);
}

function formatErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function parseRoomId(value: string): number | null {
  if (!value) {
    return null;
  }

  const roomId = Number(value);
  return Number.isFinite(roomId) ? roomId : null;
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

function createShell(doc: Document): { shell: HTMLElement; elements: ShellElements } {
  const shell = doc.createElement('div');
  shell.className = 'shell shell--manual-test';

  const toolbar = doc.createElement('header');
  toolbar.id = 'runtime-controls';
  toolbar.className = 'runtime-controls';

  const title = doc.createElement('div');
  title.className = 'runtime-title';
  const heading = doc.createElement('h1');
  heading.textContent = 'IWanna Runtime Shell';
  const intro = doc.createElement('p');
  intro.textContent = 'Manual testing cockpit for the browser-hosted runtime path.';
  title.append(heading, intro);

  const packageField = doc.createElement('label');
  packageField.className = 'field field--package';
  packageField.append('Package');
  const input = doc.createElement('input');
  input.name = 'packagePath';
  input.value = '/packages/sample';
  packageField.append(input);

  const button = doc.createElement('button');
  button.type = 'button';
  button.textContent = 'Load Package';

  const roomLabel = doc.createElement('label');
  roomLabel.className = 'field field--room';
  roomLabel.append('Room');
  const select = doc.createElement('select');
  select.name = 'roomSelect';
  resetRoomOptions(doc, select, 'Load a package first');
  roomLabel.append(select);

  const pauseButton = doc.createElement('button');
  pauseButton.type = 'button';
  pauseButton.textContent = 'Pause';

  const resetButton = doc.createElement('button');
  resetButton.type = 'button';
  resetButton.textContent = 'Reset';

  const backendStatus = doc.createElement('p');
  backendStatus.id = 'runtime-execution-path';
  backendStatus.className = 'hint runtime-execution-path';
  backendStatus.textContent = 'Execution path: static room viewer until a WASM bridge is configured.';

  toolbar.append(title, packageField, button, roomLabel, pauseButton, resetButton, backendStatus);

  const stage = doc.createElement('main');
  stage.className = 'stage';

  const canvas = doc.createElement('canvas');
  canvas.id = 'room-canvas';
  canvas.width = 960;
  canvas.height = 540;

  const hud = doc.createElement('section');
  hud.id = 'runtime-hud';
  hud.className = 'runtime-hud';

  const debugPanels = doc.createElement('section');
  debugPanels.id = 'debug-panels';
  debugPanels.className = 'debug-panels';

  const metaRoot = doc.createElement('section');
  metaRoot.className = 'meta';

  const inspectors = doc.createElement('section');
  inspectors.id = 'inspectors';

  stage.append(canvas, hud, debugPanels);
  shell.append(toolbar, stage);

  return {
    shell,
    elements: {
      input,
      button,
      select,
      pauseButton,
      resetButton,
      hud,
      debugPanels,
      backendStatus,
      metaRoot,
      toolbar,
      inspectors,
      canvas,
    }
  };
}

function setRoomOptions(doc: Document, select: HTMLSelectElement, pkg: RuntimePackage): void {
  select.innerHTML = '';
  pkg.rooms.forEach((room) => {
    const option = doc.createElement('option');
    option.value = String(room.id);
    option.textContent = `${room.id}: ${room.name}`;
    select.append(option);
  });
  select.disabled = pkg.rooms.length === 0;
}

function renderInspectors(doc: Document, metaRoot: HTMLElement, inspectors: HTMLElement, pkg: RuntimePackage): void {
  metaRoot.replaceChildren(renderManifestSummary(doc, pkg.manifest, pkg.analysis));
  inspectors.replaceChildren(renderRoomsSlice(doc, pkg.rooms), renderObjectsSlice(doc, pkg.objects), renderScriptsSlice(doc, pkg.scripts));
}

function renderRuntimeRoom(
  basePath: string,
  canvas: HTMLCanvasElement,
  roomId: number,
  pkg: RuntimePackage,
  renderStatic: typeof renderStaticRoom
): Promise<void> {
  const room = pkg.rooms.find((candidate) => candidate.id === roomId);
  if (!room) {
    return Promise.resolve();
  }
  const backgroundPaths = makeBackgroundPathMap(basePath, pkg.resources);
  const spritePaths = makeSpriteFrameMap(basePath, pkg.resources);
  return renderStatic(canvas, room, pkg.objects, backgroundPaths, spritePaths);
}

function keyToAction(key: string): 'left' | 'right' | 'restart' | null {
  switch (key) {
    case 'ArrowLeft':
    case 'a':
    case 'A':
      return 'left';
    case 'ArrowRight':
    case 'd':
    case 'D':
      return 'right';
    case 'r':
    case 'R':
      return 'restart';
    default:
      return null;
  }
}

function keyToVirtualKey(key: string): number | null {
  switch (key) {
    case 'ArrowLeft':
      return 0x25;
    case 'ArrowUp':
      return 0x26;
    case 'ArrowRight':
      return 0x27;
    case 'ArrowDown':
      return 0x28;
    case 'Shift':
      return 0x10;
    case ' ':
    case 'Spacebar':
      return 0x20;
    case 'Enter':
      return 0x0D;
    case 'Escape':
      return 0x1B;
    default:
      if (key.length === 1) {
        return key.toUpperCase().charCodeAt(0);
      }
      return null;
  }
}

export function createRuntimeShell(root: HTMLElement, dependencies: Partial<ShellDependencies> = {}): void {
  const resolved = { ...defaultDependencies, ...dependencies };
  const doc = root.ownerDocument;
  if (!doc) {
    throw new Error('Runtime shell requires an owner document');
  }

  root.innerHTML = '';
  const { shell, elements } = createShell(doc);
  root.append(shell);

  const { input, button, select, pauseButton, resetButton, backendStatus, metaRoot, inspectors, canvas } = elements;
  let loadedPackage: RuntimePackage | null = null;
  let activeBackend: ShellBackend = {
    kind: 'viewer',
    roomId: null,
    diagnostics: ['Static room viewer idle. Load a package to inspect resources.']
  };
  const keyboardState: Pick<WasmRuntimeInputState, 'left' | 'right' | 'jump' | 'restart'> = {
    left: false,
    right: false,
    jump: false,
    restart: false
  };
  const heldVirtualKeys = new Set<number>();
  const pendingPressedVirtualKeys = new Set<number>();
  const pendingReleasedVirtualKeys = new Set<number>();
  let autoTickHandle: IntervalHandle | null = null;
  let autoTickRunning = false;
  let autoTickInFlight = false;
  let skippedAutoTickIntervals = 0;
  let lastPerformance: RuntimePerformanceStats | null = null;
  const renderCache = new ResourceCache();

  const buildDebugPanels = (
    snapshot: WasmRuntimeBridgeSnapshot,
    performance: RuntimePerformanceStats | null
  ): DebugPanel[] => {
    const diagnostics = snapshot.diagnostics.slice(-MAX_VISIBLE_DIAGNOSTICS);
    const panels: DebugPanel[] = [
      {
        id: 'diagnostics-panel',
        title: 'Diagnostics',
        summary: diagnostics.length === 0 ? 'none' : `${diagnostics.length} recent`,
        content: createPreBlock(doc, 'runtime-diagnostics-detail', diagnostics),
      },
      {
        id: 'performance-panel',
        title: 'Performance',
        summary: performance ? `${performance.totalMs.toFixed(1)}ms` : 'unavailable',
        content: createPreBlock(doc, 'runtime-performance-detail', formatPerformanceDetails(performance)),
      },
      {
        id: 'tick-phases-panel',
        title: 'Tick phases',
        summary: snapshot.tickPhases ? `${(snapshot.tickPhases.totalNanos / 1_000_000).toFixed(3)}ms` : 'unavailable',
        content: createPreBlock(doc, 'runtime-tick-phases', formatTickPhaseDetails(snapshot.tickPhases ?? null)),
      },
    ];

    if (loadedPackage) {
      panels.push(
        {
          id: 'package-panel',
          title: 'Package',
          summary: loadedPackage.manifest.source_name,
          content: metaRoot,
        },
        {
          id: 'inspectors-panel',
          title: 'Inspectors',
          summary: `${loadedPackage.rooms.length} rooms, ${loadedPackage.objects.length} objects`,
          content: inspectors,
        }
      );
    }

    return panels;
  };

  const renderManualRuntimeView = (
    snapshot: WasmRuntimeBridgeSnapshot,
    roomLabel: string,
    mode: 'wasm' | 'viewer',
    statusText: string,
    performance: RuntimePerformanceStats | null = null
  ): void => {
    const visibleSnapshot = {
      ...snapshot,
      diagnostics: snapshot.diagnostics.slice(-MAX_VISIBLE_DIAGNOSTICS),
    };
    const manual = buildManualTestSnapshot({
      mode,
      status: statusText,
      roomLabel,
      snapshot: visibleSnapshot,
      performance,
    });
    renderManualTestHud(doc, elements.hud, manual);
    renderDebugPanels(doc, elements.debugPanels, buildDebugPanels(visibleSnapshot, performance));
  };

  const renderFailureState = (message: string): void => {
    renderManualRuntimeView({
      tick: 0,
      roomId: null,
      roomName: null,
      diagnostics: [message],
      inputTrace: defaultInputTrace(),
      player: null,
    }, 'none', 'viewer', message, null);
  };

  renderManualRuntimeView({
    tick: 0,
    roomId: null,
    roomName: null,
    diagnostics: activeBackend.diagnostics,
    inputTrace: defaultInputTrace(),
    player: null,
  }, 'none', 'viewer', 'Idle', null);

  const stopAutoTick = (): void => {
    if (autoTickHandle != null && resolved.clearInterval) {
      resolved.clearInterval(autoTickHandle);
    }
    autoTickHandle = null;
    autoTickRunning = false;
  };

  const updateExecutionControls = (): void => {
    const runtimeActive = loadedPackage != null && activeBackend.kind === 'wasm';
    pauseButton.disabled = !runtimeActive;
    pauseButton.textContent = runtimeActive ? (autoTickRunning ? 'Pause' : 'Resume') : 'Pause';
    resetButton.disabled = !runtimeActive;
  };

  doc.addEventListener('keydown', (event) => {
    const action = keyToAction(event.key);
    if (action) {
      keyboardState[action] = true;
    }
    const virtualKey = keyToVirtualKey(event.key);
    if (virtualKey != null) {
      heldVirtualKeys.add(virtualKey);
      pendingPressedVirtualKeys.add(virtualKey);
    }
  });

  doc.addEventListener('keyup', (event) => {
    const action = keyToAction(event.key);
    if (action) {
      keyboardState[action] = false;
    }
    const virtualKey = keyToVirtualKey(event.key);
    if (virtualKey != null) {
      heldVirtualKeys.delete(virtualKey);
      pendingReleasedVirtualKeys.add(virtualKey);
    }
  });

  const draw = async (): Promise<void> => {
    if (!loadedPackage) {
      return;
    }
    if (activeBackend.kind === 'wasm') {
      const snapshot = await activeBackend.bridge.snapshot();
      const frame = await activeBackend.bridge.frame();
      await resolved.renderWasmFrame(canvas, frame, loadedPackage.resources, input.value, renderCache);
      const roomLabel = snapshot.roomId != null
        ? `${snapshot.roomId}: ${snapshot.roomName ?? 'room'}`
        : 'none';
      renderManualRuntimeView(snapshot, roomLabel, 'wasm', `WASM runtime active: ${snapshot.roomName ?? 'room'} @ tick ${snapshot.tick}`, lastPerformance);
      return;
    }

    const room = activeBackend.roomId != null
      ? loadedPackage.rooms.find((candidate) => candidate.id === activeBackend.roomId)
      : null;
    const viewerSnapshot: WasmRuntimeBridgeSnapshot = {
      tick: 0,
      roomId: activeBackend.roomId,
      roomName: room?.name ?? (activeBackend.roomId != null ? 'Static room viewer' : null),
      diagnostics: activeBackend.diagnostics,
      inputTrace: defaultInputTrace(),
      player: null,
    };
    renderManualRuntimeView(
      viewerSnapshot,
      activeBackend.roomId != null
        ? `${activeBackend.roomId}: ${room?.name ?? 'room'}`
        : 'none',
      'viewer',
      room ? `Static room viewer: ${room.name}` : 'Static room viewer',
      null
    );
    if (activeBackend.roomId != null) {
      await renderRuntimeRoom(input.value, canvas, activeBackend.roomId, loadedPackage, resolved.renderStaticRoom);
      return;
    }

    clearCanvas(canvas);
  };

  const tickRuntimeOnce = async (): Promise<WasmRuntimeStepResult | null> => {
    if (!loadedPackage || activeBackend.kind !== 'wasm') {
      return null;
    }

    activeBackend.session.setInputState({
      ...keyboardState,
      keysHeld: [...heldVirtualKeys],
      keysPressed: [...pendingPressedVirtualKeys],
      keysReleased: [...pendingReleasedVirtualKeys]
    });
    const frameStart = resolved.now?.() ?? Date.now();
    const { snapshot, frame, timings } = await activeBackend.session.stepOnce();
    pendingPressedVirtualKeys.clear();
    pendingReleasedVirtualKeys.clear();
    const renderStart = resolved.now?.() ?? Date.now();
    await resolved.renderWasmFrame(canvas, frame, loadedPackage.resources, input.value, renderCache);
    const renderMs = (resolved.now?.() ?? Date.now()) - renderStart;
    lastPerformance = {
      inputMs: timings.inputMs,
      tickMs: timings.tickMs,
      snapshotMs: timings.snapshotMs,
      frameMs: timings.frameMs,
      runtimeMs: timings.runtimeMs,
      renderMs,
      totalMs: (resolved.now?.() ?? Date.now()) - frameStart,
      commandCount: frame.commands.length,
      skippedIntervals: skippedAutoTickIntervals
    };
    const roomLabel = snapshot.roomId != null
      ? `${snapshot.roomId}: ${snapshot.roomName ?? 'room'}`
      : 'none';
    renderManualRuntimeView(snapshot, roomLabel, 'wasm', `WASM runtime active: ${snapshot.roomName ?? 'room'} @ tick ${snapshot.tick}`, lastPerformance);
    return { snapshot, frame };
  };

  const startAutoTick = (): void => {
    if (!loadedPackage || activeBackend.kind !== 'wasm' || autoTickRunning || !resolved.setInterval) {
      updateExecutionControls();
      return;
    }

    autoTickHandle = resolved.setInterval(() => {
      if (autoTickInFlight) {
        skippedAutoTickIntervals += 1;
        return;
      }

      autoTickInFlight = true;
      void tickRuntimeOnce()
        .catch((error) => {
          stopAutoTick();
          renderFailureState(`Runtime tick failed: ${formatErrorMessage(error)}`);
        })
        .finally(() => {
          autoTickInFlight = false;
        });
    }, AUTO_TICK_MS);
    autoTickRunning = true;
    updateExecutionControls();
  };

  button.addEventListener('click', async () => {
    renderFailureState('Loading package...');
    button.disabled = true;
    select.disabled = true;
    stopAutoTick();

    try {
      const pkg = await resolved.loadPackage(input.value);
      loadedPackage = pkg;
      renderInspectors(doc, metaRoot, inspectors, pkg);
      setRoomOptions(doc, select, pkg);

      const defaultRoomId = pkg.manifest.default_room_id ?? pkg.rooms[0]?.id ?? null;
      let nextBackend: ShellBackend = {
        kind: 'viewer',
        roomId: defaultRoomId,
        diagnostics: ['Static room viewer active. Gameplay execution requires the WASM bridge.']
      };
      let roomId = defaultRoomId;
      let wasmBridgeError: unknown = null;

      if (resolved.loadWasmBridge) {
        try {
          const wasmBridge = await resolved.loadWasmBridge();
          const snapshot = await wasmBridge.boot(pkg, { basePath: input.value });
          nextBackend = {
            kind: 'wasm',
            bridge: wasmBridge,
            session: new WasmRuntimeSession(wasmBridge, resolved.now)
          };
          roomId = snapshot.roomId ?? roomId;
        } catch (error) {
          wasmBridgeError = error;
          nextBackend = {
            kind: 'viewer',
            roomId: defaultRoomId,
            diagnostics: [
              `WASM runtime unavailable: ${formatErrorMessage(error)}`,
              'Static room viewer active. Gameplay execution requires the WASM bridge.'
            ]
          };
        }
      }

      activeBackend = nextBackend;
      backendStatus.textContent = `Execution path: ${describeWasmBridgeAvailability(
        activeBackend.kind === 'wasm' ? activeBackend.bridge : null,
        wasmBridgeError
      )}`;
      updateExecutionControls();
      if (roomId != null) {
        select.value = String(roomId);
      } else {
        select.value = '';
      }
      await draw();
      if (activeBackend.kind === 'wasm') {
        startAutoTick();
      }
    } catch (error) {
      loadedPackage = null;
      activeBackend = {
        kind: 'viewer',
        roomId: null,
        diagnostics: ['Static room viewer idle. Load a package to inspect resources.']
      };
      metaRoot.replaceChildren();
      inspectors.replaceChildren();
      elements.debugPanels.replaceChildren();
      resetRoomOptions(doc, select, 'Load a package first');
      clearCanvas(canvas);
      renderFailureState(`Load failed: ${formatErrorMessage(error)}`);
      updateExecutionControls();
    } finally {
      button.disabled = false;
      if (loadedPackage && loadedPackage.rooms.length > 0) {
        select.disabled = false;
      }
    }
  });

  pauseButton.addEventListener('click', async () => {
    if (!loadedPackage || activeBackend.kind !== 'wasm') {
      return;
    }

    if (autoTickRunning) {
      stopAutoTick();
      updateExecutionControls();
      return;
    }

    startAutoTick();
  });

  resetButton.addEventListener('click', async () => {
    if (!loadedPackage || activeBackend.kind !== 'wasm') {
      return;
    }

    await activeBackend.bridge.reset();
    await draw();
    if (!autoTickRunning) {
      updateExecutionControls();
    }
  });

  select.addEventListener('change', async () => {
    if (!loadedPackage) {
      return;
    }
    const roomId = Number(select.value);
    if (activeBackend.kind === 'wasm') {
      await activeBackend.bridge.selectRoom(roomId);
      await draw();
      return;
    }

    activeBackend = {
      ...activeBackend,
      roomId: Number.isFinite(roomId) ? roomId : parseRoomId(select.value)
    };
    await draw();
  });

  updateExecutionControls();
}
