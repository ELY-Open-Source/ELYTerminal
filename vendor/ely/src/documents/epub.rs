use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce,
    ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use super::{reading::ReadingProgress, render::MarkdownRenderer};
use crate::{
    buttons::{Button, ButtonVariant, IconButton, ToggleButton, ToggleItem},
    primitives::IconName,
    theme::{ActiveTheme, ContainerSize, ControlSize, Radius, TextSize},
    typography::tabular,
};

/// Type sizes a reader steps through.
const SIZES: [TextSize; 4] = [TextSize::Sm, TextSize::Base, TextSize::Lg, TextSize::Xl];

/// A chapter of a book the app opened: its title and its text, as markdown.
#[derive(Clone, Debug, PartialEq)]
pub struct Chapter {
    pub title: SharedString,
    pub markdown: SharedString,
}

/// Where the reader is: the chapter, the type size, whether the chapters show, and the chapter's scroll.
struct Place {
    chapter: usize,
    size: usize,
    contents: bool,
    scroll: ScrollHandle,
}

/// A book read a chapter at a time: the chapter in a column sized for reading with how far through it the reader is, the chapters listed to jump between, the type a step larger or smaller, and the chapters before and after a press away.
#[derive(IntoElement)]
pub struct EpubReader {
    id: ElementId,
    title: SharedString,
    chapters: Rc<Vec<Chapter>>,
}

impl EpubReader {
    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        chapters: impl Into<Rc<Vec<Chapter>>>,
    ) -> Self {
        let chapters = chapters.into();
        assert!(!chapters.is_empty(), "a book holds a chapter");
        Self {
            id: id.into(),
            title: title.into(),
            chapters,
        }
    }
}

fn set(place: &gpui::Entity<Place>, cx: &mut App, change: impl FnOnce(&mut Place)) {
    place.update(cx, |place, cx| {
        change(place);
        cx.notify();
    });
}

impl RenderOnce for EpubReader {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let place = window.use_keyed_state((self.id.clone(), "place"), cx, |_, _| Place {
            chapter: 0,
            size: 1,
            contents: false,
            scroll: ScrollHandle::new(),
        });
        let (chapter, size, contents, scroll) = {
            let place = place.read(cx);
            (
                place.chapter.min(self.chapters.len() - 1),
                place.size,
                place.contents,
                place.scroll.clone(),
            )
        };
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let count = self.chapters.len();
        let turn = |to: usize| {
            let (place, scroll) = (place.clone(), scroll.clone());
            move |_: &gpui::ClickEvent, _: &mut Window, cx: &mut App| {
                log::info!("epub reader: chapter {to}");
                scroll.set_offset(gpui::Point::default());
                set(&place, cx, |place| place.chapter = to);
            }
        };
        let sized = |to: Option<usize>, icon: IconName, tip: &'static str| {
            let place = place.clone();
            IconButton::new((self.id.clone(), tip), icon)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip(tip)
                .disabled(to.is_none())
                .when_some(to, |button, to| {
                    button.on_click(move |_, _, cx| set(&place, cx, |place| place.size = to))
                })
        };
        let shown = &self.chapters[chapter];
        let listed = contents.then(|| {
            div()
                .id((self.id.clone(), "chapters"))
                .flex_none()
                .w(theme.sidebar_width(false))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap_0p5()
                .p_2()
                .border_r_1()
                .border_color(colors.border)
                .children(self.chapters.iter().enumerate().map(|(ix, entry)| {
                    div()
                        .id((self.id.clone(), format!("chapter-{ix}")))
                        .px_2()
                        .py_1()
                        .rounded(theme.radius(Radius::Sm))
                        .cursor_pointer()
                        .text_color(if ix == chapter {
                            colors.fg
                        } else {
                            colors.fg_muted
                        })
                        .when(ix == chapter, |row| row.bg(colors.active))
                        .hover(|row| row.bg(colors.hover))
                        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                        .on_click(turn(ix))
                        .child(entry.title.clone())
                }))
        });
        let toggle = place.clone();
        div()
            .id(self.id.clone())
            .size_full()
            .flex()
            .flex_col()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .overflow_hidden()
            .bg(colors.surface)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .h(theme.control_height(ControlSize::Lg))
                    .child(
                        ToggleButton::new(
                            (self.id.clone(), "contents"),
                            ToggleItem::new("contents")
                                .icon(IconName::List)
                                .tooltip("Chapters"),
                            contents,
                        )
                        .size(ControlSize::Sm)
                        .on_toggle(move |on, _, cx| set(&toggle, cx, |place| place.contents = on)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(colors.fg_muted)
                            .child(self.title.clone()),
                    )
                    .child(
                        tabular(
                            div()
                                .text_size(theme.text_size(TextSize::Sm))
                                .text_color(colors.fg_subtle),
                        )
                        .child(format!("{} of {count}", chapter + 1)),
                    )
                    .child(sized(
                        size.checked_sub(1),
                        IconName::AArrowDown,
                        "Smaller type",
                    ))
                    .child(sized(
                        Some(size + 1).filter(|size| *size < SIZES.len()),
                        IconName::AArrowUp,
                        "Larger type",
                    )),
            )
            .child(ReadingProgress::new(&scroll))
            .child(
                div().flex().flex_1().min_h_0().children(listed).child(
                    div()
                        .id((self.id.clone(), "page"))
                        .flex_1()
                        .min_w_0()
                        .overflow_y_scroll()
                        .track_scroll(&scroll)
                        .child(
                            div()
                                .mx_auto()
                                .max_w(theme.container_width(ContainerSize::Sm))
                                .px_8()
                                .py_8()
                                .flex()
                                .flex_col()
                                .gap_6()
                                .child(
                                    MarkdownRenderer::new(
                                        (self.id.clone(), format!("text-{chapter}")),
                                        shown.markdown.clone(),
                                    )
                                    .text_size(SIZES[size]),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .justify_between()
                                        .pt_4()
                                        .border_t_1()
                                        .border_color(colors.border)
                                        .child(div().children(chapter.checked_sub(1).map(
                                            |previous| {
                                                Button::new(
                                                    (self.id.clone(), "previous"),
                                                    format!("← {}", self.chapters[previous].title),
                                                )
                                                .variant(ButtonVariant::Ghost)
                                                .on_click(turn(previous))
                                            },
                                        )))
                                        .child(div().children((chapter + 1 < count).then(|| {
                                            Button::new(
                                                (self.id.clone(), "next"),
                                                format!("{} →", self.chapters[chapter + 1].title),
                                            )
                                            .variant(ButtonVariant::Ghost)
                                            .on_click(turn(chapter + 1))
                                        }))),
                                ),
                        ),
                ),
            )
    }
}
