import { expect, test } from '@playwright/test';

test('public upload boots the returned package on canvas', async ({ page }) => {
  await page.route('**/api/v1/games', async (route) => {
    expect(route.request().postDataBuffer()?.byteLength).toBeGreaterThan(0);
    await route.fulfill({
      contentType: 'application/json',
      body: JSON.stringify({
        id: 'synthetic',
        status: 'ready',
        compatibility: 'partial',
        package_url: '/packages/sample',
        warnings: [],
      }),
    });
  });
  await page.goto('/');
  await page.getByLabel('游戏包').setInputFiles({
    name: 'synthetic.exe',
    mimeType: 'application/octet-stream',
    buffer: Buffer.from('synthetic upload fixture'),
  });

  await page.getByRole('button', { name: '开始游戏' }).click();

  await expect(page.getByText('游戏已启动。')).toBeVisible();
  await expect(page.getByText('RUNNING')).toBeVisible();
  await expect(page.getByRole('button', { name: '重置' })).toBeEnabled();
  await expect(page.locator('#room-canvas')).toBeVisible();
  await expect.poll(() => page.locator('#room-canvas').evaluate((canvas) => (
    canvas instanceof HTMLCanvasElement ? canvas.width * canvas.height : 0
  ))).toBeGreaterThan(0);
});
