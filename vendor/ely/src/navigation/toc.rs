use gpui::{
    Animation, AnimationExt, AnyElement, App, Bounds, ElementId, Entity, FontWeight,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, RenderOnce, ScrollHandle,
    SharedString, Styled, Window, canvas, div, point, prelude::*,
};
use smallvec::SmallVec;

use crate::{
    motion::{self, Axis, Marker, glide, measure_item, measure_origin, slide},
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
};

/// Where a document's sections sit: shared by its anchors and its table of contents.
pub struct Sections {
    scroll: ScrollHandle,
    spots: Vec<(SharedString, Bounds<Pixels>)>,
}

impl Sections {
    /// `scroll` tracks the container the document scrolls in.
    pub fn new(scroll: ScrollHandle) -> Self {
        Self {
            scroll,
            spots: Vec::new(),
        }
    }

    fn spot(&self, value: &SharedString) -> Option<Bounds<Pixels>> {
        self.spots
            .iter()
            .find(|(name, _)| name == value)
            .map(|(_, bounds)| *bounds)
    }
}

/// The last entry whose section starts above `line`: a quarter down the view, or its bottom at the end.
pub(crate) fn in_view(tops: &[Option<Pixels>], line: Pixels) -> usize {
    tops.iter()
        .rposition(|top| top.is_some_and(|top| top <= line))
        .unwrap_or(0)
}

/// Marks where a section starts, so a table of contents can follow it and bring it into view.
#[derive(IntoElement)]
pub struct Anchor {
    sections: Entity<Sections>,
    value: SharedString,
    children: SmallVec<[AnyElement; 2]>,
}

impl Anchor {
    pub fn new(sections: &Entity<Sections>, value: impl Into<SharedString>) -> Self {
        Self {
            sections: sections.clone(),
            value: value.into(),
            children: SmallVec::new(),
        }
    }
}

impl ParentElement for Anchor {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Anchor {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let (sections, value) = (self.sections, self.value);
        div().relative().children(self.children).child(
            canvas(
                move |bounds, window, cx| {
                    if sections.read(cx).spot(&value) == Some(bounds) {
                        return;
                    }
                    sections.update(cx, |sections, cx| {
                        match sections.spots.iter_mut().find(|(name, _)| *name == value) {
                            Some(spot) => spot.1 = bounds,
                            None => sections.spots.push((value.clone(), bounds)),
                        }
                        cx.notify();
                        window.request_animation_frame();
                    });
                },
                |_, _, _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
    }
}

/// A document's headings. A line marks the section in view; a click brings a section to the top.
#[derive(IntoElement)]
pub struct TableOfContents {
    id: ElementId,
    sections: Entity<Sections>,
    entries: Vec<(SharedString, SharedString, usize)>,
}

impl TableOfContents {
    pub fn new(id: impl Into<ElementId>, sections: &Entity<Sections>) -> Self {
        Self {
            id: id.into(),
            sections: sections.clone(),
            entries: Vec::new(),
        }
    }

    /// A heading: its anchor's value, its text, and how deep it sits, from 0.
    pub fn entry(
        mut self,
        value: impl Into<SharedString>,
        label: impl Into<SharedString>,
        depth: usize,
    ) -> Self {
        self.entries.push((value.into(), label.into(), depth));
        self
    }
}

impl RenderOnce for TableOfContents {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        assert!(
            !self.entries.is_empty(),
            "table of contents {:?} is empty",
            self.id
        );
        let (viewport, at_end, tops) = {
            let sections = self.sections.read(cx);
            let (scroll, viewport) = (&sections.scroll, sections.scroll.bounds());
            let room = scroll.max_offset().y;
            let at_end = room > Pixels::ZERO && -scroll.offset().y >= room;
            let tops: Vec<Option<Pixels>> = self
                .entries
                .iter()
                .map(|(value, _, _)| sections.spot(value).map(|spot| spot.top()))
                .collect();
            (viewport, at_end, tops)
        };
        let line = if at_end {
            viewport.bottom()
        } else {
            viewport.top() + viewport.size.height / 4.0
        };
        let here = in_view(&tops, line);
        log::debug!(
            "table of contents {:?}: {here} in view, tops {tops:?} in {viewport:?}",
            self.id
        );
        let values: Vec<SharedString> = self
            .entries
            .iter()
            .map(|(value, ..)| value.clone())
            .collect();
        let (state, marker) = slide(
            (self.id.clone(), "slide"),
            &values,
            &values[here],
            window,
            cx,
        );
        let theme = cx.theme();
        let colors = &theme.colors;
        let duration = motion::duration(motion::BASE, cx);
        let line = marker.map(
            |Marker {
                 from,
                 to,
                 generation,
             }| {
                div()
                    .absolute()
                    .left_0()
                    .w(theme.tab_indicator())
                    .rounded_full()
                    .bg(colors.accent)
                    .with_animation(
                        ("toc-line", generation),
                        Animation::new(duration),
                        move |line, t| {
                            let (top, height) = glide(from, to, t);
                            line.top(top).h(height)
                        },
                    )
            },
        );
        let rows = self
            .entries
            .into_iter()
            .enumerate()
            .map(|(ix, (value, label, depth))| {
                let sections = self.sections.clone();
                let on = ix == here;
                let row = div()
                    .id(("toc", ix))
                    .relative()
                    .flex()
                    .items_center()
                    .min_h(theme.control_height(ControlSize::Sm))
                    .pr_2()
                    .rounded(theme.radius(Radius::Sm))
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(if on { colors.fg } else { colors.fg_muted })
                    .when(on, |row| row.font_weight(FontWeight::MEDIUM))
                    .cursor_pointer()
                    .hover(|style| style.text_color(colors.fg))
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        let (scroll, spot) = {
                            let sections = sections.read(cx);
                            (sections.scroll.clone(), sections.spot(&value))
                        };
                        let Some(spot) = spot else {
                            log::error!("table of contents: no anchor {value} has drawn");
                            return;
                        };
                        let offset = scroll.offset();
                        let lift = spot.top() - scroll.bounds().top();
                        scroll.set_offset(point(offset.x, offset.y - lift));
                        log::info!(
                            "table of contents: to {value}, lift {lift:?} from {:?} in {:?}",
                            offset.y,
                            scroll.bounds()
                        );
                        window.refresh();
                    })
                    .child(label)
                    .child(measure_item(state.clone(), ix, Axis::Vertical));
                match depth {
                    0 => row.pl_3(),
                    1 => row.pl_6(),
                    _ => row.pl_8(),
                }
            });
        div()
            .id(self.id)
            .relative()
            .flex()
            .flex_col()
            .gap_0p5()
            .border_l_1()
            .border_color(colors.border)
            .child(measure_origin(state.clone(), Axis::Vertical))
            .children(line)
            .children(rows)
    }
}

#[cfg(test)]
mod tests {
    use gpui::px;

    use super::in_view;

    #[test]
    fn the_section_in_view_is_the_last_one_started() {
        let tops = [Some(px(-400.0)), Some(px(-20.0)), Some(px(300.0)), None];
        assert_eq!(in_view(&tops, px(100.0)), 1);
        assert_eq!(in_view(&tops, px(-500.0)), 0);
        assert_eq!(in_view(&tops, px(900.0)), 2);
    }
}
