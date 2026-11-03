use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled, Window,
    div, prelude::*,
};

use crate::{
    buttons::{Button, ButtonVariant, IconButton},
    forms::{NumberInput, Run},
    i18n,
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, TextSize},
    typography::format::{self, Separators},
};

/// The first, the last, `current` and `near` pages either side; `None` marks a gap of two or more.
pub(crate) fn page_list(current: usize, total: usize, near: usize) -> Vec<Option<usize>> {
    assert!(
        total >= 1 && (1..=total).contains(&current),
        "page {current} is outside 1 to {total}"
    );
    if total == 1 {
        return vec![Some(1)];
    }
    let start = current.saturating_sub(near).max(2);
    let end = (current + near).min(total - 1);
    let mut pages = vec![Some(1)];
    if start > 3 {
        pages.push(None);
    } else {
        pages.extend((2..start).map(Some));
    }
    pages.extend((start..=end).map(Some));
    if end + 2 < total {
        pages.push(None);
    } else {
        pages.extend((end + 1..total).map(Some));
    }
    pages.push(Some(total));
    pages
}

type OnPage = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// Numbered pages between previous and next; long runs fold into gaps.
#[derive(IntoElement)]
pub struct Pagination {
    id: ElementId,
    page: usize,
    pages: usize,
    near: usize,
    size: ControlSize,
    jump: bool,
    on_change: Option<OnPage>,
}

impl Pagination {
    /// `page` counts from 1.
    pub fn new(id: impl Into<ElementId>, page: usize, pages: usize) -> Self {
        Self {
            id: id.into(),
            page,
            pages,
            near: 1,
            size: ControlSize::Sm,
            jump: false,
            on_change: None,
        }
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// A field after Next that goes to the page typed.
    pub fn jump(mut self) -> Self {
        self.jump = true;
        self
    }

    /// Pages shown on each side of the current one.
    pub fn near(mut self, near: usize) -> Self {
        self.near = near;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Pagination {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let valid = (1..=self.pages).contains(&self.page);
        if !valid {
            log::error!(
                "pagination {:?}: page {} of {}; none marked",
                self.id,
                self.page,
                self.pages
            );
        }
        let list = match self.pages {
            0 => Vec::new(),
            pages => page_list(self.page.clamp(1, pages), pages, self.near),
        };
        let (page, pages) = (self.page, self.pages);
        let go = {
            let (id, on_change) = (self.id.clone(), self.on_change);
            Rc::new(move |to: usize, window: &mut Window, cx: &mut App| {
                log::info!("pagination {id:?}: page {to}");
                if let Some(on_change) = &on_change {
                    on_change(to, window, cx);
                }
            })
        };
        let theme = cx.theme();
        let (size, subtle, small) = (
            self.size,
            theme.colors.fg_subtle,
            theme.text_size(TextSize::Sm),
        );
        let (back, ahead, jump) = (go.clone(), go.clone(), go.clone());
        let to = i18n::text(cx, "pagination.jump", &[]);
        let jump = (self.jump && pages > 0).then(|| {
            let words = div()
                .whitespace_nowrap()
                .text_size(small)
                .text_color(subtle)
                .child(to.clone());
            div().flex().items_center().gap_2().child(words).child(
                div().flex_none().child(
                    NumberInput::new((self.id.clone(), "jump"), page as f64)
                        .label(to)
                        .range(1.0, pages as f64)
                        .precision(0)
                        .size(size)
                        .on_commit(move |to, window, cx| {
                            if to as usize != page {
                                jump(to as usize, window, cx);
                            }
                        }),
                ),
            )
        });
        let cells = list
            .into_iter()
            .enumerate()
            .map(move |(ix, entry)| match entry {
                Some(number) => {
                    let go = go.clone();
                    let button = Button::new(("page", number), number.to_string())
                        .size(size)
                        .variant(if number == page {
                            ButtonVariant::Secondary
                        } else {
                            ButtonVariant::Ghost
                        })
                        .on_click(move |_, window, cx| {
                            if number != page {
                                go(number, window, cx);
                            }
                        });
                    div()
                        .debug_selector(move || format!("pagination-page-{number}"))
                        .child(button)
                        .into_any_element()
                }
                None => div()
                    .id(("gap", ix))
                    .px_1()
                    .text_color(subtle)
                    .child("…")
                    .into_any_element(),
            });
        div()
            .id(self.id)
            .debug_selector(|| "pagination-root".into())
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .child(
                div().debug_selector(|| "pagination-previous".into()).child(
                    IconButton::new("previous", IconName::ChevronLeft)
                        .size(size)
                        .variant(ButtonVariant::Ghost)
                        .tooltip(i18n::text(cx, "pagination.previous", &[]))
                        .disabled(!valid || page == 1)
                        .on_click(move |_, window, cx| back(page - 1, window, cx)),
                ),
            )
            .children(cells)
            .child(
                div().debug_selector(|| "pagination-next".into()).child(
                    IconButton::new("next", IconName::ChevronRight)
                        .size(size)
                        .variant(ButtonVariant::Ghost)
                        .tooltip(i18n::text(cx, "pagination.next", &[]))
                        .disabled(!valid || page == pages)
                        .on_click(move |_, window, cx| ahead(page + 1, window, cx)),
                ),
            )
            .children(jump)
    }
}

#[cfg(test)]
mod tests {
    use super::page_list;

    fn shown(current: usize, total: usize) -> String {
        page_list(current, total, 1)
            .into_iter()
            .map(|page| page.map_or("…".to_string(), |page| page.to_string()))
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn gaps_fold_long_runs_and_never_hide_one_page() {
        assert_eq!(shown(6, 20), "1 … 5 6 7 … 20");
        assert_eq!(shown(1, 20), "1 2 … 20");
        assert_eq!(shown(4, 20), "1 2 3 4 5 … 20");
        assert_eq!(shown(17, 20), "1 … 16 17 18 19 20");
        assert_eq!(shown(20, 20), "1 … 19 20");
        assert_eq!(shown(1, 1), "1");
        assert_eq!(shown(2, 2), "1 2");
        assert_eq!(shown(3, 5), "1 2 3 4 5");
    }
}

/// Ends a list that loads in pages: a button that spins while more come, and how many show. Once all show, only the count stays.
#[derive(IntoElement)]
pub struct LoadMore {
    id: ElementId,
    loading: bool,
    shown: Option<(usize, usize)>,
    on_load: Option<Run>,
}

impl LoadMore {
    pub fn new(id: impl Into<ElementId>, loading: bool) -> Self {
        Self {
            id: id.into(),
            loading,
            shown: None,
            on_load: None,
        }
    }

    /// How many show, of how many there are.
    pub fn shown(mut self, shown: usize, total: usize) -> Self {
        assert!(shown <= total, "{shown} shown of only {total}");
        self.shown = Some((shown, total));
        self
    }

    pub fn on_load(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_load = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for LoadMore {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let count = |value: usize| format::number(value as f64, 0, Separators::EN);
        let done = self.shown.is_some_and(|(shown, total)| shown == total);
        let (id, on_load) = (self.id.clone(), self.on_load);
        div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .py_4()
            .when(!done, |more| {
                more.child(
                    Button::new((self.id.clone(), "button"), "Load more")
                        .loading(self.loading)
                        .on_click(move |_, window, cx| {
                            log::info!("load more {id:?}: asked");
                            if let Some(on_load) = &on_load {
                                on_load(window, cx);
                            }
                        }),
                )
            })
            .when_some(self.shown, |more, (shown, total)| {
                more.child(
                    div()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(theme.colors.fg_subtle)
                        .child(if done {
                            format!("All {} shown", count(total))
                        } else {
                            format!("Showing {} of {}", count(shown), count(total))
                        }),
                )
            })
    }
}
