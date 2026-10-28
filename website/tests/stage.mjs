import assert from 'node:assert/strict';
import { chromium } from '@playwright/test';

const base = process.env.BASE_URL || 'http://127.0.0.1:5173';
const browser = await chromium.launch({ channel: 'chrome', headless: true, args: ['--use-angle=metal'] });
const scenes = ['terminal', 'sessions', 'git', 'download'];

// Records each data-swarm value the page passes through.
function watchSwarm() {
  window.swarmStates = [];
  new MutationObserver(() => {
    const state = document.documentElement.dataset.swarm;
    if (state && window.swarmStates.at(-1) !== state) window.swarmStates.push(state);
  }).observe(document, { subtree: true, attributes: true, attributeFilter: ['data-swarm'] });
}

async function open(options, setup) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, colorScheme: 'dark', ...options });
  await context.addInitScript(watchSwarm);
  if (setup) await setup(context);
  const page = await context.newPage();
  const logs = [];
  const errors = [];
  page.on('console', (message) => (message.type() === 'error' ? errors : logs).push(message.text()));
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto(base, { waitUntil: 'networkidle' });
  await page.waitForFunction(() => document.documentElement.dataset.swarm === 'ready', null, { timeout: 15000 });
  return { context, page, logs, errors };
}

async function visit(page, scene) {
  await page.evaluate((id) => scrollTo(0, document.getElementById(id).offsetTop), scene);
  await page.waitForFunction((id) => document.documentElement.dataset.scene === id, scene, { timeout: 5000 });
}

// The h1 fades in once the swarm is ready.
const inked = (page) => page.waitForFunction(() => {
  const color = getComputedStyle(document.getElementById('product-name')).color;
  return color === getComputedStyle(document.documentElement).color;
}, null, { timeout: 2000 });

const canvas = (page) => page.locator('#stage canvas').screenshot();

{
  const { context, page, logs, errors } = await open();
  assert.deepEqual(await page.evaluate(() => window.swarmStates), ['loading', 'intro', 'ready']);
  const spelled = logs.map((line) => /intro spells (\d+) cells/.exec(line)).find(Boolean);
  assert.ok(spelled && Number(spelled[1]) > 100, `intro spelled too little: ${spelled?.[1]}`);
  assert.equal(await page.locator('html').getAttribute('data-webgl'), 'ready');
  await inked(page);
  for (const scene of scenes) {
    await visit(page, scene);
    await page.waitForTimeout(400);
  }
  for (const scene of scenes) {
    assert.ok(logs.some((line) => line.endsWith(`Ely formed for ${scene}`)), `Ely never formed for ${scene}`);
  }
  const first = await canvas(page);
  await page.waitForTimeout(200);
  assert.ok(!first.equals(await canvas(page)), 'the swarm stands still in full motion');
  const intervals = await page.evaluate(() => new Promise((resolve) => {
    const samples = [];
    let previous;
    requestAnimationFrame(function frame(now) {
      if (previous) samples.push(now - previous);
      previous = now;
      if (samples.length === 120) resolve(samples);
      else requestAnimationFrame(frame);
    });
  }));
  const average = intervals.reduce((sum, value) => sum + value, 0) / intervals.length;
  console.log(`Average frame interval: ${average.toFixed(2)}ms`);
  assert.ok(average < 25, `slow frames: ${average}ms`);
  const formed = logs.length;
  await page.locator('#theme-toggle').click();
  await page.waitForTimeout(600);
  assert.ok(logs.slice(formed).some((line) => line.includes('Ely formed for')), 'a theme change did not reform Ely');
  assert.deepEqual(errors, []);
  await context.close();
}

{
  const { context, page, errors } = await open({ reducedMotion: 'reduce' });
  assert.deepEqual(await page.evaluate(() => window.swarmStates), ['ready']);
  await visit(page, 'git');
  await page.waitForTimeout(500);
  const still = await canvas(page);
  await page.waitForTimeout(400);
  assert.ok(still.equals(await canvas(page)), 'reduced motion keeps drawing');
  assert.deepEqual(errors, []);
  await context.close();
}

{
  const { context, page, logs, errors } = await open();
  await visit(page, 'sessions');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.waitForTimeout(500);
  assert.ok(logs.includes('ELYTerminal: swarm motion reduced'), 'the reduce change went unheard');
  const still = await canvas(page);
  await page.mouse.click(700, 450);
  await page.waitForTimeout(400);
  assert.ok(still.equals(await canvas(page)), 'reduced motion still moves after a live change');
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.waitForTimeout(300);
  const moving = await canvas(page);
  await page.waitForTimeout(200);
  assert.ok(!moving.equals(await canvas(page)), 'full motion did not resume');
  await page.evaluate(() => scrollTo(0, 0));
  await page.reload({ waitUntil: 'domcontentloaded' });
  await page.waitForFunction(() => document.documentElement.dataset.swarm === 'intro', null, { timeout: 15000 });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.waitForFunction(() => document.documentElement.dataset.swarm === 'ready', null, { timeout: 1000 });
  await page.waitForTimeout(500);
  assert.ok(logs.filter((line) => line.endsWith('Ely formed for terminal')).length >= 2, 'reduce mid-intro left Ely unformed');
  assert.deepEqual(errors, []);
  await context.close();
}

// A change while the wasm loads still takes effect.
for (const [from, to] of [['no-preference', 'reduce'], ['reduce', 'no-preference']]) {
  let release;
  const held = new Promise((resolve) => { release = resolve; });
  const { context, page, logs, errors } = await open({ reducedMotion: from }, async (setup) => {
    await setup.route('**/swarm*.wasm', async (route) => {
      await held;
      await route.continue();
    });
    setup.on('request', (request) => {
      if (!/swarm.*\.wasm/.test(request.url())) return;
      const target = setup.pages()[0];
      target.emulateMedia({ reducedMotion: to }).then(() => setTimeout(release, 150));
    });
  });
  const expected = to === 'reduce' ? 'reduced motion' : 'full motion';
  assert.ok(logs.includes(`ELYTerminal: stage ready, ${expected}`), `${from} > ${to} while loading: not ${expected}`);
  await page.waitForTimeout(2500);
  const first = await canvas(page);
  await page.waitForTimeout(400);
  assert.equal(first.equals(await canvas(page)), to === 'reduce', `${from} > ${to} while loading: wrong motion`);
  assert.deepEqual(errors, []);
  await context.close();
}

// A lost context leaves the themed page visible.
{
  const { context, page, errors } = await open({ viewport: { width: 320, height: 568 }, reducedMotion: 'reduce' });
  await page.evaluate(() => document.querySelector('#stage canvas').getContext('webgl2')
    .getExtension('WEBGL_lose_context').loseContext());
  await page.waitForFunction(() => document.documentElement.dataset.webgl === 'unavailable', null, { timeout: 5000 });
  await page.waitForTimeout(1000);
  const png = await page.screenshot();
  const bright = await page.evaluate(async (data) => {
    const image = new Image();
    image.src = `data:image/png;base64,${data}`;
    await image.decode();
    const canvas = document.createElement('canvas');
    canvas.width = image.width;
    canvas.height = image.height;
    const context = canvas.getContext('2d');
    context.drawImage(image, 0, 0);
    const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
    let count = 0;
    for (let index = 0; index < pixels.length; index += 4) if (pixels[index + 1] > 200) count += 1;
    return count / (pixels.length / 4);
  }, png.toString('base64'));
  assert.ok(bright < 0.1, `a dead canvas covers ${(bright * 100).toFixed(0)}% of the dark page`);
  assert.ok(errors.some((line) => line.includes('WebGL context lost')), 'the lost context was not logged');
  await context.close();
}

const failures = {
  webgl: (context) => context.addInitScript(() => {
    const original = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function getContext(type, ...args) {
      return /webgl/.test(type) ? null : original.call(this, type, ...args);
    };
  }),
  wasm: (context) => context.route('**/swarm*.wasm', (route) => route.fulfill({ status: 404, body: '' })),
};
for (const [name, setup] of Object.entries(failures)) {
  const { context, page, errors } = await open({}, setup);
  assert.equal(await page.locator('html').getAttribute('data-webgl'), 'unavailable', `${name}: no fallback`);
  assert.equal(await page.locator('.ely-still').isVisible(), true, `${name}: still Ely hidden`);
  await inked(page);
  assert.ok(errors.some((line) => line.includes('ELYTerminal:')), `${name}: the failure was not logged`);
  await context.close();
}

await browser.close();
console.log('Intro, four scenes, motion, cadence, theme reform, reduced motion, its live and loading changes, a lost context, two fallbacks passed.');
