use std::rc::Rc;

use gpui::{
    App, Bounds, ElementId, Entity, ImageSource, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, RenderOnce, Styled, Window, canvas, div, prelude::*, relative,
    transparent_black,
};

use super::marks::{Look, Mark, Tool, mark_at, paint, pin_numbers};
use crate::{
    buttons::{IconButton, ToggleGroup, ToggleItem},
    forms::{ColorSwatch, Drawing, History, pen},
    layout::seeded::{Seeded, use_seeded},
    primitives::{FocusRing, IconName, Image, checked_ratio, framed, tab_stop},
    theme::{ActiveTheme, ControlSize, TextSize},
};

type OnMarks = Rc<dyn Fn(&[Mark], &mut Window, &mut App)>;

/// How many chart colors a mark may take.
const COLORS: usize = 4;

/// Each tool, its key, its icon, and its name.
const TOOLS: [(Tool, &str, IconName, &str); 5] = [
    (Tool::Select, "v", IconName::MousePointer2, "Select"),
    (Tool::Box, "b", IconName::Square, "Box"),
    (Tool::Arrow, "a", IconName::ArrowUpRight, "Arrow"),
    (Tool::Pen, "p", IconName::PenLine, "Pen"),
    (Tool::Pin, "n", IconName::MapPin, "Pin"),
];

/// The tool and color in hand, the mark chosen, and the steps to undo.
struct Hand {
    tool: Tool,
    color: usize,
    chosen: Option<usize>,
    history: History<Vec<Mark>>,
}

/// The marks, the hand, and the host to tell.
#[derive(Clone)]
struct Board {
    marks: Entity<Seeded<Vec<Mark>>>,
    hand: Entity<Hand>,
    on_change: Option<OnMarks>,
}

impl Board {
    /// Sets the marks, keeping the ones before for Undo, and tells the host.
    fn commit(&self, next: Vec<Mark>, what: &str, window: &mut Window, cx: &mut App) {
        let before = self.marks.read(cx).value.clone();
        self.hand
            .update(cx, |hand, _| hand.history.record(before, false));
        self.set(next, what, window, cx);
    }

    fn set(&self, next: Vec<Mark>, what: &str, window: &mut Window, cx: &mut App) {
        self.marks.update(cx, |marks, cx| {
            marks.value = next.clone();
            cx.notify();
        });
        log::info!("image annotator: {what}, {} marks", next.len());
        if let Some(on_change) = &self.on_change {
            on_change(&next, window, cx);
        }
    }

    /// Undo, or Redo when not `back`.
    fn step(&self, back: bool, window: &mut Window, cx: &mut App) {
        let current = self.marks.read(cx).value.clone();
        let stepped = self.hand.update(cx, |hand, _| {
            hand.chosen = None;
            if back {
                hand.history.undo(current)
            } else {
                hand.history.redo(current)
            }
        });
        if let Some(stepped) = stepped {
            self.set(stepped, if back { "undo" } else { "redo" }, window, cx);
        }
    }

    fn remove(&self, window: &mut Window, cx: &mut App) {
        let Some(ix) = self.hand.update(cx, |hand, _| hand.chosen.take()) else {
            return;
        };
        let mut next = self.marks.read(cx).value.clone();
        next.remove(ix);
        self.commit(next, "removed a mark", window, cx);
    }

    fn pick(&self, tool: Tool, cx: &mut App) {
        self.hand.update(cx, |hand, cx| {
            (hand.tool, hand.chosen) = (tool, None);
            cx.notify();
        });
    }
}

/// Where `at`, in window pixels, falls on `bounds`, in shares.
fn share(bounds: Bounds<Pixels>, at: gpui::Point<Pixels>) -> (f32, f32) {
    let at = at - bounds.origin;
    (
        f32::from(at.x) / f32::from(bounds.size.width),
        f32::from(at.y) / f32::from(bounds.size.height),
    )
}

/// A picture to mark up, its shape from the host: boxes, arrows, freehand lines and numbered pins, each in one of four chart colors. Tools and colors sit above; V, B, A, P and N pick a tool; with Select a press chooses a mark, Delete removes it and Escape lets it go; Cmd-Z and Shift-Cmd-Z undo and redo. The host keeps the marks.
#[derive(IntoElement)]
pub struct ImageAnnotator {
    id: ElementId,
    source: ImageSource,
    ratio: f32,
    marks: Vec<Mark>,
    on_change: Option<OnMarks>,
}

impl ImageAnnotator {
    /// `ratio` is the picture's width over its height.
    pub fn new(
        id: impl Into<ElementId>,
        source: impl Into<ImageSource>,
        ratio: f32,
        marks: impl IntoIterator<Item = Mark>,
    ) -> Self {
        let marks: Vec<Mark> = marks.into_iter().collect();
        for mark in &marks {
            assert!(
                mark.color() < COLORS,
                "a mark in color {} of {COLORS}",
                mark.color()
            );
        }
        Self {
            id: id.into(),
            source: source.into(),
            ratio: checked_ratio(ratio),
            marks,
            on_change: None,
        }
    }

    pub fn on_change(mut self, handler: impl Fn(&[Mark], &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ImageAnnotator {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let board = Board {
            marks: use_seeded((self.id.clone(), "marks"), self.marks, window, cx),
            hand: window.use_keyed_state((self.id.clone(), "hand"), cx, |_, _| Hand {
                tool: Tool::Box,
                color: 0,
                chosen: None,
                history: History::default(),
            }),
            on_change: self.on_change.clone(),
        };
        let drawing =
            window.use_keyed_state((self.id.clone(), "drawing"), cx, |_, _| Drawing::default());
        let editable = self.on_change.is_some();
        let focus = tab_stop((self.id.clone(), "focus").into(), editable, window, cx);
        let now = board.marks.read(cx).value.clone();
        let (tool, color, chosen, can_undo, can_redo) = {
            let hand = board.hand.read(cx);
            let chosen = hand.chosen.filter(|ix| *ix < now.len());
            (
                hand.tool,
                hand.color,
                chosen,
                hand.history.can_undo(),
                hand.history.can_redo(),
            )
        };
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let sizes = theme.media();
        let rem = window.rem_size();
        let (stroke, head, reach) = (
            sizes.mark_stroke.to_pixels(rem),
            sizes.arrow_head.to_pixels(rem),
            f32::from(sizes.pin.to_pixels(rem)) / 2.0,
        );
        let chart = colors.chart;
        let tint = move |ix: usize| chart[ix % chart.len()];
        let picked = board.clone();
        let tools = TOOLS
            .iter()
            .fold(
                ToggleGroup::new((self.id.clone(), "tools")).size(ControlSize::Sm),
                |group, (_, key, icon, name)| {
                    group.item(
                        ToggleItem::new(*key)
                            .icon(*icon)
                            .tooltip(format!("{name}, {}", key.to_uppercase())),
                    )
                },
            )
            .selected([TOOLS
                .iter()
                .find(|(each, ..)| *each == tool)
                .expect("a listed tool")
                .1])
            .on_change(move |values, _, cx| {
                if let Some(value) = values.first() {
                    let next = TOOLS
                        .iter()
                        .find(|(_, key, ..)| *key == value.as_ref())
                        .expect("a listed tool")
                        .0;
                    picked.pick(next, cx);
                }
            });
        let swatches = (0..COLORS).map(|ix| {
            let painted = board.hand.clone();
            ColorSwatch::new((self.id.clone(), format!("color-{ix}")), tint(ix))
                .size(ControlSize::Sm)
                .selected(ix == color)
                .on_click(move |_, cx| {
                    painted.update(cx, |hand, cx| {
                        hand.color = ix;
                        cx.notify();
                    })
                })
        });
        let action = |key: &'static str,
                      icon: IconName,
                      tip: &'static str,
                      enabled: bool,
                      run: fn(&Board, &mut Window, &mut App)| {
            let board = board.clone();
            IconButton::new((self.id.clone(), key), icon)
                .size(ControlSize::Sm)
                .tooltip(tip)
                .disabled(!enabled)
                .on_click(move |_, window, cx| run(&board, window, cx))
        };
        let strip = div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap_2()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_3()
                    .child(tools)
                    .child(div().flex().gap_1().children(swatches)),
            )
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(action(
                        "undo",
                        IconName::Undo2,
                        "Undo",
                        can_undo,
                        |board, window, cx| board.step(true, window, cx),
                    ))
                    .child(action(
                        "redo",
                        IconName::Redo2,
                        "Redo",
                        can_redo,
                        |board, window, cx| board.step(false, window, cx),
                    ))
                    .child(action(
                        "remove",
                        IconName::Trash2,
                        "Remove",
                        chosen.is_some(),
                        Board::remove,
                    )),
            );
        let pins =
            now.iter()
                .zip(pin_numbers(&now))
                .enumerate()
                .filter_map(|(ix, (mark, number))| {
                    let (Mark::Pin { at, color }, Some(number)) = (mark, number) else {
                        return None;
                    };
                    let pin = div()
                        .flex_none()
                        .size(sizes.pin)
                        .rounded_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(tint(*color))
                        .border_2()
                        .border_color(if chosen == Some(ix) {
                            colors.accent
                        } else {
                            transparent_black()
                        })
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(colors.on_media)
                        .child(number.to_string());
                    Some(
                        div()
                            .absolute()
                            .left(relative(at.0))
                            .top(relative(at.1))
                            .size_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pin),
                    )
                });
        let look = Look {
            stroke,
            head,
            palette: colors.chart,
            halo: colors.accent,
        };
        let (painted, sketch) = (now.clone(), drawing.clone());
        let measured = drawing.clone();
        let plot = canvas(
            move |bounds, window, cx| {
                if measured.read(cx).bounds != bounds {
                    measured.update(cx, |drawing, cx| {
                        drawing.bounds = bounds;
                        cx.notify();
                    });
                    window.request_animation_frame();
                }
            },
            move |bounds, _, window, cx| {
                let points: Vec<(f32, f32)> = sketch
                    .read(cx)
                    .stroke
                    .iter()
                    .map(|p| share(bounds, bounds.origin + *p))
                    .collect();
                let drawn = Mark::made(tool, &points, color)
                    .filter(|mark| !matches!(mark, Mark::Pin { .. }));
                paint(&painted, drawn, chosen, look, bounds, window);
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let picture = framed(self.ratio, cx)
            .debug_selector(|| "image-annotator".into())
            .child(Image::new((self.id.clone(), "picture"), self.source).size_full())
            .child(plot)
            .children(pins);
        let area = div()
            .id(self.id.clone())
            .w_full()
            .border_1()
            .border_color(transparent_black())
            .child(picture);
        let area = match (editable, tool) {
            (false, _) => area,
            (true, Tool::Select) => {
                let (chooser, measured) = (board.clone(), drawing.clone());
                area.on_mouse_down(MouseButton::Left, move |event, _, cx| {
                    let bounds = measured.read(cx).bounds;
                    let size = (f32::from(bounds.size.width), f32::from(bounds.size.height));
                    let found = mark_at(
                        &chooser.marks.read(cx).value,
                        share(bounds, event.position),
                        size,
                        reach,
                    );
                    chooser.hand.update(cx, |hand, cx| {
                        hand.chosen = found;
                        cx.notify();
                    });
                })
            }
            (true, _) => {
                let (maker, measured) = (board.clone(), drawing.clone());
                pen(area, &drawing, move |stroke, window, cx| {
                    let bounds = measured.read(cx).bounds;
                    let points: Vec<(f32, f32)> = stroke
                        .iter()
                        .map(|p| share(bounds, bounds.origin + *p))
                        .collect();
                    if let Some(mark) = Mark::made(tool, &points, color) {
                        let mut next = maker.marks.read(cx).value.clone();
                        next.push(mark);
                        maker.commit(next, "added a mark", window, cx);
                    }
                })
            }
        };
        let keyed = board.clone();
        let area = area.when(editable, |area| {
            area.track_focus(&focus)
                .focus_ring(cx)
                .on_key_down(move |event, window, cx| {
                    let stroke = &event.keystroke;
                    let command = stroke.modifiers.platform || stroke.modifiers.control;
                    match (stroke.key.as_str(), command, stroke.modifiers.shift) {
                        ("z", true, back) => keyed.step(!back, window, cx),
                        ("delete" | "backspace", false, _) => keyed.remove(window, cx),
                        ("escape", false, _) => {
                            let chose = keyed.hand.update(cx, |hand, cx| {
                                cx.notify();
                                hand.chosen.take().is_some()
                            });
                            if !chose {
                                return;
                            }
                        }
                        (key, false, false) => {
                            match TOOLS.iter().find(|(_, each, ..)| *each == key) {
                                Some((tool, ..)) => keyed.pick(*tool, cx),
                                None => return,
                            }
                        }
                        _ => return,
                    }
                    cx.stop_propagation();
                })
        });
        div()
            .flex()
            .flex_col()
            .gap_3()
            .when(editable, |column| column.child(strip))
            .child(area)
    }
}
