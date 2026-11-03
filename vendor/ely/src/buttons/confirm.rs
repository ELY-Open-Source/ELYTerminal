use std::{rc::Rc, time::Duration};

use gpui::{
    Animation, AnimationExt, App, ElementId, FontWeight, InteractiveElement, IntoElement,
    MouseButton, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Task,
    Window, div, prelude::*, relative,
};

use super::button::{ButtonVariant, label_size, tone};
use crate::{
    motion,
    primitives::{FocusRing, Icon, IconName},
    theme::{ActiveTheme, ControlSize, Radius},
};

/// How long a hold takes to confirm.
const HOLD: Duration = Duration::from_millis(1200);
/// How long a first click stays armed.
const ARMED: Duration = Duration::from_millis(3000);
/// How long the done state shows.
const DONE: Duration = Duration::from_millis(1400);

/// Hold the button down, or click it twice. Keys always press twice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmMode {
    Hold,
    Twice,
}

#[derive(Default)]
struct Confirm {
    holding: bool,
    armed: bool,
    done: bool,
    generation: u64,
    _timer: Option<Task<()>>,
}

type OnConfirm = Rc<dyn Fn(&mut Window, &mut App)>;

/// A destructive button that asks twice: by holding, or by a second click.
#[derive(IntoElement)]
pub struct ConfirmButton {
    id: ElementId,
    label: SharedString,
    armed_label: SharedString,
    mode: ConfirmMode,
    size: ControlSize,
    on_confirm: Option<OnConfirm>,
}

impl ConfirmButton {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        mode: ConfirmMode,
    ) -> Self {
        let label = label.into();
        Self {
            id: id.into(),
            armed_label: match mode {
                ConfirmMode::Hold => "Press again to confirm".into(),
                ConfirmMode::Twice => "Click again to confirm".into(),
            },
            label,
            mode,
            size: ControlSize::default(),
            on_confirm: None,
        }
    }

    /// The label while waiting for the second click.
    pub fn armed_label(mut self, label: impl Into<SharedString>) -> Self {
        self.armed_label = label.into();
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn on_confirm(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_confirm = Some(Rc::new(handler));
        self
    }
}

/// Clears `field` after `after`, unless a newer change came first.
fn expire(
    state: &gpui::Entity<Confirm>,
    after: Duration,
    clear: fn(&mut Confirm),
    window: &mut Window,
    cx: &mut App,
) -> Task<()> {
    let (weak, generation) = (state.downgrade(), state.read(cx).generation);
    window.spawn(cx, async move |cx| {
        cx.background_executor().timer(after).await;
        let cleared = cx.update(|_, cx| {
            weak.update(cx, |confirm, cx| {
                if confirm.generation == generation {
                    clear(confirm);
                    cx.notify();
                }
            })
        });
        if let Err(error) = cleared.and_then(|inner| inner) {
            log::error!("confirm button: state gone before it expired: {error:#}");
        }
    })
}

fn confirm_now(
    state: &gpui::Entity<Confirm>,
    handler: &Option<OnConfirm>,
    window: &mut Window,
    cx: &mut App,
) {
    log::info!("confirm button: confirmed");
    state.update(cx, |confirm, _| {
        confirm.holding = false;
        confirm.armed = false;
        confirm.done = true;
        confirm.generation += 1;
    });
    let timer = expire(state, DONE, |confirm| confirm.done = false, window, cx);
    state.update(cx, |confirm, cx| {
        confirm._timer = Some(timer);
        cx.notify();
    });
    if let Some(handler) = handler {
        handler(window, cx);
    }
}

impl RenderOnce for ConfirmButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| Confirm::default());
        let (holding, armed, done, generation) = {
            let confirm = state.read(cx);
            (
                confirm.holding,
                confirm.armed,
                confirm.done,
                confirm.generation,
            )
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let variant = if armed {
            ButtonVariant::Danger
        } else {
            ButtonVariant::Outline
        };
        let tone = tone(variant, colors);
        let (text, icon_size) = label_size(self.size);
        let label = if armed {
            self.armed_label.clone()
        } else {
            self.label.clone()
        };
        let fill = colors.danger.opacity(0.16);
        let handler = self.on_confirm.clone();
        let (press, release, out, click) =
            (state.clone(), state.clone(), state.clone(), state.clone());
        let hold = self.mode == ConfirmMode::Hold;
        let (hold_handler, click_handler) = (handler.clone(), handler);
        div()
            .id(self.id)
            .relative()
            .overflow_hidden()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .gap_1p5()
            .h(theme.control_height(self.size))
            .px(theme.control_padding(self.size))
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(if holding { colors.danger } else { tone.border })
            .bg(tone.bg)
            .text_size(theme.text_size(text))
            .font_weight(FontWeight::MEDIUM)
            .text_color(if holding { colors.danger } else { tone.fg })
            .cursor_pointer()
            .tab_index(0)
            .focus_ring(cx)
            .hover(|style| style.bg(tone.hover))
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                window.prevent_default();
                if !hold || press.read(cx).done {
                    return;
                }
                press.update(cx, |confirm, _| {
                    confirm.holding = true;
                    confirm.generation += 1;
                });
                let (weak, handler) = (press.downgrade(), hold_handler.clone());
                let generation = press.read(cx).generation;
                let timer = window.spawn(cx, async move |cx| {
                    cx.background_executor().timer(HOLD).await;
                    let held = cx.update(|window, cx| {
                        let Some(state) = weak.upgrade() else {
                            return;
                        };
                        if state.read(cx).holding && state.read(cx).generation == generation {
                            confirm_now(&state, &handler, window, cx);
                        }
                    });
                    if let Err(error) = held {
                        log::error!("confirm button: window gone during a hold: {error:#}");
                    }
                });
                press.update(cx, |confirm, cx| {
                    confirm._timer = Some(timer);
                    cx.notify();
                });
            })
            .on_mouse_up(MouseButton::Left, move |_, _, cx| let_go(&release, cx))
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| let_go(&out, cx))
            .on_click(move |event, window, cx| {
                if (hold && !event.is_keyboard()) || click.read(cx).done {
                    return;
                }
                if click.read(cx).armed {
                    confirm_now(&click, &click_handler, window, cx);
                    return;
                }
                log::info!("confirm button: armed");
                click.update(cx, |confirm, _| {
                    confirm.armed = true;
                    confirm.generation += 1;
                });
                let timer = expire(&click, ARMED, |confirm| confirm.armed = false, window, cx);
                click.update(cx, |confirm, cx| {
                    confirm._timer = Some(timer);
                    cx.notify();
                });
            })
            .when(holding, |button| {
                button.child(
                    div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left_0()
                        .bg(fill)
                        .with_animation(
                            ("confirm-hold", generation),
                            Animation::new(HOLD),
                            |bar, t| bar.w(relative(t)),
                        ),
                )
            })
            .map(|button| {
                if done {
                    button.child(
                        Icon::new(IconName::Check)
                            .size(icon_size)
                            .color(colors.success)
                            .with_animation(
                                ("confirm-done", generation),
                                Animation::new(motion::duration(motion::FAST, cx))
                                    .with_easing(motion::ease_out_cubic),
                                |icon, t| icon.rotate(gpui::radians((1.0 - t) * -0.6)),
                            ),
                    )
                } else {
                    button
                }
            })
            .child(label)
    }
}

fn let_go(state: &gpui::Entity<Confirm>, cx: &mut App) {
    if !state.read(cx).holding {
        return;
    }
    log::info!("confirm button: released early");
    state.update(cx, |confirm, cx| {
        confirm.holding = false;
        confirm.generation += 1;
        confirm._timer = None;
        cx.notify();
    });
}
