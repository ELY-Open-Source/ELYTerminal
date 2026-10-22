# Linux packaging validation for v0.1.8

Checked on 2026-10-22, x86_64. This report covers the current application only.

## Source and scope

The base is upstream main `e477637690a78fe4cb1ea193d62084f72991d376`, whose
application sources and Cargo files match release tag v0.1.8
(`d5f97df52764e14a13bae4b5a7287809921a5e99`).

The packaging branch `codex/linux-packaging-v0.1.8` includes the shared commit
from `codex/linux-x11-ime-v0.1.8`. The separate IME PR should be reviewed and
merged first. No application files in `src/`, resources in `assets/` or Rust
toolchain version are changed. The Cargo patch is documented in
`vendor/xim-ctext/README.md`, with upstream provenance and its removal condition.

The packaging extends the design proposed in #4 with exact current-version
artifact names, a checksum manifest, package lifecycle checks and real-window
GUI smoke tests. The workflow publishes review artifacts, not official Releases.

## Build and local artifacts

`LINUX_DOCKER_NETWORK=host scripts/linux-packages.sh` builds once in Ubuntu
22.04 with Rust 1.95.0. All four formats use this binary and require glibc 2.35
or newer. Packaging may strip the binary or set its relative RPATH; neither
operation recompiles Rust.

The validated local build used the same application sources, Cargo files and
decoder implementation as these split PR branches. Hosted workflows rebuild
the submitted branch head. Local artifacts are in `target/release/`:

| Format | Artifact | Local result |
| --- | --- | --- |
| tar.gz | `ELYTerminal-0.1.8-linux-x86_64.tar.gz` | PASS |
| DEB | `elyterminal_0.1.8_amd64.deb` | PASS |
| RPM | `elyterminal-0.1.8-1.x86_64.rpm` | PASS |
| AppImage | `ELYTerminal-0.1.8-linux-x86_64.AppImage` | PASS |
| Checksums | `ELYTerminal-0.1.8-linux-x86_64.sha256` | All four PASS |

The DEB/RPM stripped binaries are identical, with local SHA-256
`c830b0a36aeb37c29e8a17a85ee4d697cb0c15d96ecac565d44dd17eb8470158`.
The tarball binary equals the release binary. The release, DEB, RPM and
AppImage binaries have identical `.text` sections, SHA-256
`10d3ff053813ecf171d3b2f2403bdd032494ed912b61ad543639a26f023d206f`.
All packages include the decoder's MIT license.

## Validation

| Check | Result and scope |
| --- | --- |
| Rust fmt, clippy and tests | PASS: 276 existing tests, 4 IME regressions and 9 upstream decoder tests, normal user |
| Dependency resolution | PASS: GPUI and tests use the same patched decoder |
| Shell syntax, ShellCheck, actionlint | PASS locally |
| Diff whitespace and 500-line rule | PASS locally; upstream vendor comments retain their original formatting |
| DEB lintian | PASS; documented FreeType override, no unused overrides |
| tar.gz/RPM payload | PASS: licenses, expected layout, binary identity and common code section |
| Ubuntu 22.04 DEB | PASS after IME fix: dependencies, metadata-only upgrade fixture, reinstall, verify, removal, settings preservation |
| Ubuntu 22.04 AppImage | PASS after IME fix: extraction, ELF dependencies, portable copy/removal, settings preservation |
| Ubuntu 22.04 DEB and AppImage GUI | PASS after IME fix: Xvfb, Mesa software OpenGL, real window, painted frame, PTY and exit 0 |
| Ubuntu 22.04 desktop Chinese input | PASS: user confirmed the patched v0.1.8 input works; other common operations reported working |
| Ubuntu 24.04 / Debian 12 DEB | Earlier v0.1.8 packaging checks PASS; installation not rerun locally after the decoder fix |
| Fedora 43 RPM | Earlier v0.1.8 packaging checks PASS; installation not rerun locally after the decoder fix |

The hosted Linux packages workflow repeats all three DEB distributions,
Fedora 43 RPM lifecycle checks and Ubuntu 22.04 GUI smoke tests for its head.
The separate hosted Linux IME workflow runs project checks, all application
tests and upstream decoder tests in a clean Ubuntu 22.04 container. The PRs
link exact hosted run results; local results do not stand in for hosted CI.

The GUI helper runs as a regular user with isolated settings. It requires a
visible nonblank window, PTY output, an unchanged child library path and exit 0.
Both DEB and AppImage screenshots were inspected. Static Chinese rendering
is separate from the user's successful interactive Chinese-input retest.

Local evidence is under `target/linux-validation/0.1.8/`: `ime/build.log`,
`ime/rust-checks.log`, `ime/split-checks.log`, `ime/dependency-tree.log`,
`ime/payloads.log`, `ime/deb-gui.log`, `ime/appimage-gui.log` and screenshots in
`ime/gui-deb/` and `ime/gui-appimage/`. Initial distribution checks are in
`deb-ubuntu24.log`, `deb-debian12.log` and `rpm-fedora43.log`.

Local proxy accommodations were limited to disposable containers; host package
manager and input-method settings were not modified. RPM payload extraction
used a read-only mount of the host's cpio because the build image lacks it.

## Remaining coverage

- Mesa software Vulkan produced a black capture under Xvfb; software OpenGL
  passed. CI sets `LIBGL_ALWAYS_SOFTWARE=1` and `VK_ICD_FILENAMES=/dev/null`
  only for its smoke-test processes. Vulkan rendering is not covered by this.
- Wayland, normal FUSE mounting, ARM64, Fedora 42, multi-monitor operation and
  exhaustive keyboard/mouse/drag-and-drop acceptance remain untested here.
- The DEB metadata-only upgrade fixture exercises the package manager using
  the same payload. It does not establish cross-release application migration.
  RPM cross-version upgrades are also untested.
- Manual Chinese input acceptance is limited to the tested Ubuntu X11/IBus
  desktop; other input methods and candidate placement scenarios are not
  inferred from the decoder fixtures or GUI smoke tests.
