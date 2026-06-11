import { expect, test } from '@playwright/test';

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
