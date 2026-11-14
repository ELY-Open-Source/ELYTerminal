//! A close that would end running programs asks first: the window's, Quit's and the last session's.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    i18n::text,
    overlays::{Close, Dialog},
    theme::{ActiveTheme, TextSize},
};
use gpui::{App, Context, ParentElement, Styled, Window, div};

use super::Workspace;
use crate::{sessions::plain, settings::SettingsStore};

/// What a confirmed close does.
#[derive(Clone, Copy, Debug)]
pub enum Leave {
    /// The window closes.
    Window,
    /// The app quits.
    Quit,
    /// Session `0` closes, or only its pane `1`; the window goes with its last.
    Close(usize, Option<usize>),
}

impl Workspace {
    /// Each pane running a program: its session's number and the program.
    fn running(&self, cx: &App) -> Vec<String> {
        self.sessions
            .iter()
            .enumerate()
            .flat_map(|(index, session)| {
                session.panes.iter().filter_map(move |pane| {
                    let program = pane.terminal.read(cx).busy()?;
                    Some(format!("{} {program}", index + 1))
                })
            })
            .collect()
    }

    /// Asks before `leave` while a program runs and the settings ask; false when nothing asks.
    pub fn asks(&mut self, leave: Leave, cx: &mut Context<Self>) -> bool {
        if !SettingsStore::get(cx).confirm_quit {
            return false;
        }
        let running = self.running(cx);
        if running.is_empty() {
            return false;
        }
        log::info!(
            "app: {leave:?} asks first, {} running: {running:?}",
            running.len()
        );
        self.leaving = Some((leave, running));
        cx.notify();
        true
    }

    /// A close the user asked for: one that ends the window asks first.
    pub fn close_asked(
        &mut self,
        index: usize,
        pane: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let last =
            self.sessions.len() == 1 && pane.is_none_or(|_| self.sessions[index].panes.len() == 1);
        if !(last && self.asks(Leave::Close(index, pane), cx)) {
            self.go(Leave::Close(index, pane), window, cx);
        }
    }

    fn go(&mut self, leave: Leave, window: &mut Window, cx: &mut Context<Self>) {
        match leave {
            Leave::Window => {
                log::info!("app: the window closes");
                window.remove_window();
            }
            Leave::Quit => {
                log::info!("app: quitting");
                cx.quit();
            }
            Leave::Close(index, Some(pane)) => self.close_pane(index, pane, window, cx),
            Leave::Close(index, None) => self.close(index, window, cx),
        }
    }

    /// The question while a close waits for the user's word.
    pub(super) fn leave_prompt(&self, cx: &Context<Self>) -> Option<Dialog> {
        let (leave, running) = self.leaving.clone()?;
        let this = cx.entity();
        let settle = move |go: bool| {
            let this = this.clone();
            move |window: &mut Window, cx: &mut App| {
                this.update(cx, |this, cx| {
                    if this.leaving.take().is_none() {
                        return;
                    }
                    log::info!(
                        "app: {leave:?} {}",
                        if go { "goes on" } else { "cancelled" }
                    );
                    if go {
                        this.go(leave, window, cx);
                    }
                    cx.notify();
                })
            }
        };
        let theme = cx.theme();
        let list = div()
            .flex()
            .flex_col()
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(theme.colors.fg)
            .children(running.iter().map(|line| div().child(plain(line))));
        let button = |id: &'static str, key: &str, go, variant| {
            let label = text(cx, key, &[]);
            let pick = settle(go);
            move |close: Close| {
                Button::new(id, label)
                    .variant(variant)
                    .on_click(move |_, window, cx| {
                        pick(window, cx);
                        close(window, cx);
                    })
            }
        };
        let closed = settle(false);
        let count = running.len().to_string();
        Some(
            Dialog::new(
                "leave-guard",
                text(cx, "et.leave.title", &[("count", &count)]),
                move |window, cx| closed(window, cx),
            )
            .detail(text(cx, "et.leave.detail", &[]))
            .child(list)
            .action(button(
                "leave-cancel",
                "et.leave.cancel",
                false,
                ButtonVariant::Secondary,
            ))
            .action(button(
                "leave-go",
                "et.leave.go",
                true,
                ButtonVariant::Primary,
            )),
        )
    }
}
