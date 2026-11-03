mod badges;
mod blame;
mod changes;
mod commits;
mod diff;
mod graph;
mod history;
mod merge;
mod merging;
mod refs;
mod review;
mod rows;
mod tags;
#[cfg(all(test, feature = "test-support"))]
mod tests;
mod viewer;

pub use badges::{DiffStat, GitStatusBadge};
pub use blame::{Blame, BlameView, GitBlameAnnotation};
pub use changes::{ChangeAction, ChangeSection, Changed, ChangesList, CommitInput};
pub use commits::{Commit, CommitItem, CommitList};
pub use diff::{DiffLine, LineKind, Stretch, diff, mark_words, pairs, stat};
pub use graph::{GraphRow, Half, Stroke, lanes};
pub use history::FileHistory;
pub use merge::{ConflictResolver, ThreeWayMerge};
pub use merging::{
    Conflict, MARKER, Region, RegionKind, Take, conflicts, marked, regions, resolve, result,
};
pub use refs::{Branch, BranchList, BranchSelector};
pub use review::{PullRequest, PullRequestCard, PullState, ReviewComment, ReviewNote};
pub use rows::Place;
pub use tags::{GitTag, Stash, StashAction, StashList, TagList};
pub use viewer::{DiffLayout, DiffViewer};
