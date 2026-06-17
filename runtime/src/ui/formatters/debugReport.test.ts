import { describe, expect, it } from 'vitest';
import { buildDebugReport } from './debugReport';
import type { WasmRuntimeBridgeSnapshot } from '../../runtime/wasmBridge';

const snapshot: WasmRuntimeBridgeSnapshot = {
  tick: 240,
  roomId: 143,
  roomName: 'sampleroom01',
  roomSpeed: 30,
  diagnostics: ['info runtime-instance-created object=bullet tick=3'],
  inputTrace: {
    jumpButtonKey: 0x10,
    jumpPressed: false,
    jumpJustPressed: false,
    jumpJustReleased: true,
    activeKeys: ['16', '39'],
  },
  tickPhases: {
    inputDiagNanos: 11_000,
    stepEventsNanos: 4_201_000,
    viewSyncNanos: 42_000,
    playerMovementNanos: 388_000,
    collisionEventsNanos: 1_102_000,
    alarmsNanos: 30_000,
    keyboardEventsNanos: 205_000,
    renderSubmitNanos: 2_134_000,
    totalNanos: 8_113_000,
  },
  player: {
    runtimeId: 17,
    instanceId: 17,
    objectId: 4,
    objectName: 'player',
    x: 123.5,
    y: 456,
    hspeed: 3.2,
    vspeed: -7.8,
    facingLeft: false,
    alive: true,
    jump: {
      grounded: false,
      active: true,
      holdFrames: 4,
      cutApplied: false,
    },
  },
};

describe('buildDebugReport', () => {
  it('formats the runtime report as stable plain text', () => {
    const report = buildDebugReport({
      mode: 'wasm',
      status: 'WASM runtime active',
      roomLabel: '143 sampleroom01',
      snapshot,
      performance: {
        inputMs: 0.1,
        tickMs: 8.3,
        snapshotMs: 1.4,
        frameMs: 2.2,
        runtimeMs: 12.0,
        renderMs: 2.8,
        totalMs: 25.0,
        commandCount: 128,
        skippedIntervals: 0,
      },
    });

    expect(report).toContain('Status: WASM runtime active');
    expect(report).toContain('Room: 143 sampleroom01');
    expect(report).toContain('Room Speed: 30 Hz');
    expect(report).toContain('Tick: 240');
    expect(report).toContain('Player:');
    expect(report).toContain('Performance:');
    expect(report).toContain('total=25.0ms budget=ok');
    expect(report).toContain('Tick Phases:');
    expect(report).toContain('Diagnostics:');
  });
});
