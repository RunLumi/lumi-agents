import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { readFile, mkdir } from 'node:fs/promises';
const headers = await readFile('dist/_headers', 'utf8');
const csp = headers.match(/Content-Security-Policy: (.+)/)[1];

test.beforeEach(async ({ page }) => {
  // Enforce Pages CSP on HTML. Assets inherit the document policy; intercepting
  // their bodies creates needless font-fetch teardown races in Playwright.
  await page.route('http://127.0.0.1:4321/**', async route => {
    if (route.request().resourceType() !== 'document') return route.continue();
    const response = await route.fetch();
    await route.fulfill({ response, headers: { ...response.headers(), 'content-security-policy': csp } });
  });
});
test.afterEach(async ({ page }) => {
  await page.unrouteAll({ behavior: 'wait' });
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
    await mkdir('test-results/screenshots', { recursive: true });
    await page.screenshot({ path: `test-results/screenshots/${name}.png`, fullPage: true });
    if (name === 'desktop') await page.screenshot({ path: 'test-results/screenshots/desktop-fold.png' });
    const labelColumnRatio = await page.locator('#thong-tin table').evaluate(table =>
      table.querySelector('thead th').getBoundingClientRect().width / table.getBoundingClientRect().width);
    expect(Math.abs(labelColumnRatio - (width <= 600 ? 0.32 : 0.28))).toBeLessThan(0.025);
    // The complete page above retains all chrome. Hide unrelated fixed chrome
    // only for this tall section crop, so it does not obscure the fact sheet.
    await page.locator('#thong-tin').screenshot({
      path: `test-results/screenshots/facts-${name}.png`,
      style: '.site-header, .skip-link { visibility: hidden !important; }',
    });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    expect(await page.evaluate(() => getComputedStyle(document.body).backgroundColor)).toBe('rgb(244, 240, 232)');
    expect(await page.evaluate(() => document.fonts.check('600 32px "Geist Variable"', 'Giữ quyền làm chủ'))).toBe(true);
    const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21aa']).analyze();
    expect(results.violations).toEqual([]);
    expect(errors).toEqual([]);
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
  try {
    const page = await context.newPage();
    await page.goto('http://127.0.0.1:4321/');
    await expect(page.locator('h1')).toBeVisible();
    await expect(page.locator('#thong-tin table')).toBeVisible();
    await expect(page.locator('#panel-finance')).toBeVisible();
    await page.locator('.faq-list summary').first().click();
    await expect(page.locator('.faq-list details').first()).toHaveAttribute('open', '');
    await expect(page.locator('.hero-actions a[href^="mailto:"]')).toBeVisible();
  } finally {
    await context.close();
  }
});
test('reduced motion is honored and unknown paths have a proper 404', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/');
  await page.evaluate(() => document.fonts.ready);
  expect(await page.evaluate(() => getComputedStyle(document.documentElement).scrollBehavior)).toBe('auto');
  const response = await page.goto('/this-page-does-not-exist/');
  expect(response.status()).toBe(404);
  await page.evaluate(() => document.fonts.ready);
});

// Compare rendered DOM, not only shared source imports: schema must describe
// the actual FAQ visitors can expand, including punctuation and Vietnamese text.
test('FAQ schema matches rendered answers and the fact sheet is accessible', async ({ page, request }) => {
  await page.goto('/');
  const { questions, visibleFaqs } = await page.evaluate(() => {
    const graph = [...document.querySelectorAll('script[type="application/ld+json"]')]
      .flatMap(script => JSON.parse(script.textContent)['@graph'] || []);
    const faq = graph.find(item => item['@type'] === 'FAQPage');
    return {
      questions: faq.mainEntity.map(item => [item.name, item.acceptedAnswer.text]),
      visibleFaqs: [...document.querySelectorAll('.faq-list details')]
        .map(detail => [detail.querySelector('summary').textContent.trim(), detail.querySelector('p').textContent.trim()]),
    };
  });
  expect(questions).toEqual(visibleFaqs);
  await expect(page.getByRole('table', { name: 'Thông tin sản phẩm và giới hạn hiện tại' })).toBeVisible();
  await expect(page.locator('#thong-tin a[rel="author"]')).toHaveText('RunLumi');
  const response = await request.get('/llms.txt');
  expect(response.status()).toBe(200);
  expect(response.headers()['content-type']).toContain('text/plain');
  expect(await response.text()).toContain('# Lumi Agents');
});
