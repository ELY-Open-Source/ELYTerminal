use std::rc::Rc;

use gpui::{
    App, ClipboardItem, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window,
    WindowHandle, div,
};

use super::{Hosted, open_hosted};
use crate::{
    buttons::{Button, ButtonVariant},
    layout::{Collapsible, ScrollArea},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::{Paragraph, Title},
};

type Action = Rc<dyn Fn(&mut Window, &mut App)>;

/// After a crash: what happened, the report, and a choice to send it.
#[derive(IntoElement, Clone)]
pub struct CrashReporter {
    name: SharedString,
    summary: SharedString,
    report: SharedString,
    details: bool,
    on_send: Option<Action>,
    on_dismiss: Option<Action>,
}

impl CrashReporter {
    pub fn new(
        name: impl Into<SharedString>,
        summary: impl Into<SharedString>,
        report: impl Into<SharedString>,
    ) -> Self {
        Self {
            name: name.into(),
            summary: summary.into(),
            report: report.into(),
            details: false,
            on_send: None,
            on_dismiss: None,
        }
    }

    /// Opens the report at first.
    pub fn details(mut self, open: bool) -> Self {
        self.details = open;
        self
    }

    pub fn on_send(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_send = Some(Rc::new(handler));
        self
    }

    pub fn on_dismiss(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(handler));
        self
    }

    pub fn open(self, cx: &mut App) -> anyhow::Result<WindowHandle<Hosted<CrashReporter>>> {
        let size = cx.theme().dialog_window();
        open_hosted(format!("{} crashed", self.name), size, self, cx)
    }
}

impl RenderOnce for CrashReporter {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let initial = self.details;
        let shown = window.use_keyed_state("crash-details", cx, |_, _| initial);
        let open = *shown.read(cx);
        let theme = cx.theme();
        let colors = &theme.colors;
        let action = |button: Button, action: Option<Action>| {
            action.map(|action| button.on_click(move |_, window, cx| action(window, cx)))
        };
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap_4()
            .px_8()
            .pb_6()
            .child(
                Icon::new(IconName::TriangleAlert)
                    .size(IconSize::Xl)
                    .color(colors.fg_muted),
            )
            .child(Title::new(format!("{} quit unexpectedly", self.name)))
            .child(Paragraph::new(self.summary))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        Button::new(
                            "crash-details",
                            if open { "Hide report" } else { "Show report" },
                        )
                        .variant(ButtonVariant::Link)
                        .trailing_icon(if open {
                            IconName::ChevronUp
                        } else {
                            IconName::ChevronDown
                        })
                        .on_click(move |_, _, cx| {
                            shown.update(cx, |open, cx| {
                                *open = !*open;
                                cx.notify();
                            })
                        }),
                    )
                    .child({
                        let report = self.report.clone();
                        Button::new("crash-copy", "Copy report")
                            .variant(ButtonVariant::Ghost)
                            .icon(IconName::Copy)
                            .on_click(move |_, _, cx| {
                                log::info!("crash reporter: report copied");
                                cx.write_to_clipboard(ClipboardItem::new_string(
                                    report.to_string(),
                                ));
                            })
                    }),
            )
            .child(
                Collapsible::new("crash-report", open).child(
                    div()
                        .h(theme.pane_min())
                        .rounded(theme.radius(Radius::Md))
                        .border_1()
                        .border_color(colors.border)
                        .bg(colors.sunken)
                        .font_family(theme.mono_family.clone())
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(colors.fg_muted)
                        .child(
                            ScrollArea::new("crash-report-scroll")
                                .size_full()
                                .child(div().p_3().child(self.report)),
                        ),
                ),
            )
            .child(div().flex_1())
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .children(action(
                        Button::new("crash-dismiss", "Don't send").variant(ButtonVariant::Ghost),
                        self.on_dismiss,
                    ))
                    .children(action(
                        Button::new("crash-send", "Send report").primary(),
                        self.on_send,
                    )),
            )
    }
}
