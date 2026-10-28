# Linux

English / [中文](../zh_cn/install.md)

Replace `VERSION` with the release's version. Linux builds need glibc 2.35 or
later, an X11 or Wayland desktop and working graphics drivers. Native packages
install the runtime libraries, desktop entry and icon through the package manager.
The formats below can be built with `scripts/linux-packages.sh`. Use the assets
actually listed for your selected release; packaging CI artifacts are separate
downloads.

On Ubuntu 22.04, Ubuntu 24.04 or Debian 12:

```sh
sudo apt install ./elyterminal_VERSION_amd64.deb
```

On Fedora 43:

```sh
sudo dnf install ./elyterminal-VERSION-1.x86_64.rpm
```

Install a newer package the same way to update. To remove it, use
`sudo apt remove elyterminal` or `sudo dnf remove elyterminal`.
Package operations leave `~/.config/ELYTerminal` alone.

The AppImage runs without a system installation:

```sh
chmod +x ELYTerminal-VERSION-linux-x86_64.AppImage
./ELYTerminal-VERSION-linux-x86_64.AppImage
```

It bundles the keyboard libraries, but uses the host's glibc, X11/Wayland and
graphics libraries, and installed fonts. Install `git`, `curl` and `xdg-utils`
on the host for Git, update checks and desktop links. Without FUSE, run
`./ELYTerminal-VERSION-linux-x86_64.AppImage --appimage-extract-and-run`.
Replace the file to update; delete it to uninstall. Other distributions and
Wayland desktops need testing with their own graphics stack.

The tarball remains available. Install libxkbcommon, libxkbcommon-x11, libxcb,
the desktop's runtime libraries and the tools above first:

```sh
tar -xzf ELYTerminal-VERSION-linux-x86_64.tar.gz
cd ELYTerminal-VERSION-linux-x86_64
install -Dm755 elyterminal -t ~/.local/bin
install -Dm644 com.zacharyzhang.elyterminal.desktop -t ~/.local/share/applications
install -Dm644 com.zacharyzhang.elyterminal.svg -t ~/.local/share/icons/hicolor/scalable/apps
install -Dm644 elyterminal.1.gz -t ~/.local/share/man/man1
```

The desktop entry runs `elyterminal`, so `~/.local/bin` must be on `PATH`.
Repeat these commands with a newer tarball to update. To uninstall, remove
the four installed files; settings stay in the config folder.
