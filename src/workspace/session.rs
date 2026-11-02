use std::path::PathBuf;

use gpui::{Context, EntityId, Window};

use super::Workspace;
use crate::{
    settings::{self, SettingsStore, StartIn},
    term::{
        config,
        shell::{ShellIntegration, Start, launch},
        spawn,
    },
};

impl Workspace {
    /// A session of the set shell, where Start in says.
    pub(super) fn new_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(start) = self.start_in(cx) else {
            return;
        };
        let shell = SettingsStore::get(cx).shell.clone();
        self.open(&shell, Vec::new(), start, None, window, cx);
    }

    /// A profile's session: `shell` with `args`, in `folder` or where Start in says.
    pub fn profile(
        &mut self,
        (shell, args): (String, Vec<String>),
        folder: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let start = match folder {
            Some(folder) => Start::In(folder),
            None => match self.start_in(cx) {
                Some(start) => start,
                None => return,
            },
        };
        log::info!("app: a profile opens {shell} with {} arguments", args.len());
        self.open(&shell, args, start, None, window, cx);
    }

    /// Where Start in puts a new session; none when its folder is gone.
    fn start_in(&self, cx: &Context<Self>) -> Option<Start> {
        let settings = SettingsStore::get(cx);
        match settings.start_in {
            StartIn::Home => Some(Start::Home),
            StartIn::Session => Some(
                self.sessions
                    .get(self.active)
                    .map_or(Start::Home, |session| Start::In(session.folder.clone())),
            ),
            StartIn::Folder => match settings::start_folder(&settings.start_folder) {
                Ok(folder) => Some(Start::In(folder)),
                Err(error) => {
                    log::error!("app: no new session: {error:#}");
                    None
                }
            },
        }
    }

    /// Session `id` again: the shell it runs, in the folder it last reported.
    pub fn duplicate(&mut self, id: EntityId, window: &mut Window, cx: &mut Context<Self>) {
        let Some((shell, args, folder, index)) = self
            .sessions
            .iter()
            .enumerate()
            .find(|(_, session)| session.terminal.entity_id() == id)
            .map(|(index, session)| {
                let shell = session.shell.clone();
                (shell, session.args.clone(), session.folder.clone(), index)
            })
        else {
            log::error!("app: there is no session {id:?} to duplicate");
            return;
        };
        log::info!("app: session {id:?} duplicates into {}", folder.display());
        self.open(&shell, args, Start::In(folder), Some(index + 1), window, cx);
    }

    /// The shown shell in `folder`, right after the shown row.
    pub fn here(&mut self, folder: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let Some((shell, args)) = self
            .sessions
            .get(self.active)
            .map(|session| (session.shell.clone(), session.args.clone()))
        else {
            log::error!("app: no session to open {}", folder.display());
            return;
        };
        log::info!("app: session here {}", folder.display());
        self.open(
            &shell,
            args,
            Start::In(folder),
            Some(self.active + 1),
            window,
            cx,
        );
    }

    /// Shows a `shell` session with `args` from `start`, at `at` or last.
    fn open(
        &mut self,
        shell: &str,
        args: Vec<String>,
        start: Start,
        at: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let settings = SettingsStore::get(cx);
        let started = launch(shell, &args, start, &cx.global::<ShellIntegration>().0)
            .and_then(|launch| Ok((spawn(&launch, (80, 24), (8, 16), config(settings))?, launch)));
        match started {
            Ok(((pty, events), launch)) => {
                self.add(pty, events, launch, args, window, cx);
                let last = self.sessions.len() - 1;
                if let Some(at) = at {
                    self.move_session(last, at);
                }
                self.select(at.map_or(last, |at| at.min(last)), window, cx);
            }
            Err(error) => log::error!("app: no new session: {error:#}"),
        }
    }
}
