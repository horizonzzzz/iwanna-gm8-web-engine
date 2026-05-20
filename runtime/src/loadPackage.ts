import type {
  ObjectDefinition,
  ResourceIndex,
  RoomDefinition,
  RuntimeAnalysis,
  RuntimeManifest,
  RuntimePackage,
  ScriptIrFile
} from './types';

async function readJson<T>(url: string): Promise<T> {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Failed to load ${url}: ${response.status}`);
  }

  return response.json() as Promise<T>;
}

export async function loadPackage(basePath: string): Promise<RuntimePackage> {
  const prefix = basePath.replace(/\/$/, '');
  const manifest = await readJson<RuntimeManifest>(`${prefix}/manifest.json`);
  const [rooms, objects, scripts, analysis, resources] = await Promise.all([
    readJson<RoomDefinition[]>(`${prefix}/rooms.json`),
    readJson<ObjectDefinition[]>(`${prefix}/objects.json`),
    readJson<ScriptIrFile>(`${prefix}/scripts.ir.json`),
    readJson<RuntimeAnalysis>(`${prefix}/analysis.json`),
    readJson<ResourceIndex>(`${prefix}/${manifest.resource_index_path}`)
  ]);

  return {
    manifest,
    rooms,
    objects,
    scripts,
    analysis,
    resources
  };
}
