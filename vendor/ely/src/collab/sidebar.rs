use std::rc::Rc;

use gpui::{
    App, ElementId, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce, Role,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use super::comments::{CommentThread, Thread};
use crate::{
    buttons::SegmentedControl,
    feedback::EmptyState,
    forms::{OnValue, Run, TextInput},
    i18n,
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, Radius},
};

type OnThreadFlag = Rc<dyn Fn(&SharedString, bool, &mut Window, &mut App)>;
type OnThreadReact = Rc<dyn Fn(&SharedString, usize, &SharedString, &mut Window, &mut App)>;

/// Every conversation on a document down a column, split into open and resolved when the owner resolves: a press picks one, and the one picked takes replies.
#[derive(IntoElement)]
pub struct CommentSidebar {
    id: ElementId,
    threads: Vec<Thread>,
    active: Option<SharedString>,
    reply: Option<(Entity<TextInput>, Run)>,
    on_select: Option<OnValue>,
    on_resolve: Option<OnThreadFlag>,
    on_react: Option<OnThreadReact>,
}

impl CommentSidebar {
    pub fn new(id: impl Into<ElementId>, threads: impl IntoIterator<Item = Thread>) -> Self {
        Self {
            id: id.into(),
            threads: threads.into_iter().collect(),
            active: None,
            reply: None,
            on_select: None,
            on_resolve: None,
            on_react: None,
        }
    }

    /// The thread picked, by key, and what a press on another asks.
    pub fn active(
        mut self,
        active: Option<SharedString>,
        on_select: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.active = active;
        self.on_select = Some(Rc::new(on_select));
        self
    }

    /// Answers the picked thread in `field`.
    pub fn reply(
        mut self,
        field: &Entity<TextInput>,
        on_send: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.reply = Some((field.clone(), Rc::new(on_send)));
        self
    }

    /// Gets a thread's key and true to resolve it, false to reopen.
    pub fn on_resolve(
        mut self,
        handler: impl Fn(&SharedString, bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_resolve = Some(Rc::new(handler));
        self
    }

    /// Gets a thread's key, a comment's index and the emoji to toggle on it.
    pub fn on_react(
        mut self,
        handler: impl Fn(&SharedString, usize, &SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_react = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for CommentSidebar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let showing = window.use_keyed_state((self.id.clone(), "resolved"), cx, |_, _| false);
        // Without resolving, one list.
        let resolving = self.on_resolve.is_some();
        let resolved = resolving && *showing.read(cx);
        let open = self
            .threads
            .iter()
            .filter(|thread| !thread.resolved)
            .count();
        let done = self.threads.len() - open;
        let filter = resolving.then(|| {
            SegmentedControl::new(
                (self.id.clone(), "filter"),
                if resolved { "resolved" } else { "open" },
            )
            .size(ControlSize::Sm)
            .segment(
                "open",
                i18n::text(cx, "comments.open", &[("n", &open.to_string())]),
                None,
            )
            .segment(
                "resolved",
                i18n::text(cx, "comments.resolved", &[("n", &done.to_string())]),
                None,
            )
            .on_change(move |value, _, cx| {
                showing.update(cx, |showing, cx| {
                    *showing = value.as_ref() == "resolved";
                    cx.notify();
                })
            })
        });
        if let Some(active) = &self.active
            && !self.threads.iter().any(|thread| &thread.key == active)
        {
            log::error!("comment sidebar: thread {active} is gone; none active");
        }
        let shown: Vec<Thread> = self
            .threads
            .into_iter()
            .filter(|thread| !resolving || thread.resolved == resolved)
            .collect();
        let none = match (resolving, resolved) {
            (false, _) => "comments.none",
            (true, true) => "comments.none_resolved",
            (true, false) => "comments.none_open",
        };
        let empty = shown.is_empty().then(|| {
            EmptyState::new(
                (self.id.clone(), "empty"),
                IconName::MessageSquare,
                i18n::text(cx, none, &[]),
            )
        });
        let count = shown.len();
        let theme = cx.theme();
        let (ring, radius) = (theme.colors.focus, theme.radius(Radius::Md));
        let threads = shown.into_iter().enumerate().map(|(at, thread)| {
            let name: SharedString = match (thread.comments.first(), &thread.quote) {
                (Some(first), _) => format!("{}: {}", first.author.name, first.body).into(),
                (None, Some(quote)) => quote.clone(),
                (None, None) => thread.key.clone(),
            };
            let key = thread.key.clone();
            let active = self.active.as_ref() == Some(&key);
            let mut item = CommentThread::new((self.id.clone(), format!("thread-{key}")), thread)
                .active(active);
            if active && let Some((field, send)) = &self.reply {
                let send = send.clone();
                item = item.reply(field, move |window, cx| send(window, cx));
            }
            if let Some(resolve) = self.on_resolve.clone() {
                let key = key.clone();
                item = item.on_resolve(move |on, window, cx| resolve(&key, on, window, cx));
            }
            if let Some(react) = self.on_react.clone() {
                let key = key.clone();
                item =
                    item.on_react(move |ix, emoji, window, cx| react(&key, ix, emoji, window, cx));
            }
            let select = self.on_select.clone().filter(|_| !active);
            let row = key.clone();
            div()
                .id((self.id.clone(), format!("pick-{key}")))
                .debug_selector(move || format!("comment-{row}"))
                .role(Role::ListItem)
                .aria_label(name)
                .aria_selected(active)
                .aria_position_in_set(at + 1)
                .aria_size_of_set(count)
                .tab_index(0)
                .rounded(radius)
                .border_1()
                .border_color(gpui::transparent_black())
                .focus(move |style| style.border_color(ring))
                .when_some(select, |row, select| {
                    let (click, press, pressed) = (select.clone(), key.clone(), key.clone());
                    row.on_click(move |_, window, cx| {
                        log::info!("comment sidebar: {press}");
                        click(&press, window, cx)
                    })
                    .on_key_down(move |event, window, cx| {
                        let key = event.keystroke.key.as_str();
                        if matches!(key, "enter" | "space") && !event.keystroke.modifiers.modified()
                        {
                            cx.stop_propagation();
                            log::info!("comment sidebar: {pressed}");
                            select(&pressed, window, cx)
                        }
                    })
                })
                .child(item)
        });
        div()
            .id(self.id.clone())
            .role(Role::List)
            .aria_label(i18n::text(cx, "comments.list", &[]))
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .overflow_y_scroll()
            .children(filter)
            .children(empty)
            .children(threads)
    }
}
