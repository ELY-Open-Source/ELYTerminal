use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, Div, ElementId, Entity, FocusHandle, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Pixels, RenderOnce, SharedString, Stateful, Styled, Window, canvas,
    prelude::*,
};
use jiff::civil::Date;

use super::{
    super::{
        options::{Run, float, surface},
        select::{field_button, field_text},
    },
    Calendar,
    calendar::OnDate,
    show_date, show_span,
};
use crate::{
    primitives::{Icon, IconName, tab_stop},
    theme::{ActiveTheme, ControlSize, IconSize},
};

type OnSpan = Rc<dyn Fn((Date, Date), &mut Window, &mut App)>;

/// Rows a popped calendar is about as tall as, for placing it.
pub(crate) const CALENDAR_ROWS: usize = 9;

/// A picker's open popup: where it hangs, and the focus its content takes.
pub(crate) struct Dropdown {
    pub open: bool,
    pub anchor: Bounds<Pixels>,
    pub inner: FocusHandle,
    pub start: Option<Date>,
    pub preview: Option<Date>,
}

pub(crate) fn dropdown(id: &ElementId, window: &mut Window, cx: &mut App) -> Entity<Dropdown> {
    window.use_keyed_state((id.clone(), "dropdown"), cx, |_, cx| Dropdown {
        open: false,
        anchor: Bounds::default(),
        inner: cx.focus_handle(),
        start: None,
        preview: None,
    })
}

/// Opens or closes, moving focus into the popup or back to `trigger`.
pub(crate) fn toggle(
    state: &Entity<Dropdown>,
    open: bool,
    trigger: &FocusHandle,
    window: &mut Window,
    cx: &mut App,
) {
    let inner = state.read(cx).inner.clone();
    state.update(cx, |state, cx| {
        state.open = open;
        state.start = None;
        state.preview = None;
        cx.notify();
    });
    window.focus(if open { &inner } else { trigger }, cx);
}

/// What a picker field shows while closed, and how it sits.
pub(crate) struct Face {
    pub icon: IconName,
    pub shown: Option<SharedString>,
    pub placeholder: SharedString,
    pub size: ControlSize,
    pub disabled: bool,
}

/// The trigger of a date or time picker: an icon, the value, and the popup when open.
pub(crate) fn picker_field(
    id: ElementId,
    face: Face,
    popup: impl FnOnce(Run, Bounds<Pixels>, &mut Window, &mut App) -> AnyElement,
    window: &mut Window,
    cx: &mut App,
) -> Stateful<Div> {
    let Face {
        icon,
        shown,
        placeholder,
        size,
        disabled,
    } = face;
    let trigger = tab_stop((id.clone(), "focus").into(), !disabled, window, cx);
    let state = dropdown(&id, window, cx);
    let (open, anchor, inner) = {
        let state = state.read(cx);
        (state.open, state.anchor, state.inner.clone())
    };
    if open && disabled {
        log::info!("picker {id:?}: closed when disabled");
        toggle(&state, false, &trigger, window, cx);
    } else if open && !trigger.is_focused(window) && !inner.contains_focused(window, cx) {
        log::info!("picker {id:?}: closed on blur");
        state.update(cx, |state, _| state.open = false);
    }
    let open = state.read(cx).open;
    let close: Run = {
        let (state, trigger) = (state.clone(), trigger.clone());
        Rc::new(move |window, cx| toggle(&state, false, &trigger, window, cx))
    };
    let popup = open.then(|| popup(close, anchor, window, cx));
    let subtle = cx.theme().colors.fg_subtle;
    let (click, keys, measure, focus) = (state.clone(), state.clone(), state, trigger.clone());
    field_button(id, &trigger, size, disabled, window, cx)
        .when(!disabled, |field| {
            let focus_keys = focus.clone();
            field
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    window.prevent_default();
                    toggle(&click, !open, &focus, window, cx);
                })
                .on_key_down(move |event, window, cx| {
                    if !open && matches!(event.keystroke.key.as_str(), "down" | "enter" | "space") {
                        cx.stop_propagation();
                        toggle(&keys, true, &focus_keys, window, cx);
                    }
                })
        })
        .child(Icon::new(icon).size(IconSize::Sm).color(subtle))
        .child(field_text(shown, placeholder, disabled, cx))
        .child(
            canvas(
                move |bounds, _, cx| {
                    if measure.read(cx).anchor != bounds {
                        measure.update(cx, |state, _| state.anchor = bounds);
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
        .children(popup)
}

/// A field that opens a calendar and shows the day chosen.
#[derive(IntoElement)]
pub struct DatePicker {
    id: ElementId,
    value: Option<Date>,
    placeholder: SharedString,
    bounds: (Option<Date>, Option<Date>),
    today: Option<Date>,
    size: ControlSize,
    disabled: bool,
    on_change: Option<OnDate>,
}

impl DatePicker {
    pub fn new(id: impl Into<ElementId>, value: Option<Date>) -> Self {
        Self {
            id: id.into(),
            value,
            placeholder: SharedString::from("Pick a date"),
            bounds: (None, None),
            today: None,
            size: ControlSize::default(),
            disabled: false,
            on_change: None,
        }
    }

    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// The earliest and latest days that can be picked.
    pub fn range(mut self, min: Option<Date>, max: Option<Date>) -> Self {
        self.bounds = (min, max);
        self
    }

    pub fn today(mut self, date: Date) -> Self {
        self.today = Some(date);
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(Date, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

/// A calendar in a popup, with the picker's limits and focus.
pub(crate) fn popped(
    id: &ElementId,
    state: &Entity<Dropdown>,
    (min, max, today): (Option<Date>, Option<Date>, Option<Date>),
    cx: &App,
) -> Calendar {
    let mut calendar = Calendar::new((id.clone(), "calendar")).focus(state.read(cx).inner.clone());
    if let Some(min) = min {
        calendar = calendar.min(min);
    }
    if let Some(max) = max {
        calendar = calendar.max(max);
    }
    if let Some(today) = today {
        calendar = calendar.today(today);
    }
    calendar
}

impl RenderOnce for DatePicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let (value, limits, on_change) = (
            self.value,
            (self.bounds.0, self.bounds.1, self.today),
            self.on_change,
        );
        let state = dropdown(&id, window, cx);
        picker_field(
            self.id,
            Face {
                icon: IconName::Calendar,
                shown: value.map(|date| show_date(date).into()),
                placeholder: self.placeholder,
                size: self.size,
                disabled: self.disabled,
            },
            move |close, anchor, window, cx| {
                let mut calendar = popped(&id, &state, limits, cx);
                if let Some(value) = value {
                    calendar = calendar.selected(value);
                }
                let (escape, done) = (close.clone(), close.clone());
                float(
                    id.clone(),
                    anchor,
                    CALENDAR_ROWS,
                    surface((id.clone(), "popup"), cx)
                        .on_mouse_down_out(move |_, window, cx| close(window, cx))
                        .child(calendar.on_escape(escape).on_pick(move |date, window, cx| {
                            if let Some(on_change) = &on_change {
                                on_change(date, window, cx);
                            }
                            done(window, cx);
                        })),
                    window,
                    cx,
                )
            },
            window,
            cx,
        )
    }
}

/// A field that picks a first and a last day. Two clicks set the span.
#[derive(IntoElement)]
pub struct DateRangePicker {
    id: ElementId,
    value: Option<(Date, Date)>,
    placeholder: SharedString,
    today: Option<Date>,
    size: ControlSize,
    on_change: Option<OnSpan>,
}

impl DateRangePicker {
    pub fn new(id: impl Into<ElementId>, value: Option<(Date, Date)>) -> Self {
        Self {
            id: id.into(),
            value,
            placeholder: SharedString::from("Pick dates"),
            today: None,
            size: ControlSize::default(),
            on_change: None,
        }
    }

    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.placeholder = text.into();
        self
    }

    pub fn today(mut self, date: Date) -> Self {
        self.today = Some(date);
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn((Date, Date), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

/// The span between two picks, first day first.
pub(crate) fn ordered(a: Date, b: Date) -> (Date, Date) {
    (a.min(b), a.max(b))
}

impl RenderOnce for DateRangePicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let (value, today, on_change) = (self.value, self.today, self.on_change);
        let state = dropdown(&id, window, cx);
        picker_field(
            self.id,
            Face {
                icon: IconName::CalendarRange,
                shown: value.map(|(start, end)| show_span(start, end).into()),
                placeholder: self.placeholder,
                size: self.size,
                disabled: false,
            },
            move |close, anchor, window, cx| {
                let (start, preview) = (state.read(cx).start, state.read(cx).preview);
                let span = match start {
                    Some(start) => Some(ordered(start, preview.unwrap_or(start))),
                    None => value,
                };
                let mut calendar = popped(&id, &state, (None, None, today), cx);
                if let Some((first, last)) = span {
                    calendar = calendar.span(first, last);
                }
                if let Some(anchor) = start.or(value.map(|(first, _)| first)) {
                    calendar = calendar.anchor(anchor);
                }
                let (hover, pick, escape) = (state.clone(), state.clone(), close.clone());
                float(
                    id.clone(),
                    anchor,
                    CALENDAR_ROWS,
                    surface((id.clone(), "popup"), cx)
                        .on_mouse_down_out(move |_, window, cx| close(window, cx))
                        .child(
                            calendar
                                .on_escape(escape.clone())
                                .on_hover(Rc::new(move |date, _, cx| {
                                    hover.update(cx, |state, cx| {
                                        state.preview = date;
                                        cx.notify();
                                    })
                                }))
                                .on_pick(move |date, window, cx| {
                                    let Some(first) = pick.read(cx).start else {
                                        pick.update(cx, |state, cx| {
                                            state.start = Some(date);
                                            cx.notify();
                                        });
                                        return;
                                    };
                                    let span = ordered(first, date);
                                    log::info!("date range picker: {} to {}", span.0, span.1);
                                    if let Some(on_change) = &on_change {
                                        on_change(span, window, cx);
                                    }
                                    escape(window, cx);
                                }),
                        ),
                    window,
                    cx,
                )
            },
            window,
            cx,
        )
    }
}
