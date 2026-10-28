import {
  CanvasTexture,
  DynamicDrawUsage,
  InstancedBufferAttribute,
  InstancedBufferGeometry,
  Mesh,
  PlaneGeometry,
  ShaderMaterial,
  Vector2,
} from 'three';
import { ATLAS, NOISE, NOISE_START } from './grid.js';

const FONT = "800 58px 'JetBrains Mono'";
const TILE = 64;

const vertexShader = `
  attribute vec3 aState;
  attribute vec4 aColor;
  attribute vec2 aGlyph;
  attribute float aSeed;
  uniform vec2 uViewport;
  uniform float uTime;
  uniform float uCell;
  uniform float uScramble;
  varying vec2 vUv;
  varying vec4 vColor;

  void main() {
    float glyph = aGlyph.x;
    if (uScramble * aState.z > 160.0) {
      glyph = ${NOISE_START.toFixed(1)} + mod(floor(aSeed * 31.0 + uTime * 16.0), ${NOISE.length.toFixed(1)});
    }
    float size = aGlyph.y > 0.0 ? aGlyph.y : uCell;
    vec2 pixel = aState.xy + vec2(position.x, -position.y) * size;
    vUv = vec2((glyph + position.x + 0.5) / ${ATLAS.length.toFixed(1)}, position.y + 0.5);
    vColor = aColor;
    gl_Position = vec4(pixel.x / uViewport.x * 2.0 - 1.0, 1.0 - pixel.y / uViewport.y * 2.0, 0.0, 1.0);
  }
`;

const fragmentShader = `
  uniform sampler2D uAtlas;
  varying vec2 vUv;
  varying vec4 vColor;

  void main() {
    float coverage = texture2D(uAtlas, vUv).a * vColor.a;
    if (coverage < 0.01) discard;
    gl_FragColor = vec4(vColor.rgb, coverage);
  }
`;

async function atlas() {
  if ((await document.fonts.load(FONT, ATLAS)).length === 0) {
    throw new Error('Glyphs: JetBrains Mono did not load');
  }
  const canvas = document.createElement('canvas');
  canvas.width = TILE * ATLAS.length;
  canvas.height = TILE;
  const context = canvas.getContext('2d');
  context.font = FONT;
  context.fillStyle = '#fff';
  context.textAlign = 'center';
  context.textBaseline = 'middle';
  for (const [index, glyph] of [...ATLAS].entries()) {
    context.fillText(glyph, TILE * (index + 0.5), TILE / 2);
  }
  return new CanvasTexture(canvas);
}

// One instanced quad per particle; state comes straight from WebAssembly.
export async function createGlyphs(output, count) {
  const geometry = new InstancedBufferGeometry();
  const plane = new PlaneGeometry(1, 1);
  geometry.index = plane.index;
  geometry.setAttribute('position', plane.getAttribute('position'));
  const state = new InstancedBufferAttribute(output, 3).setUsage(DynamicDrawUsage);
  const color = new InstancedBufferAttribute(new Float32Array(count * 4), 4).setUsage(DynamicDrawUsage);
  const glyph = new InstancedBufferAttribute(new Float32Array(count * 2), 2).setUsage(DynamicDrawUsage);
  const seed = new InstancedBufferAttribute(Float32Array.from({ length: count }, () => Math.random()), 1);
  geometry.setAttribute('aState', state);
  geometry.setAttribute('aColor', color);
  geometry.setAttribute('aGlyph', glyph);
  geometry.setAttribute('aSeed', seed);
  geometry.instanceCount = count;
  const uniforms = {
    uAtlas: { value: await atlas() },
    uViewport: { value: new Vector2(1, 1) },
    uTime: { value: 0 },
    uCell: { value: 10 },
    uScramble: { value: 1 },
  };
  const material = new ShaderMaterial({ uniforms, vertexShader, fragmentShader, transparent: true, depthTest: false, depthWrite: false });
  const mesh = new Mesh(geometry, material);
  mesh.frustumCulled = false;
  return { mesh, uniforms, state, color, glyph };
}
