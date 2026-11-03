import { readFileSync } from 'node:fs';
import { defineConfig } from 'vite';
import { crateVersion, stampVersion } from './version.js';

const manifest = new URL('../Cargo.toml', import.meta.url);

export default defineConfig({
  plugins: [{
    name: 'ely-version',
    transformIndexHtml: {
      order: 'pre',
      handler: (html) => stampVersion(html, crateVersion(readFileSync(manifest, 'utf8'))),
    },
  }],
});
