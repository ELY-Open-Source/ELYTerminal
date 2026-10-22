# Linux packages

The Linux release uses one x86_64 binary for the tarball, DEB, AppImage and RPM.
The Docker builder uses Ubuntu 22.04 and Rust 1.95.0, the toolchain pinned by
`rust-toolchain.toml`. The build baseline is glibc 2.35; a package does not make
a newer binary compatible with an older glibc.

## Build

With Docker running, from the repository root:

```sh
scripts/linux-packages.sh
```

The scripts forward the standard HTTP(S) proxy variables to Docker. If a local
proxy listens only on loopback, Linux users can run
`LINUX_DOCKER_NETWORK=host scripts/linux-packages.sh`; the default is Docker's
bridge network. Cargo downloads and compilation are cached under
`target/linux-cargo` and `target/linux-build`. `CARGO_BUILD_JOBS` sets the job
count, which defaults to two to limit memory use.

This builds the release binary once and writes these files to `target/release`,
where `VERSION` is read from Cargo's package metadata:

| Format | File |
| --- | --- |
| tar.gz | `ELYTerminal-VERSION-linux-x86_64.tar.gz` |
| DEB | `elyterminal_VERSION_amd64.deb` |
| AppImage | `ELYTerminal-VERSION-linux-x86_64.AppImage` |
| RPM | `elyterminal-VERSION-1.x86_64.rpm` |

The aggregate script also writes `ELYTerminal-VERSION-linux-x86_64.sha256`.
Run `sha256sum -c ELYTerminal-VERSION-linux-x86_64.sha256` from that directory
to verify all four packages. The workflow selects this version's exact filenames,
so packages left from an older build are not selected or uploaded.

The individual `scripts/linux-tar.sh`, `scripts/linux-deb.sh`,
`scripts/linux-appimage.sh` and `scripts/linux-rpm.sh` also build and package
their format through Docker. The tar script retains native ARM64 builds and
their original host architecture suffix; these need validation on ARM hardware.
The new formats and aggregate script require x86_64.

To skip compilation, pass `LINUX_BINARY` in a prepared build environment and
choose the output directory with `LINUX_OUT_DIR`. Native tools are required;
the AppImage bundler uses Ubuntu's library paths and license records:

```sh
docker run --rm --user "$(id -u):$(id -g)" \
  -v "$PWD:/src" -w /src \
  -e LINUX_BINARY=/src/target/release/elyterminal \
  -e LINUX_OUT_DIR=/src/target/release \
  elyterminal-release scripts/linux-deb.sh
```

Native packages install the binary in `/usr/bin`, the shared desktop entry in
`/usr/share/applications`, and the existing SVG under the hicolor icon theme.
ELF dependency scans supply native library requirements. Libraries loaded at
runtime and external programs need explicit requirements as well: Git uses
`git`, update checks use `curl`, and desktop links use `xdg-open`.
Native package copies are stripped; the tarball keeps the original binary.
RPM builds use an isolated temporary database, so an unprivileged builder
does not need a user or system RPM database.
`man elyterminal` describes startup and the settings location.

The release binary statically links FreeType. `freetype-sys` builds its bundled
copy when `pkg-config` finds FreeType below 24.3.18, as Ubuntu 22.04 does, and
that distribution is the glibc 2.35 build baseline. The FreeType license ships
as `/usr/share/doc/elyterminal/FreeType-LICENSE.txt`; the DEB's lintian override
records why the library is embedded, and the RPM's `License` names FTL.

The AppImage keeps its own AppDir. Its packaging tool and runtime have fixed
official download URLs and SHA-256 checks. Host glibc, graphics drivers and
desktop integration libraries stay on the host; an AppImage is not a static
binary. Changes to the library bundle need both X11 and Wayland checks on the
target desktops.

## Validate

The local `vendor/xim-ctext` backport fixes Chinese X11 IME commits rejected by
GPUI's decoder dependency. Its upstream origin and removal condition are in
`vendor/xim-ctext/README.md`; the MIT license ships in all four formats.
`cargo test -p xim-ctext --locked` and `cargo test --test ime --locked` cover
CJK, mixed ASCII and UTF-8 commits. These tests verify decoding, while actual
input method composition and candidate placement still need desktop testing.

`scripts/linux-package-test.sh` installs and removes packages and therefore
refuses to run outside a container. Mount packages and the script read-only in
a disposable image. For example, after replacing `VERSION`:

```sh
docker run --rm \
  -v "$PWD/target/release:/packages:ro" \
  -v "$PWD/scripts/linux-package-test.sh:/test.sh:ro" \
  ubuntu:22.04 sh /test.sh /packages/elyterminal_VERSION_amd64.deb
```

Run the DEB test in `ubuntu:22.04`, `ubuntu:24.04` and `debian:12`. Run the RPM
test in `fedora:43`, passing `/packages/elyterminal-VERSION-1.x86_64.rpm`.
The helper checks package metadata, installed files, desktop validation,
resolved ELF libraries, dependency resolution, same-version reinstall and
removal. It seeds a settings file and verifies that every package operation
preserves it.

Ubuntu's minimal containers normally exclude documentation; the helper includes
ELYTerminal's license files and manual page so their installation is checked.
For DEB, the helper also repacks the target with an older
`VERSION~packaging-test` metadata version, installs it, then upgrades to the real
package and asserts both installed versions. This tests the package manager's
upgrade path with the same application payload; it does not test migration
from a previous application release. RPM cross-version upgrades remain untested.

Run the AppImage test in `elyterminal-release` with
`/packages/ELYTerminal-VERSION-linux-x86_64.AppImage`. It checks the type 2
header, executable permission, extraction, AppDir files and ELF dependencies.
It copies the portable file into a temporary location, removes that copy after
the optional GUI check and verifies that the seeded settings remain unchanged.
Extraction does not need FUSE and does not prove that a GUI window opens.
CI also runs `lintian --fail-on error` on the DEB; the one embedded-library
override is checked into `packaging/linux/elyterminal.lintian-overrides`.

Test GUI startup separately as a regular desktop user, with an isolated
`XDG_CONFIG_HOME`, on X11 and Wayland. Check the terminal, shell, fonts, links,
Git, window resizing and desktop launch. Record an unavailable test as
`NOT TESTED`; do not count extraction or an unresolved startup timeout as a
successful GUI test. Existing settings must remain untouched.

CI now also starts the installed DEB and the AppImage with extraction and no
FUSE in an Ubuntu 22.04 test image, as a regular user under Xvfb. The helper
explicitly selects Mesa software OpenGL with `LIBGL_ALWAYS_SOFTWARE=1` and
`VK_ICD_FILENAMES=/dev/null`: software Vulkan produced a black capture under
Xvfb even though the application ran. This setting applies only to the test
processes. The helper checks a visible, painted X11 window, a running
PTY, the child process's library environment and a normal exit. Screenshots and
application logs accompany the workflow artifacts. This covers startup, not
interactive desktop integration or a real Wayland compositor.

To reproduce those GUI checks after building the packages:

```sh
docker build --build-arg TEST_UID="$(id -u)" \
  -f packaging/linux/Dockerfile.test -t elyterminal-gui-test packaging/linux
mkdir -p target/linux-validation
docker run --rm --init -e LINUX_GUI_TEST=1 -e LINUX_TEST_OUTPUT=/validation \
  -v "$PWD/target/release:/packages:ro" -v "$PWD/scripts:/tests:ro" \
  -v "$PWD/target/linux-validation:/validation" \
  elyterminal-gui-test sh /tests/linux-package-test.sh /packages/elyterminal_VERSION_amd64.deb
```

Pass `/packages/ELYTerminal-VERSION-linux-x86_64.AppImage` to test the AppImage.
`scripts/linux-gui-test.py PROGRAM [ARGS...]` can also use an existing X11 desktop
as a regular user. It creates isolated config, data and cache directories and
keeps its application log and screenshot in the reported folder.

## CI and releases

`.github/workflows/linux-packages.yml` builds all formats and runs the container
checks on pull requests, branch pushes, and manual dispatch. Cargo downloads,
release compilation and verified AppImage tools are cached between runs.
It has read-only
repository permission, uses no secrets and uploads workflow artifacts for
review. Downloaded AppImages may need `chmod +x` after artifact extraction.

The existing release workflow keeps its macOS signing job. Linux package
publication remains a maintainer action: build the packages for the release's
commit, inspect their checksums and test results, then use the existing helper
for each file on the already-created release:

```sh
scripts/release-asset.sh target/release/elyterminal_VERSION_amd64.deb
scripts/release-asset.sh target/release/ELYTerminal-VERSION-linux-x86_64.AppImage
scripts/release-asset.sh target/release/elyterminal-VERSION-1.x86_64.rpm
```

The helper replaces an asset with the same name. Check the version and target
release before running it. The tarball remains part of the same release; no
packaging workflow creates or publishes a release automatically.
