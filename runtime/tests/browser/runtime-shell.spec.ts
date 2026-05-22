import { expect, test } from '@playwright/test';

async function loadPackage(page: import('@playwright/test').Page, packagePath: string): Promise<void> {
  await page.goto('/');
  await page.locator('input[name="packagePath"]').fill(packagePath);
  await page.getByRole('button', { name: 'Load Package' }).click();
}

test('sample package boots through the wasm path and exposes runtime telemetry', async ({ page }) => {
  await loadPackage(page, '/packages/sample');

  await expect(page.locator('#runtime-status')).toContainText('WASM runtime active');
  await expect(page.locator('#runtime-room')).toContainText('2: rInit');
  await expect(page.locator('#runtime-player')).toContainText('Player: x=');
  await expect(page.locator('#runtime-tick')).toContainText('Tick: 0');
  await expect(page.locator('#runtime-diagnostics')).toContainText('Diagnostics: none');
  await expect(page.locator('select[name="roomSelect"]')).toHaveValue('2');
  await expect(page.locator('#runtime-status')).toContainText('rInit');
});

test('sample package can switch to rStage01 and keep wasm telemetry visible', async ({ page }) => {
  await loadPackage(page, '/packages/sample');

  await expect(page.locator('#runtime-status')).toContainText('WASM runtime active');
  await expect(page.locator('select[name="roomSelect"]')).toHaveValue('2');
  await page.locator('select[name="roomSelect"]').selectOption('147');
  await expect(page.locator('#runtime-room')).toContainText('147: rStage01');
  await expect(page.locator('#runtime-status')).toContainText('rStage01');
  await expect(page.locator('#runtime-player')).toContainText('Player: x=');
  await expect(page.locator('#runtime-tick')).toContainText('Tick: 0');
  await expect(page.locator('#runtime-diagnostics')).toContainText('Diagnostics:');
});
