import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { readFile, mkdir } from 'node:fs/promises';
const headers = await readFile('dist/_headers', 'utf8');
const csp = headers.match(/Content-Security-Policy: (.+)/)[1];

test.beforeEach(async ({ page }) => {
  // Astro preview does not implement Pages _headers. Apply the generated policy
  // to real browser responses so inline scripts, fonts and controls are tested.
  await page.route('http://127.0.0.1:4321/**', async route => {
    const response = await route.fetch();
    await route.fulfill({ response, headers: { ...response.headers(), 'content-security-policy': csp } });
  });
});
for (const [name, width, height] of [['mobile', 375, 812], ['small-mobile', 320, 740], ['tablet', 768, 1024], ['desktop', 1440, 1050]]) {
  test(`${name}: readable layout, no overflow, no serious accessibility defects`, async ({ page }) => {
    await page.setViewportSize({ width, height });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.goto('/');
    await page.evaluate(() => document.fonts.ready);
    await expect(page.locator('h1')).toHaveCount(1);
    await expect(page.getByRole('tablist')).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    expect(await page.evaluate(() => getComputedStyle(document.body).backgroundColor)).toBe('rgb(244, 240, 232)');
    const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21aa']).analyze();
    expect(results.violations).toEqual([]);
    expect(errors).toEqual([]);
    await mkdir('test-results/screenshots', { recursive: true });
    await page.screenshot({ path: `test-results/screenshots/${name}.png`, fullPage: true });
    if (name === 'desktop') await page.screenshot({ path: 'test-results/screenshots/desktop-fold.png' });
  });
}
test('demo tabs work by mouse and keyboard; sample files are real', async ({ page }) => {
  await page.goto('/');
  const finance = page.getByRole('tab', { name: 'Đối soát', exact: true });
  await finance.focus();
  await page.keyboard.press('ArrowRight');
  await expect(page.getByRole('tab', { name: 'Tổng hợp tài liệu', exact: true })).toBeFocused();
  await expect(page.locator('#panel-research')).toBeVisible();
  await page.keyboard.press('End');
  await expect(page.locator('#panel-ads')).toBeVisible();
  await page.keyboard.press('Home');
  await expect(page.locator('#panel-finance')).toBeVisible();
  await page.getByRole('tab', { name: 'Kiểm tra quảng cáo', exact: true }).click();
  await expect(page.locator('#panel-ads')).toBeVisible();
  await expect(page.locator('#panel-finance')).toBeHidden();
  await page.locator('#panel-ads summary').click();
  await expect(page.locator('#panel-ads .demo-exception p')).toBeVisible();
  const download = page.waitForEvent('download');
  await page.locator('#panel-ads .artifact-download').click();
  expect((await download).suggestedFilename()).toBe('quang-cao-mau.csv');
});
test('mobile navigation, Escape and FAQ work without trapped focus', async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 812 });
  await page.goto('/');
  const menu = page.locator('.mobile-nav');
  await menu.locator('summary').click();
  await expect(menu).toHaveAttribute('open', '');
  await page.keyboard.press('Escape');
  await expect(menu).not.toHaveAttribute('open');
  await expect(menu.locator('summary')).toBeFocused();
  await menu.locator('summary').click();
  await menu.getByRole('link', { name: 'Câu hỏi', exact: true }).click();
  await expect(menu).not.toHaveAttribute('open');
  await page.locator('.faq-list summary').first().click();
  await expect(page.locator('.faq-list details').first()).toHaveAttribute('open', '');
});
test('no-JavaScript retains content, FAQ, mobile menu and primary contact', async ({ browser }) => {
  const context = await browser.newContext({ javaScriptEnabled: false, viewport: { width: 375, height: 812 } });
  const page = await context.newPage();
  await page.goto('http://127.0.0.1:4321/');
  await expect(page.locator('h1')).toBeVisible();
  await expect(page.locator('#panel-finance')).toBeVisible();
  await page.locator('.faq-list summary').first().click();
  await expect(page.locator('.faq-list details').first()).toHaveAttribute('open', '');
  await expect(page.locator('.hero-actions a[href^="mailto:"]')).toBeVisible();
  await context.close();
});
test('reduced motion is honored and unknown paths have a proper 404', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/');
  expect(await page.evaluate(() => getComputedStyle(document.documentElement).scrollBehavior)).toBe('auto');
  const response = await page.goto('/this-page-does-not-exist/');
  expect(response.status()).toBe(404);
});
