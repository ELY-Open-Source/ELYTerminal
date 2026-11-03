use std::rc::Rc;

use gpui::{
    App, Axis, Context, ElementId, Entity, FocusHandle, ImageSource, InteractiveElement,
    IntoElement, MouseButton, ParentElement, RenderOnce, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Window, canvas, div, prelude::*, rems,
};

use crate::{
    buttons::{Button, ButtonVariant},
    forms::{Input, InputEvent, OnValue, TextInput},
    layout::{Masonry, bring_into_view, on_axis},
    motion::Skeleton,
    primitives::{FocusRing, Icon, IconName, Image, Tooltip, checked_ratio, framed, tab_stop},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
};

/// Shapes the loading tiles take, so the wait looks like GIFs.
const WAITING: [f32; 6] = [1.5, 1.0, 1.8, 1.25, 1.6, 1.1];

/// A GIF: its key, its title, its picture, and its width over its height.
#[derive(Clone)]
pub struct Gif {
    pub key: SharedString,
    pub title: SharedString,
    pub picture: ImageSource,
    pub ratio: f32,
}

impl Gif {
    pub fn new(
        key: impl Into<SharedString>,
        title: impl Into<SharedString>,
        picture: impl Into<ImageSource>,
        ratio: f32,
    ) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            picture: picture.into(),
            ratio: checked_ratio(ratio),
        }
    }
}

/// Where a GIF search stands: waiting on the host, or what it found.
#[derive(Clone)]
pub enum Gifs {
    Loading,
    Found(Vec<Gif>),
}

/// The search field, the words last reported, and who gets them.
struct Search {
    input: Entity<TextInput>,
    words: SharedString,
    on_query: Option<OnValue>,
    _changes: Subscription,
}

/// GIFs from the host in two columns packed tight, under a search the host answers; while it is empty, categories fill it. Each GIF is a Tab stop, picked by a press, Enter or Space.
#[derive(IntoElement)]
pub struct GifPicker {
    id: ElementId,
    gifs: Gifs,
    categories: Vec<SharedString>,
    on_query: Option<OnValue>,
    on_pick: Option<OnValue>,
}

impl GifPicker {
    pub fn new(id: impl Into<ElementId>, gifs: Gifs) -> Self {
        if let Gifs::Found(found) = &gifs {
            for (ix, gif) in found.iter().enumerate() {
                let twice = found[..ix].iter().any(|other| other.key == gif.key);
                assert!(!twice, "gif {} twice", gif.key);
            }
        }
        Self {
            id: id.into(),
            gifs,
            categories: Vec::new(),
            on_query: None,
            on_pick: None,
        }
    }

    /// Searches offered while the field is empty; a press puts one in it.
    pub fn categories(
        mut self,
        categories: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        self.categories = categories.into_iter().map(Into::into).collect();
        self
    }

    /// Gets the search's words, trimmed, each time they change.
    pub fn on_query(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_query = Some(Rc::new(handler));
        self
    }

    /// Gets the key of the GIF picked.
    pub fn on_pick(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for GifPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let search = window.use_keyed_state(
            (self.id.clone(), "search"),
            cx,
            |window, cx: &mut Context<Search>| {
                let input = cx.new(|cx| TextInput::new(window, cx).placeholder("Search GIFs"));
                let changes =
                    cx.subscribe_in(&input, window, |search, input, event, window, cx| {
                        if *event != InputEvent::Changed {
                            return;
                        }
                        let query = SharedString::from(input.read(cx).text().trim().to_string());
                        if query == search.words {
                            return;
                        }
                        search.words = query.clone();
                        log::info!("gif picker: search {query:?}");
                        if let Some(on_query) = search.on_query.clone() {
                            on_query(&query, window, cx);
                        }
                        cx.notify();
                    });
                Search {
                    input,
                    words: SharedString::default(),
                    on_query: None,
                    _changes: changes,
                }
            },
        );
        search.update(cx, |search, _| search.on_query = self.on_query.clone());
        let input = search.read(cx).input.clone();
        let empty = input.read(cx).text().trim().is_empty();
        let scroll = window
            .use_keyed_state((self.id.clone(), "scroll"), cx, |_, _| ScrollHandle::new())
            .read(cx)
            .clone();
        let revealed =
            window.use_keyed_state(
                (self.id.clone(), "revealed"),
                cx,
                |_, _| None::<SharedString>,
            );
        let focuses: Vec<FocusHandle> = match &self.gifs {
            Gifs::Loading => Vec::new(),
            Gifs::Found(found) => found
                .iter()
                .map(|gif| {
                    let id = (self.id.clone(), format!("gif-focus-{}", gif.key)).into();
                    tab_stop(id, true, window, cx)
                })
                .collect(),
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let round = theme.radius(Radius::Md);
        let gap = rems(0.5).to_pixels(window.rem_size());
        let categories = (empty && !self.categories.is_empty()).then(|| {
            div()
                .flex()
                .flex_wrap()
                .gap_1()
                .children(self.categories.iter().map(|category| {
                    let (field, words) = (input.clone(), category.clone());
                    Button::new(
                        (self.id.clone(), format!("category-{category}")),
                        category.clone(),
                    )
                    .variant(ButtonVariant::Subtle)
                    .size(ControlSize::Sm)
                    .on_click(move |_, _, cx| {
                        log::info!("gif picker: category {words}");
                        let words = words.clone();
                        field.update(cx, |input, cx| input.set_text(words.to_string(), cx));
                    })
                }))
        });
        let tiles: Vec<gpui::AnyElement> = match self.gifs {
            Gifs::Loading => WAITING
                .iter()
                .enumerate()
                .map(|(ix, ratio)| {
                    framed(*ratio, cx)
                        .rounded(round)
                        .child(
                            Skeleton::new((self.id.clone(), format!("waiting-{ix}"))).size_full(),
                        )
                        .into_any_element()
                })
                .collect(),
            Gifs::Found(found) => found
                .into_iter()
                .zip(focuses)
                .map(|(gif, focus)| {
                    let (key, on_pick, owner) =
                        (gif.key.clone(), self.on_pick.clone(), self.id.clone());
                    let (shown, revealed, scroll) =
                        (gif.key.clone(), revealed.clone(), scroll.clone());
                    let reveal = canvas(
                        {
                            let focus = focus.clone();
                            move |bounds, window, cx| {
                                let focused = focus.is_focused(window);
                                let held = revealed.read(cx).as_ref() == Some(&shown);
                                if focused && !held {
                                    revealed
                                        .update(cx, |revealed, _| *revealed = Some(shown.clone()));
                                    if bring_into_view(&scroll, bounds, Axis::Vertical) {
                                        log::info!("gif picker: {shown} scrolled into view");
                                        window.request_animation_frame();
                                    }
                                } else if !focused && held {
                                    revealed.update(cx, |revealed, _| *revealed = None);
                                }
                            }
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full();
                    let tile = div()
                        .id((self.id.clone(), format!("gif-{}", gif.key)))
                        .debug_selector({
                            let key = gif.key.clone();
                            move || format!("gif {key}")
                        })
                        .rounded(round)
                        .border_1()
                        .border_color(gpui::transparent_black())
                        .track_focus(&focus)
                        .focus_ring(cx)
                        .cursor_pointer()
                        .tooltip(Tooltip::text(gif.title.clone()))
                        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                        .on_click(move |_, window, cx| {
                            log::info!("gif picker {owner}: {key}");
                            if let Some(on_pick) = &on_pick {
                                on_pick(&key, window, cx);
                            }
                        })
                        .child(
                            framed(gif.ratio, cx).rounded(round).child(
                                Image::new(
                                    (self.id.clone(), format!("picture-{}", gif.key)),
                                    gif.picture,
                                )
                                .size_full()
                                .rounded(round),
                            ),
                        );
                    div()
                        .relative()
                        .child(tile)
                        .child(reveal)
                        .into_any_element()
                })
                .collect(),
        };
        let none = tiles.is_empty().then(|| {
            div()
                .debug_selector(|| "gifs-none".into())
                .py_6()
                .flex()
                .flex_col()
                .items_center()
                .gap_2()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(colors.fg_muted)
                .child(
                    Icon::new(IconName::SearchX)
                        .size(IconSize::Lg)
                        .color(colors.fg_subtle),
                )
                .child("No GIFs match")
        });
        let masonry = (!tiles.is_empty())
            .then(|| Masonry::new((self.id.clone(), "gifs"), 2, gap).children(tiles));
        div()
            .debug_selector(|| "gif-picker".into())
            .flex()
            .flex_col()
            .gap_2()
            .child(
                Input::new(&input).prefix(
                    Icon::new(IconName::Search)
                        .size(IconSize::Sm)
                        .color(colors.fg_subtle),
                ),
            )
            .children(categories)
            .child(
                on_axis(
                    div()
                        .id((self.id.clone(), "results"))
                        .debug_selector(|| "gif-results".into())
                        .track_scroll(&scroll)
                        .max_h(theme.list_max_height())
                        .overflow_y_scroll(),
                )
                .children(none)
                .children(masonry),
            )
    }
}
