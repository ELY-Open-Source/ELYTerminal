# Reviews

Each task gets at most three codex rounds. Each round: the commit, the verdict, the findings and what was done.

T01 to T49 are in `reviews-t01-t49.md`, T50 to T99 in `reviews-t50-t99.md`, T100 to T138 in `reviews-t100-t138.md`.

## T139

- Round 1, 7704b03 with gpui-pre-macos b3e25d5: PASS. The old rev aborted the same shot with 134; ten opens and closes, Cmd-W, `performClose:` and the main window closed cleanly; accessibility queries during and after close answered, every closed view freed.

## T140

- Round 1, be0264d: FAIL. The cursor preview kept the normal weight, and AGENTS named the bundled JetBrains Mono Regular and Bold alone, where Ely bundles Medium too. Fixed in the next commit.
