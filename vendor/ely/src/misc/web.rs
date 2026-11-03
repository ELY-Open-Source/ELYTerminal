use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div,
};

use crate::{
    buttons::IconButton,
    primitives::{Icon, IconName, checked_ratio, framed},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::{Ellipsis, literal},
};

/// What a web view shows: a page at an address, or the owner's HTML.
#[derive(Clone, Debug, PartialEq)]
pub enum WebSource {
    Url(SharedString),
    Html(SharedString),
}

/// Whether a box lies, at least in part, inside what the window shows of its region.
#[cfg(any(target_os = "macos", test))]
pub(crate) fn in_view(
    bounds: gpui::Bounds<gpui::Pixels>,
    shown: gpui::Bounds<gpui::Pixels>,
) -> bool {
    bounds.intersects(&shown)
}

/// A web page in a native web view laid over this box: an address, or the owner's HTML. The page draws above everything gpui paints, dialogs too, and gpui's clipping does not reach it; it follows the box each frame and hides while the box is out of view. A press on the page gives it the keys; a press anywhere gpui draws, or the view leaving, gives them back. macOS only; elsewhere the box says so.
#[derive(IntoElement)]
pub struct WebView {
    id: ElementId,
    source: WebSource,
}

impl WebView {
    pub fn new(id: impl Into<ElementId>, source: WebSource) -> Self {
        Self {
            id: id.into(),
            source,
        }
    }
}

impl RenderOnce for WebView {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        #[cfg(target_os = "macos")]
        return mac::page(self.id, self.source, window, cx);
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (window, self.id, self.source);
            let theme = cx.theme();
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.colors.fg_muted)
                .child("Web views need macOS for now.")
                .into_any_element()
        }
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use gpui::{
        AnyElement, App, Bounds, DispatchPhase, ElementId, IntoElement, MouseDownEvent,
        ParentElement, Pixels, SharedString, Styled, Window, canvas, div,
    };
    use wry::{
        Rect, WebViewBuilder,
        dpi::{LogicalPosition, LogicalSize},
    };

    use super::{WebSource, in_view};
    use crate::theme::ActiveTheme;

    /// The native view, what it shows, and why the last load failed.
    struct Page {
        view: wry::WebView,
        shown: WebSource,
        fault: Option<SharedString>,
    }

    impl Drop for Page {
        fn drop(&mut self) {
            if let Err(error) = self.view.focus_parent() {
                log::error!("web view: keys stay with the page as it goes: {error}");
            }
        }
    }

    fn rect(bounds: Bounds<Pixels>) -> Rect {
        Rect {
            position: LogicalPosition::new(f32::from(bounds.origin.x), f32::from(bounds.origin.y))
                .into(),
            size: LogicalSize::new(f32::from(bounds.size.width), f32::from(bounds.size.height))
                .into(),
        }
    }

    /// The words a box shows in place of a page that cannot show.
    fn fault(why: SharedString, cx: &App) -> AnyElement {
        let theme = cx.theme();
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .p_4()
            .text_color(theme.colors.danger)
            .child(format!("This page cannot show: {why}"))
            .into_any_element()
    }

    pub(super) fn page(
        id: ElementId,
        source: WebSource,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let page = window.use_keyed_state((id.clone(), "page"), cx, {
            let (id, source) = (id.clone(), source.clone());
            move |window, _| {
                let builder = match &source {
                    WebSource::Url(url) => WebViewBuilder::new().with_url(url.to_string()),
                    WebSource::Html(html) => WebViewBuilder::new().with_html(html.to_string()),
                };
                match builder.build_as_child(&*window) {
                    Ok(view) => {
                        log::info!("web view {id:?}: built");
                        Ok(Page {
                            view,
                            shown: source,
                            fault: None,
                        })
                    }
                    Err(error) => {
                        log::error!("web view {id:?}: wry built no view: {error}");
                        Err(SharedString::from(error.to_string()))
                    }
                }
            }
        });
        let loaded = match page.read(cx) {
            Err(why) => return fault(why.clone(), cx),
            Ok(built) if built.shown != source => Some(match &source {
                WebSource::Url(url) => built.view.load_url(url),
                WebSource::Html(html) => built.view.load_html(html),
            }),
            Ok(_) => None,
        };
        if let Some(loaded) = loaded {
            log::info!("web view {id:?}: loads anew");
            let fault = loaded.err().map(|error| {
                log::error!("web view {id:?}: load failed: {error}");
                SharedString::from(error.to_string())
            });
            page.update(cx, |page, _| {
                if let Ok(page) = page {
                    page.shown = source;
                    page.fault = fault;
                }
            });
        }
        if let Ok(Page {
            fault: Some(why), ..
        }) = page.read(cx)
        {
            let why = why.clone();
            if let Ok(built) = page.read(cx)
                && let Err(error) = built.view.set_visible(false)
            {
                log::error!("web view {id:?}: visibility: {error}");
            }
            return fault(why, cx);
        }
        let pressed = page.clone();
        div()
            .relative()
            .size_full()
            .child(
                canvas(
                    move |bounds, window, cx| {
                        let Ok(built) = page.read(cx) else { return };
                        if let Err(error) = built.view.set_bounds(rect(bounds)) {
                            log::error!("web view {id:?}: bounds: {error}");
                        }
                        let shown = in_view(bounds, window.content_mask().bounds);
                        if let Err(error) = built.view.set_visible(shown) {
                            log::error!("web view {id:?}: visibility: {error}");
                        }
                    },
                    move |_, _, window, _| {
                        window.on_mouse_event(move |_: &MouseDownEvent, phase, _, cx| {
                            if phase == DispatchPhase::Capture
                                && let Ok(built) = pressed.read(cx)
                            {
                                log::debug!("web view: keys go back to gpui");
                                if let Err(error) = built.view.focus_parent() {
                                    log::error!("web view: keys stay with the page: {error}");
                                }
                            }
                        });
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .into_any_element()
    }
}

/// A page from another address in a frame of its own: the address over the page, with Open in browser. The page is a `WebView` in the frame's shape.
#[derive(IntoElement)]
pub struct IframeEmbed {
    id: ElementId,
    url: SharedString,
    ratio: f32,
}

impl IframeEmbed {
    /// `ratio` is the frame's width over its height.
    pub fn new(id: impl Into<ElementId>, url: impl Into<SharedString>, ratio: f32) -> Self {
        Self {
            id: id.into(),
            url: url.into(),
            ratio: checked_ratio(ratio),
        }
    }
}

impl RenderOnce for IframeEmbed {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let (id, url) = (self.id, self.url);
        let opened = url.clone();
        let frame: AnyElement = framed(self.ratio, cx)
            .rounded_b(theme.radius(Radius::Lg))
            .child(WebView::new(
                (id.clone(), "page"),
                WebSource::Url(url.clone()),
            ))
            .into_any_element();
        div()
            .flex()
            .flex_col()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .border_b_1()
                    .border_color(colors.border)
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(colors.fg_muted)
                    .child(
                        Icon::new(IconName::Globe)
                            .size(IconSize::Sm)
                            .color(colors.fg_subtle),
                    )
                    .child(literal(div()).flex_1().min_w_0().child(Ellipsis::new(url)))
                    .child(
                        IconButton::new((id, "open"), IconName::ExternalLink)
                            .tooltip("Open in browser")
                            .on_click(move |_, _, cx| {
                                log::info!("iframe embed: opened {opened}");
                                cx.open_url(&opened)
                            }),
                    ),
            )
            .child(frame)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, point, px, size};

    use super::in_view;

    #[test]
    fn a_box_is_in_view_while_it_meets_what_the_window_shows() {
        let shown = Bounds::new(point(px(0.0), px(0.0)), size(px(400.0), px(300.0)));
        let at = |y: f32| Bounds::new(point(px(20.0), px(y)), size(px(200.0), px(100.0)));
        assert!(in_view(at(250.0), shown), "part of it shows");
        assert!(
            in_view(at(-50.0), shown),
            "its top lies above the view, its body in it"
        );
        assert!(!in_view(at(320.0), shown), "below the view");
        assert!(!in_view(at(-120.0), shown), "above it");
    }
}
