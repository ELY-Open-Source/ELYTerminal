# Website tasks

Progress truth for this goal. On 2026-10-28 the owner dropped the Bayer dither after the redesign; the site is ASCII alone. Work in order. Each item needs a passing Herdr/Codex review before the next. Maximum three reviews per item.

- [x] W1 §1–2 — lock the visual architecture, release facts and deployment contracts. Verify: PRD, architecture review, official sources, current assets.
- [x] W2 §6 — scaffold Vite and Three.js with the static page and local assets. Verify: clean install and production build.
- [x] W3 §1–3 — build the responsive hero, terminal scene and navigation without nested cards or filler copy. Verify: desktop/mobile screenshots and first Codex review.
- [x] W4 §4 — build Ely's layered SVG rig and scroll-linked pose choreography. Verify: distinct captured poses, reduced motion and Codex review.
- [x] W5 §2 — finish the Three.js synthwave scene, VHS texture and both palettes. Verify: WebGL and non-WebGL rendering, frame stability and Codex review.
- [x] W6 §1–3 — add the sessions, Git and Files scenes with sparse bilingual content. Verify: screenshots, content checks and Codex review.
- [x] W7 §5 — add platform detection and stable download links for every released format; publish the six versionless aliases and preserve them in the release scripts. Verify: unit tests, redirected HEAD requests and Codex review.
- [x] W8 §1–4 — replace the rejected Synthwave treatment with pale Aurora light and Apple-inspired liquid glass; remove purple gradients and VHS styling. Verify: both palettes, all scenes, responsive screenshots and Codex review.
- [x] W9 §1–2 — complete English/Chinese and dark/light controls with persistence. Verify: browser interactions and Codex review.
- [x] W10 §7 — run desktop, mobile, keyboard, reduced-motion, console and visual acceptance. Fix causes, then pass Codex review.
- [x] W11 §2, §4 — replace the rejected dither field with Ely in characters: the Rust WebAssembly swarm, the glyph grid from the posed SVG with edge glyphs, the instanced glyph renderer, the intro, scene re-forming, scramble, pointer and press. Verify: unit tests on the grid and the built WebAssembly, both palettes at all scenes, frame cadence, reduced motion, fallback and Codex review. Three reviews spent, the cap: round 3 found a lost context left the dead canvas over the page; it now hides the stage, and `tests/stage.mjs` loses a real context and reads the ground. The owner accepted it on 2026-10-28.
- [x] W12 §1–3 — rebuild the page flat and monospaced around the swarm: header, headings, transcripts, sessions with their marks, Git and Files, downloads; anime.js decode, typing and marks; delete the old chrome, glass, Space Grotesk and the dither field. Verify: layout matrix, screenshots and Codex review.
- [x] W13 §7 — rerun desktop, mobile, keyboard, reduced-motion, console and visual acceptance on the new design. Fix causes, then pass Codex review.
- [x] W14 §8 — deploy the Worker, bind terminal.elygpui.com and verify production. Record rollback and final acceptance. Deployed 2026-10-28 as version `66651f41-7518-454e-b9be-78998eace142`, on terminal.elygpui.com and elyterminal.zhangyanghaha0407.workers.dev; every built file serves byte for byte on both. Rollback: `npx wrangler rollback 6797e7d3-4e23-4cd5-bed2-04bf05030625`, the Aurora site W10 accepted.
- [x] W15 — update AGENTS.md with the website stack and commands; run the repository 500-line gate and final review.
