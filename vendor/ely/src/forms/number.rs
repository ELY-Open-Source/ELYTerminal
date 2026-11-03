use std::{cell::Cell, rc::Rc};

use gpui::{
    AnyElement, App, Context, CursorStyle, DragMoveEvent, ElementId, EmptyView, Entity, EntityId,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Window, div, prelude::*,
};

use super::{
    Input, InputEvent, TextInput,
    text::{Down, Up},
};
use crate::{
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, TextSize},
    typography::format::{self, Separators},
};

type OnChange = Rc<dyn Fn(f64, &mut Window, &mut App)>;
type Nudge = Box<dyn Fn(&mut Window, &mut App)>;

/// Reads a typed number: groups and the typographic minus are allowed.
pub(crate) fn parse(text: &str) -> Option<f64> {
    let plain: String = text
        .chars()
        .filter(|ch| *ch != ',')
        .map(|ch| if ch == format::MINUS { '-' } else { ch })
        .collect();
    plain
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

/// Rounds to `precision` places, then clamps to the rounded numbers in `min..=max`.
fn settle(value: f64, min: f64, max: f64, precision: usize) -> f64 {
    let scale = 10f64.powi(precision as i32);
    let (low, high) = (
        on_grid(min, scale, f64::ceil),
        on_grid(max, scale, f64::floor),
    );
    assert!(
        low <= high,
        "no {precision}-place number lies in {min}..={max}"
    );
    ((value * scale).round() / scale).clamp(low, high)
}

/// `bound` on the `1 / scale` grid: kept if float error alone put it off, else moved `inward`.
fn on_grid(bound: f64, scale: f64, inward: fn(f64) -> f64) -> f64 {
    let scaled = bound * scale;
    let near = scaled.round();
    if (scaled - near).abs() <= 4.0 * f64::EPSILON * near.abs().max(1.0) {
        near / scale
    } else {
        inward(scaled) / scale
    }
}

/// A number's live settings, read by its input's event handler.
struct Numeric {
    input: Entity<TextInput>,
    on_change: Option<OnChange>,
    on_commit: Option<OnChange>,
    value: f64,
    limits: (f64, f64, f64, usize),
    /// Shows no number; an empty field commits nothing.
    blank: bool,
    /// The owner's value as shown; a new one replaces a draft once focus leaves.
    owner: Option<String>,
    pending: bool,
    _events: Subscription,
}

impl Numeric {
    /// The typed number, or the owner's value while the text is not one.
    fn current(&self, cx: &App) -> f64 {
        parse(self.input.read(cx).text()).unwrap_or(self.value)
    }
}

/// Shows `value` in the field, then tells the owner.
fn show(
    input: &Entity<TextInput>,
    (on_change, on_commit): (Option<&OnChange>, Option<&OnChange>),
    value: f64,
    precision: usize,
    window: &mut Window,
    cx: &mut App,
) {
    let shown = format::number(value, precision, Separators::EN);
    input.update(cx, |input, cx| input.set_text(shown, cx));
    log::info!("number input: {value}");
    if let Some(on_change) = on_change {
        on_change(value, window, cx);
    }
    if let Some(on_commit) = on_commit {
        on_commit(value, window, cx);
    }
}

/// Settles `next` into the limits and shows it.
fn commit(state: &Entity<Numeric>, next: f64, window: &mut Window, cx: &mut App) {
    let (input, on_change, on_commit, (min, max, _, precision)) = {
        let numeric = state.read(cx);
        (
            numeric.input.clone(),
            numeric.on_change.clone(),
            numeric.on_commit.clone(),
            numeric.limits,
        )
    };
    let value = settle(next, min, max, precision);
    state.update(cx, |numeric, _| numeric.value = value);
    let handlers = (on_change.as_ref(), on_commit.as_ref());
    show(&input, handlers, value, precision, window, cx);
}

fn numeric(id: &ElementId, window: &mut Window, cx: &mut App) -> Entity<Numeric> {
    window.use_keyed_state(id.clone(), cx, |window, cx: &mut Context<Numeric>| {
        let input = cx.new(|cx| {
            TextInput::new(window, cx)
                .filter(|ch| ch.is_ascii_digit() || matches!(ch, '.' | ',' | '-' | format::MINUS))
        });
        let events = cx.subscribe_in(&input, window, |numeric, input, event, window, cx| {
            let (min, max, _, precision) = numeric.limits;
            match event {
                InputEvent::Blur | InputEvent::Submit => {
                    if numeric.blank && input.read(cx).text().trim().is_empty() {
                        return;
                    }
                    let value = settle(numeric.current(cx), min, max, precision);
                    numeric.value = value;
                    show(
                        input,
                        (numeric.on_change.as_ref(), numeric.on_commit.as_ref()),
                        value,
                        precision,
                        window,
                        cx,
                    );
                }
                InputEvent::Changed if input.read(cx).focus().is_focused(window) => {
                    let Some(typed) = parse(input.read(cx).text()) else {
                        return;
                    };
                    let settled = typed == settle(typed, min, max, precision);
                    if typed == numeric.value || !settled {
                        return;
                    }
                    log::info!("number input: typed {typed}");
                    numeric.value = typed;
                    if let Some(on_change) = numeric.on_change.clone() {
                        on_change(typed, window, cx);
                    }
                }
                _ => {}
            }
        });
        Numeric {
            input,
            on_change: None,
            on_commit: None,
            value: 0.0,
            limits: (f64::MIN, f64::MAX, 1.0, 0),
            blank: false,
            owner: None,
            pending: false,
            _events: events,
        }
    })
}

struct Scrub {
    owner: EntityId,
    start: Rc<Cell<Option<(Pixels, f64)>>>,
}

/// Wraps `face` so dragging it sideways moves the number, one step per notch.
fn scrub_handle(
    id: ElementId,
    face: AnyElement,
    state: Entity<Numeric>,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement + use<> {
    let owner = window
        .use_keyed_state(id.clone(), cx, |_, _| ())
        .entity_id();
    let notch = cx.theme().handle_hit().to_pixels(window.rem_size());
    div()
        .id(id)
        .cursor(CursorStyle::ResizeLeftRight)
        .on_drag(
            Scrub {
                owner,
                start: Rc::new(Cell::new(None)),
            },
            |scrub, _, _, cx| {
                scrub.start.set(None);
                cx.new(|_| EmptyView)
            },
        )
        .on_drag_move(move |event: &DragMoveEvent<Scrub>, window, cx| {
            let scrub = event.drag(cx);
            if scrub.owner != owner {
                return;
            }
            let (start, x) = (scrub.start.clone(), event.event.position.x);
            let (anchor, from) = start.get().unwrap_or_else(|| {
                let fresh = (x, state.read(cx).current(cx));
                start.set(Some(fresh));
                fresh
            });
            let notches = f64::from(((x - anchor) / notch).trunc());
            let step = state.read(cx).limits.2;
            commit(&state, from + notches * step, window, cx);
        })
        .child(face)
}

/// A number field: arrows or the stepper move it, dragging the grip scrubs it.
#[derive(IntoElement)]
pub struct NumberInput {
    pub(super) id: ElementId,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    precision: usize,
    prefix: Option<SharedString>,
    suffix: Option<AnyElement>,
    size: ControlSize,
    on_change: Option<OnChange>,
    on_commit: Option<OnChange>,
    scrub_label: Option<SharedString>,
    label: SharedString,
    blank: Option<SharedString>,
}

impl NumberInput {
    pub fn new(id: impl Into<ElementId>, value: f64) -> Self {
        Self {
            id: id.into(),
            value,
            min: f64::MIN,
            max: f64::MAX,
            step: 1.0,
            precision: 0,
            prefix: None,
            suffix: None,
            size: ControlSize::default(),
            on_change: None,
            on_commit: None,
            scrub_label: None,
            label: SharedString::default(),
            blank: None,
        }
    }

    /// The name assistive technology reads, drawn nowhere.
    pub fn label(mut self, text: impl Into<SharedString>) -> Self {
        self.label = text.into();
        self
    }

    /// Shows `placeholder`, not a number, as for a mixed selection; commits once typed in.
    pub fn blank(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.blank = Some(placeholder.into());
        self
    }

    pub fn range(mut self, min: f64, max: f64) -> Self {
        assert!(min <= max, "number range {min}..{max} is empty");
        self.min = min;
        self.max = max;
        self
    }

    pub fn step(mut self, step: f64) -> Self {
        assert!(step > 0.0, "number step {step} must be positive");
        self.step = step;
        self
    }

    /// Decimal places kept and shown.
    pub fn precision(mut self, places: usize) -> Self {
        self.precision = places;
        self
    }

    /// A symbol before the digits, such as `$`.
    pub fn prefix(mut self, symbol: impl Into<SharedString>) -> Self {
        self.prefix = Some(symbol.into());
        self
    }

    /// Money in an ISO 4217 currency: its symbol and minor units.
    pub fn currency(mut self, code: &str) -> Self {
        let (symbol, places, _) = format::currency_parts(code);
        self.prefix = Some(SharedString::from(symbol.to_string()));
        self.precision = places;
        self
    }

    /// A percentage, marked with `%`.
    pub fn percent(self) -> Self {
        self.suffix(div().child("%"))
    }

    /// Something after the digits, such as `%` or a unit.
    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(f64, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    /// Runs on Enter, blur, steps and scrubs; never while typing.
    pub fn on_commit(mut self, handler: impl Fn(f64, &mut Window, &mut App) + 'static) -> Self {
        self.on_commit = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for NumberInput {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = numeric(&self.id, window, cx);
        let (value, min, max, step) = (self.value, self.min, self.max, self.step);
        let shown = match self.blank {
            Some(_) => String::new(),
            None => format::number(value, self.precision, Separators::EN),
        };
        let input = state.read(cx).input.clone();
        let editing = input.read(cx).focus().is_focused(window);
        let echo = input.read(cx).text() == shown;
        let reseed = state.update(cx, |numeric, _| {
            numeric.on_change = self.on_change.clone();
            numeric.on_commit = self.on_commit.clone();
            numeric.limits = (min, max, step, self.precision);
            numeric.blank = self.blank.is_some();
            if numeric.owner.as_ref() != Some(&shown) {
                numeric.owner = Some(shown.clone());
                numeric.value = value;
                numeric.pending = !echo;
            }
            let reseed = numeric.pending && !editing;
            numeric.pending &= editing;
            reseed
        });
        if input.read(cx).label_text() != &self.label {
            let label = self.label.clone();
            input.update(cx, |input, cx| input.set_label(label, cx));
        }
        let placeholder = self.blank.clone().unwrap_or_default();
        if input.read(cx).placeholder_text() != &placeholder {
            input.update(cx, |input, cx| input.set_placeholder(placeholder, cx));
        }
        if reseed {
            input.update(cx, |input, cx| input.set_text(shown, cx));
        }
        let now = state.read(cx).current(cx);
        let nudge = |by: f64| {
            let state = state.clone();
            move |window: &mut Window, cx: &mut App| {
                let from = state.read(cx).current(cx);
                commit(&state, from + by, window, cx);
            }
        };
        let (up, down, key_up, key_down) = (nudge(step), nudge(-step), nudge(step), nudge(-step));
        let label = self.scrub_label.take().map(|label| {
            let theme = cx.theme();
            let face = div()
                .flex_none()
                .min_w(theme.control_height(ControlSize::Md))
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(theme.colors.fg_muted)
                .child(label)
                .into_any_element();
            scrub_handle(
                (self.id.clone(), "label").into(),
                face,
                state.clone(),
                window,
                cx,
            )
        });
        let subtle = cx.theme().colors.fg_subtle;
        let grip = scrub_handle(
            (self.id.clone(), "grip").into(),
            Icon::new(IconName::ChevronsUpDown)
                .size(IconSize::Xs)
                .color(subtle)
                .into_any_element(),
            state,
            window,
            cx,
        );
        let theme = cx.theme();
        let colors = &theme.colors;
        let half = |id: &'static str, icon: IconName, enabled: bool, act: Nudge| {
            div()
                .id((self.id.clone(), id))
                .flex()
                .flex_1()
                .items_center()
                .justify_center()
                .w(theme.icon_size(IconSize::Lg))
                .map(|half| {
                    if enabled {
                        half.cursor_pointer()
                            .hover(|style| style.bg(colors.hover))
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.prevent_default()
                            })
                            .on_click(move |_, window, cx| act(window, cx))
                    } else {
                        half.opacity(0.4)
                    }
                })
                .child(Icon::new(icon).size(IconSize::Xs).color(colors.fg_muted))
        };
        let stepper = div()
            .flex()
            .flex_col()
            .h_full()
            .border_l_1()
            .border_color(colors.border)
            .child(half("up", IconName::ChevronUp, now < max, Box::new(up)))
            .child(half(
                "down",
                IconName::ChevronDown,
                now > min,
                Box::new(down),
            ));
        let ends = [min, max].map(|end| format::number(end, self.precision, Separators::EN));
        let widest = ends.into_iter().max_by_key(|end| end.chars().count());
        let ranged = min > f64::MIN && max < f64::MAX;
        let field = Input::new(&input)
            .size(self.size)
            .when_some(widest.filter(|_| ranged), Input::least)
            .prefix(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(grip)
                    .when_some(self.prefix, |prefix, symbol| {
                        prefix.child(div().text_color(subtle).child(symbol))
                    }),
            )
            .suffix(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .children(self.suffix)
                    .child(stepper),
            );
        div()
            .debug_selector(|| "number-root".into())
            .flex()
            .items_center()
            .gap_2()
            .children(label)
            .child(
                div()
                    .id(self.id)
                    .flex_1()
                    .min_w_0()
                    .capture_action(move |_: &Up, window, cx| {
                        cx.stop_propagation();
                        key_up(window, cx);
                    })
                    .capture_action(move |_: &Down, window, cx| {
                        cx.stop_propagation();
                        key_down(window, cx);
                    })
                    .child(field),
            )
    }
}

mod scrub;
pub use scrub::ScrubInput;

#[cfg(test)]
mod tests;
