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
