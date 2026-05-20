import type { ObjectDefinition, RoomDefinition, RuntimeAnalysis, RuntimeManifest, ScriptIrFile } from '../types';

type DocumentLike = Pick<Document, 'createElement'>;

function renderKeyValue(doc: DocumentLike, title: string, entries: Record<string, string>): HTMLElement {
  const section = doc.createElement('section');
  section.className = 'inspector';
  const heading = doc.createElement('h3');
  heading.textContent = title;
  section.append(heading);
  const list = doc.createElement('dl');
  for (const [key, value] of Object.entries(entries)) {
    const term = doc.createElement('dt');
    term.textContent = key;
    const description = doc.createElement('dd');
    description.textContent = value;
    list.append(term, description);
  }
  section.append(list);
  return section;
}

export function renderManifestSummary(
  doc: DocumentLike,
  manifest: RuntimeManifest,
  analysis: RuntimeAnalysis
): HTMLElement {
  return renderKeyValue(doc, 'Package', {
    source: manifest.source_name,
    engine: manifest.engine_family,
    compatibility: manifest.compatibility,
    rooms: String(manifest.room_count),
    objects: String(manifest.object_count),
    script_blocks: String(manifest.script_block_count),
    warnings: analysis.warnings.join(', ') || 'none'
  });
}

function renderJsonSlice(doc: DocumentLike, title: string, value: unknown): HTMLElement {
  const section = doc.createElement('section');
  section.className = 'inspector';
  const heading = doc.createElement('h3');
  heading.textContent = title;
  const pre = doc.createElement('pre');
  pre.textContent = JSON.stringify(value, null, 2);
  section.append(heading, pre);
  return section;
}

export function renderRoomsSlice(doc: DocumentLike, rooms: RoomDefinition[]): HTMLElement {
  return renderJsonSlice(doc, 'Rooms', rooms.slice(0, 3));
}

export function renderObjectsSlice(doc: DocumentLike, objects: ObjectDefinition[]): HTMLElement {
  return renderJsonSlice(doc, 'Objects', objects.slice(0, 5));
}

export function renderScriptsSlice(doc: DocumentLike, scripts: ScriptIrFile): HTMLElement {
  return renderJsonSlice(doc, 'Script IR', scripts.blocks.slice(0, 5));
}
