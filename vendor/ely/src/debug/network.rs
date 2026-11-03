use std::rc::Rc;

use gpui::{
    AnyElement, App, Div, ElementId, FontWeight, Hsla, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::*, relative,
};

use super::OnIndex;
use crate::{
    theme::{ActiveTheme, Palette, Radius, TextSize},
    typography::{Ellipsis, format, tabular},
};

/// A request the page made: method and address, status once answered, kind, size, when it started and how long it took in milliseconds, and its headers and body.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub method: SharedString,
    pub url: SharedString,
    pub status: Option<u16>,
    pub kind: SharedString,
    pub size: u64,
    pub start: f64,
    pub duration: f64,
    pub headers: Vec<(SharedString, SharedString)>,
    pub body: Option<SharedString>,
}

/// A status's color: quiet for success, then info, warning and danger.
fn status_color(status: Option<u16>, colors: &Palette) -> Hsla {
    match status {
        None => colors.fg_subtle,
        Some(200..=299) => colors.fg,
        Some(300..=399) => colors.info,
        Some(400..=499) => colors.warning,
        Some(_) => colors.danger,
    }
}

/// The last part of an address's path, else its host.
pub(crate) fn name(url: &str) -> &str {
    let bare = url.split(['?', '#']).next().unwrap_or(url);
    let path = bare.split_once("://").map_or(bare, |(_, rest)| rest);
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
}

/// Requests in the order they started: name, status, kind, size and time, with a waterfall of when each ran; a query keeps those whose address holds it. A press opens a request's headers and body below.
#[derive(IntoElement)]
pub struct NetworkInspector {
    id: ElementId,
    requests: Vec<Request>,
    query: SharedString,
    selected: Option<usize>,
    on_select: Option<OnIndex>,
}

impl NetworkInspector {
    pub fn new(id: impl Into<ElementId>, requests: impl IntoIterator<Item = Request>) -> Self {
        Self {
            id: id.into(),
            requests: requests.into_iter().collect(),
            query: SharedString::default(),
            selected: None,
            on_select: None,
        }
    }

    pub fn query(mut self, query: impl Into<SharedString>) -> Self {
        self.query = query.into();
        self
    }

    pub fn selected(mut self, request: usize) -> Self {
        self.selected = Some(request);
        self
    }

    pub fn on_select(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for NetworkInspector {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let query = self.query.to_lowercase();
        let end = self
            .requests
            .iter()
            .map(|request| request.start + request.duration)
            .fold(0.0_f64, f64::max)
            .max(1.0);
        let columns = |row: Div| row.flex().items_center().gap_3().px_2();
        let widths = [0.5, 0.6, 0.6, 0.6, 2.0].map(|share| theme.label_width() * share);
        let head = columns(div())
            .py_1()
            .border_b_1()
            .border_color(colors.border)
            .text_color(colors.fg_subtle)
            .child(div().flex_1().min_w_0().child("Name"))
            .children(
                ["Status", "Type", "Size", "Time", "Waterfall"]
                    .iter()
                    .zip(widths)
                    .map(|(title, width)| div().flex_none().w(width).child(*title)),
            );
        let rows: Vec<AnyElement> = self
            .requests
            .iter()
            .enumerate()
            .filter(|(_, request)| query.is_empty() || request.url.to_lowercase().contains(&query))
            .map(|(ix, request)| {
                let lit = self.selected == Some(ix);
                let pick = self.on_select.clone();
                let tint = status_color(request.status, &colors);
                columns(div())
                    .id((self.id.clone(), format!("request-{ix}")))
                    .py_1()
                    .rounded(theme.radius(Radius::Sm))
                    .cursor_pointer()
                    .when(lit, |row| row.bg(colors.active))
                    .when(!lit, |row| row.hover(|row| row.bg(colors.hover)))
                    .when_some(pick, |row, pick| {
                        row.on_click(move |_, window, cx| pick(ix, window, cx))
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(if request.status.is_some_and(|code| code >= 400) {
                                tint
                            } else {
                                colors.fg
                            })
                            .child(Ellipsis::new(name(&request.url).to_string())),
                    )
                    .child(
                        tabular(div().flex_none().w(widths[0]).text_color(tint)).child(
                            request
                                .status
                                .map_or("—".to_string(), |code| code.to_string()),
                        ),
                    )
                    .child(
                        div()
                            .flex_none()
                            .w(widths[1])
                            .text_color(colors.fg_muted)
                            .child(request.kind.clone()),
                    )
                    .child(
                        tabular(div().flex_none().w(widths[2]).text_color(colors.fg_muted))
                            .child(format::file_size(request.size, false)),
                    )
                    .child(
                        tabular(div().flex_none().w(widths[3]).text_color(colors.fg_muted))
                            .child(format!("{:.0} ms", request.duration)),
                    )
                    .child(
                        div().flex_none().w(widths[4]).h_2().relative().child(
                            div()
                                .absolute()
                                .top_0()
                                .bottom_0()
                                .left(relative((request.start / end) as f32))
                                .w(relative(((request.duration / end) as f32).max(0.005)))
                                .rounded(theme.radius(Radius::Sm))
                                .bg(if request.status.is_none() {
                                    colors.fg_subtle
                                } else {
                                    colors.chart[0]
                                }),
                        ),
                    )
                    .into_any_element()
            })
            .collect();
        let details = self
            .selected
            .and_then(|ix| self.requests.get(ix))
            .map(|request| {
                let pair = |key: SharedString, value: SharedString| {
                    div()
                        .flex()
                        .gap_3()
                        .child(
                            div()
                                .flex_none()
                                .w(theme.label_width())
                                .text_color(colors.fg_subtle)
                                .child(key),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .font_family(theme.mono_family.clone())
                                .text_color(colors.fg)
                                .child(value),
                        )
                };
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_3()
                    .rounded(theme.radius(Radius::Md))
                    .border_1()
                    .border_color(colors.border)
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.fg)
                            .child(format!("{} {}", request.method, request.url)),
                    )
                    .child(pair(
                        "Status".into(),
                        request
                            .status
                            .map_or("pending".into(), |code| code.to_string().into()),
                    ))
                    .children(
                        request
                            .headers
                            .iter()
                            .map(|(key, value)| pair(key.clone(), value.clone())),
                    )
                    .children(request.body.clone().map(|body| {
                        div()
                            .mt_2()
                            .p_2()
                            .rounded(theme.radius(Radius::Sm))
                            .bg(colors.sunken)
                            .font_family(theme.mono_family.clone())
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg)
                            .child(body)
                    }))
            });
        div()
            .flex()
            .flex_col()
            .gap_2()
            .text_size(theme.text_size(TextSize::Sm))
            .child(div().flex().flex_col().child(head).children(rows))
            .children(details)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_goes_by_the_last_part_of_its_path() {
        assert_eq!(name("https://api.ely.dev/v1/quotes?symbol=ELY"), "quotes");
        assert_eq!(name("https://ely.dev/"), "ely.dev");
        assert_eq!(name("/static/app.js#top"), "app.js");
    }
}
