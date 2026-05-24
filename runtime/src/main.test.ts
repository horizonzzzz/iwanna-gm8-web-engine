import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createRuntimeShell } from './ui/shell';
import type { RuntimePackage } from './types';
import type { WasmRuntimeBridge } from './runtime/wasmBridge';

const samplePackage: RuntimePackage = {
  manifest: {
    format_version: 1,
    package_kind: 'runtime-v1',
    source_name: 'sample.exe',
    source_hash: 'abc123',
    engine_family: 'gm8',
    compatibility: 'partial',
    default_room_id: 0,
    room_count: 2,
    object_count: 1,
    script_block_count: 1,
    sprite_count: 1,
    background_count: 1,
    sound_count: 0,
    resource_index_path: 'resources/index.json',
    warnings: ['careful']
  },
  analysis: {
    dlls: [],
    included_files: [],
    warnings: ['sample'],
    unsupported_features: ['surface']
  },
  rooms: [
    {
      id: 0,
      name: 'Room 1',
      width: 320,
      height: 240,
      speed: 30,
      persistent: false,
      backgrounds: [],
      views_enabled: false,
      views: [],
      tiles: [],
      instances: [],
      creation_block_id: null,
      playable: true,
      transition_targets: []
    },
    {
      id: 1,
      name: 'Room 2',
      width: 640,
      height: 480,
      speed: 30,
      persistent: false,
      backgrounds: [],
      views_enabled: false,
      views: [],
      tiles: [],
      instances: [],
      creation_block_id: null,
      playable: true,
      transition_targets: []
    }
  ],
  objects: [
    {
      id: 0,
      name: 'Player',
      sprite_index: 0,
      parent_index: -1,
      depth: 0,
      persistent: false,
      visible: true,
      solid: false,
      mask_index: -1,
      is_hazard: null,
      is_checkpoint: null,
      is_player: true,
      events: []
    }
  ],
  scripts: {
    format: 'iwm-script-ir-v1',
    blocks: [{ id: 'block-1', name: 'Step', kind: 'step', support: 'action-list', executable_action_count: 0, ops: [] }]
  },
  rawLogic: {
    format: 'iwm-raw-logic-v1',
    room_creation_codes: [],
    instance_creation_codes: [],
    object_events: [],
    scripts: [],
    triggers: [],
    timelines: []
  },
  loweredLogic: {
    format: 'iwm-lowered-logic-v1',
    entries: []
  },
  resources: {
    sprites: [
      {
        id: 0,
        name: 'Player',
        origin_x: 0,
        origin_y: 0,
        frame_paths: ['resources/sprites/0-0.png'],
        width: 30,
        height: 40,
        bbox_left: 0,
        bbox_right: 29,
        bbox_top: 0,
        bbox_bottom: 39
      }
    ],
    backgrounds: [
      {
        id: 0,
        name: 'Bg',
        width: 640,
        height: 480,
        image_path: 'resources/backgrounds/0.png'
      }
    ],
    sounds: []
  }
};

class FakeEvent {
  constructor(public readonly type: string) {}
}

class FakeTextNode {
  constructor(public textContent: string) {}
}

class FakeElement {
  public children: Array<FakeElement | FakeTextNode> = [];
  public attributes = new Map<string, string>();
  public listeners = new Map<string, Array<() => void>>();
  public value = '';
  public disabled = false;
  public name = '';
  public type = '';
  public id = '';
  public className = '';
  public textContent = '';
  public width = 0;
  public height = 0;
  public ownerDocument: FakeDocument | null;
  public dataset: Record<string, string> = {};
  private _innerHTML = '';

  constructor(public readonly tagName: string, ownerDocument: FakeDocument | null) {
    this.ownerDocument = ownerDocument;
  }

  set innerHTML(value: string) {
    this._innerHTML = value;
    this.children = [];
  }

  get innerHTML(): string {
    return this._innerHTML;
  }

  append(...nodes: Array<FakeElement | FakeTextNode | string>): void {
    for (const node of nodes) {
      if (typeof node === 'string') {
        this.children.push(new FakeTextNode(node));
      } else {
        this.children.push(node);
      }
    }
  }

  replaceChildren(...nodes: Array<FakeElement | FakeTextNode>): void {
    this.children = [...nodes];
  }

  addEventListener(type: string, listener: () => void): void {
    const listeners = this.listeners.get(type) ?? [];
    listeners.push(listener);
    this.listeners.set(type, listeners);
  }

  dispatchEvent(event: FakeEvent): void {
    for (const listener of this.listeners.get(event.type) ?? []) {
      listener();
    }
  }

  click(): void {
    this.dispatchEvent(new FakeEvent('click'));
  }

  querySelector<T extends FakeElement>(selector: string): T | null {
    const matcher = createMatcher(selector);
    return findFirst(this, matcher) as T | null;
  }

  querySelectorAll<T extends FakeElement>(selector: string): T[] {
    const matcher = createMatcher(selector);
    const matches: T[] = [];
    visit(this, (node) => {
      if (matcher(node)) {
        matches.push(node as T);
      }
    });
    return matches;
  }

  get options(): FakeElement[] {
    return this.tagName === 'select' ? this.children.filter(isElement) : [];
  }
}

class FakeDocument {
  public readonly body: FakeElement;
  public listeners = new Map<string, Array<(event: KeyboardEvent) => void>>();

  constructor() {
    this.body = new FakeElement('body', this);
  }

  createElement(tagName: string): FakeElement {
    return new FakeElement(tagName, this);
  }

  querySelector<T extends FakeElement>(selector: string): T | null {
    return this.body.querySelector<T>(selector);
  }

  querySelectorAll<T extends FakeElement>(selector: string): T[] {
    return this.body.querySelectorAll<T>(selector);
  }

  addEventListener(type: string, listener: (event: KeyboardEvent) => void): void {
    const listeners = this.listeners.get(type) ?? [];
    listeners.push(listener);
    this.listeners.set(type, listeners);
  }

  dispatchKeyboardEvent(type: string, key: string): void {
    for (const listener of this.listeners.get(type) ?? []) {
      listener({ key } as KeyboardEvent);
    }
  }
}

function isElement(node: FakeElement | FakeTextNode): node is FakeElement {
  return node instanceof FakeElement;
}

function visit(node: FakeElement, visitor: (node: FakeElement) => void): void {
  for (const child of node.children) {
    if (!isElement(child)) {
      continue;
    }

    visitor(child);
    visit(child, visitor);
  }
}

function findFirst(node: FakeElement, matcher: (node: FakeElement) => boolean): FakeElement | null {
  for (const child of node.children) {
    if (!isElement(child)) {
      continue;
    }

    if (matcher(child)) {
      return child;
    }

    const nested = findFirst(child, matcher);
    if (nested) {
      return nested;
    }
  }

  return null;
}

function createMatcher(selector: string): (node: FakeElement) => boolean {
  if (selector === 'button' || selector === 'canvas' || selector === 'pre' || selector === 'select') {
    return (node) => node.tagName === selector;
  }

  if (selector.startsWith('#')) {
    const id = selector.slice(1);
    return (node) => node.id === id;
  }

  const inputName = selector.match(/^input\[name="(.+)"\]$/);
  if (inputName) {
    return (node) => node.tagName === 'input' && node.name === inputName[1];
  }

  const selectName = selector.match(/^select\[name="(.+)"\]$/);
  if (selectName) {
    return (node) => node.tagName === 'select' && node.name === selectName[1];
  }

  throw new Error(`Unsupported selector: ${selector}`);
}

function collectText(node: FakeElement): string {
  const parts: string[] = [];
  if (node.textContent) {
    parts.push(node.textContent);
  }

  for (const child of node.children) {
    if (isElement(child)) {
      parts.push(collectText(child));
    } else if (child.textContent) {
      parts.push(child.textContent);
    }
  }

  return parts.join(' ');
}

async function flushAsyncWork(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
}

class FakeIntervalScheduler {
  private nextId = 1;
  private readonly callbacks = new Map<number, () => void>();

  readonly setIntervalFn = vi.fn((callback: () => void, _ms: number) => {
    const id = this.nextId++;
    this.callbacks.set(id, callback);
    return id;
  });

  readonly clearIntervalFn = vi.fn((handle: unknown) => {
    this.callbacks.delete(Number(handle));
  });

  get activeCount(): number {
    return this.callbacks.size;
  }

  async fireAll(): Promise<void> {
    const callbacks = [...this.callbacks.values()];
    for (const callback of callbacks) {
      callback();
    }
    await flushAsyncWork();
  }
}

describe('main runtime shell', () => {
  let doc: FakeDocument;
  const defaultInputTrace = {
    jumpButtonKey: 0x20,
    jumpPressed: false,
    jumpJustPressed: false,
    jumpJustReleased: false,
    activeKeys: []
  };

  beforeEach(() => {
    doc = new FakeDocument();
  });

  it('renders the shell, defaults to /packages/sample, and loads/rerenders package rooms', async () => {
    const loadPackage = vi.fn(async () => samplePackage);
    const renderStaticRoom = vi.fn(async () => undefined);

    const root = doc.createElement('div');
    root.attributes.set('id', 'app');
    doc.body.append(root);

    createRuntimeShell(root as unknown as HTMLElement, { loadPackage, renderStaticRoom });

    const input = doc.querySelector<FakeElement>('input[name="packagePath"]');
    const buttons = doc.querySelectorAll<FakeElement>('button');
    const button = buttons[0];
    const pauseButton = buttons[1];
    const resetButton = buttons[2];
    const select = doc.querySelector<FakeElement>('select[name="roomSelect"]');

    expect(input?.value).toBe('/packages/sample');
    expect(button?.textContent).toContain('Load');
    expect(pauseButton?.textContent).toContain('Pause');
    expect(resetButton?.textContent).toContain('Reset');
    expect(doc.querySelector('#toolbar')).not.toBeNull();
    expect(doc.querySelector('#room-canvas')).not.toBeNull();
    expect(doc.querySelector('#inspectors')).not.toBeNull();

    button?.click();
    await flushAsyncWork();

    expect(loadPackage).toHaveBeenCalledWith('/packages/sample');
    expect(select?.options).toHaveLength(2);
    expect(collectText(doc.body)).toContain('sample.exe');
    expect(collectText(doc.body)).toContain('gm8');
    expect(collectText(doc.body)).toContain('partial');
    expect(collectText(doc.body)).toContain('No WASM bridge configured');
    expect(collectText(doc.body)).toContain('static room viewer');
    expect(renderStaticRoom).toHaveBeenCalledTimes(1);
    expect(doc.querySelectorAll('pre').length).toBeGreaterThanOrEqual(3);
    expect(pauseButton?.disabled).toBe(true);
    expect(resetButton?.disabled).toBe(true);

    if (!select) {
      throw new Error('missing room select');
    }

    select.value = '1';
    select.dispatchEvent(new FakeEvent('change'));
    await flushAsyncWork();

    expect(renderStaticRoom).toHaveBeenCalledTimes(2);
    expect(collectText(doc.body)).toContain('Room 2');
  });

  it('reports load failures without leaving stale room controls enabled', async () => {
    const loadPackage = vi.fn(async () => {
      throw new Error('bad package');
    });
    const renderStaticRoom = vi.fn(async () => undefined);

    const root = doc.createElement('div');
    root.attributes.set('id', 'app');
    doc.body.append(root);

    createRuntimeShell(root as unknown as HTMLElement, { loadPackage, renderStaticRoom });

    const button = doc.querySelector<FakeElement>('button');
    const select = doc.querySelector<FakeElement>('select[name="roomSelect"]');

    button?.click();
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(loadPackage).toHaveBeenCalledWith('/packages/sample');
    expect(renderStaticRoom).not.toHaveBeenCalled();
    expect(select?.disabled).toBe(true);
    expect(collectText(doc.body)).toContain('Load failed: bad package');
  });

  it('boots and auto-runs the wasm bridge at 60fps when one is available', async () => {
    const loadPackage = vi.fn(async () => samplePackage);
    const renderStaticRoom = vi.fn(async () => undefined);
    const renderWasmFrame = vi.fn(async () => undefined);
    const scheduler = new FakeIntervalScheduler();
    let currentTick = 0;
    let currentRoomId = 0;
    let currentRoomName = 'Room 1';
    let currentDiagnostics = ['boot ok'];
    let currentPlayer = {
      x: 12,
      y: 34,
      hspeed: 1,
      vspeed: 0,
      facing_left: false,
      jump: { grounded: true, active: false, holdFrames: 0, cutApplied: false }
    };
    const wasmBridge: WasmRuntimeBridge = {
      backend: 'opengmk-wasm',
      boot: vi.fn(async () => ({
        tick: currentTick,
        roomId: currentRoomId,
        roomName: currentRoomName,
        diagnostics: currentDiagnostics,
        inputTrace: defaultInputTrace,
        player: currentPlayer
      })),
      snapshot: vi.fn(async () => ({
        tick: currentTick,
        roomId: currentRoomId,
        roomName: currentRoomName,
        diagnostics: currentDiagnostics,
        inputTrace: defaultInputTrace,
        player: currentPlayer
      })),
      frame: vi.fn(async () => ({
        tick: currentTick,
        roomId: currentRoomId,
        width: 320,
        height: 240,
        commands: [{ kind: 'present' as const }]
      })),
      setInput: vi.fn(async () => ({
        tick: currentTick,
        roomId: currentRoomId,
        roomName: currentRoomName,
        diagnostics: ['input ok'],
        inputTrace: defaultInputTrace,
        player: { ...currentPlayer, x: currentPlayer.x + 1 }
      })),
      tick: vi.fn(async (frames = 1) => ({
        tick: (currentTick += frames),
        roomId: currentRoomId,
        roomName: currentRoomName,
        diagnostics: (currentDiagnostics = ['tick ok']),
        inputTrace: defaultInputTrace,
        player: currentPlayer
      })),
      reset: vi.fn(async () => ({
        tick: (currentTick = 0),
        roomId: (currentRoomId = 0),
        roomName: (currentRoomName = 'Room 1'),
        diagnostics: (currentDiagnostics = ['reset ok']),
        inputTrace: defaultInputTrace,
        player: (currentPlayer = {
          x: 12,
          y: 34,
          hspeed: 0,
          vspeed: 0,
          facing_left: false,
          jump: { grounded: true, active: false, holdFrames: 0, cutApplied: false }
        })
      })),
      selectRoom: vi.fn(async (roomId: number) => ({
        tick: currentTick,
        roomId: (currentRoomId = roomId),
        roomName: (currentRoomName = roomId === 1 ? 'Room 2' : 'Room 1'),
        diagnostics: (currentDiagnostics = ['select ok']),
        inputTrace: defaultInputTrace,
        player: (currentPlayer = roomId === 1
          ? null
          : {
              x: 12,
              y: 34,
              hspeed: 0,
              vspeed: 0,
              facing_left: false,
              jump: { grounded: true, active: false, holdFrames: 0, cutApplied: false }
            })
      })),
      diagnostics: vi.fn(async () => ['diag ok'])
    };
    const loadWasmBridge = vi.fn(async () => wasmBridge);

    const root = doc.createElement('div');
    root.attributes.set('id', 'app');
    doc.body.append(root);

    createRuntimeShell(root as unknown as HTMLElement, {
      loadPackage,
      renderStaticRoom,
      renderWasmFrame,
      loadWasmBridge,
      setInterval: scheduler.setIntervalFn,
      clearInterval: scheduler.clearIntervalFn
    });

    const buttons = doc.querySelectorAll<FakeElement>('button');
    const button = buttons[0];
    const pauseButton = buttons[1];
    const resetButton = buttons[2];
    const select = doc.querySelector<FakeElement>('select[name="roomSelect"]');

    button?.click();
    await flushAsyncWork();

    expect(loadWasmBridge).toHaveBeenCalledTimes(1);
    expect(wasmBridge.boot).toHaveBeenCalledWith(samplePackage);
    expect(wasmBridge.frame).toHaveBeenCalled();
    expect(renderWasmFrame).toHaveBeenCalled();
    expect(collectText(doc.body)).toContain('WASM bridge available');
    expect(collectText(doc.body)).toContain('WASM runtime active');
    expect(collectText(doc.body)).toContain('Room 1');
    expect(collectText(doc.body)).toContain('x=12');
    expect(collectText(doc.body)).toContain('hspeed=1');
    expect(collectText(doc.body)).toContain('grounded=true');
    expect(collectText(doc.body)).toContain('jumpActive=false');
    expect(collectText(doc.body)).toContain('hold=0');
    expect(collectText(doc.body)).toContain('cut=false');
    expect(doc.querySelector('#runtime-room')).not.toBeNull();
    expect(doc.querySelector('#runtime-tick')).not.toBeNull();
    expect(doc.querySelector('#runtime-player')).not.toBeNull();
    expect(pauseButton?.disabled).toBe(false);
    expect(resetButton?.disabled).toBe(false);
    expect(pauseButton?.textContent).toContain('Pause');
    expect(scheduler.setIntervalFn).toHaveBeenCalledTimes(1);
    expect(scheduler.activeCount).toBe(1);
    expect(scheduler.setIntervalFn.mock.calls[0]?.[1]).toBeCloseTo(1000 / 60, 5);

    await scheduler.fireAll();
    expect(wasmBridge.tick).toHaveBeenCalledTimes(1);
    expect(collectText(doc.body)).toContain('Tick: 1');

    pauseButton?.click();
    expect(scheduler.clearIntervalFn).toHaveBeenCalledTimes(1);
    expect(scheduler.activeCount).toBe(0);
    expect(pauseButton?.textContent).toContain('Resume');

    await scheduler.fireAll();
    expect(wasmBridge.tick).toHaveBeenCalledTimes(1);

    pauseButton?.click();
    expect(scheduler.setIntervalFn).toHaveBeenCalledTimes(2);
    expect(scheduler.activeCount).toBe(1);
    expect(pauseButton?.textContent).toContain('Pause');

    doc.dispatchKeyboardEvent('keydown', 'ArrowLeft');
    await scheduler.fireAll();

    expect(wasmBridge.setInput).toHaveBeenLastCalledWith({
      left: true,
      right: false,
      jump: false,
      jumpPressed: false,
      jumpReleased: false,
      restart: false,
      keysHeld: [0x25],
      keysPressed: [0x25],
      keysReleased: []
    });
    expect(wasmBridge.tick).toHaveBeenCalledTimes(2);

    if (!select) {
      throw new Error('missing room select');
    }

    select.value = '1';
    select.dispatchEvent(new FakeEvent('change'));
    await flushAsyncWork();

    expect(wasmBridge.selectRoom).toHaveBeenCalledWith(1);
    expect(collectText(doc.body)).toContain('Room 2');

    resetButton?.click();
    await flushAsyncWork();

    expect(wasmBridge.reset).toHaveBeenCalledTimes(1);
    expect(collectText(doc.body)).toContain('Tick: 0');
  });

  it('routes shift as a raw key without mapping w or arrow-up into semantic jump', async () => {
    const loadPackage = vi.fn(async () => samplePackage);
    const renderStaticRoom = vi.fn(async () => undefined);
    const scheduler = new FakeIntervalScheduler();
    const renderWasmFrame = vi.fn(async () => undefined);
    const wasmBridge: WasmRuntimeBridge = {
      backend: 'opengmk-wasm',
      boot: vi.fn(async () => ({
        tick: 0,
        roomId: 0,
        roomName: 'Room 1',
        diagnostics: [],
        inputTrace: defaultInputTrace,
        player: {
          x: 12,
          y: 34,
          hspeed: 0,
          vspeed: 0,
          facing_left: false,
          jump: { grounded: true, active: false, holdFrames: 0, cutApplied: false }
        }
      })),
      snapshot: vi.fn(async () => ({
        tick: 0,
        roomId: 0,
        roomName: 'Room 1',
        diagnostics: [],
        inputTrace: defaultInputTrace,
        player: {
          x: 12,
          y: 34,
          hspeed: 0,
          vspeed: 0,
          facing_left: false,
          jump: { grounded: true, active: false, holdFrames: 0, cutApplied: false }
        }
      })),
      frame: vi.fn(async () => ({
        tick: 0,
        roomId: 0,
        width: 320,
        height: 240,
        commands: [{ kind: 'clear', colour: [0, 0, 0, 255] }, { kind: 'present' }]
      })),
      setInput: vi.fn(async () => ({
        tick: 0,
        roomId: 0,
        roomName: 'Room 1',
        diagnostics: [],
        inputTrace: defaultInputTrace,
        player: {
          x: 12,
          y: 34,
          hspeed: 0,
          vspeed: 0,
          facing_left: false,
          jump: { grounded: true, active: false, holdFrames: 0, cutApplied: false }
        }
      })),
      tick: vi.fn(async () => ({
        tick: 1,
        roomId: 0,
        roomName: 'Room 1',
        diagnostics: [],
        inputTrace: defaultInputTrace,
        player: {
          x: 12,
          y: 34,
          hspeed: 0,
          vspeed: 0,
          facing_left: false,
          jump: { grounded: true, active: false, holdFrames: 0, cutApplied: false }
        }
      })),
      reset: vi.fn(async () => ({
        tick: 0,
        roomId: 0,
        roomName: 'Room 1',
        diagnostics: [],
        inputTrace: defaultInputTrace,
        player: {
          x: 12,
          y: 34,
          hspeed: 0,
          vspeed: 0,
          facing_left: false,
          jump: { grounded: true, active: false, holdFrames: 0, cutApplied: false }
        }
      })),
      selectRoom: vi.fn(async () => ({
        tick: 0,
        roomId: 0,
        roomName: 'Room 1',
        diagnostics: [],
        inputTrace: defaultInputTrace,
        player: {
          x: 12,
          y: 34,
          hspeed: 0,
          vspeed: 0,
          facing_left: false,
          jump: { grounded: true, active: false, holdFrames: 0, cutApplied: false }
        }
      })),
      diagnostics: vi.fn(async () => [])
    };

    const root = doc.createElement('div');
    doc.body.append(root);

    createRuntimeShell(root as unknown as HTMLElement, {
      loadPackage,
      renderStaticRoom,
      renderWasmFrame,
      loadWasmBridge: vi.fn(async () => wasmBridge),
      setInterval: scheduler.setIntervalFn,
      clearInterval: scheduler.clearIntervalFn
    });

    doc.querySelectorAll<FakeElement>('button')[0]?.click();
    await flushAsyncWork();

    doc.dispatchKeyboardEvent('keydown', 'Shift');
    await scheduler.fireAll();
    expect(wasmBridge.setInput).toHaveBeenLastCalledWith({
      left: false,
      right: false,
      jump: false,
      jumpPressed: false,
      jumpReleased: false,
      restart: false,
      keysHeld: [0x10],
      keysPressed: [0x10],
      keysReleased: []
    });

    doc.dispatchKeyboardEvent('keyup', 'Shift');
    await scheduler.fireAll();
    expect(wasmBridge.setInput).toHaveBeenLastCalledWith({
      left: false,
      right: false,
      jump: false,
      jumpPressed: false,
      jumpReleased: false,
      restart: false,
      keysHeld: [],
      keysPressed: [],
      keysReleased: [0x10]
    });

    doc.dispatchKeyboardEvent('keydown', 'W');
    await scheduler.fireAll();
    expect(wasmBridge.setInput).toHaveBeenLastCalledWith({
      left: false,
      right: false,
      jump: false,
      jumpPressed: false,
      jumpReleased: false,
      restart: false,
      keysHeld: [0x57],
      keysPressed: [0x57],
      keysReleased: []
    });

    doc.dispatchKeyboardEvent('keyup', 'W');
    doc.dispatchKeyboardEvent('keydown', 'ArrowUp');
    await scheduler.fireAll();
    expect(wasmBridge.setInput).toHaveBeenLastCalledWith({
      left: false,
      right: false,
      jump: false,
      jumpPressed: false,
      jumpReleased: false,
      restart: false,
      keysHeld: [0x26],
      keysPressed: [0x26],
      keysReleased: [0x57]
    });
  });

  it('preserves a very short shift tap that begins and ends before the next tick', async () => {
    const loadPackage = vi.fn(async () => samplePackage);
    const renderStaticRoom = vi.fn(async () => undefined);
    const scheduler = new FakeIntervalScheduler();
    const renderWasmFrame = vi.fn(async () => undefined);
    const wasmBridge: WasmRuntimeBridge = {
      backend: 'opengmk-wasm',
      boot: vi.fn(async () => ({
        tick: 0,
        roomId: 0,
        roomName: 'Room 1',
        diagnostics: [],
        inputTrace: defaultInputTrace,
        player: {
          x: 12,
          y: 34,
          hspeed: 0,
          vspeed: 0,
          facing_left: false,
          jump: { grounded: true, active: false, holdFrames: 0, cutApplied: false }
        }
      })),
      snapshot: vi.fn(async () => ({
        tick: 0,
        roomId: 0,
        roomName: 'Room 1',
        diagnostics: [],
        inputTrace: defaultInputTrace,
        player: {
          x: 12,
          y: 34,
          hspeed: 0,
          vspeed: 0,
          facing_left: false,
          jump: { grounded: true, active: false, holdFrames: 0, cutApplied: false }
        }
      })),
      frame: vi.fn(async () => ({
        tick: 0,
        roomId: 0,
        width: 320,
        height: 240,
        commands: [{ kind: 'clear', colour: [0, 0, 0, 255] }, { kind: 'present' }]
      })),
      setInput: vi.fn(async () => ({
        tick: 0,
        roomId: 0,
        roomName: 'Room 1',
        diagnostics: [],
        inputTrace: defaultInputTrace,
        player: {
          x: 12,
          y: 34,
          hspeed: 0,
          vspeed: 0,
          facing_left: false,
          jump: { grounded: true, active: false, holdFrames: 0, cutApplied: false }
        }
      })),
      tick: vi.fn(async () => ({
        tick: 1,
        roomId: 0,
        roomName: 'Room 1',
        diagnostics: [],
        inputTrace: defaultInputTrace,
        player: {
          x: 12,
          y: 34,
          hspeed: 0,
          vspeed: 0,
          facing_left: false,
          jump: { grounded: true, active: false, holdFrames: 0, cutApplied: false }
        }
      })),
      reset: vi.fn(async () => ({
        tick: 0,
        roomId: 0,
        roomName: 'Room 1',
        diagnostics: [],
        inputTrace: defaultInputTrace,
        player: {
          x: 12,
          y: 34,
          hspeed: 0,
          vspeed: 0,
          facing_left: false,
          jump: { grounded: true, active: false, holdFrames: 0, cutApplied: false }
        }
      })),
      selectRoom: vi.fn(async () => ({
        tick: 0,
        roomId: 0,
        roomName: 'Room 1',
        diagnostics: [],
        inputTrace: defaultInputTrace,
        player: {
          x: 12,
          y: 34,
          hspeed: 0,
          vspeed: 0,
          facing_left: false,
          jump: { grounded: true, active: false, holdFrames: 0, cutApplied: false }
        }
      })),
      diagnostics: vi.fn(async () => [])
    };

    const root = doc.createElement('div');
    doc.body.append(root);

    createRuntimeShell(root as unknown as HTMLElement, {
      loadPackage,
      renderStaticRoom,
      renderWasmFrame,
      loadWasmBridge: vi.fn(async () => wasmBridge),
      setInterval: scheduler.setIntervalFn,
      clearInterval: scheduler.clearIntervalFn
    });

    doc.querySelectorAll<FakeElement>('button')[0]?.click();
    await flushAsyncWork();

    doc.dispatchKeyboardEvent('keydown', 'Shift');
    doc.dispatchKeyboardEvent('keyup', 'Shift');
    await scheduler.fireAll();

    expect(wasmBridge.setInput).toHaveBeenLastCalledWith({
      left: false,
      right: false,
      jump: false,
      jumpPressed: false,
      jumpReleased: false,
      restart: false,
      keysHeld: [],
      keysPressed: [0x10],
      keysReleased: [0x10]
    });
  });

  it('reuses the same render cache and keeps runtime diagnostics bounded while auto-running', async () => {
    const loadPackage = vi.fn(async () => samplePackage);
    const renderStaticRoom = vi.fn(async () => undefined);
    const scheduler = new FakeIntervalScheduler();
    const renderWasmFrame = vi.fn(async () => undefined);
    let currentTick = 0;
    let currentDiagnostics = ['boot ok'];
    const wasmBridge: WasmRuntimeBridge = {
      backend: 'opengmk-wasm',
      boot: vi.fn(async () => ({
        tick: currentTick,
        roomId: 0,
        roomName: 'Room 1',
        diagnostics: currentDiagnostics,
        inputTrace: defaultInputTrace,
        player: {
          x: 12,
          y: 34,
          hspeed: 0,
          vspeed: 0,
          facing_left: false,
          jump: { grounded: true, active: false, holdFrames: 0, cutApplied: false }
        }
      })),
      snapshot: vi.fn(async () => ({
        tick: currentTick,
        roomId: 0,
        roomName: 'Room 1',
        diagnostics: currentDiagnostics,
        inputTrace: defaultInputTrace,
        player: {
          x: 12,
          y: 34,
          hspeed: 0,
          vspeed: 0,
          facing_left: false,
          jump: { grounded: true, active: false, holdFrames: 0, cutApplied: false }
        }
      })),
      frame: vi.fn(async () => ({
        tick: currentTick,
        roomId: 0,
        width: 320,
        height: 240,
        commands: [{ kind: 'present' as const }]
      })),
      setInput: vi.fn(async () => ({
        tick: currentTick,
        roomId: 0,
        roomName: 'Room 1',
        diagnostics: currentDiagnostics,
        inputTrace: defaultInputTrace,
        player: {
          x: 12,
          y: 34,
          hspeed: 0,
          vspeed: 0,
          facing_left: false,
          jump: { grounded: true, active: false, holdFrames: 0, cutApplied: false }
        }
      })),
      tick: vi.fn(async () => ({
        tick: ++currentTick,
        roomId: 0,
        roomName: 'Room 1',
        diagnostics: (currentDiagnostics = Array.from({ length: currentTick }, (_, index) => `diag-${index + 1}`)),
        inputTrace: defaultInputTrace,
        player: {
          x: 12,
          y: 34,
          hspeed: 0,
          vspeed: 0,
          facing_left: false,
          jump: { grounded: currentTick % 2 === 0, active: currentTick % 2 === 1, holdFrames: currentTick, cutApplied: false }
        }
      })),
      reset: vi.fn(async () => ({
        tick: (currentTick = 0),
        roomId: 0,
        roomName: 'Room 1',
        diagnostics: (currentDiagnostics = ['reset ok']),
        inputTrace: defaultInputTrace,
        player: {
          x: 12,
          y: 34,
          hspeed: 0,
          vspeed: 0,
          facing_left: false,
          jump: { grounded: true, active: false, holdFrames: 0, cutApplied: false }
        }
      })),
      selectRoom: vi.fn(async () => ({
        tick: currentTick,
        roomId: 0,
        roomName: 'Room 1',
        diagnostics: currentDiagnostics,
        inputTrace: defaultInputTrace,
        player: {
          x: 12,
          y: 34,
          hspeed: 0,
          vspeed: 0,
          facing_left: false,
          jump: { grounded: true, active: false, holdFrames: 0, cutApplied: false }
        }
      })),
      diagnostics: vi.fn(async () => currentDiagnostics)
    };

    const root = doc.createElement('div');
    doc.body.append(root);

    createRuntimeShell(root as unknown as HTMLElement, {
      loadPackage,
      renderStaticRoom,
      renderWasmFrame,
      loadWasmBridge: vi.fn(async () => wasmBridge),
      setInterval: scheduler.setIntervalFn,
      clearInterval: scheduler.clearIntervalFn
    });

    doc.querySelectorAll<FakeElement>('button')[0]?.click();
    await flushAsyncWork();

    for (let index = 0; index < 12; index += 1) {
      await scheduler.fireAll();
    }

    expect(renderWasmFrame).toHaveBeenCalledTimes(13);
    const cacheReferences = renderWasmFrame.mock.calls.map((call) => call[4]);
    expect(new Set(cacheReferences).size).toBe(1);

    const runtimeDiagnosticsText = doc.querySelector<FakeElement>('#runtime-diagnostics')?.textContent ?? '';
    expect(runtimeDiagnosticsText).toContain('diag-5');
    expect(runtimeDiagnosticsText).toContain('diag-12');
    expect(runtimeDiagnosticsText).not.toContain('diag-4 |');
    const runtimePlayerText = doc.querySelector<FakeElement>('#runtime-player')?.textContent ?? '';
    expect(runtimePlayerText).toContain('grounded=');
    expect(runtimePlayerText).toContain('jumpActive=');
    expect(runtimePlayerText).toContain('hold=');
    expect(runtimePlayerText).toContain('cut=');

    const diagnosticsPre = doc.querySelector<FakeElement>('pre');
    expect((diagnosticsPre?.textContent ?? '').length).toBeLessThan(200);
  });

  it('falls back to the static room viewer when the wasm runtime cannot boot', async () => {
    const loadPackage = vi.fn(async () => samplePackage);
    const renderStaticRoom = vi.fn(async () => undefined);
    const loadWasmBridge = vi.fn(async () => {
      throw new Error('bridge boot failed');
    });

    const root = doc.createElement('div');
    root.attributes.set('id', 'app');
    doc.body.append(root);

    createRuntimeShell(root as unknown as HTMLElement, { loadPackage, renderStaticRoom, loadWasmBridge });

    const buttons = doc.querySelectorAll<FakeElement>('button');
    const button = buttons[0];
    const pauseButton = buttons[1];
    const resetButton = buttons[2];
    const select = doc.querySelector<FakeElement>('select[name="roomSelect"]');

    button?.click();
    await flushAsyncWork();

    expect(loadPackage).toHaveBeenCalledWith('/packages/sample');
    expect(loadWasmBridge).toHaveBeenCalledTimes(1);
    expect(renderStaticRoom).toHaveBeenCalledTimes(1);
    expect(collectText(doc.body)).toContain('bridge boot failed');
    expect(collectText(doc.body)).toContain('static room viewer');
    expect(pauseButton?.disabled).toBe(true);
    expect(resetButton?.disabled).toBe(true);

    if (!select) {
      throw new Error('missing room select');
    }

    select.value = '1';
    select.dispatchEvent(new FakeEvent('change'));
    await flushAsyncWork();

    expect(renderStaticRoom).toHaveBeenCalledTimes(2);
  });
});
