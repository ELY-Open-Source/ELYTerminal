use std::rc::Rc;

use gpui::{
    AnyElement, App, Context, ElementId, IntoElement, ListAlignment, ListOffset, ListState,
    ParentElement, Pixels, RenderOnce, SharedString, Styled, Window, canvas, div, list, prelude::*,
};
use jiff::civil::Date;

use crate::{
    buttons::{Button, ButtonVariant},
    forms::Run,
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, Elevation, TextSize},
};

type RenderItem = Rc<dyn Fn(usize, &mut Window, &mut App) -> AnyElement>;

/// A list's place: its gpui list, how many messages it knew, whether the reader scrolled away from the newest, and how many came since.
struct Feed {
    list: ListState,
    count: usize,
    away: bool,
    fresh: usize,
}

/// How many messages arrived while the reader was away, from `before` to `now`; none once they are back.
pub(crate) fn arrived(before: usize, now: usize, away: bool, fresh: usize) -> usize {
    match (away, now >= before) {
        (true, true) => fresh + (now - before),
        _ => 0,
    }
}

/// Messages down a column drawn only near the view, held at the newest: scrolled up, it stays put as messages arrive and offers a way back down with how many are new. Give each conversation its own id.
#[derive(IntoElement)]
pub struct MessageList {
    id: ElementId,
    count: usize,
    render: RenderItem,
}

impl MessageList {
    /// `render` draws message `ix` of `count`.
    pub fn new(
        id: impl Into<ElementId>,
        count: usize,
        render: impl Fn(usize, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            count,
            render: Rc::new(render),
        }
    }
}

impl RenderOnce for MessageList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let count = self.count;
        let overdraw = cx.theme().list_overdraw().to_pixels(window.rem_size());
        let feed = window.use_keyed_state(
            (self.id.clone(), "feed"),
            cx,
            move |_, cx: &mut Context<Feed>| {
                let list = ListState::new(count, ListAlignment::Bottom, overdraw);
                let owner = cx.entity().downgrade();
                list.set_scroll_handler(move |event, _, cx| {
                    let away = event.is_scrolled;
                    if let Err(error) = owner.update(cx, |feed, cx| {
                        if feed.away != away {
                            log::info!("message list: away {away}");
                            feed.away = away;
                            feed.fresh = 0;
                            cx.notify();
                        }
                    }) {
                        log::error!("message list: its place is gone: {error:#}");
                    }
                });
                Feed {
                    list,
                    count,
                    away: false,
                    fresh: 0,
                }
            },
        );
        if feed.read(cx).count != count {
            feed.update(cx, |feed, _| {
                if count >= feed.count {
                    feed.list.splice(feed.count..feed.count, count - feed.count);
                } else {
                    feed.list.splice(count..feed.count, 0);
                }
                feed.fresh = arrived(feed.count, count, feed.away, feed.fresh);
                feed.count = count;
            });
        }
        let (state, away, fresh) = {
            let feed = feed.read(cx);
            (feed.list.clone(), feed.away, feed.fresh)
        };
        let render = self.render;
        let (down, settled) = (feed.clone(), feed.clone());
        div()
            .relative()
            .size_full()
            // gpui's list lays rows at content width.
            .child(
                list(state, move |ix, window, cx| {
                    div()
                        .w_full()
                        .child(render(ix, window, cx))
                        .into_any_element()
                })
                .size_full(),
            )
            .child(
                canvas(
                    move |_, window, cx| {
                        let feed = settled.read(cx);
                        let held = feed.list.logical_scroll_top().item_ix >= feed.list.item_count();
                        if feed.away && held {
                            log::info!("message list: back at the newest");
                            settled.update(cx, |feed, cx| {
                                feed.away = false;
                                feed.fresh = 0;
                                cx.notify();
                            });
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .when(away, |view| {
                view.child(
                    div()
                        .absolute()
                        .bottom_4()
                        .left_0()
                        .right_0()
                        .flex()
                        .justify_center()
                        .child(ScrollToBottomButton::new(
                            (self.id.clone(), "down"),
                            fresh,
                            move |_, cx| {
                                down.update(cx, |feed, cx| {
                                    log::info!("message list: back to the newest");
                                    let end = feed.list.item_count();
                                    feed.list.scroll_to(ListOffset {
                                        item_ix: end,
                                        offset_in_item: Pixels::ZERO,
                                    });
                                    feed.away = false;
                                    feed.fresh = 0;
                                    cx.notify();
                                })
                            },
                        )),
                )
            })
    }
}

/// A way back to the newest message, floating over the list, with how many came in while away.
#[derive(IntoElement)]
pub struct ScrollToBottomButton {
    id: ElementId,
    fresh: usize,
    on_press: Run,
}

impl ScrollToBottomButton {
    pub fn new(
        id: impl Into<ElementId>,
        fresh: usize,
        on_press: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            fresh,
            on_press: Rc::new(on_press),
        }
    }
}

impl RenderOnce for ScrollToBottomButton {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let label: SharedString = match self.fresh {
            0 => "Latest".into(),
            1 => "1 new message".into(),
            n => format!("{n} new messages").into(),
        };
        let press = self.on_press;
        div()
            .rounded_full()
            .shadow(cx.theme().elevation(Elevation::Floating))
            .child(
                Button::new(self.id, label)
                    .variant(ButtonVariant::Secondary)
                    .size(ControlSize::Sm)
                    .icon(IconName::ArrowDown)
                    .on_click(move |_, window, cx| press(window, cx)),
            )
    }
}

/// A day's line between messages: Today, Yesterday, or the date.
#[derive(IntoElement)]
pub struct DateSeparator {
    day: Date,
    today: Date,
}

impl DateSeparator {
    /// `today` names the reader's day, so tests and captures hold still.
    pub fn new(day: Date, today: Date) -> Self {
        Self { day, today }
    }
}

/// The words for `day` seen from `today`.
pub(crate) fn day_name(day: Date, today: Date) -> String {
    if day == today {
        "Today".to_string()
    } else if today.yesterday().is_ok_and(|yesterday| yesterday == day) {
        "Yesterday".to_string()
    } else if day.year() == today.year() {
        day.strftime("%A, %B %-d").to_string()
    } else {
        day.strftime("%B %-d, %Y").to_string()
    }
}

impl RenderOnce for DateSeparator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let rule = || div().flex_1().h_px().bg(colors.border);
        let name = day_name(self.day, self.today);
        let shown = format!("date-separator-{name}");
        div()
            .debug_selector(move || shown)
            .flex()
            .items_center()
            .gap_3()
            .py_2()
            .child(rule())
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.fg_subtle)
                    .child(name),
            )
            .child(rule())
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;

    #[test]
    fn days_read_as_today_yesterday_or_a_date() {
        let today = date(2026, 9, 26);
        assert_eq!(day_name(today, today), "Today");
        assert_eq!(day_name(date(2026, 9, 25), today), "Yesterday");
        assert_eq!(day_name(date(2026, 9, 1), today), "Tuesday, September 1");
        assert_eq!(day_name(date(2025, 12, 31), today), "December 31, 2025");
    }

    #[test]
    fn new_messages_count_only_while_away() {
        assert_eq!(arrived(3, 5, true, 0), 2);
        assert_eq!(arrived(5, 6, true, 2), 3);
        assert_eq!(
            arrived(5, 6, false, 0),
            0,
            "at the newest, nothing to count"
        );
        assert_eq!(
            arrived(6, 2, true, 3),
            0,
            "fewer messages start the count over"
        );
    }
}
