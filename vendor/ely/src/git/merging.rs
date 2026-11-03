use similar::{DiffableStr, MergeResolution, TextMerge};

/// Which side a conflict takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Take {
    Ours,
    Theirs,
    Both,
}

/// How a merge region stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionKind {
    Unchanged,
    Ours,
    Theirs,
    /// Both sides made the same change.
    Both,
    Conflict,
}

/// A stretch of a three-way merge, with each side's lines.
#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    pub kind: RegionKind,
    pub base: Vec<String>,
    pub ours: Vec<String>,
    pub theirs: Vec<String>,
}

/// A region's lines as split, each with its ending.
fn lines<'a>(
    range: std::ops::Range<usize>,
    line: impl Fn(usize) -> Option<&'a str>,
) -> Vec<String> {
    range
        .map(|ix| {
            line(ix)
                .expect("a region's lines are in its text")
                .to_string()
        })
        .collect()
}

/// The texts' line ending: the first found, else newline.
fn ending(regions: &[Region]) -> &'static str {
    let line = regions
        .iter()
        .flat_map(|region| region.base.iter().chain(&region.ours).chain(&region.theirs))
        .find(|line| line.as_str().ends_with_newline());
    match line {
        Some(line) if line.ends_with("\r\n") => "\r\n",
        Some(line) if line.ends_with('\r') => "\r",
        _ => "\n",
    }
}

/// Ends an unended last line so what follows starts fresh.
fn break_line(out: &mut String, ending: &str) {
    if !out.is_empty() && !out.as_str().ends_with_newline() {
        out.push_str(ending);
    }
}

/// `ours` and `theirs` merged against `base`, region by region.
pub fn regions(base: &str, ours: &str, theirs: &str) -> Vec<Region> {
    let merge = TextMerge::from_lines(base, ours, theirs);
    merge
        .regions()
        .iter()
        .map(|region| Region {
            kind: match region.resolution() {
                MergeResolution::Unchanged => RegionKind::Unchanged,
                MergeResolution::Ours => RegionKind::Ours,
                MergeResolution::Theirs => RegionKind::Theirs,
                MergeResolution::Both => RegionKind::Both,
                MergeResolution::Conflict => RegionKind::Conflict,
                other => unreachable!("a merge resolution this crate does not know: {other:?}"),
            },
            base: lines(region.base_range(), |ix| merge.base_line(ix)),
            ours: lines(region.ours_range(), |ix| merge.ours_line(ix)),
            theirs: lines(region.theirs_range(), |ix| merge.theirs_line(ix)),
        })
        .collect()
}

/// The merged text as taken, and how many stay open.
pub fn result(regions: &[Region], takes: &[Option<Take>]) -> (String, usize) {
    let ending = ending(regions);
    let mut out = String::new();
    let mut conflicts = 0;
    let mut open = 0;
    for region in regions {
        let (ours, theirs) = (region.ours.concat(), region.theirs.concat());
        match region.kind {
            RegionKind::Unchanged | RegionKind::Ours | RegionKind::Both => out.push_str(&ours),
            RegionKind::Theirs => out.push_str(&theirs),
            RegionKind::Conflict => {
                match takes.get(conflicts).copied().flatten() {
                    Some(Take::Ours) => out.push_str(&ours),
                    Some(Take::Theirs) => out.push_str(&theirs),
                    Some(Take::Both) => {
                        out.push_str(&ours);
                        if !theirs.is_empty() {
                            break_line(&mut out, ending);
                        }
                        out.push_str(&theirs);
                    }
                    None => {
                        open += 1;
                        for (marker, side) in [("<<<<<<< ours", &ours), ("=======", &theirs)] {
                            break_line(&mut out, ending);
                            out.push_str(marker);
                            out.push_str(ending);
                            out.push_str(side);
                        }
                        break_line(&mut out, ending);
                        out.push_str(">>>>>>> theirs");
                        out.push_str(ending);
                    }
                }
                conflicts += 1;
            }
        }
    }
    (out, open)
}

/// A marked conflict: its marker lines, from zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Conflict {
    pub start: usize,
    /// A diff3 conflict's `|||||||` line, before its base.
    pub base: Option<usize>,
    pub middle: usize,
    pub end: usize,
}

/// Git's conflict marker width, unless `conflict-marker-size` says otherwise.
pub const MARKER: usize = 7;

/// A marker line: exactly `size` of `mark`, maybe labelled.
fn marker(line: &str, mark: char, size: usize, labelled: bool) -> bool {
    let line = line.trim_end_matches(['\n', '\r']);
    let run = line.chars().take_while(|c| *c == mark).count();
    run == size && (line.len() == size || (labelled && line[size..].starts_with(' ')))
}

/// Lines as Git splits them: at LF only.
fn git_lines(text: &str) -> Vec<&str> {
    text.split_inclusive('\n').collect()
}

/// The conflicts marked in `text` with markers `size` wide.
pub fn conflicts(text: &str, size: usize) -> Vec<Conflict> {
    let mut found = Vec::new();
    let (mut start, mut base, mut middle) = (None, None, None);
    for (ix, line) in git_lines(text).into_iter().enumerate() {
        if marker(line, '<', size, true) {
            (start, base, middle) = (Some(ix), None, None);
        } else if marker(line, '|', size, true) && start.is_some() && middle.is_none() {
            base = Some(ix);
        } else if marker(line, '=', size, false) && start.is_some() && middle.is_none() {
            middle = Some(ix);
        } else if marker(line, '>', size, true)
            && let (Some(start), Some(middle)) = (start.take(), middle.take())
        {
            found.push(Conflict {
                start,
                base: base.take(),
                middle,
                end: ix,
            });
        }
    }
    found
}

/// Regions as Git grouped marked `text`; base from diff3.
pub fn marked(text: &str, size: usize) -> Vec<Region> {
    let lines: Vec<String> = git_lines(text).into_iter().map(str::to_string).collect();
    let same = |range: std::ops::Range<usize>| Region {
        kind: RegionKind::Unchanged,
        base: lines[range.clone()].to_vec(),
        ours: lines[range.clone()].to_vec(),
        theirs: lines[range].to_vec(),
    };
    let mut out = Vec::new();
    let mut at = 0;
    for conflict in conflicts(text, size) {
        if conflict.start > at {
            out.push(same(at..conflict.start));
        }
        let ours_end = conflict.base.unwrap_or(conflict.middle);
        out.push(Region {
            kind: RegionKind::Conflict,
            ours: lines[conflict.start + 1..ours_end].to_vec(),
            base: conflict
                .base
                .map_or_else(Vec::new, |base| lines[base + 1..conflict.middle].to_vec()),
            theirs: lines[conflict.middle + 1..conflict.end].to_vec(),
        });
        at = conflict.end + 1;
    }
    if at < lines.len() {
        out.push(same(at..lines.len()));
    }
    out
}

/// `text` with conflict `ix` settled; none if absent.
pub fn resolve(text: &str, ix: usize, take: Take, size: usize) -> Option<String> {
    let Some(conflict) = conflicts(text, size).get(ix).copied() else {
        log::error!("no conflict {ix} to resolve in this text");
        return None;
    };
    let lines = git_lines(text);
    // A diff3 conflict's base section is neither side.
    let ours = lines[conflict.start + 1..conflict.base.unwrap_or(conflict.middle)].concat();
    let theirs = lines[conflict.middle + 1..conflict.end].concat();
    let kept = match take {
        Take::Ours => ours,
        Take::Theirs => theirs,
        Take::Both => ours + &theirs,
    };
    log::info!("conflict {ix} took {take:?}");
    Some(
        [
            lines[..conflict.start].concat(),
            kept,
            lines[conflict.end + 1..].concat(),
        ]
        .concat(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_hold_their_width_and_whole_lines() {
        let wide = "a\n<<<<<<<<<< ours\nmine\n==========\ntop\n======= literal\nend\n>>>>>>>>>> theirs\nz\n";
        assert!(conflicts(wide, MARKER).is_empty(), "seven is not ten");
        assert_eq!(
            resolve(wide, 0, Take::Theirs, 10).as_deref(),
            Some("a\ntop\n======= literal\nend\nz\n")
        );
        let five = "<<<<< ours\nx\n=====\ny\n>>>>> theirs\n";
        assert_eq!(conflicts(five, 5).len(), 1);
    }

    #[test]
    fn marked_text_keeps_gits_grouping_and_its_base() {
        let text = "first\n<<<<<<< ours\nOURS a\nunchanged\nOURS b\n||||||| base\na\nunchanged\nb\n=======\nTHEIRS a\nunchanged\nTHEIRS b\n>>>>>>> theirs\nlast\n";
        let found = marked(text, MARKER);
        let kinds: Vec<RegionKind> = found.iter().map(|region| region.kind).collect();
        assert_eq!(
            kinds,
            [
                RegionKind::Unchanged,
                RegionKind::Conflict,
                RegionKind::Unchanged
            ],
            "one block, as Git wrote it"
        );
        assert_eq!(found[1].base, ["a\n", "unchanged\n", "b\n"]);
        assert_eq!(found[1].ours.len(), 3);
        assert_eq!(found[2].ours, ["last\n"]);
    }

    #[test]
    fn a_diff3_conflict_leaves_its_base_out() {
        let text = "a\n<<<<<<< HEAD\nours\n||||||| base\nold\n=======\ntheirs\n>>>>>>> side\nz\n";
        assert_eq!(conflicts(text, MARKER)[0].base, Some(3));
        assert_eq!(
            resolve(text, 0, Take::Ours, MARKER).as_deref(),
            Some("a\nours\nz\n")
        );
        assert_eq!(
            resolve(text, 0, Take::Both, MARKER).as_deref(),
            Some("a\nours\ntheirs\nz\n")
        );
    }

    const BASE: &str = "fn lift() {\n    0.5\n}\nfn keep() {}\n";
    const OURS: &str = "fn lift() {\n    0.6\n}\nfn keep() {}\n";
    const THEIRS: &str = "fn lift() {\n    0.75\n}\nfn keep() { log() }\n";

    #[test]
    fn a_merge_keeps_clean_changes_and_holds_conflicts() {
        let all = regions(BASE, OURS, THEIRS);
        let kinds: Vec<RegionKind> = all.iter().map(|region| region.kind).collect();
        assert!(kinds.contains(&RegionKind::Conflict) && kinds.contains(&RegionKind::Theirs));
        let (open, count) = result(&all, &[]);
        assert_eq!(count, 1);
        assert!(open.contains("<<<<<<< ours\n    0.6\n=======\n    0.75\n>>>>>>> theirs"));
        let (taken, count) = result(&all, &[Some(Take::Theirs)]);
        assert_eq!(count, 0);
        assert_eq!(taken, "fn lift() {\n    0.75\n}\nfn keep() { log() }\n");
    }

    #[test]
    fn merges_keep_line_endings_and_the_last_newline() {
        assert_eq!(result(&regions("x", "x", "x"), &[]), ("x".into(), 0));
        let crlf = regions("a\r\nb\r\n", "a\r\nB\r\n", "a\r\nb\r\n");
        assert_eq!(result(&crlf, &[]).0, "a\r\nB\r\n");
        let held = regions("a\r\n", "b\r\n", "c\r\n");
        assert_eq!(
            result(&held, &[]).0,
            "<<<<<<< ours\r\nb\r\n=======\r\nc\r\n>>>>>>> theirs\r\n"
        );
        let bare = regions("a", "b", "c");
        assert_eq!(result(&bare, &[Some(Take::Both)]).0, "b\nc");
        let marked = "a\r\n<<<<<<< HEAD\r\nours\r\n=======\r\ntheirs\r\n>>>>>>> topic\r\nz";
        assert_eq!(
            resolve(marked, 0, Take::Ours, MARKER).as_deref(),
            Some("a\r\nours\r\nz")
        );
    }

    #[test]
    fn a_bare_return_ends_a_line_as_it_does_for_similar() {
        let changed = regions("a\rb\n", "a\rb\n", "a\rB\n");
        assert_eq!(result(&changed, &[]).0, "a\rB\n");
        let held = result(&regions("a\r", "b\r", "c\r"), &[]).0;
        assert_eq!(held, "<<<<<<< ours\rb\r=======\rc\r>>>>>>> theirs\r");
        assert_eq!(
            resolve("plain\n", 0, Take::Ours, MARKER),
            None,
            "no conflict left"
        );
    }

    #[test]
    fn git_markers_split_at_lf_only() {
        let text = "<<<<<<< HEAD\nprefix\r=======\rOURS_END\n=======\nTHEIRS\n>>>>>>> side\n";
        assert_eq!(
            resolve(text, 0, Take::Theirs, MARKER).as_deref(),
            Some("THEIRS\n")
        );
        assert_eq!(
            marked(text, MARKER)[0].ours,
            ["prefix\r=======\rOURS_END\n"]
        );
        assert!(conflicts("<<<<<<< a\rb\r=======\rc\r>>>>>>> d\r", MARKER).is_empty());
    }

    #[test]
    fn taking_both_adds_no_line_to_an_empty_side() {
        assert_eq!(result(&regions("a", "b", ""), &[Some(Take::Both)]).0, "b");
        assert_eq!(result(&regions("a", "", "c"), &[Some(Take::Both)]).0, "c");
    }

    #[test]
    fn markers_resolve_to_one_side_or_both() {
        let marked = "a\n<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> topic\nz\n";
        assert_eq!(
            conflicts(marked, MARKER),
            [Conflict {
                start: 1,
                base: None,
                middle: 3,
                end: 5
            }]
        );
        assert_eq!(
            resolve(marked, 0, Take::Ours, MARKER).as_deref(),
            Some("a\nours\nz\n")
        );
        assert_eq!(
            resolve(marked, 0, Take::Both, MARKER).as_deref(),
            Some("a\nours\ntheirs\nz\n")
        );
        assert!(
            conflicts("a\n=======\nb\n", MARKER).is_empty(),
            "a rule alone is no conflict"
        );
    }
}
