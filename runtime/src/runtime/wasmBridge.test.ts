import { describe, expect, it } from 'vitest';
import {
  describeWasmBridgeAvailability,
  isWasmRuntimeBridge,
  makeWasmRuntimeBridge,
  makeWasmRuntimeHostImports,
  loadWasmRuntimeBridge,
  type WasmRuntimeBridge,
  type WasmFileHost,
} from './wasmBridge';
import { makeRuntimePackage, makeWasmFrame, makeWasmSnapshot } from '../test/packageFixtures';

function makeBridge(): WasmRuntimeBridge {
  return {
    backend: 'opengmk-wasm',
    boot: async () => makeWasmSnapshot({ roomId: 0 }),
    snapshot: async () => makeWasmSnapshot({ roomId: 0 }),
    frame: async () => makeWasmFrame({ roomId: 0 }),
    setInput: async () => makeWasmSnapshot({ roomId: 0 }),
    setGlobals: async () => makeWasmSnapshot({ roomId: 0 }),
    tick: async (frames = 1) => makeWasmSnapshot({ tick: frames, roomId: 0 }),
    reset: async () => makeWasmSnapshot({ roomId: 0 }),
    selectRoom: async (roomId: number) => makeWasmSnapshot({ roomId }),
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
    expect(describeWasmBridgeAvailability(null, new Error('module fetch failed'))).toContain('static room viewer');
    expect(describeWasmBridgeAvailability(null, null)).toContain('static room viewer');
  });

  it('wraps a low-level wasm exports object into the runtime bridge contract', async () => {
    const encodedSnapshot = new TextEncoder().encode(
      JSON.stringify(makeWasmSnapshot({
        tick: 3,
        roomId: 1,
        diagnostics: ['runtime-idle:tick advanced'],
        inputTrace: {
          jumpButtonKey: 16,
          jumpPressed: true,
          jumpJustPressed: true,
          jumpJustReleased: false,
          activeKeys: ['0x10:p1jp1jr0']
        },
        player: {
          x: 12,
          y: 34,
          hspeed: 0,
          vspeed: -8,
          facingLeft: false,
          jump: {
            grounded: false,
            active: true,
            holdFrames: 1,
            cutApplied: false
          }
        }
      }))
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
      iwm_set_input_json: () => {
        writeSnapshot();
        return snapshotPointer;
      },
      iwm_set_globals_json: () => {
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

    const boot = await bridge.boot(makeRuntimePackage({ roomId: 0, roomName: 'room0', width: 320, height: 240 }));

    expect(boot.tick).toBe(3);
    expect((await bridge.snapshot()).roomId).toBe(1);
    expect((await bridge.snapshot()).player?.jump?.holdFrames).toBe(1);
    expect((await bridge.tick(2)).roomId).toBe(1);
    expect((await bridge.setGlobals({ 'global.difficulty': 0 })).roomId).toBe(1);
    expect((await bridge.reset()).diagnostics[0]).toContain('runtime-idle');
    expect((await bridge.selectRoom(1)).tick).toBe(3);
    expect((await bridge.diagnostics())[0]).toContain('runtime-idle');
  });

  it('wraps input submission and frame snapshot exports', async () => {
    const encodedSnapshot = new TextEncoder().encode(
      JSON.stringify(makeWasmSnapshot({
        tick: 0,
        roomId: 0,
        player: null
      }))
    );
    const encodedFrame = new TextEncoder().encode(
      JSON.stringify(makeWasmFrame({
        tick: 1,
        roomId: 0,
        commands: [{ kind: 'present' }]
      }))
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
      iwm_set_globals_json: () => {
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

  it('wraps combined step export when wasm provides it', async () => {
    const encodedStep = new TextEncoder().encode(
      JSON.stringify({
        snapshot: makeWasmSnapshot({
          tick: 1,
          roomId: 0,
          player: null
        }),
        frame: makeWasmFrame({
          tick: 1,
          roomId: 0,
          commands: [{ kind: 'present' }]
        })
      })
    );

    const memory = {
      buffer: new ArrayBuffer(4096)
    };
    const pointer = 1024;
    let lastResultLength = encodedStep.byteLength;
    const writeStep = () => {
      new Uint8Array(memory.buffer).fill(0, pointer, pointer + encodedStep.byteLength + 1);
      new Uint8Array(memory.buffer).set(encodedStep, pointer);
      lastResultLength = encodedStep.byteLength;
    };

    const bridge = makeWasmRuntimeBridge({
      memory,
      iwm_alloc: () => 8,
      iwm_free: () => undefined,
      iwm_boot_json: () => pointer,
      iwm_set_input_json: () => pointer,
      iwm_set_globals_json: () => pointer,
      iwm_step_json: () => {
        writeStep();
        return pointer;
      },
      iwm_tick: () => pointer,
      iwm_reset: () => pointer,
      iwm_select_room: () => pointer,
      iwm_snapshot_json: () => pointer,
      iwm_frame_json: () => pointer,
      iwm_diagnostics_json: () => pointer,
      iwm_last_result_len: () => lastResultLength,
    });

    const result = await bridge.step?.({
      left: true,
      right: false,
      jump: false,
      jumpPressed: false,
      jumpReleased: false,
      restart: false,
    });

    expect(result?.snapshot.tick).toBe(1);
    expect(result?.frame.tick).toBe(1);
    expect(result?.frame.commands[0]?.kind).toBe('present');
  });
});

describe('wasm bridge file imports', () => {
  it('reads, writes, and removes package save bytes through the configured file host', () => {
    const files = new Map<string, Uint8Array>();
    const fileHost: WasmFileHost = {
      readFile: (path) => files.get(path),
      writeFile: (path, bytes) => {
        files.set(path, new Uint8Array(bytes));
      },
      removeFile: (path) => files.delete(path),
    };
    const imports = makeWasmRuntimeHostImports({ fileHost });
    const env = imports.env as Record<string, (...args: number[]) => number | void>;
    const memory = new WebAssembly.Memory({ initial: 1 });
    (env.__iwm_bind_memory as unknown as (memory: WebAssembly.Memory) => void)(memory);

    const bytes = new Uint8Array(memory.buffer);
    const encoder = new TextEncoder();
    const path = encoder.encode('save1');
    const payload = Uint8Array.of(7, 8, 9);
    bytes.set(path, 16);
    bytes.set(payload, 64);

    expect(env.iwm_host_write_file(16, path.byteLength, 64, payload.byteLength)).toBe(1);
    expect([...files.get('save1')!]).toEqual([7, 8, 9]);
    expect(env.iwm_host_read_file(16, path.byteLength, 0, 0)).toBe(3);
    expect(env.iwm_host_read_file(16, path.byteLength, 96, 3)).toBe(3);
    expect([...bytes.slice(96, 99)]).toEqual([7, 8, 9]);
    expect(env.iwm_host_remove_file(16, path.byteLength)).toBe(1);
    expect(env.iwm_host_read_file(16, path.byteLength, 96, 3)).toBe(-1);
  });
});
