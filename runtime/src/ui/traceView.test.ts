import { describe, expect, it } from 'vitest';
import {
  buildManualTestSnapshot,
  extractRuntimeEvents,
  formatFrameBudgetSummary,
  formatRuntimePlayer,
} from './traceView';
import type { WasmRuntimeBridgeSnapshot } from '../runtime/wasmBridge';

const snapshot: WasmRuntimeBridgeSnapshot = {
  tick: 42,
  roomId: 143,
  roomName: 'sampleroom01',
  instanceCount: 128,
  diagnostics: [
    'info runtime-execution-trace block=boot',
    'info runtime-instance-created object=bullet tick=3',
    'warning runtime-unsupported-function function=file_bin_open block=save',
    'info runtime-player-died room=143 tick=41',
  ],
  inputTrace: {
    jumpButtonKey: 0x10,
    jumpPressed: true,
    jumpJustPressed: false,
    jumpJustReleased: true,
    activeKeys: ['Shift', 'Z'],
  },
  tickPhases: {
    inputDiagNanos: 100_000,
    stepEventsNanos: 2_500_000,
    viewSyncNanos: 50_000,
    playerMovementNanos: 700_000,
    collisionEventsNanos: 400_000,
    alarmsNanos: 30_000,
    keyboardEventsNanos: 20_000,
    renderSubmitNanos: 300_000,
    totalNanos: 4_100_000,
  },
  player: {
    runtimeId: 9001,
    instanceId: 17,
    objectId: 4,
    objectName: 'player',
    x: 12.5,
    y: 34,
    hspeed: 1.25,
    vspeed: -8.5,
    facingLeft: false,
    alive: true,
    jump: {
      grounded: true,
      active: false,
      holdFrames: 0,
      cutApplied: false,
    },
  },
};

describe('traceView', () => {
  it('formats the current player as a compact hand-test row', () => {
    expect(formatRuntimePlayer(snapshot)).toBe(
      'Player: x=12.5 y=34 hspeed=1.25 vspeed=-8.5 object=player#9001 alive=true grounded=true jumpActive=false hold=0 cut=false'
    );
  });

  it('formats missing player state without hiding runtime availability', () => {
    expect(formatRuntimePlayer({ ...snapshot, player: null })).toBe('Player: unavailable');
  });

  it('extracts only high-value runtime events from diagnostics', () => {
    expect(extractRuntimeEvents(snapshot.diagnostics)).toEqual([
      'info runtime-instance-created object=bullet tick=3',
      'info runtime-player-died room=143 tick=41',
    ]);
  });

  it('summarizes frame budget without exposing profiler detail by default', () => {
    expect(formatFrameBudgetSummary({
      inputMs: 1.2,
      tickMs: 3.4,
      snapshotMs: 0.8,
      frameMs: 1.1,
      runtimeMs: 6.5,
      renderMs: 4.2,
      totalMs: 12.4,
      commandCount: 75,
      skippedIntervals: 0,
    })).toBe('Frame: 12.4ms ok skipped=0 commands=75');
  });

  it('builds a manual testing snapshot from existing bridge data', () => {
    const manual = buildManualTestSnapshot({
      mode: 'wasm',
      status: 'WASM runtime active',
      roomLabel: '143: sampleroom01',
      snapshot,
      performance: {
        inputMs: 1,
        tickMs: 2,
        snapshotMs: 3,
        frameMs: 4,
        runtimeMs: 10,
        renderMs: 5,
        totalMs: 15,
        commandCount: 12,
        skippedIntervals: 2,
      },
    });

    expect(manual.status).toBe('WASM runtime active');
    expect(manual.roomLabel).toBe('143: sampleroom01');
    expect(manual.tickLabel).toBe('Tick: 42');
    expect(manual.playerSummary).toContain('Player: x=12.5');
    expect(manual.inputSummary).toBe('Input: jumpKey=0x10 pressed=true justPressed=false justReleased=true keys=[Shift,Z]');
    expect(manual.diagnosticsSummary).toBe('Diagnostics: 4 recent, latest event runtime-player-died');
    expect(manual.frameBudgetSummary).toBe('Frame: 15.0ms ok skipped=2 commands=12');
    expect(manual.recentEvents).toHaveLength(2);
  });
});
