import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createRuntimeShell } from './ui/shell';
import type { RuntimePackage } from './types';

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
  resources: {
    sprites: [
      {
        id: 0,
        name: 'Player',
        origin_x: 0,
        origin_y: 0,
        frame_paths: ['resources/sprites/0-0.png'],
        width: 30,
        height: 40
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

describe('main runtime shell', () => {
  let doc: FakeDocument;

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
    await Promise.resolve();
    await Promise.resolve();

    expect(loadPackage).toHaveBeenCalledWith('/packages/sample');
    expect(select?.options).toHaveLength(2);
    expect(collectText(doc.body)).toContain('sample.exe');
    expect(collectText(doc.body)).toContain('gm8');
    expect(collectText(doc.body)).toContain('partial');
    expect(renderStaticRoom).toHaveBeenCalledTimes(1);
    expect(doc.querySelectorAll('pre').length).toBeGreaterThanOrEqual(3);

    if (!select) {
      throw new Error('missing room select');
    }

    select.value = '1';
    select.dispatchEvent(new FakeEvent('change'));
    await Promise.resolve();

    expect(renderStaticRoom).toHaveBeenCalledTimes(2);
    expect(collectText(doc.body)).toContain('Room 2');

    pauseButton?.click();
    await Promise.resolve();
    expect(pauseButton?.textContent).toContain('Pause');

    resetButton?.click();
    await Promise.resolve();
    expect(renderStaticRoom).toHaveBeenCalledTimes(4);
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
});
