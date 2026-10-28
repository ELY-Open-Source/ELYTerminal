# ELYTerminal website PRD

## 1. Experience

Ely, written in characters. One page, four scenes: the terminal, sessions, Git and Files, downloads. Ely changes pose as the page scrolls. English and Simplified Chinese. Dark and light themes follow the system until the visitor chooses. No tagline, feature descriptions, secondary labels, badges, or boxes inside boxes.

## 2. Visual system

Everything is monospaced. One fullscreen WebGL layer draws a swarm of glyphs in JetBrains Mono, one instanced draw.

- Ely is the original SVG, posed, rasterized onto a character grid. A cell's fill takes a glyph from the ramp ` .:-=+*#%@` by its contrast with the ground; a cell on a contour takes `|`, `/`, `-` or `\` by the edge's direction. Each glyph wears Ely's own color at that cell.
- Each cell owns one particle. Particles of empty cells, and a reserve, drift as faint dust.
- On load, glyphs fall, spell `ELYTerminal`, then fly into Ely.
- A scene change re-forms Ely in that scene's pose and place: the particles shift cells and fly, and a flying glyph scrambles until it lands.
- Glyphs part around the pointer. A press sends a shockwave.
- The physics runs in WebAssembly, a small Rust crate with no dependencies.

The page's own text is flat: no glass, blur, shadow or cards. anime.js decodes each heading from scrambled glyphs as it enters, types each transcript, and ripples the 3×3 session marks as the app draws them. Palette: ink on ground plus Ely's pink. Without WebGL or WebAssembly, the SVG Ely stands still in the hero and the page stays whole.

## 3. Product scenes

The terminal: the product name and a transcript, Ely beside them. Sessions: five sessions with their live marks and an agent's transcript. Git and Files: a Git and Files switch, changes, and a diff stat. Downloads: the detected system's download and every other format. Headings name only the product or the visible feature.

## 4. Ely motion

Reuse the original SVG paths and palette. A small rig poses root, head, eyes, gaze, hands, mouth and marks. Scroll moves Ely between the scenes' places continuously and switches pose at the scene's midpoint. Motion follows the display's frame rate and never hijacks scroll. Reduced motion skips the intro, the scramble and the drift: Ely appears formed and changes pose in place.

## 5. Downloads

Detect macOS, Windows and Linux from browser platform data. The primary link points to that system's current release asset. Other systems remain directly reachable. macOS is Apple silicon; Windows is x86_64; Linux offers AppImage, DEB, RPM and tarball. Use GitHub's stable `releases/latest/download/ASSET` URLs, never a hardcoded version. Unsupported systems go to the releases page.

Every current and future release must carry these six aliases beside its versioned files: `ELYTerminal-macos-arm64.dmg`, `ELYTerminal-windows-x86_64.zip`, `ELYTerminal-linux-x86_64.AppImage`, `elyterminal_amd64.deb`, `elyterminal.x86_64.rpm`, `ELYTerminal-linux-x86_64.tar.gz`. The release scripts upload them.

## 6. Implementation

Vite, vanilla JavaScript modules, CSS, Three.js, anime.js, and WebAssembly built from `swarm/` with the repository's Rust toolchain. No application framework, backend, database, analytics, accounts or remote runtime API. Static assets deploy through Cloudflare Workers. Every handwritten file stays at or below 500 lines. Comments are rare, one short line.

## 7. Verification

Unit checks cover platform detection, release links, locale and theme choice, the glyph grid, and the built WebAssembly's physics. Browser checks cover English, Chinese, dark, light, mobile, desktop, keyboard operation, reduced motion, scroll poses, the fallback, frame cadence, console errors and every download target. Screenshots are reviewed at each scene. Production is checked again after deployment.

## 8. Deployment

Deploy with Wrangler to `terminal.elygpui.com` as a Cloudflare Custom Domain. Keep the workers.dev URL available for rollback checks. Do not alter unrelated DNS records or the terminal application's settings.

## Sources

- Cloudflare static assets: https://developers.cloudflare.com/workers/static-assets/
- Cloudflare Custom Domains: https://developers.cloudflare.com/workers/configuration/routing/custom-domains/
- GitHub release links: https://docs.github.com/en/repositories/releasing-projects-on-github/linking-to-releases
- anime.js scrambleText: https://animejs.com/documentation/text/scrambletext
