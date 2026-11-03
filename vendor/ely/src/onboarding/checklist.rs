use std::rc::Rc;

use gpui::{
    Animation, AnimationExt, App, ElementId, FontWeight, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, Styled, Window, div, prelude::*,
};

use crate::{
    buttons::{Button, ButtonVariant},
    forms::Run,
    motion::{self, ProgressBar},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, TextSize},
    typography::tabular,
};

type OnKey = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// A first thing to do: its key, its title, a line on it, and whether it is done.
#[derive(Clone, Debug, PartialEq)]
pub struct SetupTask {
    pub key: SharedString,
    pub title: SharedString,
    pub body: SharedString,
    pub done: bool,
}

/// First things to do in a new app: how many are done, in words and a bar in as many parts; each task with its line and Start while not done, done ones struck through. The check eases in as a task gets done. Once all are, it says so and, with `on_dismiss`, offers Hide.
#[derive(IntoElement)]
pub struct SetupChecklist {
    id: ElementId,
    title: SharedString,
    tasks: Vec<SetupTask>,
    on_start: OnKey,
    on_dismiss: Option<Run>,
}

impl SetupChecklist {
    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        tasks: impl IntoIterator<Item = SetupTask>,
        on_start: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        let tasks: Vec<SetupTask> = tasks.into_iter().collect();
        assert!(
            tasks.len() >= 2,
            "a setup checklist needs two tasks, got {}",
            tasks.len()
        );
        Self {
            id: id.into(),
            title: title.into(),
            tasks,
            on_start: Rc::new(on_start),
            on_dismiss: None,
        }
    }

    /// Shows Hide once every task is done.
    pub fn on_dismiss(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SetupChecklist {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let on_start = self.on_start;
        let (done, all) = (
            self.tasks.iter().filter(|task| task.done).count(),
            self.tasks.len(),
        );
        let tally = format!("{done} of {all} done");
        let turns: Vec<usize> = self
            .tasks
            .iter()
            .map(|task| {
                motion::changes(
                    (id.clone(), format!("done-{}", task.key)),
                    task.done,
                    window,
                    cx,
                )
            })
            .collect();
        let theme = cx.theme();
        let duration = motion::duration(motion::BASE, cx);
        let rows = self
            .tasks
            .into_iter()
            .zip(turns)
            .enumerate()
            .map(|(ix, (task, turn))| {
                let (icon, ink) = match task.done {
                    true => (IconName::CircleCheck, theme.colors.success),
                    false => (IconName::Circle, theme.colors.fg_subtle),
                };
                let mark = div()
                    .flex_none()
                    .pt_0p5()
                    .child(Icon::new(icon).size(IconSize::Sm).color(ink))
                    .with_animation(
                        (id.clone(), format!("mark-{}-{turn}", task.key)),
                        Animation::new(duration).with_easing(motion::ease_out_cubic),
                        move |mark, t| match turn {
                            0 => mark,
                            _ => mark.opacity(t),
                        },
                    );
                let start = (!task.done).then(|| {
                    let (key, start) = (task.key.clone(), on_start.clone());
                    Button::new((id.clone(), format!("start-{}", task.key)), "Start")
                        .variant(ButtonVariant::Ghost)
                        .size(ControlSize::Sm)
                        .on_click(move |_, window, cx| {
                            log::info!("setup checklist: start {key}");
                            start(&key, window, cx)
                        })
                });
                let key = task.key.clone();
                div()
                    .debug_selector(move || format!("setup-task-{key}"))
                    .flex()
                    .items_start()
                    .gap_3()
                    .py_2p5()
                    .when(ix > 0, |row| {
                        row.border_t_1().border_color(theme.colors.border)
                    })
                    .child(mark)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .when(task.done, |title| {
                                        title.line_through().text_color(theme.colors.fg_muted)
                                    })
                                    .child(task.title),
                            )
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Sm))
                                    .text_color(theme.colors.fg_muted)
                                    .child(task.body),
                            ),
                    )
                    .children(start.map(|start| div().flex_none().child(start)))
            });
        let finished = (done == all).then(|| {
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap_2()
                .pt_2()
                .child(
                    div()
                        .text_size(theme.text_size(TextSize::Sm))
                        .text_color(theme.colors.success)
                        .child("All set."),
                )
                .children(self.on_dismiss.map(|run| {
                    Button::new((id.clone(), "hide"), "Hide")
                        .size(ControlSize::Sm)
                        .on_click(move |_, window, cx| {
                            log::info!("setup checklist: hidden");
                            run(window, cx)
                        })
                }))
        });
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w(theme.label_width())
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(self.title),
                    )
                    .child(
                        tabular(div())
                            .debug_selector({
                                let tally = tally.clone();
                                move || format!("setup-{tally}")
                            })
                            .flex_none()
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(theme.colors.fg_muted)
                            .child(tally),
                    ),
            )
            .child(
                ProgressBar::new((id.clone(), "progress"), done as f32 / all as f32).segments(all),
            )
            .child(div().flex().flex_col().children(rows))
            .children(finished)
    }
}
