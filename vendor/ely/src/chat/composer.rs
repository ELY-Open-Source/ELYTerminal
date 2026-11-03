use std::{path::PathBuf, rc::Rc};

use gpui::{
    AnyElement, App, ElementId, Entity, ExternalPaths, InteractiveElement, IntoElement,
    ParentElement, PathPromptOptions, RenderOnce, SharedString, Styled, Window, div, prelude::*,
};
use smallvec::SmallVec;

use crate::{
    buttons::{ButtonVariant, IconButton},
    forms::{
        Choice, Enter, OnValue, Pick, Run, Suggestions, TextInput, active_trigger, choose, dropped,
        replace_trigger,
    },
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
    typography::Kbd,
};

type OnPaths = Rc<dyn Fn(Vec<PathBuf>, &mut Window, &mut App)>;

/// Opens the system's file dialog to attach files; `on_pick` gets what was chosen.
#[derive(IntoElement)]
pub struct AttachmentButton {
    id: ElementId,
    on_pick: OnPaths,
}

impl AttachmentButton {
    pub fn new(
        id: impl Into<ElementId>,
        on_pick: impl Fn(Vec<PathBuf>, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            on_pick: Rc::new(on_pick),
        }
    }
}

impl RenderOnce for AttachmentButton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let pick = self.on_pick;
        IconButton::new(self.id, IconName::Paperclip)
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Sm)
            .tooltip("Attach files")
            .on_click(move |_, window, cx| {
                let pick = pick.clone();
                let options = PathPromptOptions {
                    files: true,
                    directories: false,
                    multiple: true,
                    prompt: Some("Attach".into()),
                };
                choose(
                    "attachment button".into(),
                    options,
                    window,
                    cx,
                    move |paths, window, cx| pick(paths, window, cx),
                );
            })
    }
}

/// Sends a message once there is one, or stops the answer while it streams.
#[derive(IntoElement)]
pub struct SendButton {
    id: ElementId,
    ready: bool,
    on_send: Run,
    on_stop: Option<Run>,
}

impl SendButton {
    pub fn new(
        id: impl Into<ElementId>,
        ready: bool,
        on_send: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            ready,
            on_send: Rc::new(on_send),
            on_stop: None,
        }
    }

    /// While an answer streams: the button stops it.
    pub fn busy(mut self, on_stop: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_stop = Some(Rc::new(on_stop));
        self
    }
}

impl RenderOnce for SendButton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        match self.on_stop {
            Some(stop) => IconButton::new(self.id, IconName::Square)
                .variant(ButtonVariant::Secondary)
                .size(ControlSize::Sm)
                .tooltip("Stop")
                .on_click(move |_, window, cx| {
                    log::info!("send button: stop");
                    stop(window, cx)
                }),
            None => {
                let send = self.on_send;
                IconButton::new(self.id, IconName::ArrowUp)
                    .variant(ButtonVariant::Primary)
                    .size(ControlSize::Sm)
                    .tooltip("Send")
                    .disabled(!self.ready)
                    .on_click(move |_, window, cx| {
                        log::info!("send button: send");
                        send(window, cx)
                    })
            }
        }
    }
}

/// The keys a composer takes, said quietly: Enter sends, Shift-Enter breaks the line.
#[derive(IntoElement)]
pub struct InputHint;

impl RenderOnce for InputHint {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .text_size(theme.text_size(TextSize::Xs))
            .text_color(theme.colors.fg_subtle)
            .child(Kbd::new("enter"))
            .child("to send ·")
            .child(Kbd::new("shift-enter"))
            .child("for a new line")
    }
}

/// While files hover its parent, a veil over it that says a drop attaches them; it takes the drop. The parent is `relative`.
#[derive(IntoElement)]
pub struct DragDropOverlay {
    on_drop: OnPaths,
}

impl DragDropOverlay {
    /// `on_drop` gets the files dropped; a drop with a folder is refused.
    pub fn new(on_drop: impl Fn(Vec<PathBuf>, &mut Window, &mut App) + 'static) -> Self {
        Self {
            on_drop: Rc::new(on_drop),
        }
    }
}

impl RenderOnce for DragDropOverlay {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let take = self.on_drop;
        div()
            .absolute()
            .inset_0()
            .invisible()
            .drag_over::<ExternalPaths>(|style, _, _, _| style.visible())
            .on_drop(
                move |paths: &ExternalPaths, window, cx| match dropped(paths.paths(), true) {
                    Ok(paths) => {
                        log::info!("drag drop overlay: {} files dropped", paths.len());
                        take(paths, window, cx)
                    }
                    Err(reason) => log::info!("drag drop overlay: refused a drop: {reason}"),
                },
            )
            .flex()
            .items_center()
            .justify_center()
            .rounded(theme.radius(Radius::Xl))
            .border_1()
            .border_dashed()
            .border_color(colors.focus)
            .bg(colors.surface.opacity(0.94))
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(colors.fg_muted)
            .child(
                div()
                    .debug_selector(|| "drag-drop-overlay".into())
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Icon::new(IconName::Upload)
                            .size(IconSize::Sm)
                            .color(colors.fg_muted),
                    )
                    .child("Drop to attach"),
            )
    }
}

/// The rows a composer offers at `/` or `@`, and what a pick does.
fn offers(
    id: ElementId,
    field: &Entity<TextInput>,
    commands: &[Choice],
    context: &[Choice],
    on_command: Option<OnValue>,
    on_mention: Option<OnValue>,
    cx: &App,
) -> Suggestions {
    let input = field.read(cx);
    let (text, caret) = (input.text().to_string(), input.cursor());
    let active = active_trigger(&text, caret, &['/', '@'], |ch| !ch.is_whitespace())
        .filter(|(at, mark)| *mark == '@' || *at == 0);
    let query = active.map_or(String::new(), |(at, mark)| {
        text[at + mark.len_utf8()..caret].to_lowercase()
    });
    let (listed, run) = match active {
        Some((_, '/')) => (commands, on_command),
        Some(_) => (context, on_mention),
        None => (&[][..], None),
    };
    let found: Vec<Choice> = listed
        .iter()
        .filter(|choice| choice.label.to_lowercase().contains(&query))
        .cloned()
        .collect();
    let rows = if run.is_some() {
        found.clone()
    } else {
        Vec::new()
    };
    let (state, field) = (field.clone(), field.clone());
    let pick: Pick = Rc::new(move |ix: usize, window: &mut Window, cx: &mut App| {
        let (at, caret) = (
            active.expect("rows show at a trigger").0,
            field.read(cx).cursor(),
        );
        replace_trigger(&field, at, caret, "", cx);
        log::info!("composer: picked {}", found[ix].value);
        if let Some(run) = &run {
            run(&found[ix].value, window, cx);
        }
    });
    Suggestions {
        id,
        state,
        trigger: active.map(|(at, _)| at),
        rows,
        pick,
    }
}

/// Where a message is written: its attachments and context above, the field, and a row of the owner's tools with send. Enter sends, Shift-Enter breaks the line; `/` at the start offers commands and `@` offers context; files dropped on it attach.
#[derive(IntoElement)]
pub struct PromptInput {
    id: ElementId,
    field: Entity<TextInput>,
    on_send: Run,
    on_stop: Option<Run>,
    above: SmallVec<[AnyElement; 2]>,
    tools: SmallVec<[AnyElement; 4]>,
    commands: (Vec<Choice>, Option<OnValue>),
    context: (Vec<Choice>, Option<OnValue>),
    on_drop: Option<OnPaths>,
}

impl PromptInput {
    /// `field` is the owner's multi-line text; `on_send` sends it.
    pub fn new(
        id: impl Into<ElementId>,
        field: &Entity<TextInput>,
        on_send: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            field: field.clone(),
            on_send: Rc::new(on_send),
            on_stop: None,
            above: SmallVec::new(),
            tools: SmallVec::new(),
            commands: (Vec::new(), None),
            context: (Vec::new(), None),
            on_drop: None,
        }
    }

    /// While an answer streams: send becomes stop.
    pub fn busy(mut self, on_stop: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_stop = Some(Rc::new(on_stop));
        self
    }

    /// Shown over the field, such as attachments, context and a quote.
    pub fn above(mut self, element: impl IntoElement) -> Self {
        self.above.push(element.into_any_element());
        self
    }

    /// A tool in the row under the field, such as attach or the model.
    pub fn tool(mut self, element: impl IntoElement) -> Self {
        self.tools.push(element.into_any_element());
        self
    }

    /// Commands offered at `/` at the start; `on_command` gets the one picked.
    pub fn commands(
        mut self,
        commands: impl IntoIterator<Item = Choice>,
        on_command: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.commands = (commands.into_iter().collect(), Some(Rc::new(on_command)));
        self
    }

    /// Context offered at `@`; `on_mention` gets the one picked.
    pub fn context(
        mut self,
        context: impl IntoIterator<Item = Choice>,
        on_mention: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.context = (context.into_iter().collect(), Some(Rc::new(on_mention)));
        self
    }

    /// Takes files dropped on the composer.
    pub fn on_drop(
        mut self,
        handler: impl Fn(Vec<PathBuf>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_drop = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for PromptInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let ready = !self.field.read(cx).text().trim().is_empty();
        let focused = self.field.read(cx).focus().contains_focused(window, cx);
        let (send, enter) = (self.on_send.clone(), self.on_send.clone());
        let suggestions = offers(
            (self.id.clone(), "offers").into(),
            &self.field,
            &self.commands.0,
            &self.context.0,
            self.commands.1.clone(),
            self.context.1.clone(),
            cx,
        );
        let button = SendButton::new((self.id.clone(), "send"), ready, move |window, cx| {
            send(window, cx)
        });
        let button = match self.on_stop.clone() {
            Some(stop) => button.busy(move |window, cx| stop(window, cx)),
            None => button,
        };
        let busy = self.on_stop.is_some();
        let body = div()
            .id(self.id.clone())
            .relative()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .rounded(theme.radius(Radius::Xl))
            .border_1()
            .border_color(if focused {
                colors.focus
            } else {
                colors.border_strong
            })
            .bg(colors.surface)
            .children(self.above)
            .child(
                suggestions.wrap(
                    div()
                        .capture_action(move |_: &Enter, window, cx| {
                            cx.stop_propagation();
                            if ready && !busy {
                                log::info!("composer: sent with Enter");
                                enter(window, cx);
                            }
                        })
                        .text_size(theme.text_size(TextSize::Base))
                        .child(self.field.clone()),
                    window,
                    cx,
                ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .children(self.tools)
                    .child(div().flex_1())
                    .child(button),
            );
        body.when_some(self.on_drop, |body, take| {
            body.child(DragDropOverlay { on_drop: take })
        })
    }
}
