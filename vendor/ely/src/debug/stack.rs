use std::{ops::Range, rc::Rc};

use gpui::{
    AnyElement, App, ElementId, Hsla, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use super::OnIndex;
use crate::{
    forms::OnFlag,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize, Palette, Radius, TextSize},
    typography::{Ellipsis, format},
};

type OnThread = Rc<dyn Fn(u64, &mut Window, &mut App)>;

/// A frame of the call stack: its function, where it is, and whether it is library code.
#[derive(Clone, Debug, PartialEq)]
pub struct StackFrame {
    pub function: SharedString,
    pub path: SharedString,
    /// From one.
    pub line: usize,
    pub library: bool,
}

/// Frames as shown: each one of the program's, and each run of library frames as one fold, by the frames it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Shown {
    Frame(usize),
    Folded(Range<usize>),
}

pub(crate) fn shown(frames: &[StackFrame], libraries: bool) -> Vec<Shown> {
    let mut out = Vec::new();
    for (ix, frame) in frames.iter().enumerate() {
        if !frame.library || libraries {
            out.push(Shown::Frame(ix));
            continue;
        }
        match out.last_mut() {
            Some(Shown::Folded(range)) if range.end == ix => range.end = ix + 1,
            _ => out.push(Shown::Folded(ix..ix + 1)),
        }
    }
    out
}

/// Where the program is paused, innermost call first: the current frame marked, runs of library frames folded until pressed; a press moves to a frame.
#[derive(IntoElement)]
pub struct CallStack {
    id: ElementId,
    frames: Vec<StackFrame>,
    current: usize,
    libraries: bool,
    on_pick: Option<OnIndex>,
    on_libraries: Option<OnFlag>,
}

impl CallStack {
    pub fn new(id: impl Into<ElementId>, frames: impl IntoIterator<Item = StackFrame>) -> Self {
        Self {
            id: id.into(),
            frames: frames.into_iter().collect(),
            current: 0,
            libraries: false,
            on_pick: None,
            on_libraries: None,
        }
    }

    pub fn current(mut self, frame: usize) -> Self {
        if frame >= self.frames.len() {
            log::error!(
                "call stack: no frame {frame} of {}; none marked",
                self.frames.len()
            );
        }
        self.current = frame;
        self
    }

    /// Whether library frames show one by one.
    pub fn libraries(mut self, shown: bool) -> Self {
        self.libraries = shown;
        self
    }

    pub fn on_pick(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }

    pub fn on_libraries(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_libraries = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for CallStack {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let rows: Vec<AnyElement> = shown(&self.frames, self.libraries)
            .into_iter()
            .map(|entry| match entry {
                Shown::Frame(ix) => {
                    let frame = &self.frames[ix];
                    let (current, pick) = (ix == self.current, self.on_pick.clone());
                    let file = frame
                        .path
                        .rsplit('/')
                        .next()
                        .unwrap_or(&frame.path)
                        .to_string();
                    div()
                        .id((self.id.clone(), format!("frame-{ix}")))
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_2()
                        .py_1()
                        .rounded(theme.radius(Radius::Sm))
                        .cursor_pointer()
                        .when(current, |row| row.bg(colors.warning.opacity(0.12)))
                        .when(!current, |row| row.hover(|row| row.bg(colors.hover)))
                        .when_some(pick, |row, pick| {
                            row.on_click(move |_, window, cx| pick(ix, window, cx))
                        })
                        .child(div().flex_none().w_4().children(current.then(|| {
                            Icon::new(IconName::ArrowRight)
                                .size(IconSize::Sm)
                                .color(colors.warning)
                        })))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .font_family(theme.mono_family.clone())
                                .text_color(if frame.library {
                                    colors.fg_muted
                                } else {
                                    colors.fg
                                })
                                .child(Ellipsis::new(frame.function.clone())),
                        )
                        .child(
                            div()
                                .flex_none()
                                .text_color(colors.fg_subtle)
                                .child(format!("{file}:{}", frame.line)),
                        )
                        .into_any_element()
                }
                Shown::Folded(range) => {
                    let show = self.on_libraries.clone();
                    div()
                        .id((self.id.clone(), format!("fold-{}", range.start)))
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_2()
                        .py_1()
                        .pl_8()
                        .rounded(theme.radius(Radius::Sm))
                        .cursor_pointer()
                        .text_color(colors.fg_subtle)
                        .hover(|row| row.text_color(colors.fg))
                        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                        .when_some(show, |row, show| {
                            row.on_click(move |_, window, cx| show(true, window, cx))
                        })
                        .child(format!(
                            "⋯ {}",
                            format::plural(range.len() as u64, "library frame", "library frames")
                        ))
                        .into_any_element()
                }
            })
            .collect();
        div()
            .flex()
            .flex_col()
            .text_size(theme.text_size(TextSize::Sm))
            .children(rows)
    }
}

/// How a thread stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadState {
    Running,
    Paused,
    /// Stopped on a breakpoint.
    Stopped,
}

impl ThreadState {
    fn look(self, colors: &Palette) -> (IconName, Hsla, &'static str) {
        match self {
            Self::Running => (IconName::Play, colors.success, "running"),
            Self::Paused => (IconName::Pause, colors.fg_muted, "paused"),
            Self::Stopped => (IconName::CircleDot, colors.danger, "on a breakpoint"),
        }
    }
}

/// A thread: its id, its name, and how it stands.
#[derive(Clone, Debug, PartialEq)]
pub struct Thread {
    pub id: u64,
    pub name: SharedString,
    pub state: ThreadState,
}

/// The program's threads, each with how it stands; a press picks the one whose stack shows.
#[derive(IntoElement)]
pub struct ThreadList {
    id: ElementId,
    threads: Vec<Thread>,
    selected: Option<u64>,
    on_pick: Option<OnThread>,
}

impl ThreadList {
    pub fn new(id: impl Into<ElementId>, threads: impl IntoIterator<Item = Thread>) -> Self {
        Self {
            id: id.into(),
            threads: threads.into_iter().collect(),
            selected: None,
            on_pick: None,
        }
    }

    pub fn selected(mut self, thread: u64) -> Self {
        self.selected = Some(thread);
        self
    }

    pub fn on_pick(mut self, handler: impl Fn(u64, &mut Window, &mut App) + 'static) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ThreadList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        div()
            .flex()
            .flex_col()
            .text_size(theme.text_size(TextSize::Sm))
            .children(self.threads.iter().map(|thread| {
                let (icon, tint, words) = thread.state.look(&colors);
                let selected = self.selected == Some(thread.id);
                let (pick, id) = (self.on_pick.clone(), thread.id);
                div()
                    .id((self.id.clone(), format!("thread-{}", thread.id)))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .rounded(theme.radius(Radius::Sm))
                    .cursor_pointer()
                    .when(selected, |row| row.bg(colors.active))
                    .when(!selected, |row| row.hover(|row| row.bg(colors.hover)))
                    .when_some(pick, |row, pick| {
                        row.on_click(move |_, window, cx| pick(id, window, cx))
                    })
                    .child(Icon::new(icon).size(IconSize::Sm).color(tint))
                    .child(
                        div()
                            .flex_1()
                            .text_color(colors.fg)
                            .child(thread.name.clone()),
                    )
                    .child(
                        div()
                            .text_color(colors.fg_subtle)
                            .child(format!("#{} · {words}", thread.id)),
                    )
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(library: bool) -> StackFrame {
        StackFrame {
            function: "f".into(),
            path: "a.rs".into(),
            line: 1,
            library,
        }
    }

    #[test]
    fn runs_of_library_frames_fold_into_one() {
        let frames = [
            frame(false),
            frame(true),
            frame(true),
            frame(false),
            frame(true),
        ];
        assert_eq!(
            shown(&frames, false),
            [
                Shown::Frame(0),
                Shown::Folded(1..3),
                Shown::Frame(3),
                Shown::Folded(4..5)
            ]
        );
        assert_eq!(shown(&frames, true).len(), 5, "shown one by one when asked");
    }
}
