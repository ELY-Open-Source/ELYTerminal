import assert from 'node:assert/strict';
import { chromium } from '@playwright/test';

const base = process.env.BASE_URL || 'http://127.0.0.1:5173';
const browser = await chromium.launch({ channel: 'chrome', headless: true, args: ['--use-angle=metal'] });
const hidden = 'main, .site-header { opacity: 0 !important; }';

// Bounds of Ely's pink pixels, with the page's text hidden.
async function drawn(page) {
  const png = await page.screenshot({ style: hidden });
  return page.evaluate(async (data) => {
    const image = new Image();
    image.src = `data:image/png;base64,${data}`;
    await image.decode();
    const canvas = document.createElement('canvas');
    canvas.width = image.width;
    canvas.height = image.height;
    const context = canvas.getContext('2d', { willReadFrequently: true });
    context.drawImage(image, 0, 0);
    const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
    const box = { count: 0, left: Infinity, top: Infinity, right: -Infinity, bottom: -Infinity };
    for (let index = 0; index < pixels.length; index += 4) {
      const [r, g] = [pixels[index], pixels[index + 1]];
      if (r < 100 || r - g < 40) continue;
      const x = (index / 4) % canvas.width;
      const y = Math.floor(index / 4 / canvas.width);
      box.count += 1;
      box.left = Math.min(box.left, x);
      box.top = Math.min(box.top, y);
      box.right = Math.max(box.right, x + 1);
      box.bottom = Math.max(box.bottom, y + 1);
    }
    const scene = document.documentElement.dataset.scene;
    const slot = document.querySelector(`#${scene} .ely-slot`).getBoundingClientRect();
    const seen = Math.max(0, Math.min(slot.bottom, innerHeight) - Math.max(slot.top, 0)) / slot.height;
    return { box, scene, slot: { left: slot.left, top: slot.top, right: slot.right, bottom: slot.bottom }, seen };
  }, png.toString('base64'));
}

function check(label, { box, scene, slot, seen }) {
  const at = `${label} ${scene}`;
  if (seen > 0.5) assert.ok(box.count > 100, `${at}: Ely is missing from her slot`);
  if (box.count === 0) return;
  const pad = 6;
  assert.ok(box.left >= slot.left - pad && box.right <= slot.right + pad
    && box.top >= slot.top - pad && box.bottom <= slot.bottom + pad,
  `${at}: Ely drawn at ${JSON.stringify(box)} outside slot ${JSON.stringify(slot)}`);
}

async function open(viewport, reducedMotion, language) {
  const context = await browser.newContext({ viewport, deviceScaleFactor: 1, colorScheme: 'dark', reducedMotion });
  await context.addInitScript((choice) => localStorage.setItem('elyterminal-language', choice), language);
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto(base, { waitUntil: 'networkidle' });
  await page.waitForFunction(() => document.documentElement.dataset.swarm === 'ready', null, { timeout: 15000 });
  return { context, page, errors };
}

// Every scroll position keeps Ely inside the current scene's slot.
for (const [width, height, resize] of [[320, 568], [844, 390], [1280, 720, [844, 390]]]) {
  for (const language of ['en', 'zh']) {
    const { context, page, errors } = await open({ width, height }, 'reduce', language);
    if (resize) await page.setViewportSize({ width: resize[0], height: resize[1] });
    const end = await page.evaluate(() => document.documentElement.scrollHeight - innerHeight);
    for (let y = 0; y <= end + 139; y += 140) {
      await page.evaluate((top) => scrollTo(0, top), Math.min(y, end));
      await page.waitForTimeout(250);
      check(`${width}x${height}${resize ? '>844x390' : ''} ${language} y=${Math.min(y, end)}`, await drawn(page));
    }
    assert.deepEqual(errors, []);
    await context.close();
  }
}

// Full motion settles in the slot after a scroll.
{
  const { context, page, errors } = await open({ width: 320, height: 568 }, 'no-preference', 'en');
  for (const scene of ['sessions', 'git']) {
    await page.evaluate((id) => scrollTo(0, document.getElementById(id).offsetTop), scene);
    await page.waitForTimeout(600);
    await page.mouse.move(160, 300);
    await page.mouse.wheel(0, 400);
    await page.waitForTimeout(3000);
    check(`320x568 full ${scene}+400`, await drawn(page));
  }
  assert.deepEqual(errors, []);
  await context.close();
}

await browser.close();
console.log('Ely stays in the current slot at every scroll position, resize and full motion.');
