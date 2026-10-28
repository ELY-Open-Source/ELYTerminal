use std::time::{Duration, Instant};

use ely_gpui_component::i18n::I18n;
use gpui::{Context, SharedString, SystemNotification};

use super::{
    notify::{self, Ask, CLOSES, Notice},
    status::{self, Record, State, Status},
    view::{TerminalEvent, TerminalView},
};
use crate::{
    sessions::{line, plain},
    workspace::TITLE,
};

/// One notification a session every 3 seconds at most.
const GAP: Duration = Duration::from_secs(3);

/// The least a command runs before its end posts, as Ghostty's `notify-on-command-finish-after`.
const RAN_FOR: Duration = Duration::from_secs(5);

/// The notifications a session keeps for presses and answers; past them the oldest goes.
const POSTED: usize = 64;

/// The system notifications a session posted and has not seen go.
#[derive(Default)]
pub struct Notices {
    posted: Vec<Posted>,
    /// When it last posted.
    last: Option<Instant>,
    /// Posts so far: they tell apart notifications without an id, and an expiry finds its own post.
    count: u64,
}

/// A notification posted.
#[derive(Clone, Debug, PartialEq)]
struct Posted {
    tag: SharedString,
    /// The OSC 99 id its program gave; none for the session's own and for OSC 9 and 777.
    id: Option<String>,
    focus: bool,
    report: bool,
    count: u64,
}

impl Notices {
    /// Program `pid`'s turn to post at `now`, kept as posted; none within 3 seconds of its last.
    fn turn(&mut self, pid: u32, asked: Option<&Notice>, now: Instant) -> Option<Posted> {
        if self.last.is_some_and(|last| now.duration_since(last) < GAP) {
            return None;
        }
        self.last = Some(now);
        self.count += 1;
        let id = asked.and_then(|notice| notice.id.clone());
        // The session's own share a tag, so the last takes its place; an OSC 99 id shares its own.
        let tag = match (asked, &id) {
            (None, _) => format!("{pid}"),
            (Some(_), Some(id)) => format!("{pid}:{id}"),
            (Some(_), None) => format!("{pid}#{}", self.count),
        };
        let posted = Posted {
            tag: tag.into(),
            id,
            focus: asked.is_none_or(|notice| notice.focus),
            report: asked.is_some_and(|notice| notice.report),
            count: self.count,
        };
        self.posted.retain(|old| old.tag != posted.tag);
        if self.posted.len() == POSTED {
            let gone = self.posted.remove(0);
            log::debug!(
                "pty {pid}: notification {} goes from the {POSTED} kept",
                gone.tag
            );
        }
        self.posted.push(posted.clone());
        Some(posted)
    }

    /// Takes notification `tag` out; with `count`, only that post of it.
    fn take(&mut self, tag: &str, count: Option<u64>) -> Option<Posted> {
        let at = self.posted.iter().position(|posted| {
            posted.tag == tag && count.is_none_or(|count| posted.count == count)
        })?;
        Some(self.posted.remove(at))
    }

    /// Whether a post after post `count` holds `tag`.
    fn replaced(&self, tag: &str, count: u64) -> bool {
        self.posted
            .iter()
            .any(|posted| posted.tag == tag && posted.count > count)
    }

    /// The ids of the notifications still alive: posted, and not pressed, closed or expired.
    fn alive(&self) -> Vec<&str> {
        self.posted
            .iter()
            .filter_map(|posted| posted.id.as_deref())
            .collect()
    }
}

impl TerminalView {
    /// Keeps a program's report, drops what it clears, and answers its query.
    pub(super) fn status(&mut self, status: Status, owner: Option<i32>) {
        match status {
            Status::Report(id, record) => self.records.report(id, record, owner),
            Status::Clear(id) => self.records.clear(id.as_deref()),
            Status::Query => self.pty.write(status::REPLY.as_bytes().to_vec()),
        }
    }

    /// Maps progress, answers the query and alive in order, and posts and closes notifications.
    pub(super) fn ask(&mut self, ask: Ask, owner: Option<i32>, cx: &mut Context<Self>) {
        match ask {
            Ask::Progress(progress) => self.records.progress(progress, owner),
            Ask::Query(id) => self.pty.write(notify::answer(&id).into_bytes()),
            Ask::Alive(id) => {
                let alive = self.notices.alive();
                log::debug!("pty {}: notifications alive: {alive:?}", self.pty.pid);
                self.pty.write(notify::alive(&id, &alive).into_bytes());
            }
            Ask::Show(notice) => self.post(said(&notice), Some(&notice), cx),
            Ask::Close(id) => self.close(&id, cx),
        }
    }

    /// Notes the state its row shows; a change goes to the owner, and one to waiting, done or error posts.
    pub(super) fn restate(&mut self, cx: &mut Context<Self>) {
        if self.records.restate() {
            let state = self.records.shows();
            log::info!("terminal {}: its row shows {state:?}", self.pty.pid);
            cx.emit(TerminalEvent::Shows(state));
            self.tell_state(cx);
        }
    }

    /// herdr's agents in place of the last ones; an open hover reads them, so the view draws again.
    pub(super) fn agents(&mut self, agents: &[(String, Record)], cx: &mut Context<Self>) {
        if self.records.herdr(agents) {
            self.restate(cx);
            cx.notify();
        }
    }

    /// Posts the state its row shows when it turned to waiting, done or error.
    fn tell_state(&mut self, cx: &mut Context<Self>) {
        let body = match self.records.shown() {
            Some((id, record)) if tells(record.state) => {
                line(&self.records, id, record, cx.global::<I18n>())
            }
            _ => return,
        };
        self.post(body, None, cx);
    }

    /// A command the shell ran ended with `status`, which a notification tells.
    pub(super) fn finished(&mut self, status: Option<i32>, cx: &mut Context<Self>) {
        if self.ended != status {
            let ended = status.map_or("with no status".into(), |code| format!("with {code}"));
            log::info!("terminal {}: its last command ended {ended}", self.pty.pid);
        }
        self.ended = status;
        // Only a command C started posts its end, so a prompt bash 3.2 draws again posts nothing.
        if let Some(start) = std::mem::take(&mut self.ran) {
            let now = Instant::now();
            if ran_long(start, now) {
                self.post(finished(status, cx.global::<I18n>()), None, cx);
            } else {
                log::debug!(
                    "terminal {}: a command under {RAN_FOR:?} ran, so its end, {status:?}, posts nothing",
                    self.pty.pid
                );
            }
        } else {
            log::debug!(
                "terminal {}: an end without its start posts nothing",
                self.pty.pid
            );
        }
    }

    /// Posts `body` while the window is not active or the session hidden, one every 3 seconds at most.
    fn post(&mut self, body: String, asked: Option<&Notice>, cx: &mut Context<Self>) {
        let pid = self.pty.pid;
        if !unseen(self.front, self.active) {
            log::debug!("terminal {pid}: in view, so it posts no {body:?}");
            return;
        }
        let Some(posted) = self.notices.turn(pid, asked, Instant::now()) else {
            log::info!("terminal {pid}: posted within 3 s, so it posts no {body:?}");
            return;
        };
        // Its title names the session, so no program speaks as another.
        let title = plain(if self.title.is_empty() {
            TITLE
        } else {
            &self.title
        });
        log::info!("terminal {pid}: posts {}: {title:?}, {body:?}", posted.tag);
        // Linux's notification daemons read the body as markup.
        let body = if cfg!(target_os = "linux") {
            escape(&body)
        } else {
            body
        };
        cx.show_system_notification(SystemNotification {
            tag: posted.tag.clone(),
            title: title.into(),
            body: body.into(),
            actions: Vec::new(),
        });
        let Some(notice) = asked else {
            return;
        };
        // gpui hears of no close, so a program that asks hears that none is tracked.
        if notice.close {
            let id = posted.id.as_deref().unwrap_or("0");
            self.pty.write(notify::untracked(id).into_bytes());
        }
        match notice.expire {
            Some(ms) if CLOSES => self.expire(posted, ms, cx),
            Some(_) => log::debug!(
                "terminal {pid}: gpui closes none on Linux, so {} stays",
                posted.tag
            ),
            None => {}
        }
    }

    /// Takes `posted` back once `ms` run out, though its session be gone, unless a later post holds its tag.
    fn expire(&mut self, posted: Posted, ms: u32, cx: &mut Context<Self>) {
        let (tag, count) = (posted.tag, posted.count);
        cx.spawn(async move |view, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(u64::from(ms)))
                .await;
            let held = view.update(cx, |view, cx| {
                if view.notices.replaced(&tag, count) {
                    return;
                }
                if view.notices.take(&tag, Some(count)).is_some() {
                    log::info!("terminal {}: notification {tag} expires", view.pty.pid);
                }
                cx.dismiss_system_notification(&tag);
            });
            // The system keeps what a closed session posted, so its expiry still takes it.
            if held.is_err() {
                log::info!("app: notification {tag} expires after its session");
                cx.update(|cx| cx.dismiss_system_notification(&tag));
            }
        })
        .detach();
    }

    /// A program's close of notification `id`, where gpui can close one.
    fn close(&mut self, id: &str, cx: &mut Context<Self>) {
        let tag = format!("{}:{id}", self.pty.pid);
        if !CLOSES {
            log::debug!(
                "terminal {}: gpui closes none on Linux, so {tag} stays",
                self.pty.pid
            );
        } else if self.notices.take(&tag, None).is_some() {
            log::info!("terminal {}: closes notification {tag}", self.pty.pid);
            cx.dismiss_system_notification(&tag);
        }
    }

    /// A press on notification `tag`, if this session posted it: a program that asked hears of it; whether the session shows.
    pub fn pressed(&mut self, tag: &str) -> Option<bool> {
        let pid = self.pty.pid;
        let Some(posted) = self.notices.take(tag, None) else {
            // A twin Linux stacked, pressed before, or one past the 64 kept, still shows its session.
            let own = owns(pid, tag);
            if own {
                log::info!(
                    "terminal {pid}: notification {tag} pressed, its record gone, so it tells no program"
                );
            }
            return own.then_some(true);
        };
        log::info!("terminal {pid}: notification {tag} pressed");
        if posted.report {
            let id = posted.id.as_deref().unwrap_or("0");
            self.pty.write(notify::activated(id).into_bytes());
        }
        Some(posted.focus)
    }
}

/// Whether a session posts: not while it is the session shown in an active window.
fn unseen(front: bool, active: bool) -> bool {
    !(front && active)
}

/// Whether the row's turn to `state` posts: waiting, done and error do.
fn tells(state: State) -> bool {
    matches!(state, State::Blocked | State::Done | State::Error)
}

/// Whether program `pid`'s session posted notification `tag`: its own, `PID`, else `PID:ID` or `PID#N`.
fn owns(pid: u32, tag: &str) -> bool {
    tag.strip_prefix(&pid.to_string())
        .is_some_and(|rest| rest.is_empty() || rest.starts_with([':', '#']))
}

/// `text` as markup reads it as text.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Whether a command that ran from `start` to `now` ran long enough to post its end.
fn ran_long(start: Instant, now: Instant) -> bool {
    now.duration_since(start) >= RAN_FOR
}

/// What a command's end says: it finished, or the status it failed with.
fn finished(status: Option<i32>, words: &I18n) -> String {
    match status {
        Some(code) if code != 0 => words.text("et.command.exited", &[("code", &code.to_string())]),
        _ => words.text("et.command.finished", &[]),
    }
    .to_string()
}

/// A program's notification as posted: its title, then its body, each line shown as text.
fn said(notice: &Notice) -> String {
    let lines = notice.title.lines().chain(notice.body.lines());
    let shown: Vec<String> = lines.map(plain).filter(|line| !line.is_empty()).collect();
    shown.join("\n")
}

#[cfg(test)]
mod tests;
