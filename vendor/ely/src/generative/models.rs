use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::*,
};

use crate::{
    buttons::{Button, ButtonVariant},
    data_display::{Badge, DescriptionList, Meter, Tone},
    forms::Run,
    motion::{ProgressBar, Spinner},
    primitives::Severity,
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
    typography::{
        Ellipsis,
        format::{file_size, percent},
    },
};

/// A model to choose: its name and maker, a line on what it does, facts such as its size and license, tags for what it can do, and Use unless it is in use.
#[derive(IntoElement)]
pub struct ModelCard {
    id: ElementId,
    name: SharedString,
    maker: SharedString,
    about: SharedString,
    facts: Vec<(SharedString, SharedString)>,
    tags: Vec<SharedString>,
    in_use: bool,
    on_use: Option<Run>,
}

impl ModelCard {
    pub fn new(
        id: impl Into<ElementId>,
        name: impl Into<SharedString>,
        maker: impl Into<SharedString>,
        about: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            maker: maker.into(),
            about: about.into(),
            facts: Vec::new(),
            tags: Vec::new(),
            in_use: false,
            on_use: None,
        }
    }

    /// A fact and its value, such as "Parameters" and "8B".
    pub fn fact(mut self, label: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        self.facts.push((label.into(), value.into()));
        self
    }

    /// Something it can do, such as "Vision".
    pub fn tag(mut self, tag: impl Into<SharedString>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn in_use(mut self, in_use: bool) -> Self {
        self.in_use = in_use;
        self
    }

    pub fn on_use(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_use = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ModelCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let label = theme.label_width();
        let facts = self
            .facts
            .into_iter()
            .fold(DescriptionList::new(), |list, (label, value)| {
                list.item(label, value)
            });
        let (name, in_use) = (self.name.clone(), self.in_use);
        let choose = self.on_use.filter(|_| !in_use).map(|choose| {
            Button::new((self.id.clone(), "use"), "Use")
                .variant(ButtonVariant::Secondary)
                .size(ControlSize::Sm)
                .on_click(move |_, window, cx| {
                    log::info!("model card: use {name}");
                    choose(window, cx)
                })
        });
        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_start()
                    .gap_x_3()
                    .gap_y_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w(label)
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Base))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.fg)
                                    .child(Ellipsis::new(self.name)),
                            )
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Xs))
                                    .text_color(colors.fg_muted)
                                    .child(Ellipsis::new(self.maker)),
                            ),
                    )
                    .child(
                        div()
                            .flex_none()
                            .children(in_use.then(|| Badge::new("In use").tone(Tone::Success)))
                            .children(choose),
                    ),
            )
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(colors.fg_muted)
                    .child(self.about),
            )
            .child(facts)
            .when(!self.tags.is_empty(), |card| {
                card.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_1()
                        .children(self.tags.into_iter().map(Badge::new)),
                )
            })
    }
}

/// Where a model stands on this machine.
#[derive(Clone, Debug, PartialEq)]
pub enum ModelState {
    Idle,
    Loading(f32),
    Ready,
    Failed(SharedString),
}

impl ModelState {
    fn word(&self) -> String {
        match self {
            Self::Idle => "Not loaded".to_string(),
            Self::Loading(share) => format!("Loading · {}", percent(f64::from(*share), 0, false)),
            Self::Ready => "Loaded".to_string(),
            Self::Failed(_) => "Failed".to_string(),
        }
    }
}

/// A model on this machine: a mark and a word for where it stands, a bar while it loads, and once loaded, the GPU memory it holds.
#[derive(IntoElement)]
pub struct ModelStatus {
    id: ElementId,
    name: SharedString,
    state: ModelState,
    memory: Option<(u64, u64)>,
}

impl ModelStatus {
    pub fn new(id: impl Into<ElementId>, name: impl Into<SharedString>, state: ModelState) -> Self {
        if let ModelState::Loading(share) = state {
            assert!(
                (0.0..=1.0).contains(&share),
                "a model loaded to {share} of 1"
            );
        }
        Self {
            id: id.into(),
            name: name.into(),
            state,
            memory: None,
        }
    }

    /// GPU memory it holds and the card's whole, in bytes.
    pub fn memory(mut self, used: u64, total: u64) -> Self {
        assert!(total > 0, "GPU memory {used} of none");
        if used > total {
            log::error!("model status: GPU memory {used} past {total}; the meter fills");
        }
        self.memory = Some((used, total));
        self
    }
}

impl RenderOnce for ModelStatus {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let tone = match self.state {
            ModelState::Idle => None,
            ModelState::Loading(_) => Some(Severity::Info),
            ModelState::Ready => Some(Severity::Success),
            ModelState::Failed(_) => Some(Severity::Danger),
        };
        let ink = tone.map_or(colors.fg_subtle, |tone| tone.color(&colors));
        let mark = match self.state {
            ModelState::Loading(_) => Spinner::new((self.id.clone(), "loading"))
                .size(IconSize::Xs)
                .color(ink)
                .into_any_element(),
            _ => div()
                .size(theme.status_dot())
                .rounded_full()
                .when(tone.is_some(), |dot| dot.bg(ink))
                .when(tone.is_none(), |dot| dot.border_1().border_color(ink))
                .into_any_element(),
        };
        let below = match (&self.state, self.memory) {
            (ModelState::Loading(share), _) => {
                Some(ProgressBar::new((self.id.clone(), "load"), *share).into_any_element())
            }
            (ModelState::Failed(reason), _) => Some(
                div()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(ink)
                    .child(Ellipsis::new(reason.clone()))
                    .into_any_element(),
            ),
            (ModelState::Ready, Some((used, total))) => Some(
                Meter::new(
                    (self.id.clone(), "memory"),
                    "GPU memory",
                    used.min(total) as f32 / total as f32,
                )
                .detail(format!(
                    "{} of {}",
                    file_size(used, false),
                    file_size(total, false)
                ))
                .into_any_element(),
            ),
            _ => None,
        };
        div()
            .flex()
            .flex_col()
            .gap_2()
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_none()
                            .w(theme.icon_size(IconSize::Xs))
                            .flex()
                            .justify_center()
                            .child(mark),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.fg)
                            .child(Ellipsis::new(self.name)),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(if tone == Some(Severity::Danger) {
                                ink
                            } else {
                                colors.fg_muted
                            })
                            .child(self.state.word()),
                    ),
            )
            .children(below)
    }
}
