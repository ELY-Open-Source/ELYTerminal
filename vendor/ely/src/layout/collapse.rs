use gpui::{
    Animation, AnimationExt, AnyElement, App, ElementId, Entity, FontWeight, InteractiveElement,
    IntoElement, ParentElement, Pixels, RenderOnce, SharedString, StatefulInteractiveElement,
    Styled, Window, canvas, div,
};
use smallvec::SmallVec;
use web_time::Instant;

use crate::{
    motion,
    primitives::{Disclosure, FocusRing},
    theme::{ActiveTheme, IconSize, TextSize},
};

struct Fold {
    open: bool,
    height: Pixels,
    generation: u64,
    changed: Instant,
}

/// Region that opens and closes by animating its measured height.
#[derive(IntoElement)]
pub struct Collapsible {
    id: ElementId,
    open: bool,
    body: SmallVec<[AnyElement; 2]>,
}

impl Collapsible {
    pub fn new(id: impl Into<ElementId>, open: bool) -> Self {
        Self {
            id: id.into(),
            open,
            body: SmallVec::new(),
        }
    }
}

impl ParentElement for Collapsible {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

fn measured(state: Entity<Fold>) -> impl IntoElement {
    canvas(
        move |bounds, window, cx| {
            if state.read(cx).height != bounds.size.height {
                state.update(cx, |fold, cx| {
                    fold.height = bounds.size.height;
                    cx.notify();
                    window.request_animation_frame();
                });
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

impl RenderOnce for Collapsible {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open = self.open;
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| Fold {
            open,
            height: Pixels::ZERO,
            generation: 0,
            changed: Instant::now(),
        });
        if state.read(cx).open != open {
            state.update(cx, |fold, _| {
                fold.open = open;
                fold.generation += 1;
                fold.changed = Instant::now();
            });
        }
        let (height, generation, changed) = {
            let fold = state.read(cx);
            (fold.height, fold.generation, fold.changed)
        };
        let duration = motion::duration(motion::BASE, cx);
        let settled = generation == 0 || changed.elapsed() >= duration;
        let inner = div().relative().children(self.body).child(measured(state));
        let frame = div().id(self.id).overflow_hidden();
        match (settled, open) {
            (true, true) => frame.child(inner).into_any_element(),
            (true, false) => frame.into_any_element(),
            (false, _) => frame
                .child(inner)
                .with_animation(
                    ("fold", generation),
                    Animation::new(duration).with_easing(motion::ease_in_out_cubic),
                    move |frame, t| frame.h(height * if open { t } else { 1.0 - t }),
                )
                .into_any_element(),
        }
    }
}

/// One titled, foldable row of an `Accordion`.
pub struct AccordionItem {
    title: SharedString,
    body: SmallVec<[AnyElement; 2]>,
}

impl AccordionItem {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            body: SmallVec::new(),
        }
    }
}

impl ParentElement for AccordionItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

#[derive(Default)]
struct Folds {
    open: Vec<bool>,
}

/// Stacked items that fold; one open at a time unless `multiple`.
#[derive(IntoElement)]
pub struct Accordion {
    id: ElementId,
    items: Vec<AccordionItem>,
    multiple: bool,
    first_open: bool,
}

impl Accordion {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
            multiple: false,
            first_open: false,
        }
    }

    pub fn item(mut self, item: AccordionItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn multiple(mut self) -> Self {
        self.multiple = true;
        self
    }

    /// Starts with the first item open.
    pub fn first_open(mut self) -> Self {
        self.first_open = true;
        self
    }
}

impl RenderOnce for Accordion {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let count = self.items.len();
        let first_open = self.first_open;
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| Folds {
            open: (0..count).map(|ix| first_open && ix == 0).collect(),
        });
        if state.read(cx).open.len() != count {
            state.update(cx, |folds, _| folds.open.resize(count, false));
        }
        let theme = cx.theme();
        let (border, muted) = (theme.colors.border, theme.colors.fg_muted);
        let title_size = theme.text_size(TextSize::Base);
        let multiple = self.multiple;
        let id = self.id.clone();
        let folds = state.read(cx);
        let rows: Vec<_> = self
            .items
            .into_iter()
            .enumerate()
            .map(|(ix, item)| (ix, item, folds.open[ix]))
            .collect();
        div()
            .id(self.id)
            .flex()
            .flex_col()
            .border_t_1()
            .border_color(border)
            .children(rows.into_iter().map(|(ix, item, open)| {
                let toggle = state.clone();
                let chevron = Disclosure::new((id.clone(), format!("chevron-{ix}")), open)
                    .size(IconSize::Sm)
                    .color(muted);
                div()
                    .border_b_1()
                    .border_color(border)
                    .child(
                        div()
                            .id(ix)
                            .flex()
                            .items_center()
                            .justify_between()
                            .py_3()
                            .px_1()
                            .border_1()
                            .border_color(gpui::transparent_black())
                            .cursor_pointer()
                            .tab_index(0)
                            .focus_ring(cx)
                            .text_size(title_size)
                            .font_weight(FontWeight::MEDIUM)
                            .child(item.title)
                            .child(chevron)
                            .on_click(move |_, _, cx| {
                                toggle.update(cx, |folds, cx| {
                                    let now = !folds.open[ix];
                                    for (other, open) in folds.open.iter_mut().enumerate() {
                                        let next = if other == ix {
                                            now
                                        } else {
                                            *open && (multiple || !now)
                                        };
                                        *open = next;
                                    }
                                    cx.notify();
                                })
                            }),
                    )
                    .child(
                        Collapsible::new(("body", ix), open)
                            .child(div().pb_4().px_1().text_color(muted).children(item.body)),
                    )
            }))
    }
}
