# ELYTerminal PRD

A terminal for macOS, Windows and Linux. Rust, GPUI, Ely components. It draws a terminal as well as Ghostty does, shows every setting on screen, and makes no noise.

Chapters run in build order. `TASKS.md` follows them line by line.

## 1. Product

- One window. Sessions on the left, the terminal in the middle, Git or Files on the right.
- Every setting has a control in the settings window. No setting lives only in a file.
- Silent: no sound, no bell, no notifications but a session's, below, no update checks, no telemetry, no tips, no onboarding.
- Clean: no subtitle, no tagline, no logging options, no status bar, no badges that do not carry state.
- Built-in themes, each shown as a small preview, Catppuccin among them. A theme colors the terminal and the chrome.
- The window background blurs on macOS, Windows, Wayland and X11.

## 2. Foundation

- One binary crate, `elyterminal`, edition 2024, Rust 1.95.
- Ely from github.com/ELY-Open-Source by pinned rev. gpui, gpui_platform from crates.io `gpui-pre` at the exact version Ely pins.
- `alacritty_terminal` 0.26 for the grid, the parser and the pseudo-terminal, as Ely uses it.
- Logs through `log` to stderr, `RUST_LOG` sets the level. No log setting in the app.
- CI on GitHub Actions: macOS builds, tests and lints; Ubuntu and Windows build and test.
- Repo `ELY-Open-Source/ELYTerminal` on GitHub, public; it lived on git.zacharyzhang.com until v0.1.7. No credential in any file or commit.

## 3. Settings model and storage

- One JSON file, `settings.json`, in the platform config folder (`dirs::config_dir()/ELYTerminal`).
- Every field has a default. A missing file means defaults. A missing field means its default.
- A file that does not parse, or a value out of range, stops the start with an error naming the file and the field. The app never overwrites a file it could not read.
- Writes go to a temp file in the same folder, then rename. One writer: the app's settings entity.
- Fields: see chapter 10.

## 4. Themes

- Built in: Ely Dark, Ely Light, Catppuccin Mocha, Catppuccin Macchiato, Catppuccin Frappé, Catppuccin Latte, Tokyo Night, Dracula, Nord, Gruvbox Dark, Rosé Pine, Solarized Light, One Light, Kanagawa Wave.
- A theme holds background, foreground, cursor, cursor text, selection background and foreground, and the 16 ANSI colors. Values come from the theme's upstream files (iTerm2-Color-Schemes' Ghostty export, or Ely's palette).
- A theme maps onto Ely's `Palette` through `Theme::set_palette`, so panels, inputs and lists wear the theme.
- Dark mode: one theme, or a second theme for when the system is dark.
- On Omarchy 4 (Linux), an Omarchy theme wears Omarchy's current theme and changes with each switch. It is the default where Omarchy runs; picking another theme stops following.

## 5. Terminal engine

Ghostty is the bar for what the terminal does. ELYTerminal owns its engine on `alacritty_terminal`; Ely's `Terminal` is a gallery widget without mouse reporting, IME, kitty keys or style settings.

- PTY: alacritty's `tty` and event loop. The pty is wrapped so its reader passes bytes through an OSC scanner. The scanner reports the working directory and prompt marks.
- Shell integration for zsh, bash, fish and PowerShell, injected at launch as Ghostty does: each prompt sends OSC 7 and a title. On Windows a Shell of `wsl` runs the distribution's bash, zsh or fish with that integration, and its folder reaches Files and Git as Windows names it, git running inside the distribution. A shell without integration reports nothing, and the panels say so.
- Prompt marks, OSC 133: the shell integration marks where each prompt starts (A) and ends (B), where a command starts (C), and where it ends with its exit status (D), all but C on macOS's bash 3.2, which has no hook for it; the terminal reads any program's OSC 133. Cmd-Up and Cmd-Down, and Cmd-Shift-Up and Cmd-Shift-Down, on macOS, Ctrl-Shift-Up and Ctrl-Shift-Down elsewhere, scroll to the prompt before or after the view's top, as in Ghostty.
- Program status, OSC 7501 revision 0.4: each terminal keeps the records programs report (state, id, kind, progress, app, title, msg), checked as the spec checks them; a new prompt drops working, blocked and idle records, a full reset drops them all; the terminal answers the support query with the states and kinds it takes.
- Progress, OSC 9;4: set, error, indeterminate and pause stand as the root record until a program reports OSC 7501, until a full reset.
- Notifications: OSC 9 (iTerm2), OSC 777 notify (rxvt) and OSC 99 (kitty: title and body in chunks, base64, ids, focus and report on activation, close, expiry, which ones live, and its query; `o` passes by, since a session posts only while unseen, below); ConEmu's other OSC 9 commands are no notifications.
- Inside herdr, which keeps its panes' OSC 7, the session's folder is that of herdr's focused pane, read from herdr's socket while herdr's client runs in the session, on macOS, Linux and Windows.
- Inside herdr, which passes no OSC 7501 on, herdr's agent states (idle, working, blocked, done) are the session's status records.
- Grid: colors (named, 256, truecolor, OSC 4/10/11/12), bold, dim, italic, underline (single, double, curly, dotted, dashed), strikethrough, inverse, hidden, wide characters, combining marks.
- Box drawing (U+2500–U+257F), block elements (U+2580–U+259F) and powerline shapes (U+E0B0–U+E0B7) painted as geometry, so lines and segments join across cells.
- Fonts: JetBrains Mono by default, any installed font by choice, and fallback families tried in order for a character it lacks, a Chinese font after a Latin one among them; Nerd Font icons the font lacks come from a bundled Nerd Font.
- Cursor: block, bar, underline, hollow block; blinking per setting and per DECSCUSR; hollow when the window is not focused.
- Keys: xterm encoding with modifiers, application cursor mode, Option as Alt on macOS, and natural text editing there when the setting is on: Option-Left and Option-Right go as ESC b and ESC f, Option-Backspace and Option-Delete as ESC DEL and ESC d, Command-Left and Command-Right as Ctrl-A and Ctrl-E, Command-Backspace and Command-Delete as Ctrl-U and Ctrl-K, as iTerm2's preset; the kitty keyboard protocol as the program sets its flags; a key Shift makes a symbol of goes as its own key with Shift, the symbol its alternate, as Ghostty sends it, on macOS and Windows. On Windows keys go as win32-input-mode records while ConPTY asks, as Windows Terminal sends them. Keypad keys type their characters, as Ghostty's do with mode 1035 on by default.
- IME: marked text drawn at the cursor; committed text sent to the program.
- Mouse: select by character, word (double press) and line (triple press); Shift extends; mouse reporting (X10, normal, button, any; SGR and UTF-8); wheel scrolls back, or reports, or sends arrows on the alternate screen.
- Click to move: while the shell reads a line, as its OSC 133 marks say, a plain click on that line, past its prompt, moves the shell's cursor there, as Ghostty's `cursor-click-to-move`: a shell that takes clicks, as fish does, hears the click; others get arrow keys.
- Clipboard: copy, paste (bracketed when asked), copy on select per setting, OSC 52 writes.
- Drop: files dropped on the terminal type their paths at the cursor, as macOS Terminal does, each kept literal for the session's shell and followed by a space.
- Links: OSC 8 and plain URLs; Cmd-press (Ctrl-press off macOS) opens them. Inside herdr, while it takes the mouse, herdr opens them, web links only, as herdr does, whole where its pane wraps them, and underlines them while the key is held; with Shift the terminal opens what it sees.
- Focus reporting, title (OSC 0/2), scrollback limit per setting, find in scrollback (Ely's `FindWidget`).
- A scrollbar over the history, along the terminal's right edge: a drag on its thumb scrolls, whatever the program does with the wheel.
- Bell: no sound. The session row marks it until the session is shown.

## 6. Main window and layout

- Ely's `ColumnShell`: sessions column, terminal column, right column. The window opens with blur when the setting is on.
- The sessions column and the right column toggle, each by a button at its end of the title bar or by key, and keep their state in settings, so the window opens as it was left.
- The sessions column and the right column resize by a drag on their border toward the terminal, on macOS, Windows and Linux, from 160 to 960 pixels; the widths are kept in settings once a drag rests, so the window opens with them.
- The window opens at the size it was left at, on macOS, Windows and Linux: the size is kept in settings once it settles; a maximized or full-screen window keeps the size from before; a kept size past the screen shrinks to it; the window is no smaller than 100 by 100, the least the settings keep.
- The title shows the active session's title, and no other text.
- The pointer over the terminal is the arrow.
- Ely, the icon's girl, peeks over the window's top edge from behind it, on macOS, Windows and Linux; where no room lies above the window, at the screen's top or full screen, she stands inside it at its bottom left. She moves all the time, as she does in the films: she breathes, sways and blinks, and how the sessions are doing, waiting, an error, work, a finish or calm, moves her. Nothing but her shows: no frame, line or shadow around her. A hop lifts more of her from behind the edge, never her body off it. A press pets her, and wakes her from a doze. A drag moves her along the edge she stands on, and she stays where she is let go. A switch in Settings, Terminal, turns her off.
- The title bar is transparent, as Ghostty's: the window's background, opacity and blur run under it, so it wears the terminal's color.
- Every control in the title bar takes its own press on every platform, the window buttons Ely draws on Windows and Linux too.

## 7. Sessions panel

- A list of sessions. Each row: title, then the working directory, home as `~`.
- New session (+ and Cmd-T), close (row's ×, Cmd-W), select (press, Cmd-1…9, Ctrl-Tab and Ctrl-Shift-Tab).
- A right press on a row opens a menu at the pointer, while Session menu in Settings › Terminal is on: Duplicate session opens a session of the shell that row runs, in the folder it last reported, right after it and shown; Close session closes the row. A shell that reports no folder, as cmd does, duplicates into the folder it started in.
- A drag on a row moves it to another place in the list, and the session shown stays shown; Drag sessions to reorder in Settings › Terminal turns the drag off.
- Rename: a double press on a row's title opens a field in its place; Enter keeps the name, Escape drops it. The name shows in the row, the terminal's header and the window's title over the program's own until renamed again; an empty name gives the title back to the program.
- A session whose program ends closes its row. The last row closing closes the window.
- A bell in a hidden session marks its row.
- A row shows its session's status: working, with its progress where known; waiting on the user, for a permission, a question or a login; done; error; and a mark when the last command failed. Hover shows each record's program, title and message. A hidden session that starts waiting marks its row until it is shown.
- The mark is 3 by 3 dots that move: work ripples, or fills by its progress; waiting dips; a settled state draws its shape, then holds still.
- System notifications on macOS, Windows and Linux: a session that starts waiting, finishes or fails, a command that ran 5 seconds or more ending, and a program's OSC 9, 777 or 99 notification, post one while the window is not active or the session is hidden, one a session every 3 seconds at most; it names the session, and a press on it shows the window and the session. On macOS only the app bundle posts them.

## 8. Right panel: Files

- A switch at the top, centered below the title bar: Git, Files.
- Files shows the active session's working directory as a tree, folders first, Git status colors on names.
- Double press: a folder opens, a file opens in the system's default app.
- It reloads when the session reports a new directory and at each prompt.

## 9. Right panel: Git

- Through the `git` command line. No libgit2.
- Shows the branch, staged and unstaged changes, a commit message field and Commit, and recent commits.
- Stage and unstage a file. Press a file to see its diff in Ely's `DiffViewer`.
- Discard, which Ely's changes list draws on every unstaged and untracked row, asks first: an edit goes back to the index, a new file is deleted.
- Outside a repository it says so. A failed git command shows its error.
- It reloads at each prompt and on directory change.

## 10. Settings window

Cmd-, opens it; on Windows and Linux, which have no menu bar, so does a gear in the title bar. Categories on the left, rows on the right, each side scrolling on its own, Ely's components. Changes apply at once to every session and save.

- Appearance
  - Theme: a grid of previews; on Omarchy, the Omarchy theme first.
  - Separate dark theme: a switch; when on, a second grid for dark mode.
  - Font family: a list of installed fonts. Fallbacks: installed families tried in order for a character the family lacks, each added, changed or removed in a list of pickers. Font size. Line height. Ligatures.
  - Cursor: style (block, bar, underline, hollow), blink, with a live preview.
  - Background opacity: a slider. Background blur: a switch.
  - Padding: horizontal and vertical.
- Terminal
  - Shell: a path; empty runs the login shell, on Windows `pwsh`, else `powershell`, else `cmd`. On Windows a session takes the registry's environment as it starts.
  - Start in: Home, or the active session's folder.
  - Scrollback lines.
  - Option as Alt: off, left, right, both (macOS only).
  - Natural text editing: a switch, off by default (macOS only); Option and Command edit by word and line, as the shell expects.
  - Copy on select.
  - Drag sessions to reorder: a switch, on by default; off, a drag on a session's row moves nothing.
  - Session menu: a switch, on by default; off, a right press on a session's row opens nothing.
  - Ely: a switch.
  - Check for updates automatically: a switch, on by default, which checks at start and once a day.
  - Updates: what the last check found, and Check now; a newer release brings Download, which opens its page, and one an automatic check finds posts a notification.
  - Shortcuts: every shortcut, as this platform's keys, with a search.

## 11. Background blur

- macOS: gpui's `WindowBackgroundAppearance::Blurred` (NSVisualEffectView).
- Windows: gpui's `Blurred` (acrylic accent). On Windows 10, whose DWM drags an acrylic window seconds behind the pointer, the glass clears while the window moves or sizes and blurs again when it rests; Windows 11 keeps its acrylic throughout.
- Wayland: gpui's `Blurred` speaks `org_kde_kwin_blur`; ELYTerminal adds `ext_background_effect_v1` on the window's surface, which GNOME 51, Plasma 6.7 and Hyprland support. Hyprland honors it only while its own blur is on, which Omarchy 4.0.4 turns off.
- X11: ELYTerminal sets `_KDE_NET_WM_BLUR_BEHIND_REGION` (CARDINAL, 32) on the window, as Ghostty does; removes it when blur turns off.
- A compositor with neither protocol shows transparency without blur. The app logs which path it took.

## 12. Menus and shortcuts

- macOS menu bar: ELYTerminal (Settings, Quit), Shell (New Session, Close Session), Edit (Copy, Paste, Select All, Find), View (Toggle Sessions, Toggle Right Panel, Bigger, Smaller, Reset Size), Window.
- The same shortcuts on Windows and Linux with Ctrl-Shift for terminal-colliding keys. On Windows Ctrl-V pastes too, as in Windows Terminal.

## 13. Acceptance

- Each chapter's tasks ticked in `TASKS.md` with a passing review in `tasks/reviews.md`.
- `scripts/check.sh` passes: fmt, clippy with `-D warnings`, tests, no file over 500 lines.
- CI green on macOS, Ubuntu and Windows.

## 14. Icon and the macOS app

- The icon is Ely, a girl with long pink hair, holding a terminal in front of her.
- macOS: the icon on the rounded square macOS gives app icons. Windows and Linux: the figure alone, no ground, flat, as Papirus draws.
- `scripts/mac-app.sh` builds `ELYTerminal.app` from a release build with its icon, signed ad hoc, or for a release with Developer ID and notarized, and a disk image holding it beside a link to Applications.

## 15. Release

- A release on GitHub, tagged `vVERSION`, carries three builds: the macOS disk image (Apple silicon, signed with Developer ID and notarized), a Windows zip (x86_64) and a Linux tarball (x86_64).
- The Windows executable carries the icon. The Linux tarball carries the icon and a desktop entry, and the window names itself after it, `com.zacharyzhang.elyterminal`, as the macOS bundle does.
- Each build comes from its platform's script: `scripts/mac-app.sh`, `scripts/windows-zip.ps1`, `scripts/linux-tar.sh`. The macOS one runs on the signing runner, through the Release workflow.

## Deferred

Splits, tabs inside a session, profiles, key rebinding, kitty graphics, sixel, session restore, push and pull, quick terminal.
