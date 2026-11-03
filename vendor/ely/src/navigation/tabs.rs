use std::rc::Rc;

use gpui::{
    Animation, AnimationExt, AnyElement, App, ElementId, FontWeight, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Pixels, Point, RenderOnce, Role, ScrollHandle,
    SharedString, StatefulInteractiveElement, Styled, Window, canvas, div, prelude::*,
};

use crate::{
    forms::{Choice, Pick, step},
    layout::{measure_width, text_width},
    motion::{self, Axis, Marker, glide, measure_item, measure_origin, slide},
    primitives::{Icon, Tooltip, tab_stop},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
};

/// Where a tab strip sits against its panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabPlacement {
    #[default]
    Top,
    Left,
    Bottom,
}

type OnValue = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
type OnContext = Rc<dyn Fn(&SharedString, Point<Pixels>, &mut Window, &mut App)>;

/// A strip of tabs and the chosen tab's panel. A line slides to the chosen tab; arrows move it.
#[derive(IntoElement)]
pub struct Tabs {
    id: ElementId,
    tabs: Vec<Choice>,
    selected: SharedString,
    placement: TabPlacement,
    panel: Option<AnyElement>,
    on_change: Option<OnValue>,
    on_context: Option<OnContext>,
}

impl Tabs {
    pub fn new(
        id: impl Into<ElementId>,
        tabs: impl IntoIterator<Item = Choice>,
        selected: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            tabs: tabs.into_iter().collect(),
            selected: selected.into(),
            placement: TabPlacement::default(),
            panel: None,
            on_change: None,
            on_context: None,
        }
    }

    pub fn placement(mut self, placement: TabPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// The chosen tab's content.
    pub fn panel(mut self, panel: impl IntoElement) -> Self {
        self.panel = Some(panel.into_any_element());
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl Tabs {
    /// A right click on a tab: its value and the pointer.
    pub fn on_context(
        mut self,
        handler: impl Fn(&SharedString, Point<Pixels>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_context = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Tabs {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let values: Vec<SharedString> = self.tabs.iter().map(|tab| tab.value.clone()).collect();
        let chosen = values.iter().position(|value| *value == self.selected);
        let placement = self.placement;
        let axis = match placement {
            TabPlacement::Left => Axis::Vertical,
            TabPlacement::Top | TabPlacement::Bottom => Axis::Horizontal,
        };
        let (state, marker) = slide(
            (self.id.clone(), "slide"),
            &values,
            &self.selected,
            window,
            cx,
        );
        let turn = motion::changes(
            (self.id.clone(), "panel"),
            self.selected.clone(),
            window,
            cx,
        );
        // The chosen tab scrolls in per choice and width.
        let scroll = window
            .use_keyed_state((self.id.clone(), "scroll"), cx, |_, _| ScrollHandle::new())
            .read(cx)
            .clone();
        let followed = window.use_keyed_state((self.id.clone(), "followed"), cx, |_, _| {
            None::<(SharedString, Pixels)>
        });
        let follow =
            (axis == Axis::Horizontal).then(|| (followed, self.selected.clone(), scroll.clone()));
        let focus = tab_stop((self.id.clone(), "focus").into(), true, window, cx);
        let focused = focus.is_focused(window);
        let tabs = Rc::new(self.tabs);
        let pick: Pick = {
            let (id, tabs, on_change) = (self.id.clone(), tabs.clone(), self.on_change);
            Rc::new(move |ix, window, cx| {
                let value = &tabs[ix].value;
                log::info!("tabs {id:?}: {value}");
                if let Some(on_change) = &on_change {
                    on_change(value, window, cx);
                }
            })
        };
        // Last frame's strip width decides icons only.
        let strip_width =
            window.use_keyed_state((self.id.clone(), "strip-width"), cx, |_, _| Pixels::ZERO);
        let shown = *strip_width.read(cx);
        let compact = axis == Axis::Horizontal
            && tabs.iter().all(|tab| tab.icon.is_some())
            && shown > Pixels::ZERO
            && shown < natural(&tabs, window, cx);
        let theme = cx.theme();
        let colors = &theme.colors;
        let (thickness, duration) = (theme.tab_indicator(), motion::duration(motion::SLOW, cx));
        let indicator = marker.map(
            |Marker {
                 from,
                 to,
                 generation,
             }| {
                let bar = div().absolute().bg(colors.accent);
                let bar = match placement {
                    TabPlacement::Top => bar.bottom_0().h(thickness),
                    TabPlacement::Bottom => bar.top_0().h(thickness),
                    TabPlacement::Left => bar.right_0().w(thickness),
                };
                bar.with_animation(
                    ("tab-indicator", generation),
                    Animation::new(duration),
                    move |bar, t| {
                        let (start, length) = glide(from, to, t);
                        match axis {
                            Axis::Horizontal => bar.left(start).w(length),
                            Axis::Vertical => bar.top(start).h(length),
                        }
                    },
                )
            },
        );
        let items = tabs
            .iter()
            .enumerate()
            .map(|(ix, tab)| {
                let on = Some(ix) == chosen;
                let fg = match (tab.disabled, on) {
                    (true, _) => colors.fg_disabled,
                    (false, true) => colors.fg,
                    (false, false) => colors.fg_muted,
                };
                let pick = pick.clone();
                let context = self.on_context.clone().filter(|_| !tab.disabled);
                let value = tab.value.clone();
                div()
                    .id(("tab", ix))
                    .debug_selector(move || format!("tab-{ix}"))
                    .role(Role::Tab)
                    .aria_selected(on)
                    .aria_label(tab.label.clone())
                    .relative()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .h(theme.control_height(ControlSize::Md))
                    .px_3()
                    .rounded(theme.radius(Radius::Md))
                    .border_1()
                    .border_color(if focused && on {
                        colors.focus
                    } else {
                        gpui::transparent_black()
                    })
                    .text_size(theme.text_size(TextSize::Sm))
                    .font_weight(if on {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(fg)
                    .when(!tab.disabled, |item| {
                        item.cursor_pointer()
                            .hover(|style| style.text_color(colors.fg))
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.prevent_default()
                            })
                            .on_click(move |_, window, cx| {
                                if !on {
                                    pick(ix, window, cx);
                                }
                            })
                    })
                    .when_some(context, |item, context| {
                        item.on_mouse_down(MouseButton::Right, move |event, window, cx| {
                            window.prevent_default();
                            cx.stop_propagation();
                            log::info!("tabs: context on {value}");
                            context(&value, event.position, window, cx);
                        })
                    })
                    .when_some(tab.icon, |item, icon| {
                        item.child(Icon::new(icon).size(IconSize::Sm).color(fg))
                    })
                    .when(compact, |item| {
                        item.tooltip(Tooltip::text(tab.label.clone()))
                    })
                    .when(axis == Axis::Horizontal, |item| item.max_w_full())
                    .when(!compact, |item| {
                        item.child(div().min_w_0().truncate().child(tab.label.clone()))
                    })
                    .when_some(tab.note.clone().filter(|_| !compact), |item, note| {
                        item.child(div().text_color(colors.fg_subtle).child(note))
                    })
                    .child(measure_item(state.clone(), ix, axis))
            })
            .collect::<Vec<_>>();
        let (keys, key_pick) = (tabs.clone(), pick.clone());
        let strip = div()
            .id((self.id.clone(), "strip"))
            .role(Role::TabList)
            .track_focus(&focus)
            .relative()
            .flex()
            .when(axis == Axis::Vertical, |strip| strip.flex_col())
            .when(axis == Axis::Horizontal, |strip| {
                strip.min_w_0().overflow_x_scroll().track_scroll(&scroll)
            })
            .gap_1()
            .border_color(colors.border)
            .map(|strip| match placement {
                TabPlacement::Top => strip.border_b_1(),
                TabPlacement::Bottom => strip.border_t_1(),
                TabPlacement::Left => strip.border_r_1().pr_1(),
            })
            .on_key_down(move |event, window, cx| {
                if keys.is_empty() {
                    return;
                }
                let back = if axis == Axis::Vertical { "up" } else { "left" };
                let ahead = if axis == Axis::Vertical {
                    "down"
                } else {
                    "right"
                };
                let key = event.keystroke.key.as_str();
                let (from_back, from_ahead) = match chosen {
                    Some(at) => (at, at),
                    None => (0, keys.len() - 1),
                };
                let to = match key {
                    key if key == back => step(&keys, from_back, -1),
                    key if key == ahead => step(&keys, from_ahead, 1),
                    "home" => step(&keys, keys.len() - 1, 1),
                    "end" => step(&keys, 0, -1),
                    _ => return,
                };
                cx.stop_propagation();
                if Some(to) != chosen && !keys[to].disabled {
                    key_pick(to, window, cx);
                }
            })
            // Items first: a child's index is its tab's.
            .children(items)
            .child(measure_width(strip_width))
            .child(measure_origin(state, axis))
            .children(indicator)
            .children(follow.map(|(followed, selected, scroll)| {
                canvas(
                    move |_, window, cx| {
                        let now = (selected, scroll.bounds().size.width);
                        if now.1 <= Pixels::ZERO || followed.read(cx).as_ref() == Some(&now) {
                            return;
                        }
                        followed.update(cx, |followed, _| *followed = Some(now));
                        if let Some(ix) = chosen {
                            scroll.scroll_to_item(ix);
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
            }));
        let panel = self.panel.map(|panel| {
            div().flex_1().min_w_0().child(panel).with_animation(
                ("tab-panel", turn),
                Animation::new(motion::duration(motion::BASE, cx))
                    .with_easing(motion::ease_out_cubic),
                move |panel, t| if turn == 0 { panel } else { panel.opacity(t) },
            )
        });
        let frame = div().id(self.id).flex().min_w_0().gap_4();
        match placement {
            TabPlacement::Top => frame.flex_col().child(strip).children(panel),
            TabPlacement::Bottom => frame.flex_col().children(panel).child(strip),
            TabPlacement::Left => frame.child(strip).children(panel),
        }
    }
}

/// Every tab's icon, label and note in full: the width that cuts none.
fn natural(tabs: &[Choice], window: &Window, cx: &App) -> Pixels {
    let theme = cx.theme();
    let rem = window.rem_size();
    let pad = (gpui::rems(0.75).to_pixels(rem) + theme.hairline()) * 2.0;
    let glyph = theme.icon_size(IconSize::Sm).to_pixels(rem) + gpui::rems(0.375).to_pixels(rem);
    let gap = gpui::rems(0.25).to_pixels(rem);
    tabs.iter().fold(Pixels::ZERO, |sum, tab| {
        let text =
            |text: &SharedString| text_width(text, TextSize::Sm, FontWeight::MEDIUM, window, cx);
        let note = tab.note.as_ref().map_or(Pixels::ZERO, |note| {
            text(note) + gpui::rems(0.375).to_pixels(rem)
        });
        let icon = if tab.icon.is_some() {
            glyph
        } else {
            Pixels::ZERO
        };
        sum + pad + icon + text(&tab.label) + note + gap
    })
}
