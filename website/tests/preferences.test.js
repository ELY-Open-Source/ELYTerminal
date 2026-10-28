import assert from 'node:assert/strict';
import test from 'node:test';
import { detectLocale, resolveTheme } from '../src/preferences.js';

test('locale follows Chinese browser language and saved theme overrides system', () => {
  assert.equal(detectLocale('zh-CN'), 'zh');
  assert.equal(detectLocale('zh-Hans-CN'), 'zh');
  assert.equal(detectLocale('zh-TW'), 'zh');
  assert.equal(detectLocale('en-US'), 'en');
  assert.equal(detectLocale('fr-FR'), 'en');
  assert.equal(detectLocale(''), 'en');
  assert.equal(resolveTheme('light', 'dark'), 'light');
  assert.equal(resolveTheme('dark', 'light'), 'dark');
  assert.equal(resolveTheme(null, 'light'), 'light');
  assert.equal(resolveTheme('invalid', 'dark'), 'dark');
});
