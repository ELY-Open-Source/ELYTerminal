export function createFrameScheduler() {
  const updates = [];
  let previous = performance.now();

  function frame(now) {
    // A frame's timestamp may precede the scheduler's start.
    const delta = Math.min(Math.max(0, now - previous) / 1000, 0.05);
    previous = now;
    for (const update of updates) update(delta, now);
    requestAnimationFrame(frame);
  }

  requestAnimationFrame(frame);
  return {
    add(update) {
      updates.push(update);
    },
  };
}
