import { animate, cleanInlineStyles, createTimeline, scrambleText, stagger, steps } from 'animejs';

export const STATES = ['working', 'blocked', 'error', 'done', 'idle'];
export const SHAPES = {
  error: [0, 4, 8, 2, 6],
  done: [3, 7, 5, 2],
  idle: [1, 2, 5, 8, 7, 6, 3, 0],
};
export const BRIGHT = { error: 1, done: 1, idle: 0.6 };
export const FAINT = 0.12;
export const DIM = 0.35;
export const DIP = 0.25;
export const STEP = 80;
export const RIPPLE = 1200;
export const LAG = 300;
export const GLYPHS = '.:=+*#%@/\\|_~^$-';

const CHAR = 28;
const BEAT = 320;
const AFTER = 180;
const OUTPUT = 70;

// How far a dot's ripple trails the centre's, in ms.
export function lag(dot) {
  return Math.round(Math.hypot((dot % 3) - 1, Math.floor(dot / 3) - 1) * LAG);
}

function crest(time) {
  return DIM + (1 - DIM) * (0.5 + 0.5 * Math.cos((2 * Math.PI * time) / RIPPLE));
}

// Each dot's opacity once the mark stands still.
export function levels(state) {
  if (state === 'working') {
    return Array.from({ length: 9 }, (_, dot) => Math.round(crest(-lag(dot)) * 100) / 100);
  }
  if (state === 'blocked') return Array(9).fill(1);
  const shape = SHAPES[state];
  if (!shape) throw new Error(`Type motion: unknown mark state "${state}"`);
  return Array.from({ length: 9 }, (_, dot) => (shape.includes(dot) ? BRIGHT[state] : FAINT));
}

function checkHeading(heading) {
  if (heading.children.length) {
    throw new Error(`Type motion: heading "${heading.textContent}" must hold plain text`);
  }
}

function checkText(text) {
  if (/[<&]/.test(text)) {
    throw new Error(`Type motion: heading "${text}" holds < or &, which innerHTML would parse`);
  }
}

function checkTranscript(transcript) {
  const lines = transcript.querySelectorAll('.line');
  if (!lines.length) throw new Error('Type motion: a transcript has no lines');
  for (const line of lines) {
    const typed = line.querySelectorAll('.typed');
    if (line.classList.contains('command') ? typed.length !== 1 : typed.length) {
      throw new Error(`Type motion: line "${line.textContent}" needs one .typed exactly when it is a command`);
    }
  }
  return [...lines];
}

function checkOrb(orb) {
  if (!STATES.includes(orb.dataset.state)) {
    throw new Error(`Type motion: unknown mark state "${orb.dataset.state}"`);
  }
  const dots = [...orb.children];
  if (dots.length !== 9 || dots.some((dot) => dot.tagName !== 'I')) {
    throw new Error(`Type motion: a ${orb.dataset.state} mark needs nine <i> dots, found ${dots.length} children`);
  }
  return dots;
}

function once(threshold, play) {
  const observer = new IntersectionObserver((entries) => {
    for (const entry of entries) {
      if (!entry.isIntersecting) continue;
      observer.unobserve(entry.target);
      play(entry.target);
    }
  }, { threshold });
  return observer;
}

function createDecode(headings, reduce) {
  const running = new Map();
  const started = new Set();

  function settle(heading, text) {
    heading.textContent = text;
    heading.removeAttribute('aria-label');
    running.delete(heading);
  }

  function play(heading) {
    started.add(heading);
    const text = heading.textContent;
    checkText(text);
    if (reduce.matches) return;
    heading.setAttribute('aria-label', text);
    running.set(heading, animate(heading, {
      innerHTML: scrambleText({ text, chars: GLYPHS, revealRate: 30, settleDuration: 360 }),
      onComplete: () => settle(heading, text),
    }));
  }

  const observer = once(0.4, play);
  for (const heading of headings) observer.observe(heading);
  document.addEventListener('elyterminal:locale', () => {
    for (const heading of started) {
      running.get(heading)?.cancel();
      running.delete(heading);
      heading.removeAttribute('aria-label');
      play(heading);
    }
  });

  return {
    still() {
      for (const animation of [...running.values()]) animation.complete();
    },
  };
}

function createTyping(transcripts, reduce) {
  const lines = new Map(transcripts.map((transcript) => [transcript, checkTranscript(transcript)]));
  const running = new Map();
  const waiting = new Set();

  function finish(transcript) {
    const timeline = running.get(transcript);
    if (timeline) {
      timeline.cancel();
      cleanInlineStyles(timeline);
      running.delete(transcript);
    }
    waiting.delete(transcript);
    observer.unobserve(transcript);
    for (const line of transcript.querySelectorAll('[data-typing]')) delete line.dataset.typing;
    delete transcript.dataset.type;
  }

  function play(transcript) {
    waiting.delete(transcript);
    if (reduce.matches) return finish(transcript);
    transcript.dataset.type = 'typing';
    const timeline = createTimeline({ onComplete: () => finish(transcript) });
    for (const line of lines.get(transcript)) {
      if (!line.classList.contains('command')) {
        timeline.set(line, { opacity: 1 }, `+=${OUTPUT}`);
        continue;
      }
      const typed = line.querySelector('.typed');
      const count = [...typed.textContent].length;
      timeline
        .set(line, { opacity: 1 }, `+=${OUTPUT}`)
        .call(() => { line.dataset.typing = ''; })
        .add(typed, { '--typed': [0, 1], duration: count * CHAR, ease: steps(count) }, `+=${BEAT}`)
        .call(() => { delete line.dataset.typing; }, `+=${AFTER}`);
    }
    running.set(transcript, timeline);
  }

  const observer = once(0.25, play);
  if (!reduce.matches) {
    for (const transcript of transcripts) {
      transcript.dataset.type = 'pending';
      waiting.add(transcript);
      observer.observe(transcript);
    }
  }

  return {
    still() {
      for (const transcript of [...waiting, ...running.keys()]) finish(transcript);
    },
  };
}

function createMarks(orbs, reduce) {
  const dots = new Map(orbs.map((orb) => [orb, checkOrb(orb)]));
  const loops = [];
  const drawing = new Map();

  function loop(orb) {
    const lattice = dots.get(orb);
    if (orb.dataset.state === 'blocked') {
      loops.push(animate(lattice, {
        opacity: [
          { from: 1, to: DIP, duration: 2 * STEP },
          { to: 1, duration: 2 * STEP },
          { to: DIP, duration: 2 * STEP },
          { to: 1, duration: 14 * STEP },
        ],
        ease: steps(1, true),
        loop: true,
      }));
      return;
    }
    lattice.forEach((dot, index) => {
      const wave = animate(dot, {
        opacity: [
          { from: 1, to: DIM, duration: RIPPLE / 2 },
          { to: 1, duration: RIPPLE / 2 },
        ],
        ease: 'inOutSine',
        loop: true,
      });
      wave.seek((RIPPLE - lag(index)) % RIPPLE);
      loops.push(wave);
    });
  }

  function settle(orb) {
    const animation = drawing.get(orb);
    if (animation) {
      animation.cancel();
      cleanInlineStyles(animation);
      drawing.delete(orb);
    }
    observer.unobserve(orb);
    delete orb.dataset.mark;
  }

  function draw(orb) {
    const state = orb.dataset.state;
    const lattice = dots.get(orb);
    drawing.set(orb, animate(SHAPES[state].map((dot) => lattice[dot]), {
      opacity: [FAINT, BRIGHT[state]],
      duration: 2 * STEP,
      delay: stagger(STEP),
      ease: 'linear',
      onComplete: () => settle(orb),
    }));
  }

  const observer = once(0.6, draw);
  const settled = orbs.filter((orb) => SHAPES[orb.dataset.state]);
  const moving = orbs.filter((orb) => !SHAPES[orb.dataset.state]);

  function move() {
    for (const orb of moving) loop(orb);
  }

  if (!reduce.matches) {
    move();
    for (const orb of settled) {
      orb.dataset.mark = 'pending';
      observer.observe(orb);
    }
  }

  return {
    still() {
      for (const animation of loops.splice(0)) animation.revert();
      for (const orb of settled) settle(orb);
    },
    move,
  };
}

function required(selector) {
  const elements = [...document.querySelectorAll(selector)];
  if (!elements.length) throw new Error(`Type motion: no ${selector} on the page`);
  return elements;
}

export function createType() {
  const reduce = matchMedia('(prefers-reduced-motion: reduce)');
  const headings = required('[data-decode]');
  headings.forEach(checkHeading);
  const decode = createDecode(headings, reduce);
  const typing = createTyping(required('.transcript'), reduce);
  const marks = createMarks(required('.orb'), reduce);
  reduce.addEventListener('change', () => {
    if (!reduce.matches) return marks.move();
    decode.still();
    typing.still();
    marks.still();
  });
  console.info('ELYTerminal: type motion ready');
}
