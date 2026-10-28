import assert from 'node:assert/strict';
import { chromium } from '@playwright/test';

const base = process.env.BASE_URL || 'http://127.0.0.1:5173';
const cases = [[1440, 900], [1280, 720], [390, 844], [320, 568], [844, 390], [1280, 600]];
const browser = await chromium.launch({ channel: 'chrome', headless: true });

for (const [width, height] of cases) {
  const context = await browser.newContext({ viewport: { width, height }, colorScheme: 'dark', reducedMotion: 'reduce' });
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto(base, { waitUntil: 'networkidle' });
  for (const scene of ['terminal', 'sessions', 'git', 'download']) {
    const box = await page.evaluate((id) => {
      const section = document.getElementById(id);
      const rect = (element) => element.getBoundingClientRect();
      const parts = [...section.querySelectorAll('h1, h2, .line, .sessions li, .panel:not([hidden]) p, .tree, .download-primary, .download-alternatives a')];
      return {
        parts: parts.length,
        left: Math.min(...parts.map((part) => rect(part).left)),
        right: Math.max(...parts.map((part) => rect(part).right)),
        bottom: Math.max(...parts.map((part) => rect(part).bottom)),
        sectionBottom: rect(section).bottom,
        overflow: document.documentElement.scrollWidth - innerWidth,
      };
    }, scene);
    assert.ok(box.parts > 0, `${width} ${scene}: empty scene`);
    assert.equal(box.overflow, 0, `${width} ${scene}: page overflow`);
    assert.ok(box.left >= 0 && box.right <= width, `${width} ${scene}: text clipped ${box.left}..${box.right}`);
    assert.ok(box.bottom <= box.sectionBottom, `${width} ${scene}: text leaves its section`);
  }
  await page.locator('#files-tab').scrollIntoViewIfNeeded();
  await page.locator('#files-tab').click();
  assert.equal(await page.locator('#files-panel').isVisible(), true);
  assert.equal(await page.locator('#git-panel').isVisible(), false);
  assert.equal(await page.locator('#files-tab').getAttribute('aria-selected'), 'true');
  await page.locator('#files-tab').press('ArrowLeft');
  assert.equal(await page.locator('#git-panel').isVisible(), true);
  assert.equal(await page.locator('#git-tab').evaluate((tab) => tab === document.activeElement), true);
  await page.locator('#git-tab').press('End');
  assert.equal(await page.locator('#files-panel').isVisible(), true);
  assert.equal(await page.locator('.sessions li').count(), 5);
  assert.equal(await page.locator('.sessions [aria-current] strong').textContent(), 'codex');
  const translations = await page.locator('main [data-en][data-zh]').evaluateAll((elements) => (
    elements.map((element) => [element.dataset.en, element.dataset.zh])
  ));
  for (const pair of [['Sessions', '会话'], ['Git / Files', 'Git / 文件'], ['Files', '文件'], ['Unstaged', '未暂存'], ['Staged', '已暂存'], ['Download', '下载']]) {
    assert.ok(translations.some(([en, zh]) => en === pair[0] && zh === pair[1]), `missing translation: ${pair[0]}`);
  }
  assert.ok(translations.every(([en, zh]) => en.trim() && zh.trim()), 'a translation is blank');
  assert.deepEqual(errors, []);
  await context.close();
}

// No text line may cross a slot Ely draws in.
for (const [width, height] of cases) {
  for (const language of ['en', 'zh']) {
    for (const theme of ['dark', 'light']) {
      const context = await browser.newContext({ viewport: { width, height }, colorScheme: theme, reducedMotion: 'reduce' });
      await context.addInitScript((choice) => localStorage.setItem('elyterminal-language', choice), language);
      const page = await context.newPage();
      await page.goto(base, { waitUntil: 'networkidle' });
      await page.waitForFunction(() => document.documentElement.dataset.webgl === 'ready', null, { timeout: 15000 });
      for (const tab of ['#git-tab', '#files-tab']) {
        await page.locator(tab).click();
        const hits = await page.evaluate(() => {
          const found = [];
          const slots = [...document.querySelectorAll('.ely-slot')].map((slot) => [slot.closest('section').id, slot.getBoundingClientRect()]);
          const walker = document.createTreeWalker(document.querySelector('main'), NodeFilter.SHOW_TEXT);
          for (let node = walker.nextNode(); node; node = walker.nextNode()) {
            if (!node.textContent.trim() || !node.parentElement.checkVisibility()) continue;
            const range = document.createRange();
            range.selectNodeContents(node);
            for (const line of range.getClientRects()) {
              for (const [scene, slot] of slots) {
                const x = Math.min(line.right, slot.right) - Math.max(line.left, slot.left);
                const y = Math.min(line.bottom, slot.bottom) - Math.max(line.top, slot.top);
                if (x > 0.5 && y > 0.5) found.push(`${scene}: "${node.textContent.trim().slice(0, 30)}"`);
              }
            }
          }
          return found;
        });
        assert.deepEqual(hits, [], `${width}x${height} ${language} ${theme} ${tab}: text inside Ely's slot`);
      }
      await context.close();
    }
  }
}

// Text that scrolls under the header stays hidden.
for (const [width, height] of [[320, 568], [844, 390]]) {
  for (const theme of ['dark', 'light']) {
    const context = await browser.newContext({ viewport: { width, height }, deviceScaleFactor: 1, colorScheme: theme, reducedMotion: 'reduce' });
    const page = await context.newPage();
    await page.goto(base, { waitUntil: 'networkidle' });
    await page.waitForFunction(() => document.documentElement.dataset.swarm === 'ready', null, { timeout: 15000 });
    const band = await page.locator('.site-header').boundingBox();
    const clip = { x: 0, y: 0, width, height: Math.ceil(band.height) };
    const end = await page.evaluate(() => document.documentElement.scrollHeight - innerHeight);
    for (let y = 0; y <= end; y += 160) {
      await page.evaluate((top) => scrollTo(0, top), y);
      await page.waitForTimeout(150);
      const shown = await page.screenshot({ clip });
      const bare = await page.screenshot({ clip, style: 'main { opacity: 0 !important; }' });
      assert.ok(shown.equals(bare), `${width}x${height} ${theme} y=${y}: text shows through the header`);
    }
    await context.close();
  }
}

await browser.close();
console.log('Six viewports, four scenes in bounds, tabs, translations, text clear of Ely and the header passed.');
