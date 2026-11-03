use std::rc::Rc;

use gpui::{
    App, Context, ElementId, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Subscription, Window, div, prelude::*,
};
use jiff::{SignedDuration, Timestamp, civil::DateTime, tz::TimeZone};

use super::{
    event::{Event, When},
    recurrence::RecurrenceEditor,
    reminders::ReminderPicker,
    repeat::Recurrence,
};
use crate::{
    buttons::{Button, ButtonVariant},
    forms::{
        ColorPalette, DatePicker, DateTimePicker, FormError, FormField, Input, InputEvent, Run,
        Switch, TextInput,
    },
    theme::{ActiveTheme, HUE_NAMES},
    typography::format,
};

type OnDraft = Rc<dyn Fn(&EventDraft, &mut Window, &mut App)>;

/// An event's parts as an editor hands them back.
#[derive(Clone, Debug, PartialEq)]
pub struct EventDraft {
    pub title: SharedString,
    pub when: When,
    pub place: SharedString,
    pub hue: usize,
    pub repeat: Option<Recurrence>,
    pub reminders: Vec<u32>,
    pub notes: SharedString,
}

impl EventDraft {
    /// `event`'s parts, with no repeat, reminders or notes.
    pub fn of(event: &Event) -> Self {
        Self {
            title: event.title.clone(),
            when: event.when,
            place: event.place.clone().unwrap_or_default(),
            hue: event.hue,
            repeat: None,
            reminders: Vec::new(),
            notes: SharedString::default(),
        }
    }

    /// Why it cannot be saved: no title, or an end before its start.
    pub(crate) fn fault(&self) -> Option<&'static str> {
        if self.title.trim().is_empty() {
            return Some("Give it a title.");
        }
        let backward = match self.when {
            When::Timed { start, end } => end <= start,
            When::AllDay { first, last } => last < first,
        };
        backward.then_some("It ends before it starts.")
    }
}

/// `when` as whole days, or as hours from nine to ten on its first day; none when the zone holds no such hours.
pub(super) fn toggled(when: When, zone: &TimeZone) -> Option<When> {
    match when {
        When::Timed { start, end } => {
            let first = start.to_zoned(zone.clone()).date();
            let last = end.to_zoned(zone.clone()).date().max(first);
            Some(When::AllDay { first, last })
        }
        When::AllDay { first, .. } => {
            let at = |hour| match first.at(hour, 0, 0, 0).to_zoned(zone.clone()) {
                Ok(at) => Some(at.timestamp()),
                Err(error) => {
                    log::error!("event editor: {first} at {hour}:00 has no moment: {error}");
                    None
                }
            };
            Some(When::Timed {
                start: at(9)?,
                end: at(10)?,
            })
        }
    }
}

/// `when` with its start moved to `start`, its length kept; none for an all-day event or an end past what jiff holds.
pub(super) fn moved(when: When, start: Timestamp) -> Option<When> {
    let When::Timed { start: was, end } = when else {
        log::error!("event editor: an all-day event does not move by the hour");
        return None;
    };
    let length: SignedDuration = end.duration_since(was);
    match start.checked_add(length) {
        Ok(end) => Some(When::Timed { start, end }),
        Err(error) => {
            log::error!("event editor: no end {length} after {start}: {error}");
            None
        }
    }
}

/// The editor's working copy: what the owner gave, the parts changed so far, and the fields that hold its words.
struct Editing {
    seed: EventDraft,
    draft: EventDraft,
    title: Entity<TextInput>,
    place: Entity<TextInput>,
    notes: Entity<TextInput>,
    _title: Subscription,
}

impl Editing {
    fn fill(&mut self, seed: EventDraft, cx: &mut Context<Self>) {
        for (input, text) in [
            (&self.title, &seed.title),
            (&self.place, &seed.place),
            (&self.notes, &seed.notes),
        ] {
            input.update(cx, |input, cx| input.set_text(text.to_string(), cx));
        }
        self.draft = seed.clone();
        self.seed = seed;
    }

    /// The draft as it stands, its words read from the fields.
    fn current(&self, cx: &App) -> EventDraft {
        let words =
            |input: &Entity<TextInput>| SharedString::from(input.read(cx).text().to_string());
        EventDraft {
            title: words(&self.title).trim().to_string().into(),
            place: words(&self.place).trim().to_string().into(),
            notes: words(&self.notes),
            ..self.draft.clone()
        }
    }
}

/// An event's parts in a form: its title, whole days or hours from and until, where, its color, how it repeats, its reminders and notes. Save waits for a title and an end after the start; Delete shows with its handler. A new draft from the owner starts it over.
#[derive(IntoElement)]
pub struct EventEditor {
    id: ElementId,
    draft: EventDraft,
    zone: Option<TimeZone>,
    on_save: Option<OnDraft>,
    on_cancel: Option<Run>,
    on_delete: Option<Run>,
}

impl EventEditor {
    pub fn new(id: impl Into<ElementId>, draft: EventDraft) -> Self {
        Self {
            id: id.into(),
            draft,
            zone: None,
            on_save: None,
            on_cancel: None,
            on_delete: None,
        }
    }

    /// The zone its hours read in; the system's otherwise.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    /// Gets the draft to keep.
    pub fn on_save(
        mut self,
        handler: impl Fn(&EventDraft, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_save = Some(Rc::new(handler));
        self
    }

    pub fn on_cancel(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_cancel = Some(Rc::new(handler));
        self
    }

    /// Offers Delete, for an event that already exists.
    pub fn on_delete(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_delete = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for EventEditor {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let zone = self
            .zone
            .unwrap_or_else(|| format::system_zone("event editor"));
        let seed = self.draft.clone();
        let editing = window.use_keyed_state(
            (self.id.clone(), "editing"),
            cx,
            |window, cx: &mut Context<Editing>| {
                let mut field = |placeholder: &'static str, text: &SharedString, lines: bool| {
                    cx.new(|cx| {
                        let input = TextInput::new(window, cx).placeholder(placeholder);
                        let mut input = match lines {
                            true => input.multi_line(3, 6),
                            false => input,
                        };
                        input.set_text(text.to_string(), cx);
                        input
                    })
                };
                let title = field("Title", &seed.title, false);
                let place = field("Where", &seed.place, false);
                let notes = field("Notes", &seed.notes, true);
                let watch = cx.subscribe(&title, |_, _, event: &InputEvent, cx| {
                    if *event == InputEvent::Changed {
                        cx.notify();
                    }
                });
                Editing {
                    seed: seed.clone(),
                    draft: seed.clone(),
                    title,
                    place,
                    notes,
                    _title: watch,
                }
            },
        );
        if editing.read(cx).seed != self.draft {
            log::info!("event editor {:?}: started over", self.id);
            editing.update(cx, |editing, cx| editing.fill(self.draft.clone(), cx));
        }
        let draft = editing.read(cx).current(cx);
        let edit = {
            let editing = editing.clone();
            move |change: Rc<dyn Fn(&mut EventDraft)>| {
                let editing = editing.clone();
                move |cx: &mut App| {
                    editing.update(cx, |editing, cx| {
                        change(&mut editing.draft);
                        cx.notify();
                    })
                }
            }
        };
        let civil = |at: Timestamp| at.to_zoned(zone.clone()).datetime();
        let exact = {
            let zone = zone.clone();
            move |at: DateTime| match at.to_zoned(zone.clone()) {
                Ok(at) => Some(at.timestamp()),
                Err(error) => {
                    log::error!("event editor: {at} has no moment in its zone: {error}");
                    None
                }
            }
        };
        let whole = matches!(draft.when, When::AllDay { .. });
        let toggle = {
            let zone = zone.clone();
            let when = draft.when;
            edit(Rc::new(move |draft| {
                if let Some(when) = toggled(when, &zone) {
                    draft.when = when;
                }
            }))
        };
        let span: Vec<gpui::AnyElement> = match draft.when {
            When::Timed { start, end } => {
                let (at, until) = (exact.clone(), exact.clone());
                vec![
                    DateTimePicker::new((self.id.clone(), "starts"), Some(civil(start)))
                        .on_change({
                            let edit = edit.clone();
                            move |picked, _, cx| {
                                let Some(start) = at(picked) else {
                                    return;
                                };
                                edit(Rc::new(move |draft| {
                                    if let Some(when) = moved(draft.when, start) {
                                        draft.when = when;
                                    }
                                }))(cx)
                            }
                        })
                        .into_any_element(),
                    DateTimePicker::new((self.id.clone(), "ends"), Some(civil(end)))
                        .on_change({
                            let edit = edit.clone();
                            move |picked, _, cx| {
                                let Some(end) = until(picked) else {
                                    return;
                                };
                                edit(Rc::new(move |draft| {
                                    if let When::Timed { start, .. } = draft.when {
                                        draft.when = When::Timed { start, end };
                                    }
                                }))(cx)
                            }
                        })
                        .into_any_element(),
                ]
            }
            When::AllDay { first, last } => vec![
                DatePicker::new((self.id.clone(), "first"), Some(first))
                    .on_change({
                        let edit = edit.clone();
                        move |day, _, cx| {
                            let length = last - first;
                            let Ok(last) = day.checked_add(length) else {
                                log::error!("event editor: no day {length} after {day}");
                                return;
                            };
                            edit(Rc::new(move |draft| {
                                draft.when = When::AllDay { first: day, last };
                            }))(cx)
                        }
                    })
                    .into_any_element(),
                DatePicker::new((self.id.clone(), "last"), Some(last))
                    .on_change({
                        let edit = edit.clone();
                        move |day, _, cx| {
                            edit(Rc::new(move |draft| {
                                if let When::AllDay { first, .. } = draft.when {
                                    draft.when = When::AllDay { first, last: day };
                                }
                            }))(cx)
                        }
                    })
                    .into_any_element(),
            ],
        };
        let theme = cx.theme();
        let chart = theme.colors.chart;
        let (first, _) = Event {
            key: self.id.to_string().into(),
            title: draft.title.clone(),
            when: draft.when,
            hue: draft.hue,
            place: None,
        }
        .days(&zone);
        let fault = draft.fault();
        let save = {
            let (editing, on_save, id) = (editing.clone(), self.on_save.clone(), self.id.clone());
            Button::new((self.id.clone(), "save"), "Save")
                .variant(ButtonVariant::Primary)
                .disabled(fault.is_some())
                .on_click(move |_, window, cx| {
                    let draft = editing.read(cx).current(cx);
                    log::info!("event editor {id:?}: save {}", draft.title);
                    if let Some(on_save) = &on_save {
                        on_save(&draft, window, cx);
                    }
                })
        };
        let cancel = self.on_cancel.clone().map(|on_cancel| {
            Button::new((self.id.clone(), "cancel"), "Cancel")
                .variant(ButtonVariant::Ghost)
                .on_click(move |_, window, cx| on_cancel(window, cx))
        });
        let delete = self.on_delete.clone().map(|on_delete| {
            Button::new((self.id.clone(), "delete"), "Delete")
                .variant(ButtonVariant::Ghost)
                .on_click(move |_, window, cx| on_delete(window, cx))
        });
        let (title, place, notes) = {
            let editing = editing.read(cx);
            (
                editing.title.clone(),
                editing.place.clone(),
                editing.notes.clone(),
            )
        };
        div()
            .debug_selector(|| "event-editor".into())
            .flex()
            .flex_col()
            .gap_4()
            .child(
                FormField::new((self.id.clone(), "title-field"), "Title")
                    .required()
                    .child(Input::new(&title)),
            )
            .child(
                Switch::new((self.id.clone(), "all-day"), whole)
                    .label("All day")
                    .on_change(move |_, _, cx| toggle(cx)),
            )
            .child(
                div().flex().flex_wrap().gap_3().children(
                    span.into_iter()
                        .zip(["Starts", "Ends"])
                        .map(|(picker, label)| {
                            div().flex_1().min_w(theme.label_width()).child(
                                FormField::new((self.id.clone(), label), label).child(picker),
                            )
                        }),
                ),
            )
            .children((fault == Some("It ends before it starts.")).then(|| {
                div()
                    .debug_selector(|| "editor-fault".into())
                    .child(FormError::new(
                        (self.id.clone(), "fault"),
                        "It ends before it starts.",
                    ))
            }))
            .child(
                FormField::new((self.id.clone(), "place-field"), "Where").child(Input::new(&place)),
            )
            .child(
                FormField::new((self.id.clone(), "color-field"), "Color").child(
                    ColorPalette::new(
                        (self.id.clone(), "color"),
                        HUE_NAMES
                            .iter()
                            .zip(chart)
                            .map(|(name, color)| (*name, color)),
                    )
                    .selected(chart[draft.hue])
                    .on_change({
                        let edit = edit.clone();
                        move |color, _, cx| {
                            let hue = chart
                                .iter()
                                .position(|each| *each == color)
                                .expect("a hue of the chart");
                            edit(Rc::new(move |draft| draft.hue = hue))(cx)
                        }
                    }),
                ),
            )
            .child(
                FormField::new((self.id.clone(), "repeat-field"), "Repeat").child(
                    RecurrenceEditor::new((self.id.clone(), "repeat"), first, draft.repeat.clone())
                        .on_change({
                            let edit = edit.clone();
                            move |rule, _, cx| {
                                edit(Rc::new(move |draft| draft.repeat = rule.clone()))(cx)
                            }
                        }),
                ),
            )
            .child(
                FormField::new((self.id.clone(), "reminders-field"), "Reminders").child(
                    ReminderPicker::new((self.id.clone(), "reminders"), draft.reminders.clone())
                        .on_change({
                            let edit = edit.clone();
                            move |minutes, _, cx| {
                                let minutes = minutes.to_vec();
                                edit(Rc::new(move |draft| draft.reminders = minutes.clone()))(cx)
                            }
                        }),
                ),
            )
            .child(
                FormField::new((self.id.clone(), "notes-field"), "Notes").child(Input::new(&notes)),
            )
            .child(
                div()
                    .debug_selector(|| "editor-actions".into())
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(div().children(delete))
                    .child(div().flex().gap_2().children(cancel).child(save)),
            )
    }
}
