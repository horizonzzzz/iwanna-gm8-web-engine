import { expect, test } from '@playwright/test';
import {
  expectFinitePhaseTimings,
  expectNoRuntimeBlockers,
  hasDiagnostic,
  readRuntimeScenario,
  runWasmScenario,
  summarizePlayerTrace,
} from './wasmScenario';

async function loadPackage(page: import('@playwright/test').Page, packagePath: string): Promise<void> {
  await page.goto('/');
  await page.locator('input[name="packagePath"]').fill(packagePath);
  await page.getByRole('button', { name: 'Load Package' }).click();
}

test('sample package boots through the wasm path and exposes runtime telemetry', async ({ page }) => {
  await loadPackage(page, '/packages/sample');

  await expect(page.locator('#runtime-status')).toContainText('WASM runtime active');
  await expect(page.locator('#runtime-room')).toContainText(/Room:/);
  await expect(page.locator('#runtime-player')).toContainText(/Player:/);
  await expect.poll(async () => {
    const text = await page.locator('#runtime-tick').textContent();
    return Number(text?.replace('Tick: ', '') ?? '0');
  }).toBeGreaterThan(0);
  await expect(page.locator('#runtime-diagnostics')).toContainText('Diagnostics:');
  await expect(page.locator('#runtime-input')).toContainText('Input:');
  await expect(page.locator('#runtime-performance')).toContainText('Frame:');
  await expect(page.getByText('Debug Report')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Copy' })).toBeVisible();

  const report = page.locator('h2', { hasText: 'Debug Report' }).locator('..').locator('..').locator('pre').first();
  await expect(report).toContainText('Status:');
  await expect(report).toContainText('Performance:');
  await expect(report).toContainText('Tick Phases:');
  await expect(report).toContainText('Diagnostics:');
});

test('sample package can switch to rStage01 and keep wasm telemetry visible', async ({ page }) => {
  await loadPackage(page, '/packages/sample');

  await expect(page.locator('select[name="roomSelect"]')).toBeVisible();
  await page.locator('select[name="roomSelect"]').selectOption({ index: 0 });
  await expect(page.locator('#runtime-room')).toContainText(/Room:/);
  await expect(page.locator('#runtime-status')).toContainText('WASM runtime active');
  await expect(page.locator('#runtime-player')).toContainText(/Player:/);
  await expect.poll(async () => {
    const text = await page.locator('#runtime-tick').textContent();
    return Number(text?.replace('Tick: ', '') ?? '0');
  }).toBeGreaterThan(0);
  await expect(page.locator('#runtime-performance')).toContainText('Frame:');
});

test('sample package pause button stops and resumes automatic ticking', async ({ page }) => {
  await loadPackage(page, '/packages/sample');

  const tickLocator = page.locator('#runtime-tick');
  const pauseButton = page.getByRole('button', { name: 'Pause' });

  await expect.poll(async () => {
    const text = await tickLocator.textContent();
    return Number(text?.replace('Tick: ', '') ?? '0');
  }).toBeGreaterThan(0);

  await expect(pauseButton).toBeVisible();
  await pauseButton.click();
  await expect(page.getByRole('button', { name: 'Resume' })).toBeVisible();

  const pausedBaseline = Number((await tickLocator.textContent())?.replace('Tick: ', '') ?? '0');
  await page.waitForTimeout(120);
  const pausedTick = Number((await tickLocator.textContent())?.replace('Tick: ', '') ?? '0');
  expect(pausedTick).toBe(pausedBaseline);

  await page.getByRole('button', { name: 'Resume' }).click();
  await expect(page.getByRole('button', { name: 'Pause' })).toBeVisible();
  await expect.poll(async () => {
    const text = await tickLocator.textContent();
    return Number(text?.replace('Tick: ', '') ?? '0');
  }).toBeGreaterThan(pausedTick);
});

test('sample package creates and renders a bullet from the browser wasm bridge on Z press', async ({ page }) => {
  await page.goto('/');

  const result = await page.evaluate(async () => {
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
    for (let tick = 0; tick < 180; tick += 1) {
      const snapshot = await bridge.snapshot();
      if (snapshot.inputTrace.jumpButtonKey === 0x10) {
        break;
      }
      await bridge.tick(1);
    }

    await bridge.selectRoom(147);
    const beforeFrame = await bridge.frame();
    await bridge.setInput({
      left: false,
      right: false,
      jump: false,
      jumpPressed: false,
      jumpReleased: false,
      restart: false,
      keysHeld: [0x5A],
      keysPressed: [0x5A],
      keysReleased: [],
    });
    const beforeShot = await bridge.snapshot();
    const afterShot = await bridge.tick(1);
    const afterFrame = await bridge.frame();

    return {
      activeKeys: afterShot.inputTrace.activeKeys,
      beforeInstanceCount: beforeShot.instanceCount,
      afterInstanceCount: afterShot.instanceCount,
      beforeCommandCount: beforeFrame.commands.length,
      afterCommandCount: afterFrame.commands.length,
      creationEvent: afterShot.diagnostics.find(
        (item) => item.includes('runtime-instance-created') && item.includes('object=bullet')
      ) ?? null,
    };
  });

  expect(result.activeKeys).toContain('0x5a:p1jp1jr0');
  expect(result.creationEvent).toContain('object=bullet');
  expect(result.afterInstanceCount).toBe(result.beforeInstanceCount + 1);
  expect(result.afterCommandCount).toBeGreaterThan(result.beforeCommandCount);
});

test('sample package ignores raw R save-load input on the difficulty room', async ({ page }) => {
  await page.goto('/');

  const result = await page.evaluate(async () => {
    const { loadPackage } = await import('/src/loadPackage.ts');
    const { instantiateWasmRuntimeBridge } = await import('/src/runtime/wasmBridge.ts');

    function saveBytes(roomId: number, x: number, y: number): Uint8Array {
      const bytes: number[] = [];
      for (const value of [roomId, x, y]) {
        bytes.push(Math.trunc(value / 10000));
        bytes.push(Math.trunc((value % 10000) / 100));
        bytes.push(value % 100);
      }
      bytes.push(0);
      bytes.push(0);
      bytes.push(...Array.from({ length: 16 }, () => 0));
      bytes.push(0);
      bytes.push(...Array.from({ length: 8 }, () => 0));
      bytes.push(0);
      bytes.push(0);
      return Uint8Array.from(bytes);
    }

    const files = new Map<string, Uint8Array>();
    const pkg = await loadPackage('/packages/sample');
    const difficultyRoom = pkg.rooms.find((room) => room.name === 'rSelectStage')?.id;
    if (difficultyRoom == null) {
      throw new Error('sample package is missing rSelectStage');
    }

    const bridge = await instantiateWasmRuntimeBridge('/wasm/iwm_runtime_web.wasm', {}, {
      audioHost: {
        playSound: () => undefined,
        stopSound: () => undefined,
        stopAllSounds: () => undefined,
        isSoundPlaying: () => false,
      },
      fileHost: {
        readFile: (path) => files.get(path) ?? null,
        writeFile: (path, bytes) => {
          files.set(path, new Uint8Array(bytes));
        },
        removeFile: (path) => files.delete(path),
      },
    });

    await bridge.boot(pkg);
    files.set('save1', saveBytes(143, 321, 654));
    await bridge.selectRoom(difficultyRoom);
    const before = await bridge.snapshot();

    await bridge.setInput({
      left: false,
      right: false,
      jump: false,
      jumpPressed: false,
      jumpReleased: false,
      restart: false,
      keysHeld: [0x52],
      keysPressed: [0x52],
      keysReleased: [],
    });
    const afterPress = await bridge.tick(1);
    const diagnostics = await bridge.diagnostics();

    return {
      activeKeys: afterPress.inputTrace.activeKeys,
      afterPlayer: afterPress.player,
      afterRoomId: afterPress.roomId,
      beforePlayer: before.player,
      beforeRoomId: before.roomId,
      diagnostics,
      difficultyRoom,
    };
  });

  expect(result.beforeRoomId).toBe(result.difficultyRoom);
  expect(result.afterRoomId).toBe(result.difficultyRoom);
  expect(result.activeKeys).toContain('0x52:p1jp1jr0');
  expect(result.afterPlayer ? [result.afterPlayer.x, result.afterPlayer.y] : null).not.toEqual([321, 654]);
  expect(result.diagnostics.some((item) => item.includes('runtime-room-restart-requested'))).toBe(false);
});

const room143MovementBaselines = [
  {
    name: 'tap jump',
    scenario: 'dife-room143-tap-jump.json',
    ticks: 240,
    expected: {
      sampleCount: 12,
      minX: 81.0,
      maxX: 81.0,
      minY: 558.135,
      maxY: 567.445,
      maxAbsHspeed: 0.0,
      maxAbsVspeed: 3.155,
    },
  },
  {
    name: 'held jump',
    scenario: 'dife-room143-hold-jump.json',
    ticks: 240,
    expected: {
      sampleCount: 12,
      minX: 81.0,
      maxX: 81.0,
      minY: 482.4,
      maxY: 567.3,
      maxAbsHspeed: 0.0,
      maxAbsVspeed: 6.7,
    },
  },
  {
    name: 'release-cut jump',
    scenario: 'dife-room143-release-cut.json',
    ticks: 240,
    expected: {
      sampleCount: 12,
      minX: 81.0,
      maxX: 81.0,
      minY: 511.55,
      maxY: 567.13,
      maxAbsHspeed: 0.0,
      maxAbsVspeed: 1.615,
    },
  },
  {
    name: 'right movement',
    scenario: 'dife-room143-move-right.json',
    ticks: 120,
    expected: {
      sampleCount: 6,
      minX: 135.0,
      maxX: 154.0,
      minY: 567.4,
      maxY: 567.4,
      maxAbsHspeed: 3.0,
      maxAbsVspeed: 0.0,
    },
  },
] as const;

for (const baseline of room143MovementBaselines) {
  test(`sample package browser wasm bridge matches the room143 ${baseline.name} trace baseline`, async ({ page }) => {
    const result = await runWasmScenario(page, {
      scenario: readRuntimeScenario(baseline.scenario),
      roomId: 143,
      ticks: baseline.ticks,
      preselectTicks: 2,
      traceEvery: 20,
    });
    const summary = summarizePlayerTrace(result.trace);

    expectNoRuntimeBlockers(result.diagnostics);
    expect(result.finalRoomId).toBe(143);
    expect(summary.sampleCount).toBe(baseline.expected.sampleCount);
    expect(summary.minX).toBeCloseTo(baseline.expected.minX, 3);
    expect(summary.maxX).toBeCloseTo(baseline.expected.maxX, 3);
    expect(summary.minY).toBeCloseTo(baseline.expected.minY, 3);
    expect(summary.maxY).toBeCloseTo(baseline.expected.maxY, 3);
    expect(summary.maxAbsHspeed).toBeCloseTo(baseline.expected.maxAbsHspeed, 3);
    expect(summary.maxAbsVspeed).toBeCloseTo(baseline.expected.maxAbsVspeed, 3);
  });
}

test('sample package browser wasm bridge replays the room143 shoot scenario', async ({ page }) => {
  const result = await runWasmScenario(page, {
    scenario: readRuntimeScenario('dife-room143-shoot.json'),
    roomId: 143,
    ticks: 80,
    preselectTicks: 2,
    traceEvery: 20,
  });

  expectNoRuntimeBlockers(result.diagnostics);
  expect(result.finalRoomId).toBe(143);
  expect(result.finalFrameCommandCount).toBeGreaterThan(0);
  expect(result.trace.map((sample) => sample.tick)).toEqual([20, 40, 60, 80]);
  expect(result.trace.every((sample) => sample.player?.objectName === 'player')).toBe(true);
  expect(hasDiagnostic(result.diagnostics, 'runtime-instance-created', 'object=bullet')).toBe(true);
  expect(hasDiagnostic(result.diagnostics, 'runtime-instance-destroyed', 'object=bullet')).toBe(true);
});

test('sample package browser wasm bridge replays the room151 hazard death scenario', async ({ page }) => {
  const result = await runWasmScenario(page, {
    scenario: readRuntimeScenario('dife-room151-death-right.json'),
    roomId: 151,
    ticks: 180,
    preselectTicks: 2,
    traceEvery: 20,
  });

  expectNoRuntimeBlockers(result.diagnostics);
  expect(result.finalRoomId).toBe(151);
  expect(result.finalFrameCommandCount).toBeGreaterThan(0);
  expect(result.trace.map((sample) => sample.tick)).toContain(20);
  expect(result.trace.map((sample) => sample.tick)).toContain(40);
  expect(result.trace.some((sample) => (sample.player?.hspeed ?? 0) > 0)).toBe(true);
  expect(hasDiagnostic(result.diagnostics, 'runtime-player-died', 'object=player', 'reason=hazard')).toBe(true);
  expect(hasDiagnostic(result.diagnostics, 'runtime-instance-created', 'object=GAMEOVER')).toBe(true);
  expect(hasDiagnostic(result.diagnostics, 'runtime-instance-created', 'object=bloodEmitter2')).toBe(true);
});

test('sample package browser wasm bridge exposes timing phase samples during the room151 scenario', async ({ page }) => {
  const result = await runWasmScenario(page, {
    scenario: readRuntimeScenario('dife-room151-death-right.json'),
    roomId: 151,
    ticks: 180,
    preselectTicks: 2,
    performanceEvery: 30,
  });

  expectNoRuntimeBlockers(result.diagnostics);
  expect(result.performance.map((sample) => sample.tick)).toEqual([30, 60, 90, 120, 150, 180]);
  for (const sample of result.performance) {
    expectFinitePhaseTimings(sample);
  }
});
