import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { createPhysics } from '../src/physics.js';

const module = await WebAssembly.compile(readFileSync(new URL('../src/swarm.wasm', import.meta.url)));
const [WIDTH, HEIGHT] = [1280, 800];
const FRAME = [100, 50, 9, 18, WIDTH, HEIGHT];
const DT = 1 / 60;
const WANDER = 16;
const MAX_SPEED = 4000;

async function load(count, seed = 1) {
  return createPhysics(await WebAssembly.instantiate(module), count, seed);
}

function random(seed) {
  let state = seed;
  return () => {
    state = (state * 1664525 + 1013904223) >>> 0;
    return state / 2 ** 32;
  };
}

function run(physics, seconds, { time = 0, dt = DT, pointer = [0, 0, 0], each } = {}) {
  for (let i = 0; i < Math.round(seconds / dt); i += 1) {
    time += dt;
    physics.step(dt, time, ...FRAME, ...pointer);
    each?.(time);
  }
  return time;
}

function target(physics, i) {
  const [x, y, kind] = physics.homes.subarray(i * 3, i * 3 + 3);
  if (kind === 1) return [FRAME[0] + x * FRAME[2], FRAME[1] + y * FRAME[3]];
  if (kind === 2) return [x, y];
  throw new Error(`test: particle ${i} has kind ${kind}, which has no fixed target`);
}

function distance(physics, i, [x, y]) {
  return Math.hypot(physics.output[i * 3] - x, physics.output[i * 3 + 1] - y);
}

test('cell particles converge on origin plus home times cell, staggered', async () => {
  const physics = await load(120);
  for (let i = 0; i < physics.count; i += 1) {
    physics.homes.set([i % 12, Math.floor(i / 12), 1], i * 3);
    const [x, y] = target(physics, i);
    physics.output.set([x, y - 600], i * 3);
  }
  const spread = () => {
    const gaps = Array.from({ length: physics.count }, (_, i) => distance(physics, i, target(physics, i)));
    return Math.max(...gaps) - Math.min(...gaps);
  };
  const time = run(physics, 0.3);
  assert.ok(spread() > 20, `arrivals are staggered, spread ${spread()}`);
  run(physics, 2.7, { time });
  for (let i = 0; i < physics.count; i += 1) {
    assert.ok(distance(physics, i, target(physics, i)) < 1, `cell ${i} is home`);
  }
});

test('kind 2 particles converge on absolute pixels', async () => {
  const physics = await load(200);
  const next = random(2);
  for (let i = 0; i < physics.count; i += 1) physics.homes.set([next() * WIDTH, next() * HEIGHT, 2], i * 3);
  run(physics, 3);
  for (let i = 0; i < physics.count; i += 1) {
    assert.ok(distance(physics, i, target(physics, i)) < 1, `particle ${i} is home`);
  }
});

test('dust wanders inside the viewport plus its wander amplitude', async () => {
  const physics = await load(500);
  const next = random(3);
  for (let i = 0; i < physics.count; i += 1) {
    physics.homes.set([next(), next(), 0], i * 3);
    physics.output.set([next() * WIDTH, next() * HEIGHT], i * 3);
  }
  const low = Array(physics.count).fill(Infinity);
  const high = Array(physics.count).fill(-Infinity);
  run(physics, 20, {
    each: (time) => {
      for (let i = 0; i < physics.count; i += 1) {
        const [x, y] = physics.output.subarray(i * 3, i * 3 + 2);
        assert.ok(x >= -WANDER - 0.5 && x <= WIDTH + WANDER + 0.5, `dust ${i} x ${x} at ${time}`);
        assert.ok(y >= -WANDER - 0.5 && y <= HEIGHT + WANDER + 0.5, `dust ${i} y ${y} at ${time}`);
        if (time > 8) {
          low[i] = Math.min(low[i], x);
          high[i] = Math.max(high[i], x);
        }
      }
    },
  });
  const sway = high.reduce((sum, value, i) => sum + value - low[i], 0) / physics.count;
  assert.ok(sway > 8 && sway < 2 * WANDER, `dust sways about ${WANDER} px, mean ${sway}`);
});

test('snap places particles on their targets at rest', async () => {
  const physics = await load(30);
  const next = random(4);
  for (let i = 0; i < physics.count; i += 1) physics.homes.set([next() * 40, next() * 30, i % 3], i * 3);
  run(physics, 0.5);
  physics.shock(400, 300, 2000, 900);
  physics.snap(...FRAME, 12);
  for (let i = 0; i < physics.count; i += 1) {
    const [x, y, speed] = physics.output.subarray(i * 3, i * 3 + 3);
    assert.equal(speed, 0);
    if (i % 3 === 0) {
      assert.ok(Math.abs(x - physics.homes[i * 3] * WIDTH) <= WANDER, `dust ${i} x`);
      assert.ok(Math.abs(y - physics.homes[i * 3 + 1] * HEIGHT) <= WANDER, `dust ${i} y`);
    } else {
      assert.ok(distance(physics, i, target(physics, i)) < 1e-3, `particle ${i} sits on its target`);
    }
  }
  physics.step(DT, 12, ...FRAME, 0, 0, 0);
  for (let i = 1; i < physics.count; i += 3) {
    assert.ok(distance(physics, i, target(physics, i)) < 1e-3, `cell ${i} stays at rest`);
  }
});

test('the pointer opens a hole that closes once it leaves', async () => {
  const physics = await load(2);
  physics.homes.set([300, 300, 2, 600, 300, 2]);
  physics.snap(...FRAME, 0);
  const radius = 90;
  const time = run(physics, 2, { pointer: [300, 300, radius] });
  const pushed = distance(physics, 0, [300, 300]);
  assert.ok(pushed > 0.7 * radius && pushed < radius, `pushed to ${pushed} of ${radius}`);
  assert.ok(distance(physics, 1, [600, 300]) < 1e-3, 'a particle outside the radius stays put');
  run(physics, 3, { time });
  assert.ok(distance(physics, 0, [300, 300]) < 1, 'the particle returns home');
});

test('shock raises speed near its center and nowhere past its radius', async () => {
  const offsets = [0, 20, 60, 125, 400];
  const physics = await load(offsets.length);
  offsets.forEach((offset, i) => physics.homes.set([500 + offset, 500, 2], i * 3));
  physics.snap(...FRAME, 0);
  physics.shock(500, 500, 1500, 120);
  const speeds = offsets.map((_, i) => physics.output[i * 3 + 2]);
  assert.ok(Math.abs(speeds[0] - 1500) < 1, `center ${speeds[0]}`);
  assert.ok(speeds[1] > 1000, `near ${speeds[1]}`);
  assert.ok(speeds[2] > 300 && speeds[2] < 450, `halfway ${speeds[2]}`);
  assert.equal(speeds[3], 0);
  assert.equal(speeds[4], 0);
  physics.step(DT, DT, ...FRAME, 0, 0, 0);
  assert.ok(physics.output[3] > 520, 'the near particle flies outward');
  assert.ok(physics.output[3 * 3 + 2] < 1e-3, 'just past the radius stays still');
});

test('a dt of 0.05 stays finite, bounded and settles', async () => {
  const physics = await load(300);
  const next = random(5);
  for (let i = 0; i < physics.count; i += 1) {
    physics.homes.set(i % 3 === 0 ? [next(), next(), 0] : [next() * 60, next() * 40, i % 3], i * 3);
    physics.output.set([(next() - 0.5) * 6000, (next() - 0.5) * 6000], i * 3);
  }
  const check = () => {
    for (const value of physics.output) assert.ok(Number.isFinite(value), 'output is finite');
    for (let i = 0; i < physics.count; i += 1) assert.ok(physics.output[i * 3 + 2] <= MAX_SPEED + 1, 'speed is clamped');
  };
  let time = 0;
  for (let i = 0; i < 400; i += 1) {
    time += 0.05;
    physics.step(0.05, time, ...FRAME, 300 + 200 * Math.sin(time), 300, 90);
    if (i % 40 === 0) physics.shock(400, 300, 3000, 200);
    check();
  }
  run(physics, 5, { time, dt: 0.05, each: check });
  for (let i = 0; i < physics.count; i += 1) {
    if (i % 3 !== 0) assert.ok(distance(physics, i, target(physics, i)) < 1, `particle ${i} settles`);
  }
});

test('a long step splits into substeps of at most 1/120 s', async () => {
  const play = async (steps) => {
    const physics = await load(60);
    const next = random(7);
    for (let i = 0; i < physics.count; i += 1) {
      physics.homes.set([next(), next(), i % 3], i * 3);
      physics.output.set([next() * 900, next() * 900], i * 3);
    }
    for (let i = 0; i < steps; i += 1) physics.step(0.05 / steps, 1, ...FRAME, 300, 300, 90);
    return Array.from(physics.output);
  };
  const whole = await play(1);
  const split = await play(6);
  whole.forEach((value, i) => assert.ok(Math.abs(value - split[i]) < 1e-2, `value ${i}: ${value} vs ${split[i]}`));
});

test('the same seed gives the same motion, another seed another', async () => {
  const play = async (seed) => {
    const physics = await load(100, seed);
    const next = random(6);
    for (let i = 0; i < physics.count; i += 1) physics.homes.set([next(), next(), i % 3], i * 3);
    let time = run(physics, 1, { pointer: [200, 200, 90] });
    physics.shock(300, 300, 1200, 150);
    time = run(physics, 1, { time });
    return Array.from(physics.output);
  };
  const first = await play(7);
  assert.deepEqual(await play(7), first);
  assert.notDeepEqual(await play(8), first);
});

test('the swarm fails fast on misuse', async () => {
  const instance = await WebAssembly.instantiate(module);
  assert.throws(() => createPhysics(instance, 0, 1), /swarm: count 0/);
  assert.throws(() => createPhysics(instance, 1.5, 1), /swarm: count 1.5/);
  const physics = createPhysics(instance, 4, 1);
  assert.throws(() => createPhysics(instance, 4, 1), WebAssembly.RuntimeError);
  assert.throws(() => physics.step(1, 0, ...FRAME, 0, 0, 0), WebAssembly.RuntimeError);
  physics.homes[2] = 3;
  assert.throws(() => physics.step(DT, 0, ...FRAME, 0, 0, 0), WebAssembly.RuntimeError);
});
