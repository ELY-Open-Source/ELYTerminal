use std::{
    collections::HashMap,
    path::PathBuf,
    process::ExitStatus,
    rc::Rc,
    time::{Duration, Instant},
};

use alacritty_terminal::{
    event::Event,
    grid::Dimensions,
    term::{ClipboardType, TermMode},
    vte::ansi::CursorShape,
};
use ely_gpui_component::{
    forms::{Copy, Paste, SelectAll},
    theme::ActiveTheme,
};
use futures::StreamExt;
use gpui::{
    Bounds, Context, EventEmitter, ExternalPaths, FocusHandle, Focusable, IntoElement, MouseButton,
    ParentElement, Pixels, Render, Styled, Subscription, Task, Window, div, prelude::*, px,
};

use super::{
    click::Shell,
    find::Finder,
    fonts::look,
    frame::frame,
    ink::Ink,
    input::{self, Find, Held, NextPrompt, PreviousPrompt, Went},
    marks::Mark,
    mouse::MouseMode,
    paint,
    pointer::Pointer,
    pty::{Events, Pty, PtyEvent, config, fit},
    reports::Notices,
    scrollbar::{self, Span},
    status::{Records, State},
    zoom,
};
use crate::{settings::SettingsStore, themes};

/// How long the cursor shows, then hides, while it blinks.
pub const BLINK: Duration = Duration::from_millis(600);

/// What a terminal tells its owner.
#[derive(Clone, Debug, PartialEq)]
pub enum TerminalEvent {
    Title(String),
    Folder(PathBuf),
    Bell,
    /// The state its row shows changed, to this one.
    Shows(Option<State>),
    Exited(ExitStatus),
}

/// A program on a pseudo-terminal, drawn cell by cell in the theme and font the settings choose.
pub struct TerminalView {
    pub(super) pty: Pty,
    pub(super) focus: FocusHandle,
    /// Text an input method is composing, drawn at the cursor.
    pub(super) marked: Option<String>,
    /// The cursor's cell in the window, from the last layout.
    pub(super) caret: Bounds<Pixels>,
    pub(super) pointer: Pointer,
    /// The mouse mode, as the tap follows it.
    pub(super) mouse: MouseMode,
    /// Keys go as Windows' records, as ConPTY asks.
    #[cfg(windows)]
    pub(super) win32: bool,
    /// herdr's client leads the session, so herdr opens its panes' links.
    pub(super) herdr: bool,
    /// Keys whose press this terminal saw, so their releases may go out, each with how its press went.
    pub(super) held: HashMap<Held, Went>,
    pub(super) finder: Option<Finder>,
    pub(super) size: (usize, usize),
    /// How many folders the program has reported.
    pub(super) reports: u64,
    /// What programs say they are doing, OSC 7501.
    pub(super) records: Records,
    /// The status its shell's last command ended with; none before one, or when it gave none.
    pub(super) ended: Option<i32>,
    /// A command started, as OSC 133 C says, since the last one ended, at the moment C came.
    pub(super) ran: Option<Instant>,
    /// Where the shell stands, as its OSC 133 marks say.
    pub(super) shell: Shell,
    /// The system notifications it posted.
    pub(super) notices: Notices,
    /// The program's title; empty while it names none.
    pub(super) title: String,
    /// It is the session shown.
    pub(super) front: bool,
    /// Its window is active.
    pub(super) active: bool,
    cell: (u16, u16),
    /// Focused in an active window, as the program last heard.
    focused: bool,
    blinking: bool,
    shown: bool,
    /// The kitty keyboard flags, as the parser last left them.
    kitty: TermMode,
    _events: Task<()>,
    _blink: Task<()>,
    _herdr: Task<()>,
    _focus: [Subscription; 3],
    _settings: Subscription,
    _zoom: Subscription,
}

impl EventEmitter<TerminalEvent> for TerminalView {}

impl Focusable for TerminalView {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl TerminalView {
    /// Draws `pty`; the first layout fits its grid to the box.
    pub fn new(pty: Pty, events: Events, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let size = {
            let term = pty.term.lock();
            (term.columns(), term.screen_lines())
        };
        let focus = cx.focus_handle().tab_stop(true);
        let refocus =
            |view: &mut Self, window: &mut Window, cx: &mut Context<Self>| view.refocus(window, cx);
        Self {
            pty,
            _focus: [
                cx.observe_window_activation(window, refocus),
                cx.on_focus(&focus, window, refocus),
                cx.on_blur(&focus, window, refocus),
            ],
            focus,
            marked: None,
            caret: Bounds::default(),
            pointer: Pointer::default(),
            mouse: MouseMode::default(),
            #[cfg(windows)]
            win32: false,
            herdr: false,
            held: HashMap::new(),
            finder: None,
            size,
            reports: 0,
            records: Records::default(),
            ended: None,
            ran: None,
            shell: Shell::Busy,
            notices: Notices::default(),
            title: String::new(),
            front: true,
            active: false,
            cell: (8, 16),
            focused: false,
            blinking: false,
            shown: true,
            kitty: TermMode::empty(),
            _events: Self::pump(events, cx),
            _blink: Self::blink(cx),
            _herdr: super::herdr::follow(cx),
            _settings: cx.observe_global::<SettingsStore>(|view, cx| {
                view.pty
                    .term
                    .lock()
                    .set_options(config(SettingsStore::get(cx)));
                view.search(false, cx);
                cx.notify();
            }),
            _zoom: cx.observe_global::<zoom::Enlarged>(|_, cx| cx.notify()),
        }
    }

    pub fn pid(&self) -> u32 {
        self.pty.pid
    }

    /// What programs in this terminal say they are doing.
    pub fn records(&self) -> &Records {
        &self.records
    }

    /// The status its shell's last command ended with.
    pub fn ended(&self) -> Option<i32> {
        self.ended
    }

    /// Whether it is the session shown: one shown in an active window posts no notification.
    pub fn set_front(&mut self, front: bool) {
        self.front = front;
    }

    fn pump(mut events: Events, cx: &mut Context<Self>) -> Task<()> {
        cx.spawn(async move |view, cx| {
            while let Some(first) = events.next().await {
                let mut batch = vec![first];
                while let Ok(more) = events.try_recv() {
                    batch.push(more);
                }
                if view.update(cx, |view, cx| view.handle(batch, cx)).is_err() {
                    break;
                }
            }
        })
    }

    fn blink(cx: &mut Context<Self>) -> Task<()> {
        cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor().timer(BLINK).await;
                let ticked = view.update(cx, |view, cx| {
                    if view.blinking {
                        view.shown = !view.shown;
                        cx.notify();
                    }
                });
                if ticked.is_err() {
                    break;
                }
            }
        })
    }

    /// Fits the grid to `columns` by `lines`; true when the size changed.
    pub(super) fn fit(&mut self, columns: usize, lines: usize, cx: &mut Context<Self>) -> bool {
        let fitted = fit(columns, lines, self.cell);
        let size = (usize::from(fitted.num_cols), usize::from(fitted.num_lines));
        if size == self.size {
            return false;
        }
        self.size = size;
        self.pty.resize(size, self.cell);
        log::debug!("terminal {}: {} by {}", self.pty.pid, size.0, size.1);
        self.search(false, cx);
        true
    }

    /// Tells a program that asked, by mode 1004, when the terminal gains or loses focus.
    fn refocus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.active = window.is_window_active();
        let focused = self.focus.is_focused(window) && window.is_window_active();
        if focused != self.focused {
            self.focused = focused;
            let asked = self.pty.term.lock().mode().contains(TermMode::FOCUS_IN_OUT);
            log::debug!(
                "terminal {}: focus {}{}",
                self.pty.pid,
                if focused { "in" } else { "out" },
                if asked { ", reported" } else { "" }
            );
            if asked {
                self.pty.write(if focused {
                    &b"\x1b[I"[..]
                } else {
                    &b"\x1b[O"[..]
                });
            }
        }
        cx.notify();
    }

    fn handle(&mut self, events: Vec<PtyEvent>, cx: &mut Context<Self>) {
        let drawn = events
            .iter()
            .any(|event| matches!(event, PtyEvent::Term(Event::Wakeup)));
        for event in events {
            if let PtyEvent::Mark(mark, holder) = &event {
                self.records.mark(mark, *holder);
            }
            match event {
                PtyEvent::Folder(folder) => {
                    self.reports += 1;
                    cx.emit(TerminalEvent::Folder(folder));
                }
                PtyEvent::Term(event) => self.term_event(event, cx),
                PtyEvent::Mouse(mode) => self.mouse = mode,
                #[cfg(windows)]
                PtyEvent::Win32(on) => self.win32 = on,
                PtyEvent::Mark(Mark::Done(status), _) => self.finished(status, cx),
                PtyEvent::Mark(Mark::Prompt(click), _) => {
                    self.shell = Shell::Prompting(click);
                    self.restate(cx);
                }
                PtyEvent::Mark(Mark::Output, _) => {
                    self.ran = Some(Instant::now());
                    self.shell = Shell::Busy;
                }
                PtyEvent::Mark(Mark::Input, _) => self.shell = self.shell.reads(),
                PtyEvent::Status(status, owner) => {
                    self.status(status, owner);
                    self.restate(cx);
                }
                PtyEvent::Ask(ask, owner) => {
                    self.ask(ask, owner, cx);
                    self.restate(cx);
                }
                PtyEvent::Title(title) => {
                    self.title.clone_from(&title);
                    cx.emit(TerminalEvent::Title(title));
                }
                PtyEvent::Reset => {
                    self.records.reset();
                    self.title.clear();
                    cx.emit(TerminalEvent::Title(String::new()));
                    self.restate(cx);
                }
            }
        }
        if drawn {
            let kitty = *self.pty.term.lock().mode() & TermMode::KITTY_KEYBOARD_PROTOCOL;
            if kitty != self.kitty {
                log::debug!("terminal {}: kitty keyboard flags {kitty:?}", self.pty.pid);
                self.kitty = kitty;
            }
        }
        // ponytail: a full search per output batch while find is open; search only new lines if it lags.
        if drawn && self.finder.is_some() {
            self.search(false, cx);
        }
        cx.notify();
    }

    fn term_event(&mut self, event: Event, cx: &mut Context<Self>) {
        match event {
            Event::Wakeup => self.shown = true,
            // The tap reads the title in the output's order; alacritty's title events come after it.
            Event::MouseCursorDirty
            | Event::CursorBlinkingChange
            | Event::Title(_)
            | Event::ResetTitle => {}
            Event::Bell => {
                log::debug!("terminal {}: a bell, kept silent", self.pty.pid);
                cx.emit(TerminalEvent::Bell);
            }
            Event::ClipboardStore(kind, text) => {
                log::info!("terminal {}: OSC 52 writes the {kind:?}", self.pty.pid);
                self.copy_text(text, kind == ClipboardType::Selection, cx);
            }
            Event::ClipboardLoad(..) => log::error!(
                "terminal {}: a clipboard read arrived though OSC 52 reads are off",
                self.pty.pid
            ),
            Event::ColorRequest(index, reply) => {
                let theme = themes::active(SettingsStore::get(cx), cx.window_appearance(), cx);
                let term = self.pty.term.lock();
                let rgb = Ink::new(&theme, term.colors()).query(index, term.colors());
                drop(term);
                self.pty.write(reply(rgb).into_bytes());
            }
            Event::TextAreaSizeRequest(reply) => {
                let size = fit(self.size.0, self.size.1, self.cell);
                self.pty.write(reply(size).into_bytes());
            }
            Event::PtyWrite(text) => self.pty.write(text.into_bytes()),
            Event::ChildExit(status) => cx.emit(TerminalEvent::Exited(status)),
            Event::Exit => log::debug!("terminal {}: the grid closed", self.pty.pid),
        }
    }
}

impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = SettingsStore::get(cx).clone();
        let theme = themes::active(&settings, window.appearance(), cx);
        let term = self.pty.term.lock();
        let ink = Ink::new(&theme, term.colors());
        let lit = self.finder.as_ref().map_or_else(Vec::new, |finder| {
            finder.lit(&term, cx.theme().colors.warning)
        });
        let shown = frame(&term, &ink, &lit);
        let blinks = term.cursor_style().blinking;
        let span = Span::of(&term);
        drop(term);
        let focused = self.focus.is_focused(window) && window.is_window_active();
        self.blinking = focused && blinks;
        if !self.blinking {
            self.shown = true;
        }
        let cursor = shown
            .cursor
            .clone()
            .and_then(|cursor| match (focused, self.shown) {
                (false, _) => Some((cursor, CursorShape::HollowBlock)),
                (true, false) => None,
                (true, true) => {
                    let shape = cursor.shape;
                    Some((cursor, shape))
                }
            });
        let root = div()
            .id(("terminal", cx.entity_id()))
            .key_context(input::CONTEXT)
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down))
            .on_key_up(cx.listener(Self::key_up))
            .on_action(cx.listener(|view, _: &Copy, _, cx| view.copy(cx)))
            .on_action(cx.listener(|view, _: &Paste, _, cx| view.paste(cx)))
            .on_action(cx.listener(|view, _: &SelectAll, _, cx| view.select_all(cx)))
            .on_action(cx.listener(|view, _: &Find, window, cx| view.find(window, cx)))
            .on_action(
                cx.listener(|view, _: &PreviousPrompt, _, cx| view.scroll_to_prompt(true, cx)),
            )
            .on_action(cx.listener(|view, _: &NextPrompt, _, cx| view.scroll_to_prompt(false, cx)))
            .on_any_mouse_down(cx.listener(Self::mouse_down))
            .on_mouse_move(cx.listener(Self::mouse_move))
            .on_scroll_wheel(cx.listener(Self::wheel))
            .on_drop(cx.listener(|view, paths: &ExternalPaths, _, cx| view.drop_paths(paths, cx)))
            .size_full()
            .px(px(settings.padding_x))
            .py(px(settings.padding_y))
            // The window's glass grounds the theme's background; a program's own paints over it.
            .when(ink.bg != theme.background, |root| {
                root.bg(ink.bg.alpha(settings.background_opacity))
            });
        let root = [MouseButton::Left, MouseButton::Middle, MouseButton::Right]
            .into_iter()
            .fold(root, |root, button| {
                root.on_mouse_up(button, cx.listener(Self::mouse_up))
                    .on_mouse_up_out(button, cx.listener(Self::mouse_up))
            });
        let root = match look(&settings, zoom::size(cx), &ink, window) {
            Ok(look) => {
                self.cell = (
                    f32::from(look.cell.width).round() as u16,
                    f32::from(look.cell.height).round() as u16,
                );
                let preedit = self.marked.clone();
                root.child(paint::grid(
                    cx.entity(),
                    Rc::new(shown),
                    cursor,
                    preedit,
                    look,
                ))
            }
            Err(error) => {
                log::error!("terminal {}: {error:#}", self.pty.pid);
                root.text_color(ink.fg).child(format!("{error:#}"))
            }
        };
        div()
            .relative()
            .size_full()
            .child(root)
            .children(scrollbar::bar(span, window, cx))
            .children(
                self.find_box(cx)
                    .map(|found| div().absolute().top_2().right_4().child(found)),
            )
    }
}
