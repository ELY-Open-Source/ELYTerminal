use std::{ops::Range, rc::Rc};

use gpui::{
    App, ElementId, FontWeight, HighlightStyle, InteractiveElement, IntoElement, MouseButton,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, StyledText,
    Window, div, prelude::*,
};
use jiff::Timestamp;

use crate::{
    buttons::{Button, ButtonVariant, ConfirmButton, ConfirmMode, IconButton},
    feedback::EmptyState,
    forms::{OnValue, Pick, Run},
    motion::Reorder,
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
    typography::{RelativeTime, format::plural},
};

/// A page that links here: its icon and title, and the words around the link, the link's range among them.
#[derive(Clone, Debug, PartialEq)]
pub struct Backlink {
    pub icon: SharedString,
    pub page: SharedString,
    pub context: SharedString,
    pub mention: Range<usize>,
}

/// The pages that link to this one, each with the words around its link, the link lit; a press opens that page.
#[derive(IntoElement)]
pub struct Backlinks {
    id: ElementId,
    links: Vec<Backlink>,
    on_open: Option<Pick>,
}

impl Backlinks {
    pub fn new(id: impl Into<ElementId>, links: impl IntoIterator<Item = Backlink>) -> Self {
        let links: Vec<Backlink> = links.into_iter().collect();
        for link in &links {
            assert!(
                link.mention.end <= link.context.len()
                    && link.context.is_char_boundary(link.mention.start)
                    && link.context.is_char_boundary(link.mention.end),
                "the mention {:?} lies outside {:?}",
                link.mention,
                link.context
            );
        }
        Self {
            id: id.into(),
            links,
            on_open: None,
        }
    }

    pub fn on_open(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Backlinks {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(colors.fg_muted)
                    .child(plural(self.links.len() as u64, "backlink", "backlinks")),
            )
            .children(self.links.into_iter().enumerate().map(|(ix, link)| {
                let open = self.on_open.clone();
                let lit = HighlightStyle {
                    color: Some(colors.link),
                    font_weight: Some(FontWeight::MEDIUM),
                    ..HighlightStyle::default()
                };
                div()
                    .id((self.id.clone(), format!("link-{ix}")))
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .p_2()
                    .rounded(theme.radius(Radius::Md))
                    .cursor_pointer()
                    .hover(|row| row.bg(colors.hover))
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .when_some(open, |row, open| {
                        row.on_click(move |_, window, cx| {
                            log::info!("backlinks: open {ix}");
                            open(ix, window, cx)
                        })
                    })
                    .child(
                        div()
                            .flex()
                            .gap_1p5()
                            .font_weight(FontWeight::MEDIUM)
                            .child(link.icon.clone())
                            .child(link.page.clone()),
                    )
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(colors.fg_muted)
                            .child(
                                StyledText::new(link.context.clone())
                                    .with_highlights([(link.mention.clone(), lit)]),
                            ),
                    )
            }))
    }
}

/// A page kept close at hand: its key, icon and title.
#[derive(Clone, Debug, PartialEq)]
pub struct Favorite {
    pub key: SharedString,
    pub icon: SharedString,
    pub title: SharedString,
}

type OnMove = Rc<dyn Fn(usize, usize, &mut Window, &mut App)>;

/// Pages pinned for quick reach: a press opens one, dragging reorders them, and the star lets one go.
#[derive(IntoElement)]
pub struct Favorites {
    id: ElementId,
    pages: Vec<Favorite>,
    on_open: Option<OnValue>,
    on_unpin: Option<OnValue>,
    on_move: Option<OnMove>,
}

impl Favorites {
    pub fn new(id: impl Into<ElementId>, pages: impl IntoIterator<Item = Favorite>) -> Self {
        Self {
            id: id.into(),
            pages: pages.into_iter().collect(),
            on_open: None,
            on_unpin: None,
            on_move: None,
        }
    }

    pub fn on_open(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }

    pub fn on_unpin(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_unpin = Some(Rc::new(handler));
        self
    }

    /// Asks to move the page at `from` to place `to`.
    pub fn on_move(
        mut self,
        handler: impl Fn(usize, usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_move = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Favorites {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let moved = self.on_move.clone();
        let list = self.pages.iter().fold(
            Reorder::new((self.id.clone(), "pages")).on_reorder(move |from, to, window, cx| {
                if let Some(moved) = &moved {
                    log::info!("favorites: {from} to {to}");
                    moved(from, to, window, cx);
                }
            }),
            |list, page| {
                let (open, unpin, key) = (
                    self.on_open.clone(),
                    self.on_unpin.clone(),
                    page.key.clone(),
                );
                let group = SharedString::from(format!("favorite-{}", page.key));
                let unpin_key = key.clone();
                list.row(
                    page.key.clone(),
                    div()
                        .id((self.id.clone(), format!("page-{key}")))
                        .group(group.clone())
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_2()
                        .py_1()
                        .rounded(theme.radius(Radius::Sm))
                        .cursor_pointer()
                        .hover(|row| row.bg(colors.hover))
                        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                        .when_some(open, |row, open| {
                            row.on_click(move |_, window, cx| open(&key, window, cx))
                        })
                        .child(page.icon.clone())
                        .child(div().flex_1().min_w_0().child(page.title.clone()))
                        .child(
                            div()
                                .opacity(0.0)
                                .group_hover(group, |star| star.opacity(1.0))
                                .child(
                                    IconButton::new(
                                        (self.id.clone(), format!("unpin-{unpin_key}")),
                                        IconName::Star,
                                    )
                                    .variant(ButtonVariant::Ghost)
                                    .size(ControlSize::Sm)
                                    .tooltip("Remove from favorites")
                                    .when_some(
                                        unpin,
                                        |button, unpin| {
                                            button.on_click(move |_, window, cx| {
                                                cx.stop_propagation();
                                                unpin(&unpin_key, window, cx)
                                            })
                                        },
                                    ),
                                ),
                        ),
                )
            },
        );
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .px_2()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.fg_subtle)
                    .child("Favorites"),
            )
            .child(list)
    }
}

/// A page in the trash: its key, icon, title, where it lived and when it went.
#[derive(Clone, Debug, PartialEq)]
pub struct Trashed {
    pub key: SharedString,
    pub icon: SharedString,
    pub title: SharedString,
    pub place: SharedString,
    pub deleted: Timestamp,
}

/// Deleted pages, the newest first, with where each lived and when it went: restore one, delete one for good, or empty it all, each final step asked twice.
#[derive(IntoElement)]
pub struct TrashBin {
    id: ElementId,
    pages: Vec<Trashed>,
    on_restore: Option<OnValue>,
    on_delete: Option<OnValue>,
    on_empty: Option<Run>,
}

impl TrashBin {
    pub fn new(id: impl Into<ElementId>, pages: impl IntoIterator<Item = Trashed>) -> Self {
        let mut pages: Vec<Trashed> = pages.into_iter().collect();
        pages.sort_by_key(|page| std::cmp::Reverse(page.deleted));
        Self {
            id: id.into(),
            pages,
            on_restore: None,
            on_delete: None,
            on_empty: None,
        }
    }

    pub fn on_restore(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_restore = Some(Rc::new(handler));
        self
    }

    pub fn on_delete(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_delete = Some(Rc::new(handler));
        self
    }

    pub fn on_empty(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_empty = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for TrashBin {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        if self.pages.is_empty() {
            return EmptyState::new(
                (self.id.clone(), "empty"),
                IconName::Trash2,
                "The trash is empty",
            )
            .body("Deleted pages wait here until they are emptied.")
            .into_any_element();
        }
        let empty = self.on_empty.clone();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .pb_1()
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(colors.fg_muted)
                            .child(plural(self.pages.len() as u64, "page", "pages")),
                    )
                    .children(empty.map(|empty| {
                        ConfirmButton::new(
                            (self.id.clone(), "empty-all"),
                            "Empty trash",
                            ConfirmMode::Twice,
                        )
                        .size(ControlSize::Sm)
                        .on_confirm(move |window, cx| {
                            log::info!("trash: emptied");
                            empty(window, cx)
                        })
                    })),
            )
            .children(self.pages.into_iter().map(|page| {
                let (restore, delete, key) = (
                    self.on_restore.clone(),
                    self.on_delete.clone(),
                    page.key.clone(),
                );
                let restore_key = key.clone();
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_2()
                    .py_1p5()
                    .rounded(theme.radius(Radius::Sm))
                    .child(page.icon.clone())
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(page.title.clone())
                            .child(
                                div()
                                    .flex()
                                    .gap_1()
                                    .text_size(theme.text_size(TextSize::Xs))
                                    .text_color(colors.fg_subtle)
                                    .child(format!("In {} ·", page.place))
                                    .child(RelativeTime::new(
                                        (self.id.clone(), format!("when-{key}")),
                                        page.deleted,
                                    )),
                            ),
                    )
                    .child(
                        Button::new((self.id.clone(), format!("restore-{key}")), "Restore")
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Sm)
                            .when_some(restore, |button, restore| {
                                button.on_click(move |_, window, cx| {
                                    log::info!("trash: restore {restore_key}");
                                    restore(&restore_key, window, cx)
                                })
                            }),
                    )
                    .children(delete.map(|delete| {
                        let gone = key.clone();
                        ConfirmButton::new(
                            (self.id.clone(), format!("delete-{key}")),
                            "Delete",
                            ConfirmMode::Twice,
                        )
                        .size(ControlSize::Sm)
                        .on_confirm(move |window, cx| {
                            log::info!("trash: delete {gone} for good");
                            delete(&gone, window, cx)
                        })
                    }))
            }))
            .into_any_element()
    }
}
