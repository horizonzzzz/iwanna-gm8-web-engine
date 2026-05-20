import { afterEach, describe, expect, it, vi } from 'vitest';
import { loadPackage } from './loadPackage';

afterEach(() => {
  vi.restoreAllMocks();
});

describe('loadPackage', () => {
  it('loads the runtime-v1 files from the manifest-defined package root', async () => {
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      const fixtures: Record<string, unknown> = {
        '/packages/sample/manifest.json': {
          format_version: 1,
          package_kind: 'runtime-v1',
          source_name: 'sample.exe',
          source_hash: 'abc123',
          engine_family: 'gm8',
          compatibility: 'partial',
          default_room_id: 0,
          room_count: 1,
          object_count: 1,
          script_block_count: 1,
          sprite_count: 1,
          background_count: 1,
          sound_count: 1,
          resource_index_path: 'resources/index.json',
          warnings: ['script-ir-partial']
        },
        '/packages/sample/rooms.json': [
          {
            id: 0,
            name: 'Room 1',
            width: 640,
            height: 480,
            speed: 30,
            persistent: false,
            backgrounds: [],
            views_enabled: false,
            views: [],
            instances: [],
            creation_block_id: null
          }
        ],
        '/packages/sample/objects.json': [
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
            events: []
          }
        ],
        '/packages/sample/scripts.ir.json': {
          format: 'iwm-script-ir-v1',
          blocks: []
        },
        '/packages/sample/analysis.json': {
          dlls: [],
          included_files: [],
          warnings: ['ok'],
          unsupported_features: []
        },
        '/packages/sample/resources/index.json': {
          sprites: [],
          backgrounds: [],
          sounds: []
        }
      };

      if (!(url in fixtures)) {
        return new Response('', { status: 404 });
      }

      return new Response(JSON.stringify(fixtures[url]), {
        status: 200,
        headers: { 'content-type': 'application/json' }
      });
    });

    vi.stubGlobal('fetch', fetchMock);

    const result = await loadPackage('/packages/sample');

    expect(result.manifest.package_kind).toBe('runtime-v1');
    expect(result.rooms).toHaveLength(1);
    expect(result.objects[0]?.name).toBe('Player');
    expect(result.scripts.format).toBe('iwm-script-ir-v1');
    expect(result.analysis.warnings).toEqual(['ok']);
    expect(result.resources.sprites).toEqual([]);
    expect(fetchMock).toHaveBeenCalledWith('/packages/sample/manifest.json');
    expect(fetchMock).toHaveBeenCalledWith('/packages/sample/resources/index.json');
  });
});
