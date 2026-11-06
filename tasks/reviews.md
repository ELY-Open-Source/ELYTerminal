# Reviews

Each task gets at most three codex rounds. Each round: the commit, the verdict, the findings and what was done.

T01 to T49 are in `reviews-t01-t49.md`, T50 to T99 in `reviews-t50-t99.md`, T100 to T138 in `reviews-t100-t138.md`.

## T139

- Round 1, 7704b03 with gpui-pre-macos b3e25d5: PASS. The old rev aborted the same shot with 134; ten opens and closes, Cmd-W, `performClose:` and the main window closed cleanly; accessibility queries during and after close answered, every closed view freed.

## T140

- Round 1, be0264d: FAIL. The cursor preview kept the normal weight, and AGENTS named the bundled JetBrains Mono Regular and Bold alone, where Ely bundles Medium too. Fixed in the next commit.
- Round 2, 900d408: FAIL. AGENTS gave Medium to 401 through 500, where font-kit draws 450 and 499 Regular. AGENTS names the faces at 400, 401, 450, 499 and 500 as measured.
- Round 3, 3283b88: PASS.

## T141

- Round 1, ad79521: FAIL. Fonts went per character, so a variation selector or a mark left its Han base and lost the variant; vertical and small forms and the kana supplements fell outside the ranges. Runs now choose a font per grapheme cluster; the ranges hold U+FE10 to FE1F, FE50 to FE6F, 16FE0 to 16FFF and 1AFF0 to 1B16F.
- Round 2, 66df88e: FAIL. A Nerd Font icon with a combining mark went whole to the icon font, which drew the icon as a missing glyph. Only a cluster that starts CJK goes whole; any other picks per character as before.
- Round 3, ce62f78: PASS.

## T142

- Round 1, 81df238 and 2c05590: PASS. A probe built on the real store and interface switched three cached windows and a new one; without the refresh the cached view stayed grayscale.

## T143

- Round 1, 65dbc86 and 2e9bfed: PASS. Thirteen probes on the glide functions under a driven clock; restoring the old target start, the early inversion or the second clock read each failed them.

## T144

- Round 1, 55552a0: FAIL. AGENTS said a drag from another tree types nothing; a probe tree whose key equals a listed path, and a drag that outlived a rebuild, typed it. A Files key is its path, one to one, so the path typed is the path the key names; the window holds no other `Tree`. AGENTS now says so.
- Round 2, 482b16a: PASS. The reviewer withdrew the runtime finding; a drag that outlived a rebuild typed its own node's path, a gone key typed nothing.

## T145

- Round 1, 573710c: PASS. 32 real launches, each flag, bad input refused with the usage; xdg-terminal-exec at 80d6ea7 passed the folder as `--working-directory DIR`. Three zsh resize tests failed once under load and passed on the final gate, as in T141.

## T146

- Round 1, the reply to #59 before posting: PASS. Each claim and label matched the code and both catalogs. Posted and closed as not planned.

## T147

- Round 1, 23a5ee0: FAIL. Add profile wrote the login shell's bare path, so `C:\Program Files\…` split at its space and was refused; an edit to one field saved it beside the other fields as saved, so a valid name cleared the refusal of a command still shown open. Add quotes the path through `command_line`; each edit saves the profile its three fields show.
- Round 2, de03bbd: PASS. Both repros fixed; a duplicate name, a missing program and folder each refused until mended; Duplicate kept the arguments.

## T148

- Round 1, abbdf17: FAIL. The acceptance row still named the pin `c2752ee` as unmerged and macOS as not run. It names `294a832` and the macOS checks.
- Round 2, 41f7cd7: PASS.

## T149

- Round 1, the comment before posting: FAIL. It said 33 files; the PR changes 28, +2,739 and -153. Fixed.
- Round 2: PASS. Posted; the PR stays open.

## T150

- Round 1, 5309400: FAIL. PRD said closing any window leaves the app running, where the last terminal window quits it; the guides left out bold's 900 cap, WSL's Docker Desktop exclusion and the Touch Bar fix. Each mended.
- Round 2, f04466a: PASS.

## T151

- Round 1, the notes: FAIL. WSL lacked Docker Desktop's exclusion, the 1/255 measure read as general, and New session here's failure dialog was left out. Each mended.
- Round 2, the notes: PASS. v0.2.2 published at a0550a6, twelve assets: CI passed on macOS, Windows and Linux; the Windows zip from `scripts/windows-zip.ps1` on the owner's PC after its 415 tests (sha256 d4267f60…); the four Linux packages from run 41524992041, checked against its SHA-256 file; the disk image from Release run 41526766309, stapled and accepted by Gatekeeper as notarized Developer ID, its app 0.2.2. The version-less links resolve to it. The tap holds 0.2.2, installed into a temp folder, team 9PDD8M2SMS, and removed; winget: `winget validate` passed, microsoft/winget-pkgs#469024 beside #466704.
- Round 3, the replies to #52, #55 and #60: FAIL. #60 promised the nearest face where font-kit matches its own way; #55 promised folder-following to any program alone. Both mended to the code, round 3 the last; posted, the three closed as completed.

## T152

Reviews from here on run in Claude Code on Kimi K3.1 Pro High, in a herdr pane.

- Round 1, Ely 601ad331: FAIL. No crate's version rose for the new gpui pin; the README and the Kit guide still named Kit 0.8; the ADR said never sleep where Ely's own transitions need it, and AGENTS named a glide that gpui drives; the dated acceptance table took a new pin. Each crate published at 0.1.1 is 0.2.0, ely-core 0.3.0; both docs name Kit 0.9; the table is back as checked. The 500-line finding was the brief's error: Ely warns past 800.
- Round 2, Ely 8ad88955: FAIL. The AGENTS list of transitions that read `Instant::now` left four out. The rule names no list.
- Round 3, Ely 105e2620: PASS.

## T153

- Round 1, 8292748 and the reply to #73: PASS. Posted; #73 stays open for zcg's theme.

## T154

- Round 1, the decision on PR #79 and its reply: PASS. The table, the units, Ghostty's `Metrics.zig` rule and gpui's private metrics checked. Posted on #79 and #78.

## T155

- Round 1, the reply to #66: FAIL. `typed` logs at trace, and a release build on Windows has no console. The log line names `elyterminal::term::input=trace` and redirects stderr through `Start-Process`, which a release build on the owner's PC proved.
- Round 2: PASS. Posted; #66 stays open.

## T156

- Round 1, the probe and the reply to #77: PASS. The reviewer ran the probes again and read Grok's own docs. Posted; #77 closed.

## T157

- Round 1, cae1833 and 92d8196: FAIL. The report remainder summed a touchpad's pixel lines with a wheel's notches. Only line deltas count as notches now.
- Round 2: PASS.

## T158

- Round 1, d21c4fa and the AGENTS line: PASS.

## T159

- Round 1, gpui-pre-windows 96cede5: FAIL. The cache started at 0,0, compared logical points across a DPI change, and missed non-client moves. It holds device pixels, unset until a client move, cleared on leave and on a non-client move.
- Round 2, c141eba: PASS.
- Round 3, the pin 0b74ab7 and the docs: PASS.

## T160

- Round 1, b0d10d9: PASS.

## T161

- Round 1, c9d21d2 and 038c3f7: PASS. The reviewer compared old and new Cmd-press over 1,049,760 cases within the budgets and found no difference.

## T162

- Round 1, the reply to #76: FAIL. It asked for `elyterminal --version`, which is no flag. It asks for the version Settings shows.
- Round 2: PASS. Posted; #76 stays open.

## T163

- Round 1, c6b8ca1 and 37445d8: FAIL. On Linux the picture lay in window coordinates inside ResizeBorder's margin. It lies in the box it paints in, measured.
- Round 2: PASS. Under Hyprland, floated, both scopes met the column borders and the pane's edges.

## T164

- Round 1, 3100fe8: FAIL. A covering picture escaped its box; one axis stopped at 400 tiles in silence; `center 10px` took the wrong axis and `left right` passed; negative offsets were refused; the guides lacked the file's repeat spellings and AGENTS gave percentages a range of 0 to 100. Each fixed.
- Round 2, 2189207: PASS.

## T165

- Round 1, the reply to #58: PASS. Posted; #58 stays open.

## T166

- Round 1, the reply to #64: PASS. Posted; #64 closed.

## T167

- Round 1, 231527f and dcd1280: PASS. Pushed before the review, against the order; CI run 42126749944 green on all four jobs; the artifact held AArch64 binaries.

## T168

- Round 1, e2f80d3: FAIL. The settings count and tasks were stale, the picture row cited T109 alone, a capture claim named no run, the T159 row sat in section 10, and Right click pastes had no settings row. Each mended.
- Round 2, 31f22a7: FAIL. The settings row lacked T125 and T134; a trace of each field also found T67 and T73.
- Round 3, ee20cfc: PASS.

## T169

- Round 1, bf10492 and 42926fb: PASS. The ceil holds the one-pixel floor; the tests fail on the old rounding; `cargo test --lib fonts` passed.

## T170

- v0.2.3 published at c82e206: CI passed on macOS, Linux, Windows and Windows ARM64 (run 42147003020); the Windows x86_64 zip from `scripts/windows-zip.ps1` on the owner's PC after its 447 tests (sha256 ad87763d…); the ARM64 zip from CI; the four Linux packages from run 42147003019, checked against its SHA-256 file; the disk image from Release run 42148683456, stapled and accepted by Gatekeeper as notarized Developer ID, its app 0.2.3. The version-less links resolve to it. #78 and #79 answered and closed; the tap holds 0.2.3; winget: microsoft/winget-pkgs#471905.
