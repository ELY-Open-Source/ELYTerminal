use std::rc::Rc;

use gpui::{
    Animation, AnimationExt, AnyElement, App, ElementId, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, Styled, Window, div, prelude::*,
};
use smallvec::SmallVec;

use super::{options::Run, text::Submit};
use crate::{
    motion,
    primitives::{HelpTooltip, Icon, IconName},
    theme::{ActiveTheme, IconSize, TextSize},
    typography::{Caption, Heading, Label},
};

/// The key context a form sets, so Cmd-Enter reaches it from any control inside.
pub(crate) const FORM_CONTEXT: &str = "ElyForm";

/// Sections stacked with room between them. Cmd-Enter on any field or control inside submits the form.
#[derive(IntoElement)]
pub struct Form {
    id: ElementId,
    children: SmallVec<[AnyElement; 4]>,
    on_submit: Option<Run>,
}

impl Form {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            children: SmallVec::new(),
            on_submit: None,
        }
    }

    pub fn on_submit(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_submit = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for Form {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Form {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        div()
            .id(self.id)
            .key_context(FORM_CONTEXT)
            .flex()
            .flex_col()
            .gap_8()
            .when_some(self.on_submit, |form, submit| {
                form.capture_action(move |_: &Submit, window, cx| {
                    cx.stop_propagation();
                    log::info!("form {id:?}: submitted");
                    submit(window, cx);
                })
            })
            .children(self.children)
    }
}

/// A titled group of fields, with a line of context under the title.
#[derive(IntoElement)]
pub struct FormSection {
    title: SharedString,
    description: Option<SharedString>,
    children: SmallVec<[AnyElement; 4]>,
}

impl FormSection {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            description: None,
            children: SmallVec::new(),
        }
    }

    pub fn description(mut self, text: impl Into<SharedString>) -> Self {
        self.description = Some(text.into());
        self
    }
}

impl ParentElement for FormSection {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for FormSection {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(Heading::h4(self.title))
                    .children(self.description.map(Caption::new)),
            )
            .children(self.children)
    }
}

/// A field's name, with a required mark and a help tip when asked.
#[derive(IntoElement)]
pub struct FormLabel {
    id: ElementId,
    text: SharedString,
    required: bool,
    help: Option<SharedString>,
}

impl FormLabel {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            required: false,
            help: None,
        }
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// A tip behind a help mark beside the name.
    pub fn help(mut self, text: impl Into<SharedString>) -> Self {
        self.help = Some(text.into());
        self
    }
}

impl RenderOnce for FormLabel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = &cx.theme().colors;
        div()
            .flex()
            .items_center()
            .gap_1()
            .child(Label::new(self.text))
            .when(self.required, |label| {
                label.child(div().text_color(colors.danger).child("*"))
            })
            .when_some(self.help, |label, help| {
                label.child(HelpTooltip::new((self.id, "help"), help))
            })
    }
}

/// Why a field failed, in the danger color. Each new message eases in.
#[derive(IntoElement)]
pub struct FormError {
    id: ElementId,
    text: SharedString,
}

impl FormError {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
        }
    }
}

impl RenderOnce for FormError {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let danger = theme.colors.danger;
        div()
            .flex()
            .items_center()
            .gap_1()
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(danger)
            .child(
                Icon::new(IconName::CircleAlert)
                    .size(IconSize::Xs)
                    .color(danger),
            )
            .child(self.text.clone())
            .with_animation(
                ElementId::from((self.id, self.text)),
                Animation::new(motion::duration(motion::FAST, cx))
                    .with_easing(motion::ease_out_cubic),
                |error, t| error.opacity(t).mt(motion::NUDGE * (t - 1.0)),
            )
    }
}

/// A labeled control, with a description beneath, or the error once it fails.
#[derive(IntoElement)]
pub struct FormField {
    id: ElementId,
    label: FormLabel,
    description: Option<SharedString>,
    error: Option<SharedString>,
    children: SmallVec<[AnyElement; 2]>,
}

impl FormField {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        let id = id.into();
        Self {
            label: FormLabel::new((id.clone(), "label"), label),
            id,
            description: None,
            error: None,
            children: SmallVec::new(),
        }
    }

    pub fn required(mut self) -> Self {
        self.label = self.label.required();
        self
    }

    pub fn help(mut self, text: impl Into<SharedString>) -> Self {
        self.label = self.label.help(text);
        self
    }

    pub fn description(mut self, text: impl Into<SharedString>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// Shown in place of the description.
    pub fn error(mut self, text: impl Into<SharedString>) -> Self {
        self.error = Some(text.into());
        self
    }
}

impl ParentElement for FormField {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for FormField {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let note = match (self.error, self.description) {
            (Some(error), _) => Some(FormError::new((self.id, "error"), error).into_any_element()),
            (None, Some(description)) => Some(Caption::new(description).into_any_element()),
            (None, None) => None,
        };
        div()
            .flex()
            .flex_col()
            .gap_1p5()
            .child(self.label)
            .children(self.children)
            .children(note)
    }
}

/// Fields and actions in one row, their bottoms aligned; the row wraps when narrow.
#[derive(IntoElement)]
pub struct InlineForm {
    children: SmallVec<[AnyElement; 4]>,
}

impl InlineForm {
    pub fn new() -> Self {
        Self {
            children: SmallVec::new(),
        }
    }
}

impl Default for InlineForm {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for InlineForm {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for InlineForm {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_wrap()
            .items_end()
            .gap_3()
            .children(self.children)
    }
}

/// A dot and a word while there are unsaved changes; nothing once saved.
#[derive(IntoElement)]
pub struct DirtyIndicator {
    id: ElementId,
    dirty: bool,
    label: SharedString,
}

impl DirtyIndicator {
    pub fn new(id: impl Into<ElementId>, dirty: bool) -> Self {
        Self {
            id: id.into(),
            dirty,
            label: SharedString::from("Unsaved changes"),
        }
    }

    pub fn label(mut self, text: impl Into<SharedString>) -> Self {
        self.label = text.into();
        self
    }
}

impl RenderOnce for DirtyIndicator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let shown = self.dirty.then(|| {
            div()
                .flex()
                .items_center()
                .gap_1p5()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(theme.colors.fg_muted)
                .child(
                    div()
                        .size(theme.status_dot())
                        .rounded_full()
                        .bg(theme.colors.warning),
                )
                .child(self.label)
                .with_animation(
                    self.id,
                    Animation::new(motion::duration(motion::BASE, cx))
                        .with_easing(motion::ease_out_cubic),
                    |shown, t| shown.opacity(t),
                )
        });
        div().children(shown)
    }
}
