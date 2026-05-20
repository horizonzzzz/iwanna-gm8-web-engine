import { describe, expect, it } from 'vitest';
import {
  describeWasmBridgeAvailability,
  isWasmRuntimeBridge,
  makeWasmRuntimeBridge,
  loadWasmRuntimeBridge,
  type WasmRuntimeBridge,
} from './wasmBridge';

function makeBridge(): WasmRuntimeBridge {
  return {
    backend: 'opengmk-wasm',
    boot: async () => ({ tick: 0, roomId: 0, diagnostics: [] }),
    snapshot: async () => ({ tick: 0, roomId: 0, diagnostics: [] }),
    frame: async () => ({ tick: 0, roomId: 0, width: 320, height: 240, commands: [{ kind: 'present' as const }] }),
    setInput: async () => ({ tick: 0, roomId: 0, diagnostics: [] }),
    tick: async (frames = 1) => ({ tick: frames, roomId: 0, diagnostics: [] }),
    reset: async () => ({ tick: 0, roomId: 0, diagnostics: [] }),
    selectRoom: async (roomId: number) => ({ tick: 0, roomId, diagnostics: [] }),
    diagnostics: async () => [],
  };
}

describe('wasm bridge loader', () => {
  it('accepts a valid bridge module', async () => {
    const bridge = await loadWasmRuntimeBridge(async () => ({
      initRuntimeHost: async () => makeBridge(),
    }));

    expect(isWasmRuntimeBridge(bridge)).toBe(true);
    expect(bridge.backend).toBe('opengmk-wasm');
  });

  it('rejects modules without the expected initializer', async () => {
    await expect(loadWasmRuntimeBridge(async () => ({}))).rejects.toThrow(
      'WASM bridge module is missing initRuntimeHost()'
    );
  });

  it('describes configured and missing bridge states clearly', () => {
    expect(describeWasmBridgeAvailability(makeBridge(), null)).toContain('WASM bridge available');
    expect(describeWasmBridgeAvailability(null, new Error('module fetch failed'))).toContain(
      'module fetch failed'
    );
    expect(describeWasmBridgeAvailability(null, null)).toContain('No WASM bridge configured');
  });

  it('wraps a low-level wasm exports object into the runtime bridge contract', async () => {
    const encodedSnapshot = new TextEncoder().encode(
      JSON.stringify({
        tick: 3,
        roomId: 1,
        diagnostics: ['runtime-idle:tick advanced']
      })
    );
    const encodedDiagnostics = new TextEncoder().encode(
      JSON.stringify(['runtime-idle:tick advanced'])
    );

    const memory = {
      buffer: new ArrayBuffer(4096)
    };
    const snapshotPointer = 2048;
    const diagnosticsPointer = 3072;
    let lastResultLength = encodedSnapshot.byteLength;
    const writeSnapshot = () => {
      new Uint8Array(memory.buffer).fill(0, snapshotPointer, snapshotPointer + encodedSnapshot.byteLength + 1);
      new Uint8Array(memory.buffer).set(encodedSnapshot, snapshotPointer);
      lastResultLength = encodedSnapshot.byteLength;
    };
    const writeDiagnostics = () => {
      new Uint8Array(memory.buffer).fill(0, diagnosticsPointer, diagnosticsPointer + encodedDiagnostics.byteLength + 1);
      new Uint8Array(memory.buffer).set(encodedDiagnostics, diagnosticsPointer);
      lastResultLength = encodedDiagnostics.byteLength;
    };

    const bridge = makeWasmRuntimeBridge({
      memory,
      iwm_alloc: (size) => {
        expect(size).toBeGreaterThan(0);
        return 8;
      },
      iwm_free: () => undefined,
      iwm_boot_json: () => {
        writeSnapshot();
        return snapshotPointer;
      },
      iwm_tick: () => {
        writeSnapshot();
        return snapshotPointer;
      },
      iwm_reset: () => {
        writeSnapshot();
        return snapshotPointer;
      },
      iwm_select_room: (roomId) => {
        expect(roomId).toBe(1);
        writeSnapshot();
        return snapshotPointer;
      },
      iwm_snapshot_json: () => {
        writeSnapshot();
        return snapshotPointer;
      },
      iwm_diagnostics_json: () => {
        writeDiagnostics();
        return diagnosticsPointer;
      },
      iwm_last_result_len: () => lastResultLength,
    });

    const boot = await bridge.boot({
      manifest: {
        format_version: 1,
        package_kind: 'runtime-v1',
        source_name: 'sample.exe',
        source_hash: 'abc123',
        engine_family: 'gm8',
        compatibility: 'partial',
        default_room_id: 0,
        room_count: 1,
        object_count: 0,
        script_block_count: 0,
        sprite_count: 0,
        background_count: 0,
        sound_count: 0,
        resource_index_path: 'resources/index.json',
        warnings: []
      },
      rooms: [],
      objects: [],
      scripts: {
        format: 'iwm-script-ir-v1',
        blocks: []
      },
      resources: {
        sprites: [],
        backgrounds: [],
        sounds: []
      },
      analysis: {
        dlls: [],
        included_files: [],
        warnings: [],
        unsupported_features: []
      }
    });

    expect(boot.tick).toBe(3);
    expect((await bridge.snapshot()).roomId).toBe(1);
    expect((await bridge.tick(2)).roomId).toBe(1);
    expect((await bridge.reset()).diagnostics[0]).toContain('runtime-idle');
    expect((await bridge.selectRoom(1)).tick).toBe(3);
    expect((await bridge.diagnostics())[0]).toContain('runtime-idle');
  });

  it('wraps input submission and frame snapshot exports', async () => {
    const encodedSnapshot = new TextEncoder().encode(
      JSON.stringify({
        tick: 0,
        roomId: 0,
        diagnostics: []
      })
    );
    const encodedFrame = new TextEncoder().encode(
      JSON.stringify({
        tick: 1,
        roomId: 0,
        width: 320,
        height: 240,
        commands: [{ kind: 'present' }]
      })
    );

    const memory = {
      buffer: new ArrayBuffer(4096)
    };
    const snapshotPointer = 1024;
    const framePointer = 2048;
    let lastResultLength = encodedSnapshot.byteLength;

    const writeSnapshot = () => {
      new Uint8Array(memory.buffer).fill(0, snapshotPointer, snapshotPointer + encodedSnapshot.byteLength + 1);
      new Uint8Array(memory.buffer).set(encodedSnapshot, snapshotPointer);
      lastResultLength = encodedSnapshot.byteLength;
    };

    const writeFrame = () => {
      new Uint8Array(memory.buffer).fill(0, framePointer, framePointer + encodedFrame.byteLength + 1);
      new Uint8Array(memory.buffer).set(encodedFrame, framePointer);
      lastResultLength = encodedFrame.byteLength;
    };

    const bridge = makeWasmRuntimeBridge({
      memory,
      iwm_alloc: () => 8,
      iwm_free: () => undefined,
      iwm_boot_json: () => {
        writeSnapshot();
        return snapshotPointer;
      },
      iwm_tick: () => {
        writeSnapshot();
        return snapshotPointer;
      },
      iwm_reset: () => {
        writeSnapshot();
        return snapshotPointer;
      },
      iwm_select_room: () => {
        writeSnapshot();
        return snapshotPointer;
      },
      iwm_snapshot_json: () => {
        writeSnapshot();
        return snapshotPointer;
      },
      iwm_diagnostics_json: () => {
        writeSnapshot();
        return snapshotPointer;
      },
      iwm_set_input_json: () => {
        writeSnapshot();
        return snapshotPointer;
      },
      iwm_frame_json: () => {
        writeFrame();
        return framePointer;
      },
      iwm_last_result_len: () => lastResultLength,
    });

    await bridge.setInput({
      left: true,
      right: false,
      jump: true,
      jumpPressed: true,
      jumpReleased: false,
      restart: false,
    });

    expect((await bridge.frame()).tick).toBe(1);
  });
});
