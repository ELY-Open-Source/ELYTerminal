use std::{borrow::Cow, process::ExitStatus, rc::Rc, sync::Arc};

use alacritty_terminal::{
    event::{Event, Notify, OnResize},
    event_loop::Notifier,
    grid::Scroll,
    sync::FairMutex,
    term::{Term, test::TermSize},
    vte::ansi::{ClearMode, Handler, Processor},
};
use futures::StreamExt;
use gpui::{
    App, ClipboardItem, Context, EventEmitter, FocusHandle, Focusable, IntoElement, ParentElement,
    Pixels, Point, Render, SharedString, Size, Styled, Task, Window, div, font, prelude::*, size,
};
use web_time::Instant;

use super::{
    colors::Ink,
    frame::{Frame, frame},
    input,
    links::Target,
    paint,
    pty::{self, Launch, Listener, Session},
    search::Search,
};
use crate::{
    motion,
    theme::{ActiveTheme, TextSize},
};

/// Line height over the text size.
const LEADING: f32 = 1.35;

/// No program types into a replay, so it shows no cursor.
const HIDE_CURSOR: &[u8] = b"\x1b[?25l";

/// What a terminal tells its host.
#[derive(Clone, Debug, PartialEq)]
pub enum TerminalEvent {
    Title(SharedString),
    Exited(ExitStatus),
    Bell,
    /// A path a Cmd-press opened; URLs open in the browser.
    Open(Target),
    /// Cmd-F: the host shows its find widget.
    Find,
}

/// A terminal: a program on a pseudo-terminal, or replayed output, drawn cell by cell. Type to it, drag to select, Cmd-press a link.
pub struct Terminal {
    pub(crate) term: Arc<FairMutex<Term<Listener>>>,
    pub(crate) pty: Option<Notifier>,
    pub(crate) focus: FocusHandle,
    title: SharedString,
    exited: Option<ExitStatus>,
    /// Columns and lines.
    pub(crate) size: (usize, usize),
    pub(crate) cell: Size<Pixels>,
    /// The grid's top-left in the window, from the last paint.
    pub(crate) origin: Point<Pixels>,
    pub(crate) shown: Rc<Frame>,
    pub(crate) search: Search,
    /// The link under the pointer while Cmd is held: its row, columns and target.
    pub(crate) hovered: Option<(usize, std::ops::Range<usize>, Target)>,
    /// Wheel pixels not yet a whole line.
    pub(crate) rest: Pixels,
    /// Whether a press here started the drag that selects.
    pub(crate) selecting: bool,
    bell: Option<Instant>,
    _events: Task<()>,
}

impl EventEmitter<TerminalEvent> for Terminal {}

impl Focusable for Terminal {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Terminal {
    /// Starts `launch` on a pseudo-terminal; the first layout sizes its grid to the box.
    pub fn spawn(launch: Launch, cx: &mut Context<Self>) -> anyhow::Result<Self> {
        let size = (80, 24);
        let session = pty::spawn(&launch, TermSize::new(size.0, size.1), (8, 16))?;
        Ok(Self::new(session, size, "Terminal".into(), cx))
    }

    /// Output recorded elsewhere, escape codes and all, on a grid of `columns` by `lines` no process feeds.
    pub fn replay(bytes: &[u8], columns: usize, lines: usize, cx: &mut Context<Self>) -> Self {
        let session = pty::detached(TermSize::new(columns, lines));
        {
            let mut term = session.term.lock();
            let mut parser: Processor = Processor::new();
            parser.advance(&mut *term, bytes);
            parser.advance(&mut *term, HIDE_CURSOR);
        }
        Self::new(session, (columns, lines), "Output".into(), cx)
    }

    fn new(
        session: Session,
        size: (usize, usize),
        title: SharedString,
        cx: &mut Context<Self>,
    ) -> Self {
        let Session {
            term,
            pty,
            mut events,
        } = session;
        let task = cx.spawn(async move |this, cx| {
            while let Some(first) = events.next().await {
                let mut batch = vec![first];
                while let Ok(more) = events.try_recv() {
                    batch.push(more);
                }
                if this
                    .update(cx, |terminal, cx| terminal.handle(batch, cx))
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            term,
            pty,
            focus: cx.focus_handle().tab_stop(true),
            title,
            exited: None,
            size,
            cell: size_of_cell(),
            origin: Point::default(),
            shown: Rc::default(),
            search: Search::default(),
            hovered: None,
            rest: Pixels::ZERO,
            selecting: false,
            bell: None,
            _events: task,
        }
    }

    pub fn title(&self) -> &SharedString {
        &self.title
    }

    /// How the program ended, once it has.
    pub fn exit_status(&self) -> Option<ExitStatus> {
        self.exited
    }

    /// The screen's text as it shows now, trailing spaces gone.
    pub fn text(&self) -> String {
        self.shown.text()
    }

    /// Sends bytes to the program, as typing would.
    pub fn write(&mut self, bytes: impl Into<Cow<'static, [u8]>>) {
        match &self.pty {
            Some(pty) => pty.notify(bytes),
            None => log::debug!("terminal: input to a replayed grid"),
        }
    }

    /// Clears the screen and the scrollback; a shell redraws its prompt.
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.search = Search::default();
        {
            let mut term = self.term.lock();
            term.clear_screen(ClearMode::Saved);
            if self.pty.is_none() {
                term.clear_screen(ClearMode::All);
            }
        }
        if self.pty.is_some() {
            self.write(&b"\x0c"[..]);
        }
        log::info!("terminal: cleared");
        cx.notify();
    }

    pub(crate) fn scroll(&mut self, scroll: Scroll, cx: &mut Context<Self>) {
        self.term.lock().scroll_display(scroll);
        cx.notify();
    }

    /// Fits the grid to `columns` by `lines`, and tells the program.
    pub(crate) fn resize(&mut self, columns: usize, lines: usize) {
        let size = (columns.max(2), lines.max(1));
        if size == self.size {
            return;
        }
        self.size = size;
        self.term.lock().resize(TermSize::new(size.0, size.1));
        let cell = (
            f32::from(self.cell.width).round() as u16,
            f32::from(self.cell.height).round() as u16,
        );
        if let Some(pty) = &mut self.pty {
            pty.on_resize(pty::window_size(&TermSize::new(size.0, size.1), cell));
        }
        log::debug!("terminal: {columns} by {lines}");
    }

    fn handle(&mut self, events: Vec<Event>, cx: &mut Context<Self>) {
        for event in events {
            match event {
                Event::Wakeup | Event::MouseCursorDirty | Event::CursorBlinkingChange => {}
                Event::Title(title) => {
                    self.title = title.into();
                    cx.emit(TerminalEvent::Title(self.title.clone()));
                }
                Event::ResetTitle => {
                    self.title = "Terminal".into();
                    cx.emit(TerminalEvent::Title(self.title.clone()));
                }
                Event::Bell => {
                    self.bell = Some(Instant::now());
                    cx.emit(TerminalEvent::Bell);
                }
                Event::ClipboardStore(_, text) => {
                    cx.write_to_clipboard(ClipboardItem::new_string(text))
                }
                Event::ClipboardLoad(_, reply) => {
                    let text = cx
                        .read_from_clipboard()
                        .and_then(|item| item.text())
                        .unwrap_or_default();
                    self.write(reply(&text).into_bytes());
                }
                Event::ColorRequest(index, reply) => {
                    let ink = Ink::new(&cx.theme().colors);
                    let rgb = ink.query(index, self.term.lock().colors());
                    self.write(reply(rgb).into_bytes());
                }
                Event::TextAreaSizeRequest(reply) => {
                    let cell = (
                        f32::from(self.cell.width).round() as u16,
                        f32::from(self.cell.height).round() as u16,
                    );
                    let size = TermSize::new(self.size.0, self.size.1);
                    self.write(reply(pty::window_size(&size, cell)).into_bytes());
                }
                Event::PtyWrite(text) => self.write(text.into_bytes()),
                Event::ChildExit(status) => {
                    log::info!("terminal: the program ended, {status}");
                    self.exited = Some(status);
                    cx.emit(TerminalEvent::Exited(status));
                }
                Event::Exit => log::info!("terminal: the grid closed"),
            }
        }
        cx.notify();
    }
}

/// A cell before the first layout measures one.
fn size_of_cell() -> Size<Pixels> {
    size(Pixels::from(8.0), Pixels::from(16.0))
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if let Some(pty) = &self.pty
            && pty
                .0
                .send(alacritty_terminal::event_loop::Msg::Shutdown)
                .is_err()
        {
            log::debug!("terminal: the program had already ended");
        }
    }
}

impl Render for Terminal {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let ink = Ink::new(&theme.colors);
        let text = theme.text_size(TextSize::Sm);
        let mono = font(theme.mono_family.clone());
        let pixels = text.to_pixels(window.rem_size());
        let resolved = window.text_system().resolve_font(&mono);
        let advance = match window.text_system().advance(resolved, pixels, 'm') {
            Ok(advance) => advance.width,
            Err(error) => {
                log::error!("terminal: no advance for the code font: {error:#}; cells guessed");
                pixels * 0.6
            }
        };
        self.cell = size(advance, (pixels * LEADING).round());
        let (found, current) = self.search.visible();
        self.shown = Rc::new(frame(&self.term.lock(), &ink, found, current));
        let flash = self.bell.and_then(|rang| {
            let lasting = motion::duration(motion::SLOW, cx);
            let spent = rang.elapsed();
            (spent < lasting).then(|| 1.0 - spent.as_secs_f32() / lasting.as_secs_f32())
        });
        if flash.is_some() {
            window.request_animation_frame();
        }
        let colors = theme.colors.clone();
        let look = paint::Look {
            font: mono,
            size: pixels,
            cell: self.cell,
            focused: self.focus.is_focused(window),
            caret: colors.focus,
            caret_width: theme.caret_width().to_pixels(window.rem_size()),
            rule: theme.underline_thickness(),
            selection: colors.selection,
            found: colors.warning.opacity(0.25),
            current: colors.warning.opacity(0.55),
            link: colors.link,
        };
        let root = div()
            .id(("terminal", cx.entity_id()))
            .key_context(input::CONTEXT)
            .track_focus(&self.focus)
            .size_full()
            .p_2()
            .bg(ink.bg)
            .when_some(flash, |root, left| {
                root.bg(ink.bg.blend(colors.warning.opacity(0.12 * left)))
            })
            .when(self.hovered.is_some(), |root| root.cursor_pointer())
            .child(paint::grid(
                cx.entity(),
                self.shown.clone(),
                self.hovered.clone(),
                look,
            ));
        input::listen(root, cx)
    }
}
