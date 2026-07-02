import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { expect, type Page } from '@playwright/test';

export type RuntimeScenarioTick = {
  tick: number;
  press_keys?: number[];
  hold_keys?: number[];
  release_keys?: number[];
};

export type RuntimeScenario = {
  ticks: RuntimeScenarioTick[];
};

export type WasmScenarioTraceSample = {
  tick: number;
  roomId: number | null;
  roomName?: string | null;
  player: {
    objectName?: string;
    x: number;
    y: number;
    hspeed: number;
    vspeed: number;
    alive?: boolean;
    jump: {
      grounded: boolean;
      active: boolean;
      holdFrames: number;
      cutApplied: boolean;
    };
  } | null;
};

export type WasmScenarioPerformanceSample = {
  tick: number;
  commandCount: number;
  tickPhases?: {
    inputDiagNanos: number;
    stepEventsNanos: number;
    viewSyncNanos: number;
    playerMovementNanos: number;
    collisionEventsNanos: number;
    alarmsNanos: number;
    keyboardEventsNanos: number;
    renderSubmitNanos: number;
    totalNanos: number;
  };
};

export type WasmScenarioResult = {
  diagnostics: string[];
  finalTick: number;
  finalRoomId: number | null;
  finalFrameCommandCount: number;
  performance: WasmScenarioPerformanceSample[];
  trace: WasmScenarioTraceSample[];
};

export type PlayerTraceSummary = {
  sampleCount: number;
  minX: number;
  maxX: number;
  minY: number;
  maxY: number;
  maxAbsHspeed: number;
  maxAbsVspeed: number;
};

export function readRuntimeScenario(fileName: string): RuntimeScenario {
  const fullPath = resolve(process.cwd(), '..', 'docs', 'notes', 'runtime-scenarios', fileName);
  return JSON.parse(readFileSync(fullPath, 'utf8')) as RuntimeScenario;
}

export async function runWasmScenario(
  page: Page,
  options: {
    scenario: RuntimeScenario;
    roomId: number;
    ticks: number;
    preselectTicks?: number;
    traceEvery?: number;
    performanceEvery?: number;
  }
): Promise<WasmScenarioResult> {
  await page.goto('/');
  return page.evaluate(async ({ scenario, roomId, ticks, preselectTicks = 0, traceEvery = 0, performanceEvery = 0 }) => {
    const { loadPackage } = await import('/src/loadPackage.ts');
    const { instantiateWasmRuntimeBridge } = await import('/src/runtime/wasmBridge.ts');

    const pkg = await loadPackage('/packages/sample');
    const bridge = await instantiateWasmRuntimeBridge('/wasm/iwm_runtime_web.wasm', {}, {
      audioHost: {
        playSound: () => undefined,
        stopSound: () => undefined,
        stopAllSounds: () => undefined,
        isSoundPlaying: () => false,
      },
    });

    await bridge.boot(pkg);
    const diagnostics = new Set<string>(await bridge.diagnostics());
    for (let tick = 0; tick < preselectTicks; tick += 1) {
      await bridge.tick(1);
      for (const diagnostic of await bridge.diagnostics()) {
        diagnostics.add(diagnostic);
      }
    }
    await bridge.selectRoom(roomId);
    for (const diagnostic of await bridge.diagnostics()) {
      diagnostics.add(diagnostic);
    }

    const scriptTicks = new Map<number, RuntimeScenarioTick>();
    for (const row of scenario.ticks) {
      const existing = scriptTicks.get(row.tick) ?? { tick: row.tick };
      existing.press_keys = [...(existing.press_keys ?? []), ...(row.press_keys ?? [])];
      existing.hold_keys = [...(existing.hold_keys ?? []), ...(row.hold_keys ?? [])];
      existing.release_keys = [...(existing.release_keys ?? []), ...(row.release_keys ?? [])];
      scriptTicks.set(row.tick, existing);
    }

    const heldKeys = new Set<number>();
    const trace: WasmScenarioTraceSample[] = [];
    const performance: WasmScenarioPerformanceSample[] = [];
    let lastSnapshot = await bridge.snapshot();
    let lastFrame = await bridge.frame();

    for (let runTick = 0; runTick < ticks; runTick += 1) {
      const row = scriptTicks.get(runTick);
      const previousHeldKeys = new Set(heldKeys);

      for (const key of row?.release_keys ?? []) {
        heldKeys.delete(key);
      }
      for (const key of row?.hold_keys ?? []) {
        heldKeys.add(key);
      }

      const newlyHeldKeys = (row?.hold_keys ?? []).filter((key) => !previousHeldKeys.has(key));
      const pressedKeys = [...(row?.press_keys ?? []), ...newlyHeldKeys];
      const releasedKeys = row?.release_keys ?? [];
      const activeKeys = new Set([...heldKeys, ...(row?.press_keys ?? [])]);

      await bridge.setInput({
        left: false,
        right: false,
        jump: false,
        jumpPressed: false,
        jumpReleased: false,
        restart: false,
        keysHeld: [...activeKeys],
        keysPressed: pressedKeys,
        keysReleased: releasedKeys,
      });
      lastSnapshot = await bridge.tick(1);
      lastFrame = await bridge.frame();
      for (const diagnostic of await bridge.diagnostics()) {
        diagnostics.add(diagnostic);
      }

      if (traceEvery > 0 && lastSnapshot.tick % traceEvery === 0) {
        trace.push({
          tick: lastSnapshot.tick,
          roomId: lastSnapshot.roomId,
          roomName: lastSnapshot.roomName,
          player: lastSnapshot.player
            ? {
                objectName: lastSnapshot.player.objectName,
                x: lastSnapshot.player.x,
                y: lastSnapshot.player.y,
                hspeed: lastSnapshot.player.hspeed,
                vspeed: lastSnapshot.player.vspeed,
                alive: lastSnapshot.player.alive,
                jump: lastSnapshot.player.jump,
              }
            : null,
        });
      }

      if (performanceEvery > 0 && lastSnapshot.tick % performanceEvery === 0) {
        performance.push({
          tick: lastSnapshot.tick,
          commandCount: lastFrame.commands.length,
          tickPhases: lastSnapshot.tickPhases,
        });
      }
    }

    return {
      diagnostics: [...diagnostics],
      finalTick: lastSnapshot.tick,
      finalRoomId: lastSnapshot.roomId,
      finalFrameCommandCount: lastFrame.commands.length,
      performance,
      trace,
    };
  }, options);
}

export function expectNoRuntimeBlockers(diagnostics: string[]): void {
  expect(diagnostics.filter((item) => item.includes('runtime-unsupported-'))).toEqual([]);
}

export function hasDiagnostic(diagnostics: string[], ...patterns: string[]): boolean {
  return diagnostics.some((item) => patterns.every((pattern) => item.includes(pattern)));
}

export function summarizePlayerTrace(trace: WasmScenarioTraceSample[]): PlayerTraceSummary {
  const playerSamples = trace.flatMap((sample) => sample.player ? [sample.player] : []);
  expect(playerSamples.length).toBeGreaterThan(0);

  return {
    sampleCount: playerSamples.length,
    minX: Math.min(...playerSamples.map((player) => player.x)),
    maxX: Math.max(...playerSamples.map((player) => player.x)),
    minY: Math.min(...playerSamples.map((player) => player.y)),
    maxY: Math.max(...playerSamples.map((player) => player.y)),
    maxAbsHspeed: Math.max(...playerSamples.map((player) => Math.abs(player.hspeed))),
    maxAbsVspeed: Math.max(...playerSamples.map((player) => Math.abs(player.vspeed))),
  };
}

export function expectFinitePhaseTimings(sample: WasmScenarioPerformanceSample): void {
  expect(sample.commandCount).toBeGreaterThan(0);
  expect(sample.tickPhases).toBeTruthy();
  expect(Number.isFinite(sample.tickPhases?.inputDiagNanos)).toBe(true);
  expect(Number.isFinite(sample.tickPhases?.stepEventsNanos)).toBe(true);
  expect(Number.isFinite(sample.tickPhases?.collisionEventsNanos)).toBe(true);
  expect(Number.isFinite(sample.tickPhases?.renderSubmitNanos)).toBe(true);
  expect(Number.isFinite(sample.tickPhases?.totalNanos)).toBe(true);
}
