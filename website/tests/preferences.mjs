import assert from 'node:assert/strict';
import { mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { chromium } from '@playwright/test';

const base = process.env.BASE_URL || 'http://127.0.0.1:5173';
const output = path.resolve(fileURLToPath(new URL('.', import.meta.url)), '../../target/website-shots');
await mkdir(output, { recursive: true });
const browser = await chromium.launch({ channel: 'chrome', headless: true });

const context = await browser.newContext({
  viewport: { width: 1440, height: 900 }, colorScheme: 'dark', locale: 'en-US',
});
const page = await context.newPage();
const errors = [];
page.on('pageerror', (error) => errors.push(error.message));
await page.goto(base, { waitUntil: 'networkidle' });
assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark');
assert.equal(await page.locator('html').getAttribute('data-locale'), 'en');
assert.equal(await page.locator('html').getAttribute('lang'), 'en');
assert.equal(await page.locator('#language-toggle').textContent(), '中文');
assert.equal(await page.locator('#primary-label').textContent(), 'Download for macOS');
await page.emulateMedia({ colorScheme: 'light' });
await page.waitForFunction(() => document.documentElement.dataset.theme === 'light');
assert.equal(await page.locator('html').getAttribute('data-theme'), 'light');
await page.emulateMedia({ colorScheme: 'dark' });
await page.waitForFunction(() => document.documentElement.dataset.theme === 'dark');
assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark');

await page.locator('#language-toggle').click();
assert.equal(await page.locator('html').getAttribute('data-locale'), 'zh');
assert.equal(await page.locator('html').getAttribute('lang'), 'zh-CN');
assert.equal(await page.locator('#language-toggle').textContent(), 'EN');
assert.equal(await page.locator('#language-toggle').getAttribute('aria-label'), '切换到英语');
assert.equal(await page.locator('.site-header nav').getAttribute('aria-label'), '主导航');
assert.equal(await page.locator('.site-header nav a[href="#sessions"]').textContent(), '会话');
assert.equal(await page.getByRole('link', { name: '会话', exact: true }).count(), 1);
await page.waitForFunction(() => document.getElementById('sessions-title').textContent === '会话');
assert.equal(await page.locator('#files-tab').textContent(), '文件');
assert.equal(await page.locator('#primary-label').textContent(), '下载 macOS 版');
assert.equal(await page.locator('#theme-toggle').getAttribute('aria-label'), '切换到浅色主题');
await page.locator('#theme-toggle').click();
assert.equal(await page.locator('html').getAttribute('data-theme'), 'light');
assert.equal(await page.locator('meta[name="theme-color"]').getAttribute('content'), '#f1efe9');
assert.equal(await page.locator('#theme-toggle').getAttribute('aria-label'), '切换到深色主题');
await page.screenshot({ path: path.join(output, 'preferences-light-zh.png') });
await page.reload({ waitUntil: 'networkidle' });
assert.equal(await page.locator('html').getAttribute('data-theme'), 'light');
assert.equal(await page.locator('html').getAttribute('data-locale'), 'zh');
await page.emulateMedia({ colorScheme: 'dark' });
await page.waitForTimeout(100);
assert.equal(await page.locator('html').getAttribute('data-theme'), 'light');

await page.locator('#language-toggle').focus();
await page.keyboard.press('Enter');
assert.equal(await page.locator('html').getAttribute('data-locale'), 'en');
assert.equal(await page.locator('.site-header nav a[href="#sessions"]').textContent(), 'Sessions');
assert.equal(await page.getByRole('link', { name: 'Sessions', exact: true }).count(), 1);
await page.locator('#theme-toggle').focus();
await page.keyboard.press('Space');
assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark');
await page.emulateMedia({ colorScheme: 'light' });
await page.waitForTimeout(100);
assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark');
await page.reload({ waitUntil: 'networkidle' });
assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark');
assert.equal(await page.locator('html').getAttribute('data-locale'), 'en');
await page.screenshot({ path: path.join(output, 'preferences-dark-en.png') });
assert.deepEqual(errors, []);
await context.close();

const chinese = await browser.newContext({
  viewport: { width: 1280, height: 720 }, colorScheme: 'light', locale: 'zh-CN',
});
const chinesePage = await chinese.newPage();
await chinesePage.goto(base, { waitUntil: 'networkidle' });
assert.equal(await chinesePage.locator('html').getAttribute('data-locale'), 'zh');
assert.equal(await chinesePage.locator('html').getAttribute('data-theme'), 'light');
assert.match(await chinesePage.locator('#primary-label').textContent(), /^(下载 .+ 版|查看发布)$/);
await chinese.close();

for (const [width, height] of [[1440, 900], [1280, 720], [390, 844], [320, 568], [844, 390], [1280, 600]]) {
  const sized = await browser.newContext({ viewport: { width, height }, colorScheme: 'light', locale: 'zh-CN' });
  const sizedPage = await sized.newPage();
  const pageErrors = [];
  sizedPage.on('pageerror', (error) => pageErrors.push(error.message));
  await sizedPage.goto(base, { waitUntil: 'networkidle' });
  await sizedPage.evaluate(async () => { await document.fonts.ready; });
  const checkLayout = async () => {
    const metrics = await sizedPage.evaluate(() => {
      const header = document.querySelector('.site-header');
      const box = header.getBoundingClientRect();
      const nav = document.querySelector('.site-header nav').getBoundingClientRect();
      const controls = document.querySelector('.site-controls').getBoundingClientRect();
      const buttons = [...document.querySelectorAll('.site-controls button')].map((button) => {
        const rect = button.getBoundingClientRect();
        return { width: rect.width, height: rect.height, right: rect.right };
      });
      return {
        width: document.documentElement.scrollWidth,
        headerLeft: box.left, headerRight: box.right,
        headerClient: header.clientWidth, headerScroll: header.scrollWidth,
        navRight: nav.right, controlsLeft: controls.left, buttons,
      };
    });
    const locale = await sizedPage.locator('html').getAttribute('data-locale');
    assert.equal(metrics.width, width, `${width} ${locale}: page overflow`);
    assert.ok(metrics.headerLeft >= 0 && metrics.headerRight <= width, `${width} ${locale}: header clipped`);
    assert.ok(metrics.headerScroll <= metrics.headerClient + 1, `${width} ${locale}: header content overflow`);
    assert.ok(metrics.navRight <= metrics.controlsLeft, `${width} ${locale}: navigation covers controls`);
    for (const button of metrics.buttons) {
      assert.ok(button.width >= 44 && button.height >= 44, `${width} ${locale}: undersized control`);
      assert.ok(button.right <= metrics.headerRight, `${width} ${locale}: control clipped`);
    }
  };
  await checkLayout();
  await sizedPage.locator('#language-toggle').click();
  await checkLayout();
  await sizedPage.locator('#theme-toggle').focus();
  const focusRing = await sizedPage.locator('#theme-toggle').evaluate((button) => {
    const rect = button.getBoundingClientRect();
    const style = getComputedStyle(button);
    const edge = parseFloat(style.outlineWidth) + parseFloat(style.outlineOffset);
    return { left: rect.left - edge, right: rect.right + edge };
  });
  assert.ok(focusRing.left >= 0 && focusRing.right <= width, `${width}: focus ring clipped`);
  if (width === 390) await sizedPage.screenshot({ path: path.join(output, 'preferences-mobile.png') });
  assert.deepEqual(pageErrors, []);
  await sized.close();
}

const blocked = await browser.newContext({ viewport: { width: 1280, height: 720 }, colorScheme: 'light' });
await blocked.addInitScript(() => {
  Object.defineProperty(window, 'localStorage', {
    configurable: true,
    get() { throw new DOMException('Storage blocked', 'SecurityError'); },
  });
});
const blockedPage = await blocked.newPage();
const blockedErrors = [];
blockedPage.on('pageerror', (error) => blockedErrors.push(error.message));
await blockedPage.goto(base, { waitUntil: 'networkidle' });
assert.equal(await blockedPage.locator('html').getAttribute('data-theme'), 'light');
await blockedPage.locator('#theme-toggle').click();
assert.equal(await blockedPage.locator('html').getAttribute('data-theme'), 'dark');
assert.deepEqual(blockedErrors, []);
await blocked.close();

await browser.close();
console.log('Locale, theme, persistence, keyboard, storage and six layouts passed.');
