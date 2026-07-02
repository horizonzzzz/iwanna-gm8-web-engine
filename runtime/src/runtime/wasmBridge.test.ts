import { describe, expect, it, vi } from 'vitest';
import {
  createLocalStorageWasmFileHost,
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

  it('prefers binary combined step export when wasm provides it', async () => {
    const memory = {
      buffer: new ArrayBuffer(4096)
    };
    const pointer = 1024;
    const encoder = new TextEncoder();
    const bytes: number[] = [];
    const pushU8 = (value: number) => bytes.push(value & 0xff);
    const pushU16 = (value: number) => {
      pushU8(value);
      pushU8(value >> 8);
    };
    const pushU32 = (value: number) => {
      pushU8(value);
      pushU8(value >> 8);
      pushU8(value >> 16);
      pushU8(value >> 24);
    };
    const pushI32 = (value: number) => pushU32(value >>> 0);
    const pushU64 = (value: number) => {
      const buffer = new ArrayBuffer(8);
      new DataView(buffer).setBigUint64(0, BigInt(value), true);
      bytes.push(...new Uint8Array(buffer));
    };
    const pushF64 = (value: number) => {
      const buffer = new ArrayBuffer(8);
      new DataView(buffer).setFloat64(0, value, true);
      bytes.push(...new Uint8Array(buffer));
    };
    const pushString = (value: string) => {
      const encoded = encoder.encode(value);
      pushU32(encoded.byteLength);
      bytes.push(...encoded);
    };
    const pushOptionU32 = (value: number | null) => {
      pushU8(value == null ? 0 : 1);
      if (value != null) {
        pushU32(value);
      }
    };
    const pushOptionString = (value: string | null) => {
      pushU8(value == null ? 0 : 1);
      if (value != null) {
        pushString(value);
      }
    };
    const pushStringArray = (values: string[]) => {
      pushU32(values.length);
      for (const value of values) {
        pushString(value);
      }
    };

    pushU32(0x424d5749);
    pushU16(1);
    pushU16(2);
    pushString('ready');
    pushU64(1);
    pushOptionU32(0);
    pushOptionString('room0');
    pushOptionU32(60);
    pushU32(4);
    pushU8(0);
    pushU16(16);
    pushU8(1);
    pushU8(1);
    pushU8(0);
    pushStringArray(['0x10:p1jp1jr0']);
    for (const nanos of [1, 2, 3, 4, 5, 6, 7, 8, 36]) {
      pushU64(nanos);
    }
    pushStringArray(['runtime-idle:tick advanced']);
    pushU64(1);
    pushOptionU32(0);
    pushU32(320);
    pushU32(240);
    pushU32(7);
    pushU8(0);
    pushU8(1);
    pushU8(2);
    pushU8(3);
    pushU8(4);
    pushU8(1);
    pushU32(44);
    pushI32(-10);
    pushI32(20);
    pushU8(1);
    pushU8(0);
    pushU8(1);
    pushU8(0);
    pushU8(2);
    pushU32(55);
    pushI32(32);
    pushI32(64);
    pushU32(8);
    pushU32(16);
    pushU32(32);
    pushU32(32);
    pushF64(1.5);
    pushF64(0.5);
    pushU8(3);
    pushU32(7);
    pushU32(0);
    pushI32(96);
    pushI32(128);
    pushI32(16);
    pushI32(24);
    pushF64(1.0);
    pushF64(1.0);
    pushF64(0.5);
    pushF64(0.0);
    pushU8(4);
    pushI32(1);
    pushI32(2);
    pushU32(3);
    pushU32(4);
    pushU8(250);
    pushU8(251);
    pushU8(252);
    pushU8(253);
    pushU8(5);
    pushString('GAME OVER');
    pushI32(160);
    pushI32(88);
    pushU32(32);
    pushOptionString('font32');
    pushU8(1);
    pushU8(0);
    pushU8(232);
    pushU8(36);
    pushU8(48);
    pushU8(220);
    pushString('center');
    pushU8(6);

    const encodedStep = Uint8Array.from(bytes);
    let lastResultLength = encodedStep.byteLength;
    const capturedInput: Uint8Array[] = [];
    const writeStep = () => {
      new Uint8Array(memory.buffer).set(encodedStep, pointer);
      lastResultLength = encodedStep.byteLength;
    };

    const legacyStep = vi.fn(() => {
      throw new Error('legacy JSON step should not be used');
    });
    const binaryStep = vi.fn((inputPointer: number, inputLength: number) => {
      capturedInput.push(new Uint8Array(memory.buffer.slice(inputPointer, inputPointer + inputLength)));
      writeStep();
      return pointer;
    });

    const bridge = makeWasmRuntimeBridge({
      memory,
      iwm_alloc: () => 8,
      iwm_free: () => undefined,
      iwm_boot_json: () => pointer,
      iwm_set_input_json: () => pointer,
      iwm_step_json: legacyStep,
      iwm_step_buffer: binaryStep,
      iwm_tick: () => pointer,
      iwm_reset: () => pointer,
      iwm_select_room: () => pointer,
      iwm_snapshot_json: () => pointer,
      iwm_frame_json: () => pointer,
      iwm_diagnostics_json: () => pointer,
      iwm_last_result_len: () => lastResultLength,
    } as any);

    const result = await bridge.step?.({
      left: true,
      right: false,
      jump: true,
      jumpPressed: true,
      jumpReleased: false,
      restart: false,
      keysHeld: [0x10],
      keysPressed: [0x10],
      keysReleased: [],
    });

    expect(legacyStep).not.toHaveBeenCalled();
    expect(binaryStep).toHaveBeenCalledTimes(1);
    expect(capturedInput[0]?.[0]).toBe(0x49);
    expect(result?.snapshot.inputTrace.activeKeys).toEqual(['0x10:p1jp1jr0']);
    expect(result?.frame.commands.map((command) => command.kind)).toEqual([
      'clear',
      'drawBackground',
      'drawTile',
      'drawSprite',
      'fillRect',
      'drawText',
      'present',
    ]);
    expect(result?.frame.commands[3]).toMatchObject({ kind: 'drawSprite', spriteId: 7, alpha: 0.5 });
    expect(result?.frame.commands[5]).toMatchObject({ kind: 'drawText', text: 'GAME OVER', fontName: 'font32', align: 'center' });
  });
});

describe('wasm bridge file imports', () => {
  it('clears stale restart temp state on a fresh package boot while preserving saves', () => {
    const storage = new Map<string, string>();
    vi.stubGlobal('localStorage', {
      getItem: vi.fn((key: string) => storage.get(key) ?? null),
      setItem: vi.fn((key: string, value: string) => {
        storage.set(key, value);
      }),
      removeItem: vi.fn((key: string) => {
        storage.delete(key);
      }),
    });
    const host = createLocalStorageWasmFileHost('test-runtime-save');
    const pkg = makeRuntimePackage({ sourceHash: 'hash' });

    host.configurePackage?.(pkg, '/packages/sample');
    host.writeFile('temp', Uint8Array.of(1));
    host.writeFile('save1', Uint8Array.of(7, 8, 9));
    host.configurePackage?.(pkg, '/packages/sample');

    expect(host.readFile('temp')).toBeNull();
    expect([...(host.readFile('save1') ?? [])]).toEqual([7, 8, 9]);
  });

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
