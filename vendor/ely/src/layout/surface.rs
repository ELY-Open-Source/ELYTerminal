use gpui::{
    AnyElement, App, Div, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, div, prelude::*,
};
use smallvec::SmallVec;

use crate::theme::{ActiveTheme, ControlSize, Radius, TextSize};

macro_rules! container_parts {
    ($name:ident) => {
        impl Styled for $name {
            fn style(&mut self) -> &mut StyleRefinement {
                self.base.style()
            }
        }

        impl ParentElement for $name {
            fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
                self.body.extend(elements);
            }
        }
    };
}

fn heading(
    title: SharedString,
    description: Option<SharedString>,
    size: TextSize,
    cx: &App,
) -> impl IntoElement + use<> {
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .gap_0p5()
        .min_w_0()
        .child(
            div()
                .text_size(theme.text_size(size))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.colors.fg)
                .child(title),
        )
        .when_some(description, |block, text| {
            block.child(
                div()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(theme.colors.fg_muted)
                    .child(text),
            )
        })
}

/// Title row for a `Card`: title, note, one trailing action.
#[derive(IntoElement)]
pub struct CardHeader {
    title: SharedString,
    description: Option<SharedString>,
    action: Option<AnyElement>,
}

impl CardHeader {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            description: None,
            action: None,
        }
    }

    pub fn description(mut self, text: impl Into<SharedString>) -> Self {
        self.description = Some(text.into());
        self
    }

    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.action = Some(action.into_any_element());
        self
    }
}

impl RenderOnce for CardHeader {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .items_start()
            .justify_between()
            .gap_4()
            .child(heading(self.title, self.description, TextSize::Md, cx))
            .children(self.action)
    }
}

/// Flat surface: header, body, footer. Hairline, no shadow.
#[derive(IntoElement)]
pub struct Card {
    base: Div,
    header: Option<CardHeader>,
    footer: Option<AnyElement>,
    body: SmallVec<[AnyElement; 2]>,
}

impl Card {
    pub fn new() -> Self {
        Self {
            base: div(),
            header: None,
            footer: None,
            body: SmallVec::new(),
        }
    }

    pub fn header(mut self, header: CardHeader) -> Self {
        self.header = Some(header);
        self
    }

    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }
}

impl Default for Card {
    fn default() -> Self {
        Self::new()
    }
}

container_parts!(Card);

impl RenderOnce for Card {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        self.base
            .flex()
            .flex_col()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.surface)
            .when_some(self.header, |card, header| {
                card.child(div().px_5().pt_5().child(header))
            })
            .child(div().p_5().flex().flex_col().gap_3().children(self.body))
            .when_some(self.footer, |card, footer| {
                card.child(
                    div()
                        .flex()
                        .justify_end()
                        .gap_2()
                        .px_5()
                        .py_3()
                        .border_t_1()
                        .border_color(theme.colors.border)
                        .child(footer),
                )
            })
    }
}

/// App region with a header bar: title and actions.
#[derive(IntoElement)]
pub struct Panel {
    base: Div,
    title: SharedString,
    actions: SmallVec<[AnyElement; 2]>,
    body: SmallVec<[AnyElement; 2]>,
}

impl Panel {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            base: div(),
            title: title.into(),
            actions: SmallVec::new(),
            body: SmallVec::new(),
        }
    }

    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }
}

container_parts!(Panel);

impl RenderOnce for Panel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        self.base
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.surface)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(theme.control_height(ControlSize::Lg))
                    .pl_3()
                    .pr_1()
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.colors.fg_muted)
                            .child(self.title),
                    )
                    .child(div().flex().items_center().gap_0p5().children(self.actions)),
            )
            .child(div().flex_1().children(self.body))
    }
}

/// Titled block of a page, with an optional note and action.
#[derive(IntoElement)]
pub struct Section {
    base: Div,
    title: SharedString,
    description: Option<SharedString>,
    action: Option<AnyElement>,
    body: SmallVec<[AnyElement; 2]>,
}

impl Section {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            base: div(),
            title: title.into(),
            description: None,
            action: None,
            body: SmallVec::new(),
        }
    }

    pub fn description(mut self, text: impl Into<SharedString>) -> Self {
        self.description = Some(text.into());
        self
    }

    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.action = Some(action.into_any_element());
        self
    }
}

container_parts!(Section);

impl RenderOnce for Section {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        self.base
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_end()
                    .justify_between()
                    .gap_4()
                    .child(heading(self.title, self.description, TextSize::Lg, cx))
                    .children(self.action),
            )
            .children(self.body)
    }
}

/// Named group of fields inside a hairline.
#[derive(IntoElement)]
pub struct Fieldset {
    base: Div,
    legend: SharedString,
    body: SmallVec<[AnyElement; 2]>,
}

impl Fieldset {
    pub fn new(legend: impl Into<SharedString>) -> Self {
        Self {
            base: div(),
            legend: legend.into(),
            body: SmallVec::new(),
        }
    }
}

container_parts!(Fieldset);

impl RenderOnce for Fieldset {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        self.base
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Xs))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.colors.fg_muted)
                    .child(self.legend),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .rounded(theme.radius(Radius::Lg))
                    .border_1()
                    .border_color(theme.colors.border)
                    .children(self.body),
            )
    }
}

/// Bordered region, no fill.
#[derive(IntoElement)]
pub struct Frame {
    base: Div,
    body: SmallVec<[AnyElement; 2]>,
}

impl Frame {
    pub fn new() -> Self {
        Self {
            base: div(),
            body: SmallVec::new(),
        }
    }
}

impl Default for Frame {
    fn default() -> Self {
        Self::new()
    }
}

container_parts!(Frame);

impl RenderOnce for Frame {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        self.base
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(theme.colors.border_strong)
            .p_4()
            .children(self.body)
    }
}

/// Recessed region on the sunken tone.
#[derive(IntoElement)]
pub struct Well {
    base: Div,
    body: SmallVec<[AnyElement; 2]>,
}

impl Well {
    pub fn new() -> Self {
        Self {
            base: div(),
            body: SmallVec::new(),
        }
    }
}

impl Default for Well {
    fn default() -> Self {
        Self::new()
    }
}

container_parts!(Well);

impl RenderOnce for Well {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        self.base
            .rounded(theme.radius(Radius::Lg))
            .bg(theme.colors.sunken)
            .border_1()
            .border_color(theme.colors.border)
            .p_4()
            .children(self.body)
    }
}
