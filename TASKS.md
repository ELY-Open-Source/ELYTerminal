# Tasks

Progress truth. Top to bottom, one at a time. A task is ticked only after its codex review passes; rounds go in `tasks/reviews.md`, three at most.

## 2. Foundation

- [x] T01 Repo, crate, window: git, `.gitignore`, `Cargo.toml` with Ely pinned, `main.rs` opens one window through Ely, logging, `scripts/check.sh`, CI workflow, private Gitea repo, first push.

## 3. Settings model and storage

- [x] T02 `Settings` with defaults and ranges; load (missing file, missing field, bad file, bad value) and atomic save; tests.

## 4. Themes

- [x] T03 Built-in themes from upstream files; mapping onto Ely's `Palette`; dark-mode choice; tests.

## 5. Terminal engine

- [x] T04 PTY: spawn through alacritty's event loop, OSC 7 tap on the reader, child id, events to the app; tests.
- [x] T05 Shell integration for zsh, bash, fish, PowerShell, injected at launch; tests that zsh and bash report their folder.
- [x] T06 Terminal view: frame from the grid, colors and styles, wide and combining cells, cursor shapes and blink, font, line height, padding, resize.
- [x] T07 Box drawing, block elements and powerline shapes (U+E0B0–E0B7) painted as geometry; tests.
- [x] T08 Keys: xterm encoding, cursor mode, Option as Alt, kitty keyboard protocol, IME; tests.
- [x] T09 Mouse: selection, mouse reporting, wheel and scrollback, clipboard, copy on select, OSC 52, links; tests.
- [x] T10 Focus reporting, title, silent bell, scrollback limit, find.

## 6. Main window and layout

- [x] T11 `ColumnShell` with sessions, terminal and right columns; toggles kept in settings; window title; a transparent title bar over the window's background, as Ghostty's.

## 7. Sessions panel

- [x] T12 Sessions: new, close, select, keys, titles and folders, ended programs, bell marks.

## 8. Right panel: Files

- [x] T13 Git/Files switch; file tree of the session's folder with Git colors; open; reload on folder and prompt.

## 9. Right panel: Git

- [x] T14 Git through the command line: status, branch, stage, unstage, commit, log, diff; tests on a temp repo.
- [x] T15 Git panel: branch, changes, stage and unstage, commit, recent commits, diff view, errors.

## 10. Settings window

- [x] T16 Settings window: categories, General rows, live apply and save.
- [x] T17 Appearance: theme grids with previews, font, cursor with preview, opacity, blur, padding.
- [x] T18 Terminal rows: scrollback, Option as Alt, copy on select.

## 11. Background blur

- [x] T19 macOS and Windows: `Blurred` or `Transparent` or `Opaque` from settings, applied live.
- [x] T20 Wayland: `ext_background_effect_v1` on the window's surface.
- [x] T21 X11: `_KDE_NET_WM_BLUR_BEHIND_REGION` on the window.

## 12. Menus and shortcuts

- [x] T22 macOS menu bar; key bindings on all platforms.

## 13. Acceptance

- [x] T23 End-to-end checks, README, acceptance table, CI green on three platforms.

## 14. Icon and the macOS app

- [x] T24 Icon: Ely holding a terminal, on macOS's rounded square and alone for Windows and Linux; `scripts/mac-app.sh` builds `ELYTerminal.app` and its disk image.

## 15. Release

- [x] T25 Windows zip with the icon in the executable, Linux tarball with the icon and a desktop entry, the app id on Linux windows, their scripts; release v0.1.0 on Gitea with the three builds.

## 16. Fixes from the owner's first run

- [x] T26 On Windows every title bar control takes its press, Ely's window buttons too: in Ely, window buttons and header items occlude the drag region; the Git and Files switch moves below the title bar, centered; on Windows and Linux a gear in the title bar opens Settings; on Windows Ctrl-V pastes, as in Windows Terminal; the pointer over the terminal is the arrow; the settings window drops General, whose Shell and Start in lead Terminal, which lists every shortcut, and its categories and page scroll apart; the release's macOS disk image signed with Developer ID and notarized on the signing runner; release v0.1.1 with the three builds.

## 17. Omarchy

- [x] T27 On Omarchy 4, an Omarchy theme wears the system's current theme and follows each switch, the default there; the v0.1.1 Linux tarball rebuilt with it, in place of the old one on the release.
- [x] T28 Blur on Hyprland, which the owner found absent on Omarchy: Hyprland blurs behind a window only while its own `decoration:blur:enabled` is on, which Omarchy 4.0.4 turns off; the PRD, README, AGENTS.md, the acceptance table and the release notes say so, and how to turn it on.

## 18. herdr

- [x] T29 Inside herdr the right column follows herdr's focused pane, which the owner found frozen: herdr keeps its panes' OSC 7, so on macOS and Linux ELYTerminal reads the folder from herdr's socket while herdr's client runs in the session's foreground.
- [x] T30 The main window opens at the size it was left at, on macOS, Windows and Linux, which the owner asked for: the size is kept in the settings.
- [x] T31 Release v0.1.2 with T29 and T30: version 0.1.2; the macOS disk image, the Windows zip and the Linux tarball, each from its script; the notes say it fixes herdr compatibility and keeps the window's size.
- [x] T32 On Windows the right column follows herdr's focused pane too, which the owner found frozen there: ELYTerminal finds herdr's client under the session's shell, reads its arguments and variables, and asks herdr's named pipe.

## 19. Column widths

- [x] T33 The sessions and right columns resize by a drag on their border toward the terminal, on macOS, Windows and Linux, which the owner asked for: in Ely, a `ShellColumn` resizable between a least and a greatest width; the two widths kept in the settings.

## 20. Prompts, program status and notifications

- [x] T34 OSC 133 from the shell integration, which the owner asked for with everything below: zsh, bash, fish and PowerShell mark each prompt's start (A) and end (B), a command's start (C), and its end with the exit status (D); the tap reads any program's OSC 133; each session keeps its last command's status.
- [x] T35 Jump to prompt: a prompt's first row is marked once the prompt is drawn; Cmd-Up and Cmd-Down, and Cmd-Shift-Up and Cmd-Shift-Down, on macOS, Ctrl-Shift-Up and Ctrl-Shift-Down elsewhere, scroll to the prompt before or after the view's top, as in Ghostty.
- [x] T36 OSC 7501 program status, revision 0.4: reports checked as the spec checks them; records per terminal, by id, with clear and a cap of 256; a new prompt drops working, blocked and idle records, a full reset all; the support query answered with the states and kinds.
- [x] T37 OSC 9;4 progress as the root record until an OSC 7501 report; OSC 9, 777 and 99 notifications read, OSC 99's chunks, base64, ids, occasions, actions, close and query included; ConEmu's other OSC 9 commands are no notifications.
- [x] T38 Inside herdr, herdr's agent states from its snapshot are the session's records.
- [x] T39 Session rows show the status: working with its progress, waiting by kind, done, error, a failed last command; hover shows each record's program, title and message; a hidden session that starts waiting marks its row.
- [x] T40 System notifications on macOS, Windows and Linux through gpui's: a session that starts waiting, finishes or fails, and OSC 9, 777 and 99 requests, while the window is inactive or the session hidden, rate-limited; a press shows the window and the session.

## 21. Release

- [x] T41 Release v0.1.3 with T32 to T40: version 0.1.3; the macOS disk image, the Windows zip and the Linux tarball, each from its script; the notes list what changed.

## 22. Status marks that move

- [x] T42 The status marks move, which the owner asked for after aicss.dev's orbs: 3 by 3 dots in the state's color; work ripples, or fills by its progress, waiting dips, a settled state draws its shape, then holds; the window draws again only at the marks' steps while one moves.

## 23. Release

- [x] T43 v0.1.3 rebuilt with T42, which the owner asked for in place of a v0.1.4: the version stays 0.1.3; the macOS disk image, the Windows zip and the Linux tarball, each from its script, replace the old ones on the release; the tag moves to their commit; the notes say what changed.

## 24. Keys, links and Ely

- [x] T44 herdr's shortcuts work, which a user found dead: `cmd+shift+ctrl+[` and `]` on macOS, and the same keys as herdr reads them on Linux and Windows: under kitty's flags a key Shift makes a symbol of goes as its own key with Shift, the symbol its alternate, on macOS and Windows; on Windows keys go as win32-input-mode records, which ConPTY asks for and herdr reads; on Linux gpui keeps no key code, which the docs say.
- [x] T45 Cmd-press on macOS, Ctrl-press on Windows and Linux opens the link under the pointer, which the owner asked for, wherever the terminal shows one: inside herdr, which wraps long URLs at its pane's edge and passes no OSC 8 target on, while herdr takes the mouse the press goes to herdr, which opens its pane's link whole, web links only, as herdr does, Cmd heard as Ctrl on macOS.
- [x] T46 Ely peeks over the main window's top edge from behind it, which the owner asked for after the 0.1.3 film, with several faces and moves; a switch in Settings, Terminal, turns her off: a window of her own kept on the edge, macOS's child window, Windows' owned popup, X11's popup moved and stacked, Wayland's anchored popup; ten faces from the icon's paths, following the sessions' marks.

## 25. Release

- [x] T47 Release v0.1.4 with T44 to T46: version 0.1.4; the macOS disk image, the Windows zip and the Linux tarball, each from its script; the notes list what changed.

## 26. Ely on Windows

- [x] T48 Ely on Windows stands on the main window's top edge in a box her own size, which a user found broken in 0.1.4: gpui opened her window as large as the display when her box's middle lay off it, so she sat at its bottom and the rest of it took the clicks meant for the windows behind; her window is a bare popup, without the caption, border, corners and shadow gpui's popup carries, and the glue sets her size from her own DPI as well as her place on the pane's top edge.
- [x] T49 Ely moves all the time, as she does in the films, which the owner asked for after 0.1.4, where she stood still and only a press made her wink: rigged from the icon's paths as the films rig her, she bobs, her side locks sway and her ahoge breathes, she blinks and glances on her own, and each mood moves her through that rig, smoothly.

## 27. Release

- [x] T50 v0.1.4 rebuilt with T48 and T49, which the owner asked for in place of a v0.1.5: the version stays 0.1.4; the macOS disk image, the Windows zip and the Linux tarball, each from its script, replace the old ones on the release; the tag moves to their commit; the notes give the new SHA-256 and what changed.

## 28. Ely's place, sleep and frame

- [x] T51 Ely stands inside the main window at its bottom left while no room lies above it, which the owner asked for after 0.1.4, where on a window at the screen's top or full screen she stuck between the menu bar and the terminal, on macOS, Windows and Linux: her bottom stands on the pane's top edge while her box fits under the top of the work area of the main window's display, else on the pane's bottom edge, as far in from the left; on Wayland, which tells a client nothing of its place, on the bottom edge while the window is full screen, maximized or tiled at its top as gpui reports it, which it does for a window it draws itself.
- [x] T52 A press wakes Ely from her doze, which the owner asked for: she dozes after five calm minutes since the later of her mood's start and the last press, so a press wakes her for five more.
- [x] T53 Ely's window shows nothing but her, which the owner asked for after a line showed along its top on macOS: there her window is a borderless panel, without the titled frame gpui's popup carries, whose edge macOS draws; on Windows and Linux her window shows no frame, border or shadow.

## 29. Release

- [x] T54 Release v0.1.5 with T51 to T53: version 0.1.5; the macOS disk image, the Windows zip and the Linux tarball, each from its script; the notes list what changed.

## 30. Ely's drag, updates and the scrollbar

- [x] T55 A drag moves Ely along the edge she stands on, which the owner asked for after 0.1.5: she follows the pointer sideways inside the pane, on macOS, Windows and X11 as it moves, on Wayland, whose popup keeps its place, once it lets go; where she stands, in points from the pane's left edge, the settings keep as `ely_left`.
- [x] T56 Settings › Terminal checks for updates, which the owner asked for after 0.1.5: under Ely, a switch checks ELYTerminal's latest release at start and once a day, on by default, and a button checks at once; the row says what the check found, and a newer release brings a button that opens its page; a newer release an automatic check finds posts a system notification, whose press opens that page.
- [x] T58 The terminal shows a scrollbar over its history, which a user asked for in issue #2 after 0.1.5, since a program that takes the mouse takes the wheel too: a thumb along its right edge, in Ely's sizes and colors, stands where the view stands in the history, and a drag on it moves the view through it; none shows without history, as on the alternate screen, and a press on it never reaches the terminal.

## 31. Release

- [x] T57 Release v0.1.6 with T55, T56 and T58: version 0.1.6; the macOS disk image, the Windows zip and the Linux tarball, each from its script; the notes list what changed.

## 32. User feedback after 0.1.6

- [x] T59 A plain click on the line typed at the shell prompt moves the cursor there, which a user asked for after 0.1.6, as Ghostty's `cursor-click-to-move`: while the shell's OSC 133 marks say it reads a line, a click that does not drag sends Left or Right arrows, one per character, to the clicked cell of the cursor's line, soft wraps joined; never on the alternate screen, under mouse tracking, with a selection, or before the prompt's end.
- [x] T66 Files dropped on the terminal type their paths, which a user asked for after 0.1.6, as macOS Terminal does: each path quoted for the shell, joined by spaces, sent as a paste, on macOS, Windows and Linux.
- [x] T60 Fallback fonts, which a user asked for after 0.1.6: `font_fallbacks` in the settings, a list of installed families tried in order for a character Font family lacks, CJK among them; Settings › Appearance lists them under Font family, each an Ely `FontPicker` with a remove button, and an Add button; each saves only when gpui loads it.
- [x] T61 On Windows the IME's candidate window stays at the cursor, which a user found jumping after 0.1.6: find why it moves, fix the cause, and log what the input handler tells the IME. Not reproduced on the owner's PC, so the owner closed it; the caret the IME is told is logged.
- [x] T62 Ely's body runs on below the edge she holds, which a user asked for after 0.1.6, where a hop lifted her off the window and left a gap under her collar: her shirt continues down, hidden behind the edge, so a hop shows more of her and never a gap.
- [x] T63 A double-click on a session's title renames it, which a user asked for after 0.1.6: the row's title turns into a field, Enter keeps the name, Escape drops it; the name shows in the row, the terminal's header and the window's title over the program's own until renamed again, and an empty name gives the title back to the program; it lives as long as the session.
- [x] T65 Dragging the window on Windows 10 with Background blur on follows the pointer, which a user found lagging seconds behind in issue #3 after 0.1.5: find where the lag comes from in gpui's acrylic path, fix the cause on Windows 10 alone, log what changes, and reply on the issue.

## 33. Release

- [x] T64 Release v0.1.7 with T59 to T63, T65 and T66: version 0.1.7; the macOS disk image, the Windows zip and the Linux tarball, each from its script; the notes list what changed in English and Chinese.

## 34. Issues and pull requests after 0.1.7

- [x] T67 Windows sessions take the registry's environment and a default shell of `pwsh`, then `powershell`, then `cmd`, from PR #5: a folder added to `PATH` after ELYTerminal opened reaches the next session, a `%NAME%` the registry holds is expanded, `PATH` names each folder once, and the chosen program runs as chosen, a path with a space too.
- [x] T68 A session whose shell is `wsl` follows its folder, which a user asked for in issue #3: the distribution's bash, zsh or fish loads ELYTerminal's integration, its OSC 7 maps to `\\wsl.localhost\DISTRO\PATH`, or to the drive a `/mnt` path names, so Files and Git follow it, git running inside the distribution.
- [x] T69 Natural text editing on macOS, from PR #8: a Settings › Terminal switch, off by default, sends Option and Command's editing keys as readline's bytes.
- [x] T70 A right press on a session's row duplicates or closes it, and a drag moves the row, from PR #9, which issue #7 asked for: Duplicate session opens the shell that row runs in the folder it last reported, right after it and shown.
- [x] T71 A command's end posts a notification only after the command ran 5 seconds, as Ghostty's `notify-on-command-finish-after`, from PR #11, which issue #10 asked for: omp's own OSC 133 marks around each message post nothing.

## 35. Release

- [x] T72 Release v0.1.8 with T67 to T71: version 0.1.8; the macOS disk image, the Windows zip and the Linux tarball, each from its script; the notes list what changed in English and Chinese; a short thanks on issues #3, #7, #10 and PRs #5, #8, #9, #11, then each closed; issue #2 closed without a word.

## 36. The sessions column's switches

- [x] T73 Settings › Terminal turns off either half of T70, which the owner asked for after 0.1.8: Drag sessions to reorder, a switch on by default, leaves a drag on a row moving nothing and the rows on `List`'s own gap; Session menu, a switch on by default, leaves a right press on a row opening nothing. A settings file without either field takes it on, so an older file still drags and still opens the menu. The owner turned both off and on in the app and found each took effect.
