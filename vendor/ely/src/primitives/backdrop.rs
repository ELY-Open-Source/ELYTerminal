use std::rc::Rc;

use gpui::{
    Animation, AnimationExt, AnyElement, App, ElementId, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Point, RenderOnce, Styled, Window, anchored, div, prelude::*,
};
use smallvec::SmallVec;

use super::raise;
use crate::{motion, theme::ActiveTheme};

type Dismiss = Rc<dyn Fn(&mut Window, &mut App)>;

/// Where a `Backdrop` holds its children.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Place {
    #[default]
    Center,
    Left,
    Right,
    Top,
    Bottom,
}

/// Full-window scrim. Children sit above it, centered or on an edge.
#[derive(IntoElement)]
pub struct Backdrop {
    id: ElementId,
    place: Place,
    on_dismiss: Option<Dismiss>,
    children: SmallVec<[AnyElement; 1]>,
}

impl Backdrop {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            place: Place::Center,
            on_dismiss: None,
            children: SmallVec::new(),
        }
    }

    /// Edges stretch children along that edge.
    pub fn place(mut self, place: Place) -> Self {
        self.place = place;
        self
    }

    /// Runs on a click outside the children.
    pub fn on_dismiss(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for Backdrop {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Backdrop {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let viewport = window.viewport_size();
        let fade = motion::duration(motion::BASE, cx);
        let scrim = div()
            .id(self.id.clone())
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .bg(cx.theme().colors.backdrop)
            .occlude()
            .when_some(self.on_dismiss, |scrim, dismiss| {
                scrim.on_mouse_down(MouseButton::Left, move |_, window, cx| dismiss(window, cx))
            })
            .with_animation(
                self.id.clone(),
                Animation::new(fade).with_easing(motion::ease_out_cubic),
                |scrim, t| scrim.opacity(t),
            );

        raise(
            (self.id, "raised"),
            anchored().position(Point::default()).child(
                div()
                    .relative()
                    .w(viewport.width)
                    .h(viewport.height)
                    .flex()
                    .map(|layer| match self.place {
                        Place::Center => layer.items_center().justify_center(),
                        Place::Left => layer.flex_row().justify_start(),
                        Place::Right => layer.flex_row().justify_end(),
                        Place::Top => layer.flex_col().justify_start(),
                        Place::Bottom => layer.flex_col().justify_end(),
                    })
                    .child(scrim)
                    .children(
                        self.children
                            .into_iter()
                            .map(|child| div().relative().occlude().child(child)),
                    ),
            ),
        )
    }
}
