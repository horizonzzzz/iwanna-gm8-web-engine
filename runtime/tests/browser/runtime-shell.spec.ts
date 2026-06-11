import { expect, test } from '@playwright/test';

async function loadPackage(page: import('@playwright/test').Page, packagePath: string): Promise<void> {
  await page.goto('/');
  await page.locator('input[name="packagePath"]').fill(packagePath);
  await page.getByRole('button', { name: 'Load Package' }).click();
}

test('sample package boots through the wasm path and exposes runtime telemetry', async ({ page }) => {
  await loadPackage(page, '/packages/sample');

  await expect(page.locator('#runtime-status')).toContainText('WASM runtime active');
  await expect(page.locator('#runtime-room')).toContainText(/Room: (2: rInit|111: rTitle|110: rMenu|109: rSelectStage)/);
  await expect(page.locator('#runtime-player')).toHaveText(/Player: (unavailable|x=)/);
  await expect.poll(async () => {
    const text = await page.locator('#runtime-tick').textContent();
    return Number(text?.replace('Tick: ', '') ?? '0');
  }).toBeGreaterThan(0);
  await expect(page.locator('#runtime-diagnostics')).toContainText('Diagnostics:');
  await expect(page.locator('#runtime-input')).toContainText('Input:');
  await expect(page.locator('#runtime-performance')).toContainText('Frame:');
  await expect(page.locator('select[name="roomSelect"]')).toHaveValue(/^(2|111|110|109)$/);
  await expect(page.locator('#runtime-status')).toContainText(/rInit|rTitle|rMenu|rSelectStage/);
});

test('sample package can switch to rStage01 and keep wasm telemetry visible', async ({ page }) => {
  await loadPackage(page, '/packages/sample');

  await expect(page.locator('#runtime-status')).toContainText('WASM runtime active');
  await expect(page.locator('select[name="roomSelect"]')).toHaveValue(/^(2|111|110|109)$/);
  await page.locator('select[name="roomSelect"]').selectOption('147');
  await expect(page.locator('#runtime-room')).toContainText('147: rStage01');
  await expect(page.locator('#runtime-status')).toContainText('rStage01');
  await expect(page.locator('#runtime-player')).toContainText('Player: x=');
  await expect.poll(async () => {
    const text = await page.locator('#runtime-tick').textContent();
    return Number(text?.replace('Tick: ', '') ?? '0');
  }).toBeGreaterThan(0);
  await expect(page.locator('#runtime-diagnostics')).toContainText('Diagnostics:');
  await expect(page.locator('#runtime-input')).toContainText('Input:');
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
