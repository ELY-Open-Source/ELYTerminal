use std::rc::Rc;

use gpui::{
    App, ElementId, ImageSource, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div, prelude::*,
};

use crate::{
    buttons::{Button, ButtonVariant, IconButton},
    feedback::InlineMessage,
    forms::{Enter, Input, TextInput},
    primitives::{FocusRing, Icon, IconName, Image, Severity, checked_ratio, framed, tab_stop},
    theme::{ActiveTheme, ControlSize, IconSize, Radius},
};

type OnAnswer = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
type OnRefresh = Rc<dyn Fn(&mut Window, &mut App)>;

/// Where a captcha stands, as its owner says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CaptchaState {
    #[default]
    Asking,
    /// The owner checks the answer: Verify spins and takes no press, and Enter hands nothing over.
    Checking,
    /// The answer did not match.
    Wrong,
    /// The field rests, and Verify becomes a mark that takes its focus and the field's.
    Passed,
}

/// A picture of characters to copy, from the owner, over a field to type them. Verify or Enter hands the answer to the owner, who checks it and says how it stood; the field empties with each answer and keeps focus, and Verify with nothing typed sends focus to it. With `on_refresh`, a button asks the owner for a new picture.
#[derive(IntoElement)]
pub struct Captcha {
    id: ElementId,
    picture: ImageSource,
    ratio: f32,
    state: CaptchaState,
    on_answer: OnAnswer,
    on_refresh: Option<OnRefresh>,
}

impl Captcha {
    /// `ratio` is the picture's width over its height.
    pub fn new(
        id: impl Into<ElementId>,
        picture: impl Into<ImageSource>,
        ratio: f32,
        on_answer: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            picture: picture.into(),
            ratio: checked_ratio(ratio),
            state: CaptchaState::default(),
            on_answer: Rc::new(on_answer),
            on_refresh: None,
        }
    }

    pub fn state(mut self, state: CaptchaState) -> Self {
        self.state = state;
        self
    }

    /// Asks for a new picture.
    pub fn on_refresh(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_refresh = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Captcha {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let on_answer = self.on_answer;
        let field = window.use_keyed_state((id.clone(), "field"), cx, |window, cx| {
            TextInput::new(window, cx).placeholder("Type the characters")
        });
        let passed = self.state == CaptchaState::Passed;
        if field.read(cx).is_disabled() != passed {
            field.update(cx, |field, cx| field.set_disabled(passed, cx));
        }
        let verify = tab_stop((id.clone(), "verify").into(), !passed, window, cx);
        if passed && field.read(cx).focus().is_focused(window) {
            log::info!("captcha: passed; focus goes to the mark");
            window.focus(&verify, cx);
        }
        let checking = self.state == CaptchaState::Checking;
        let answer = {
            let field = field.clone();
            Rc::new(move |window: &mut Window, cx: &mut App| {
                if checking {
                    return;
                }
                let text = SharedString::from(field.read(cx).text().trim().to_string());
                if text.is_empty() {
                    window.focus(&field.read(cx).focus().clone(), cx);
                    return;
                }
                log::info!("captcha: answered");
                on_answer(&text, window, cx);
                field.update(cx, |field, cx| field.set_text("", cx));
                window.focus(&field.read(cx).focus().clone(), cx);
            })
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let radius = theme.radius(Radius::Md);
        let action = match passed {
            true => div()
                .id((id.clone(), "verified"))
                .debug_selector(|| match verify.is_focused(window) {
                    true => "captcha-verified-focused".into(),
                    false => "captcha-verified".into(),
                })
                .track_focus(&verify)
                .flex()
                .flex_none()
                .items_center()
                .gap_1()
                .h(theme.control_height(ControlSize::Md))
                .px(theme.control_padding(ControlSize::Md))
                .rounded(radius)
                .border_1()
                .border_color(colors.border.alpha(0.0))
                .focus_ring(cx)
                .text_color(colors.success)
                .child(
                    Icon::new(IconName::Check)
                        .size(IconSize::Sm)
                        .color(colors.success),
                )
                .child("Verified")
                .into_any_element(),
            false => {
                let answer = answer.clone();
                Button::new((id.clone(), "verify-button"), "Verify")
                    .variant(ButtonVariant::Primary)
                    .focus_handle(&verify)
                    .loading(checking)
                    .on_click(move |_, window, cx| answer(window, cx))
                    .into_any_element()
            }
        };
        let refresh = self.on_refresh.map(|on_refresh| {
            IconButton::new((id.clone(), "refresh"), IconName::RefreshCw)
                .tooltip("New picture")
                .on_click(move |_, window, cx| {
                    log::info!("captcha: asked for a new picture");
                    on_refresh(window, cx)
                })
        });
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div().flex_1().min_w_0().child(
                            framed(self.ratio, cx).rounded(radius).child(
                                Image::new((id.clone(), "picture"), self.picture)
                                    .size_full()
                                    .rounded(radius),
                            ),
                        ),
                    )
                    .children(refresh),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .capture_action(move |_: &Enter, window, cx| {
                                cx.stop_propagation();
                                answer(window, cx);
                            })
                            .child(Input::new(&field)),
                    )
                    .child(action),
            )
            .when(self.state == CaptchaState::Wrong, |captcha| {
                captcha.child(div().debug_selector(|| "captcha-wrong".into()).child(
                    InlineMessage::new(Severity::Danger, "That did not match; try again."),
                ))
            })
    }
}
