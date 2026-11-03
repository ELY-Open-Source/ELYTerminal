use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::*,
};

use super::{FileGrid, OnEntry, columns::columns};
use crate::{
    buttons::SegmentedControl,
    forms::OnValue,
    lists::{DirEntry, DirectoryListing, OnClimb, listed, path_bar},
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, TextSize},
    typography::Ellipsis,
};

pub(super) type OnPath = Rc<dyn Fn(&[SharedString], &mut Window, &mut App)>;
type OnView = Rc<dyn Fn(FileView, &mut Window, &mut App)>;

/// How an explorer shows a folder: a list that sorts by column, tiles, or columns along the path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileView {
    List,
    Icons,
    Columns,
}

/// Each view with its key, label and icon, in the switch's order.
const VIEWS: [(FileView, &str, &str, IconName); 3] = [
    (FileView::List, "list", "List", IconName::List),
    (FileView::Icons, "icons", "Icons", IconName::LayoutGrid),
    (FileView::Columns, "columns", "Columns", IconName::Columns3),
];

/// A folder three ways: a list that sorts by column, tiles, or columns that walk the path. Its name, its count and a switch of views sit above the path, which climbs. Opening a folder goes into it; opening a file hands it to the host, which lists each folder it goes to.
#[derive(IntoElement)]
pub struct FileExplorer {
    id: ElementId,
    path: Vec<SharedString>,
    levels: Vec<Vec<DirEntry>>,
    view: FileView,
    selected: Option<SharedString>,
    on_view: Option<OnView>,
    on_navigate: Option<OnPath>,
    on_select: Option<OnValue>,
    on_open: Option<OnEntry>,
}

impl FileExplorer {
    /// `path` names each folder from the root down; `levels` lists each of them, the root's first.
    pub fn new(
        id: impl Into<ElementId>,
        path: impl IntoIterator<Item = impl Into<SharedString>>,
        levels: impl IntoIterator<Item = Vec<DirEntry>>,
        view: FileView,
    ) -> Self {
        let path: Vec<SharedString> = path.into_iter().map(Into::into).collect();
        let levels: Vec<Vec<DirEntry>> = levels.into_iter().collect();
        assert!(
            !path.is_empty() && path.len() == levels.len(),
            "{} folders on the path, {} listed",
            path.len(),
            levels.len()
        );
        for (depth, name) in path.iter().enumerate().skip(1) {
            let listed = levels[depth - 1]
                .iter()
                .any(|entry| entry.is_folder() && entry.name() == name);
            if !listed {
                log::error!("file explorer: {name} is no folder in {}", path[depth - 1]);
            }
        }
        Self {
            id: id.into(),
            path,
            levels,
            view,
            selected: None,
            on_view: None,
            on_navigate: None,
            on_select: None,
            on_open: None,
        }
    }

    /// The entry picked in the folder shown, by name.
    pub fn selected(mut self, name: impl Into<SharedString>) -> Self {
        let name = name.into();
        let here = self.levels.last().expect("a listed folder");
        if !here.iter().any(|entry| *entry.name() == name) {
            log::error!("file explorer: no entry {name}; none selected");
        }
        self.selected = Some(name);
        self
    }

    pub fn on_view(mut self, handler: impl Fn(FileView, &mut Window, &mut App) + 'static) -> Self {
        self.on_view = Some(Rc::new(handler));
        self
    }

    /// Gets the path to go to, from the root down; the pick stays with the folder left, so the host drops it.
    pub fn on_navigate(
        mut self,
        handler: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_navigate = Some(Rc::new(handler));
        self
    }

    /// Gets the name of the entry picked.
    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Gets a file to open; a folder opens in place.
    pub fn on_open(mut self, handler: impl Fn(&DirEntry, &mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for FileExplorer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let path: Rc<[SharedString]> = self.path.into();
        let go: OnPath = {
            let (id, on_navigate) = (id.clone(), self.on_navigate);
            Rc::new(move |to, window, cx| {
                log::info!("file explorer {id:?}: go to {}", to.join("/"));
                if let Some(on_navigate) = &on_navigate {
                    on_navigate(to, window, cx);
                }
            })
        };
        let climb: OnClimb = {
            let (go, path) = (go.clone(), path.clone());
            Rc::new(move |level, window, cx| go(&path[..=level], window, cx))
        };
        let open: OnEntry = {
            let (id, go, path, on_open) = (id.clone(), go.clone(), path.clone(), self.on_open);
            Rc::new(move |entry, window, cx| {
                if entry.is_folder() {
                    let inside: Vec<SharedString> =
                        path.iter().cloned().chain([entry.name().clone()]).collect();
                    return go(&inside, window, cx);
                }
                log::info!("file explorer {id:?}: open {}", entry.name());
                if let Some(on_open) = &on_open {
                    on_open(entry, window, cx);
                }
            })
        };
        let select: OnValue = {
            let (id, on_select) = (id.clone(), self.on_select);
            Rc::new(move |name, window, cx| {
                log::info!("file explorer {id:?}: picked {name}");
                if let Some(on_select) = &on_select {
                    on_select(name, window, cx);
                }
            })
        };
        let mut levels = self.levels;
        let here = levels.last().expect("a listed folder").clone();
        let theme = cx.theme();
        let colors = &theme.colors;
        let count = here.len();
        let key = VIEWS
            .iter()
            .find(|each| each.0 == self.view)
            .expect("a listed view")
            .1;
        let on_view = self.on_view;
        let switch = VIEWS
            .iter()
            .fold(
                SegmentedControl::new((id.clone(), "view"), key).size(ControlSize::Sm),
                |switch, (_, key, label, icon)| switch.segment(*key, *label, Some(*icon)),
            )
            .on_change(move |key, window, cx| {
                let view = VIEWS
                    .iter()
                    .find(|each| each.1 == key.as_ref())
                    .expect("a listed view")
                    .0;
                if let Some(on_view) = &on_view {
                    on_view(view, window, cx);
                }
            });
        let header = div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap_2()
            .child(
                div()
                    .flex_1()
                    .min_w(theme.label_width())
                    .flex()
                    .items_baseline()
                    .gap_2()
                    .child(
                        div()
                            .min_w_0()
                            .text_size(theme.text_size(TextSize::Md))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.fg)
                            .child(Ellipsis::new(path[path.len() - 1].clone())),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(colors.fg_muted)
                            .child(match count {
                                1 => "1 item".to_string(),
                                count => format!("{count} items"),
                            }),
                    ),
            )
            .child(div().flex_none().child(switch));
        let body = match self.view {
            FileView::List => {
                let listing = DirectoryListing::new((id.clone(), "list"), path.to_vec(), here)
                    .on_select(move |name, window, cx| select(name, window, cx))
                    .on_open(move |entry, window, cx| open(entry, window, cx))
                    .on_climb(move |level, window, cx| climb(level, window, cx));
                match self.selected {
                    Some(name) => listing.selected(name),
                    None => listing,
                }
                .into_any_element()
            }
            FileView::Icons => {
                let grid = FileGrid::new((id.clone(), "icons"), listed(here))
                    .on_select(move |name, window, cx| select(name, window, cx))
                    .on_open(move |entry, window, cx| open(entry, window, cx));
                let grid = match self.selected {
                    Some(name) => grid.selected(name),
                    None => grid,
                };
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(path_bar((id.clone(), "path"), &path, Some(climb)))
                    .child(grid)
                    .into_any_element()
            }
            FileView::Columns => {
                levels
                    .iter_mut()
                    .for_each(|level| *level = listed(std::mem::take(level)));
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(path_bar((id.clone(), "path"), &path, Some(climb)))
                    .child(columns(
                        (id.clone(), "columns").into(),
                        path.clone(),
                        levels,
                        self.selected,
                        (go, select, open),
                        window,
                        cx,
                    ))
                    .into_any_element()
            }
        };
        div()
            .debug_selector(|| "file-explorer".into())
            .flex()
            .flex_col()
            .gap_3()
            .child(header)
            .child(body)
    }
}
