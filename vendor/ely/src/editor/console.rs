use std::rc::Rc;

use gpui::{
    App, ElementId, Entity, Hsla, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div,
};

use crate::{
    buttons::{ButtonVariant, IconButton, ToggleButton, ToggleItem},
    forms::{Choice, Enter, Input, Select, TextInput},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, Palette, Radius, TextSize},
};

/// The color a log line reads in, from the level it opens with.
pub(crate) fn tone(line: &str, colors: &Palette) -> Hsla {
    let lower = line.trim_start().to_ascii_lowercase();
    if lower.starts_with("error") || lower.starts_with("[error") {
        colors.danger
    } else if lower.starts_with("warn") || lower.starts_with("[warn") {
        colors.warning
    } else if lower.starts_with("debug") || lower.starts_with("[debug") {
        colors.fg_subtle
    } else {
        colors.fg_muted
    }
}

type OnText = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
type OnFlag = Rc<dyn Fn(bool, &mut Window, &mut App)>;
type OnRun = Rc<dyn Fn(&mut Window, &mut App)>;

/// A tool's log by channel: pick the channel, clear it, and keep the newest line in view or let it stay put.
#[derive(IntoElement)]
pub struct OutputPanel {
    id: ElementId,
    channels: Vec<SharedString>,
    channel: SharedString,
    lines: Vec<SharedString>,
    follow: bool,
    on_channel: Option<OnText>,
    on_clear: Option<OnRun>,
    on_follow: Option<OnFlag>,
}

impl OutputPanel {
    pub fn new(
        id: impl Into<ElementId>,
        channels: impl IntoIterator<Item = impl Into<SharedString>>,
        channel: impl Into<SharedString>,
        lines: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        let channels: Vec<SharedString> = channels.into_iter().map(Into::into).collect();
        let channel = channel.into();
        if !channels.contains(&channel) {
            log::error!("output panel: {channel} is not among {channels:?}; none chosen");
        }
        Self {
            id: id.into(),
            channels,
            channel,
            lines: lines.into_iter().map(Into::into).collect(),
            follow: true,
            on_channel: None,
            on_clear: None,
            on_follow: None,
        }
    }

    /// Whether the newest line stays in view.
    pub fn follow(mut self, follow: bool) -> Self {
        self.follow = follow;
        self
    }

    pub fn on_channel(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_channel = Some(Rc::new(handler));
        self
    }

    pub fn on_clear(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_clear = Some(Rc::new(handler));
        self
    }

    pub fn on_follow(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_follow = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for OutputPanel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let scroll = window.use_keyed_state((self.id.clone(), "scroll"), cx, |_, _| {
            gpui::ScrollHandle::new()
        });
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let scroll = scroll.read(cx).clone();
        if self.follow {
            scroll.scroll_to_bottom();
        }
        let channel = Select::new(
            (self.id.clone(), "channel"),
            self.channels
                .iter()
                .map(|name| Choice::new(name.clone(), name.clone())),
        )
        .selected(self.channel.clone())
        .size(ControlSize::Sm);
        let channel = match self.on_channel {
            Some(on_channel) => {
                channel.on_change(move |name, window, cx| on_channel(name, window, cx))
            }
            None => channel,
        };
        let follow = ToggleButton::new(
            (self.id.clone(), "follow"),
            ToggleItem::new("follow")
                .icon(IconName::ArrowDownWideNarrow)
                .tooltip("Keep the newest line in view"),
            self.follow,
        )
        .size(ControlSize::Sm);
        let follow = match self.on_follow {
            Some(on_follow) => follow.on_toggle(move |on, window, cx| on_follow(on, window, cx)),
            None => follow,
        };
        let lines = self
            .lines
            .iter()
            .map(|line| div().text_color(tone(line, &colors)).child(line.clone()));
        div()
            .flex()
            .flex_col()
            .gap_1()
            .size_full()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(div().w(theme.label_width() * 1.5).child(channel))
                    .child(div().flex_1())
                    .child(follow)
                    .children(self.on_clear.map(|clear| {
                        IconButton::new((self.id.clone(), "clear"), IconName::Eraser)
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Sm)
                            .tooltip("Clear")
                            .on_click(move |_, window, cx| clear(window, cx))
                    })),
            )
            .child(
                div()
                    .id((self.id.clone(), "lines"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&scroll)
                    .p_2()
                    .rounded(theme.radius(Radius::Md))
                    .bg(colors.sunken)
                    .font_family(theme.mono_family.clone())
                    .text_size(theme.text_size(TextSize::Xs))
                    .children(lines),
            )
    }
}

/// What a debug console shows: what was typed, what came back, a value with its type, or an error.
#[derive(Clone, Debug, PartialEq)]
pub enum ConsoleEntry {
    Input(SharedString),
    Output(SharedString),
    Value(SharedString, SharedString),
    Error(SharedString),
}

/// A read-eval-print prompt beside a paused program: entries above, the prompt below; Enter evaluates.
#[derive(IntoElement)]
pub struct DebugConsole {
    id: ElementId,
    entries: Vec<ConsoleEntry>,
    field: Entity<TextInput>,
    on_eval: Option<OnText>,
}

impl DebugConsole {
    pub fn new(
        id: impl Into<ElementId>,
        field: &Entity<TextInput>,
        entries: impl IntoIterator<Item = ConsoleEntry>,
    ) -> Self {
        Self {
            id: id.into(),
            entries: entries.into_iter().collect(),
            field: field.clone(),
            on_eval: None,
        }
    }

    pub fn on_eval(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_eval = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for DebugConsole {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let syntax = colors.syntax;
        let prompt = || {
            div()
                .flex_none()
                .w(theme.icon_size(IconSize::Sm))
                .text_color(colors.fg_subtle)
                .child("›")
        };
        let entries = self.entries.iter().map(|entry| {
            let row = div().flex().items_start().gap_1p5();
            match entry {
                ConsoleEntry::Input(text) => row
                    .child(prompt())
                    .child(div().text_color(colors.fg).child(text.clone())),
                ConsoleEntry::Output(text) => row
                    .pl(theme.icon_size(IconSize::Sm))
                    .child(div().text_color(colors.fg_muted).child(text.clone())),
                ConsoleEntry::Value(text, kind) => {
                    let ink = match kind.as_ref() {
                        "string" | "&str" | "String" => syntax.string,
                        "bool" => syntax.constant,
                        _ if kind.starts_with(['i', 'u', 'f']) => syntax.number,
                        _ => syntax.variable,
                    };
                    row.pl(theme.icon_size(IconSize::Sm))
                        .child(div().text_color(ink).child(text.clone()))
                        .child(div().text_color(colors.fg_subtle).child(kind.clone()))
                }
                ConsoleEntry::Error(text) => row
                    .child(
                        Icon::new(IconName::CircleX)
                            .size(IconSize::Xs)
                            .color(colors.danger),
                    )
                    .child(div().text_color(colors.danger).child(text.clone())),
            }
        });
        let (field, eval) = (self.field.clone(), self.on_eval);
        div()
            .id(self.id)
            .flex()
            .flex_col()
            .gap_1()
            .p_2()
            .rounded(theme.radius(Radius::Md))
            .bg(colors.sunken)
            .font_family(theme.mono_family.clone())
            .text_size(theme.text_size(TextSize::Xs))
            .children(entries)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .capture_action(move |_: &Enter, window, cx| {
                        cx.stop_propagation();
                        let text = field.read(cx).text().trim().to_string();
                        if text.is_empty() {
                            return;
                        }
                        log::info!("debug console: {text}");
                        field.update(cx, |field, cx| field.set_text("", cx));
                        if let Some(eval) = &eval {
                            eval(&text.into(), window, cx);
                        }
                    })
                    .child(prompt())
                    .child(
                        div()
                            .flex_1()
                            .child(Input::new(&self.field).size(ControlSize::Sm)),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_take_their_levels_color() {
        let colors = Palette::light(false);
        assert_eq!(tone("ERROR: build failed", &colors), colors.danger);
        assert_eq!(tone("  warning: unused import", &colors), colors.warning);
        assert_eq!(tone("[debug] tick", &colors), colors.fg_subtle);
        assert_eq!(tone("Compiling ely v0.1.0", &colors), colors.fg_muted);
    }
}
