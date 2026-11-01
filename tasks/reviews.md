# Reviews

Each task gets at most three codex rounds. Each round: the commit, the verdict, the findings and what was done.

T01 to T49 are in `reviews-t01-t49.md`, T50 to T136 in `reviews-t50-t136.md`.

## T137

- Round 1: FAIL. The folder argument's lines missed cmd's share limit, winget's line its catalog's merge, the guides Windows 11's Show more options, and the acceptance table still pinned Ely `494c17d`. Fixed.
- Round 2: PASS.

## T138

- v0.2.1 at 0f4696a, published and latest, twelve assets: CI passed on macOS, Windows and Linux; the Windows zip from `scripts/windows-zip.ps1` on the owner's PC after its 405 tests (sha256 a1c84372…); the four Linux packages from the Linux packages run 41159321599, checked against its SHA-256 file; the disk image from the Release run 41159369740, stapled and accepted by Gatekeeper as notarized Developer ID. The website's six version-less links resolve to it.
- `ELY-Open-Source/homebrew-tap` made; `brew install --cask ely-open-source/tap/elyterminal` installed 0.2.1, team 9PDD8M2SMS, and uninstalled. winget: `winget validate` passed; microsoft/winget-pkgs#466704, its CLA signed at the owner's word from the owner's account, waits on Microsoft's validation and moderators.
- Issues #40, #41, #44, #45, #46, #47 and #50 answered and closed; #42 answered and open until winget merges; PR #48 thanked.

## T139

- The final entry is a right press on the Files tab. An inline Collapsible pushes the tree down; blank-space handling was removed. Row menus stay unchanged. Tab controls keep RadioButton roles and selected state.
- Static review found two lifetime issues, both fixed: the native error prompt receiver must survive until answered, and keyboard focus must return to the terminal before the action row disappears and the chooser opens. A chooser result notifies the view to re-enable the action.
- Windows: formatting, clippy with warnings denied, tests, release build, diff whitespace and the 500-line/comment rules pass. The test runner needs the machine's standard OS value, Windows_NT; no product code was changed for the host's missing environment variable.
- The earlier blank-space test package started successfully with its bundled ConPTY. The updated tab entry and push-down animation await the user's interactive test, as requested; macOS/Linux build and interaction: not_run.
- Independent review, round 1: FAIL. A right press on the Files tab left the keys on the tab: gpui focuses a focusable element at any button's press, children before parents, so the wrapper's handler came too late; AGENTS.md named neither the new modules nor the decision. Fixed: both presses are stopped on the tab itself, and AGENTS.md holds the modules and the decision. The fix awaits the user's interactive test.
- The author's choice after round 1: the tabs go back to Ely's `SegmentedControl`, through `SegmentedControl::on_context`, added to Ely on its `elyterminal-0.1.1` branch with a test that fails when the handler sits on a wrapper; the pin moves once Ely merges it. Built and tested against that Ely locally: fmt, clippy, 407 tests and a release build pass. The author tried that build on Windows and found no problem.
- Independent review, round 2: PASS for the code, against Ely's local branch: Ely's clippy and 2156 tests and ELYTerminal's clippy and 407 tests pass; Ely's `cargo fmt --check` failed on the new test, fixed. Before a pull request: Ely's commit on GitHub and on its main, the pin and AGENTS.md's Stack moved to it, since the pinned `b0350ad` has no `on_context`; macOS and Linux not_run.
- Pin: `c2752ee`, ELY-Open-Source/Ely-GPUI-Components#10 to `elyterminal-0.1.1`, its main twin `f40838b` in ELY-Open-Source/Ely-GPUI-Components#11; `cargo build --locked` fetches it from the org's repository, and fmt, clippy, 407 tests and a release build pass on it. It moves again if Ely merges it under another commit.
