use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
};

use super::section::{SettingsRow, SettingsSection};
use crate::{
    canvas::{OnEdit, editing},
    forms::{Checkbox, Choice, Select, Switch},
    theme::{ActiveTheme, TextSize},
    typography::Ellipsis,
};

/// What an app may share and keep.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Privacy {
    pub telemetry: bool,
    pub crash_reports: bool,
    pub personalization: bool,
    pub history: bool,
}

/// What an app may share and keep, a switch for each with what it means.
#[derive(IntoElement)]
pub struct PrivacySettings {
    id: ElementId,
    privacy: Privacy,
    on_change: OnEdit<Privacy>,
}

impl PrivacySettings {
    pub fn new(
        id: impl Into<ElementId>,
        privacy: Privacy,
        on_change: impl Fn(Privacy, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            privacy,
            on_change: Rc::new(on_change),
        }
    }
}

impl RenderOnce for PrivacySettings {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let on_change = self.on_change;
        let (id, privacy, on_change) = (self.id, self.privacy, &on_change);
        let row = |key: &'static str,
                   title: &'static str,
                   text: &'static str,
                   on: bool,
                   set: fn(&mut Privacy, bool)| {
            SettingsRow::new(title).description(text).control(
                Switch::new((id.clone(), key), on)
                    .on_change(editing("privacy", &privacy, on_change, set)),
            )
        };
        SettingsSection::new("Privacy")
            .description("What this app shares and keeps.")
            .row(row(
                "telemetry",
                "Usage data",
                "Counts of features used, without content.",
                privacy.telemetry,
                |p, on| p.telemetry = on,
            ))
            .row(row(
                "crashes",
                "Crash reports",
                "What went wrong when the app quits on its own.",
                privacy.crash_reports,
                |p, on| p.crash_reports = on,
            ))
            .row(row(
                "personal",
                "Personal suggestions",
                "Suggestions drawn from what you do here.",
                privacy.personalization,
                |p, on| p.personalization = on,
            ))
            .row(row(
                "history",
                "Keep history",
                "Recent files and searches, on this device only.",
                privacy.history,
                |p, on| p.history = on,
            ))
    }
}

/// Which channels each kind of notice arrives by, and whether notices wait.
#[derive(Clone, Debug, PartialEq)]
pub struct Notices {
    pub channels: Vec<SharedString>,
    pub kinds: Vec<(SharedString, SharedString)>,
    /// The (kind, channel) pairs that are on.
    pub on: Vec<(SharedString, SharedString)>,
    pub quiet: bool,
}

/// Notices by kind: a row for each, a named box for each channel it may arrive by that folds under the name when narrow, and a switch that holds every notice back.
#[derive(IntoElement)]
pub struct NotificationSettings {
    id: ElementId,
    notices: Notices,
    on_change: OnEdit<Notices>,
}

impl NotificationSettings {
    pub fn new(
        id: impl Into<ElementId>,
        notices: Notices,
        on_change: impl Fn(Notices, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            notices,
            on_change: Rc::new(on_change),
        }
    }
}

impl RenderOnce for NotificationSettings {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_change = self.on_change;
        let (id, notices, on_change) = (self.id, self.notices, &on_change);
        let theme = cx.theme();
        let rows = notices.kinds.iter().map(|(kind, name)| {
            let boxes = notices.channels.iter().map(|channel| {
                let pair = (kind.clone(), channel.clone());
                let on = notices.on.contains(&pair);
                let set = editing(
                    "notifications",
                    &notices,
                    on_change,
                    move |notices: &mut Notices, on: bool| {
                        notices.on.retain(|each| *each != pair);
                        if on {
                            notices.on.push(pair.clone());
                        }
                    },
                );
                Checkbox::new((id.clone(), format!("{kind}-{channel}")), on)
                    .label(channel.clone())
                    .on_change(set)
            });
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_x_4()
                .gap_y_1()
                .py_2()
                .border_b_1()
                .border_color(theme.colors.border)
                .child(
                    div()
                        .flex_1()
                        .min_w(theme.label_width())
                        .text_size(theme.text_size(TextSize::Sm))
                        .child(Ellipsis::new(name.clone())),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .flex_none()
                        .gap_x_3()
                        .children(boxes),
                )
        });
        let quiet = editing(
            "notifications",
            &notices,
            on_change,
            |notices: &mut Notices, on: bool| notices.quiet = on,
        );
        SettingsSection::new("Notifications")
            .description("Which notices reach you, and where.")
            .row(
                SettingsRow::new("Do not disturb")
                    .description("Hold every notice back until it is off.")
                    .control(Switch::new((id.clone(), "quiet"), notices.quiet).on_change(quiet)),
            )
            .row(div().pt_3().children(rows))
    }
}

/// What an app does when it starts.
#[derive(Clone, Debug, PartialEq)]
pub struct Startup {
    pub at_login: bool,
    pub restore: bool,
    pub opens: SharedString,
    pub updates: bool,
}

/// What an app does when it starts: whether it opens at login, restores its windows and looks for updates, and what it opens to, from `pages`.
#[derive(IntoElement)]
pub struct StartupSettings {
    id: ElementId,
    startup: Startup,
    pages: Vec<Choice>,
    on_change: OnEdit<Startup>,
}

impl StartupSettings {
    pub fn new(
        id: impl Into<ElementId>,
        startup: Startup,
        pages: impl IntoIterator<Item = Choice>,
        on_change: impl Fn(Startup, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            startup,
            pages: pages.into_iter().collect(),
            on_change: Rc::new(on_change),
        }
    }
}

impl RenderOnce for StartupSettings {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_change = self.on_change;
        let (id, startup, on_change) = (self.id, self.startup, &on_change);
        let switch = |key: &'static str, on: bool, set: fn(&mut Startup, bool)| {
            Switch::new((id.clone(), key), on)
                .on_change(editing("startup", &startup, on_change, set))
        };
        let open = editing(
            "startup",
            &startup,
            on_change,
            |startup: &mut Startup, page: SharedString| startup.opens = page,
        );
        SettingsSection::new("Startup")
            .row(SettingsRow::new("Open at login").control(switch(
                "login",
                startup.at_login,
                |s, on| s.at_login = on,
            )))
            .row(
                SettingsRow::new("Restore windows")
                    .description("Reopen what was open when the app quit.")
                    .control(switch("restore", startup.restore, |s, on| s.restore = on)),
            )
            .row(
                SettingsRow::new("Opens to").control(
                    div().w(cx.theme().label_width()).child(
                        Select::new((id.clone(), "opens"), self.pages)
                            .selected(startup.opens.clone())
                            .on_change(move |page, window, cx| open(page.clone(), window, cx)),
                    ),
                ),
            )
            .row(SettingsRow::new("Look for updates").control(switch(
                "updates",
                startup.updates,
                |s, on| s.updates = on,
            )))
    }
}
