import type {
  ObjectDefinition,
  ResourceIndex,
  RuntimeLoweredLogicFile,
  RoomDefinition,
  RuntimeAnalysis,
  RuntimeManifest,
  RuntimePackage,
  RuntimeRawLogicFile,
  ScriptIrFile
} from './types';

const EMPTY_RAW_LOGIC: RuntimeRawLogicFile = {
  format: 'iwm-raw-logic-v1',
  room_creation_codes: [],
  instance_creation_codes: [],
  object_events: [],
  scripts: [],
  triggers: [],
  timelines: []
};

const EMPTY_LOWERED_LOGIC: RuntimeLoweredLogicFile = {
  format: 'iwm-lowered-logic-v1',
  entries: []
};

type ReadJsonOptions<T> = {
  fallback?: T;
};

async function readJson<T>(url: string, options: ReadJsonOptions<T> = {}): Promise<T> {
  const response = await fetch(url);
  if (!response.ok) {
    if (options.fallback !== undefined) {
      return options.fallback;
    }
    throw new Error(`Failed to load ${url}: ${response.status}`);
  }

  const body = await response.text();
  const contentType = response.headers.get('content-type') ?? '';
  const trimmed = body.trimStart();
  const htmlFallback = trimmed.startsWith('<!doctype') || trimmed.startsWith('<html');

  if (options.fallback !== undefined && (htmlFallback || !contentType.toLowerCase().includes('json'))) {
    return options.fallback;
  }

  try {
    return JSON.parse(body) as T;
  } catch (error) {
    if (options.fallback !== undefined && htmlFallback) {
      return options.fallback;
    }
    throw error;
  }
}

export async function loadPackage(basePath: string): Promise<RuntimePackage> {
  const prefix = basePath.replace(/\/$/, '');
  const manifest = await readJson<RuntimeManifest>(`${prefix}/manifest.json`);
  const [rooms, objects, scripts, rawLogic, loweredLogic, analysis, resources] = await Promise.all([
    readJson<RoomDefinition[]>(`${prefix}/rooms.json`),
    readJson<ObjectDefinition[]>(`${prefix}/objects.json`),
    readJson<ScriptIrFile>(`${prefix}/scripts.ir.json`),
    readJson<RuntimeRawLogicFile>(`${prefix}/logic.raw.json`, { fallback: EMPTY_RAW_LOGIC }),
    readJson<RuntimeLoweredLogicFile>(`${prefix}/logic.lowered.json`, { fallback: EMPTY_LOWERED_LOGIC }),
    readJson<RuntimeAnalysis>(`${prefix}/analysis.json`),
    readJson<ResourceIndex>(`${prefix}/${manifest.resource_index_path}`)
  ]);

  return {
    manifest,
    rooms,
    objects,
    scripts,
    rawLogic,
    loweredLogic,
    analysis,
    resources
  };
}
