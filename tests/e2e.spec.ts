import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test.use({ viewport: { width: 390, height: 844 } });

test('@claim:free-core creates, exports, and deletes a receipt without a Pro license', async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on('console', message => { if (message.type() === 'error') consoleErrors.push(message.text()); });
  page.on('pageerror', error => consoleErrors.push(error.message));
  const unique = Date.now();
  await page.goto('/');
  await expect(page).toHaveTitle(/Client Decision Receipt/);
  await expect(page.locator('h1')).toHaveCount(1);
  await page.locator('#make').scrollIntoViewIfNeeded();
  await page.getByLabel('Proposal title').fill(`Identity field guide ${unique}`);
  await page.getByLabel('Your name').first().fill('Fern Studio');
  await page.getByLabel('Your email').first().fill('hello@fern.test');
  await page.getByLabel('Client name').fill('Aster Client');
  await page.getByLabel('Client email').fill('aster@example.test');
  await page.getByRole('textbox', { name: 'Scope item', exact: true }).fill('Research and visual direction');
  await page.getByLabel('Details (optional)').fill('One workshop and a written direction');
  await page.getByLabel('Unit price').fill('850');
  await page.getByRole('button', { name: 'Create decision link' }).click();
  await expect(page.getByRole('heading', { name: 'Your links are ready.' })).toBeVisible();
  const clientUrl = await page.getByLabel('Client decision link').inputValue();
  const managerUrl = await page.getByLabel('Your private management link').inputValue();
  await page.goto(clientUrl);
  await expect(page.getByRole('heading', { name: 'Your decision is requested.' })).toBeVisible();
  await page.getByText('Accept', { exact: true }).click();
  await page.getByLabel('Your email').fill('aster@example.test');
  await page.getByLabel(/Decision note/).fill('Approved as listed.');
  await page.getByRole('button', { name: 'Record final decision' }).click();
  await expect(page.getByRole('heading', { name: 'Decision recorded.' })).toBeVisible();
  await expect(page.getByText('SHA-256 receipt seal')).toBeVisible();
  await expect(page.getByText('Accepted', { exact: true })).toBeVisible();
  const violations = await new AxeBuilder({ page }).analyze();
  expect(violations.violations.filter(v => ['serious', 'critical'].includes(v.impact || ''))).toEqual([]);
  await page.goto(managerUrl);
  await expect(page.getByRole('heading', { name: 'A decision is on record.' })).toBeVisible();
  await expect(page.getByText('sender not configured')).toHaveCount(2);
  const download = page.waitForEvent('download');
  await page.getByRole('link', { name: 'Export JSON' }).click();
  expect((await download).suggestedFilename()).toMatch(/-receipt\.json$/);
  await page.getByRole('button', { name: 'Delete data' }).click();
  await page.getByLabel(/Type DELETE/).fill('DELETE');
  await page.getByRole('button', { name: 'Delete permanently' }).click();
  await expect(page.getByRole('heading', { name: 'The proposal data was deleted.' })).toBeVisible();
  expect(consoleErrors).toEqual([]);
});

test('legal pages and offline shell have semantic landmarks', async ({ page }) => {
  for (const path of ['/privacy', '/terms']) {
    await page.goto(path);
    await expect(page.locator('main')).toHaveCount(1);
    await expect(page.locator('h1')).toHaveCount(1);
    await expect(page.locator('html')).toHaveAttribute('lang', 'en');
  }
});

test('routes set titles, provide a designed 404, and move skip-link focus to main', async ({ page }) => {
  await page.goto('/privacy');
  await expect(page).toHaveTitle('Privacy — Client Decision Receipt');
  await page.goto('/terms');
  await expect(page).toHaveTitle('Terms — Client Decision Receipt');
  const missing = await page.goto('/a-link-that-does-not-exist');
  expect(missing?.status()).toBe(404);
  await expect(page).toHaveTitle('Page not found — Client Decision Receipt');
  await expect(page.getByRole('heading', { name: 'This page could not be found.' })).toBeVisible();
  await page.goto('/');
  await page.getByRole('link', { name: 'Skip to main content' }).focus();
  await page.keyboard.press('Enter');
  await expect(page.locator('#main')).toBeFocused();
});

test('home and demo have no serious accessibility violations or mobile overflow', async ({ page }) => {
  for (const path of ['/', '/demo']) {
    await page.goto(path);
    const violations = await new AxeBuilder({ page }).analyze();
    expect(violations.violations.filter(v => ['serious', 'critical'].includes(v.impact || ''))).toEqual([]);
    await expect(page.locator('html')).toHaveJSProperty('scrollWidth', 390);
  }
});

test('@claim:demo-sandbox a sample decision is isolated and resettable', async ({ page }) => {
  const real = await page.request.post('/api/proposals', {
    data: {
      title: 'Real record kept apart', freelancerName: 'North Studio', freelancerEmail: 'hello@north.test',
      clientName: 'River Client', clientEmail: 'river@example.test', message: 'A real record for isolation testing', currency: 'USD',
      items: [{ label: 'Planning', description: 'Real work', quantity: 1, unitAmountCents: 50000 }],
    },
  });
  expect(real.status()).toBe(201);
  const links = await real.json() as { clientPath: string; managePath: string };

  let expiresAt = '';
  page.on('response', async response => {
    if (response.url().endsWith('/api/demo') && response.request().method() === 'POST') {
      expiresAt = (await response.json()).expiresAt;
    }
  });
  await page.goto('/');
  await page.getByRole('link', { name: 'Try it with sample data' }).click();
  await expect(page).toHaveURL(/\/demo$/);
  await expect(page.getByText('Demo — sample data, nothing is saved')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Review this sample decision request.' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Spring website refresh' })).toBeVisible();
  expect(Date.parse(expiresAt) - Date.now()).toBeGreaterThan(23 * 60 * 60 * 1000);
  expect(Date.parse(expiresAt) - Date.now()).toBeLessThanOrEqual(24 * 60 * 60 * 1000);

  await page.getByRole('radio', { name: /Accept/ }).check();
  await page.getByLabel('Your email').fill('maya@mossandmorning.example');
  await page.getByLabel(/Decision note/).fill('The sample scope is approved.');
  await page.getByRole('button', { name: 'Record sample decision' }).click();
  await expect(page.getByRole('heading', { name: 'Sample decision recorded.' })).toBeVisible();
  await expect(page.getByText('SHA-256 receipt seal')).toBeVisible();
  await expect(page.getByText('Demo — sample data, nothing is saved')).toBeVisible();

  await page.getByRole('button', { name: 'Reset demo' }).click();
  await expect(page.getByRole('heading', { name: 'Review this sample decision request.' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Record sample decision' })).toBeVisible();

  const unchanged = await page.request.get(`/api/proposals/${links.clientPath.slice(3)}`);
  expect(unchanged.status()).toBe(200);
  expect((await unchanged.json()).decision).toBeNull();
  const deleted = await page.request.delete(`/api/manage/${links.managePath.slice(3)}`, { data: { confirmation: 'DELETE' } });
  expect(deleted.status()).toBe(204);
});

test('@claim:frozen-receipt a final sample decision keeps the exact scope and seal', async ({ page }) => {
  await page.goto('/demo');
  await expect(page.getByRole('heading', { name: 'Spring website refresh' })).toBeVisible();
  await expect(page.getByText('Planning workshop')).toBeVisible();
  await expect(page.getByText('$3,200.00')).toBeVisible();
  await page.getByRole('radio', { name: /Request changes/ }).check();
  await page.getByLabel('Your email').fill('maya@mossandmorning.example');
  await page.getByLabel(/Decision note/).fill('Please include a newsletter signup section.');
  await page.getByRole('button', { name: 'Record sample decision' }).click();
  await expect(page.getByText('Changes requested', { exact: true })).toBeVisible();
  await expect(page.getByText('Planning workshop')).toBeVisible();
  await expect(page.getByText('SHA-256 receipt seal')).toBeVisible();
  await expect(page.locator('.seal code')).toHaveText(/[a-f0-9]{64}/);
});

test('@claim:no-tracking demo requests stay on this product origin', async ({ page }) => {
  const origins = new Set<string>();
  page.on('request', request => {
    const url = new URL(request.url());
    if (url.protocol === 'http:' || url.protocol === 'https:') origins.add(url.origin);
  });
  await page.goto('/demo');
  await expect(page.getByRole('heading', { name: 'Spring website refresh' })).toBeVisible();
  await page.getByRole('button', { name: 'Reset demo' }).click();
  await expect(page.getByRole('button', { name: 'Record sample decision' })).toBeVisible();
  expect([...origins]).toEqual(['http://127.0.0.1:4173']);
});

test('@claim:offline-reload the sample remains available offline after its first visit', async ({ browser }) => {
  const context = await browser.newContext({ viewport: { width: 390, height: 844 } });
  const page = await context.newPage();
  await page.goto('/demo');
  await expect(page.getByRole('heading', { name: 'Spring website refresh' })).toBeVisible();
  await page.evaluate(() => navigator.serviceWorker.ready);
  await context.setOffline(true);
  await page.reload();
  await expect(page.getByRole('heading', { name: 'Review this sample decision request.' })).toBeVisible();
  await context.close();
});
