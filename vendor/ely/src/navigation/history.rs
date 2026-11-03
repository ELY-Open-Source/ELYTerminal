use std::rc::Rc;

use gpui::{App, ElementId, IntoElement, ParentElement, RenderOnce, Styled, Window, div};

use super::overflow::list_button;
use crate::{
    buttons::{ButtonVariant, IconButton},
    forms::Choice,
    primitives::IconName,
    theme::ControlSize,
};

type OnGo = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// Back and forward through visited places, and a list of every one of them, newest first.
#[derive(IntoElement)]
pub struct BackForwardNavigation {
    id: ElementId,
    places: Vec<Choice>,
    current: usize,
    on_go: Option<OnGo>,
}

impl BackForwardNavigation {
    /// `places` run oldest first; `current` is where you stand among them.
    pub fn new(
        id: impl Into<ElementId>,
        places: impl IntoIterator<Item = Choice>,
        current: usize,
    ) -> Self {
        let places: Vec<Choice> = places.into_iter().collect();
        assert!(!places.is_empty(), "history has no places");
        Self {
            id: id.into(),
            places,
            current,
            on_go: None,
        }
    }

    /// Runs with the index of the place to go to.
    pub fn on_go(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_go = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for BackForwardNavigation {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (current, last) = (self.current, self.places.len() - 1);
        let valid = current <= last;
        if !valid {
            log::error!(
                "history {:?}: place {current} of {}; none marked",
                self.id,
                last + 1
            );
        }
        let go: OnGo = {
            let (id, on_go) = (self.id.clone(), self.on_go);
            Rc::new(move |to, window, cx| {
                log::info!("history {id:?}: go to {to}");
                if let Some(on_go) = &on_go {
                    on_go(to, window, cx);
                }
            })
        };
        let rows: Vec<Choice> = self
            .places
            .iter()
            .enumerate()
            .rev()
            .map(|(ix, place)| {
                let row = Choice::new(ix.to_string(), place.label.clone());
                match place.icon {
                    Some(icon) => row.icon(icon),
                    None => row,
                }
            })
            .collect();
        let (back, ahead, listed) = (go.clone(), go.clone(), go);
        let list = list_button(
            &(self.id.clone(), "history").into(),
            IconName::History,
            Rc::new(rows),
            Some(&current.to_string().into()),
            move |value, window, cx| {
                let to = value.parse().expect("history rows carry their index");
                listed(to, window, cx);
            },
            window,
            cx,
        );
        div()
            .flex()
            .items_center()
            .gap_0p5()
            .child(
                IconButton::new((self.id.clone(), "back"), IconName::ArrowLeft)
                    .size(ControlSize::Sm)
                    .variant(ButtonVariant::Ghost)
                    .tooltip("Back")
                    .disabled(!valid || current == 0)
                    .on_click(move |_, window, cx| back(current - 1, window, cx)),
            )
            .child(
                IconButton::new((self.id, "forward"), IconName::ArrowRight)
                    .size(ControlSize::Sm)
                    .variant(ButtonVariant::Ghost)
                    .tooltip("Forward")
                    .disabled(!valid || current == last)
                    .on_click(move |_, window, cx| ahead(current + 1, window, cx)),
            )
            .child(list)
    }
}
