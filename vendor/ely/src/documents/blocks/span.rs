use gpui::{
    ClipboardItem, Context, DispatchPhase, Focusable, IntoElement, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, Styled, Subscription, Window, canvas,
};

use super::{BlockData, BlockEditor};

/// A place in a field: its block, the field, and a byte offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Spot {
    pub key: u64,
    pub field: usize,
    pub at: usize,
}

/// A press in a field that may drag out of it, and a selection across fields as anchor and head.
#[derive(Default)]
pub(crate) struct Across {
    press: Option<Spot>,
    span: Option<(Spot, Spot)>,
    /// Watches the focused end: an edit there takes the selection's place, a caret move lets it go.
    held: Option<Subscription>,
    /// The focused end, and how many edits it had taken.
    holder: Option<((u64, usize), u64)>,
}

/// What takes a selection's place: an edit already made at one end, nothing, or a break.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Put {
    Typed(u64, usize),
    Nothing,
    Break,
}

impl BlockEditor {
    fn rank(&self, order: &[(u64, usize)], spot: Spot) -> Option<(usize, usize)> {
        let at = order
            .iter()
            .position(|each| *each == (spot.key, spot.field))?;
        Some((at, spot.at))
    }

    /// The selection across fields, start first in reading order.
    pub(crate) fn span(&self) -> Option<(Spot, Spot)> {
        let (anchor, head) = self.across.span?;
        let order = self.reading_order();
        let forward = self.rank(&order, anchor)? <= self.rank(&order, head)?;
        Some(if forward {
            (anchor, head)
        } else {
            (head, anchor)
        })
    }

    pub(crate) fn pressing(&self) -> bool {
        self.across.press.is_some()
    }

    /// The blocks wholly inside the selection, by place.
    pub(crate) fn washed(&self) -> Option<std::ops::Range<usize>> {
        let (start, end) = self.span()?;
        Some(self.index(start.key)? + 1..self.index(end.key)?)
    }

    fn spot(
        &self,
        key: u64,
        field: usize,
        position: Point<Pixels>,
        cx: &Context<Self>,
    ) -> Option<Spot> {
        let input = self.blocks[self.place(key, field)?].fields[field].read(cx);
        Some(Spot {
            key,
            field,
            at: input.offset_for_point(position),
        })
    }

    /// A press in a field: with Shift, it selects from the caret's field to here; else a drag may start.
    pub(crate) fn pressed(
        &mut self,
        key: u64,
        field: usize,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(here) = self.spot(key, field, event.position, cx) else {
            return;
        };
        let prior = self
            .caret
            .filter(|caret| *caret != (key, field))
            .and_then(|(key, field)| {
                let input = self.blocks.get(self.index(key)?)?.fields.get(field)?;
                let at = input.read(cx).cursor();
                Some(Spot { key, field, at })
            });
        self.clear_span(window, cx);
        self.across.press = None;
        match prior.filter(|_| event.modifiers.shift) {
            Some(prior) => {
                log::info!(
                    "block editor: Shift selects from block {} to {key}",
                    prior.key
                );
                self.across.span = Some((prior, here));
                self.show_span(window, cx);
                self.hold(window, cx);
            }
            None => self.across.press = Some(here),
        }
        cx.notify();
    }

    /// The shown field nearest `position`: inside it, else closest above or below, then across.
    fn field_at(&self, position: Point<Pixels>, cx: &Context<Self>) -> Option<(u64, usize)> {
        let outside =
            |near: Pixels, far: Pixels, at: Pixels| (near - at).max(at - far).max(Pixels::ZERO);
        self.reading_order()
            .into_iter()
            .filter_map(|(key, field)| {
                let block = &self.blocks[self.index(key)?];
                let bounds = block.fields[field].read(cx).layout.as_ref()?.bounds;
                let dy = outside(bounds.top(), bounds.bottom(), position.y);
                let dx = outside(bounds.left(), bounds.right(), position.x);
                Some(((dy, dx), (key, field)))
            })
            .min_by(|one, two| one.0.partial_cmp(&two.0).expect("pixels are finite"))
            .map(|(_, field)| field)
    }

    fn drag_to(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(press) = self.across.press else {
            return;
        };
        let Some((key, field)) = self.field_at(position, cx) else {
            return;
        };
        if (key, field) == (press.key, press.field) {
            return self.clear_span(window, cx);
        }
        let Some(head) = self.spot(key, field, position, cx) else {
            return;
        };
        if self.across.span != Some((press, head)) {
            if self.across.span.is_none() {
                log::info!("block editor: a drag selects across blocks");
            }
            self.across.span = Some((press, head));
            self.show_span(window, cx);
            self.hold(window, cx);
        }
    }

    /// Follows a drag that began in a field until the button lifts.
    pub(crate) fn drag_listener(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let editor = cx.entity();
        canvas(
            |_, _, _| {},
            move |_, _, window, _| {
                let moved = editor.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                    if phase == DispatchPhase::Bubble && event.dragging() {
                        moved.update(cx, |editor, cx| editor.drag_to(event.position, window, cx));
                    }
                });
                let lifted = editor.clone();
                window.on_mouse_event(move |_: &MouseUpEvent, phase, _, cx| {
                    if phase == DispatchPhase::Bubble {
                        lifted.update(cx, |editor, cx| {
                            editor.across.press = None;
                            cx.notify();
                        });
                    }
                });
            },
        )
        .absolute()
    }

    /// Selects each end field's part and whole fields between in the end blocks; the pressed field selects itself.
    fn show_span(&mut self, window: &Window, cx: &mut Context<Self>) {
        let Some((start, end)) = self.span() else {
            return;
        };
        let order = self.reading_order();
        let (from, to) = (self.rank(&order, start), self.rank(&order, end));
        let pressed = self.across.press.map(|press| (press.key, press.field));
        for block in &self.blocks {
            for (ix, field) in block.fields.iter().enumerate() {
                if pressed == Some((block.key, ix)) {
                    continue;
                }
                let len = field.read(cx).text().len();
                let rank = order.iter().position(|each| *each == (block.key, ix));
                let ends = block.key == start.key || block.key == end.key;
                let want = match rank {
                    Some(rank) if Some((rank, start.at)) == from => Some(start.at..len),
                    Some(rank) if Some((rank, end.at)) == to => Some(0..end.at),
                    Some(rank)
                        if ends
                            && from
                                .zip(to)
                                .is_some_and(|(from, to)| from.0 < rank && rank < to.0) =>
                    {
                        Some(0..len)
                    }
                    _ => None,
                };
                let now = field.read(cx).selection();
                match want {
                    Some(range) if range != now => {
                        field.update(cx, |field, cx| field.select(range, cx))
                    }
                    None if !now.is_empty() && !field.focus_handle(cx).is_focused(window) => {
                        let cursor = field.read(cx).cursor();
                        field.update(cx, |field, cx| field.select(cursor..cursor, cx));
                    }
                    _ => {}
                }
            }
        }
        cx.notify();
    }

    /// Lets the selection across fields go, and the end fields' parts with it.
    pub(crate) fn clear_span(&mut self, window: &Window, cx: &mut Context<Self>) {
        (self.across.held, self.across.holder) = (None, None);
        if self.across.span.take().is_none() {
            return;
        }
        log::debug!("block editor: the selection across blocks lets go");
        for field in self.blocks.iter().flat_map(|block| &block.fields) {
            let now = field.read(cx).selection();
            if !now.is_empty() && !field.focus_handle(cx).is_focused(window) {
                let cursor = field.read(cx).cursor();
                field.update(cx, |field, cx| field.select(cursor..cursor, cx));
            }
        }
        cx.notify();
    }

    /// Watches the focused end field from the moment a selection spans fields.
    fn hold(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.across.held.is_some() {
            return;
        }
        let Some((key, field)) = self.focused(window, cx) else {
            return;
        };
        let input =
            self.blocks[self.index(key).expect("a focused block is drawn")].fields[field].clone();
        let (edits, _) = input.read(cx).edits();
        self.across.holder = Some(((key, field), edits));
        self.across.held = Some(cx.observe_in(&input, window, move |editor, _, window, cx| {
            editor.held(key, field, window, cx)
        }));
    }

    /// The focused end changed: an edit takes the selection's place once composing ends; focus or a caret leaving lets it go.
    fn held(&mut self, key: u64, field: usize, window: &mut Window, cx: &mut Context<Self>) {
        let (Some((start, end)), Some(ix)) = (self.span(), self.index(key)) else {
            return;
        };
        let input = self.blocks[ix].fields[field].read(cx);
        let ((edits, composing), counted) = (input.edits(), self.across.holder.map(|(_, n)| n));
        if !input.focus_handle(cx).is_focused(window) {
            return self.clear_span(window, cx);
        }
        if Some(edits) != counted {
            if !composing {
                self.collapse_span(Put::Typed(key, field), window, cx);
            }
            return;
        }
        let expected = match (key, field) {
            at if at == (start.key, start.field) => start.at..input.text().len(),
            at if at == (end.key, end.field) => 0..end.at,
            _ => 0..0,
        };
        if input.selection() != expected {
            self.clear_span(window, cx);
        }
    }

    /// The selection across fields as Markdown on the clipboard; false when there is none.
    pub(crate) fn copy_span(&self, cx: &mut Context<Self>) -> bool {
        let Some((start, end)) = self.span() else {
            return false;
        };
        let order = self.reading_order();
        let (Some(from), Some(to)) = (self.rank(&order, start), self.rank(&order, end)) else {
            return false;
        };
        let (Some(first), Some(last)) = (self.index(start.key), self.index(end.key)) else {
            return false;
        };
        let snapshot = self.snapshot(cx);
        let blocks: Option<Vec<BlockData>> = snapshot.blocks[first..=last]
            .iter()
            .map(|(key, data)| {
                let mut data = data.clone();
                for (ix, text) in data.texts.iter_mut().enumerate() {
                    let rank = order.iter().position(|each| *each == (*key, ix));
                    *text = match rank {
                        Some(rank) if rank == from.0 => text.get(start.at..)?.to_string(),
                        Some(rank) if rank == to.0 => text.get(..end.at)?.to_string(),
                        Some(rank) if rank < from.0 || rank > to.0 => String::new(),
                        _ => std::mem::take(text),
                    };
                }
                Some(data)
            })
            .collect();
        let Some(blocks) = blocks else {
            log::error!("block editor: the selection across blocks no longer fits its text");
            return false;
        };
        let markdown = BlockData::to_markdown(&blocks);
        cx.write_to_clipboard(ClipboardItem::new_string(markdown.trim_end().to_string()));
        log::info!("block editor: {} blocks copied as markdown", blocks.len());
        true
    }
}

impl BlockEditor {
    /// A field changed while a selection spans fields: the held end's edit is the selection's to take; another lets it go.
    pub(crate) fn span_takes(
        &mut self,
        key: u64,
        field: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.across.span.is_none() {
            return false;
        }
        if self.across.holder.is_some_and(|(at, _)| at == (key, field)) {
            return true;
        }
        log::info!(
            "block editor: field {field} of block {key} changed; the selection across blocks lets go"
        );
        self.clear_span(window, cx);
        false
    }

    /// Replaces the selection across fields with `put`, as one undo step. Two prose ends join, or part at a break.
    pub(crate) fn collapse_span(
        &mut self,
        put: Put,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Spot> {
        let (start, end) = self.span()?;
        let typed_at = match put {
            Put::Typed(key, field) => Some((key, field)),
            Put::Nothing | Put::Break => None,
        };
        let was = |spot: Spot| {
            let (_, data) = self
                .current
                .blocks
                .iter()
                .find(|(key, _)| *key == spot.key)?;
            data.texts.get(spot.field).cloned()
        };
        let now = |spot: Spot| {
            let block = &self.blocks[self.place(spot.key, spot.field)?];
            Some(block.fields[spot.field].read(cx).text().to_string())
        };
        let parts = was(start).zip(was(end)).and_then(|(first, last)| {
            Some((
                first.get(..start.at)?.to_string(),
                last.get(end.at..)?.to_string(),
            ))
        });
        let typed = parts.as_ref().and_then(|(head, tail)| match typed_at {
            Some(at) if at == (start.key, start.field) => {
                now(start)?.strip_prefix(head.as_str()).map(str::to_string)
            }
            Some(_) => now(end)?.strip_suffix(tail.as_str()).map(str::to_string),
            None => Some(String::new()),
        });
        let (Some((head, tail)), Some(typed)) = (parts, typed) else {
            log::error!("block editor: an edit across blocks landed outside its selection");
            self.clear_span(window, cx);
            return None;
        };
        let (first, last) = (self.index(start.key)?, self.index(end.key)?);
        let mut before = self.current.clone();
        before.caret = Some((start.key, start.field, start.at..start.at));
        self.history.record(before, false);
        let prose = |ix: usize| self.blocks[ix].kind.is_prose();
        let joins = first != last && prose(first) && prose(last);
        let at_end = typed_at == Some((end.key, end.field));
        let order = self.reading_order();
        let (from, to) = (self.rank(&order, start)?, self.rank(&order, end)?);
        let set = |spot: Spot, text: String, cx: &mut Context<Self>| {
            if let Some(ix) = self.place(spot.key, spot.field) {
                self.blocks[ix].fields[spot.field].update(cx, |field, cx| field.set_text(text, cx));
            }
        };
        let parts = joins && put == Put::Break;
        let caret = if parts {
            set(start, head.clone(), cx);
            None
        } else if joins {
            set(start, format!("{head}{typed}{tail}"), cx);
            Some(Spot {
                at: head.len() + typed.len(),
                ..start
            })
        } else {
            for (key, field) in order.iter().take(to.0).skip(from.0 + 1) {
                if *key == start.key || *key == end.key {
                    set(
                        Spot {
                            key: *key,
                            field: *field,
                            at: 0,
                        },
                        String::new(),
                        cx,
                    );
                }
            }
            match at_end {
                true => {
                    set(start, head.clone(), cx);
                    set(end, format!("{typed}{tail}"), cx);
                    Some(Spot {
                        at: typed.len(),
                        ..end
                    })
                }
                false => {
                    set(start, format!("{head}{typed}"), cx);
                    set(end, tail.clone(), cx);
                    Some(Spot {
                        at: head.len() + typed.len(),
                        ..start
                    })
                }
            }
        };
        let gone = if joins {
            first + 1..last + 1
        } else {
            first + 1..last.max(first + 1)
        };
        log::info!(
            "block editor: a selection across {} blocks is replaced",
            last - first + 1
        );
        self.blocks.drain(gone);
        let caret = match caret {
            Some(caret) => caret,
            None => {
                let key = self.fresh_key();
                let kind = self.blocks[first].kind.next();
                let data = BlockData {
                    kind,
                    texts: vec![tail],
                };
                let block = self.build(data, key, window, cx);
                self.blocks.insert(first + 1, block);
                Spot {
                    key,
                    field: 0,
                    at: 0,
                }
            }
        };
        self.across = Across::default();
        self.after_change(cx);
        self.focus_field(caret.key, caret.field, Some(caret.at), window, cx);
        Some(caret)
    }
}
