use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div,
};

use super::json_editor::reformat;
use crate::{
    data_display::{Badge, Tone},
    forms::Choice,
    navigation::Tabs,
    tables::Table,
    theme::{ActiveTheme, Radius, TextSize},
    typography::format,
};

/// What came back: its status, how long it took in milliseconds, its size in bytes, its headers and its body.
#[derive(Clone, Debug, PartialEq)]
pub struct ApiResponse {
    pub status: u16,
    pub took_ms: u32,
    pub size: u64,
    pub headers: Vec<(SharedString, SharedString)>,
    pub body: SharedString,
}

/// A status's standard words.
pub fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        422 => "Unprocessable Content",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "",
    }
}

/// The tone a status class wears: success, a redirect, the client's fault, or the server's.
pub fn status_tone(status: u16) -> Tone {
    match status {
        200..=299 => Tone::Success,
        300..=399 => Tone::Info,
        400..=499 => Tone::Warning,
        _ => Tone::Danger,
    }
}

/// Headers in a table, names down the left and values beside them.
#[derive(IntoElement)]
pub struct HeadersTable {
    headers: Vec<(SharedString, SharedString)>,
}

impl HeadersTable {
    pub fn new(
        headers: impl IntoIterator<Item = (impl Into<SharedString>, impl Into<SharedString>)>,
    ) -> Self {
        Self {
            headers: headers
                .into_iter()
                .map(|(name, value)| (name.into(), value.into()))
                .collect(),
        }
    }
}

impl RenderOnce for HeadersTable {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.headers
            .into_iter()
            .fold(Table::new(["Header", "Value"]), |table, (name, value)| {
                table.row([name, value])
            })
    }
}

/// A body as the viewer shows it: JSON indented, anything else as it came.
fn shown_body(body: &str) -> String {
    reformat(body, true).unwrap_or_else(|| body.to_string())
}

/// An answer to a request: its status in its tone with the standard words, how long it took and how big it is; then its body, JSON indented, or its headers.
#[derive(IntoElement)]
pub struct ResponseViewer {
    id: ElementId,
    response: ApiResponse,
}

impl ResponseViewer {
    pub fn new(id: impl Into<ElementId>, response: ApiResponse) -> Self {
        Self {
            id: id.into(),
            response,
        }
    }
}

impl RenderOnce for ResponseViewer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let response = self.response;
        let tab = window.use_keyed_state((self.id.clone(), "tab"), cx, |_, _| {
            SharedString::from("body")
        });
        let shown = tab.read(cx).clone();
        let theme = cx.theme();
        let body = shown_body(&response.body);
        let panel = match shown.as_ref() {
            "headers" => HeadersTable::new(response.headers.clone()).into_any_element(),
            _ => div()
                .id((self.id.clone(), "body"))
                .max_h(theme.list_max_height())
                .overflow_y_scroll()
                .p_3()
                .rounded(theme.radius(Radius::Md))
                .bg(theme.colors.sunken)
                .font_family(theme.mono_family.clone())
                .text_size(theme.text_size(TextSize::Sm))
                .child(body)
                .into_any_element(),
        };
        let quiet = |text: String| {
            div()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(theme.colors.fg_muted)
                .child(text)
        };
        let tabs = Tabs::new(
            (self.id, "tabs"),
            [
                Choice::new("body", "Body"),
                Choice::new("headers", format!("Headers {}", response.headers.len())),
            ],
            shown,
        )
        .panel(panel)
        .on_change(move |next, _, cx| {
            log::info!("response viewer: {next}");
            tab.update(cx, |tab, cx| {
                *tab = next.clone();
                cx.notify();
            })
        });
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_3()
                    .child(
                        Badge::new(format!("{} {}", response.status, reason(response.status)))
                            .tone(status_tone(response.status)),
                    )
                    .child(quiet(format!("{} ms", response.took_ms)))
                    .child(quiet(format::file_size(response.size, false))),
            )
            .child(tabs)
    }
}

#[cfg(test)]
mod tests {
    use super::shown_body;
    use super::{reason, status_tone};
    use crate::data_display::Tone;

    #[test]
    fn a_status_takes_its_words_and_its_class_tone() {
        assert_eq!(
            (reason(404), status_tone(404)),
            ("Not Found", Tone::Warning)
        );
        assert_eq!(status_tone(201), Tone::Success);
        assert_eq!(status_tone(503), Tone::Danger);
        assert_eq!(reason(299), "");
    }

    #[test]
    fn a_json_body_shows_indented_and_other_text_as_it_came() {
        assert_eq!(shown_body(r#"{"a":[1]}"#), "{\n  \"a\": [\n    1\n  ]\n}");
        assert_eq!(shown_body("plain words"), "plain words");
    }
}
