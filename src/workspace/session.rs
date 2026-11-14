use std::path::PathBuf;

use anyhow::Context as _;
use gpui::{Context, EntityId, Window};

use super::Workspace;
use crate::{
    sessions::Program,
    settings::{self, SettingsStore, StartIn},
    term::{
        Events, Launch, Pty, config,
        shell::{ShellIntegration, Start, command, launch},
        spawn,
    },
};

impl Workspace {
    /// A session of the set shell, where Start in says.
    pub(super) fn new_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(start) = self.start_in(cx) else {
            return;
        };
        let program = Program {
            shell: SettingsStore::get(cx).shell.clone(),
            ..Program::default()
        };
        if let Err(error) = self.open(program, start, None, window, cx) {
            log::error!("app: no new session: {error:#}");
        }
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
        let program = Program {
            shell,
            args,
            written: false,
        };
        if let Err(error) = self.open(program, start, None, window, cx) {
            log::error!("app: no profile session: {error:#}");
        }
    }

    /// Where Start in puts a new session; none when its folder is gone.
    fn start_in(&self, cx: &Context<Self>) -> Option<Start> {
        let settings = SettingsStore::get(cx);
        match settings.start_in {
            StartIn::Home => Some(Start::Home),
            StartIn::Session => Some(
                self.sessions
                    .get(self.active)
                    .map_or(Start::Home, |session| {
                        Start::In(session.pane().folder.clone())
                    }),
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
        let Some((program, folder, index)) = self
            .sessions
            .iter()
            .enumerate()
            .find(|(_, session)| session.id == id)
            .map(|(index, session)| {
                let pane = session.pane();
                (pane.program.clone(), pane.folder.clone(), index)
            })
        else {
            log::error!("app: there is no session {id:?} to duplicate");
            return;
        };
        log::info!("app: session {id:?} duplicates into {}", folder.display());
        if let Err(error) = self.open(program, Start::In(folder), Some(index + 1), window, cx) {
            log::error!("app: no duplicate session: {error:#}");
        }
    }

    /// The shown shell in `folder`, right after the shown row.
    pub fn here(
        &mut self,
        folder: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<()> {
        let program = self
            .sessions
            .get(self.active)
            .map(|session| session.pane().program.clone())
            .context("app: no active session for the chosen folder")?;
        log::info!("app: session here {}", folder.display());
        let at = Some(self.active + 1);
        self.open(program, Start::In(folder), at, window, cx)
    }

    /// Starts `program` from `start`, its pty sized as the first layout will fit it.
    pub(super) fn started(
        &self,
        program: &Program,
        start: Start,
        cx: &Context<Self>,
    ) -> anyhow::Result<(Pty, Events, Launch)> {
        let Program {
            shell,
            args,
            written,
        } = program;
        let launch = match (written, start) {
            (true, Start::In(folder)) => command(shell, args, folder)?,
            (true, Start::Home) => anyhow::bail!("app: a written program starts in a folder"),
            (false, start) => launch(shell, args, start, &cx.global::<ShellIntegration>().0)?,
        };
        let (pty, events) = spawn(&launch, (80, 24), (8, 16), config(SettingsStore::get(cx)))?;
        Ok((pty, events, launch))
    }

    /// Shows a session of `program` from `start`, at `at` or last.
    fn open(
        &mut self,
        program: Program,
        start: Start,
        at: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<()> {
        let started = self.started(&program, start, cx)?;
        self.add(started, program, window, cx);
        let last = self.sessions.len() - 1;
        if let Some(at) = at {
            self.move_session(last, at);
        }
        self.select(at.map_or(last, |at| at.min(last)), window, cx);
        Ok(())
    }
}
