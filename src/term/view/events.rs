//! What the pty reports, as the view hears it.

use super::*;

/// How long output and scrolling rest before history packs.
const IDLE: Duration = Duration::from_secs(2);

impl TerminalView {
    /// Packs idle history once output and scrolling have rested.
    pub(super) fn compact_later(&mut self, cx: &mut Context<Self>) {
        self.compacting = Some(cx.spawn(async move |view, cx| {
            cx.background_executor().timer(IDLE).await;
            let packed = view.update(cx, |view, _| {
                let mut term = view.pty.term.lock();
                term.compact();
                let lines = term.grid().history_size();
                log::debug!(
                    "terminal {}: {lines} idle history lines packed",
                    view.pty.pid
                );
            });
            if packed.is_err() {
                log::debug!("compaction: the terminal closed first");
            }
        }));
    }

    pub(super) fn handle(&mut self, events: Vec<PtyEvent>, cx: &mut Context<Self>) {
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
                PtyEvent::Clipboard(packet) => self.clipboard(packet, cx),
                PtyEvent::Reset => {
                    self.records.reset();
                    self.title.clear();
                    cx.emit(TerminalEvent::Title(String::new()));
                    self.restate(cx);
                }
            }
        }
        if drawn {
            self.compact_later(cx);
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
