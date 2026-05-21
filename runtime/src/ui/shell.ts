import { loadPackage } from '../loadPackage';
import { makeBackgroundPathMap, makeSpriteFrameMap } from '../render/resourceCache';
import { renderStaticRoom } from '../render/staticRoomRenderer';
import { renderWasmFrame } from '../render/wasmFrameRenderer';
import { renderManifestSummary, renderObjectsSlice, renderRoomsSlice, renderScriptsSlice } from './inspectors';
import type { RuntimePackage } from '../types';
import { WasmRuntimeSession } from '../runtime/wasmSession';
import {
  describeWasmBridgeAvailability,
  type WasmRuntimeBridge,
  type WasmRuntimeInputState,
} from '../runtime/wasmBridge';

type ShellDependencies = {
  loadPackage: typeof loadPackage;
  renderStaticRoom: typeof renderStaticRoom;
  renderWasmFrame: typeof renderWasmFrame;
  loadWasmBridge?: () => Promise<WasmRuntimeBridge>;
};

const defaultDependencies: ShellDependencies = {
  loadPackage,
  renderStaticRoom,
  renderWasmFrame
};

type ShellElements = {
  input: HTMLInputElement;
  button: HTMLButtonElement;
  select: HTMLSelectElement;
  pauseButton: HTMLButtonElement;
  resetButton: HTMLButtonElement;
  status: HTMLElement;
  diagnostics: HTMLElement;
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

function createShell(doc: Document): { shell: HTMLElement; elements: ShellElements } {
  const shell = doc.createElement('div');
  shell.className = 'shell';

  const sidebar = doc.createElement('aside');
  sidebar.className = 'sidebar';

  const title = doc.createElement('h1');
  title.textContent = 'IWanna Runtime Shell';
  const intro = doc.createElement('p');
  intro.textContent = 'Developer harness for runtime package inspection and playable runtime execution.';

  const packageField = doc.createElement('label');
  packageField.className = 'field';
  packageField.append('Package');
  const input = doc.createElement('input');
  input.name = 'packagePath';
  input.value = '/packages/sample';
  packageField.append(input);

  const button = doc.createElement('button');
  button.type = 'button';
  button.textContent = 'Load Package';

  const pauseButton = doc.createElement('button');
  pauseButton.type = 'button';
  pauseButton.textContent = 'Pause';

  const resetButton = doc.createElement('button');
  resetButton.type = 'button';
  resetButton.textContent = 'Reset';

  const status = doc.createElement('p');
  status.className = 'status';
  status.textContent = 'Idle';

  const diagnostics = doc.createElement('section');
  diagnostics.className = 'diagnostics';

  const backendStatus = doc.createElement('p');
  backendStatus.className = 'hint';
  backendStatus.textContent = 'Execution path: static room viewer until a WASM bridge is configured.';

  const metaRoot = doc.createElement('section');
  metaRoot.className = 'meta';

  sidebar.append(title, intro, packageField, button, pauseButton, resetButton, status, backendStatus, diagnostics, metaRoot);

  const stage = doc.createElement('main');
  stage.className = 'stage';

  const toolbar = doc.createElement('div');
  toolbar.id = 'toolbar';
  const roomLabel = doc.createElement('label');
  roomLabel.append('Room');
  const select = doc.createElement('select');
  select.name = 'roomSelect';
  resetRoomOptions(doc, select, 'Load a package first');
  roomLabel.append(select);
  toolbar.append(roomLabel);

  const canvas = doc.createElement('canvas');
  canvas.id = 'room-canvas';
  canvas.width = 960;
  canvas.height = 540;

  const inspectors = doc.createElement('section');
  inspectors.id = 'inspectors';

  stage.append(toolbar, canvas, inspectors);
  shell.append(sidebar, stage);

  return {
    shell,
    elements: { input, button, select, pauseButton, resetButton, status, diagnostics, backendStatus, metaRoot, toolbar, inspectors, canvas }
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

function renderTextDiagnostics(doc: Document, root: HTMLElement, diagnostics: string[]): void {
  root.replaceChildren();
  const section = doc.createElement('section');
  section.className = 'inspector';
  const heading = doc.createElement('h3');
  heading.textContent = 'Diagnostics';
  const pre = doc.createElement('pre');
  pre.textContent = JSON.stringify(diagnostics, null, 2);
  section.append(heading, pre);
  root.append(section);
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

function keyToAction(key: string): 'left' | 'right' | 'jump' | 'restart' | null {
  switch (key) {
    case 'ArrowLeft':
    case 'a':
    case 'A':
      return 'left';
    case 'ArrowRight':
    case 'd':
    case 'D':
      return 'right';
    case ' ':
    case 'Spacebar':
    case 'ArrowUp':
    case 'w':
    case 'W':
      return 'jump';
    case 'r':
    case 'R':
      return 'restart';
    default:
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

  const { input, button, select, pauseButton, resetButton, status, diagnostics, backendStatus, metaRoot, inspectors, canvas } = elements;
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

  doc.addEventListener('keydown', (event) => {
    const action = keyToAction(event.key);
    if (action) {
      keyboardState[action] = true;
    }
  });

  doc.addEventListener('keyup', (event) => {
    const action = keyToAction(event.key);
    if (action) {
      keyboardState[action] = false;
    }
  });

  const draw = async (): Promise<void> => {
    if (!loadedPackage) {
      return;
    }
    if (activeBackend.kind === 'wasm') {
      const snapshot = await activeBackend.bridge.snapshot();
      const frame = await activeBackend.bridge.frame();
      await resolved.renderWasmFrame(canvas, frame, loadedPackage.resources, input.value);
      renderTextDiagnostics(doc, diagnostics, snapshot.diagnostics);
      status.textContent = `WASM runtime active: ${snapshot.roomName ?? 'room'} @ tick ${snapshot.tick}`;
      return;
    }

    renderTextDiagnostics(doc, diagnostics, activeBackend.diagnostics);
    if (activeBackend.roomId != null) {
      await renderRuntimeRoom(input.value, canvas, activeBackend.roomId, loadedPackage, resolved.renderStaticRoom);
      const room = loadedPackage.rooms.find((candidate) => candidate.id === activeBackend.roomId);
      status.textContent = room ? `Static room viewer: ${room.name}` : 'Static room viewer';
      return;
    }

    clearCanvas(canvas);
    status.textContent = 'Package loaded, but no room is available to preview.';
  };

  const updateExecutionControls = (): void => {
    const runtimeActive = loadedPackage != null && activeBackend.kind === 'wasm';
    pauseButton.disabled = !runtimeActive;
    resetButton.disabled = !runtimeActive;
  };

  button.addEventListener('click', async () => {
    status.textContent = 'Loading package...';
    button.disabled = true;
    select.disabled = true;

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
          const snapshot = await wasmBridge.boot(pkg);
          nextBackend = {
            kind: 'wasm',
            bridge: wasmBridge,
            session: new WasmRuntimeSession(wasmBridge)
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
    } catch (error) {
      loadedPackage = null;
      activeBackend = {
        kind: 'viewer',
        roomId: null,
        diagnostics: ['Static room viewer idle. Load a package to inspect resources.']
      };
      metaRoot.replaceChildren();
      inspectors.replaceChildren();
      diagnostics.replaceChildren();
      resetRoomOptions(doc, select, 'Load a package first');
      clearCanvas(canvas);
      status.textContent = `Load failed: ${formatErrorMessage(error)}`;
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

    activeBackend.session.setInputState(keyboardState);
    await activeBackend.session.stepOnce();
    await draw();
  });

  resetButton.addEventListener('click', async () => {
    if (!loadedPackage || activeBackend.kind !== 'wasm') {
      return;
    }

    await activeBackend.bridge.reset();
    await draw();
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
