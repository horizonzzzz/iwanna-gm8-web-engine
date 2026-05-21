import { expect, test } from '@playwright/test';

async function loadPackage(page: import('@playwright/test').Page, packagePath: string): Promise<void> {
  await page.goto('/');
  await page.locator('input[name="packagePath"]').fill(packagePath);
  await page.getByRole('button', { name: 'Load Package' }).click();
}

test('mashikaku exposes runtime telemetry for boot and room selection', async ({ page }) => {
  await loadPackage(page, '/packages/mashikaku');

  await expect(page.locator('#runtime-status')).toContainText('WASM runtime active');
  await expect(page.locator('#runtime-room')).toContainText('2: rInit');
  await expect(page.locator('#runtime-player')).toContainText('Player: unavailable');
  await expect(page.locator('#runtime-tick')).toContainText('Tick: 0');

  await page.locator('select[name="roomSelect"]').selectOption('87');
  await expect(page.locator('#runtime-room')).toContainText('87: rStage01');
  await expect(page.locator('#runtime-tick')).toContainText('Tick: 0');
  await expect(page.locator('#runtime-diagnostics')).toContainText('Diagnostics:');
});

test('kamilia still boots through the wasm path', async ({ page }) => {
  await loadPackage(page, '/packages/kamilia');

  await expect(page.locator('#runtime-status')).toContainText('WASM runtime active');
  await expect(page.locator('#runtime-room')).toContainText('0: startRoom');
  await expect(page.locator('#runtime-diagnostics')).toContainText('Diagnostics:');
});
