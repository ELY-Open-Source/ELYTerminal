const MAX_COUNT = 1 << 20;

export function createPhysics(instance, count, seed) {
  if (!Number.isInteger(count) || count < 1 || count > MAX_COUNT) {
    throw new Error(`swarm: count ${count} is not an integer in 1..${MAX_COUNT}`);
  }
  if (!Number.isInteger(seed) || seed < 0 || seed > 0xffffffff) {
    throw new Error(`swarm: seed ${seed} is not a 32-bit unsigned integer`);
  }
  const wasm = instance.exports;
  wasm.init(count, seed);
  const homes = new Float32Array(wasm.memory.buffer, wasm.homes(), count * 3);
  const output = new Float32Array(wasm.memory.buffer, wasm.output(), count * 3);
  console.info(`ELYTerminal: swarm ready, ${count} particles`);
  return { count, homes, output, step: wasm.step, shock: wasm.shock, snap: wasm.snap };
}
