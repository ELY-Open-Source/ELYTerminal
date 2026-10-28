import { OrthographicCamera, Scene, WebGLRenderer } from 'three';
import { ASPECT, COLS, POSES, ROWS, SCENES, figure, sampleEly, sceneProgress } from './ely.js';
import { createGlyphs } from './glyphs.js';
import { cellsFromPixels } from './grid.js';
import { createPhysics } from './physics.js';

const CELLS = COLS * ROWS;
const SHIFT = 2749;
const SWITCH = 0.58;
const POINTER = 90;
const DUST_SIZE = 12;
const DUST_GLYPHS = [1, 2, 3, 5, 6];
const INTRO_HOLD = 1.9;
const LIGHT_SHADE = 0.5;
const LIGHT_DUST = 0.45;

function token(name) {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  const match = /^#([0-9a-f]{6})$/i.exec(value);
  if (!match) throw new Error(`Stage: ${name} is "${value}", not #rrggbb`);
  const number = parseInt(match[1], 16);
  return [(number >> 16) & 255, (number >> 8) & 255, number & 255].map((part) => part / 255);
}

const luminance = ([r, g, b]) => 0.2126 * r + 0.7152 * g + 0.0722 * b;

// The h1's letters as glyph cells in viewport pixels.
async function letterCells(heading, ground) {
  const size = parseFloat(getComputedStyle(heading).fontSize);
  const cell = Math.max(6, size / 11);
  const width = cell * ASPECT;
  const box = heading.getBoundingClientRect();
  const cols = Math.ceil(box.width / width);
  const rows = Math.ceil(box.height / cell);
  const sub = 4;
  const canvas = document.createElement('canvas');
  canvas.width = cols * sub;
  canvas.height = rows * sub;
  const context = canvas.getContext('2d', { willReadFrequently: true });
  context.setTransform(sub / width, 0, 0, sub / cell, 0, 0);
  for (const span of heading.children) {
    const style = getComputedStyle(span);
    const rect = span.getBoundingClientRect();
    context.font = `${style.fontWeight} ${style.fontSize} ${style.fontFamily}`;
    context.fillStyle = getComputedStyle(heading).getPropertyValue(span.classList.contains('ely') ? '--pink' : '--ink');
    context.textBaseline = 'alphabetic';
    const ascent = context.measureText(span.textContent).fontBoundingBoxAscent;
    context.fillText(span.textContent, rect.left - box.left, rect.top - box.top + ascent);
  }
  const { data } = context.getImageData(0, 0, canvas.width, canvas.height);
  const { glyphs, colors } = cellsFromPixels(data, cols, rows, sub, ground, ASPECT);
  const cells = [];
  for (const [index, glyph] of glyphs.entries()) {
    if (glyph === 0) continue;
    cells.push({
      x: box.left + (index % cols + 0.5) * width,
      y: box.top + (Math.floor(index / cols) + 0.5) * cell,
      glyph,
      color: colors.slice(index * 3, index * 3 + 3),
    });
  }
  return { cells, size: cell };
}

export async function createStage(container, svg, heading) {
  const root = document.documentElement;
  const motion = matchMedia('(prefers-reduced-motion: reduce)');
  const slots = SCENES.map((id) => {
    const slot = document.querySelector(`#${id} .ely-slot`);
    if (!slot) throw new Error(`Stage: #${id} has no .ely-slot`);
    return slot;
  });
  const response = fetch(new URL('./swarm.wasm', import.meta.url));
  const { instance } = await WebAssembly.instantiateStreaming(response);
  const count = CELLS + (innerWidth <= 768 ? 500 : 2400);
  const physics = createPhysics(instance, count, 0x51ee7);
  const glyphs = await createGlyphs(physics.output, count);
  // Read after the last await; the listener below follows.
  let reduced = motion.matches;
  const renderer = new WebGLRenderer({ antialias: false, alpha: true, powerPreference: 'high-performance' });
  renderer.setClearColor(0x000000, 0);
  container.append(renderer.domElement);
  const scene = new Scene();
  scene.add(glyphs.mesh);
  const camera = new OrthographicCamera();
  glyphs.uniforms.uScramble.value = reduced ? 0 : 1;

  const dust = Array.from({ length: count }, () => ({
    x: Math.random(),
    y: Math.random(),
    glyph: DUST_GLYPHS[Math.floor(Math.random() * DUST_GLYPHS.length)],
    alpha: 0.1 + Math.random() * 0.18,
  }));
  let palette;
  let width = 0;
  let height = 0;
  let anchors = [];
  let shown = 0;
  let current = -1;
  let requests = 0;
  let pointer = [0, 0, 0];
  let failed = false;
  let intro = false;
  let drawn = '';

  const write = (index, home, glyph, size, color, alpha) => {
    physics.homes.set(home, index * 3);
    glyphs.glyph.array.set([glyph, size], index * 2);
    glyphs.color.array.set([...color, alpha], index * 4);
  };
  const toDust = (index) => {
    const mote = dust[index];
    write(index, [mote.x, mote.y, 0], mote.glyph, DUST_SIZE, palette.ink, mote.alpha * palette.dust);
  };
  const touched = () => {
    glyphs.glyph.needsUpdate = true;
    glyphs.color.needsUpdate = true;
    drawn = '';
  };

  function assign(cells, shift) {
    for (let index = 0; index < count; index += 1) {
      const slot = (index + shift) % count;
      if (slot >= CELLS || cells.glyphs[slot] === 0) {
        toDust(index);
        continue;
      }
      const color = cells.colors.slice(slot * 3, slot * 3 + 3);
      write(index, [slot % COLS + 0.5, Math.floor(slot / COLS) + 0.5, 1], cells.glyphs[slot], 0, color, 1);
    }
    touched();
  }

  function fail(error) {
    failed = true;
    console.error('ELYTerminal: swarm stopped', error);
    root.dataset.webgl = 'unavailable';
    root.dataset.swarm = 'ready';
  }

  function resample() {
    const index = current;
    const request = ++requests;
    sampleEly(svg, POSES[index], palette.ground, palette.light ? LIGHT_SHADE : 1).then((cells) => {
      if (request !== requests || intro) return;
      assign(cells, (index * SHIFT) % count);
      console.info(`ELYTerminal: Ely formed for ${SCENES[index]}`);
    }).catch(fail);
  }

  // The current scene's slot, live, in viewport pixels.
  function place() {
    const rect = slots[current].getBoundingClientRect();
    if (!rect.height) throw new Error(`Stage: the ${SCENES[current]} slot has no height`);
    return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2, height: rect.height };
  }

  function enter(index) {
    current = index;
    root.dataset.scene = SCENES[index];
    const { x, y } = place();
    if (!reduced && !intro) physics.shock(x, y, 900, Math.max(width, height) * 0.5);
    resample();
  }

  function readPalette() {
    const ground = token('--ground');
    const light = root.dataset.theme === 'light';
    palette = { ground: luminance(ground), ink: token('--ink'), light, dust: light ? LIGHT_DUST : 1 };
  }

  function measure() {
    width = innerWidth;
    height = innerHeight;
    renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
    renderer.setSize(width, height, false);
    glyphs.uniforms.uViewport.value.set(width, height);
    const end = document.documentElement.scrollHeight - innerHeight;
    anchors = SCENES.map((id) => {
      const section = document.getElementById(id);
      if (!section) throw new Error(`Stage: missing #${id}`);
      return Math.min(section.getBoundingClientRect().top + scrollY, end);
    });
    drawn = '';
  }

  async function play() {
    intro = true;
    root.dataset.swarm = 'intro';
    const letters = await letterCells(heading, palette.ground);
    if (!intro) return;
    const order = Array.from({ length: count }, (_, index) => index).sort(() => Math.random() - 0.5);
    for (const [rank, index] of order.entries()) {
      const letter = letters.cells[rank];
      if (letter) write(index, [letter.x, letter.y, 2], letter.glyph, letters.size, letter.color, 1);
      else toDust(index);
      const x = letter ? letter.x + (Math.random() - 0.5) * 60 : Math.random() * width;
      physics.output.set([x, -20 - Math.random() * height, 0], index * 3);
    }
    touched();
    console.info(`ELYTerminal: intro spells ${letters.cells.length} cells`);
    addEventListener('scroll', finish, { passive: true });
    setTimeout(finish, INTRO_HOLD * 1000);
  }

  function finish() {
    if (!intro) return;
    intro = false;
    removeEventListener('scroll', finish);
    root.dataset.swarm = 'ready';
    const box = heading.getBoundingClientRect();
    if (!reduced) physics.shock(box.left + box.width / 2, box.top + box.height / 2, 700, box.width * 0.6);
    resample();
  }

  function update(delta, now) {
    if (failed) return;
    try {
      draw(delta, now);
    } catch (error) {
      fail(error);
    }
  }

  function draw(delta, now) {
    const target = sceneProgress(scrollY, anchors);
    shown = reduced ? target : shown + (target - shown) * (1 - Math.exp(-delta * 10));
    const nearest = Math.round(shown);
    if (nearest !== current && Math.abs(shown - current) >= SWITCH) enter(nearest);
    const shape = figure(place());
    glyphs.uniforms.uCell.value = shape.cellHeight;
    const frame = [shape.x, shape.y, shape.cellWidth, shape.cellHeight, width, height];
    if (reduced) {
      const key = frame.join();
      if (key === drawn) return;
      drawn = key;
      physics.snap(...frame, 0);
    } else {
      glyphs.uniforms.uTime.value = now / 1000;
      physics.step(delta, now / 1000, ...frame, ...pointer);
    }
    glyphs.state.needsUpdate = true;
    renderer.render(scene, camera);
  }

  addEventListener('pointermove', (event) => {
    if (event.pointerType === 'mouse' || event.buttons) pointer = [event.clientX, event.clientY, POINTER];
  }, { passive: true });
  document.addEventListener('pointerleave', () => { pointer = [0, 0, 0]; });
  addEventListener('pointerdown', (event) => {
    pointer = [event.clientX, event.clientY, POINTER];
    if (!reduced) physics.shock(event.clientX, event.clientY, 1100, 240);
  }, { passive: true });
  for (const type of ['pointerup', 'pointercancel']) {
    addEventListener(type, (event) => {
      if (event.pointerType !== 'mouse') pointer = [0, 0, 0];
    }, { passive: true });
  }
  motion.addEventListener('change', () => {
    reduced = motion.matches;
    glyphs.uniforms.uScramble.value = reduced ? 0 : 1;
    pointer = [0, 0, 0];
    drawn = '';
    if (reduced) finish();
    console.info(`ELYTerminal: swarm motion ${reduced ? 'reduced' : 'full'}`);
  });
  renderer.domElement.addEventListener('webglcontextlost', () => fail(new Error('WebGL context lost')));
  new MutationObserver(() => {
    readPalette();
    if (intro) return;
    resample();
  }).observe(root, { attributes: true, attributeFilter: ['data-theme'] });
  new ResizeObserver(measure).observe(document.body);

  readPalette();
  measure();
  current = Math.round(sceneProgress(scrollY, anchors));
  shown = current;
  root.dataset.scene = SCENES[current];
  root.dataset.webgl = 'ready';
  if (reduced || scrollY > 0) {
    root.dataset.swarm = 'ready';
    for (let index = 0; index < count; index += 1) {
      physics.output.set([Math.random() * width, Math.random() * height, 0], index * 3);
    }
    resample();
  } else {
    await play();
  }
  console.info(`ELYTerminal: stage ready, ${reduced ? 'reduced motion' : 'full motion'}`);
  return { update };
}
