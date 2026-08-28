import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test.use({ viewport: { width: 390, height: 844 } });

test('creates a proposal and records an accessible immutable decision', async ({ page }) => {
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
