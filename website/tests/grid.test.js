import assert from 'node:assert/strict';
import test from 'node:test';
import { ATLAS, RAMP, cellsFromPixels, edgeGlyph } from '../src/grid.js';

const glyph = (index) => ATLAS[index];

// Paint sub-sampled pixels for each cell a predicate picks.
function pixels(cols, rows, sub, color) {
  const data = new Uint8ClampedArray(cols * sub * rows * sub * 4);
  for (let y = 0; y < rows * sub; y += 1) {
    for (let x = 0; x < cols * sub; x += 1) {
      const rgba = color(x / sub, y / sub);
      if (rgba) data.set(rgba, (y * cols * sub + x) * 4);
    }
  }
  return data;
}

test('edge strokes follow the tangent in screen space', () => {
  assert.equal(glyph(edgeGlyph(0, 1)), '-');
  assert.equal(glyph(edgeGlyph(1, 0)), '|');
  assert.equal(glyph(edgeGlyph(1, -1)), '\\');
  assert.equal(glyph(edgeGlyph(1, 1)), '/');
});

test('a filled square draws its outline and a dense inside', () => {
  const white = [255, 255, 255, 255];
  const data = pixels(9, 9, 2, (x, y) => (x >= 2 && x < 7 && y >= 2 && y < 7 ? white : null));
  const { glyphs, colors } = cellsFromPixels(data, 9, 9, 2, 0, 1);
  const at = (col, row) => glyph(glyphs[row * 9 + col]);
  assert.equal(at(4, 2), '-');
  assert.equal(at(4, 6), '-');
  assert.equal(at(2, 4), '|');
  assert.equal(at(6, 4), '|');
  assert.equal(at(4, 4), '@');
  assert.equal(at(0, 0), ' ');
  assert.equal(glyphs[0], 0);
  assert.deepEqual([...colors.slice(40 * 3, 40 * 3 + 3)], [1, 1, 1]);
});

test('a diagonal edge takes a slanted stroke', () => {
  const pink = [240, 90, 151, 255];
  const data = pixels(12, 12, 2, (x, y) => (x + y < 12 ? pink : null));
  const { glyphs } = cellsFromPixels(data, 12, 12, 2, 0, 1);
  assert.equal(glyph(glyphs[5 * 12 + 6]), '/');
});

test('contrast with the ground sets density, so a dark fill thins on a dark ground', () => {
  const dark = [40, 36, 60, 255];
  const data = pixels(9, 9, 2, () => dark);
  const onDark = cellsFromPixels(data, 9, 9, 2, 0.02, 1).glyphs[40];
  const onLight = cellsFromPixels(data, 9, 9, 2, 0.95, 1).glyphs[40];
  assert.equal(onDark, RAMP.indexOf('-'));
  assert.equal(onLight, RAMP.indexOf('%'));
});

test('a cell under the solid threshold stays empty', () => {
  const faint = [255, 255, 255, 40];
  const { glyphs } = cellsFromPixels(pixels(4, 4, 2, () => faint), 4, 4, 2, 0, 1);
  assert.ok(glyphs.every((value) => value === 0));
});

test('a buffer of the wrong size fails', () => {
  assert.throws(() => cellsFromPixels(new Uint8ClampedArray(8), 4, 4, 2, 0, 1), /Glyph grid/);
});

test('a seam between two fills draws one stroke, not two', () => {
  const data = pixels(10, 10, 2, (x) => (x < 5 ? [255, 255, 255, 255] : [90, 90, 90, 255]));
  const { glyphs } = cellsFromPixels(data, 10, 10, 2, 0, 1);
  const row = [...glyphs.slice(40, 50)].map(glyph).join('');
  assert.equal(row.slice(1, 9).split('|').length - 1, 1);
});

test('a shade darkens the colors and sets density by the shaded contrast', () => {
  const pale = [236, 134, 181, 255];
  const data = pixels(9, 9, 2, () => pale);
  const plain = cellsFromPixels(data, 9, 9, 2, 0.93, 1);
  const shaded = cellsFromPixels(data, 9, 9, 2, 0.93, 1, 0.5);
  assert.ok(shaded.glyphs[40] > plain.glyphs[40]);
  assert.equal(shaded.colors[40 * 3], plain.colors[40 * 3] * 0.5);
});
