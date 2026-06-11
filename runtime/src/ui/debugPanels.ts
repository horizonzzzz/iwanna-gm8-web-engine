import type { RuntimePerformanceStats } from './traceView';
import type { WasmRuntimeTickPhases } from '../runtime/wasmBridge';

type DocumentLike = Pick<Document, 'createElement'>;

export type DebugPanel = {
  id: string;
  title: string;
  summary: string;
  content: HTMLElement;
  open?: boolean;
};

export function createPreBlock(doc: DocumentLike, id: string, value: unknown): HTMLElement {
  const pre = doc.createElement('pre');
  pre.id = id;
  pre.textContent = typeof value === 'string' ? value : JSON.stringify(value, null, 2);
  return pre;
}

export function formatPerformanceDetails(performance: RuntimePerformanceStats | null): string {
  if (!performance) {
    return 'Frame ms: unavailable';
  }

  return [
    `Frame ms: total=${performance.totalMs.toFixed(1)}`,
    `input=${performance.inputMs.toFixed(1)}`,
    `tick=${performance.tickMs.toFixed(1)}`,
    `snapshot=${performance.snapshotMs.toFixed(1)}`,
    `frame=${performance.frameMs.toFixed(1)}`,
    `render=${performance.renderMs.toFixed(1)}`,
    `runtime=${performance.runtimeMs.toFixed(1)}`,
    `commands=${performance.commandCount}`,
    `skipped=${performance.skippedIntervals}`,
  ].join(' ');
}

function formatNanosAsMs(nanos: number): string {
  return (nanos / 1_000_000).toFixed(3);
}

export function formatTickPhaseDetails(phases: WasmRuntimeTickPhases | null): string {
  if (!phases) {
    return 'Tick phases ms: unavailable';
  }

  return [
    `Tick phases ms: total=${formatNanosAsMs(phases.totalNanos)}`,
    `inputDiag=${formatNanosAsMs(phases.inputDiagNanos)}`,
    `step=${formatNanosAsMs(phases.stepEventsNanos)}`,
    `view=${formatNanosAsMs(phases.viewSyncNanos)}`,
    `player=${formatNanosAsMs(phases.playerMovementNanos)}`,
    `collision=${formatNanosAsMs(phases.collisionEventsNanos)}`,
    `alarms=${formatNanosAsMs(phases.alarmsNanos)}`,
    `keyboard=${formatNanosAsMs(phases.keyboardEventsNanos)}`,
    `renderSubmit=${formatNanosAsMs(phases.renderSubmitNanos)}`,
  ].join(' ');
}

export function renderDebugPanels(doc: DocumentLike, root: HTMLElement, panels: DebugPanel[]): void {
  root.className = 'debug-panels';
  root.replaceChildren(...panels.map((panel) => {
    const details = doc.createElement('details') as HTMLDetailsElement;
    details.id = panel.id;
    details.className = 'debug-panel';
    details.open = panel.open ?? false;

    const summary = doc.createElement('summary');
    summary.textContent = `${panel.title}: ${panel.summary}`;

    details.append(summary, panel.content);
    return details;
  }));
}
