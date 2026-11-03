use std::rc::Rc;

use gpui::{
    Animation, AnimationExt, AnyElement, App, Div, ElementId, FontWeight, IntoElement,
    ParentElement, RenderOnce, SharedString, Styled, Window, div, prelude::*,
};
use smallvec::SmallVec;

use crate::{
    buttons::Button,
    forms::Run,
    motion,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::tabular,
};

/// The words and buttons under a state's head.
pub(super) struct Words {
    title: SharedString,
    pub(super) body: Option<SharedString>,
    detail: Option<SharedString>,
    pub(super) actions: SmallVec<[AnyElement; 2]>,
}

impl Words {
    pub(super) fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            body: None,
            detail: None,
            actions: SmallVec::new(),
        }
    }
}

/// `block`, centered and no wider than prose; a flex row sets its width before its text wraps.
fn measure(block: Div, cx: &App) -> Div {
    div()
        .w_full()
        .flex()
        .justify_center()
        .child(block.flex_1().min_w_0().max_w(cx.theme().prose_width()))
}

/// A head, a title, a line, a detail and actions, centered; it rises in once.
pub(super) fn state_view(
    id: ElementId,
    head: AnyElement,
    words: Words,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let colors = &theme.colors;
    div()
        .debug_selector(|| "state-root".into())
        .flex()
        .flex_col()
        .items_center()
        .gap_4()
        .px_6()
        .py_8()
        .child(head)
        .child(measure(
            div()
                .text_center()
                .child(
                    div()
                        .text_size(theme.text_size(TextSize::Lg))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(colors.fg)
                        .child(words.title),
                )
                .when_some(words.body, |text, body| {
                    text.child(
                        div()
                            .mt_1()
                            .text_size(theme.text_size(TextSize::Base))
                            .text_color(colors.fg_muted)
                            .child(body),
                    )
                }),
            cx,
        ))
        .when_some(words.detail, |state, detail| {
            state.child(measure(
                div()
                    .px_3()
                    .py_2()
                    .rounded(theme.radius(Radius::Md))
                    .bg(colors.sunken)
                    .font_family(theme.mono_family.clone())
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.fg_muted)
                    .child(detail),
                cx,
            ))
        })
        .when(!words.actions.is_empty(), |state| {
            state.child(div().flex().gap_2().children(words.actions))
        })
        .with_animation(
            id,
            Animation::new(motion::duration(motion::SLOW, cx)).with_easing(motion::ease_out_cubic),
            |state, t| state.opacity(t).mt(motion::NUDGE * 2.0 * (1.0 - t)),
        )
}

/// An icon on a soft disc, the head of an empty or broken view.
pub(super) fn disc(icon: IconName, cx: &App) -> AnyElement {
    let theme = cx.theme();
    div()
        .flex()
        .items_center()
        .justify_center()
        .size(theme.icon_size(IconSize::Xxl) + theme.icon_size(IconSize::Md))
        .rounded_full()
        .bg(theme.colors.sunken)
        .child(
            Icon::new(icon)
                .size(IconSize::Lg)
                .color(theme.colors.fg_subtle),
        )
        .into_any_element()
}

macro_rules! words_builders {
    () => {
        pub fn body(mut self, text: impl Into<SharedString>) -> Self {
            self.words.body = Some(text.into());
            self
        }

        pub fn action(mut self, action: impl IntoElement) -> Self {
            self.words.actions.push(action.into_any_element());
            self
        }
    };
}
pub(super) use words_builders;

/// What a view shows with nothing in it: an icon on a soft disc, a title, a line of help and actions. It rises in when it appears.
#[derive(IntoElement)]
pub struct EmptyState {
    id: ElementId,
    icon: IconName,
    words: Words,
}

impl EmptyState {
    pub fn new(id: impl Into<ElementId>, icon: IconName, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            icon,
            words: Words::new(title),
        }
    }

    words_builders!();
}

impl RenderOnce for EmptyState {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        state_view(self.id, disc(self.icon, cx), self.words, cx)
    }
}

enum Head {
    Code(SharedString),
    Icon(IconName),
}

/// A view that could not show what was asked: a status code or an icon, what happened, and a way on.
#[derive(IntoElement)]
pub struct ErrorView {
    id: ElementId,
    head: Head,
    words: Words,
}

impl ErrorView {
    pub fn new(id: impl Into<ElementId>, icon: IconName, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            head: Head::Icon(icon),
            words: Words::new(title),
        }
    }

    /// Headed by a status code, such as 404, instead of an icon.
    pub fn with_code(
        id: impl Into<ElementId>,
        code: impl Into<SharedString>,
        title: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            head: Head::Code(code.into()),
            words: Words::new(title),
        }
    }

    pub fn not_found(id: impl Into<ElementId>) -> Self {
        Self::with_code(id, "404", "Page not found")
            .body("The link may be broken, or the page has moved.")
    }

    pub fn no_permission(id: impl Into<ElementId>) -> Self {
        Self::with_code(id, "403", "You don't have access")
            .body("Ask an owner to share it with you.")
    }

    pub fn maintenance(id: impl Into<ElementId>) -> Self {
        Self::new(id, IconName::Wrench, "Down for maintenance").body("We'll be back shortly.")
    }

    pub fn offline(id: impl Into<ElementId>) -> Self {
        Self::new(id, IconName::WifiOff, "You're offline")
            .body("Check your connection. Your work stays on this device.")
    }

    /// The error itself, in a quiet mono box.
    pub fn detail(mut self, text: impl Into<SharedString>) -> Self {
        self.words.detail = Some(text.into());
        self
    }

    words_builders!();
}

impl RenderOnce for ErrorView {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let head = match self.head {
            Head::Icon(icon) => disc(icon, cx),
            Head::Code(code) => {
                let theme = cx.theme();
                tabular(div())
                    .text_size(theme.text_size(TextSize::Display))
                    .font_weight(FontWeight::LIGHT)
                    .text_color(theme.colors.fg_subtle)
                    .child(code)
                    .into_any_element()
            }
        };
        state_view(self.id, head, self.words, cx)
    }
}

/// Shows its content, or an `ErrorView` of the error it holds, logged once per error. It never catches panics.
#[derive(IntoElement)]
pub struct ErrorBoundary {
    id: ElementId,
    content: anyhow::Result<AnyElement>,
    on_retry: Option<Run>,
}

impl ErrorBoundary {
    pub fn new<E: IntoElement>(id: impl Into<ElementId>, content: anyhow::Result<E>) -> Self {
        Self {
            id: id.into(),
            content: content.map(IntoElement::into_any_element),
            on_retry: None,
        }
    }

    pub fn on_retry(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_retry = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ErrorBoundary {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let error = match self.content {
            Ok(content) => return content,
            Err(error) => error,
        };
        let chain = format!("{error:#}");
        let logged = window.use_keyed_state((self.id.clone(), "logged"), cx, |_, _| None::<String>);
        if logged.read(cx).as_deref() != Some(chain.as_str()) {
            log::error!("error boundary {:?}: {chain}", self.id);
            logged.update(cx, |logged, _| *logged = Some(chain.clone()));
        }
        let retry = (self.id.clone(), "retry");
        ErrorView::new(self.id, IconName::CircleAlert, "Something went wrong")
            .body(error.to_string())
            .detail(chain)
            .when_some(self.on_retry, |view, run| {
                view.action(
                    Button::new(retry, "Try again").on_click(move |_, window, cx| run(window, cx)),
                )
            })
            .into_any_element()
    }
}
