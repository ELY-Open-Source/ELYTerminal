import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { BRIGHT, FAINT, GLYPHS, SHAPES, STATES, lag, levels } from '../src/type.js';

const css = readFileSync(new URL('../src/orbs.css', import.meta.url), 'utf8');

function drawn(shape) {
  const grid = Array(9).fill('.');
  for (const dot of shape) grid[dot] = '#';
  return [0, 3, 6].map((row) => grid.slice(row, row + 3).join(''));
}

function cssLevels(state) {
  const dots = Array(9).fill(null);
  const rule = new RegExp(`^\\.orb\\[data-state="${state}"\\] i(?::is\\(([^)]*\\)[^{]*)\\))? \\{ opacity: ([\\d.]+); \\}$`, 'gm');
  for (const [, children, opacity] of css.matchAll(rule)) {
    const picked = children
      ? [...children.matchAll(/:nth-child\((\d)\)/g)].map(([, n]) => Number(n) - 1)
      : [...dots.keys()];
    for (const dot of picked) dots[dot] = Number(opacity);
  }
  return dots;
}

test('the error mark is an X drawn one stroke then the other', () => {
  assert.deepEqual(SHAPES.error, [0, 4, 8, 2, 6]);
  assert.deepEqual(drawn(SHAPES.error), ['#.#', '.#.', '#.#']);
});

test('the done mark is a check from its left end', () => {
  assert.deepEqual(SHAPES.done, [3, 7, 5, 2]);
  assert.deepEqual(drawn(SHAPES.done), ['..#', '#.#', '.#.']);
});

test('the idle mark is the ring clockwise from the top, dimmer', () => {
  assert.deepEqual(SHAPES.idle, [1, 2, 5, 8, 7, 6, 3, 0]);
  assert.deepEqual(drawn(SHAPES.idle), ['###', '#.#', '###']);
  assert.ok(BRIGHT.idle < BRIGHT.done);
});

test('work ripples from the centre to the edges then the corners', () => {
  assert.deepEqual([0, 1, 2, 3, 4, 5, 6, 7, 8].map(lag), [424, 300, 424, 300, 0, 300, 424, 300, 424]);
});

test('still levels match the app frame by frame', () => {
  assert.deepEqual(levels('working'), [0.48, 0.68, 0.48, 0.68, 1, 0.68, 0.48, 0.68, 0.48]);
  assert.deepEqual(levels('blocked'), Array(9).fill(1));
  assert.deepEqual(levels('error'), [1, FAINT, 1, FAINT, 1, FAINT, 1, FAINT, 1]);
  assert.deepEqual(levels('done'), [FAINT, FAINT, 1, 1, FAINT, 1, FAINT, 1, FAINT]);
  assert.deepEqual(levels('idle'), [0.6, 0.6, 0.6, 0.6, FAINT, 0.6, 0.6, 0.6, 0.6]);
  assert.throws(() => levels('waiting'), /Type motion: unknown mark state "waiting"/);
});

test('orbs.css draws every still shape as levels() does', () => {
  for (const state of STATES) assert.deepEqual(cssLevels(state), levels(state), state);
});

test('scramble glyphs are terminal symbols innerHTML cannot misread', () => {
  assert.doesNotMatch(GLYPHS, /[<>&\sA-Za-z0-9]/);
  assert.ok(GLYPHS.endsWith('-'), 'a dash anywhere else is a range');
});
