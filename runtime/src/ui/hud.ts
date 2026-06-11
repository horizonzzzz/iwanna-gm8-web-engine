import type { ManualTestSnapshot } from './traceView';

type DocumentLike = Pick<Document, 'createElement'>;

function createHudCard(doc: DocumentLike, id: string, title: string, text: string): HTMLElement {
  const card = doc.createElement('section');
  card.className = 'hud-card';

  const label = doc.createElement('span');
  label.className = 'hud-card__label';
  label.textContent = title;

  const value = doc.createElement('p');
  value.className = 'hud-card__value';
  value.id = id;
  value.textContent = text;

  card.append(label, value);
  return card;
}

function formatEvents(events: string[]): string {
  return events.length > 0 ? `Events: ${events.join(' | ')}` : 'Events: none';
}

export function renderManualTestHud(
  doc: DocumentLike,
  root: HTMLElement,
  snapshot: ManualTestSnapshot
): void {
  root.className = 'runtime-hud';
  root.replaceChildren(
    createHudCard(doc, 'runtime-status', 'Status', snapshot.status),
    createHudCard(doc, 'runtime-room', 'Room', `Room: ${snapshot.roomLabel}`),
    createHudCard(doc, 'runtime-tick', 'Tick', snapshot.tickLabel),
    createHudCard(doc, 'runtime-player', 'Player', snapshot.playerSummary),
    createHudCard(doc, 'runtime-input', 'Input', snapshot.inputSummary),
    createHudCard(doc, 'runtime-events', 'Events', formatEvents(snapshot.recentEvents)),
    createHudCard(doc, 'runtime-diagnostics', 'Diagnostics', snapshot.diagnosticsSummary),
    createHudCard(doc, 'runtime-performance', 'Frame', snapshot.frameBudgetSummary)
  );
}
