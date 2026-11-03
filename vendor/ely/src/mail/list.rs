use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};
use jiff::{Timestamp, tz::TimeZone};

use crate::{
    forms::{OnFlag, OnValues},
    lists::{ListItem, SelectableList},
    primitives::{FocusRing, Icon, IconName, Tooltip},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::format,
};

type OnStar = Rc<dyn Fn(&SharedString, bool, &mut Window, &mut App)>;

/// A message as a list shows it: who sent it, its subject and first words, when it came, and its flags.
#[derive(Clone, Debug, PartialEq)]
pub struct Mail {
    pub key: SharedString,
    pub from: SharedString,
    pub subject: SharedString,
    pub snippet: Option<SharedString>,
    pub at: Timestamp,
    pub unread: bool,
    pub starred: bool,
    pub attachments: usize,
}

impl Mail {
    pub fn new(
        key: impl Into<SharedString>,
        from: impl Into<SharedString>,
        subject: impl Into<SharedString>,
        at: Timestamp,
    ) -> Self {
        Self {
            key: key.into(),
            from: from.into(),
            subject: subject.into(),
            snippet: None,
            at,
            unread: false,
            starred: false,
            attachments: 0,
        }
    }

    /// Its first words.
    pub fn snippet(mut self, text: impl Into<SharedString>) -> Self {
        self.snippet = Some(text.into());
        self
    }

    pub fn unread(mut self, unread: bool) -> Self {
        self.unread = unread;
        self
    }

    pub fn starred(mut self, starred: bool) -> Self {
        self.starred = starred;
        self
    }

    /// How many files it carries.
    pub fn attachments(mut self, count: usize) -> Self {
        self.attachments = count;
        self
    }
}

/// When a message came, as a list says it: the time today, the weekday within the week, the date within the year, and the year past that.
fn when(at: Timestamp, now: Timestamp, zone: &TimeZone) -> String {
    let (at, now) = (at.to_zoned(zone.clone()), now.to_zoned(zone.clone()));
    let days = at
        .date()
        .until(now.date())
        .expect("two dates have a span")
        .get_days();
    let pattern = match days {
        0 => "%H:%M",
        1..=6 => "%a",
        _ if at.year() == now.year() => "%b %-d",
        _ => "%b %-d, %Y",
    };
    at.strftime(pattern).to_string()
}

/// A message as a row: its sender, strong beside a dot while unread, the subject and first words under it, when it came, a paperclip when it carries files, and its star. With a handler the star toggles, its press kept off the row.
#[derive(IntoElement)]
pub struct MailItem {
    id: ElementId,
    mail: Mail,
    zone: Option<TimeZone>,
    on_star: Option<OnFlag>,
}

impl MailItem {
    pub fn new(id: impl Into<ElementId>, mail: Mail) -> Self {
        Self {
            id: id.into(),
            mail,
            zone: None,
            on_star: None,
        }
    }

    /// The zone its date reads in; the system's otherwise.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    /// Gets the star's new state.
    pub fn on_star(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_star = Some(Rc::new(handler));
        self
    }

    fn item(self, cx: &App) -> ListItem {
        let theme = cx.theme();
        let colors = &theme.colors;
        let mail = self.mail;
        let zone = self
            .zone
            .unwrap_or_else(|| format::system_zone("mail item"));
        let date = when(mail.at, Timestamp::now(), &zone);
        let dot = div()
            .size(theme.icon_size(IconSize::Xs))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .size(theme.status_dot())
                    .rounded_full()
                    .when(mail.unread, |dot| dot.bg(colors.accent)),
            );
        let starred = mail.starred;
        let tint = if starred {
            colors.warning
        } else {
            colors.fg_subtle
        };
        let mark = |icon| Icon::new(icon).size(IconSize::Sm).color(tint);
        let star: Option<AnyElement> = match self.on_star {
            Some(set) => {
                let key = mail.key.clone();
                let named = key.clone();
                Some(
                    div()
                        .id((self.id.clone(), "star"))
                        .debug_selector(move || format!("star {named}"))
                        .p_0p5()
                        .rounded(theme.radius(Radius::Sm))
                        .border_1()
                        .border_color(gpui::transparent_black())
                        .tab_index(0)
                        .focus_ring(cx)
                        .cursor_pointer()
                        .hover(|style| style.bg(colors.hover))
                        .tooltip(Tooltip::text(if starred { "Unstar" } else { "Star" }))
                        .on_mouse_down(MouseButton::Left, |_, window, cx| {
                            window.prevent_default();
                            cx.stop_propagation();
                        })
                        .on_key_down(|event, _, cx| {
                            let key = event.keystroke.key.as_str();
                            if matches!(key, "space" | "enter")
                                && !event.keystroke.modifiers.modified()
                            {
                                cx.stop_propagation();
                            }
                        })
                        .on_click(move |_, window, cx| {
                            log::info!("mail {key}: star {}", !starred);
                            set(!starred, window, cx)
                        })
                        .child(mark(IconName::Star))
                        .into_any_element(),
                )
            }
            None => starred.then(|| mark(IconName::Star).into_any_element()),
        };
        let clip = (mail.attachments > 0).then(|| {
            Icon::new(IconName::Paperclip)
                .size(IconSize::Sm)
                .color(colors.fg_subtle)
        });
        let trail = div()
            .flex()
            .flex_col()
            .items_end()
            .gap_1()
            .child(div().text_size(theme.text_size(TextSize::Xs)).child(date))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .children(clip)
                    .children(star),
            );
        let row = ListItem::new(self.id, mail.from)
            .leading(dot)
            .strong(mail.unread)
            .description(mail.subject)
            .trailing(trail);
        match mail.snippet {
            Some(snippet) => row.detail(snippet),
            None => row,
        }
    }
}

impl RenderOnce for MailItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        self.item(cx)
    }
}

/// Messages in a list, in the order given: a press or an arrow selects one, and Command or Shift with a press takes several; Space toggles. Unread ones stand out.
#[derive(IntoElement)]
pub struct MailList {
    id: ElementId,
    mails: Vec<Mail>,
    selected: Vec<SharedString>,
    zone: Option<TimeZone>,
    on_select: Option<OnValues>,
    on_star: Option<OnStar>,
}

impl MailList {
    pub fn new(id: impl Into<ElementId>, mails: impl IntoIterator<Item = Mail>) -> Self {
        let mails: Vec<Mail> = mails.into_iter().collect();
        for (ix, mail) in mails.iter().enumerate() {
            let twice = mails[..ix].iter().any(|other| other.key == mail.key);
            assert!(!twice, "mail {} twice", mail.key);
        }
        Self {
            id: id.into(),
            mails,
            selected: Vec::new(),
            zone: None,
            on_select: None,
            on_star: None,
        }
    }

    /// The messages selected now, by key.
    pub fn selected(mut self, keys: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.selected = keys.into_iter().map(Into::into).collect();
        self
    }

    /// The zone dates read in; the system's otherwise.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    /// Gets the keys selected, in list order.
    pub fn on_select(
        mut self,
        handler: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Gets a message's key and its star's new state.
    pub fn on_star(
        mut self,
        handler: impl Fn(&SharedString, bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_star = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for MailList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        for key in &self.selected {
            let listed = self.mails.iter().any(|mail| mail.key == *key);
            if !listed {
                log::error!("mail list: no mail {key}; it is not selected");
            }
        }
        let theme = cx.theme();
        if self.mails.is_empty() {
            return div()
                .debug_selector(|| "mail-none".into())
                .px_3()
                .py_4()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(theme.colors.fg_muted)
                .child("No mail here.")
                .into_any_element();
        }
        let zone = self
            .zone
            .unwrap_or_else(|| format::system_zone("mail list"));
        let list = self.mails.into_iter().fold(
            SelectableList::new((self.id.clone(), "mails")).multiple(),
            |list, mail| {
                let key = mail.key.clone();
                let item = MailItem::new((self.id.clone(), format!("mail-{key}")), mail)
                    .zone(zone.clone());
                let item = match self.on_star.clone() {
                    Some(star) => {
                        let starred = key.clone();
                        item.on_star(move |on, window, cx| star(&starred, on, window, cx))
                    }
                    None => item,
                };
                list.row(key, item.item(cx))
            },
        );
        let (id, on_select) = (self.id.clone(), self.on_select);
        div()
            .debug_selector(|| "mail-list".into())
            .child(
                list.selected(self.selected)
                    .on_change(move |keys, window, cx| {
                        log::info!("mail list {id}: select {keys:?}");
                        if let Some(on_select) = &on_select {
                            on_select(keys, window, cx);
                        }
                    }),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;

    #[test]
    fn a_date_reads_as_the_time_the_weekday_the_date_or_the_year() {
        let zone = TimeZone::UTC;
        let at = |y, m, d, h| {
            date(y, m, d)
                .at(h, 5, 0, 0)
                .to_zoned(zone.clone())
                .expect("a UTC time")
                .timestamp()
        };
        let now = at(2026, 9, 26, 12);
        assert_eq!(when(at(2026, 9, 26, 9), now, &zone), "09:05");
        assert_eq!(when(at(2026, 9, 25, 23), now, &zone), "Fri");
        assert_eq!(when(at(2026, 9, 20, 9), now, &zone), "Sun");
        assert_eq!(when(at(2026, 9, 19, 9), now, &zone), "Sep 19");
        assert_eq!(when(at(2025, 12, 31, 9), now, &zone), "Dec 31, 2025");
    }
}
