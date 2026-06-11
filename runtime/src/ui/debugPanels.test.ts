import { describe, expect, it } from 'vitest';
import {
  createPreBlock,
  formatPerformanceDetails,
  formatTickPhaseDetails,
  renderDebugPanels,
} from './debugPanels';

class TestElement {
  public readonly children: Array<TestElement | string> = [];
  public id = '';
  public className = '';
  public textContent = '';
  public open = false;

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

describe('debugPanels', () => {
  it('renders closed details by default while keeping detail ids queryable', () => {
    const doc = new TestDocument();
    const root = doc.createElement('section');
    const diagnostics = createPreBlock(doc as unknown as Document, 'runtime-diagnostics-detail', [
      'diag one',
      'diag two',
    ]);

    renderDebugPanels(doc as unknown as Document, root as unknown as HTMLElement, [
      {
        id: 'diagnostics-panel',
        title: 'Diagnostics',
        summary: '2 recent',
        content: diagnostics,
      },
    ]);

    expect(root.children).toHaveLength(1);
    expect((root.children[0] as TestElement).tagName).toBe('details');
    expect((root.children[0] as TestElement).open).toBe(false);
    expect(root.querySelector('#runtime-diagnostics-detail')?.textContent).toContain('diag one');
  });

  it('formats performance detail separately from the HUD frame summary', () => {
    expect(formatPerformanceDetails({
      inputMs: 1,
      tickMs: 2,
      snapshotMs: 3,
      frameMs: 4,
      runtimeMs: 10,
      renderMs: 5,
      totalMs: 15,
      commandCount: 12,
      skippedIntervals: 2,
    })).toBe('Frame ms: total=15.0 input=1.0 tick=2.0 snapshot=3.0 frame=4.0 render=5.0 runtime=10.0 commands=12 skipped=2');
  });

  it('formats runtime-core tick phases in milliseconds', () => {
    expect(formatTickPhaseDetails({
      inputDiagNanos: 100_000,
      stepEventsNanos: 2_500_000,
      viewSyncNanos: 50_000,
      playerMovementNanos: 700_000,
      collisionEventsNanos: 400_000,
      alarmsNanos: 30_000,
      keyboardEventsNanos: 20_000,
      renderSubmitNanos: 300_000,
      totalNanos: 4_100_000,
    })).toBe('Tick phases ms: total=4.100 inputDiag=0.100 step=2.500 view=0.050 player=0.700 collision=0.400 alarms=0.030 keyboard=0.020 renderSubmit=0.300');
  });
});
