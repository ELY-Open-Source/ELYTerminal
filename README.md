# ELYTerminal

A terminal for macOS, Windows and Linux, in Rust on GPUI and Ely components. Sessions sit on the left, the terminal in the middle, Git or Files on the right. Every setting has a control in the settings window.

## Install

A [release on GitHub](https://github.com/ELY-Open-Source/ELYTerminal/releases) carries three builds.

- macOS 11 or later, Apple silicon: open `ELYTerminal-VERSION.dmg` and drag ELYTerminal into Applications. The app is signed with Developer ID and notarized.
- Windows 10 or later, x86_64: unzip `ELYTerminal-VERSION-windows-x86_64.zip` and run `elyterminal.exe`. It is unsigned; if SmartScreen stops it, press More info, then Run anyway.
- Linux, x86_64, glibc 2.35 or later, with libxkbcommon, libxkbcommon-x11 and libxcb:

```sh
tar -xzf ELYTerminal-VERSION-linux-x86_64.tar.gz
cd ELYTerminal-VERSION-linux-x86_64
install -Dm755 elyterminal -t ~/.local/bin
install -Dm644 com.zacharyzhang.elyterminal.desktop -t ~/.local/share/applications
install -Dm644 com.zacharyzhang.elyterminal.svg -t ~/.local/share/icons/hicolor/scalable/apps
```

The desktop entry runs `elyterminal`, so `~/.local/bin` must be on `PATH`.

## Build and run

Rust 1.95.0, which `rust-toolchain.toml` pins; rustup installs it on the first build.

```sh
cargo run --release
```

Each platform's script builds its release in `target/release`: on macOS, `scripts/mac-app.sh` builds `ELYTerminal.app`, with its icon, and a disk image of it; on Windows, `scripts/windows-zip.ps1` zips `elyterminal.exe`, its icon inside; on Linux, `scripts/linux-tar.sh` builds in Docker and packs the binary with its desktop entry and icon.

On Linux, install the libraries gpui links first. On Debian and Ubuntu:

```sh
sudo apt-get install pkg-config libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev \
  libxcb1-dev libx11-xcb-dev libfontconfig-dev libfreetype-dev
```

The app writes its log to stderr. `RUST_LOG=elyterminal=debug` writes more; `trace` logs each paint. A Windows release build opens no console: `elyterminal.exe 2> log.txt` keeps its log.

## What it does

- The terminal runs on `alacritty_terminal`: 256 colors and truecolor, bold, dim, italic, five underline styles, wide characters and combining marks. Box drawing, block elements and powerline shapes are drawn as geometry, so lines join across cells. Keys follow xterm and the kitty keyboard protocol, and on Windows win32-input-mode, as Windows Terminal sends them, so herdr's shortcuts work; input methods compose at the cursor; programs can track the mouse; pastes are bracketed when a program asks; OSC 8 and plain links open with the link press below; inside herdr, herdr opens them, web links only, whole where its pane wraps them, and with Shift the terminal opens what it sees; find searches the scrollback.
- A scrollbar along the terminal's right edge scrolls its history, even while a program takes the wheel.
- Shell integration for zsh, bash, fish and PowerShell, and on Windows for the bash, zsh or fish a Shell of `wsl` starts, so Files and Git follow a WSL session: each prompt reports its folder and title, and OSC 133 marks each prompt, each command and its exit status.
- Ely peeks over the window's top edge, or up from its bottom left when the window leaves her no room above it, and moves all the time: she breathes, sways and blinks. She follows your sessions: she hops when a program waits on you, shakes at an error, cheers a finish, reads along while a program works, and dozes when all is calm for long. Press her and she winks, waking her from a doze. Drag her along the edge to move her. Settings › Terminal › Ely turns her off.
- Sessions: new, close and switch. A right press on a row duplicates it, in the shell it runs and the folder it last reported, or closes it, and a drag moves a row to another place; Settings › Terminal turns either off. A bell marks a hidden session's row and makes no sound. A row's mark shows what its programs report through OSC 7501, OSC 9;4 or herdr: waiting on you, an error, work with its progress, done or idle, else a failed last command. The mark is 3 by 3 dots: work ripples and waiting dips while they last. Hover over the mark lists each report.
- Notifications: while the window is in the background or the session hidden, a session that starts waiting, finishes or fails, a command that ran 5 seconds or more ending, and a program's OSC 9, 777 or 99, post a system notification, one a session every 3 seconds at most; a press shows the session. On macOS only the app bundle posts them, not `cargo run`.
- Cmd-Up and Cmd-Down, Ctrl-Shift-Up and Ctrl-Shift-Down off macOS, jump to the prompt before or after the view's top.
- Natural text editing on macOS, off by default in Settings › Terminal: Option-Left and Option-Right move by word, Option-Backspace and Option-Delete delete by word, Command-Left and Command-Right move to the line's ends, and Command-Backspace and Command-Delete delete to them, sent as the editing bytes the shell expects.
- A button at each end of the title bar hides or shows its side column, and a drag on the border between a side column and the terminal resizes it; the window opens as it was left, at the size and widths it was left at. On Windows and Linux a gear beside the right one opens Settings.
- Files shows the session's folder as a tree with Git's marks. Inside herdr the folder is that of herdr's focused pane.
- Git shows the branch, stages, unstages, discards, commits, lists recent commits and opens diffs, all through the `git` command line.
- Themes: Ely Dark, Ely Light, Catppuccin Mocha, Macchiato, Frappé and Latte, Tokyo Night, Dracula, Nord, Gruvbox Dark, Rosé Pine, Solarized Light, One Light and Kanagawa Wave. A second theme can take over while the system is dark. On Omarchy 4, an Omarchy theme follows the system's theme as it switches, and is the default there.
- Background opacity and blur: NSVisualEffectView on macOS, acrylic on Windows, `ext_background_effect_v1` or `org_kde_kwin_blur` on Wayland, `_KDE_NET_WM_BLUR_BEHIND_REGION` on X11, which KWin reads. Blur shows only while the opacity is under 100%. Hyprland blurs only while its own `decoration:blur:enabled` is on, and Omarchy 4.0.4 turns it off; to turn it on there, add `hl.config({ decoration = { blur = { enabled = true } } })` to `~/.config/hypr/looknfeel.lua`.

## Keys

Settings › Terminal lists them all, as this platform binds them.

| | macOS | Windows and Linux |
|---|---|---|
| New session | Cmd-T | Ctrl-Shift-T |
| Close session | Cmd-W | Ctrl-Shift-W |
| Session 1 to 8, last | Cmd-1 to Cmd-8, Cmd-9 | Alt-1 to Alt-8, Alt-9 |
| Next, previous session | Ctrl-Tab, Ctrl-Shift-Tab | Ctrl-Tab, Ctrl-Shift-Tab |
| Sessions column | Cmd-B | Ctrl-Shift-B |
| Right column | Cmd-Alt-B | Ctrl-Shift-Alt-B |
| Copy, paste | Cmd-C, Cmd-V | Ctrl-Shift-C, Ctrl-Shift-V; on Windows Ctrl-V pastes too |
| Select all | Cmd-A | Ctrl-Shift-A |
| Find | Cmd-F | Ctrl-Shift-F |
| Bigger, smaller, reset size | Cmd-=, Cmd-minus, Cmd-0 | Ctrl-=, Ctrl-minus, Ctrl-0 |
| Open a link | Cmd-press | Ctrl-press |
| Settings | Cmd-, | Ctrl-, |
| Minimize | Cmd-M | |
| Quit | Cmd-Q | Ctrl-Shift-Q |

## Settings

The settings window saves each change at once to `settings.json` in the platform's config folder:

- macOS: `~/Library/Application Support/ELYTerminal/settings.json`
- Linux: `$XDG_CONFIG_HOME/ELYTerminal/settings.json`, else `~/.config/ELYTerminal/settings.json`
- Windows: `%APPDATA%\ELYTerminal\settings.json`

A file that does not parse, or holds a value out of range, stops the start with an error naming the file and the field. The app never overwrites it.

Settings › Terminal checks for a newer release at start and once a day, or at once with Check now; a newer one shows a Download button that opens its page. The check runs `curl`, which macOS and Windows ship.

## Development

`scripts/check.sh` runs fmt, clippy, the tests and the 500-line rule. `AGENTS.md` lists the rest: the layout, the decisions and the tools.

## License

MIT, in `LICENSE`. Box drawing and key encoding follow Ghostty's code, MIT, in `LICENSE-GHOSTTY`. The theme files come from iTerm2-Color-Schemes, MIT, with its license in `assets/themes`. JetBrains Mono and its Nerd Font build are under the SIL Open Font License, in `assets/fonts`.
