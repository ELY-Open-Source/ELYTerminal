//! What the main window answers a program asking through the control door.

use anyhow::{Context as _, bail};
use gpui::Context;

use super::Workspace;
use crate::control::USAGE;

impl Workspace {
    /// Answers `words`: the sessions, a session's last rows, or text or keys typed into one.
    pub fn asked(&mut self, words: &[String], cx: &mut Context<Self>) -> anyhow::Result<String> {
        let (verb, rest) = words.split_first().context("no command")?;
        if verb == "sessions" && rest.is_empty() {
            return Ok(self.listed(cx));
        }
        let (id, rest) = rest
            .split_first()
            .with_context(|| format!("{verb} of which session?\n{USAGE}"))?;
        let pid: u32 = id
            .parse()
            .with_context(|| format!("{id} is no session's number"))?;
        let terminal = self
            .sessions
            .iter()
            .map(|session| &session.terminal)
            .find(|terminal| terminal.read(cx).pid() == pid)
            .with_context(|| format!("no session {pid}; `elyterminal ctl sessions` lists them"))?
            .clone();
        match (verb.as_str(), rest) {
            ("read", []) => Ok(terminal.read(cx).tail(None) + "\n"),
            ("read", [count]) => {
                let count = count
                    .parse()
                    .with_context(|| format!("{count} is no count of lines"))?;
                Ok(terminal.read(cx).tail(Some(count)) + "\n")
            }
            ("send", [text]) => {
                terminal.update(cx, |view, cx| view.typed(text, cx));
                Ok(String::new())
            }
            ("key", keys) if !keys.is_empty() => {
                terminal.update(cx, |view, cx| view.keyed(keys, cx))?;
                Ok(String::new())
            }
            _ => bail!("{verb} does not take that\n{USAGE}"),
        }
    }

    /// Each session on a line: its number, * for the one shown, its title and its folder, tabs between.
    fn listed(&self, cx: &Context<Self>) -> String {
        let line = |(index, session): (usize, &crate::sessions::Session)| {
            let shown = if index == self.active { "*" } else { "" };
            let pid = session.terminal.read(cx).pid();
            let folder = session.folder.display();
            format!("{pid}\t{shown}\t{}\t{folder}\n", session.named())
        };
        self.sessions.iter().enumerate().map(line).collect()
    }
}
