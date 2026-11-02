# Reviews, T100 to T138

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

## T130

- PR #48 merged at the owner's word: macOS 400 tests and Linux 414 pass on the merge, and the PR's zsh tests fail with its blanking taken out.
- Round 1, the merged PR: FAIL. Blanking erased a background job's output on the prompt's line, a continued command's first lines and the prompt without zle; a `sched` at every prompt reset `$pipestatus`. Reworked in eb2c3e2 as kitty keeps a prompt's rows.
- Round 2, eb2c3e2: FAIL. A soft wrap into the prompt joined output to it, `unsetopt zle` kept `redraw=1`, `reset-prompt` lost `$pipestatus`; a background job's output kept and cut, recorded as a limit, as kitty cuts it. Fixed in 81cbbcd.
- Round 3, 81cbbcd: FAIL. `ksh_arrays`, statuses past 128, `err_return`, `globalrcs` off and an unset PS2 under `nounset` each broke the line-init path. Fixed in c5db798.
- Round 4, c5db798: FAIL. Five more in the line-init path: a loaded helper, a failing widget before ours, options and locals at expansion, an empty IFS. The path went in 81906b3: a theme holding the last hook gets its A printed, as kitty does.
- Round 5, 81906b3: FAIL. A widget running the precmd hooks printed a mark mid-line, fixed in af5a8f7 as kitty guards it; a printing hook under such a theme, recorded as kitty's own limit.
- Round 6, af5a8f7: FAIL. A line typed taller than the screen piled up copies: kept rows reached the history zsh's redraw cannot reach. Fixed in 8e2c91f as kitty keeps no history row.
- Round 7, 8e2c91f: FAIL. A blank line in the prompt hid its start above the screen. Fixed in fbd6231, the start sought past blank rows.
- Round 8, fbd6231: PASS.

## T131

- Round 1, b0c63b4 with vte 8fa83a9 and alacritty d2781ca: PASS.

## T132

- Round 1: FAIL. An opened folder's new file went unmarked, an edit inside a closed folder too, and closed folders were still read. Ely `Tree::on_toggle` (d1d7cf7, main 141ecf2); Git's marks every tick.
- Round 2: FAIL. A remounted tree opened what Files did not read, a renamed folder was read on, marks rebuilt on the main thread. Files seeds the tree, prunes gone folders, builds marks off the main thread.
- Round 3: FAIL. A Git failure cleared the marks, a deleted root's folders were read on. Failures keep the last marks, logged once; an unreadable parent drops all below.
- Rounds 4 to 9: FAIL, each a case of an unreadable `.git` that git reports as no repository: a literal path scan, links, `GIT_CEILING_DIRECTORIES`, nested repositories, `core.worktree`. Git answers it exactly as no repository, so Files believes git, as the Git panel does; recorded in AGENTS.
- Round 10: FAIL. A closed folder's cached listing outlived its deletion; `prune` takes them too.
- Round 11: FAIL. A closed root was read no more; the root is always read.
- Round 12: PASS.

## T133

- Round 1: FAIL. Ely kept her error face when C cleared the failure, and a user's `PROMPT` with `&` ran through `/k prompt`. `TerminalEvent::Ended` at each change; the prompt through the environment.
- Round 2: FAIL. A script block around `cmd` left `$?` true, and OSC 7 decoded a folder named `rel%20one`. The line runs as typed, the next prompt restores `PROMPT`; cmd reports its folder by ConEmu's raw OSC 9;9.
- Round 3: FAIL. `ElyCmdBack` unset under strict mode, and cmd's first D ended PowerShell's `cmd` early. Initialized; cmd sends no D.
- Round 4: FAIL. A nested cmd's named A closed PowerShell's command, dropping a program's records. Nested, its A is unnamed.
- Round 5: FAIL. An inherited marked prompt kept its old name. Ours is marked again for its place.
- Round 6: PASS.

## T134

- Round 1: FAIL. Tooltips kept the system font, gpui drawing each as a root of its own. Ely `b0350ad` (main `dd727f4`) gives a tooltip the theme's family, which ELYTerminal now sets.
- Round 2: FAIL. The root's 13 px shrank text of no size of its own from gpui's 16 px. The root takes gpui's 1 rem, scaled.
- Round 3: PASS.

## T135

- Round 1: FAIL. cmd given a share started in the Windows folder, and wsl given another distribution's share started the default. WSL now starts that distribution there; cmd pushed into the share.
- Round 2: the reviewer's pane read round 1's verdict again; round 3 read the change.
- Round 3: FAIL. pushd's drive outlives the session. On the owner's PC a session closed through ConPTY left it mapped, on the letter of the owner's remembered `Z:`; cleaned. pushd went: cmd says itself it cannot sit in a share.
- Round 4: PASS.

## T136

- Round 1: FAIL. `tr` hid a missing zip digest behind an empty SHA, and `MpCmdRun.exe` is not on PATH. The digest fails alone; the guides call it by its full path from an administrator's PowerShell.
- Round 2: PASS.

## T137

- Round 1: FAIL. The folder argument's lines missed cmd's share limit, winget's line its catalog's merge, the guides Windows 11's Show more options, and the acceptance table still pinned Ely `494c17d`. Fixed.
- Round 2: PASS.

## T138

- v0.2.1 at 0f4696a, published and latest, twelve assets: CI passed on macOS, Windows and Linux; the Windows zip from `scripts/windows-zip.ps1` on the owner's PC after its 405 tests (sha256 a1c84372…); the four Linux packages from the Linux packages run 41159321599, checked against its SHA-256 file; the disk image from the Release run 41159369740, stapled and accepted by Gatekeeper as notarized Developer ID. The website's six version-less links resolve to it.
- `ELY-Open-Source/homebrew-tap` made; `brew install --cask ely-open-source/tap/elyterminal` installed 0.2.1, team 9PDD8M2SMS, and uninstalled. winget: `winget validate` passed; microsoft/winget-pkgs#466704, its CLA signed at the owner's word from the owner's account, waits on Microsoft's validation and moderators.
- Issues #40, #41, #44, #45, #46, #47 and #50 answered and closed; #42 answered and open until winget merges; PR #48 thanked.
