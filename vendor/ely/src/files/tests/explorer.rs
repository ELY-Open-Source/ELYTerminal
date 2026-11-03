use gpui::{
    Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled, TestAppContext,
    VisualTestContext, Window, div, px,
};

use super::{press, settle, setup, then};
use crate::{
    files::{FileExplorer, FileView},
    lists::DirEntry,
};

/// An explorer two folders deep, and what it asked for.
struct Exploring {
    view: FileView,
    selected: Option<&'static str>,
    went: Vec<Vec<SharedString>>,
    picked: Vec<SharedString>,
    opened: Vec<SharedString>,
}

impl Render for Exploring {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (view, go, pick, open) = (cx.entity(), cx.entity(), cx.entity(), cx.entity());
        let root = vec![
            DirEntry::folder("Projects", then()),
            DirEntry::folder("Documents", then()),
            DirEntry::file("Notes.txt", 10, then()),
        ];
        let here = vec![
            DirEntry::file("Brief.md", 10, then()),
            DirEntry::folder("Atrium", then()),
        ];
        let explorer = FileExplorer::new("explorer", ["Home", "Projects"], [root, here], self.view)
            .on_view(move |to, _, cx| {
                view.update(cx, |view, cx| {
                    view.view = to;
                    cx.notify();
                })
            })
            .on_navigate(move |path, _, cx| go.update(cx, |view, _| view.went.push(path.to_vec())))
            .on_select(move |name, _, cx| pick.update(cx, |view, _| view.picked.push(name.clone())))
            .on_open(move |entry, _, cx| {
                open.update(cx, |view, _| view.opened.push(entry.name().clone()))
            });
        let explorer = match self.selected {
            Some(name) => explorer.selected(name),
            None => explorer,
        };
        div().w(px(640.0)).child(explorer)
    }
}

fn exploring<'a>(
    view: FileView,
    selected: Option<&'static str>,
    cx: &'a mut TestAppContext,
) -> (Entity<Exploring>, &'a mut VisualTestContext) {
    setup(cx);
    let (host, cx) = cx.add_window_view(move |_, _| Exploring {
        view,
        selected,
        went: Vec::new(),
        picked: Vec::new(),
        opened: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (host, cx)
}

/// Moves focus to the `nth` Tab stop from none: the three views, the path's first level, then the view's own.
fn tab_to(nth: usize, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.blur(cx);
        for _ in 0..nth {
            window.focus_next(cx);
        }
    });
    settle(cx);
}

fn went(host: &Entity<Exploring>, cx: &mut VisualTestContext) -> Vec<Vec<SharedString>> {
    host.read_with(cx, |host, _| host.went.clone())
}

#[gpui::test]
fn a_segment_switches_the_view(cx: &mut TestAppContext) {
    let (host, cx) = exploring(FileView::List, None, cx);
    tab_to(2, cx);
    press("space", cx);
    assert_eq!(host.read_with(cx, |host, _| host.view), FileView::Icons);
    assert!(cx.debug_bounds("file-grid").is_some());
}

#[gpui::test]
fn a_folder_opens_in_place_and_a_file_goes_to_the_host(cx: &mut TestAppContext) {
    let (host, cx) = exploring(FileView::Icons, None, cx);
    tab_to(5, cx);
    press("right", cx);
    press("enter", cx);
    assert_eq!(
        went(&host, cx),
        [["Home", "Projects", "Atrium"]],
        "folders come first"
    );
    press("right", cx);
    press("enter", cx);
    assert_eq!(
        host.read_with(cx, |host, _| host.opened.clone()),
        ["Brief.md"]
    );
}

#[gpui::test]
fn the_path_climbs(cx: &mut TestAppContext) {
    let (host, cx) = exploring(FileView::Icons, None, cx);
    tab_to(4, cx);
    press("enter", cx);
    assert_eq!(went(&host, cx), [["Home"]]);
}

#[gpui::test]
fn columns_walk_the_path_and_pick_in_the_last(cx: &mut TestAppContext) {
    let (host, cx) = exploring(FileView::Columns, None, cx);
    assert!(cx.debug_bounds("file-columns").is_some());
    tab_to(5, cx);
    press("up", cx);
    assert_eq!(
        went(&host, cx),
        [["Home", "Documents"]],
        "the lit folder was Projects"
    );
    tab_to(6, cx);
    press("down", cx);
    press("enter", cx);
    let heard = host.read_with(cx, |host, _| (host.picked.clone(), host.opened.clone()));
    assert_eq!(heard, (vec!["Brief.md".into()], vec!["Brief.md".into()]));
}

#[gpui::test]
fn a_file_picked_in_an_earlier_column_closes_the_columns_past_it(cx: &mut TestAppContext) {
    let (host, cx) = exploring(FileView::Columns, None, cx);
    tab_to(5, cx);
    press("down", cx);
    let heard = host.read_with(cx, |host, _| (host.went.clone(), host.picked.clone()));
    assert_eq!(heard, (vec![vec!["Home".into()]], vec!["Notes.txt".into()]));
}

#[gpui::test]
fn the_pick_carries_into_the_list(cx: &mut TestAppContext) {
    let (host, cx) = exploring(FileView::List, Some("Brief.md"), cx);
    tab_to(5, cx);
    press("up", cx);
    assert_eq!(
        host.read_with(cx, |host, _| host.picked.clone()),
        ["Atrium"]
    );
}

#[test]
#[should_panic(expected = "2 folders on the path, 1 listed")]
fn each_folder_on_the_path_is_listed() {
    let _ = FileExplorer::new(
        "explorer",
        ["Home", "Projects"],
        [Vec::new()],
        FileView::List,
    );
}

/// An explorer that goes where it is sent, over a small tree, and the files it opened.
struct Walking {
    path: Vec<SharedString>,
    opened: Vec<SharedString>,
}

fn folder(path: &[SharedString]) -> Vec<DirEntry> {
    let names: Vec<&str> = path.iter().map(|level| level.as_ref()).collect();
    match names.join("/").as_str() {
        "Home" => vec![
            DirEntry::folder("Projects", then()),
            DirEntry::folder("Documents", then()),
        ],
        "Home/Projects" => vec![
            DirEntry::folder("Atrium", then()),
            DirEntry::folder("Dunes", then()),
        ],
        "Home/Projects/Dunes" => vec![DirEntry::file("Survey.csv", 10, then())],
        "Home/Documents" => vec![
            DirEntry::file("Contract.docx", 10, then()),
            DirEntry::file("Invoice.pdf", 10, then()),
        ],
        other => panic!("no folder {other}"),
    }
}

impl Render for Walking {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (go, open) = (cx.entity(), cx.entity());
        let levels = (1..=self.path.len()).map(|depth| folder(&self.path[..depth]));
        let explorer = FileExplorer::new("explorer", self.path.clone(), levels, FileView::Columns)
            .on_navigate(move |path, _, cx| {
                go.update(cx, |host, cx| {
                    host.path = path.to_vec();
                    cx.notify();
                })
            })
            .on_open(move |entry, _, cx| {
                open.update(cx, |host, _| host.opened.push(entry.name().clone()))
            });
        div().w(px(640.0)).child(explorer)
    }
}

#[gpui::test]
fn a_new_folder_at_a_depth_starts_its_column_fresh(cx: &mut TestAppContext) {
    setup(cx);
    let (host, cx) = cx.add_window_view(|_, _| Walking {
        path: ["Home", "Projects"].map(SharedString::from).to_vec(),
        opened: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    tab_to(6, cx);
    press("down", cx);
    host.update(cx, |host, cx| {
        host.path = ["Home", "Documents"].map(SharedString::from).to_vec();
        cx.notify();
    });
    settle(cx);
    tab_to(6, cx);
    press("enter", cx);
    assert_eq!(
        host.read_with(cx, |host, _| host.opened.clone()),
        ["Contract.docx"]
    );
}
