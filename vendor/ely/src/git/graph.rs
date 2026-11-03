/// Which part of a row a graph line crosses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Half {
    /// From the row's top into its commit.
    Top,
    /// From the commit down to the row's bottom.
    Bottom,
    /// Past the commit, top to bottom.
    Through,
}

/// A line of the graph in one row, from a lane to a lane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stroke {
    pub from: usize,
    pub to: usize,
    pub half: Half,
}

/// A row of the graph: its commit's lane, the strokes through it, and how many lanes it spans.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphRow {
    pub lane: usize,
    pub strokes: Vec<Stroke>,
    pub width: usize,
}

/// Lanes for commits given newest first, each with its parents' ids, first parent first. A commit takes the lane that waits for it, else the first free one; its first parent carries on in its lane, other parents take a free one, and a parent some lane already waits for joins that lane.
pub fn lanes<'a>(commits: impl IntoIterator<Item = (&'a str, &'a [&'a str])>) -> Vec<GraphRow> {
    let mut waiting: Vec<Option<&str>> = Vec::new();
    let mut rows = Vec::new();
    for (id, parents) in commits {
        let before = waiting.clone();
        let lane = before
            .iter()
            .position(|wait| *wait == Some(id))
            .or_else(|| waiting.iter().position(Option::is_none))
            .unwrap_or_else(|| {
                waiting.push(None);
                waiting.len() - 1
            });
        for wait in waiting.iter_mut().filter(|wait| **wait == Some(id)) {
            *wait = None;
        }
        let mut strokes: Vec<Stroke> = before
            .iter()
            .enumerate()
            .filter_map(|(at, wait)| {
                let wait = (*wait)?;
                Some(if wait == id {
                    Stroke {
                        from: at,
                        to: lane,
                        half: Half::Top,
                    }
                } else {
                    Stroke {
                        from: at,
                        to: at,
                        half: Half::Through,
                    }
                })
            })
            .collect();
        for (ix, parent) in parents.iter().enumerate() {
            let target = match waiting.iter().position(|wait| *wait == Some(*parent)) {
                Some(joined) => joined,
                None if ix == 0 => {
                    waiting[lane] = Some(parent);
                    lane
                }
                None => {
                    let free = waiting.iter().position(Option::is_none).unwrap_or_else(|| {
                        waiting.push(None);
                        waiting.len() - 1
                    });
                    waiting[free] = Some(parent);
                    free
                }
            };
            strokes.push(Stroke {
                from: lane,
                to: target,
                half: Half::Bottom,
            });
        }
        let width = before.len().max(waiting.len()).max(lane + 1);
        while waiting.last() == Some(&None) {
            waiting.pop();
        }
        rows.push(GraphRow {
            lane,
            strokes,
            width,
        });
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stroke(from: usize, to: usize, half: Half) -> Stroke {
        Stroke { from, to, half }
    }

    #[test]
    fn a_line_of_commits_keeps_one_lane() {
        let rows = lanes([("c", &["b"][..]), ("b", &["a"][..]), ("a", &[][..])]);
        assert!(rows.iter().all(|row| row.lane == 0 && row.width == 1));
        assert_eq!(
            rows[1].strokes,
            [stroke(0, 0, Half::Top), stroke(0, 0, Half::Bottom)]
        );
    }

    #[test]
    fn a_merge_opens_a_lane_and_the_branch_joins_back() {
        use Half::*;
        let rows = lanes([
            ("m", &["a", "b"][..]),
            ("a", &["c"][..]),
            ("b", &["c"][..]),
            ("c", &[][..]),
        ]);
        let lanes: Vec<usize> = rows.iter().map(|row| row.lane).collect();
        assert_eq!(lanes, [0, 0, 1, 0]);
        assert_eq!(
            rows[0].strokes,
            [stroke(0, 0, Bottom), stroke(0, 1, Bottom)]
        );
        assert_eq!(
            rows[1].strokes,
            [
                stroke(0, 0, Top),
                stroke(1, 1, Through),
                stroke(0, 0, Bottom)
            ]
        );
        assert_eq!(
            rows[2].strokes,
            [
                stroke(0, 0, Through),
                stroke(1, 1, Top),
                stroke(1, 0, Bottom)
            ]
        );
        assert_eq!(rows[3].strokes, [stroke(0, 0, Top)]);
        assert_eq!(rows[2].width, 2);
    }
}
