import { describe, expect, it } from 'vitest';
import { renderManualTestHud } from './hud';
import type { ManualTestSnapshot } from './traceView';

class TestElement {
  public readonly children: Array<TestElement | string> = [];
  public id = '';
  public className = '';
  public textContent = '';

  constructor(public readonly tagName: string) {}

  append(...children: Array<TestElement | string>): void {
    this.children.push(...children);
  }

  replaceChildren(...children: TestElement[]): void {
    this.children.length = 0;
    this.children.push(...children);
  }

  querySelector(selector: string): TestElement | null {
    const id = selector.startsWith('#') ? selector.slice(1) : null;
    if (!id) {
      throw new Error(`Unsupported selector ${selector}`);
    }
    return findById(this, id);
  }
}

class TestDocument {
  createElement(tagName: string): TestElement {
    return new TestElement(tagName);
  }
}

function findById(root: TestElement, id: string): TestElement | null {
  for (const child of root.children) {
    if (typeof child === 'string') {
      continue;
    }
    if (child.id === id) {
      return child;
    }
    const nested = findById(child, id);
    if (nested) {
      return nested;
    }
  }
  return null;
}

function collectText(root: TestElement): string {
  const parts = root.textContent ? [root.textContent] : [];
  for (const child of root.children) {
    parts.push(typeof child === 'string' ? child : collectText(child));
  }
  return parts.join(' ');
}

const manual: ManualTestSnapshot = {
  mode: 'wasm',
  status: 'WASM runtime active',
  roomLabel: '143: sampleroom01',
  tickLabel: 'Tick: 42',
  playerSummary: 'Player: x=12 y=34 hspeed=1 vspeed=0 object=player#1 alive=true grounded=true jumpActive=false hold=0 cut=false',
  inputSummary: 'Input: jumpKey=0x10 pressed=true justPressed=true justReleased=false keys=[Shift]',
  diagnosticsSummary: 'Diagnostics: 2 recent, latest event runtime-instance-created',
  frameBudgetSummary: 'Frame: 12.4ms ok skipped=0 commands=75',
  recentEvents: ['info runtime-instance-created object=bullet tick=3'],
  diagnostics: ['diag one', 'diag two'],
  performance: null,
  tickPhases: null,
};

describe('hud', () => {
  it('renders preserved runtime smoke ids as compact HUD cards', () => {
    const doc = new TestDocument();
    const root = doc.createElement('section');

    renderManualTestHud(doc as unknown as Document, root as unknown as HTMLElement, manual);

    expect(root.querySelector('#runtime-status')?.textContent).toBe('WASM runtime active');
    expect(root.querySelector('#runtime-room')?.textContent).toBe('Room: 143: sampleroom01');
    expect(root.querySelector('#runtime-tick')?.textContent).toBe('Tick: 42');
    expect(root.querySelector('#runtime-player')?.textContent).toContain('Player: x=12');
    expect(root.querySelector('#runtime-input')?.textContent).toContain('jumpKey=0x10');
    expect(root.querySelector('#runtime-diagnostics')?.textContent).toContain('Diagnostics: 2 recent');
    expect(root.querySelector('#runtime-performance')?.textContent).toContain('Frame: 12.4ms');
  });

  it('renders recent runtime events without requiring the diagnostics drawer', () => {
    const doc = new TestDocument();
    const root = doc.createElement('section');

    renderManualTestHud(doc as unknown as Document, root as unknown as HTMLElement, manual);

    expect(root.querySelector('#runtime-events')?.textContent).toContain('runtime-instance-created');
    expect(collectText(root)).not.toContain('diag one');
  });
});
