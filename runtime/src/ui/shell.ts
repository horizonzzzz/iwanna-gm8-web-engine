import { loadPackage } from '../loadPackage';
import { makeBackgroundPathMap, makeSpriteFrameMap } from '../render/resourceCache';
import { renderStaticRoom } from '../render/staticRoomRenderer';
import { renderManifestSummary, renderObjectsSlice, renderRoomsSlice, renderScriptsSlice } from './inspectors';
import type { RuntimePackage } from '../types';
import { GameRuntime } from '../runtime/gameRuntime';
import {
  describeWasmBridgeAvailability,
  loadWasmRuntimeBridge,
  type WasmRuntimeBridge,
} from '../runtime/wasmBridge';

type ShellDependencies = {
  loadPackage: typeof loadPackage;
  renderStaticRoom: typeof renderStaticRoom;
  loadWasmBridge?: typeof loadWasmRuntimeBridge;
};

const defaultDependencies: ShellDependencies = {
  loadPackage,
  renderStaticRoom
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
  backendStatus.textContent = 'Execution path: transitional TypeScript runtime. No WASM bridge configured yet.';

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

function renderDiagnostics(doc: Document, root: HTMLElement, diagnostics: ReturnType<GameRuntime['getDiagnostics']>): void {
  root.replaceChildren();
  const section = doc.createElement('section');
  section.className = 'inspector';
  const heading = doc.createElement('h3');
  heading.textContent = 'Diagnostics';
  const pre = doc.createElement('pre');
  pre.textContent = JSON.stringify(diagnostics.slice(-12), null, 2);
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
  const runtime = new GameRuntime();
  let loadedPackage: RuntimePackage | null = null;
  let wasmBridge: WasmRuntimeBridge | null = null;

  const draw = async (): Promise<void> => {
    if (!loadedPackage) {
      return;
    }
    const snapshot = runtime.snapshot;
    renderDiagnostics(doc, diagnostics, snapshot.diagnostics);
    if (snapshot.roomId != null) {
      await renderRuntimeRoom(input.value, canvas, snapshot.roomId, loadedPackage, resolved.renderStaticRoom);
      status.textContent = `${snapshot.status}: ${snapshot.roomName ?? 'room'} @ tick ${snapshot.tick}`;
    }
  };

  const updatePausedButton = (): void => {
    pauseButton.textContent = runtime.snapshot.paused ? 'Resume' : 'Pause';
  };

  button.addEventListener('click', async () => {
    status.textContent = 'Loading package...';
    button.disabled = true;
    select.disabled = true;

    try {
      let wasmBridgeError: unknown = null;
      wasmBridge = null;
      if (resolved.loadWasmBridge) {
        try {
          wasmBridge = await resolved.loadWasmBridge(async () => ({}));
        } catch (error) {
          wasmBridgeError = error;
        }
      }
      backendStatus.textContent = `Execution path: ${describeWasmBridgeAvailability(wasmBridge, wasmBridgeError)}`;

      const pkg = await resolved.loadPackage(input.value);
      loadedPackage = pkg;
      renderInspectors(doc, metaRoot, inspectors, pkg);
      setRoomOptions(doc, select, pkg);
      runtime.load(pkg);
      runtime.pause();
      updatePausedButton();
      const roomId = runtime.snapshot.roomId ?? pkg.manifest.default_room_id ?? pkg.rooms[0]?.id;
      if (roomId != null) {
        select.value = String(roomId);
      }
      await draw();
      status.textContent =
        runtime.snapshot.roomId != null ? `Loaded ${pkg.manifest.source_name}` : 'Package loaded';
    } catch (error) {
      loadedPackage = null;
      metaRoot.replaceChildren();
      inspectors.replaceChildren();
      diagnostics.replaceChildren();
      backendStatus.textContent = 'Execution path: transitional TypeScript runtime. WASM bridge probe did not complete.';
      resetRoomOptions(doc, select, 'Load a package first');
      clearCanvas(canvas);
      status.textContent = `Load failed: ${formatErrorMessage(error)}`;
    } finally {
      button.disabled = false;
      if (loadedPackage && loadedPackage.rooms.length > 0) {
        select.disabled = false;
      }
    }
  });

  pauseButton.addEventListener('click', async () => {
    if (!loadedPackage) {
      return;
    }
    if (runtime.snapshot.paused) {
      runtime.resume();
      runtime.tick();
    } else {
      runtime.pause();
    }
    updatePausedButton();
    await draw();
  });

  resetButton.addEventListener('click', async () => {
    if (!loadedPackage) {
      return;
    }
    runtime.reset();
    runtime.tick();
    await draw();
  });

  select.addEventListener('change', async () => {
    if (!loadedPackage) {
      return;
    }
    const roomId = Number(select.value);
    runtime.queueRoomTransition({ roomId });
    runtime.tick();
    await draw();
  });
}
