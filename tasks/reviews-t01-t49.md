# Reviews, T01 to T49

## T01

- Round 1, 4e2f3c9: FAIL. Linux had no X11 or Wayland backend and would start headless; macOS CI linked no binary; `Cargo.lock` passed the 500-line rule unstated. Fixed in 9c0c067 and f8857fe: `wayland` and `x11` on, Linux starts with `WAYLAND | X11` and no headless; `cargo build --locked` in macOS CI; the lockfile named as exempt.
- Round 2, f8857fe: PASS. The Windows runner's missing `rustc` goes to T23.

## T02

- Round 1, e1770ba: FAIL. One shared temp name let overlapping saves corrupt the file; parse errors lost the field name. Fixed in 83ec4b1: a temp per save (pid, counter, create_new), cleaned on failure; serde_path_to_error names the field; a four-writer test, red on the old save.
- Round 2, 83ec4b1: PASS.

## T03

- Round 1, 7d4d535: PASS. Theme files match upstream 21f281d byte for byte.

## T04

- Round 1, 62e50fe: FAIL. A folder without enter permission started the shell elsewhere; no pid for the app; folder and exit unlogged; grid and pty sizes split past the kernel's cap. Fixed in 96ae85b.
- Round 2, 96ae85b: FAIL. The permission check raced alacritty's chdir, whose result alacritty ignores; closing a live pty logged nothing. Fixed in fff436c: ELYTerminal's own Unix pty, a checked chdir in pre_exec that fails the spawn, closing and hung-up logs with a test.
- Round 3, fff436c: FAIL. Drop waited on the program while holding the master, so a program ignoring SIGHUP never ended. Fixed in 0e7f309: the master closes before the wait; a test that traps HUP went red, then green.
- The review cap of three rounds is reached. The round-3 fix stands on its red-green test, not a fourth round.

## T05

- Round 1, dfaf378: FAIL. fish searched <dir>/fish/fish, so its script never loaded (fish 4 sends OSC 7 itself and hid it); PowerShell's Restricted policy would block the script; control bytes in a folder name broke OSC 7; bash and PowerShell prompts reset $?; install swallowed read errors. Fixed in c2da9dd, each break shown red in a scratch copy.
- Round 2, c2da9dd: PASS.

## T06

- Round 1, ae1d8ce: FAIL. SGR 8 text showed under the block cursor, in emoji and in underlines; an inactive window kept a solid cursor; combining marks dropped below their base (gpui adds CoreText's upward y offsets to downward screen y). Fixed in 3c8f746: hidden cells blank in the frame; window activation counts as focus; every glyph painted from the shaped layout on one baseline, CoreText's y flipped on macOS.
- Round 2, 3c8f746: PASS.

## T07

- Round 1, 387bfa1: FAIL. Dim geometry brightened where pieces overlapped (gpui blends each quad and path triangle on its own); a combining mark sent its drawn base back to the font, thicker and past the cell. Fixed in 2804aa6: overlaps cut from rects, strokes turned into polygons, the cross cut around its first band, a drawn base drawn under its marks; pixels of the dim row checked, no value above the dim color.
- Round 2, 2804aa6: PASS.

## T08

- Round 1, 25365a9: FAIL. Ely's focus bindings took F6 and Shift-F6; any kitty flag forced disambiguation; IME commits ignored kitty; macOS repeats routed through AppKit's commands went out as presses; Cmd combos never reached kitty; a stray key.zig of 876 lines slipped past the line rule. Fixed in 11d7a30: NoAction bindings under Terminal; each flag reshapes only what it names; Cmd is Super, bit 8; repeats read from AppKit's event; a local monitor hands Command key-ups to the view; key.zig out; the line rule reads every text file. The IME finding was answered: kitty's keys.c and Ghostty's encoder send commits as UTF-8 under every flag. Mutations of the encoder shown red in a scratch copy.
- Round 2, 11d7a30: PASS.

## T09

- Round 1, 9f81f1f: FAIL. Resetting mode 9 revived an older 1000 (alacritty keeps each mouse mode apart); a second held button wiped the first one's release; URLs lost their combining marks; selections logged nothing; two shot comments ran past 200 characters. Fixed in d6fa38e: the tap holds one tracking and one format as xterm does; reported buttons kept as a set, motion reports the first held; marks kept; selection start, run and end logged; the gesture syntax moved to AGENTS.md. shot now hands real buttons to the view and shows its window without activating.
- Round 2, d6fa38e: FAIL. A NUL inside CSI ? 1000 l ended the tap's sequence while vte runs it and goes on, so tracking stayed on. Fixed in 8bb2763: the scanner mirrors vte's escape and CSI transitions (C0 runs, DEL and high bytes skipped, CAN and SUB cancel, ESC restarts, subparameters count toward 32 values); a test compares it with vte's parser after every byte. Copy on select and OSC 52 primary writes go to Linux's selection.
- Round 3, 8bb2763: PASS.

## T10

- Round 1, 4d8788f: FAIL. Matches went stale after a resize or a scrollback trim; the current match kept its index, not its line; alacritty's search read no combining marks; whole word used ASCII edges; a reset left the old title. Fixed in 1c21dfd: find runs Ely's find_all on the grid's shown text (marks, hidden cells blank, wrapped rows joined, Unicode words); output, resizes and settings changes search again; the current match follows its line; a reset replays the title.
- Round 2, 1c21dfd: FAIL. Following was quadratic on a long line (60 s on a million characters); text appended to the current line moved the current match; a reset inside a synchronized update kept the old title. Fixed in 11b2ce6: one pass, a forward walk and line ends found once, guarded by a timed test; identical lines rank first, then lines grown or cut, then line and column; a cleared or trimmed history moves no line; the title replays on CursorBlinkingChange, in the parser's order.
- Round 3, 11b2ce6: FAIL. With two identical lines, growing the current one moved the match to its twin. The three-round cap reached: fixed in 39761d9 on a red-green test, a line kept, grown or cut ranks first when held under the same line above or in its place. The reviewer's round 2 and 3 scenario records replayed with fresh homes, focus and long-line cases aside, all as expected; three mutations of the rule each fail a test.

## T11

- Round 1, 34ce046: FAIL. A press on a blank column or a header gave focus to the focus root, below which the Workspace key context and its toggle actions sat, so Cmd-B and Cmd-Alt-B did nothing. Fixed in 0de9e15: the context and its actions sit on the outermost element, above ResizeBorder and the focus root; the four press-then-toggle cases hide both columns and save both flags.
- Round 2, 0de9e15: PASS.

## T12

- Round 1, 5fd674e: FAIL. A session switched while a mouse button was down left the hidden terminal's pressed button set: its release landed on the other session, and back on the first one plain moves went out as drags. Fixed in dcf9f7b: a terminal about to be hidden ends its mouse gesture as releases would, the selection drag ending and each heard press reported coming up at the last cell in the current protocol; the reviewer's repro now gives the program a press and its release, as the single-session control does.
- Round 2, dcf9f7b: PASS.

## T13

- Round 1, 8369d96: FAIL. A file or folder named with a newline aborted gpui's line shaping under Ely's Ellipsis; on macOS a decomposed name on disk missed Git's composed mark; a folder named with a newline split the rev-parse prefix, so marks fell on a wrong path. Fixed in 937dab8: a label built from a path shows control characters as their pictures, the session row's folder too, which shared the cause; on macOS a name meets Git's mark in composed form; the prefix keeps its own newlines.
- Round 2, 937dab8: PASS.

## T14

- Round 1, e296668: FAIL. A staged copy kept its source as an old path, so unstaging or diffing the copy took the source's own staged change along; paths went through lossy text before git saw them, so a name not in UTF-8 could unstage another file that showed alike; diffs still ran text converters, which can hide a real change; a CRLF warning on stderr failed a valid untracked diff; a conflict where our side deleted the file diffed as empty. Fixed in 911500c: paths stay git's bytes until shown, Files' keys and listings too, which shared the cause; a copy is a new file; diffs pass --no-textconv; an untracked diff stands when git printed one, its warning logged; a conflict diffs against HEAD.
- Round 2, 911500c: PASS.

## T15

- Round 1, a8a30d3: FAIL. After a folder change the old view stayed until the new read landed, so an action pressed in between reached the repository left behind; a diff still loading from the old folder popped up over the new one; a slow commit's success emptied a draft typed while it ran. Fixed in 27270ae: no action runs until the shown folder has read, and the view blanks on a folder outside the shown work tree; a folder change drops a diff still loading; a commit empties only the fields that still hold what it committed; actions chain, each after the one asked before it, a race of the same kind found while fixing.
- Round 2, 27270ae: FAIL. Entering a nested repository kept the outer view while it read, and a commit finishing then emptied a retyped draft: its completion checked the view, not the read. Fixed in fcd0792: the completion passes the same gate as every action.
- Round 3, fcd0792: PASS.

## T16

- Round 1, 3e1535e: FAIL. The shell check asked only for an execute bit, so a file the running user may not run, as 0641 or an execute-deny ACL, saved and then failed every new session; a bare name counted relative PATH folders, which hang on each session's own folder, so the check and the start could find different programs. Fixed in 3c51b01: the check asks access(X_OK), and a bare name looks only in PATH's full folders; a session runs the full path the check found.
- Round 2, 3c51b01: FAIL. On Windows the .exe candidate replaced an explicit extension, so review.com saved and review.exe ran; a program found at a path not in UTF-8 saved, though the start takes UTF-8 and refused it. Fixed in 9e69288: .exe joins only a name without an extension; the shared resolver returns the program as UTF-8 text and refuses a match that is not, before the save.
- Round 3, 9e69288: PASS.

## T17

- Round 1, f2cc16f: FAIL. The font picker saved a family gpui cannot load, such as Apple Color Emoji, which has no m, and the terminal drew with a fallback without a word; after a refused save a number field kept the refused draft, so once a later save cleared the message the field and the settings disagreed. Fixed in ee37018: a font saves, and a settings file starts, only when gpui loads it as the terminal draws it, and the terminal draws with no other font; numbers save on commit; a refusal resets the number fields and the font picker to the settings.
- Round 2, ee37018: PASS.

## T18

- Round 1, 764ff45: PASS.

## T19

- Round 1, c98bbad: PASS.

## T20

- Round 1, 6523632 and 107069b: PASS. 6523632 fixed a test the first Linux run caught: on Linux a program that ignores SIGHUP reads EIO when its pty closes, so GNU cat exits 1 where macOS gives EOF.

## T21

- Round 1, b5e4967: PASS. The work found that Ely's ResizeBorder painted a client-drawn window's pane solid, so no Linux window showed the desktop and T20's blur could not show; Ely 5dc4dd1 adds ResizeBorder::glass, and T20's region now covers the pane alone.

## T22

- Round 1, 5ee67bd: FAIL. Edit's Copy, Paste and Select All named the terminal's own actions, so with find, a settings field or the commit field in focus gpui disabled them and a menu press did nothing; Select All took a space under a combining mark for a blank and dropped the mark at either end; two new comments ran long. Fixed in 37220a7: Ely a2cb531, approved by the user, exports its text input's Copy, Cut, Paste and SelectAll, which the terminal and the menu now name; a cell with a mark counts as a character; every new comment is one short line.
- Round 2, 37220a7: PASS.

## T23

- Round 1, e3b5cfb..82bdbaf: PASS. The owner's checks on the release build found three faults, fixed in 09930c2 before the review: on macOS 27 gpui's Selection material drew no blur, the debug build lagged, and no button hid the side columns. CI went green once the owner let the runners be put right: fish and PowerShell on the macOS runner, rustup's proxies on the Windows runner, a Linux runner of the repository's own; on the way, run 907 caught the PowerShell test typing before the line editor read keys.

## T24

- Round 1, c0a9eb3..65bb4d3: PASS. Before the review, 65bb4d3 named the icon's parts as SVG groups instead of comments, drawing the same pixels, and made mac-app.sh read the version in a clone named elyterminal. CI run 926 on 65bb4d3 passed on macOS, Ubuntu and Windows.

## T25

- Round 1, 2b8ae5d and 0075667: FAIL. The v0.1.0 release was not yet on Gitea, so the task line's release stood open.
- Round 2, 59e41ea: PASS. v0.1.0 at 0075667 carries the disk image, the Windows zip and the Linux tarball; the reviewer downloaded each and found its SHA-256, signature, icon, manifest and glibc as recorded. CI run 934 on 0075667 passed on macOS, Ubuntu and Windows.

## T26

- Round 1, 6f7d3a9 and 0f87676 with Ely cdc5ce8: FAIL. v0.1.1 was still a draft, so the task line's release stood open.
- Round 2, c316d27, 90a4625, 7bfb462, 43ea5b1 and 194c7bb: PASS. The owner's later asks went in first: a gear for Settings on Windows and Linux, Ctrl-V paste on Windows, the arrow pointer, the settings window without General, with the shortcut list and with columns that scroll apart, and the release's disk image signed with Developer ID and notarized on the macos-signing runner. v0.1.1 at 43ea5b1 is published and the repository public; the reviewer downloaded each file anonymously, checked its hash, signature and notarization, and drove the app on X11 and macOS. CI run 955 on 43ea5b1 passed on macOS, Ubuntu and Windows.

## T27

- Round 1, 340c7a9 and d8ebade: FAIL. A failed read marked the file's stamp as seen, so a theme file made readable again was never worn; `is_file`, `exists` and `metadata().ok()` took a permission error for absence, so a fresh install wore Catppuccin Mocha over an unreadable Omarchy folder, and an outage while following logged nothing. Fixed in aae7fba: the watcher compares the file's text, logs a failure once while it lasts and tries again; only a missing file means no Omarchy, and a file that cannot be read stops the start. The v0.1.1 Linux tarball was rebuilt from aae7fba on the owner's PC, which now runs Omarchy, and took the place of the one from 340c7a9 (1faf8ca).
- Round 2, aae7fba and 1faf8ca: PASS. The reviewer reproduced both faults on the parent build and saw them gone, ran all 22 of Omarchy's themes through the window, and matched the new tarball's download to its SHA-256. CI run 976 on aae7fba passed on macOS and Ubuntu; its Windows job waits for the PC to run Windows again.

## T28

- Round 1, 741e48b with the v0.1.1 release notes: PASS. The reviewer read the new lines against Hyprland 0.56.2's `shouldBlur` and background-effect protocol and Omarchy 4.0.4's default `looknfeel.lua`, and the owner's captures with Hyprland's blur on and off against the acceptance row, which stays Partial for Wayland.

## T29

- Round 1, 3e30f95: FAIL. A shell that ran `exec herdr` kept its process group, so the session never looked at its foreground again and the panel stayed where it was; a relative `HERDR_SOCKET_PATH` was looked up from ELYTerminal's own folder, not the client's, so no socket answered. Fixed in 2af45cb: the foreground is its group and the executable its leader runs, a program that did not read is tried again, and a relative socket lies in the client's folder (`/proc/PID/cwd`, `proc_pidinfo` on macOS); on Linux an executable an update replaced still names itself herdr.
- Round 2, 2af45cb: PASS. The reviewer ran real herdr 0.9.5 under the native view: a direct start, a delayed exec, a relative socket, both at once, and a folder with a space and an é each gave the folder events herdr's own snapshot named. CI run 988 on 3e30f95 and run 990 on 2af45cb passed on macOS, Ubuntu and Windows.

## T30

- Round 1, 1bf420c: FAIL. On X11 a maximized window, minimized, wrote the screen's size, since gpui's X11 window calls a maximized window unmaximized while it is hidden; and the window could be dragged under the 100 by 100 the settings keep, so a smaller size was refused and lost. Fixed in 2f58a06: a hidden window keeps the size from before, and the window is no smaller than 100 by 100.
- Round 2, 2f58a06: FAIL. A size the window took just before something covered it was skipped while it was hidden and never read again once it showed. Fixed in 9729d24: a window that shows again is read again; and the window opens centered at whole points, since macOS grew a window whose centered edge fell on a half point by a point, so a kept 900 by 600 had opened at 900 by 601.
- Round 3, 9729d24: PASS. The reviewer replayed the cover on macOS through gpui and AppKit, against the parent too, and saw the size kept once shown and the restart open at it; the window opened at its kept size and a move wrote nothing; on X11 a maximized, minimized and restored window and a resize minimized at once kept the right size, and the 100 by 100 minimum held. CI runs 993 on 1bf420c and 995 on 2f58a06 passed on macOS, Ubuntu and Windows; run 996 on 9729d24 passed on Ubuntu and Windows.

## T31

- Round 1, 4364802 and 2a1060c, with release v0.1.2: PASS. The reviewer compared `Cargo.lock` with its parent, where only elyterminal moved to 0.1.2; matched the 112 files of the source the builds came from to the tag byte for byte; downloaded the three files anonymously against their SHA-256s; checked the disk image's notarization and its app's signature, the zip's icon, manifest and imports, and the tarball's glibc 2.35; and ran the downloaded macOS app with herdr 0.9.5: the folder followed herdr's focus, and the window reopened at its kept 923 by 617. CI run 1000 on 4364802 passed on macOS, Ubuntu and Windows before the release was published; Release run 1001 built the notarized disk image.

## T32

- Round 1, d7dc1af: FAIL. On Windows the search took the herdr nearest under the shell, so a server the client started under itself, or one started beside it, was read as the session's herdr; its arguments named no client, nothing was tried again, and the panel stayed where it was. Fixed in 3b5bf46: the herdr processes at or under the shell are listed, none under another herdr, whose server and panes are its own; when that list changes their arguments are read, nearest first, and a server or a command beside the client is passed by.
- Round 2, 3b5bf46: PASS. The reviewer ran the committed search and pick against both trees, a nested server, a stale parent loop, an unread process that reads later, a client's start, end and return, and a remote client, eight probes; undid each half of the fix in a copy and saw its regression test fail; and on macOS saw a herdr answer that came 800 ms after the shell's report dropped. CI run 1027 on d7dc1af passed on macOS, Ubuntu and Windows, run 1046 on 3b5bf46 on Ubuntu and Windows, the Windows runner running 120 and then 121 tests; the Linux suite and the real-herdr X11 run passed on both. No real herdr has run on Windows yet.

## T33

- Round 1, 6336c05 and da55942, with Ely 5c2822c and 005faef: FAIL. In a window too narrow for both kept widths, a drag set off from the squeezed width drawn, and the width it reported was squeezed again: the border jumped back against the pointer, and a drag with no sideways move saved the squeezed width. Fixed in 2173c74 through Ely 0b4848b: a drag sets off from the wider of the width drawn and the one kept, and one that has not moved sideways changes nothing.
- Round 2, 2173c74, with Ely 0b4848b: FAIL. A drag that went out and came back where it set off kept the last width it had reported, since its move back to the press was taken for no move. Fixed in 3ea8b61 through Ely da3fdca: a drag back where it set off gives the width it had, and the width a drag sets off from lives with its press, so each press starts afresh.
- Round 3, 3ea8b61, with Ely da3fdca: PASS. The reviewer replayed drags out and back on both borders on macOS, at 240 and squeezed at 900 and 900, presses one after another, a second drag with no sideways move, a drag past a limit and back, and a column hidden and shown again, and saw each width and border come back; Ely's round-trip test failed on 0b4848b and passed on da3fdca. Before the review, testing found the handle's line painted half a pixel on macOS and none on X11, since gpui paints a border no wider than half its box; Ely 005faef draws it filled. CI runs 1058 on da55942 and 1065 on 2173c74 passed on macOS, Ubuntu and Windows; run 1067 on 3ea8b61 passed on Ubuntu and Windows, its macOS job queued behind Ely's. No real drag on Windows yet.

## T34

- Round 1, c53b540: FAIL. PowerShell called a cmdlet's failure under `-ErrorAction Ignore` a success and read 1 for a native exit under `$PSNativeCommandUseErrorActionPreference`; bash 3.2 sent no C or D for a line that is only a subshell, and none at all while the user held a DEBUG trap; zsh left the first prompt unmarked under a theme that sets PS1 in its own hook. Testing before the review had found PowerShell's prompt broken under strict mode. Fixed in 0a22200: PowerShell reads a native code by the new object each run stores; bash 3.2 runs the user's trap after its own and keeps `$_`, which its trap had overwritten, and a lone subshell's D comes from the command number; zsh wraps the first prompt again through sched, which would reset `$pipestatus` on later prompts.
- Round 2, 0a22200: FAIL. Under `extdebug` the bash 3.2 trap's status skipped the next command; a lone subshell still sent no C on bash 3.2; PowerShell gave a cmdlet's failure the code of a program run earlier on the line. A probe then found the trap running for key bindings as for typed commands, with nothing in it to tell them apart. Fixed in d4bc6e6: bash 3.2 drops its DEBUG trap and sends no C, its D from the command number; PowerShell reads the line's syntax tree, since a hook on every command lookup slowed a loop by half; each check waits for the prompt before its line, so a repeated status is the line's own.
- Round 3, d4bc6e6: FAIL. The reviewer accepted bash 3.2 without C. PowerShell read 1 for a call through a variable, and bash without `promptvars` marked no command and said nothing. Fixed in 2c25116, after the last round, each with a check that fails on d4bc6e6: a line keeps a program's code unless it surely ends in PowerShell, and bash without `promptvars`, zsh without `prompt_percent` and PowerShell without PSReadLine say at start what they cannot mark. CI runs 1071 on c53b540, 1074 on 0a22200, 1076 on d4bc6e6 and 1077 on 2c25116 passed on macOS, Ubuntu and Windows; the Linux suite ran fish 4.0 and bash 5.2, CI's Ubuntu fish 3.7.

## T35

- Round 1, 15e83f5: FAIL. A prompt that opens a link of its own lost its mark after that link, and a mark inside a link left open took over the program's target; a blank row inside a prompt split it in two; the rewrite made `133;AB`, which the session refuses, a prompt. Fixed in 310c219: an OSC that may be a mark is held until it ends and read as the session reads it, and each prompt's link carries the prompt's number.
- Round 2, 310c219: FAIL. A prompt all inside a program's link carried no number, nor a first row that was; an ESC followed by 1 MiB of controls held them all, and the tap stopped reading. Fixed in e2bb615: a program's link inside a prompt keeps its target and carries the prompt's number in its id, a prompt inside a link left open opens it again with the number, and controls inside an escape go on before the ESC.
- Round 3, e2bb615: FAIL. A target over 256 bytes went unnumbered, and the end of such a prompt left output numbered; controls after an OSC's closing ESC went on before it, so the parser, still inside the OSC, dropped a line break. Fixed in 11ec0ef, after the last round, each with a check that fails on e2bb615: targets are kept whole as the scanner keeps them, and the tap reads about 2 KB at a time to hold the growth, as fast through a pty as larger reads; an ESC that ends an OSC the parser has goes on at once. The reviewer's 33 round 3 probes then ran against 11ec0ef: 32 passed, and the one left counts 200 reads, which carry 2 KB each now, not a stall, as its pty twin showed READY. CI runs 1080 on 15e83f5, 1082 on 310c219, 1083 on e2bb615 and 1085 on 11ec0ef passed on macOS, Ubuntu and Windows. The keys off macOS are bound, not run.

## T36

- Round 1, 42b272e: FAIL. A clear dropped the records before its other pairs were checked, so a clear with a newline in its msg emptied them; only the last of a repeated key was checked, so an earlier app over 32 bytes passed; the 4096-byte bound counted the body after the scanner skipped its controls, so 5018 bytes passed; the scanner left an escape at DEL or a high byte, which vte passes by, so `ESC DEL c` reset the screen and kept the records. Fixed in 4a269cb: every pair of a known key is checked, a clear's too, before the last of each wins; the scanner counts a sequence from OSC through ST with its skipped controls; the scanners and the prompt rewrite pass DEL and high bytes by in an escape.
- Round 2, 4a269cb: PASS. The reviewer's probes failed on 42b272e and passed on 4a269cb at every read split; in the app on macOS the clear with a newline, the repeated long app and the 5018-byte report were refused, a full reset emptied the records, and a program read the query's answer before DA1's. CI runs 1089 on 42b272e and 1090 on 4a269cb passed on macOS, Ubuntu and Windows.

## T37

- Round 1, ef33a0d: FAIL. OSC 99's base64 was decoded and checked as UTF-8 chunk by chunk, so a payload split inside a group or inside a character dropped; decoded text was refused for a newline, which only plain text may not hold; the scanner bounded the whole body at 4096 bytes, so a full 4096-byte encoded payload with its metadata dropped and its next chunk showed alone. Fixed in 628ed6b: base64 is read group by group, so chunks split before encoding or after join, and the text is checked as UTF-8 once whole; plain text alone refuses controls; a chunk that breaks a rule drops what its notification gathered; the scanner keeps an OSC 99 body to 5124 bytes, room for 1024 bytes of metadata and a full payload.
- Round 2, 628ed6b: PASS. In the app on macOS the round 1 probes gave Hello, the split emoji, both lines of the body and 3072 `a`s with END; boundary probes passed 1,627 chunkings after encoding and 433 before, and the scanner 5,140 read splits; the round 1 probes failed again on ef33a0d and with the scanner's old bound. CI runs 1093 on ef33a0d and 1095 on 628ed6b passed on macOS, Ubuntu and Windows.

## T38

- Round 1, a175b18: FAIL. herdr's records took ids under `herdr/`, which a program may name through OSC 7501, so a program's `herdr/build` went at herdr's first snapshot, an empty one too, and at its leaving. Fixed in 7d56063: herdr's records take `herdr:` before the pane, a colon no OSC 7501 id holds, since a program's id ends at one; the test that keeps a program's `herdr/build` and `herdr` through an empty snapshot, a full one and herdr's leaving fails with the old prefix.
- Round 2, 7d56063: PASS. The reviewer's round 1 probes failed on a175b18 and passed on 7d56063; probes with a socket and JSON replies of the reviewer's own, under real GPUI scheduling and the project's pty, passed the states and fields, clear and reset, errors, a reply after the shell's folder, and herdr's leaving; no herdr server of the owner's was asked. CI runs 1127 on 72acbe4, 1128 on a175b18 and 1139 on 7d56063 passed on macOS, Ubuntu and Windows.

## T39

- Round 1, 85bcbf4: FAIL. A hidden session that waited, worked and waited again in one output lost its mark, since the workspace heard the records once an output; an open hover kept the text it opened with; ten long records ran past the window and could not be read; at 0% the ring vanished on a selected row in Ely Dark, its track the row's ground. Fixed in 53b8978: the view notes each change of the state its row shows and tells the workspace of each; hover reads the records and the last command's status, now the view's, at each frame, stays under the pointer and scrolls within the window; the ring holds Ely's loader.
- Round 2, 53b8978: FAIL. herdr's records replaced in the same state left an open hover as it was, since nothing drew the view again; a hidden session that waited and worked again in one output still lost its mark, since the workspace checked the state at the output's end. Fixed in 3aaeb36: a change of herdr's records draws the view again; a hidden session's turn to waiting marks its row though it went on within the same output.
- Round 3, 3aaeb36 at efda50f: PASS. In the app on macOS the reviewer's repros of all three rounds passed: herdr's new records showed in an open hover through a mock herdr of the reviewer's own, a hidden session that waited and worked, or waited and finished, in one output kept its mark until shown, and the hover, long list, scrolling and Ely Dark 0% repros held. CI runs 1142 on b504bfb, 1144 on 85bcbf4, 1146 on 53b8978 and 1149 on efda50f, which holds 3aaeb36, passed on macOS, Ubuntu and Windows.

## T40

- Pre-review, 4e029f1, a first version kept in a clone and never pushed: three Claude agents read it against kitty's spec, gpui's platform code, and the PRD and rules. They found the alive answer behind a later DA1, since the workspace answered it after the output; close and expiry claimed on Linux, where gpui closes nothing; an expiry lost with its session; a body read as markup on Linux; a press lost with its record; bash 3.2's redrawn D posting again; and no test of the posting conditions. 31de5e4 rewrote it so each session keeps its notices in its terminal view, where answers keep their order; 52576bb fixed the rest; efda50f posts at each turn of the row, as T39's round 2 set the waiting mark.
- Round 1, 31de5e4 to 52576bb: FAIL. The first prompt, sent at 31de5e4, was lost while Codex started, so the round ran at 52576bb. A notification took the session's title from before a title set or a full reset in the same output, since the tap reads a notice before alacritty's parser reads the title. Fixed in 22752a2: the tap follows OSC 0 and 2, the title stack and full resets, and sends each title in order.
- Round 2, 22752a2: FAIL. alacritty's later title events still overwrote the tap's, so a full reset, 2 KB more output and a notice named the old title; the tap kept 17 parts of a title where vte keeps 15, and dropped a title over 4096 bytes that vte keeps. Fixed in 140c2a5: the title comes from the tap alone, so the row, the window and a notification name the session alike; the tap splits a title as vte does and keeps 65536 bytes of one.
- Round 3, 140c2a5: PASS. In the app on macOS, outside the bundle, where the log names each post, the reviewer's probes of all three rounds passed: a title set in one write or split, after a reset with up to 32 KB more output, of 17 parts and of 4100 bytes; 21 title cases matched the pinned vte and alacritty; and the lifecycle, rate limit, untracked, alive before DA1, expiry, close, in-view and press probes held. No system has shown a notification yet; the owner posts and presses one on each. CI runs 1148 on 31de5e4, 1149 on efda50f and 1150 on 52576bb passed on macOS, Ubuntu and Windows; 140c2a5's code is CI'd on T41's version commit, which holds it.

## T41

- Round 1, 7569f00 with release v0.1.3: PASS. The reviewer compared `Cargo.lock` with its parent, where of 844 packages only elyterminal moved, to 0.1.3; matched the 127 files of the tag's source archive to 7569f00 byte for byte; downloaded the three files anonymously against their SHA-256s; checked the disk image's stapled notarization and its app's Developer ID signature, hardened runtime and version 0.1.3, the zip's icon and manifest against their sources byte for byte and its imports, no C runtime DLL among them, and the tarball's three files and glibc 2.35; and matched the notes to T32 to T40. The app was not run, since its first notification asks the owner's leave. CI run 1156 on 7569f00 passed on macOS, Ubuntu and Windows before the release was published; Release run 1157 built the notarized disk image.

## T42

- No review: the owner waived it for this change.

## T43

- No review: the owner waived it for this change, its release included.

## T44

- Round 1, fc10c4d: FAIL. Under kitty's events flag alone, Option-Shift-[ as Alt sent ESC [ for ESC {, since the legacy bytes a key falls back to took the restored key; a key that is down was known by gpui's name, which Shift changes, so a release after Shift came up went nowhere, which on Windows keeps herdr's lease on the key. Fixed in 6f8bfae: legacy bytes keep gpui's character; a key that is down is known by AppKit's key code or Windows' virtual key; shot posts a press or a release alone.
- Round 2, 6f8bfae: FAIL. On Windows a repeat that types after its modifiers came up overwrote the record its press made, so its release went as no record. Fixed in 3f47134: a repeat goes as a record where its press did, through one function, records, tested on every system.
- Round 3, 3f47134: PASS. The reviewer's replay of the Windows handlers with herdr 0.9.5's lease code passed both release orders after a typing repeat, and a mutant that drops the fix failed; on macOS real herdr switched one, two, three, two, one with Shift, Ctrl or every modifier let go first; kitty flags 2 and 7 and plain text and an IME commit sent the bytes they should; check.sh, 226 Linux tests and the Windows cross-check passed.

## T45

- Round 1, ae3de24: FAIL. Inside herdr a plain file:// link the app had opened before went to herdr, whose client opens web links only, so nothing opened. Changed in af66a70: a link the app found of another kind it opened itself.
- Round 2, af66a70: FAIL. herdr keeps its panes' OSC 8 targets from the terminal while it takes the mouse, so an OSC 8 link labelled with a file:// URL and aimed at a web page opened the label's file. Fixed in c30bd51: while herdr takes the mouse every press is herdr's again, its web-only rule with it, as its client and docs say; Shift with the press, herdr's named way past it, lets the terminal open what it sees.
- Round 3, c30bd51: PASS. With real herdr 0.9.5 the label's prefix and its file:// part both opened the web target through herdr; Shift-Cmd-press opened a visible file:///etc/hosts in the app; with herdr's mouse capture off the app opened OSC 8 web and file targets and plain file URLs itself; wrapped, tail, OSC 8 and long web links opened whole through herdr, and links outside herdr in the app; check.sh passed.

## T46

- Round 1, 9aeb3f3: FAIL. A press on Ely made her panel AppKit's key window, so keys typed after it went nowhere; switched back on during an error she opened calm, since her mood waited for the next change; on a 100-point window her fixed 140-point offset left her beside it. Fixed in 6df7534: on macOS her panel becomes key only if needed, and on Windows her window answers a press's activation with none; she opens with the sessions' mood; she stands 140 points in or less, so she holds the pane's edge at any width, and on Wayland she opens again when a new width moves her.
- Round 2, 6df7534: PASS. A native AppKit press on her kept the main window key and the program read abc; switched off and on during an error she came back worried; on a 100-point window she stood at its left edge, and 140 points in once it grew; minimized, both hid, and restored, both showed; check.sh, 8 Linux pet tests under Xvfb and the Windows cross-check passed, and on the Windows VM cargo test passed 200 tests at 6df7534. Native Windows interaction and a real Wayland desktop went unexercised.

## T47

- Round 1, 2d9180e and cdafdc2 with release v0.1.4: PASS. The reviewer compared Cargo.lock with its parent, where of 844 packages only elyterminal moved, to 0.1.4; found the release public, its tag and target at 2d9180e; downloaded the three files anonymously against the SHA-256 in the notes, and matched the notes to notes.md and to T44 to T46; read CI run 1215's three jobs, 228 tests on macOS, 234 on Ubuntu and 200 on Windows, done before the release was published, and Release run 1216's notarization; checked the disk image's stapled ticket and Gatekeeper, its app's Developer ID signature, hardened runtime, version 0.1.4, arm64 and macOS 11.0, the zip's icon against its source byte for byte, its manifest and its imports, no C runtime DLL among them, and the tarball's three files, their modes and glibc 2.35; and ran the downloaded app with a HOME of its own: it loaded its settings, started its shell, showed Ely, followed working, waiting, error and done, and quit with its last shell. check.sh passed.

## T48

- Round 1, c1cdddf: FAIL. With two displays of different DPI, her height came from the main window's DPI while her size came from her own, so her box fell 62 pixels into the terminal or stood above it. Fixed in 8ddf470: her height is her own DPI's and her bottom the pane's top; at a new DPI she kept her bottom left corner.
- Round 2, 8ddf470: FAIL. Between two displays side by side, a corner kept always set her DPI back and forth. Fixed in f7e5563: her corner follows once the move is done, posted, and only where her box would lie mostly on a display of that DPI; else Windows' rect stands.
- Round 3, f7e5563: FAIL. With two places piled up, a keep posted at a DPI no longer hers moved her to a display of it, and the keeps renewed each other. Fixed in 47826ee: a keep moves her only while its DPI is still hers. The three reviews spent, the owner allowed a fourth. Preparing it, e4b0107 fixed a place sized when it was posted, which left her the old DPI's size on the new display: a place now carries her corner, and her window sizes her when it lands.
- Round 4, 47826ee and e4b0107: PASS. The reviewer's replays drained round 3's queue, left no stale size, and across 1,000 crossings of side-by-side and stacked display borders at 96 to 192 DPI, 8,432 messages, each place brought at most one DPI change and each keep none; check.sh and the Windows cross-check passed. Native multi-display runs remain out of reach.

## T49

- Round 1, 9d9144f: FAIL. Only a change of act started her ease, so a mood that changed inside a wink dropped her 12 points mid-hop, an old finish cleared mid-glance snapped her eyes back, and a second press mid-hop dropped her too. Fixed in a9be8ab: a change of mood and a press each ease her from the pose she was in, as a change of act does; her brows ease with her.
- Round 2, a9be8ab: PASS. In the reviewer's replays a mood that changed inside a wink kept her lift at 40.3 and faded the wait's mark over 0.3 s, an old finish cleared mid-glance eased her look and tilt back, and a second press mid-hop kept her lift; 160 mixes of moods, times and winks passed; macOS shot replays of all three showed the ease; check.sh passed with 237 tests. Windows ran at 9d9144f, 212 tests, and drew her; the VM was offline for the fix, which changes no Windows code.
