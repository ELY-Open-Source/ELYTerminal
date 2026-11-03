use std::rc::Rc;

use gpui::{
    App, Bounds, ElementId, Hsla, InteractiveElement, IntoElement, MouseButton, ParentElement,
    Pixels, RenderOnce, StatefulInteractiveElement, Styled, Window, canvas, div, prelude::*,
    transparent_black,
};

use crate::{
    data_display::star,
    primitives::tab_stop,
    theme::{ActiveTheme, IconSize, Radius},
};

fn paint_star(bounds: Bounds<Pixels>, filled: bool, color: Hsla, window: &mut Window) {
    if filled {
        window.paint_path(star(bounds, true), color);
    }
    window.paint_path(star(bounds, false), color);
}

type OnRate = Rc<dyn Fn(u8, &mut Window, &mut App)>;

/// Stars from one to `max`. Hover previews; clicking the current value clears it.
#[derive(IntoElement)]
pub struct Rating {
    id: ElementId,
    value: u8,
    max: u8,
    disabled: bool,
    on_change: Option<OnRate>,
}

impl Rating {
    pub fn new(id: impl Into<ElementId>, value: u8) -> Self {
        Self {
            id: id.into(),
            value,
            max: 5,
            disabled: false,
            on_change: None,
        }
    }

    pub fn max(mut self, max: u8) -> Self {
        assert!(max > 0, "a rating needs at least one star");
        self.max = max;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(u8, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Rating {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (value, max) = (self.value, self.max);
        assert!(value <= max, "rating {value} is above {max}");
        let focus = tab_stop(
            (self.id.clone(), "focus").into(),
            !self.disabled,
            window,
            cx,
        );
        let focused = focus.is_focused(window);
        let hover = window.use_keyed_state((self.id.clone(), "hover"), cx, |_, _| None::<u8>);
        let shown = hover.read(cx).unwrap_or(value);
        let rate: OnRate = {
            let (id, on_change) = (self.id.clone(), self.on_change);
            Rc::new(move |next, window, cx| {
                log::info!("rating {id:?}: {next}");
                if let Some(on_change) = &on_change {
                    on_change(next, window, cx);
                }
            })
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let (on, off) = (colors.accent, colors.border_strong);
        let stars: Vec<_> = (1..=max)
            .map(|n| {
                let filled = n <= shown;
                let color = if filled { on } else { off };
                let (hover, rate) = (hover.clone(), rate.clone());
                div()
                    .id(("star", n as usize))
                    .size(theme.icon_size(IconSize::Lg))
                    .child(
                        canvas(
                            |_, _, _| {},
                            move |bounds, _, window, _| paint_star(bounds, filled, color, window),
                        )
                        .size_full(),
                    )
                    .when(!self.disabled, |star| {
                        star.cursor_pointer()
                            .on_hover(move |inside, _, cx| {
                                if *inside {
                                    hover.update(cx, |hover, cx| {
                                        *hover = Some(n);
                                        cx.notify();
                                    });
                                }
                            })
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.prevent_default()
                            })
                            .on_click(move |_, window, cx| {
                                rate(if n == value { 0 } else { n }, window, cx)
                            })
                    })
            })
            .collect();
        let (leave, keys) = (hover, rate);
        div()
            .id(self.id)
            .track_focus(&focus)
            .flex()
            .gap_0p5()
            .p_0p5()
            .rounded(theme.radius(Radius::Sm))
            .border_1()
            .border_color(if focused {
                colors.focus
            } else {
                transparent_black()
            })
            .when(self.disabled, |row| row.opacity(0.5))
            .on_hover(move |inside, _, cx| {
                if !*inside {
                    leave.update(cx, |hover, cx| {
                        *hover = None;
                        cx.notify();
                    });
                }
            })
            .when(!self.disabled, |row| {
                row.on_key_down(move |event, window, cx| {
                    let next = match event.keystroke.key.as_str() {
                        "left" | "down" => value.saturating_sub(1),
                        "right" | "up" => (value + 1).min(max),
                        "home" => 0,
                        "end" => max,
                        _ => return,
                    };
                    cx.stop_propagation();
                    if next != value {
                        keys(next, window, cx);
                    }
                })
            })
            .children(stars)
    }
}
