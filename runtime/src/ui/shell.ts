import { loadPackage } from '../loadPackage';
import { makeBackgroundPathMap, makeSpriteFrameMap } from '../render/resourceCache';
import { renderStaticRoom } from '../render/staticRoomRenderer';
import {
  renderManifestSummary,
  renderObjectsSlice,
  renderRoomsSlice,
  renderScriptsSlice
} from './inspectors';
import type { RuntimePackage } from '../types';

type ShellDependencies = {
  loadPackage: typeof loadPackage;
  renderStaticRoom: typeof renderStaticRoom;
};

const defaultDependencies: ShellDependencies = {
  loadPackage,
  renderStaticRoom
};

type ShellElements = {
  input: HTMLInputElement;
  button: HTMLButtonElement;
  select: HTMLSelectElement;
  status: HTMLElement;
  metaRoot: HTMLElement;
  toolbar: HTMLElement;
  inspectors: HTMLElement;
  canvas: HTMLCanvasElement;
};

function createShell(doc: Document): { shell: HTMLElement; elements: ShellElements } {
  const shell = doc.createElement('div');
  shell.className = 'shell';

  const sidebar = doc.createElement('aside');
  sidebar.className = 'sidebar';

  const title = doc.createElement('h1');
  title.textContent = 'IWanna Runtime Shell';
  const intro = doc.createElement('p');
  intro.textContent = 'Developer harness for runtime package inspection and static room rendering.';

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

  const status = doc.createElement('p');
  status.className = 'status';
  status.textContent = 'Idle';

  const metaRoot = doc.createElement('section');
  metaRoot.className = 'meta';

  sidebar.append(title, intro, packageField, button, status, metaRoot);

  const stage = doc.createElement('main');
  stage.className = 'stage';

  const toolbar = doc.createElement('div');
  toolbar.id = 'toolbar';
  const roomLabel = doc.createElement('label');
  roomLabel.append('Room');
  const select = doc.createElement('select');
  select.name = 'roomSelect';
  select.disabled = true;
  const emptyOption = doc.createElement('option');
  emptyOption.value = '';
  emptyOption.textContent = 'Load a package first';
  select.append(emptyOption);
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
    elements: { input, button, select, status, metaRoot, toolbar, inspectors, canvas }
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
  inspectors.replaceChildren(
    renderRoomsSlice(doc, pkg.rooms),
    renderObjectsSlice(doc, pkg.objects),
    renderScriptsSlice(doc, pkg.scripts)
  );
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

  const { input, button, select, status, metaRoot, inspectors, canvas } = elements;
  let loadedPackage: RuntimePackage | null = null;

  const drawRoom = async (roomId: number): Promise<void> => {
    if (!loadedPackage) {
      return;
    }

    const room = loadedPackage.rooms.find((candidate) => candidate.id === roomId);
    if (!room) {
      return;
    }

    const backgroundPaths = makeBackgroundPathMap(input.value, loadedPackage.resources);
    const spritePaths = makeSpriteFrameMap(input.value, loadedPackage.resources);
    await resolved.renderStaticRoom(canvas, room, loadedPackage.objects, backgroundPaths, spritePaths);
    status.textContent = `Viewing ${room.name}`;
  };

  button.addEventListener('click', async () => {
    status.textContent = 'Loading package...';
    const pkg = await resolved.loadPackage(input.value);
    loadedPackage = pkg;
    renderInspectors(doc, metaRoot, inspectors, pkg);
    setRoomOptions(doc, select, pkg);
    const initialRoomId = pkg.manifest.default_room_id ?? pkg.rooms[0]?.id;
    if (initialRoomId != null) {
      select.value = String(initialRoomId);
      await drawRoom(initialRoomId);
    }
  });

  select.addEventListener('change', async () => {
    await drawRoom(Number(select.value));
  });
}
