use std::rc::Rc;

use gpui::{App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Window, div};

use crate::{
    lists::{ListItem, SelectableList},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize},
};

type OnStep = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// What was done, oldest first, with the document at step `at`: the steps after it were undone and read quiet. Enter or a double press on a step takes the document back or forward to it.
#[derive(IntoElement)]
pub struct HistoryPanel {
    id: ElementId,
    steps: Vec<SharedString>,
    at: usize,
    on_step: Option<OnStep>,
}

impl HistoryPanel {
    /// `at` counts the steps in force; none undone is `steps.len()`.
    pub fn new(
        id: impl Into<ElementId>,
        steps: impl IntoIterator<Item = impl Into<SharedString>>,
        at: usize,
    ) -> Self {
        let steps: Vec<SharedString> = steps.into_iter().map(Into::into).collect();
        if at > steps.len() {
            log::error!(
                "history panel: at {at} of {} steps; none current",
                steps.len()
            );
        }
        Self {
            id: id.into(),
            steps,
            at,
            on_step: None,
        }
    }

    /// Gets how many steps to keep in force.
    pub fn on_step(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_step = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for HistoryPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let at = self.at;
        let rows = self.steps.into_iter().enumerate().fold(
            SelectableList::new((self.id.clone(), "steps")),
            |list, (ix, step)| {
                let done = ix < at;
                let mark = Icon::new(if done {
                    IconName::Check
                } else {
                    IconName::RotateCcw
                })
                .size(IconSize::Sm)
                .color(if done {
                    theme.colors.fg_muted
                } else {
                    theme.colors.fg_subtle
                });
                let row = ListItem::new((self.id.clone(), format!("step-{ix}")), step)
                    .leading(mark)
                    .quiet(!done)
                    .current(ix + 1 == at);
                list.row(format!("{ix}"), row)
            },
        );
        let on_step = self.on_step;
        div().child(rows.on_activate(move |key, window, cx| {
            let ix: usize = key.parse().expect("a step's key is its place");
            log::info!("history panel: back to step {}", ix + 1);
            if let Some(on_step) = &on_step {
                on_step(ix + 1, window, cx);
            }
        }))
    }
}
