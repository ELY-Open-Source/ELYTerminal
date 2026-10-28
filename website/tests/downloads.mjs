import assert from 'node:assert/strict';
import { chromium } from '@playwright/test';
import { ASSETS, RELEASES_PAGE, releaseUrl } from '../src/downloads.js';

const base = process.env.BASE_URL || 'http://127.0.0.1:5173';
const browser = await chromium.launch({ channel: 'chrome', headless: true });
const platforms = [
  ['macOS', 'MacIntel', 'Macintosh; Intel Mac OS X 10_15_7', 'macos', ASSETS.macos, 'Download for macOS'],
  ['Windows', 'Win32', 'Windows NT 10.0; Win64; x64', 'windows', ASSETS.windows, 'Download for Windows'],
  ['Linux', 'Linux x86_64', 'X11; Linux x86_64', 'linux', ASSETS.appimage, 'Download for Linux'],
  ['Android', 'Linux armv8l', 'Android; Mobile; Linux', 'other', null, 'View releases', 0],
  ['macOS', 'MacIntel', 'Macintosh; Intel Mac OS X 10_15_7', 'other', null, 'View releases', 5],
];

for (const [modern, platform, userAgent, detected, asset, label, maxTouchPoints] of platforms) {
  const context = await browser.newContext({ viewport: { width: 1280, height: 720 }, userAgent, locale: 'en-US' });
  await context.addInitScript(({ modern, platform, maxTouchPoints }) => {
    Object.defineProperty(navigator, 'userAgentData', { value: { platform: modern }, configurable: true });
    Object.defineProperty(navigator, 'platform', { value: platform, configurable: true });
    Object.defineProperty(navigator, 'maxTouchPoints', { value: maxTouchPoints, configurable: true });
  }, { modern, platform, maxTouchPoints });
  const page = await context.newPage();
  await page.goto(base, { waitUntil: 'networkidle' });
  const primary = page.locator('#primary-download');
  assert.equal(await primary.getAttribute('data-platform'), detected);
  assert.equal(await primary.getAttribute('href'), asset ? releaseUrl(asset) : RELEASES_PAGE);
  assert.equal(await page.locator('#primary-label').textContent(), label);
  assert.ok(await page.locator('#primary-label').getAttribute('data-zh'));
  const links = await page.locator('[data-download]').evaluateAll((elements) => elements.map((element) => [element.dataset.download, element.href]));
  assert.equal(links.length, 6);
  for (const [key, href] of links) assert.equal(href, releaseUrl(ASSETS[key]));
  await context.close();
}

for (const [width, height] of [[1440, 900], [1280, 720], [390, 844], [320, 568], [844, 390], [1280, 600]]) {
  const context = await browser.newContext({ viewport: { width, height }, colorScheme: 'dark' });
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto(base, { waitUntil: 'networkidle' });
  await page.evaluate(() => {
    document.documentElement.style.scrollBehavior = 'auto';
    window.scrollTo(0, document.getElementById('download').offsetTop);
  });
  await page.waitForTimeout(900);
  const metrics = await page.evaluate(() => {
    const title = document.getElementById('download-title').getBoundingClientRect();
    const actions = document.querySelector('#primary-download').getBoundingClientRect();
    const section = document.getElementById('download').getBoundingClientRect();
    const formats = document.querySelector('.download-alternatives').getBoundingClientRect();
    return {
      titleBottom: title.bottom, top: actions.top,
      left: Math.min(actions.left, formats.left), right: Math.max(actions.right, formats.right),
      bottom: formats.bottom, sectionBottom: section.bottom,
      width: document.documentElement.scrollWidth,
    };
  });
  assert.equal(metrics.width, width);
  assert.ok(metrics.left >= 0 && metrics.right <= width, `${width}: downloads clipped`);
  assert.ok(metrics.titleBottom <= metrics.top, `${width}: heading covered by downloads`);
  assert.ok(metrics.bottom <= metrics.sectionBottom, `${width}: downloads outside section`);
  if (metrics.bottom > height) {
    await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight));
    const bottom = await page.locator('.download-alternatives').evaluate((element) => element.getBoundingClientRect().bottom);
    assert.ok(bottom <= height, `${width}: downloads cannot be scrolled into view`);
  }
  assert.deepEqual(errors, []);
  await context.close();
}

await browser.close();
console.log('Five platform choices, six links and six download layouts passed.');
