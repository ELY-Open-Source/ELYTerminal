import { mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { chromium } from '@playwright/test';

const base = process.env.BASE_URL || 'http://127.0.0.1:5173';
const output = path.resolve(fileURLToPath(new URL('.', import.meta.url)), '../../target/website-shots');
await mkdir(output, { recursive: true });

const browser = await chromium.launch({ channel: 'chrome', headless: true, args: ['--use-angle=metal'] });
const viewports = [
  { name: 'desktop', width: 1440, height: 900, mobile: false },
  { name: 'laptop', width: 1280, height: 720, mobile: false },
  { name: 'mobile', width: 390, height: 844, mobile: true },
  { name: 'small', width: 320, height: 568, mobile: true },
  { name: 'short', width: 844, height: 390, mobile: false },
];
const scenes = ['terminal', 'sessions', 'git', 'download'];
const only = process.env.ONLY?.split(',');
const failures = [];

for (const viewport of viewports.filter((item) => !only || only.includes(item.name))) {
  for (const theme of ['dark', 'light']) {
    const context = await browser.newContext({
      viewport: { width: viewport.width, height: viewport.height },
      deviceScaleFactor: 1,
      isMobile: viewport.mobile,
      hasTouch: viewport.mobile,
      colorScheme: theme,
    });
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror', (error) => errors.push(error.message));
    page.on('console', (message) => message.type() === 'error' && errors.push(message.text()));
    await page.goto(base, { waitUntil: 'networkidle' });
    await page.waitForFunction(() => document.documentElement.dataset.swarm === 'ready', null, { timeout: 15000 });
    for (const scene of scenes) {
      await page.evaluate((id) => scrollTo(0, document.getElementById(id).offsetTop), scene);
      await page.waitForFunction((id) => document.documentElement.dataset.scene === id, scene, { timeout: 5000 })
        .catch(() => failures.push(`${viewport.name} ${theme} ${scene}: scene stayed ${scene}`));
      await page.waitForTimeout(2600);
      await page.screenshot({ path: path.join(output, `${viewport.name}-${theme}-${scene}.png`) });
    }
    const state = await page.evaluate(() => ({
      webgl: document.documentElement.dataset.webgl,
      overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
    }));
    if (state.webgl !== 'ready') failures.push(`${viewport.name} ${theme}: webgl ${state.webgl}`);
    if (state.overflow > 0) failures.push(`${viewport.name} ${theme}: ${state.overflow}px horizontal overflow`);
    for (const error of errors) failures.push(`${viewport.name} ${theme}: ${error}`);
    await context.close();
  }
}

await browser.close();
console.log(`Screenshots in ${output}`);
if (failures.length) {
  console.error(failures.join('\n'));
  process.exit(1);
}
