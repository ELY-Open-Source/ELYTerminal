use std::{rc::Rc, time::Duration};

use gpui::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::*,
};

use super::scrubber::OnTime;
use crate::{
    buttons::{Button, ButtonVariant, IconButton},
    forms::InlineEdit,
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
};

type OnCues = Rc<dyn Fn(&[Cue], &mut Window, &mut App)>;

/// How long a cue added at the playhead lasts.
const FRESH: Duration = Duration::from_secs(2);

/// A caption: its key, when it shows and when it goes, and its words.
#[derive(Clone, Debug, PartialEq)]
pub struct Cue {
    pub key: SharedString,
    pub start: Duration,
    pub end: Duration,
    pub text: SharedString,
}

/// A time read from `mm:ss.mmm` or `h:mm:ss.mmm`, its fraction optional; none when it does not read or no clock holds it.
pub(crate) fn parse_time(text: &str) -> Option<Duration> {
    let text = text.trim();
    let (clock, fraction) = text.split_once('.').unwrap_or((text, ""));
    let parts: Vec<u64> = clock
        .split(':')
        .map(|part| part.parse().ok())
        .collect::<Option<_>>()?;
    let seconds = match parts.as_slice() {
        [minutes, seconds] if *seconds < 60 => minutes.checked_mul(60)?.checked_add(*seconds)?,
        [hours, minutes, seconds] if *minutes < 60 && *seconds < 60 => hours
            .checked_mul(3600)?
            .checked_add(minutes * 60 + seconds)?,
        _ => return None,
    };
    let millis = match fraction.len() {
        0 => 0,
        1..=3 if fraction.chars().all(|ch| ch.is_ascii_digit()) => {
            format!("{fraction:0<3}").parse().ok()?
        }
        _ => return None,
    };
    Some(Duration::from_millis(
        seconds.checked_mul(1000)?.checked_add(millis)?,
    ))
}

/// A time as `mm:ss.mmm`, the hours ahead once it runs past one.
pub(crate) fn stamp(time: Duration) -> String {
    let (millis, seconds) = (time.subsec_millis(), time.as_secs());
    match seconds / 3600 {
        0 => format!("{:02}:{:02}.{millis:03}", seconds / 60, seconds % 60),
        hours => format!(
            "{hours}:{:02}:{:02}.{millis:03}",
            seconds / 60 % 60,
            seconds % 60
        ),
    }
}

/// Which time of a cue an edit changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Edge {
    Start,
    End,
}

/// The cues after `key`'s `edge` reads `text`, in time order, or why they stay as they were.
pub(crate) fn retimed(cues: &[Cue], key: &str, edge: Edge, text: &str) -> Result<Vec<Cue>, String> {
    let time = parse_time(text).ok_or_else(|| {
        format!(
            "\u{201c}{}\u{201d} is not a time; write mm:ss.mmm",
            text.trim()
        )
    })?;
    let mut next = cues.to_vec();
    let cue = next
        .iter_mut()
        .find(|cue| cue.key == key)
        .expect("a listed cue");
    match edge {
        Edge::Start => cue.start = time,
        Edge::End => cue.end = time,
    }
    if cue.start >= cue.end {
        return Err("A cue must end after it starts".to_string());
    }
    next.sort_by_key(|cue| cue.start);
    Ok(next)
}

/// Captions to edit, a cue a block in time order: its start and end as mm:ss.mmm over its words, each edited in place, with seek and remove beside. The cue under the playhead is lit; Add puts a two-second cue there. A time that does not read, or ends before it starts, leaves the cue as it was and says why under it.
#[derive(IntoElement)]
pub struct SubtitleEditor {
    id: ElementId,
    cues: Vec<Cue>,
    at: Duration,
    on_change: Option<OnCues>,
    on_seek: Option<OnTime>,
}

impl SubtitleEditor {
    /// `cues` in time order, each ending after it starts; `at` is the playhead.
    pub fn new(
        id: impl Into<ElementId>,
        cues: impl IntoIterator<Item = Cue>,
        at: Duration,
    ) -> Self {
        let cues: Vec<Cue> = cues.into_iter().collect();
        for (ix, cue) in cues.iter().enumerate() {
            assert!(cue.start < cue.end, "cue {} ends before it starts", cue.key);
            assert!(
                ix == 0 || cues[ix - 1].start <= cue.start,
                "cue {} out of order",
                cue.key
            );
            assert!(
                !cues[..ix].iter().any(|other| other.key == cue.key),
                "cue {} twice",
                cue.key
            );
        }
        Self {
            id: id.into(),
            cues,
            at,
            on_change: None,
            on_seek: None,
        }
    }

    pub fn on_change(mut self, handler: impl Fn(&[Cue], &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    pub fn on_seek(mut self, handler: impl Fn(Duration, &mut Window, &mut App) + 'static) -> Self {
        self.on_seek = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SubtitleEditor {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let wrong = window.use_keyed_state((self.id.clone(), "wrong"), cx, |_, _| {
            None::<(SharedString, String)>
        });
        let complaint = wrong.read(cx).clone();
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let cues = Rc::new(self.cues);
        let at = self.at;
        let rows = cues.iter().map(|cue| {
            let lit = cue.start <= at && at < cue.end;
            let key = cue.key.clone();
            let retime = |edge: Edge| {
                let (cues, key, wrong, change) = (
                    cues.clone(),
                    key.clone(),
                    wrong.clone(),
                    self.on_change.clone(),
                );
                move |text: &SharedString, window: &mut Window, cx: &mut App| match retimed(
                    &cues, &key, edge, text,
                ) {
                    Ok(next) => {
                        wrong.update(cx, |wrong, cx| {
                            *wrong = None;
                            cx.notify();
                        });
                        log::info!("subtitle editor: {key} {edge:?} at {}", text.trim());
                        if let Some(change) = &change {
                            change(&next, window, cx);
                        }
                    }
                    Err(reason) => {
                        log::info!("subtitle editor: {key} kept: {reason}");
                        wrong.update(cx, |wrong, cx| {
                            *wrong = Some((key.clone(), reason));
                            cx.notify();
                        });
                    }
                }
            };
            let words = {
                let (cues, key, change) = (cues.clone(), key.clone(), self.on_change.clone());
                move |text: &SharedString, window: &mut Window, cx: &mut App| {
                    let next: Vec<Cue> = cues
                        .iter()
                        .map(|cue| {
                            if cue.key == key {
                                Cue {
                                    text: text.clone(),
                                    ..cue.clone()
                                }
                            } else {
                                cue.clone()
                            }
                        })
                        .collect();
                    if let Some(change) = &change {
                        change(&next, window, cx);
                    }
                }
            };
            let editable = self.on_change.is_some();
            let time = |edge: Edge, value: Duration| {
                let field =
                    InlineEdit::new((self.id.clone(), format!("{key}-{edge:?}")), stamp(value));
                if editable {
                    field.on_commit(retime(edge))
                } else {
                    field
                }
            };
            let seek = self.on_seek.clone().map(|seek| {
                let start = cue.start;
                IconButton::new((self.id.clone(), format!("{key}-seek")), IconName::Play)
                    .size(ControlSize::Sm)
                    .tooltip("Play from here")
                    .on_click(move |_, window, cx| seek(start, window, cx))
            });
            let remove = self.on_change.clone().map(|change| {
                let (cues, key) = (cues.clone(), key.clone());
                IconButton::new((self.id.clone(), format!("{key}-remove")), IconName::X)
                    .size(ControlSize::Sm)
                    .tooltip("Remove")
                    .on_click(move |_, window, cx| {
                        let next: Vec<Cue> =
                            cues.iter().filter(|cue| cue.key != key).cloned().collect();
                        log::info!("subtitle editor: removed {key}");
                        change(&next, window, cx)
                    })
            });
            let said = complaint
                .as_ref()
                .filter(|(which, _)| *which == key)
                .map(|(_, reason)| {
                    div()
                        .debug_selector(|| "subtitle-complaint".into())
                        .px_2()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(colors.danger)
                        .child(reason.clone())
                });
            let text = InlineEdit::new((self.id.clone(), format!("{key}-words")), cue.text.clone())
                .placeholder("Words");
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap_0p5()
                .p_1()
                .rounded(theme.radius(Radius::Md))
                .when(lit, |row| row.bg(colors.active))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .children(seek)
                        .child(time(Edge::Start, cue.start))
                        .child(div().text_color(colors.fg_muted).child("→"))
                        .child(time(Edge::End, cue.end)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(div().flex_1().min_w_0().child(if editable {
                            text.on_commit(words)
                        } else {
                            text
                        }))
                        .children(remove),
                )
                .children(said)
        });
        let add = self.on_change.clone().map(|change| {
            let cues = cues.clone();
            Button::new((self.id.clone(), "add"), "Add a cue")
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .icon(IconName::Plus)
                .on_click(move |_, window, cx| {
                    let taken = |key: &str| cues.iter().any(|cue| cue.key == key);
                    let key = (cues.len() + 1..)
                        .map(|n| format!("cue-{n}"))
                        .find(|key| !taken(key))
                        .expect("a free key");
                    let mut next = cues.to_vec();
                    next.push(Cue {
                        key: key.clone().into(),
                        start: at,
                        end: at + FRESH,
                        text: SharedString::default(),
                    });
                    next.sort_by_key(|cue| cue.start);
                    log::info!("subtitle editor: added {key} at {}", stamp(at));
                    change(&next, window, cx)
                })
        });
        div()
            .flex()
            .flex_col()
            .gap_1()
            .children(rows)
            .child(div().flex().children(add))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Cue, Edge, parse_time, retimed, stamp};

    #[test]
    fn times_read_minutes_and_hours_with_or_without_a_fraction() {
        assert_eq!(parse_time("01:02.5"), Some(Duration::from_millis(62_500)));
        assert_eq!(
            parse_time(" 1:02:03.004 "),
            Some(Duration::from_millis(3_723_004))
        );
        assert_eq!(parse_time("75:00"), Some(Duration::from_secs(4500)));
        for bad in [
            "1:60",
            "a:10",
            "1:2:3:4",
            "01:02.5000",
            "01:02.x",
            "",
            "18446744073709551615:00",
            "5124095576030431:00:00",
            "307445734561825860:00",
        ] {
            assert_eq!(parse_time(bad), None, "{bad:?}");
        }
        assert_eq!(stamp(Duration::from_millis(62_500)), "01:02.500");
        assert_eq!(stamp(Duration::from_millis(3_723_004)), "1:02:03.004");
    }

    #[test]
    fn a_retimed_cue_keeps_order_and_refuses_a_bad_time_or_a_backward_one() {
        let cue = |key: &str, start: u64, end: u64| Cue {
            key: key.to_string().into(),
            start: Duration::from_secs(start),
            end: Duration::from_secs(end),
            text: key.to_string().into(),
        };
        let cues = [cue("a", 1, 3), cue("b", 5, 8)];
        let moved = retimed(&cues, "b", Edge::Start, "00:00.500").expect("a good time");
        assert_eq!(
            moved.iter().map(|cue| cue.key.as_ref()).collect::<Vec<_>>(),
            ["b", "a"]
        );
        assert!(retimed(&cues, "a", Edge::End, "1:x").is_err());
        assert_eq!(
            retimed(&cues, "a", Edge::End, "00:00.500"),
            Err("A cue must end after it starts".to_string())
        );
    }
}
