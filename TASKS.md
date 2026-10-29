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

## 36. Issues and pull requests after 0.1.8

Each task ends with a short thanks on its issue or pull request, then closes it.

- [x] T73 On Windows output reaches the screen as the program writes it, which a user found waiting for a key in issue #19 with fastfetch and llama-server: find where the bytes wait, between ConPTY, the reader thread, the tap, alacritty's event loop and the view; fix the cause; log it.
- [x] T74 Git for Windows' bash and sh follow their folder, which a user asked for in issue #13 with `C:\Program Files\Git\bin\sh.exe`: they load ELYTerminal's integration and name the folder as Windows names it, so Files and Git follow.
- [x] T75 A program's OSC 7501 records outlive the prompt marks a running command prints, from PR #17, which omp needs (issue #10): only a shell's own prompt drops working, blocked and idle records, as the spec asks. Three reviews spent, the cap: round 3's two findings were fixed in 89b4873 and checked against every probe of the three rounds, with no fourth review. T92's review passed its last fix and its recorded limit, a shell the integration does not reach, nested or over ssh, which keeps its own group's records at its prompts, a limit T93 narrows; the owner accepted it at the cap on 2026-10-24.
- [x] T76 Wide characters keep their columns, which a user found broken in issue #22 under codex on Windows: an erase that splits a wide character clears all of it, and text after one stays in its column.
- [x] T77 Option as Alt works under macOS input methods, from PR #18: with a Chinese input source, `⌥f` sends `ESC f`, and composition still works.
- [x] T78 A quote types on its first press on Windows, which a user found in issue #12: find which layout or input method holds it, fix the cause, log the path a key takes.
- [x] T79 Backspace deletes in omp on Windows, which a user found in issue #21: find what omp reads for it through ConPTY, and send what it expects.
- [x] T80 Chinese input methods commit on X11 without a crash, from PR #15: the compound text decoder gpui's XIM uses reads every set libX11 writes, through libX11's own tables, GB2312, Traditional Chinese's JIS X 0212 and Big5, Big5-HKSCS and GBK segments among them.
- [x] T81 A wheel notch on Windows scrolls the lines Windows sets, which a user found tripled in issue #24: gpui's lines there already carry the system's count, and on Linux 3; one screen at a time scrolls a screen, and Shift with the wheel, which takes it back from a program that tracks the mouse, scrolls as where nothing tracks it. Three reviews spent, the cap: round 3's finding was fixed in c32fe52 and checked by the review's own probe over 12,800 splits of a notch, with no fourth review. T92's review passed its last fix and a paged notch's parts; the owner accepted it at the cap on 2026-10-24.
- [x] T82 Ely stays with the main window on KDE Plasma's Wayland, which a user found floating over other windows, draggable while the window slept, and left behind at minimize in issue #27.
- [x] T83 Settings › Terminal switches for the sessions column's drag and its right press, from PR #14.
- [x] T84 A long branch name keeps "Amend last commit" readable, which a user found squeezed in issue #20: Ely's `CommitInput` moves its commit button below when both do not fit, from the reporter's fix; ELYTerminal pins that Ely.
- [x] T85 The interface in Simplified Chinese, which a user asked for in issue #26: a Language row in Settings › Appearance, English or 简体中文, through Ely's `I18n`.
- [x] T86 Start in knows WSL and takes a folder of your own, which a user asked for in issue #23: Home starts a `wsl` session in the distribution's home, and a third choice starts every session in a chosen folder.
- [x] T87 Linux DEB, RPM and AppImage packages, from PR #16.
- [x] T88 The right column's Git tab takes a press while the window is maximized or full screen on GNOME's Wayland, which a user found dead in issue #28: reproduce it under GNOME, find the cause, fix it or log what the next report needs.
- [x] T89 Issue #25's kind words thanked and closed; every issue and pull request above answered; the acceptance table checked line by line. Three reviews spent, the cap: round 3 found row 156 certifying an update notification's press that a Check now, running or failed, loses; T90 fixes that in the code. T92's review passed T90's fix of its last finding; the owner accepted it at the cap on 2026-10-24.
- [x] T90 The update notification's press opens the page it named, as T56 asked, whatever a check since has found or is doing, which T89's third review found lost to a Check now running or failed.
- [x] T91 Every comment written since 0.1.8, in ELYTerminal and in its Ely and alacritty commits, fits one line of ten words, as the owner's rules ask; the reviews checked lines, not words.
- [x] T92 The last fixes of T75, T81 and T89, which spent their three reviews, pass a review of their own: every probe of their rounds runs on HEAD, and their rows and decisions hold.
- [x] T93 A shell nested in a session drops its own live records at its prompts, which T92's review found kept: a fish started inside zsh, its own marks unnamed, kept what its builtins reported. An unnamed prompt from another group is a shell's when that group's leader runs zsh, bash or fish started with options alone and the group holds it alone, or the leader is fish and its line takes clicks; a wrapper's script or `-c`, a subshell, a shell with job control off and a fish script's `read` still keep the program's records. Three reviews spent, the cap: round 3's finding, a fish script's `read`, whose line takes clicks as fish's own prompt does, read as a shell's prompt, was fixed in 6dc0409 and checked against every probe of the three rounds, 58 in all, with no fourth review. The owner accepted it at the cap on 2026-10-24.

## 37. Release

- [x] T94 Release v0.1.9 with T73 to T93: version 0.1.9; the macOS disk image, the Windows zip, and the Linux tarball, DEB, RPM and AppImage, each from its script; the notes list what changed in English and Chinese and credit the pull requests' authors.
- [x] T95 The 0.1.9 update film: about 30 seconds in the 0.1.7 film's style, Ely presenting what 0.1.9 does in a real recording, captions in English and Chinese; its files stay in the untracked `videos/`. Rendered at 1080p30, 30 s, 23 MB, from the real take; the owner ticked it without a review.
- [x] T96 A failed settings save names the step the system refused, create, write or rename, after CI's first Windows attempt on 8c76ba3 failed `overlapping_saves_leave_one_whole_file`: `Access is denied` on two of the four writers' first saves, under one message for all four steps. The owner's PC did not repeat it in 300 runs of the test, 60 of the suite and 20 at sixteen writers, so its cause stays unknown and no fix is guessed.

## 38. Nested shells and the Windows save race

- [x] T97 A nested shell started with an option's argument, `fish -C CMD`, `bash --rcfile FILE`, `zsh -o OPT`, reads as interactive: each shell's options that take an argument are known. Three reviews spent, the cap: round 3's finding, bash's single-dash `-rcfile` and `-init-file`, was fixed in 1d1956a with no fourth review; the owner accepted it at the cap on 2026-10-25.
- [x] T98 A nested nushell and a nested PowerShell drop their own live records at their prompts, as zsh, bash and fish do. Three reviews spent, the cap: round 3's finding, PowerShell's `-EncodedArguments` value and its Unicode dashes and blanks, was fixed; the owner allowed more rounds, and round 5 passed on bfc8631.
- [x] T99 On Windows, which has no process groups, a nested shell's prompt is known from the process tree under the session's shell. Three reviews spent, the cap: round 3's finding, an unknown start time in the tree, which now leaves the prompt's holder unknown, was fixed; the owner allowed more rounds, and round 5 passed on bfc8631.
- [x] T100 A shell over ssh, under sudo or in a container drops its live records at its prompts: find what the terminal can know of it, and do it, or record why not. Done: a root shell under sudo, su or doas, on the pty itself by its name or on their own pty through that pty's foreground group; a shell over ssh, mosh or in a container sends what omp sends through the same local process, clicks too, a limit only an integration on the far side could lift.
- [x] T101 The Windows save race: run CI's Windows job until `overlapping_saves_leave_one_whole_file` fails again with the named step, then fix its cause. On 2026-10-25 the job ran 25 times more on 1ccf725's run 40173728077 without a failure, beside 380 runs on the owner's PC: about one failure in 435 runs. Its cause stays unknown; T96's step names wait in the log for the next one. The owner closed it as a known issue on 2026-10-25: the next failure's log names the step, and its cause is fixed then.

## 39. Pull requests and issues

- [x] T102 PR #37: each row shapes by foreground color section; checked on macOS, Windows and Linux, merged with its author. Each grapheme cluster sits on its grid column, as Ghostty draws, which the owner chose over main's drift; three reviews spent, the owner allowed a fourth, and round 4 passed on 4838df5.
- [x] T103 PR #34: on the owner's Windows PC, find whether ConPTY drops a program's mouse modes; merge the fix, or fix the real cause; zcg's authorship restored. The system's conhost drops them and OpenConsole passes them, so sessions run on Microsoft's bundled OpenConsole, as the owner chose; the PR's IME fix landed as zcg's c37ea46. Round 2 passed on 5ef7b5e.
- [x] T104 PR #35: the README split into English and Chinese guides; every claim and link checked against the code and v0.1.9, then merged. Round 2 passed on 7ed2327.
- [x] T105 Issue #33 and PR #36: Ely's `Tree` takes a right press, on Ely's `elyterminal-0.1.1` branch and its main; Files' right-press menu, Open, New session here and Copy path, behind a Settings switch, off by default; no vendored Ely; the author credited. Ely's main held `Tree::on_context` already, so its tree part went to the pin's branch as 494c17d; the PR's code landed as myflycat's 0863969. Round 2 passed on ac3a967.
- [x] T106 Issue #29: Shift types capitals and Ctrl-Shift shortcuts work on niri's Wayland; the cause found and fixed, and logged. Unreproduced under niri 26.04 with fcitx5 5.1.22 on the Omarchy machine; per the goal's rule, no guessed fix: each key down logs its keystroke at trace, the reporter was asked for full logs, and #29 stays open. Round 2 passed on 979c02a.
- [x] T107 Issue #30: on Windows a fallback font's italic draws its own glyphs; the cause found and fixed, and logged. gpui-pre-windows rebuilt DirectWrite's served face by name and drew Maple's Italic glyph ids from its Regular file; the org's fork draws with the served face. Round 1 passed on f79691b.
- [x] T108 Issue #32: the main window opens on the display and at the place it was left, or centered on the primary display when that display is gone. `window_place` keeps the display's gpui uuid and an offset; the window opens inside that display's visible area, else centered, logged. Five reviews spent; round 5's finding, a monitor gone from one X screen whose middle the screen still holds, stays a recorded limit, since gpui's X11 display is the screen; closed under the owner's standing word.
- [x] T109 Issue #31: an Add card after the last theme makes a custom theme from Ely Dark, whose colors the user sets; built-in themes never change; a background image setting. New theme copies Ely Dark into `custom_themes`; the worn one's name, 21 colors and Delete; `background_image` under the terminal at the background's opacity. Round 3 passed on 8821701.

## 40. Ghostty 1.4

- [x] T110 Shift+Enter makes a new line in codex and Claude Code, through the kitty keyboard protocol, on macOS, Windows and Linux; proven in each, fixed where not. macOS: codex 0.163 set kitty's disambiguate, events and alternates, Claude Code 2.1.303 disambiguate and alternates; Shift+Enter went as `CSI 13;2u` and each composer held two lines. Linux, under niri on the Omarchy machine: both set the same flags and Shift+Enter went as `CSI 13;2u`; neither agent is signed in there, so no composer showed. Windows, on the owner's PC through the bundled OpenConsole, headless, since the owner was at its desktop: codex 0.165.0 set no kitty flags, so ConPTY's win32-input-mode carried Shift+Enter as ELYTerminal's records, `CSI 13;28;13;1;48;1_` and its release, and Claude Code 2.1.312 set disambiguate and alternates, so it went as `CSI 13;2u`; each composer held two lines. The probe drove ELYTerminal's encoder and pty, not gpui's dispatch of a real key, which the owner's desktop kept from this run.
- [x] T111 APC reaches the terminal: an org fork of vte dispatches APC strings, and the alacritty fork hands kitty graphics commands to ELYTerminal at the cursor. The org's vte fork dispatches APC; the alacritty fork's `Term` hands each `G` command to a `Graphics` hook at the cursor; the tap passes APC, DCS, SOS and PM strings in order. Round 3 passed on ff8d0f7.
- [x] T123 A process mid-exec is no hidden process: on macOS the holder read a group's arguments while it `exec`ed, macOS answered `EIO`, and the holder took it as another user's hidden arguments, so `sh -c 'echo ready; exec sleep 30'` read as a shell; `only_an_interactive_shell_leading_its_group_draws_its_prompt` failed so under load in T109's review and T111's check.
- [x] T112 Kitty graphics transmission: direct, file, temporary file and shared memory; RGB, RGBA and PNG; zlib; chunks; ids and numbers; responses and quiet; storage limit; delete commands.
- [x] T113 Kitty graphics placements: at the cursor, the cursor moved after them; z-index under text, under the ground and over text; source rectangle, cell size, offsets; images scroll with text, scroll regions clip them, clears delete them as kitty does.
- [x] T114 Kitty graphics Unicode placeholders and relative placements (P, Q, H, V).
- [x] T115 Kitty graphics animation: frames (a=f), composition (a=c), control (a=a: state, loops, gaps, current frame); playback pauses while its session is hidden.
- [x] T116 Image payloads decode in bulk with SIMD; measured against byte-at-a-time decoding.
- [x] T117 A hidden session releases its images' GPU textures and stops drawing them, and rebuilds them when shown; measured.
- [x] T118 Kitty clipboard protocol, OSC 5522: writes allowed up to 64 MiB; each read asks, with a preview and a choice to remember for the session; text, images and file lists; paste events under mode 5522.
- [x] T119 Scrollback compression: idle history compressed in the alacritty fork on macOS, Windows and Linux, decompressed on demand; resident memory measured.
- [x] T120 Throughput: plain ASCII through the tap and the parser measured, the tap's scan made bulk, the numbers recorded.
- [x] T121 Renderer: on Linux, through gpui's wgpu, text is crisp at 125% and 150% and nothing tears, on Vulkan and on GL; checked on the Omarchy machine, fixed where not, recorded.
- [x] T122 PRD, AGENTS.md and `tasks/acceptance.md` hold every change of T102 to T121; the acceptance table checked row by row.

## 41. 0.2.0

- [x] T124 Issue #29, again: on niri's Wayland with fcitx5, Shift held by its key reaches the keystroke's modifiers, so Ctrl+Shift+C and Ctrl+Shift+V copy and paste, and under fcitx5's Shift trigger too; reproduced first, as the reporter's log shows it, then fixed. The window's state reads the masks, as gpui's did; a shadow xkb state takes every key, and only a key no mask answered settles its modifiers into the window's, the group left to the masks; seventeen review rounds, a fuzz test against a compositor's own state.
- [x] T125 Issue #38: on Windows 11 the window's backdrop may be Mica or Mica Alt beside Acrylic, a setting in Settings › Appearance; Windows 10 keeps Acrylic.
- [x] T126 The README, its Chinese twin and the guides in `docs/en_us` and `docs/zh_cn` say what 0.2.0 does, each claim true to the code.
- [x] T128 Issue #39: on Windows, PowerShell's `cd` takes Git Bash's drive paths, `/e` as `E:\` and `/e/x` as `E:\x`, where no such path lies on the current drive, through ELYTerminal's PowerShell integration; every other `cd` as before.
- [x] T127 Release 0.2.0: the version raised, the builds for macOS, Windows and Linux on the release, the notes in English and Chinese, the website's links following. v0.2.0 at 0cef65d, published with the signed and notarized disk image, the Windows zip and the four Linux packages, each with its version-less alias, which the website's links follow; the notes reviewed in three rounds.
- [x] T129 Mica draws no caption buttons of DWM's behind Ely's: the Windows fork stops extending the frame, the backdrop drawn behind the clear client area; 0.2.0's Windows zip rebuilt and replaced on its release.
