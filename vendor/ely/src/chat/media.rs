use std::rc::Rc;

use gpui::{
    AnyElement, App, Div, ElementId, FontWeight, InteractiveElement, IntoElement, ObjectFit,
    ParentElement, Pixels, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window,
    div, prelude::*,
};

use crate::{
    buttons::{ButtonVariant, IconButton},
    documents::source,
    forms::{Pick, Run},
    primitives::{Icon, IconName, Image, file_icon},
    theme::{ActiveTheme, AvatarSize, ContainerSize, IconSize, Radius, TextSize},
    typography::{Ellipsis, format::file_size},
};

/// A box in a picture's own shape: as wide as the picture, but no wider than `widest` or its container.
pub(crate) fn shaped(width: f32, height: f32, widest: Pixels) -> Div {
    assert!(
        width > 0.0 && height > 0.0,
        "a picture of {width}x{height} has no shape"
    );
    let mut frame = div()
        .w(Pixels::from(width.min(f32::from(widest))))
        .max_w_full();
    frame.style().aspect_ratio = Some(width / height);
    frame
}

/// A picture in a message, in its own shape up to the column's width, a caption under it; a press asks the owner to open it large.
#[derive(IntoElement)]
pub struct ImageMessage {
    id: ElementId,
    source: SharedString,
    size: (f32, f32),
    caption: Option<SharedString>,
    on_open: Option<Run>,
}

impl ImageMessage {
    /// `source` is a file or a web address; `width` and `height` its pixels.
    pub fn new(
        id: impl Into<ElementId>,
        source: impl Into<SharedString>,
        width: f32,
        height: f32,
    ) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
            size: (width, height),
            caption: None,
            on_open: None,
        }
    }

    pub fn caption(mut self, caption: impl Into<SharedString>) -> Self {
        self.caption = Some(caption.into());
        self
    }

    pub fn on_open(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ImageMessage {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let widest = theme.prose_width().to_pixels(window.rem_size());
        let open = self.on_open;
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div().child(
                    shaped(self.size.0, self.size.1, widest)
                        .id(self.id.clone())
                        .relative()
                        .when_some(open, |picture, open| {
                            picture
                                .cursor_pointer()
                                .on_click(move |_, window, cx| open(window, cx))
                        })
                        .child(
                            Image::new((self.id.clone(), "picture"), source(&self.source))
                                .fit(ObjectFit::Contain)
                                .size_full(),
                        ),
                ),
            )
            .children(self.caption.map(|caption| {
                div()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(theme.colors.fg_muted)
                    .child(caption)
            }))
    }
}

/// Pictures sent together as a grid: one fills it, two share a row, three set one large beside two, four make a square; past four, the last tile counts the rest. A press asks to open one.
#[derive(IntoElement)]
pub struct ImageGrid {
    id: ElementId,
    sources: Vec<SharedString>,
    on_open: Option<Pick>,
}

impl ImageGrid {
    pub fn new(
        id: impl Into<ElementId>,
        sources: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        let sources: Vec<SharedString> = sources.into_iter().map(Into::into).collect();
        assert!(!sources.is_empty(), "a grid holds a picture");
        Self {
            id: id.into(),
            sources,
            on_open: None,
        }
    }

    /// Gets the index of the picture pressed.
    pub fn on_open(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ImageGrid {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let side = theme.prose_width();
        let rest = self.sources.len().saturating_sub(4);
        let tile = |ix: usize| -> AnyElement {
            let open = self.on_open.clone();
            div()
                .id((self.id.clone(), format!("tile-{ix}")))
                .relative()
                .flex_1()
                .min_w_0()
                .h_full()
                .bg(colors.sunken)
                .when_some(open, |tile, open| {
                    tile.cursor_pointer()
                        .on_click(move |_, window, cx| open(ix, window, cx))
                })
                .child(
                    Image::new(
                        (self.id.clone(), format!("picture-{ix}")),
                        source(&self.sources[ix]),
                    )
                    .fit(ObjectFit::Cover)
                    .size_full(),
                )
                .when(ix == 3 && rest > 0, |tile| {
                    tile.child(
                        div()
                            .absolute()
                            .inset_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(colors.media_backdrop.opacity(0.55))
                            .text_size(theme.text_size(TextSize::Lg))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.on_media)
                            .child(format!("+{rest}")),
                    )
                })
                .into_any_element()
        };
        let row = || div().flex().flex_1().min_h_0().gap_0p5();
        let (count, rem) = (self.sources.len(), window.rem_size());
        let wide = f32::from(side.to_pixels(rem));
        let grid = shaped(
            wide,
            if count == 2 { wide / 2.0 } else { wide },
            side.to_pixels(rem),
        )
        .flex()
        .flex_col()
        .gap_0p5();
        let grid = match count {
            1 => grid.child(row().child(tile(0))),
            2 => grid.child(row().child(tile(0)).child(tile(1))),
            3 => grid.child(
                div().flex().flex_1().gap_0p5().child(tile(0)).child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .child(row().child(tile(1)))
                        .child(row().child(tile(2))),
                ),
            ),
            _ => grid
                .child(row().child(tile(0)).child(tile(1)))
                .child(row().child(tile(2)).child(tile(3))),
        };
        div().child(grid)
    }
}

/// A file in a message: its kind's icon, its name and size, and a way to download it.
#[derive(IntoElement)]
pub struct FileMessage {
    id: ElementId,
    name: SharedString,
    bytes: u64,
    on_download: Option<Run>,
}

impl FileMessage {
    pub fn new(id: impl Into<ElementId>, name: impl Into<SharedString>, bytes: u64) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            bytes,
            on_download: None,
        }
    }

    pub fn on_download(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_download = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for FileMessage {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        div()
            .max_w(theme.prose_width())
            .flex()
            .items_center()
            .gap_3()
            .p_2()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .child(
                div()
                    .flex_none()
                    .size(theme.avatar_size(AvatarSize::Md))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(theme.radius(Radius::Sm))
                    .bg(colors.hover)
                    .child(
                        Icon::new(file_icon(&self.name, cx))
                            .size(IconSize::Md)
                            .color(colors.fg_muted),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .font_weight(FontWeight::MEDIUM)
                            .child(Ellipsis::new(self.name.clone())),
                    )
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg_subtle)
                            .child(file_size(self.bytes, false)),
                    ),
            )
            .children(self.on_download.map(|download| {
                IconButton::new((self.id.clone(), "download"), IconName::Download)
                    .variant(ButtonVariant::Ghost)
                    .tooltip("Download")
                    .on_click(move |_, window, cx| download(window, cx))
            }))
    }
}

/// A link unfolded: the site, the page's title and a line about it, and its picture; a press opens it.
#[derive(IntoElement)]
pub struct LinkPreviewCard {
    id: ElementId,
    site: SharedString,
    title: SharedString,
    description: Option<SharedString>,
    picture: Option<SharedString>,
    on_open: Option<Run>,
}

impl LinkPreviewCard {
    pub fn new(
        id: impl Into<ElementId>,
        site: impl Into<SharedString>,
        title: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            site: site.into(),
            title: title.into(),
            description: None,
            picture: None,
            on_open: None,
        }
    }

    pub fn description(mut self, text: impl Into<SharedString>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// A square picture, a file or a web address.
    pub fn picture(mut self, picture: impl Into<SharedString>) -> Self {
        self.picture = Some(picture.into());
        self
    }

    pub fn on_open(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for LinkPreviewCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let open = self.on_open;
        div()
            .id(self.id.clone())
            .max_w(theme.container_width(ContainerSize::Sm))
            .flex()
            .gap_3()
            .p_3()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .when_some(open, |card, open| {
                card.cursor_pointer()
                    .hover(|card| card.bg(colors.hover))
                    .on_click(move |_, window, cx| open(window, cx))
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg_subtle)
                            .child(
                                Icon::new(IconName::Globe)
                                    .size(IconSize::Xs)
                                    .color(colors.fg_subtle),
                            )
                            .child(self.site),
                    )
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .font_weight(FontWeight::MEDIUM)
                            .child(self.title),
                    )
                    .children(self.description.map(|text| {
                        div()
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg_muted)
                            .child(text)
                    })),
            )
            .children(self.picture.map(|picture| {
                div()
                    .flex_none()
                    .size(theme.avatar_size(AvatarSize::Xl))
                    .rounded(theme.radius(Radius::Md))
                    .bg(colors.sunken)
                    .child(
                        Image::new((self.id.clone(), "picture"), source(&picture))
                            .fit(ObjectFit::Cover)
                            .rounded(theme.radius(Radius::Md))
                            .size_full(),
                    )
            }))
    }
}
