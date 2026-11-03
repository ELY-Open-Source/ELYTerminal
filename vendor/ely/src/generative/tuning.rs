use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div,
};

use crate::{
    buttons::{Button, ButtonVariant},
    charts::{LineChart, Series},
    data_display::{Badge, Tone},
    forms::Run,
    motion::ProgressBar,
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
    typography::{Ellipsis, format::percent, tabular},
};

/// Where a fine-tuning job stands.
#[derive(Clone, Debug, PartialEq)]
pub enum TunePhase {
    /// Waiting, with this many ahead of it.
    Queued(usize),
    /// Training: the epoch under way from one, of how many, and the share of it done.
    Running(usize, usize, f32),
    Done,
    Failed(SharedString),
    Canceled,
}

impl TunePhase {
    fn badge(&self) -> (&'static str, Tone) {
        match self {
            Self::Queued(_) => ("Queued", Tone::Neutral),
            Self::Running(..) => ("Training", Tone::Info),
            Self::Done => ("Done", Tone::Success),
            Self::Failed(_) => ("Failed", Tone::Danger),
            Self::Canceled => ("Canceled", Tone::Neutral),
        }
    }
}

/// A running job's bar, whole epochs and the share of the one under way, and the line under it.
pub(crate) fn progress(epoch: usize, epochs: usize, share: f32) -> (f32, String) {
    let whole = ((epoch - 1) as f32 + share) / epochs as f32;
    let line = format!(
        "Epoch {epoch} of {epochs} · {}",
        percent(f64::from(share), 0, false)
    );
    (whole, line)
}

/// A fine-tuning job: its name, the base model and data it learns from, where it stands, how far it has come by epoch, and its loss on training and held-out data. Cancel while it waits or trains; Open once done.
#[derive(IntoElement)]
pub struct FineTuneJobCard {
    id: ElementId,
    name: SharedString,
    base: SharedString,
    data: SharedString,
    phase: TunePhase,
    train: Vec<f64>,
    held: Vec<f64>,
    on_cancel: Option<Run>,
    on_open: Option<Run>,
}

impl FineTuneJobCard {
    pub fn new(
        id: impl Into<ElementId>,
        name: impl Into<SharedString>,
        base: impl Into<SharedString>,
        data: impl Into<SharedString>,
        phase: TunePhase,
    ) -> Self {
        let mut phase = phase;
        if let TunePhase::Running(epoch, epochs, share) = &mut phase {
            assert!(
                *epochs > 0 && (0.0..=1.0).contains(share),
                "epoch {epoch} of {epochs} at {share}"
            );
            if !(1..=*epochs).contains(epoch) {
                log::error!("fine-tuning: epoch {epoch} of {epochs}; pegged");
                *epoch = (*epoch).clamp(1, *epochs);
            }
        }
        Self {
            id: id.into(),
            name: name.into(),
            base: base.into(),
            data: data.into(),
            phase,
            train: Vec::new(),
            held: Vec::new(),
            on_cancel: None,
            on_open: None,
        }
    }

    /// Loss at each checkpoint, on training data and on held-out data, one value each.
    pub fn loss(mut self, train: impl Into<Vec<f64>>, held: impl Into<Vec<f64>>) -> Self {
        let (train, held) = (train.into(), held.into());
        assert_eq!(
            train.len(),
            held.len(),
            "a loss for each checkpoint on both sets"
        );
        self.train = train;
        self.held = held;
        self
    }

    pub fn on_cancel(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_cancel = Some(Rc::new(handler));
        self
    }

    pub fn on_open(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for FineTuneJobCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let (word, tone) = self.phase.badge();
        let live = matches!(self.phase, TunePhase::Queued(_) | TunePhase::Running(..));
        let action = |key: &'static str, label: &'static str, run: Option<Run>| {
            run.map(|run| {
                Button::new((self.id.clone(), key), label)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .on_click(move |_, window, cx| {
                        log::info!("fine-tune job: {key}");
                        run(window, cx)
                    })
            })
        };
        let actions = match self.phase {
            TunePhase::Done => action("open", "Open", self.on_open),
            _ if live => action("cancel", "Cancel", self.on_cancel),
            _ => None,
        };
        let progress = match &self.phase {
            TunePhase::Running(epoch, epochs, share) => {
                let (whole, line) = progress(*epoch, *epochs, *share);
                Some(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1p5()
                        .child(match *epochs {
                            1 => ProgressBar::new((self.id.clone(), "epochs"), whole),
                            _ => ProgressBar::new((self.id.clone(), "epochs"), whole)
                                .segments(*epochs),
                        })
                        .child(
                            tabular(div())
                                .text_size(theme.text_size(TextSize::Xs))
                                .text_color(colors.fg_muted)
                                .child(line),
                        )
                        .into_any_element(),
                )
            }
            TunePhase::Queued(ahead) => Some(
                div()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.fg_muted)
                    .child(format!("{ahead} ahead"))
                    .into_any_element(),
            ),
            TunePhase::Failed(reason) => Some(
                div()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.danger)
                    .child(reason.clone())
                    .into_any_element(),
            ),
            _ => None,
        };
        let chart = (self.train.len() > 1).then(|| {
            let steps: Vec<SharedString> = (1..=self.train.len())
                .map(|step| step.to_string().into())
                .collect();
            LineChart::new((self.id.clone(), "loss"), steps)
                .series(Series::new("Training", self.train))
                .series(Series::new("Held out", self.held))
                .format(|value| format!("{value:.2}"))
                .h(theme.generative().loss)
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
                            .min_w(theme.label_width())
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Sm))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(colors.fg)
                                    .child(Ellipsis::new(self.name)),
                            )
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Xs))
                                    .text_color(colors.fg_muted)
                                    .child(Ellipsis::new(format!("{} · {}", self.base, self.data))),
                            ),
                    )
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(Badge::new(word).tone(tone))
                            .children(actions),
                    ),
            )
            .children(progress)
            .children(chart)
    }
}

#[cfg(test)]
mod tests {
    use super::progress;

    #[test]
    fn the_bar_counts_whole_epochs_and_the_share_of_the_one_under_way() {
        let (whole, line) = progress(2, 3, 0.64);
        assert!((whole - 0.5467).abs() < 1e-4, "{whole}");
        assert_eq!(line, "Epoch 2 of 3 · 64%");
        assert_eq!(progress(1, 1, 0.5), (0.5, "Epoch 1 of 1 · 50%".to_string()));
        assert_eq!(progress(3, 3, 1.0).0, 1.0);
    }
}
