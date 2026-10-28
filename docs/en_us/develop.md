# Development

English / [中文](../zh_cn/develop.md)

Each platform's script builds its release in `target/release`: on macOS, `scripts/mac-app.sh` builds `ELYTerminal.app`, with its icon, and a disk image of it; on Windows, `scripts/windows-zip.ps1` zips `elyterminal.exe`, its icon inside, with Microsoft's `conpty.dll`, `OpenConsole.exe` and their license; on Linux, `scripts/linux-packages.sh` builds once in Docker on Ubuntu 22.04 and packs the same binary as tar.gz, DEB, AppImage and RPM. `scripts/linux-tar.sh` still builds the tarball on its own. [Linux packaging](../../packaging/linux/README.md) covers individual scripts, validation and release uploads.

On Linux, install the libraries gpui links first. On Debian and Ubuntu:

```sh
sudo apt-get install pkg-config libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev \
  libxcb1-dev libx11-xcb-dev libfontconfig-dev libfreetype-dev
```

The app writes its log to stderr, its own info lines and Ely's. `RUST_LOG=elyterminal=debug,ely_gpui_component=info` writes more; `trace` logs each paint. A Windows release build opens no console: `elyterminal.exe 2> log.txt` keeps its log.

`scripts/check.sh` runs fmt, clippy, the tests and the 500-line rule. [AGENTS.md](../../AGENTS.md) lists the rest: the layout, the decisions and the tools.
