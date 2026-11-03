use gpui::{
    Context, IntoElement, Modifiers, ParentElement, Render, Styled, TestAppContext, Window, div,
    point, px,
};

use super::{settle, setup, then};
use crate::{
    files::{
        FileExplorer, FileGrid, FileOperation, FileOperationProgress, FileView, TransferState,
    },
    lists::DirEntry,
};

/// A path its folders no longer hold, gone picks, and bytes past their total.
struct Lagging;

impl Render for Lagging {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let root = vec![DirEntry::file("Projects", 10, then())];
        let here = vec![DirEntry::file("Brief.md", 10, then())];
        div()
            .w(px(640.0))
            .h(px(480.0))
            .child(
                FileExplorer::new(
                    "explorer",
                    ["Home", "Projects"],
                    [root, here.clone()],
                    FileView::List,
                )
                .selected("Ghost.md"),
            )
            .child(FileGrid::new("grid", here).selected("Ghost.md"))
            .child(FileOperationProgress::new(
                "copy",
                FileOperation::Copy,
                3,
                "Backup",
                (12, 10),
                TransferState::Moving { rate: 4 },
            ))
    }
}

#[gpui::test]
fn stale_paths_picks_and_overruns_draw(cx: &mut TestAppContext) {
    setup(cx);
    let (_, cx) = cx.add_window_view(|_, _| Lagging);
    settle(cx);
}

/// Columns down a path whose folder became a file of the same name.
struct Replaced;

impl Render for Replaced {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let root = vec![DirEntry::file("Projects", 10, then())];
        let here = vec![DirEntry::file("Brief.md", 10, then())];
        div().w(px(640.0)).h(px(480.0)).child(FileExplorer::new(
            "explorer",
            ["Home", "Projects"],
            [root, here],
            FileView::Columns,
        ))
    }
}

#[gpui::test]
fn a_file_where_a_path_folder_stood_is_not_marked(cx: &mut TestAppContext) {
    setup(cx);
    let (_, cx) = cx.add_window_view(|_, _| Replaced);
    settle(cx);
    assert!(cx.debug_bounds("item-selected-Projects").is_none());
}

/// A list the owner may point at an entry the folder lost.
struct Listing {
    selected: Option<&'static str>,
}

impl Render for Listing {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let here = vec![
            DirEntry::file("Brief.md", 10, then()),
            DirEntry::file("Notes.md", 10, then()),
        ];
        let explorer = FileExplorer::new("explorer", ["Home"], [here], FileView::List);
        div().w(px(640.0)).h(px(480.0)).child(match self.selected {
            Some(name) => explorer.selected(name),
            None => explorer,
        })
    }
}

#[gpui::test]
fn an_owner_pick_gone_from_the_list_replaces_the_local_one(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Listing { selected: None });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    let header = cx.debug_bounds("listing-name").expect("the name header");
    let first = point(header.center().x, header.bottom() + px(12.0));
    cx.simulate_click(first, Modifiers::none());
    settle(cx);
    assert!(
        cx.debug_bounds("item-selected-Brief.md").is_some(),
        "the press picks it"
    );
    view.update(cx, |view, cx| {
        view.selected = Some("Ghost.md");
        cx.notify();
    });
    settle(cx);
    assert!(cx.debug_bounds("item-selected-Brief.md").is_none());
}
