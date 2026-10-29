# Reviews

Each task gets at most three codex rounds. Each round: the commit, the verdict, the findings and what was done.

T01 to T49 are in `reviews-t01-t49.md`.

## T50

- Round 1, a54f4ff and 5d10979 with v0.1.4 rebuilt: PASS. The reviewer found T50's code, resources, scripts and workflows those of a9be8ab, where CI run 1253 passed on macOS, Ubuntu and Windows; read Release run 1256 notarize and staple the disk image at a54f4ff and put it on the release; downloaded the three files anonymously against the notes' SHA-256, each unlike the first 0.1.4's; matched the tag's source archive to a54f4ff file by file and the notes to notes.md; checked the disk image's ticket, signatures, Gatekeeper, version 0.1.4 and macOS 11.0, the zip's icon and manifest against the first build byte for byte and its imports, no C runtime DLL among them, and the tarball's files, modes and glibc 2.35; and ran the downloaded app with a HOME of its own, where Ely stood 112 by 124 on the window's top edge and moved between captures. check.sh passed. Windows and Linux windows, live multi-display DPI and Wayland went unexercised.

## T51

- Round 1, 15ef5d5: FAIL. On Wayland her bottom anchor kept the height it opened at, and a change of state at the same size never moved her; on X11 the room ran to the root's top, not her monitor's; Sway's tiling went unseen. Fixed in e6c0c14: twice a second she checks where the window's size and state put her anchor, counted from the pane as gpui counts it, and opens there again; X11 reads RandR's monitors; tiling counts where gpui reports it, for a window it draws itself, as T51's line and AGENTS now say.
- Round 2, e6c0c14: FAIL. The reviewer's replays under headless Sway moved her anchor (140,0) to (140,999), (140,799) and (140,999), and full screen at the same size moved her; but X11 picked the monitor under her bottom left corner, so over a window held mostly by a staggered monitor she stood over the gap beside it. Fixed in 8b64798: the monitor holding most of the pane, as macOS and Windows pick a window's display.
- Round 3, 8b64798: PASS. The staggered replay put her at 112x123+1240+877, her bottom on the pane's bottom; the stacked one at 112x123+240+1277; check.sh passed with 238 tests and Linux with 245. Physical multi-monitor screens went unexercised.

## T52

- Round 1, 0eb8fc5: PASS. The reviewer ran the two new tests against the parent's look.rs, where after the wink she stayed Doze with Lines eyes, and against the change, where they passed; its probes covered the 299.999 and 300 s edges, a second press counting again, an old press before a new mood, a finish's count, work, waits and errors, a press with no records at all, the zs fading, and 242 poses over three weeks; check.sh passed with 240 tests and Linux with 247. A native click, frame by frame, and five real minutes went unexercised.

## T53

- Round 1, a6c044f: PASS. On macOS 27.2 the reviewer read her panel back as mask 0x80, no shadow, key only if needed, never key, a child of the main window, its frame and content rect alike; her window 112 by 124, inside the bottom edge under the menu bar; the top 32 rows and the outer columns of her picture clear; abc reached cat. A probe that kept setStyleMask: from taking logged 'pet: Ely goes, as her window cannot be kept there' and 'pet: her panel keeps style mask 0x8081', left no child window, and the main window still took abc. check.sh passed with 240 tests. This macOS draws no edge for a titled panel, so the hairline's going awaits the owner's macOS; Windows and Linux rest on the captures supplied.

## T54

- Round 1, 0119776 and 15153b4 with v0.1.5 published: PASS. The reviewer found the bump only elyterminal's version; downloaded the three files anonymously, each matching the notes' SHA-256 and target/t54/anon byte for byte, and the tag's source archive the commit's 141 files; checked the disk image's ticket, strict signatures and Gatekeeper, the hardened arm64 app at 0.1.5 and macOS 11.0, the zip's GUI executable with its seven icon images, icon group 1, PerMonitorV2 manifest and no C runtime DLL, and the tarball's modes and glibc 2.35; found T51's and the failed glue's markers in all three binaries, T53's in the app and the monitors' in the tarball; and read CI run 1311 succeed on macOS, Ubuntu and Windows at 0119776 a minute before the release was published, and Release run 1312 on the signing runner. check.sh passed with 240 tests. Windows and Linux windows went unexercised.

## T55

- No review: the owner asked for 0.1.6 without one. Checked by scripts/check.sh (240 tests), Linux tests (247) and clippy, Windows gnu clippy, an xdotool drag under Xorg, a drag under Sway nested in Xorg, and the owner's own drags on macOS.

## T56

- No review: the owner asked for 0.1.6 without one. Checked by scripts/check.sh (243 tests), Linux tests (250) and clippy, Windows gnu clippy, and under Xorg's dummy screen the start's check, Check now, the switch, and a build named 0.1.4 that found 0.1.5 out and opened its page.

## T58

- No review: the owner asked for it at once, in 0.1.6, without one. Checked by scripts/check.sh (244 tests), Linux tests (251) and clippy, Windows gnu clippy, and under Xorg's dummy screen drags of the thumb through 500 lines, with mouse tracking on as well.

## T57

- No review: the owner asked for 0.1.6 without one. v0.1.6 published at 8a9941b once CI run 1342 passed on macOS, Ubuntu and Windows; its disk image from Release run 1343, notarized, its zip and tarball from their scripts on the owner's PC and its WSL; a first build at 9e185ea, held back for the scrollbar, was replaced on the draft and never published; each file downloaded back anonymously matches its SHA-256 in the notes.

## T59

- Round 1, 82cfe91: FAIL. fish 4.9 moves a code point per Left, so a counted click put `X` inside `é`; fish's redraw dropped the wrap flag of a long line, so a second click on its next row moved nothing; a press let go in another cell still moved the cursor; a program that turned on mouse tracking while the button was down still got arrows. Fixed in 0c55d41: OSC 133 A's `click_events`, which fish sends on its own, makes the click go to the shell as an SGR press, as kitty and Ghostty send it; the release must land in the press's cell, and the program must track no mouse at the release.
- Round 2, 0c55d41: FAIL. The four findings of round 1 replayed fixed, and zsh, bash, the prompt jump and the shell tests held; but a click on fish's `$` went to fish as a press, and fish moved its cursor to the start, where T59 asks for clicks past the prompt. Fixed in e74ea74: the press goes only past the prompt's last cell before the cursor, and never on a prompt cell.
- Round 3, e74ea74 and 9fa68b2: a first run found that fish's empty left prompt beside a right prompt carrying its own marks panicked in `click::press`, then stopped without a verdict when Codex's content filter cut it off; 9fa68b2 fixed the panic. A second run, at 9fa68b2: FAIL. Every earlier finding replayed fixed and the panic gone, but a prompt that prints nothing, fish's empty left prompt with a plain or no right prompt, left no cells to find the input by, so clicks on the typed line did nothing. The owner chose a fix and a fourth round over leaving it documented. Fixed in 84364ce: OSC 133 B stamps the line typed after a prompt with a link of its own, numbered by the commands ended, which every prompt drawn again on that line shares, so the input starts at its first typed cell where no prompt cell of its line comes before it.
- Round 4, 84364ce: FAIL. Round 3's finding replayed fixed, every earlier one held, and the rewrite passed 5,291 split reads, 15 pty replays, Cmd-press in prompt, input and output, bash 3.2, find, select all and copy; but the newest typed line was taken by its place, lowest on the screen, so after `printf '\e[H'` fish's prompt at the top over older lines took no click. Fixed in 28d7615: the newest line is the one of the highest number, and a press goes no lower than its last row or the cursor's, where fish moved its cursor for a click on older text below. Round 4 was the last the owner allowed.
- Round 5, 28d7615, which the owner allowed: PASS. fish's prompt drawn at the top over older lines took its click, `abcXdef`, and a click on the older text below did nothing; every earlier finding replayed fixed; zsh and bash arrows, the prompt jump, Cmd-press in prompt, input and output, bash 3.2, find, select all and copy held; 5,291 split reads and 15 pty replays of the rewrite matched; check.sh passed with 256 tests. Windows' win32-input-mode and Linux's native clicks went unexercised.

## T66

- Round 1, 545fb46: FAIL. On Windows a path went bare, or in double quotes when it held a space, so PowerShell expanded `$HOME` and `$(1+1)`, broke on `it's`, and ran `a;Write-Output(42)` as two statements, and cmd kept `&` live; and `^` went unescaped, so zsh with `extendedglob` read a dropped `a^b` as a glob and passed four files. zsh, bash and fish read 317 paths back exactly, controls and bad UTF-8 stopped the drop, and selection, click to move, mouse reports and the scrollbar held after a drop. Fixed in 887378a: a path stays literal for the shell the session started, by its name: backslashes for POSIX shells, `^` and `~` among them; single quotes with their quotes doubled for PowerShell; double quotes for cmd, which refuses a `%`.
- Round 2, 887378a: FAIL. PowerShell read 384 hard names back exactly and a real pwsh session logged its quoting; zsh with `extendedglob`, bash and fish read 376; but cmd, from its docs and parsers, expands `!NAME!` inside quotes under `/v:on`, and `"C:\"` reads to a program as a quote, joining the next path. Fixed in 5660562: a `!` stops a cmd drop, and the backslashes ending a path are doubled; node, run from a cmd batch line on the owner's PC, received six hard paths exactly, `C:\` among them.
- Round 3, 5660562: PASS. A `!` anywhere in a cmd drop stops it with no byte sent, and doubled trailing backslashes keep `C:\` and the next path apart under MSVCRT and UCRT parsing; 1,062 cmd paths, UNC and runs of up to 32 backslashes among them, kept their contents, order and bounds; zsh, zsh with `extendedglob`, bash and fish read 376 paths back exactly and PowerShell 384; check.sh passed with 261 tests. Windows' own cmd and ConPTY were checked only through the owner's PC run.

## T60

- Round 1, cb32f00: FAIL. On macOS the order, bold, italic, ligatures, icons, old files, refusals, add, change and cancel all held; but removing the row whose input held the focus left the removed family in it, and Enter saved it over the next family; and on Linux gpui's cosmic-text path carries the fallback a character took into the next while it covers it, so `क中` drew 中 in the second fallback. Fixed in 5f8ea89: a removal drops that row's input and the focus; the Linux path is gpui's, kept for its shaping clusters, and the owner chose to record it.
- Round 2, 5f8ea89: FAIL. Round 1's removal case and removals first, middle, last, in sequence, then adds and cancels held, and the Linux record was true; but the pickers were keyed by place, so after a removal moved the inputs, the picker state of the second row, listening to the input now first, wrote PingFang SC into it at a blur, and Enter saved `[PingFang SC, PingFang SC]`. Fixed in 5d131e5: each picker is keyed by its input's entity.
- Round 3, 5d131e5: PASS. Each picker keyed by its input kept round 2's case at `[Songti SC, PingFang SC]` through a removal, a focus cycle and Enter; drafts blurred after removals first, middle, last and among duplicates came back to the right family; a sequence of removals and adds through five focus cycles, and a cancel, re-add and removal through three, saved what the screen showed; a refused pick restored the saved list; check.sh passed with 262 tests. Windows and Linux windows went unexercised.

## T61

- No review: the owner closed it once it did not reproduce. On the owner's Windows 11 PC (26H2, 150%), with WeType and powershell.exe, 23 compositions put the candidate box at the caret: three on one line, then one on each of three lines below, fifteen committed and begun again at once, the caret stepping 4 cells each time, and eight under a status line redrawn every 100 ms with the cursor hidden; each caret the IME asked for matched the cursor. Microsoft Pinyin went untested.

## T62

- Round 1, 760f054: PASS. 24 poses rendered by gpui beside the build before, rest, breath, tilts, hops, a press in a hop, typing, a shake, the doze, waking and rising, kept her outline above the shoulders and joined the shirt and hair to the edge in every hop; 3,244,407 poses sampled, head bob, tilt and the rotated hair's corners counted, left the shirt 4.4 and the hair 1.3 units below the edge at her highest, 52.7 units; on macOS 48 snaps at the top and bottom edges and 52 through presses dispatched to her window kept her bottom row opaque; check.sh passed with 263 tests. Windows, X11 and Wayland went by their source.

## T63

- Round 1, 3085a94: FAIL. Presses, Enter, Escape, a blur by a press, other rows, closes, retitles, a full reset, long and Unicode names held, and Ely's `ListItem::editor` left other rows as they were; but leaving the app mid-rename blurred the field while the window's focus still pointed at it, the field went, and back in the app the terminal heard no key until pressed. Fixed in ca3809b: a field that closes while it still holds the focus gives it to the shown terminal; a press that took it elsewhere keeps it there.
- Round 2, ca3809b: PASS. Back in the app after leaving it mid-rename, `a` reached the program with no press first; 18 scenarios held: Enter, Escape, a blur by a press, presses inside the field, switching and opening sessions, renaming two rows, closing the other session, the renamed one or by Cmd-W, Settings' fields and the Git summary keeping their focus, a triple press, a name of spaces and a later program title; check.sh passed with 263 tests. Windows and Linux windows went unexercised.

## T65

- Round 1, 1384ef5: FAIL. Moves and sizes, clear and opaque windows, changes at rest, blur turned off mid-move, per-window marks, destruction, the Windows 11 boundary and the structure layouts held in a replay against recorded Win32 calls; but blur turned on mid-move, which gpui sets as acrylic at once, kept acrylic for the rest of the move, since Windows sends `WM_ENTERSIZEMOVE` once. Fixed in 0a30760: a moving mark from `WM_ENTERSIZEMOVE` to `WM_EXITSIZEMOVE`, and a window blurred while it holds the mark takes the clear accent again.
- Round 2, 0a30760: PASS. The build before failed its two mid-move cases and the fix passed them; 46,875 sequences of six actions from opaque, clear and blurred starts, settings, moves, ends, destruction and a second window among them, kept the accent, both marks and the frame refreshes as a model says; check.sh and the Windows type check passed. Windows 10 itself went untested.

## T64

- Round 1, 526e00a with v0.1.7 published on GitHub at f360143: PASS. GitHub's main and the tag name f360143, which CI run 39348049724 tested on macOS, Windows and Linux; the release is published, no prerelease, and `releases/latest` answers it; the three files downloaded anonymously match the notes' SHA-256; the disk image is stapled, notarized, arm64, 0.1.7 and macOS 11, beside Applications; the zip's executable a GUI program with its seven icon images and PerMonitorV2 manifest, byte for byte 0.1.6's, and no C runtime DLL; the tarball's three files and modes, glibc 2.35; each binary holds GitHub's `releases/latest`, no Gitea host, and the code of T59, T60, T62, T63 and T66, the Windows one T65's too; the notes match the work.

## T67

- Round 1, e887f31: FAIL. PR #5's own expansion ran eight rounds and grew a name that named itself without bound; an App Execution Alias ahead of PowerShell 7 hid it; `TERM`, `COLORTERM` and the integration's variables went in beside a `term` the registry held, and alacritty kept one at random; a registry `LANG` gave way to the default; a failed registry read started the session on a mixed environment; Unicode names went unmatched. Fixed in 4ffd79f: `term::env` reads as Windows Terminal's `til::env::regenerate`, each key's plain values first, then those to expand, each once; names compare through `CompareStringOrdinal`; `Env::set` replaces a name under any case; a registry error fails the start, naming its key; the shell search goes past an alias down `PATH`.
- Round 2, 4ffd79f: FAIL. F1 to F6 held; but a `PATH` only this process held was lost where the registry named none, and a folder whose name held a `;` was split again. Fixed in 15c47dd: `settle` keeps each path variable this process held; each folder reaches the search through `join_paths`.
- Round 3, 15c47dd: PASS. 24 probes held, F7 and F8 among them; check.sh, the Windows clippy, and the env, shell and win32 tests on the owner's Windows 11 PC passed.

## T68

- Round 1, 112598b: FAIL. fish 4, which reads `?` as no glob, passed every drive folder by; a network share went as another host's folder; names WSL shows Windows through U+F000 escapes reached git as they were; zsh under `nounset` lost its folder outside WSL. Fixed in 84ea235: fish matches a drive by regex; every share goes with an empty host, and `tap` lost its WSL host rule; `git::wsl` maps the escapes both ways, Files' marks too; zsh reads `${ELY_WSL-}`.
- Round 2, 84ea235: PASS. Round 1's 90-case matrix, its launcher and git harnesses, 130 escape cases and 11 parser cases held; check.sh, the Windows clippy and the Linux tests passed; the owner's PC followed `/tmp/ely t68/re:po` and read its git through `wsl.exe`.
- Round 3, 305b650: PASS. CI then failed on Windows: the reused runner's checkout kept the PowerShell script's carriage returns from before `.gitattributes`, which PowerShell reads, so the guard reads only the scripts WSL runs; 15 guard cases and a stale-checkout replay held.

## T69

- Round 1, 60ea60e: FAIL. A key's release took the natural editing path by the modifiers left at release, so Option-Left let go after Option sent kitty's Left release, and Left pressed plain lost its own once Option went down. Fixed in 4f54998: each held key remembers how its press went, bytes, a Windows record or natural editing, and its repeats and release follow it.
- Round 2, 4f54998: PASS. The finding's three chords and its reverse held under kitty's flags 31; Windows' record rule matched the old one in 576 combinations; check.sh and the Linux and Windows lints passed.

## T70

- Round 1, 5560e2a: FAIL. Duplicate and Close held across shells, moves and closes under an open menu; but a drag selecting text in the rename field lifted the row too and moved the session. Fixed in a455df9: the field's wrapper keeps a left press from the row and its drag.
- Round 2, a455df9: PASS. The gesture selected `GHIJK` with the rows in place; presses, double presses, Enter, Escape, blur, a right press in the field and the rows' own drags held; check.sh passed.

## T71

- Round 1, a9637ad: PASS. omp's bursts of A, B, C and D;0, which posted three ends on the parent, posted none; a 5.2-second command posted, a 2.1-second one and an end without its C did not; `ended`, the row's error mark, OSC 7501 and OSC 9 kept their paths; bash 3.2 and zsh held; check.sh passed.

## T72

- Round 1, 26cd2aa with v0.1.8 published on GitHub at d5f97df: FAIL, on one point: the macOS and Windows executables keep dependencies' source paths under the build hosts' home folders, panic locations rustc keeps. That criterion was the round's prompt's, not the project's; every release since v0.1.0 carries them, v0.1.7 1249 in its executable; the owner was told, a decision apart.
- Round 2, 26cd2aa: PASS. The release is published and latest at d5f97df, which CI run 39730393420 tested; the three files match the notes' SHA-256 downloaded anonymously; the disk image stapled and notarized, the zip's executable a GUI program with its icons and no C runtime DLL, the tarball's modes and glibc 2.35; each binary holds T67 to T71's work for its platform and LF scripts; the notes match the work in English and Chinese and credit the four PR authors. Then the issues and PRs got their thanks and closed, #2 without a word.

## T73

- Round 1, 480dbf9: PASS. check.sh and the Windows clippy passed; 84 buffer and fragmentation cases, held marks and links, overflowing bodies and WouldBlock, Interrupted, EIO and EBADF after bytes kept the tap's output whole, and the parent returned nothing with bytes waiting; a simulation over piper 0.2.5's real pipe drained 2,000,000 bytes through 1,540 turns, and without the wake stopped at 66,660. Native IOCP and the reporter's own stall went unexercised.

## T74

- Round 1, 46047a3: FAIL. Git's `sh.exe` took the integration and reported plain folders; but `pwd -W` gives a name's controls and `"*:<>?|` back as ASCII, which Windows holds at U+F000 and up, so `bell\adir` and `re:po` named other paths, and the command substitution dropped a name's last newline, naming a sibling. Fixed in 92f6199: past the drive the script maps those characters forward again, and a mark after `pwd -W` keeps the name's own newline.
- Round 2, 92f6199: PASS. 57 cases on bash 3.2 and 5.2, all 31 controls and the reserved characters, drive roots, UNC, MSYS's `/tmp` and `/`, DEL, a backslash, non-ASCII names, a failing `pwd -W` and `set -u`, held, and removing either fix failed them again; 114 receiving paths matched the names Windows holds; check.sh, the Linux tests and the Windows clippy passed. Real UNC shares, symlinks and junctions on Windows went unexercised.

## T75

- Round 1, 3988967: FAIL. One C and D count for every program stuck when a child died between its C and D, and a nested integrated zsh kept it too, so later shell prompts stopped dropping working records; zsh, fish, PowerShell and a Linux bash 5.2 capture showed it. Fixed in ef8e552: a shell's folder report closed every open command.
- Round 2, ef8e552: FAIL. A running program's own OSC 7 cleared its records and the count, and a shell that reports no folder, WSL where `wslpath` fails or a prompt of OSC 133 alone, still kept a killed child's work. Fixed in ffcf61e: each A says whose it is, an `aid` from ELYTerminal's integrations or the foreground process group the tap reads, each report keeping the group that held the terminal; `libc::tcgetpgrp` in place of rustix's wrapper, which asserted on 0.
- Round 3, ffcf61e: FAIL. fish's own prompt is unnamed and a fish builtin or a background job reports from fish's group, so group equality kept their records at fish's prompt; and any program's `aid` counted as the shell's. Fixed in 89b4873: only `aid=elyterminal` names a prompt, and an unnamed prompt held by the session's own group is the shell's unless that shell names its own, as PowerShell does while it runs programs in its group. All eleven probes of the three rounds pass against 89b4873, the scanner's with its named prompt sent as ELYTerminal's, as do fresh Linux bash 5.2 captures and, on the owner's PC, PowerShell's and Git Bash's prompts named through ConPTY; check.sh, the Linux tests, the Windows tests and the Windows clippy pass. No fourth review: the rules cap reviews at three.

## T76

- Round 1, f088b8b: FAIL. A delete across two wide characters joined one's left half to the other's right half, so an erased `单` stood whole, and the halves the batch mend cleared took a neighbor's ground, not the edit's; a mend after the batch could know neither. Fixed in c2bc2cd: the clearing moved into alacritty's handlers, in the org's fork, pinned by rev, as xterm's `DamagedCells` clears, and DCH counts only the columns from the cursor on.
- Round 2, c2bc2cd: FAIL. Text typed in insert mode shifts the row through `Term::input`'s own swap, outside the edge rule: an `X` inserted at `单`'s right half left `单` drawn over it, and one at the line's start left a lone left half in the last column. Fixed in 4b94326: insert mode clears at the cursor and at the first cell it pushes off, a wide character that wraps wraps before the shift, and a character written over one half clears the other in the current ground, as xterm's `WriteText` does.
- Round 3, 4b94326: PASS. The first two rounds' nine assertions held; 1805 edits and 910 writes and inserts matched xterm.js in text and drawn ground; 5832 more cases over widths 2 to 11 left no unpaired wide cell, against 962 on the fork's first commit; real XTerm 398 under Xvfb cleared an overwritten half to the current ground and dropped its inverse, underline and strike; check.sh, the fork's 136 unit, 45 ref and doc tests, Linux's 287 and a Windows check passed. A combining mark after a wide character refused at the last column with autowrap off, alacritty's own before the fork, lies outside T76.

## T77

- Round 1, 831c150: PASS. Under WeType Pinyin 28 byte checks over Off, Left, Right and Both, legacy keys and kitty's flags 1 to 31, held each side, Shift, repeats and releases; a plain `n` and `N` under report-all-keys reached the input method as the first parent sent them, where PR #18 as written sent `CSI 110;;110u`; natural text editing, Ctrl and Command kept their routes; a stale Option event at draw changes nothing on macOS; check.sh passed.

## T78

- Round 1, 68d9579: PASS. US-International's dead quotes match the report, and ELYTerminal and a console PowerShell drew the same six steps; the debug line sits inside the `prefer_character_input` branch alone, which still returns, and without it `input.rs` is its parent's; a probe built from `key_down`, `records` and gpui's `process_key` held over 36 press, repeat and held cases, 384 routes and 13 conversions, and failed when the level or the branch changed; check.sh passed.

## T79

- Round 1, cd949e5: PASS. Backspace and Left went as `7f` and `ESC [ D` under kitty's flags 1 and 3 and as win32-input-mode records, and omp 18.12.1's own parser, its N-API wrapper set aside, read 1,076 edit-key cases right; omp's fix `3bc77d6` lies after v18.12.1 and in v18.12.2, and a probe of its stdin method before and after showed a relayed DA1 reaching the prompt before and not after; the kitty flags log named eight changes, the alternate screen and RIS among them, and nothing else; check.sh and a Windows GNU clippy passed.

## T80

- Round 1, 7b926dc: FAIL. The decoder read each set through encoding_rs's tables, which differ from libX11's: Big5-HKSCS in 3,043 characters, 1,159 private-use ones among them, which stopped the app at its commit, U+F83E first; Big5's `☉` and `♁`, and 255 more under a Big5 locale; GB2312's `・` and `―`, read through GBK, which maps cells GB2312 leaves empty. Fixed in 715aa4e: each set reads through libX11's own table, which `tables/generate.sh` writes from libX11 1.8.12's lcUniConv, and the Linux tarball carries libX11's notice.
- Round 2, 715aa4e: PASS. The generator rewrote all eight tables byte for byte; 1,007,622 oracle records of round 1, 4,006 mixed strings in five locales through libX11 1.8.12 and 1,116,038 records through fcitx5's own xcb-imdkit 1.0.9, every scalar from U+0020 to U+10FFFF, decoded as their encoders' own reverse conversions; a real IBus engine's `・〜−☉♁` and U+F83E reached the shell whole; 50,000 random byte strings decoded or failed without a panic; check.sh, the Linux IME tests and CI on all three systems passed.

## T81

- Round 1, 47fc31d: FAIL. A line count turned into pixels and back lost a line at a cell a fraction of a pixel high, 3 making 2 at 125%; gpui turns a Shift notch sideways on Windows, so Shift with the wheel scrolled nothing; and gpui gives Linux's notch as 3 lines, which ELYTerminal multiplied by 3 again. Fixed in 8ed6611: `term::wheel` counts in lines, keeps its 3 for macOS alone, and reads a sideways Shift notch as the wheel, on Windows in the lines Windows sets.
- Round 2, 8ed6611: FAIL. The docs promised the history under Shift where the alternate screen sends arrows under mode 1007; and the Shift conversion read Windows' settings live while gpui counts in a copy it reads once, so a silent change gave 21 lines for 7, and 0 lines with 0 characters left NaN in the wheel for good. Fixed in 74ded0c: the docs say Shift's wheel does what it does where nothing tracks the mouse; `wheel::keep` reads the pair where gpui reads its own, as the main window opens; a NaN notch is logged and moves nothing.
- Round 3, 74ded0c: FAIL. A notch sent in parts, as a fine wheel sends it, summed in f32 a hair short of its lines, so ten parts of a Shift notch at 3 lines and 7 characters scrolled 2. Fixed in c32fe52: a sum within a thousandth of a whole line counts as that line; the review's probe then gave every split of a notch, 12,800 in all, its whole count, plain and under Shift, where the function before fell short in 1,516, every line at one line a notch in parts of 1; check.sh, Linux's 288 and 7 and Windows' 268 and clippy passed. Three reviews spent, the cap.

## T82

- Round 1, 614834d: FAIL. Every path issue #27 names passed under KWin 6.3.6, a press, a drag, the settings window and moods while she was closed among them; but a window kept above an active terminal, over her spot, lay under her and lost the presses there. Answered in 44612f4: KWin's popup layer stands above its kept-above windows for every popup, menus and tooltips among them, Wayland tells no client what covers its window, and gpui-pre-linux 0.3.8 places a child only as a popup and makes no subsurface, so AGENTS.md, the PRD and the acceptance table name the limit.
- Round 2, 44612f4: PASS. KWin's layers and gpui's three Wayland surfaces bore the reasons out, and a registry probe found `org_kde_plasma_shell` shown to an ordinary client but `org_kde_plasma_window_management`, which says where the main window moved, withheld; the kept-above run and the lifecycle run repeated; nothing that runs changed; check.sh passed.

## T83

- Round 1, 88d1780: PASS. Under all four settings of the two switches, three rows took a right press, the menu's keys, a drag, a rename with a drag inside its field, a press and a ×: menus and moves came as the switches said, and the rest as before; two windows followed a change in Settings at once, and a restart kept it; a settings file from 0.1.8 started with both on, and an unknown field stopped the start naming the file and the field; the rows stood 51 points apart in `List` and `Reorder` alike, and sixteen rows scrolled end to end; check.sh passed.

## T84

- Round 1, d5a6ea1: PASS. The reporter's tests passed at `996bd22` and failed on the old footer, the label 42 pixels tall at 240; a harness on macOS's own text system laid out every width from 143 to 960 pixels over four branches, a 200-character one and a multibyte one among them, amend off and on, 6,544 layouts, each label on one line, and 336 commits by press and Cmd-Enter landed; the app at a 160-pixel panel committed and amended a fixture by press and by Cmd-Enter; a fresh Cargo cache fetched the pin, the lock changing only Ely's source; Ely's main held the same footer byte for byte, and `ely-git`'s 40 tests passed alone on Linux; check.sh passed.

## T85

- Round 1, 6c26cd5: FAIL. The 150 keys, both catalogs and every page, menu, dialog, find and the Git panel switched at once in both directions, and Ely's PathInput and FontPicker spoke the language; but a tooltip open through the switch kept the old words, since gpui keeps an open tooltip's view, Ely's own tooltips too.
- Round 2, 525eee4: FAIL. An open tooltip withdrew; but one whose words were taken before the switch and which gpui opened after it, inside its show delay, came up in English beside Chinese, since `Tooltip` read the locale when gpui built it.
- Round 3, 186c0bc: PASS. Both scenarios replayed natively through the real Language control withdrew, re-hover showed 新建会话 and back again New session; Ely's two tooltip tests fail on `d3adf78` and the second on `9565a6c`; six boundary probes, serial and parallel, held the count per thread and per locale change.

## T86

- Round 1, 1277cf6: FAIL. WSL's `--cd ~` at Home alone, Folder's checks at load, save and start, the settings window and a live language switch all held, and the store probe refused relative, empty, file, missing and NUL paths; but Browse wrote its pick by Ely `PathInput`'s lossy name, so on Linux a folder named with a byte not in UTF-8 saved its sibling of the lossy name.
- Round 2, 154e982: PASS. Ely's PathInput test fails on the old conversion and passes on `92f69a3`; an app probe pressed both fields' real Browse buttons through gpui's test dialog: a raw pick was refused in its own row and the next good pick cleared it, and picks with Chinese, spaces, `%`, quotes and a newline kept their folder and program.


## T87

- Round 1, 19035b6: FAIL. The merge kept main's decoder, its tests, CI and Release; the four x86_64 formats from hosted run 40143259149 carried the binary, desktop entry, icon and seven licenses, `libX11-lcUniConv` among them, held glibc 2.35 and refused a binary asking 2.36, and the workflow held a read-only token, no secrets and pinned actions; but the tarball lacked the man page the other three carry, which `linux-tar.sh` left in its stage.
- Round 2, 99e8020: PASS. The x86_64 tarball from hosted run 40145343964 and the ARM64 one carry `elyterminal.1.gz`, byte for byte the source's; the README's four install lines, run as a user on Ubuntu 22.04, let man-db find and render it, and its uninstall removed all four; every other file of the four formats stayed as it was.

## T88

- Round 1, 0d4f9bb: PASS. On Ubuntu 26.04.1's GNOME 50.1 under Wayland, 0.1.8 maximized from 800 by 560 sent a Right resize and a pointer leave for each tab press, and no release reached it; true full screen let the releases through. The fix took every tab press normal, maximized and full screen on GNOME 50.1 and 48.7 under Wayland and on GNOME 48.7 under X11, where `_GTK_EDGE_CONSTRAINTS` marked the tiled edges; all four margins and corners still grew a floating window, a left-tiled window's free edge grew it, and restore worked. An oracle over all 16 tiling masks, 1,936 cases a branch, failed both branches with the right edge's guard removed; Ely main's whole gate passed at `c360498`; the default filter names Ely's lines with `RUST_LOG` unset.

## T89

- Round 1, 7a9ad8e: FAIL. #25 and #28 were thanked and closed, every issue and pull request §36 names was answered and closed or merged, and every PRD bullet mapped to a row; but row 13 certified "no update checks" against T56's check, which `PRD.md` line 11 still forbade, row 181 left out the seven tasks ticked without a review, and rows 12 and 23 held old inventories. Fixed in 78ebefc.
- Round 2, 78ebefc: FAIL. The first round's findings held fixed, and a scan of this goal's 61 commits and the Ely and alacritty commits beside them found no credential; but row 13's new evidence promised the update notification "once a day at most", which nothing across runs keeps, as AGENTS.md's T56 decision said too. Fixed in ff3ae30.
- Round 3, ff3ae30: FAIL. The second round's finding held fixed; but row 156 certified that the update notification opens its page, while `updates::pressed` read the page from the last check's state, which a Check now running or failed offline replaces, so the press opened nothing. T56 asked that the press open that page, so T90 fixes the cause in the code. The cap: no fourth round.

## T90

- Round 1, fc04494: FAIL. The press opened a page during a Check now and after a failed one, where it had opened nothing; but the fix kept only the last newer release's page, so after 0.1.9's notification a Check now that found 0.1.10, or on Linux a later notification posted beside it, made the 0.1.9 notification open 0.1.10's page. Fixed in 6984ace: each notification carries its release's tag, `elyterminal-update-VERSION`, and the app keeps each release's page by that tag.
- Round 2, 6984ace: PASS. Round 1's three failures replay on the parent and pass on the fix; every retained notification opens its own release's page across four daily checks, repeated tags, reverse presses and a withdrawn release; 1,500 deterministic operations checked 30,468 presses against their notifications' versions; mutants that clear older pages or read the check's state fail; gpui 0.3.8 carries the tag back on macOS, Windows and Linux.

## T91

- Round 1, ba84990: PASS. Of 168 comment entries added or moved since v0.1.8, the 11 over ten words stand verbatim at v0.1.8; this work's comments on Ely's pin branch and main and in the alacritty fork hold ten words at most, one line each; stripping comments leaves every changed file byte for byte as before, in all three repositories; the pins differ from their parents in the two revisions alone, and each rewrite still says what its code does. It noted a 12-word module docstring in `scripts/linux-gui-test.py`, from PR #16, which is no comment token; the pass commit shortens it to ten words.

## T92

- Round 1, 7ec247d: FAIL. T75's twelve probes passed on HEAD, a fresh bash capture and the scanner's prompt named as today's; T81's and T90's replays passed; but a nested fish kept its own builtin's working, blocked and idle records at its prompts, one nested zsh made every later unnamed prompt of the outer fish a program's, and under one screen at a time a notch split into 2, 10 or 120 messages scrolled 106, 530 or 6,360 lines on 53 rows. Fixed in b20fb33: a named prompt names the session's shell only from the session's own group; a paged notch's parts scroll their share of a screen; a shell the integration does not reach, nested or over ssh, recorded as a limit, since its unnamed prompts carry what omp's marks carry and no launcher hands a child the integration.
- Round 2, b20fb33: PASS. On the parent the split notch scrolled 106, 530 and 6,360 lines, on the fix 53, with 21,504 random paged splits, 40,804 whole notches and 12,800 directed splits passing; the old `named` line restored brings back the nested zsh failure; T75's thirteen probes pass and a fresh bash 5.2 capture replays clean; the nested fish limit stands as recorded and row 60 stays Partial; 289 tests, comments within ten words, every issue and pull request closed and thanked.
- After round 2: T75, T81 and T89, ticked on this pass, are unticked again. A pass of T92 is no review of theirs, and the cap allows them no fourth; whether T92's pass accepts them is the owner's call.

## T93

- Round 1, 5913f98: FAIL. The nested fish's builtin records drop and every probe of T75's and T92's rounds passes, but a subshell `( … )` running omp, a forked zsh with its parent's arguments in a group of its own, and a fish with job control off read as a shell, so omp's first mark dropped its working record; and `bash +H`, whose `+H` is an option, read as holding an operand, so its prompts kept its builtin's records. CI's Linux job failed as well: Linux names an exec's executable before its arguments, which read empty meanwhile, and none read passed as options alone. Fixed in 726cd35: a prompt whose line takes clicks is a shell's from any group, as fish from 4.1 marks every prompt; a plain one is a shell's when its group holds one process, zsh, bash or fish started with options alone, `+` among them for zsh and bash, its arguments read; the test waits for each child to say it runs. The review prompt's claim that fish runs git after its A was wrong: the second process in a fresh fish's group was its first start's completion builder.
- Round 2, 726cd35: FAIL. Round 1's three failures pass, with every earlier probe, 56 in all, and 350 runs of the waiting test in Linux; but on a Linux `/proc` mounted `hidepid=1`, another user's `/proc/PID/stat` refused the count, which ended at the first refusal, so a nested bash alone in its group read as a program's and kept its builtin's records, where round 1's commit had dropped them. Fixed in a0849c7: the count reads each listed process's group through `getpgid`, which `hidepid` leaves open; the review's own probe, built from the tree, reads that bash alone under `/proc` plain, `hidepid=1` and `hidepid=2`, and a subshell running a job as two.
- Round 3, a0849c7: FAIL. Round 2's finding is fixed at its cause: under `hidepid=1` the nested bash reads alone and drops its builtin's records, a subshell running a job keeps them, and every earlier probe passes, 56 in all; but a fish script's `read`, run as a file or through `-c`, marks its line `click_events=1` as fish's own prompt does, and the click alone made it a shell's, so the installer's blocked record went while it waited. Fixed in 6dc0409: a click counts only where the group's leader is fish started with options alone, since fish forks no copy of itself, its backgrounded blocks and functions running in place, so the script's operand makes the line the program's; zsh's and bash's clicks need the group alone, as their plain marks do, and a click without a group tells nothing. The round's probe passes with the rest of the set, 58 in all, three runs in a row; the hidepid and subshell runs in Linux hold; `fish -C` returns to the recorded limit of an option's argument, and a nested fish with job control off takes a script's `read` for its own, recorded with a probe. The cap allows no fourth review.
- On 2026-10-24 the owner accepted T75, T81, T89 and T93 at the cap, asked with the evidence; each is ticked.

## T94

- Round 1, 8c76ba3 with v0.1.9 published on GitHub: PASS. The release is published and latest at 8c76ba3, which CI run 40167432357 tested on its second Windows attempt, the first failing `overlapping_saves_leave_one_whole_file` on `Access is denied`, a race left to its own task; the six files downloaded anonymously match the notes' SHA-256; the disk image stapled and notarized, arm64, 0.1.9, macOS 11; the zip's executable a GUI program with its seven icons, PerMonitorV2, no C runtime DLL and its five scripts with LF; the tarball's modes, man page and licenses, glibc 2.35; the DEB's and RPM's binaries the tarball's, stripped, with 0.1.9 and their dependencies; the AppImage runs the same build, one build ID across the four Linux formats; each binary holds T73 to T93's work for its platform; the notes match `git log v0.1.8..v0.1.9` in English and Chinese, eleven Chinese labels the catalogs', and credit the four pull requests' authors.

## T96

- Round 1, 3d09fbd: PASS. Create, write or sync, and rename each name their step and files, the system's error kept beneath; the order of the steps, the closed temp file before the rename and the temp's removal stand as before; the new test fails on d9ff8d2's message and on a create that fails, and ran in CI run 40169495331 on Windows, 276 tests, and Linux, 297; the CI log holds two first saves refused under one message, and Rust 1.95's rename falls back between MoveFileExW and FileRenameInfoEx, so even a rename's refusal would need more evidence to name its cause; the cause stays open.

## T97

- Round 1, 598f60a: FAIL. `-c'…'`, `--command=` and a script after `--` read as interactive; bash's `-o -O +o +O` may stand without a value; bundles such as `zsh -lo vi` were missed. Fixed in d5bfb84: the options are parsed, not matched whole.
- Round 2, d5bfb84: FAIL. bash takes each bundled `o`/`O` value from the next argument; zsh also stops at `+`, `+-`, `-b` and a trailing `-`; fish names a long option by a unique prefix. Fixed in ba2a36b.
- Round 3, ba2a36b: FAIL. bash's single-dash long options, `-rcfile FILE` and `-init-file FILE`, read as a bundle. Fixed in 1d1956a; the cap allows no fourth review.
- On 2026-10-25 the owner accepted T97 at the cap; it is ticked.

## T98

- Round 1, 8a2e7a9: FAIL. pwsh abbreviations; nu bundles and list options. Fixed in 1d1956a.
- Round 2, 1d1956a: FAIL. PowerShell's own switch order and least prefixes, `--` and `/`; nu's `=` lists. Fixed in c36f986.
- Round 3, c36f986: FAIL. `-EncodedArguments`' value; Unicode dashes and blanks. Fixed in the next commit; the cap allows no fourth review.

## T99

- Round 1, 8994388: FAIL. A reused parent id hung an orphan under a program. Fixed in 1d1956a.
- Round 2, 1d1956a: FAIL. An unreadable start dropped a program from the tree. Fixed in c36f986.
- Round 3, c36f986: FAIL. An unreadable start kept a reused-id edge. Fixed in the next commit: any unknown start leaves the holder unknown; the cap allows no fourth review.

## T100

- Round 1, ba2a36b: FAIL. A click through a relay read as a shell's; sudo's own pty foreground was readable. Fixed in 1d1956a.
- Round 2, 1d1956a: FAIL. macOS `proc_pidinfo` refused a root relay. Fixed in c36f986 through `ps`.
- Round 3, c36f986: PASS.
- On 2026-10-25 the owner allowed T98 and T99 a fourth review.
- On 2026-10-25 round 4 of T98 and T99 failed: a doubled Unicode dash; a console whose start hides; reviews.md past 500 lines. Fixed in the next commit, which splits T01 to T49 into reviews-t01-t49.md.
- Round 5 of T98 and T99, bfc8631: PASS.
- On 2026-10-25 the owner kept ssh and container shells as a recorded limit: no remote integration.
- On 2026-10-25 the owner accepted the other limits row 60 records as limits: `bash -s`, a subshell marking by builtins, sudo under `hidepid=invisible`.

## T102

- Round 1, 09ca1c6: FAIL. Hebrew split across two colors drew its sections swapped, `abc אב גד xyz` in red and blue; two comments ran past ten words. Fixed in 5261932: main and the merge both placed a glyph where shaping put it, so the shaper's reordering leaked into a grid of logical order; a glyph took its character's column.
- Round 2, 5261932: FAIL. Tamil கொ shapes as vowel, consonant, vowel, indices [3, 0, 3], and its vowel's two parts met in one place. Fixed in 1244310: a glyph takes its grapheme cluster's column, its offset from the cluster's leftmost glyph kept, and rows shape without forced advances; the owner chose grid columns over main's drift, so a syllable wider than its cells overlaps the next.
- Round 3, 1244310: FAIL. A flag split by color, `\e[31m🇺\e[34m🇸`, put both halves in one cell; `👨🏾X` joined the modifier to the space holding the man's second cell, which shaped at the emoji font's width over X. Fixed in 4838df5: clusters stop at color sections, each shaped alone, and after a space, which in a row's text holds no mark. Shots match main for both; check.sh 303 tests, Linux 309 and 7 IME, Windows 288.
- On 2026-10-28 the owner allowed T102 a fourth review.
- Round 4, 4838df5: PASS. The reviewer drew 36 inputs and 52 comparisons on macOS and Linux, all as expected; each new test caught its mutation; check.sh passed with 303 tests.

## T103

- Round 1, 24cc492: FAIL. `conpty()?` sat inside `log::info!`, so `RUST_LOG=warn` skipped the check and a session started on the system's conhost, its mouse modes lost; the zip script's first comment ran twelve words. Fixed in 5ef7b5e: the check runs before the log line.
- Round 2, 5ef7b5e: PASS. The reviewer ran 15 start cases on the PC: the bundled pair starts, a missing pair or one from PATH fails, under five log levels alike; the Windows suite passed.

## T104

- Round 1, 0639ab0: FAIL. The READMEs named conpty.dll, OpenConsole.exe and LICENSE-conpty in a zip whose v0.1.9 holds elyterminal.exe alone; both features pages pointed "below" to a link press now on the keys page. Fixed in 7ed2327.
- Round 2, 7ed2327: PASS. The install line matches v0.1.9's zip, both features pages link their keys page, and 26 relative links resolve.

## T105

- Round 1, 15d9754: FAIL. A closed menu opened again when the right column or the switch brought it back: Ely's ContextMenu keeps the request it used in keyed state, which goes when it leaves the frame, and Files sent the same request each render. Fixed in ac3a967: a render takes the press's point once.
- Round 2, ac3a967: PASS. In the app five right presses opened five menus, and a hidden column or a turned switch reopened none; New session here ran.

## T106

- Round 1, 979c02a: FAIL. AGENTS.md said the reporter was asked for logs while #29 held no comment yet. The request was posted, issuecomment-6484160625.
- Round 2, 979c02a with the comment: PASS.

## T107

- Round 1, f79691b: PASS. The reviewer's Windows probe broke the italic glyphs on the old dependency and matched every glyph id on the fork; 120 rasterizations, 20 color glyphs, passed; 24,000 layouts of varied text and sizes kept the cached font ids stable.

## T108

- Round 1, bdc618f: FAIL. A place alone named no display: gpui's macOS displays all stand at 0, 0, and Windows' logical bounds overlap at mixed DPI; a place whose middle lay on screen could leave the title bar above it. Fixed in 88533ed: the display's uuid and an offset, and the window moved inside the visible area.
- Round 2, 88533ed: FAIL. Two X screens share gpui's nil uuid, so a place on screen 1 reopened on screen 0. Fixed in 5606352: a uuid naming more than one display centers, logged.
- Round 3, 5606352: FAIL. A monitor gone from one X screen shrinks the screen, and the clamp pulled the window to its edge. Fixed in the next commit: a kept place whose middle left its display centers, logged.
- On 2026-10-28 the owner's standing word, every recommended option, allowed T108 a fourth review.
- Round 4, aae8597: FAIL. The middle was checked after fitting: a 1200 by 1269 window kept at 300, -600, its middle on the screen, centered once shrunk to the visible height. Fixed in the next commit: the kept bounds' middle decides, then the fit and the move. The owner's standing word allowed a fifth review.
- Round 5, 6a57856: FAIL. On one X screen, a place kept on a left monitor that went, the right one moved to the origin, still had its middle on the shrunk screen and opened there, not centered: gpui's X11 display is the screen and knows no monitor in it. Recorded as a limit in AGENTS.md; the window opens whole on a monitor that stayed. Under the owner's standing word, every recommended option, T108 closes after five reviews with that limit, as T97 closed at its cap.

## T109

- Round 1, ee5bf13: FAIL. A custom color changed the terminal, not the chrome; New theme on a dark system with a dark theme set the light one; deleting the dark theme let the light one show; a picture overwritten, cleared and named again showed its old pixels. Fixed in f8bfe77.
- Round 2, f8bfe77: FAIL. Cmd-Z after a theme switch wrote one theme's name onto another; a picture renamed while the terminal was minimized kept its old pixels. Fixed in 8821701: a name field for each theme; the app drops gpui's cached picture where the setting changes.
- Round 3, 8821701: PASS. Both earlier rounds' inputs replayed in the app; one PTY test failed once in the reviewer's first check.sh and passed in the full rerun, 317 tests. Windows went unchecked on 8821701: the owner's PC reset every SSH connection; f8bfe77 passed clippy and the tests there, and CI's Windows job checks the push.

## T111

- Round 1, ef2adb2: FAIL. `ESC _ ... ESC CR \` lost its CR: the tap held the string's ESC as one from ground and moved the CR into the APC; vte's no_std tests did not build; a fork comment ran eleven words. Fixed in 6a5df6a.
- Round 2, 6a5df6a: FAIL. Two new comments in src/term/prompts.rs ran past ten words. Fixed in ff8d0f7.
- Round 3, ff8d0f7: PASS. Before it, check.sh failed `only_an_interactive_shell_leading_its_group_draws_its_prompt` once under the full suite, as it had in T109's review; alone it passed five times. Its log named the cause, `group's arguments hide, taken as options: Input/output error`: macOS refused the arguments of a process mid-exec, and the holder took that as another user's hidden arguments. T123 fixes it.

## T123

- Round 1, d64e6cb: FAIL. A read that straddled the exec mixed two programs: `sh -c 'echo ready; exec /bin/cat'` read the executable `/bin/bash` and then the arguments `["/bin/cat"]`, so `cat` read as a shell, 52 in 1,000 for the reviewer. One read after `ready` never caught it here, even under load; 50 reads across the exec in each of 200 runs gave 25, 32 and 28 false shells. Fixed in 5ecc9bc: the executable is read again after the arguments, and a change means no shell.
- Round 2, 5ecc9bc: FAIL. The reviewer's probe gave no false shells in 1,000 runs, and its log showed 44 caught by the second read; one comment ran eleven words. Fixed in 0ef0223.
- Round 3, 0ef0223: PASS. Linux passed 327 tests on 5ecc9bc; Windows, which reads no groups, went unchecked: the owner's PC reset every SSH connection.

## T112

- Round 1, first run on 66e91df: no verdict. The reviewer's provider stopped its race probe as a cybersecurity request; before that it counted two test comments at twelve words. The rerun asked for checks by reading the code and ordinary files.
- Round 1, 90b1a91: FAIL. A temporary file's folder swapped for a link after the check let the read and delete reach a file outside it; a FIFO under /dev/shm would block Linux's shm_open; a local image past the quota answered ENOMEM where every other local failure answers EBADF; the two comments, left unstaged by an amend. Fixed in e5a5a21: the file opens through its held folder (Windows: the handle's own checked path) and deletes through it; shm_open takes O_NONBLOCK and wants a regular file on Linux; the quota failure maps to EBADF. Each has a test red before.
- Round 2, e5a5a21: FAIL. The held folder opened for reading, so a file in a folder one may only search (0111) answered EBADF. Fixed in 581f534: O_SEARCH on macOS, O_PATH on Linux.
- Round 3, 581f534: FAIL. rustfmt had split the Windows shared memory read into a statement and `[range].to_vec()`, which did not compile; the Windows PC, reachable again, found it first. Fixed in 2a27578; 51d9182 counts the ConPTY test's 17 bytes. A fourth round ran under the owner's standing word.
- Round 4, 51d9182: PASS. Windows: clippy clean and 321 tests, shared memory, temporary files and the ConPTY APC among them; macOS 340; Linux 347 on 581f534.

## T110

- macOS and Linux were proven in ceb4bea; Windows waited on the PC's SSH.
- Round 1, 488e9a5: FAIL. The Windows Claude Code record ran past the evidence file, whose line filter had dropped the `❯ a` row, drawn with a no-break space, and the version banner. The encoding check passed: the probe picks the listener's own bytes for both modes. Fixed in 842b6b7: whole PROBE blocks in the evidence, and both records name the proof's reach, the encoder and OpenConsole, not gpui's dispatch of a real key.
- Round 2, 842b6b7: PASS.
- A first try at a GUI run on the PC grabbed the owner's own ELYTerminal window by name; it was caught at the first capture, before any key, the settings restored and only the test's process stopped. The proof ran headless after.

## T113

- Round 1, f58b66c: FAIL. A placement whose cursor move scrolled the screen stayed put; a region from the top (`CSI 1;4 r`) cut nothing; a reflow left a placement off its text. Fixed in bc5acb1: a whole-screen scroll moves every placement; a region cuts at its top; the fork's resize hands a map of cells.
- Round 2, bc5acb1: FAIL. A wide character's reflow put a placement a cell off; a placement ED 3 cut was dropped at the next resize. Fixed in add5212, with the fork's map counting text cells and logical lines from the oldest.
- Round 3, add5212: FAIL. A shorter window cutting bottom lines and a full history dropping a wrapped line's front misled the line count. Fixed in 9ff07fc: the fork marks each anchor's cell with a hyperlink of its own through the resize.
- Round 4, 9ff07fc: the run disconnected after one finding: a program's look-alike link made the next resize panic. Fixed in 00e1508: marks match by identity.
- Round 5, 00e1508: two runs disconnected; the reviewer restarted fresh. FAIL. gpui's clip squeezed an image cut at the edge; a cut in rows let a smaller cell uncover pixels. Fixed in a1596c2: cuts go into the source, as Ghostty clips, and a clipped image paints whole under a mask.
- Round 6, a1596c2: FAIL. A bottom cut of a 1x1 image over two rows rounded to nothing. Fixed in 0aff760: the source keeps fractions of a pixel.
- Round 7, 0aff760: PASS, with 216 clipping cases and a native draw check. Rounds 4 to 7 ran under the owner's standing word. macOS 354, Linux 361, Windows 335.

## T114

- Round 1, ea753a4: FAIL. A relative child of a virtual parent survived `a=d`; one at a negative column escaped `d=p`. Both came from resolving a relative placement's chain for deletes by place. Fixed in 5f2d7cb by Ghostty's rule: deletes by place pick grid placements alone, and a relative placement goes with its parent or by id, number, range or z. The test fails on the old code.
- Round 2, 5f2d7cb: PASS, with 64 delete cases and 33 decode and size checks. macOS 361, Linux 368, Windows 342.

## T115

- Round 1, c8eb050: FAIL. A frame whose quota reserve evicted a placed image and still failed left that image's placement, which an image later given its id would show. Fixed in c562d60: the evicted images' placements go on success and failure alike. The test fails on the old code.
- Round 2, c562d60: PASS. macOS 370; Linux 376 and Windows 350 on c8eb050, the fix's one test on macOS.

## T116

- Round 1, f24e6f1 with vte 7c7404d: FAIL. AGENTS.md named SSE4.1 where base64-simd 0.8 dispatches SSSE3 or AVX2 on x86. Fixed in a4d1d92. The bulk APC path and the decoder passed as they stood.
- Round 2, a4d1d92: PASS. macOS 371, Linux 378, Windows 352 on f24e6f1; the fix touched words alone.

## T117

- Round 1, 76ee8eb: PASS. macOS 372, Linux 379, Windows 353; on macOS ten image sessions held 166 MB of GPU memory before and 20 to 23 MB after.

## T118

- Round 1, cf8d705: FAIL, eight findings. A paste password read another place and never expired; aliases copied data past the limit; an overlong packet without its type was passed by; the tap cleaned controls out of bad base64; a failed system write answered DONE; a Unix file name not in UTF-8 was lost; X11 sent large data in one request, which ended its server thread. Fixed in fad0356 and the Linux fork's c2b8743, each with a test that fails without it.
- Round 2, fad0356: FAIL, seven findings. Alias type names went uncounted; an old image passed the read-back; Wayland answered DONE for a skipped write; a missing Wayland primary selection was not ENOSYS; a held Windows clipboard read as empty; X11 INCR transfers stalled each other and kept a gone requestor's data. Fixed in e6ecd51 and the fork's 128f5a9.
- Round 3, e6ecd51: FAIL, four findings. Empty types grew a write without limit; one read copied its data per repeated type; Wayland kept a claim another client had taken; X11's stall deadline waited for an event. Fixed in 6230912 and the fork's 15cc240.
- Round 4, 6230912: FAIL. X11's serve thread could sleep on a socket another thread had drained. Fixed in 359c0a4 and the fork's 829b808.
- Round 5, 359c0a4: PASS. Rounds 4 and 5 ran under the owner's standing word. macOS 385, Linux 392, Windows 366 on 6230912; round 4 touched the Linux fork alone, tested on Linux. X11 and Wayland end to end in Docker on 359c0a4.

## T119

- Round 1, e588ee5 with the fork's f5671c5: FAIL. A search left the whole history unpacked with nothing to pack it again; an unpacked row was sized by its text's bytes, not its cells. Fixed in a9daa00 and the fork's 0baf57d, the sizing with a test that fails without it.
- Round 2, a9daa00: FAIL. A resize's reflow, and select all or a prompt jump that returned early, still left history unpacked. Fixed in 44a4c0d by design rather than per reader: each terminal packs every two seconds output and scrolling rest, and the fork's compact returns the rows it changed.
- Round 3, 44a4c0d: PASS. macOS 385, Linux 392, Windows 366; a counting allocator measured 85 to 95% less history memory.

## T120

- Round 1, 00f42ce with vte 5142c08 and alacritty 10d36c1: PASS. macOS 385, Linux 392, Windows 366; a 400-stream fuzz test holds the fast path to the per-character one.

## T121

- Round 1, 5cb2468: FAIL. The raw captures blend a horizontal line's rows, which the record called sharp. foot, run in the same nested niri, blends alike, so the shift is the composition's, 602 logical rows at 1.25 making 752.5 physical ones; 32d348e records it so. No code changed.
- Round 2, 32d348e: PASS.

## T122

- Round 1, 99a821b: FAIL. §13 said T102 to T121 each passed a round, but T108 closed on its fifth failing round, so its row was Partial, not Pass; T103 had no rows, OpenConsole nor its one-line composition; a test path selected nothing; a blank line broke the Foundation table. Fixed in d0dfd8f, and every cited path matched against `cargo test --list`.
- Round 2, d0dfd8f: PASS. `scripts/check.sh` passes with 385 tests.

## T124

- Round 1, 2cad27d with the fork's 63b0ba2: FAIL. A mask kept held keys it left out, so a lost Left Shift release lasted past a mask without Shift; Enter's keys were dropped. Fixed in 50cfcea and the fork's 4f8943a, each with a test that failed first.
- Round 2, 50cfcea: FAIL. A new keymap forgot the keys still held; Meta was read as Alt by a keysym table, where `altwin:meta_win` makes it Mod4. Fixed in df60b5e and the fork's 129a74e: a key's modifiers read from the keymap's own action, held keys carried across a keymap.
- Round 3, c4dab8d: FAIL. That reading used group 0 and level 1, so Multi_key on Shift with Right Alt counted as AltGr and Right Alt in a German group as Alt. Fixed in 32aab68 and the fork's 09711c2.
- Round 4, 32aab68: FAIL. Ctrl with Left Win as a group switch read as Super; a release read its symbol again; held keys were sorted in group 0 before the mask's group came. Each round had found a case a reading of a key's modifiers of its own missed, so the design changed: in 86f2fc2 and the fork's 8fcc867 the client's xkb state takes every key through `update_key`, as a compositor's does.
- Round 5, 86f2fc2: FAIL. Rebuilding the state at each mask replayed held keys in the new group; the keystroke was read after `update_key` had taken a latch; a key's group change left gpui's layout stale. Fixed in a28532f and the fork's 526c5ca: a mask is set over the live state, unseen keys are the mask's alone, the keystroke is read first, and the layout refreshes.
- Round 6, a28532f: FAIL. A Shift held into the window lost its modifier to the other Shift's tap, xkb's count reaching zero while the compositor's unchanged mask said nothing; a key sharing a modifier escaped a mask; Leave left xkb's filters. Fixed in 8e235a5 and the fork's 7a2d154: each mask rebuilds the state over the held keys, each in its own group, keys down before focus included.
- Round 7, 8e235a5: FAIL. The replay used the mask's locks, so a locking press under `shift:both_capslock` replayed as an unlocking one; keys down before focus and across a keymap kept no group. Fixed in 0eabb86 and the fork's be261d2, and a fuzz test, 3,200 random sequences over eight keymaps against a compositor's own xkb state, now holds the design to its contract; it fails on round 5's and round 7's code.
- Round 8, 8928e3c: FAIL. Replays picked another level under `shift:breaks_caps`, and guessed at keys down before focus under `caps:shiftlock` and `shift:both_capslock`. Fixed in a84775d and the fork's 97f9a7c: a mask rebuilds the state only when the depressed modifiers disagree, so with every key seen the state is the compositor's own; a replay sets the whole state its press found; keys down before focus are never replayed, the mask's modifiers held while they are down. The fuzz test grew to thirteen keymaps and fails on round 8's code.
- Round 9, a84775d: FAIL. The hold for keys down before focus took each mask's modifiers, so a letter held into the window held AltGr keys pressed after it, and their release forced a rebuild that lost a latch. Fixed in 99921e2 and the fork's 6ed13f7: the hold is the first mask's, and later masks only shrink it.
- Round 10, 5f2a35a: FAIL. A focus lost cleared the keys but left xkb's filters, so a Shift let go away from the window lingered; an unseen Shift's release rebuilt the state and lost a latch's history. Fixed in the next commit and the fork's 0455e06: only a mask leaving out a seen key's own modifier rebuilds, a focus lost gives the state a fresh start, and a fuzz test of focus lost and found joins the others.
- Round 11, c878403: FAIL. The fresh start on a focus lost reset the group to 0, so a layout changed away from the window could leave gpui's layout cache stale. Fixed in the fork's e46d37a: a focus lost keeps the group and locks, with a test that failed first.
- Round 12, cb48f91: FAIL. A key a compositor binding takes never reaches the client, so the client's xkb filters drifted while every mask still came, and the old mask-only reading was right. The design changed again in the fork's 87b07ae: the window's state reads the masks alone, as gpui's did; a shadow state takes every key; only a key no mask answered settles into the window's state, adding what the shadow adds and dropping only what settling added, so a mask's word stands. A fuzz variant with keys the compositor takes joined, and 84,000 sequences passed off the record.
- Round 13, 3ed7cbc: FAIL. A release no mask answered settled only at the next key, so a click before it read the cached Shift and extended the selection. Fixed in the fork's 857921f: a pending key settles before a pointer event reads the modifiers too.
- Round 14, 25183f0: FAIL. A mask sent before its key, against the protocol's order, which the brief had asked about, applies a group toggle twice in the shadow. Recorded in AGENTS.md as a limit: deferring masks to catch it would put every ordinary path at risk.
- Round 15, 6e5b1d3: FAIL. Under libxkbcommon 1.13, a group latch a compositor-taken key broke kept its filter in the shadow, which later moved the group, and settling took it. Fixed in the fork's 0518b2c: a mask the shadow did not foresee rebuilds it; two latch tests failed first on libxkbcommon 1.13.2 on the Omarchy machine.
- Round 16, ce0a7e9: FAIL. A group latch across a focus change: the shadow's latch state, lost at Leave, cannot be rebuilt from a mask, whose single group number hides a pending latch, and settling moved the group. Fixed in the fork's db0ff3c: settling never moves the group, which only masks set, as gpui's did; the case is a test that failed first on libxkbcommon 1.13.2.
- Round 17, 98934d1 with the fork's db0ff3c: PASS. Seventeen rounds in all, the owner having allowed more than three; the fork's 90 tests pass on libxkbcommon 1.13.2 and 1.4, and Linux's suite 401.

## T125

- Round 1, 1b6e836 and ef30e08 with the Windows fork's a23c98f: PASS. Windows 368 tests, macOS 387; a probe window on the owner's PC drew Acrylic, Mica, Mica Alt and clear as asked.

## T126

- Round 1, a019f5f: FAIL. The history figure read as universal; the drive path's condition, the clipboard's unasked reads, the speed gains and the Wayland fix were missing or loose. Fixed in e49d702.
- Round 2, e49d702: FAIL. The READMEs still promised every read asked; the paste password's read was said to hold the paste's contents. Fixed in def721a.
- Round 3, def721a: FAIL. The drive path's condition named a folder where the code tests any path. Fixed in 8626938.
- Round 4, 8626938: PASS.

## T128

- Round 1, 4b03fcd: FAIL. A wrapper over `$args` lost a failure's `$?`, pipeline input and `-PassThru:$false`. Fixed in c79b631 with Set-Location's generated proxy.
- Round 2, c79b631: FAIL. The proxy checked the path literally where `-Path` takes wildcards, and the exit-status logic took it for a function. Fixed in 78c48f4.
- Round 3, 18487a4: FAIL. Replacing `cd` overrode a user's own `cd` and lost its folder-only completion. The approach changed in 7ab6c59: the typed line is rewritten before it runs, `cd` itself untouched.
- Round 4, 86e6205: FAIL. A line break, curly quotes, a user's own Set-Location function and tr-TR's capital i each slipped through. Fixed in 5211c92.
- Round 5, 5211c92: PASS.

## T127

- Round 1, the v0.2.0 draft at 0cef65d: FAIL. The notes' clipboard line was too broad, the history figure lacked its measure, Mica lacked its build, and the composition crash fix was missing. Fixed in the draft's notes.
- Round 2: FAIL. Only the newest paste's password reads unasked; fixed.
- Round 3: PASS. Published: v0.2.0 at 0cef65d, latest, twelve assets; the website's six version-less links resolve to it.

## T129

- Round 1, 19e0cd2 with the Windows fork's 8b9ce59: FAIL. The fork's README left the Mica change out of its removal condition. Fixed on the fork's main, ef184ce, a README alone.
- Round 2: PASS. 0.2.0's Windows zip, rebuilt at 19e0cd2 (sha256 b7c9e6f0…), replaced the one on v0.2.0 with its alias.
