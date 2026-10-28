export const RAMP = ' .:-=+*#%@';
export const EDGES = '-\\|/';
export const NOISE = '!<>_[]{}=+*^?#%$01';
export const ATLAS = RAMP + EDGES + NOISE;
export const NOISE_START = RAMP.length + EDGES.length;

const EDGE_START = RAMP.length;
const EDGE_STRENGTH = 0.9;
const SOLID = 0.25;

function luminance(r, g, b) {
  return (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
}

// Average each sub×sub block into one cell: alpha, color, contrast.
function average(data, cols, rows, sub, ground, shade) {
  const width = cols * sub;
  const alpha = new Float32Array(cols * rows);
  const signal = new Float32Array(cols * rows);
  const colors = new Float32Array(cols * rows * 3);
  for (let row = 0; row < rows; row += 1) {
    for (let col = 0; col < cols; col += 1) {
      let a = 0;
      let r = 0;
      let g = 0;
      let b = 0;
      for (let y = row * sub; y < (row + 1) * sub; y += 1) {
        for (let x = col * sub; x < (col + 1) * sub; x += 1) {
          const at = (y * width + x) * 4;
          const weight = data[at + 3] / 255;
          a += weight;
          r += data[at] * weight;
          g += data[at + 1] * weight;
          b += data[at + 2] * weight;
        }
      }
      const cell = row * cols + col;
      alpha[cell] = a / (sub * sub);
      if (a === 0) continue;
      colors.set([r, g, b].map((channel) => (channel / a / 255) * shade), cell * 3);
      signal[cell] = alpha[cell] * Math.abs(luminance(r / a, g / a, b / a) * shade - ground);
    }
  }
  return { alpha, signal, colors };
}

// An edge's tangent angle on screen picks its stroke.
export function edgeGlyph(gx, gy) {
  const angle = (Math.atan2(gx, -gy) + Math.PI) % Math.PI;
  return EDGE_START + Math.round(angle / (Math.PI / 4)) % 4;
}

export function cellsFromPixels(data, cols, rows, sub, ground, aspect, shade = 1) {
  if (data.length !== cols * sub * rows * sub * 4) throw new Error('Glyph grid: pixel buffer does not match the grid');
  const { alpha, signal, colors } = average(data, cols, rows, sub, ground, shade);
  const index = (col, row) => Math.min(rows - 1, Math.max(0, row)) * cols + Math.min(cols - 1, Math.max(0, col));
  const field = (col, row) => signal[index(col, row)] + (alpha[index(col, row)] > SOLID ? 0.08 : 0);
  const gx = new Float32Array(cols * rows);
  const gy = new Float32Array(cols * rows);
  const strength = new Float32Array(cols * rows);
  for (let row = 0; row < rows; row += 1) {
    for (let col = 0; col < cols; col += 1) {
      const cell = row * cols + col;
      if (alpha[cell] < SOLID) continue;
      gx[cell] = (field(col + 1, row - 1) + 2 * field(col + 1, row) + field(col + 1, row + 1)
        - field(col - 1, row - 1) - 2 * field(col - 1, row) - field(col - 1, row + 1)) / aspect;
      gy[cell] = field(col - 1, row + 1) + 2 * field(col, row + 1) + field(col + 1, row + 1)
        - field(col - 1, row - 1) - 2 * field(col, row - 1) - field(col + 1, row - 1);
      strength[cell] = Math.hypot(gx[cell], gy[cell]);
    }
  }
  const glyphs = new Uint8Array(cols * rows);
  for (let row = 0; row < rows; row += 1) {
    for (let col = 0; col < cols; col += 1) {
      const cell = row * cols + col;
      if (alpha[cell] < SOLID) continue;
      // Keep the ridge alone, so contours stay one glyph wide.
      const dx = Math.round(gx[cell] / strength[cell]);
      const dy = Math.round(gy[cell] / strength[cell]);
      const ridge = strength[cell] >= strength[index(col + dx, row + dy)]
        && strength[cell] > strength[index(col - dx, row - dy)];
      if (strength[cell] > EDGE_STRENGTH && ridge) {
        glyphs[cell] = edgeGlyph(gx[cell], gy[cell]);
        continue;
      }
      glyphs[cell] = Math.max(1, Math.min(RAMP.length - 1, Math.round(Math.sqrt(signal[cell]) * (RAMP.length - 1))));
    }
  }
  return { glyphs, colors };
}
