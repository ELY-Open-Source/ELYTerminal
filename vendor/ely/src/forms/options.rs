use std::rc::Rc;

use gpui::{
    Anchor, Animation, AnimationExt, AnyElement, App, Bounds, Div, ElementId, Entity,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, ScrollHandle,
    SharedString, Stateful, StatefulInteractiveElement, Styled, Window, anchored, canvas, div,
    prelude::*,
};

use crate::{
    data_display::Avatar,
    motion,
    primitives::{Icon, IconName, raise},
    theme::{ActiveTheme, AvatarSize, ControlSize, Elevation, IconSize, Radius, TextSize},
    typography::Ellipsis,
};

/// One option: a value, the label shown for it, and an optional icon and note.
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub value: SharedString,
    pub label: SharedString,
    pub icon: Option<IconName>,
    pub avatar: Option<SharedString>,
    pub note: Option<SharedString>,
    pub disabled: bool,
    pub depth: usize,
}

impl Choice {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            icon: None,
            avatar: None,
            note: None,
            disabled: false,
            depth: 0,
        }
    }

    /// How far it sits under a parent, for choices that come from a tree.
    pub fn depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Leads with the initials of `name` on its tone, as a person's row does.
    pub fn avatar(mut self, name: impl Into<SharedString>) -> Self {
        self.avatar = Some(name.into());
        self
    }

    /// Quiet text at the row's end, such as a count or a shortcut.
    pub fn note(mut self, note: impl Into<SharedString>) -> Self {
        self.note = Some(note.into());
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}

pub(crate) type Pick = Rc<dyn Fn(usize, &mut Window, &mut App)>;
pub(crate) type Run = Rc<dyn Fn(&mut Window, &mut App)>;
pub(crate) type OnFlag = Rc<dyn Fn(bool, &mut Window, &mut App)>;
pub(crate) type OnNumber = Rc<dyn Fn(f64, &mut Window, &mut App)>;
pub(crate) type OnValue = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
pub(crate) type OnValues = Rc<dyn Fn(&[SharedString], &mut Window, &mut App)>;

/// The next row from `from` that is not disabled, stepping by `by` and wrapping.
pub(crate) fn step(rows: &[Choice], from: usize, by: isize) -> usize {
    let count = rows.len() as isize;
    let mut at = from as isize;
    for _ in 0..count {
        at = (at + by).rem_euclid(count);
        if !rows[at as usize].disabled {
            return at as usize;
        }
    }
    from
}

/// One row: a check column when `checked` is known, then icon, label and note.
pub(crate) fn option_row(
    id: impl Into<ElementId>,
    choice: &Choice,
    highlighted: bool,
    checked: Option<bool>,
    cx: &App,
) -> Stateful<Div> {
    marked_row(id, choice, None, highlighted, checked, cx)
}

/// An option row with `mark`, such as a label's color, before the label.
pub(crate) fn marked_row(
    id: impl Into<ElementId>,
    choice: &Choice,
    mark: Option<AnyElement>,
    highlighted: bool,
    checked: Option<bool>,
    cx: &App,
) -> Stateful<Div> {
    let id = id.into();
    let theme = cx.theme();
    let colors = &theme.colors;
    let fg = if choice.disabled {
        colors.fg_disabled
    } else {
        colors.fg
    };
    div()
        .id(id.clone())
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .py_1()
        .rounded(theme.radius(Radius::Md))
        .text_color(fg)
        .when(highlighted && !choice.disabled, |row| row.bg(colors.hover))
        .when(!choice.disabled, |row| {
            row.cursor_pointer().hover(|style| style.bg(colors.hover))
        })
        .when_some(checked, |row, on| {
            row.child(
                div()
                    .flex_none()
                    .size(theme.icon_size(IconSize::Sm))
                    .when(on, |mark| {
                        mark.child(Icon::new(IconName::Check).size(IconSize::Sm).color(fg))
                    }),
            )
        })
        .when(choice.depth > 0, |row| {
            row.child(
                div()
                    .flex_none()
                    .w(theme.tree_indent() * choice.depth as f32),
            )
        })
        .when_some(choice.icon, |row, icon| {
            row.child(Icon::new(icon).size(IconSize::Sm).color(colors.fg_muted))
        })
        .when_some(choice.avatar.clone(), |row, name| {
            row.child(Avatar::new((id, "avatar"), name).size(AvatarSize::Xs))
        })
        .children(mark)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(Ellipsis::new(choice.label.clone())),
        )
        .when_some(choice.note.clone(), |row, note| {
            row.child(div().text_color(colors.fg_subtle).child(note))
        })
}

/// The card a floating list sits on.
pub(crate) fn surface(id: impl Into<ElementId>, cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    let colors = &theme.colors;
    div()
        .id(id)
        .rounded(theme.radius(Radius::Lg))
        .bg(colors.overlay)
        .border_1()
        .border_color(colors.border)
        .shadow(theme.elevation(Elevation::Floating))
        .text_size(theme.text_size(TextSize::Sm))
}

/// Floats `content` of about `rows` rows under `anchor`, or over it when only above has room; `id` is its owner's.
pub(crate) fn float(
    id: impl Into<ElementId>,
    anchor: Bounds<Pixels>,
    rows: usize,
    content: impl IntoElement,
    window: &Window,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let rem = window.rem_size();
    let row = theme.control_height(ControlSize::Md).to_pixels(rem);
    let height = (row * (rows + 1) as f32).min(theme.list_max_height().to_pixels(rem));
    float_height(id, anchor, height, content, window, cx)
}

/// Whether `height` goes over `anchor`: it does not fit below, and above has more room.
pub(crate) fn opens_up(anchor: Bounds<Pixels>, height: Pixels, viewport: Pixels) -> bool {
    let below = viewport - anchor.bottom();
    height > below && anchor.top() > below
}

/// Floats `content` of a known `height` under `anchor`, or over it when only above has room; `id` is its owner's.
pub(crate) fn float_height(
    id: impl Into<ElementId>,
    anchor: Bounds<Pixels>,
    height: Pixels,
    content: impl IntoElement,
    window: &Window,
    cx: &App,
) -> AnyElement {
    let up = opens_up(anchor, height, window.viewport_size().height);
    let enter =
        Animation::new(motion::duration(motion::FAST, cx)).with_easing(motion::ease_out_cubic);
    let placed = if up {
        anchored()
            .position(anchor.origin)
            .anchor(Anchor::BottomLeft)
            .child(
                div()
                    .child(content)
                    .with_animation("popup-in", enter, |popup, t| {
                        popup.opacity(t).mb(motion::NUDGE * (2.0 - t))
                    }),
            )
    } else {
        anchored()
            .position(anchor.bottom_left())
            .child(
                div()
                    .child(content)
                    .with_animation("popup-in", enter, |popup, t| {
                        popup.opacity(t).mt(motion::NUDGE * (2.0 - t))
                    }),
            )
    };
    raise(id, placed.snap_to_window())
        .with_priority(1)
        .into_any_element()
}

/// Scrolls child `ix` of `handle`'s box into view once the box has its bounds, asks one more frame, then runs `done`. Put it inside the box.
pub(crate) fn revealer(handle: &ScrollHandle, ix: usize, done: Run) -> impl IntoElement + use<> {
    let handle = handle.clone();
    canvas(
        move |_, window, cx| {
            handle.scroll_to_item(ix);
            window.request_animation_frame();
            done(window, cx);
        },
        |_, _, _, _| {},
    )
    .absolute()
}

/// The row a list scrolls into view: `at` while `shown` and not yet revealed there. A hidden list forgets.
pub(crate) fn reveal<T: 'static>(
    state: &Entity<T>,
    revealed: fn(&mut T) -> &mut Option<usize>,
    shown: bool,
    at: usize,
    cx: &mut App,
) -> Option<(usize, Run)> {
    let last = state.update(cx, |state, _| {
        let last = revealed(state);
        if !shown {
            *last = None;
        }
        *last
    });
    (shown && last != Some(at)).then(|| {
        let state = state.clone();
        let mark: Run =
            Rc::new(move |_, cx| state.update(cx, |state, _| *revealed(state) = Some(at)));
        (at, mark)
    })
}

/// A list floating under `anchor`, at least as wide. Rows leave focus where it was.
pub(crate) struct Popup<'a> {
    pub id: ElementId,
    pub anchor: Bounds<Pixels>,
    pub rows: &'a [Choice],
    pub highlighted: Option<usize>,
    pub checked: Option<&'a [SharedString]>,
    pub pick: Pick,
    pub dismiss: Option<Run>,
    pub scroll: Option<&'a ScrollHandle>,
    /// A row to scroll into view once the list has its bounds, and what marks it done.
    pub reveal: Option<(usize, Run)>,
}

impl Popup<'_> {
    pub fn render(self, window: &Window, cx: &App) -> AnyElement {
        let theme = cx.theme();
        let (anchor, pick) = (self.anchor, self.pick);
        let rows = self.rows.iter().enumerate().map(|(ix, choice)| {
            let pick = pick.clone();
            let checked = self.checked.map(|values| values.contains(&choice.value));
            option_row(
                ("option", ix),
                choice,
                self.highlighted == Some(ix),
                checked,
                cx,
            )
            .when(!choice.disabled, |row| {
                row.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    window.prevent_default();
                    cx.stop_propagation();
                    pick(ix, window, cx);
                })
            })
        });
        let list = surface(self.id.clone(), cx)
            .debug_selector(|| "option-list".into())
            .min_w(theme.tooltip_max_width())
            .when(anchor.size.width > Pixels::ZERO, |list| {
                list.min_w(anchor.size.width)
            })
            .max_w(window.viewport_size().width - theme.window_margin() * 2.0)
            .max_h(theme.list_max_height())
            .occlude()
            .overflow_y_scroll()
            .when_some(self.scroll, |list, handle| list.track_scroll(handle))
            .p_1()
            .flex()
            .flex_col()
            .when_some(self.dismiss, |list, dismiss| {
                list.on_mouse_down_out(move |_, window, cx| dismiss(window, cx))
            })
            .children(rows)
            .when_some(
                self.scroll.zip(self.reveal),
                |list, (handle, (ix, done))| list.child(revealer(handle, ix, done)),
            );
        float(self.id, anchor, self.rows.len(), list, window, cx)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, point, px, size};

    use super::{Choice, opens_up, step};

    #[test]
    fn a_panel_opens_up_only_when_it_does_not_fit_below_and_above_has_more() {
        let low = Bounds::new(point(px(0.0), px(600.0)), size(px(80.0), px(28.0)));
        assert!(opens_up(low, px(260.0), px(790.0)));
        assert!(!opens_up(low, px(120.0), px(790.0)));
        let high = Bounds::new(point(px(0.0), px(100.0)), size(px(80.0), px(28.0)));
        assert!(!opens_up(high, px(700.0), px(790.0)));
    }

    #[test]
    fn stepping_wraps_and_skips_disabled_rows() {
        let rows = [
            Choice::new("a", "A"),
            Choice::new("b", "B").disabled(),
            Choice::new("c", "C"),
        ];
        assert_eq!(step(&rows, 0, 1), 2);
        assert_eq!(step(&rows, 2, 1), 0);
        assert_eq!(step(&rows, 0, -1), 2);
        let none = [Choice::new("a", "A").disabled()];
        assert_eq!(step(&none, 0, 1), 0);
    }
}
