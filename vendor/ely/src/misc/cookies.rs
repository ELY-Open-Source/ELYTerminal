use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div,
};

use crate::{
    buttons::{Button, ButtonVariant},
    forms::Switch,
    primitives::tab_stop,
    theme::{ActiveTheme, Radius, TextSize},
};

type OnChoose = Rc<dyn Fn(&[SharedString], &mut Window, &mut App)>;

/// Which kinds the viewer turned on, and whether the switches show.
struct Choosing {
    open: bool,
    on: Vec<bool>,
}

/// Cookies a site in the app would set: what they are for, then Accept all, Reject all, or Choose, which lays out a switch for each kind, with Save choices in Choose's place and focus. Necessary cookies stay on.
#[derive(IntoElement)]
pub struct CookieBanner {
    id: ElementId,
    message: SharedString,
    kinds: Vec<(SharedString, SharedString)>,
    on_choose: OnChoose,
}

impl CookieBanner {
    pub fn new(
        id: impl Into<ElementId>,
        message: impl Into<SharedString>,
        on_choose: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            message: message.into(),
            kinds: Vec::new(),
            on_choose: Rc::new(on_choose),
        }
    }

    /// A kind the viewer may turn on: its name, and what it is for.
    pub fn kind(mut self, name: impl Into<SharedString>, detail: impl Into<SharedString>) -> Self {
        self.kinds.push((name.into(), detail.into()));
        self
    }
}

impl RenderOnce for CookieBanner {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        assert!(
            !self.kinds.is_empty(),
            "cookie banner {id:?} has no kinds to choose"
        );
        let on_choose = self.on_choose;
        let count = self.kinds.len();
        let choosing = window.use_keyed_state((id.clone(), "choosing"), cx, move |_, _| Choosing {
            open: false,
            on: vec![false; count],
        });
        if choosing.read(cx).on.len() != count {
            log::error!("cookie banner {id:?}: its kinds changed; each starts off again");
            choosing.update(cx, |choosing, _| choosing.on = vec![false; count]);
        }
        let choose = tab_stop((id.clone(), "choose").into(), true, window, cx);
        let (open, on) = (choosing.read(cx).open, choosing.read(cx).on.clone());
        let theme = cx.theme();
        let colors = &theme.colors;
        let names: Rc<[SharedString]> = self.kinds.iter().map(|(name, _)| name.clone()).collect();
        let answer = move |picked: Vec<SharedString>, window: &mut Window, cx: &mut App| {
            log::info!("cookie banner: {} kinds on", picked.len());
            on_choose(&picked, window, cx);
        };
        let answer = Rc::new(answer);
        let switches = open.then(|| {
            let rows = self.kinds.iter().enumerate().map(|(ix, (name, detail))| {
                let turned = choosing.clone();
                row(
                    Switch::new((id.clone(), format!("kind-{ix}")), on[ix])
                        .label(name.clone())
                        .on_change(move |next, _, cx| {
                            turned.update(cx, |choosing, cx| {
                                choosing.on[ix] = next;
                                cx.notify();
                            })
                        }),
                    detail.clone(),
                    cx,
                )
            });
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child({
                    let (on, rests) = (true, true);
                    div()
                        .debug_selector(|| format!("cookie-necessary-{on}-{rests}"))
                        .child(row(
                            Switch::new((id.clone(), "necessary"), on)
                                .label("Necessary")
                                .disabled(rests),
                            "Keeps the site working; always on.".into(),
                            cx,
                        ))
                })
                .children(rows)
        });
        let (all, none, saved) = (answer.clone(), answer.clone(), answer);
        let (every, chosen, opened) = (names.clone(), names, choosing.clone());
        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div().flex().child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_color(colors.fg)
                        .child(self.message),
                ),
            )
            .children(switches)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Button::new((id.clone(), "accept"), "Accept all")
                            .primary()
                            .on_click(move |_, window, cx| all(every.to_vec(), window, cx)),
                    )
                    .child(
                        Button::new((id.clone(), "reject"), "Reject all")
                            .on_click(move |_, window, cx| none(Vec::new(), window, cx)),
                    )
                    .child(match open {
                        false => Button::new((id.clone(), "choose"), "Choose")
                            .variant(ButtonVariant::Ghost)
                            .focus_handle(&choose)
                            .on_click(move |_, _, cx| {
                                log::info!("cookie banner: choosing");
                                opened.update(cx, |choosing, cx| {
                                    choosing.open = true;
                                    cx.notify();
                                })
                            }),
                        true => Button::new((id, "save"), "Save choices")
                            .variant(ButtonVariant::Ghost)
                            .focus_handle(&choose)
                            .on_click(move |_, window, cx| {
                                let picked = chosen
                                    .iter()
                                    .zip(&on)
                                    .filter(|(_, on)| **on)
                                    .map(|(name, _)| name.clone())
                                    .collect();
                                saved(picked, window, cx)
                            }),
                    }),
            )
    }
}

/// A switch over what its kind is for.
fn row(switch: Switch, detail: SharedString, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    div().flex().flex_col().gap_1().child(switch).child(
        div().flex().child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(theme.colors.fg_muted)
                .child(detail),
        ),
    )
}
