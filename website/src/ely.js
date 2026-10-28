import { cellsFromPixels } from './grid.js';

export const COLS = 96;
export const ROWS = 80;
export const ASPECT = 0.6;
const CROP = { x: 80, y: 14, width: 352, height: 492 };

export const SCENES = ['terminal', 'sessions', 'git', 'download'];

export const POSES = [
  {
    rootX: 0, rootY: 0, rootTurn: 0, rootScale: 1, head: 0,
    gazeX: 0, gazeY: 0, wide: 1, leftHand: 0, rightHand: 0,
    bang: 0, sparkles: 0, mouth: 'smile', eyes: 'open',
  },
  {
    rootX: -8, rootY: 5, rootTurn: -2, rootScale: 1.02, head: -4,
    gazeX: 5, gazeY: 5, wide: 1, leftHand: -5, rightHand: 1,
    bang: 0, sparkles: 0, mouth: 'flat', eyes: 'open',
  },
  {
    rootX: 8, rootY: -2, rootTurn: 2, rootScale: 1, head: 6,
    gazeX: -4, gazeY: 2, wide: 1.12, leftHand: 1, rightHand: -3,
    bang: 1, sparkles: 0, mouth: 'gasp', eyes: 'open',
  },
  {
    rootX: 0, rootY: -16, rootTurn: 0, rootScale: 1.02, head: -3,
    gazeX: 0, gazeY: 0, wide: 1, leftHand: -5, rightHand: -5,
    bang: 0, sparkles: 1, mouth: 'grin', eyes: 'arcs',
  },
];

export function sceneProgress(scrollY, anchors) {
  if (scrollY <= anchors[0]) return 0;
  for (let index = 1; index < anchors.length; index += 1) {
    if (scrollY < anchors[index]) {
      return index - 1 + (scrollY - anchors[index - 1]) / (anchors[index] - anchors[index - 1]);
    }
  }
  return anchors.length - 1;
}

export function figure(place) {
  const cellHeight = place.height / ROWS;
  const cellWidth = cellHeight * ASPECT;
  return {
    x: place.x - (COLS * cellWidth) / 2,
    y: place.y - place.height / 2,
    cellWidth,
    cellHeight,
  };
}

function pivot(x, y, turn, scale, shiftX = 0, shiftY = 0) {
  return `translate(${shiftX} ${shiftY}) translate(${x} ${y}) rotate(${turn}) scale(${scale}) translate(${-x} ${-y})`;
}

function poseSvg(svg, pose) {
  const part = (id) => {
    const element = svg.querySelector(`#${id}`);
    if (!element) throw new Error(`Ely: missing #${id}`);
    return element;
  };
  part('ely-root').setAttribute('transform', pivot(256, 286, pose.rootTurn, pose.rootScale, pose.rootX, pose.rootY));
  const head = pivot(256, 236, pose.head, 1);
  part('hair-back').setAttribute('transform', head);
  part('head-front').setAttribute('transform', head);
  part('eye-left').setAttribute('transform', pivot(222, 192, 0, pose.wide));
  part('eye-right').setAttribute('transform', pivot(290, 192, 0, pose.wide));
  for (const id of ['gaze-left', 'gaze-right']) part(id).setAttribute('transform', `translate(${pose.gazeX} ${pose.gazeY})`);
  part('hand-left').setAttribute('transform', `translate(0 ${pose.leftHand})`);
  part('hand-right').setAttribute('transform', `translate(0 ${pose.rightHand})`);
  part('eyes-open').style.display = pose.eyes === 'open' ? '' : 'none';
  part('eyes-arcs').style.display = pose.eyes === 'arcs' ? '' : 'none';
  part('mark-bang').setAttribute('opacity', pose.bang);
  part('mark-sparkles').setAttribute('opacity', pose.sparkles);
  for (const mouth of ['smile', 'flat', 'gasp', 'grin']) {
    part(`mouth-${mouth}`).style.display = mouth === pose.mouth ? '' : 'none';
  }
}

// Pose a copy of the SVG and sample its glyphs.
export async function sampleEly(svg, pose, ground, shade, sub = 3) {
  const copy = svg.cloneNode(true);
  copy.setAttribute('viewBox', `${CROP.x} ${CROP.y} ${CROP.width} ${CROP.height}`);
  copy.setAttribute('width', COLS * sub);
  copy.setAttribute('height', ROWS * sub);
  copy.setAttribute('preserveAspectRatio', 'none');
  poseSvg(copy, pose);
  const url = URL.createObjectURL(new Blob([new XMLSerializer().serializeToString(copy)], { type: 'image/svg+xml' }));
  const image = new Image();
  image.src = url;
  try {
    await image.decode();
  } finally {
    URL.revokeObjectURL(url);
  }
  const canvas = document.createElement('canvas');
  canvas.width = COLS * sub;
  canvas.height = ROWS * sub;
  const context = canvas.getContext('2d', { willReadFrequently: true });
  context.drawImage(image, 0, 0);
  const { data } = context.getImageData(0, 0, canvas.width, canvas.height);
  return cellsFromPixels(data, COLS, ROWS, sub, ground, (CROP.width / COLS) / (CROP.height / ROWS), shade);
}
