use std::time::Duration;

use gpui::{
    AnyElement, App, Div, ElementId, Entity, FontWeight, InteractiveElement, IntoElement,
    MouseButton, ParentElement, RenderOnce, SharedString, Stateful, StatefulInteractiveElement,
    Styled, Window, div, prelude::*, transparent_black,
};
use smallvec::SmallVec;

use crate::{
    chat::{CodeBlock, StepState, step_mark},
    primitives::{Disclosure, FocusRing, Icon, IconName, Severity},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::{Ellipsis, format::took, tabular},
};

/// A row that opens a body through `toggle`: a Tab stop while it has one.
fn header(id: ElementId, toggle: Option<Entity<bool>>, cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    div()
        .id(id.clone())
        .w_full()
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_2()
        .rounded(theme.radius(Radius::Md))
        .border_1()
        .border_color(transparent_black())
        .when_some(toggle, |row, state| {
            row.tab_index(0)
                .focus_ring(cx)
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                .on_click(move |_, _, cx| {
                    state.update(cx, |open, cx| {
                        *open = !*open;
                        log::info!("tool call: open {open}");
                        cx.notify();
                    })
                })
        })
}

/// One call to a tool: its status, name, what it touched and how long it took; opened, its arguments and what came back.
#[derive(IntoElement)]
pub struct ToolCallCard {
    id: ElementId,
    name: SharedString,
    status: StepState,
    summary: Option<SharedString>,
    took: Option<Duration>,
    arguments: Option<SharedString>,
    result: Option<SharedString>,
}

impl ToolCallCard {
    pub fn new(id: impl Into<ElementId>, name: impl Into<SharedString>, status: StepState) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            status,
            summary: None,
            took: None,
            arguments: None,
            result: None,
        }
    }

    /// A few words on what the call touched, such as a path or a query.
    pub fn summary(mut self, text: impl Into<SharedString>) -> Self {
        self.summary = Some(text.into());
        self
    }

    pub fn took(mut self, took: Duration) -> Self {
        self.took = Some(took);
        self
    }

    /// The arguments, as JSON.
    pub fn arguments(mut self, json: impl Into<SharedString>) -> Self {
        self.arguments = Some(json.into());
        self
    }

    /// What came back, or the error when the call failed.
    pub fn result(mut self, text: impl Into<SharedString>) -> Self {
        self.result = Some(text.into());
        self
    }
}

impl RenderOnce for ToolCallCard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state((self.id.clone(), "open"), cx, |_, _| false);
        let open = *state.read(cx);
        let opens = self.arguments.is_some() || self.result.is_some();
        let failed = self.status == StepState::Failed;
        let mark = step_mark((self.id.clone(), "mark").into(), self.status, window, cx);
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let label = |text: &'static str| {
            div()
                .text_size(theme.text_size(TextSize::Xs))
                .font_weight(FontWeight::MEDIUM)
                .text_color(colors.fg_subtle)
                .child(text)
        };
        div()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                header(
                    (self.id.clone(), "header").into(),
                    opens.then_some(state),
                    cx,
                )
                .child(mark)
                .child(
                    div()
                        .min_w_0()
                        .font_family(theme.mono_family.clone())
                        .text_color(colors.fg)
                        .child(Ellipsis::new(self.name)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_color(colors.fg_muted)
                        .children(self.summary.map(Ellipsis::new)),
                )
                .children(self.took.map(|spent| {
                    tabular(div().flex_none().text_color(colors.fg_subtle)).child(took(spent))
                }))
                .when(opens, |row| {
                    row.child(
                        Disclosure::new((self.id.clone(), "chevron"), open).size(IconSize::Sm),
                    )
                }),
            )
            .when(open && opens, |card| {
                card.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .px_3()
                        .pt_1()
                        .pb_3()
                        .children(self.arguments.map(|json| {
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(label("Arguments"))
                                .child(
                                    CodeBlock::new((self.id.clone(), "arguments"), json)
                                        .language("json"),
                                )
                        }))
                        .children(self.result.map(|text| {
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(label(if failed { "Error" } else { "Result" }))
                                .child(if failed {
                                    div()
                                        .text_color(Severity::Danger.color(&colors))
                                        .child(text)
                                        .into_any_element()
                                } else {
                                    CodeBlock::new((self.id.clone(), "result"), text)
                                        .into_any_element()
                                })
                        })),
                )
            })
    }
}

/// Several tool calls under one line, shut until opened, such as "Read 3 files".
#[derive(IntoElement)]
pub struct ToolCallGroup {
    id: ElementId,
    label: SharedString,
    calls: SmallVec<[AnyElement; 4]>,
}

impl ToolCallGroup {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            calls: SmallVec::new(),
        }
    }
}

impl ParentElement for ToolCallGroup {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.calls.extend(elements);
    }
}

impl RenderOnce for ToolCallGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state((self.id.clone(), "open"), cx, |_, _| false);
        let open = *state.read(cx);
        let theme = cx.theme();
        let colors = theme.colors.clone();
        div()
            .flex()
            .flex_col()
            .gap_2()
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                header((self.id.clone(), "header").into(), Some(state), cx)
                    .px_1()
                    .py_0p5()
                    .text_color(colors.fg_muted)
                    .child(
                        Icon::new(IconName::Wrench)
                            .size(IconSize::Sm)
                            .color(colors.fg_subtle),
                    )
                    .child(div().flex_1().min_w_0().child(Ellipsis::new(self.label)))
                    .child(Disclosure::new((self.id.clone(), "chevron"), open).size(IconSize::Sm)),
            )
            .when(open, |group| {
                group.child(div().flex().flex_col().gap_2().pl_3().children(self.calls))
            })
    }
}
