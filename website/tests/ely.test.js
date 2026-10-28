import assert from 'node:assert/strict';
import test from 'node:test';
import { COLS, ROWS, ASPECT, figure, sceneProgress } from '../src/ely.js';

const anchors = [0, 900, 1800, 2700];

test('scroll progress follows the four document scenes', () => {
  assert.equal(sceneProgress(-10, anchors), 0);
  assert.equal(sceneProgress(0, anchors), 0);
  assert.equal(sceneProgress(450, anchors), 0.5);
  assert.equal(sceneProgress(1800, anchors), 2);
  assert.equal(sceneProgress(4000, anchors), 3);
});

test('the figure is centred on its place with cells of the glyph aspect', () => {
  const shape = figure({ x: 500, y: 400, height: ROWS * 10 });
  assert.equal(shape.cellHeight, 10);
  assert.equal(shape.cellWidth, 10 * ASPECT);
  assert.equal(shape.x + (COLS * shape.cellWidth) / 2, 500);
  assert.equal(shape.y + (ROWS * shape.cellHeight) / 2, 400);
});
