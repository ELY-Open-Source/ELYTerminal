use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div, relative,
};

use super::work::Person;
use crate::{
    data_display::Avatar,
    theme::{ActiveTheme, AvatarSize, TextSize},
    typography::{Ellipsis, tabular},
};

/// Work someone has against what they can take, in one unit, as points.
#[derive(Clone, Debug, PartialEq)]
pub struct Load {
    pub person: Person,
    pub assigned: f32,
    pub capacity: f32,
}

impl Load {
    pub fn new(person: Person, assigned: f32, capacity: f32) -> Self {
        assert!(
            assigned.is_finite() && assigned >= 0.0,
            "{} has {assigned} assigned",
            person.name
        );
        assert!(
            capacity.is_finite() && capacity > 0.0,
            "{} can take {capacity}",
            person.name
        );
        Self {
            person,
            assigned,
            capacity,
        }
    }

    /// Past what they can take.
    pub fn over(&self) -> bool {
        self.assigned > self.capacity
    }

    /// The share of the track the bar fills, whole at capacity and past it.
    pub(crate) fn filled(&self) -> f32 {
        (self.assigned / self.capacity).min(1.0)
    }
}

/// Who carries how much: each person with a bar of their work against what they can take, red past it, and the two counts.
#[derive(IntoElement)]
pub struct WorkloadView {
    id: ElementId,
    loads: Vec<Load>,
    unit: SharedString,
}

impl WorkloadView {
    /// Loads counted in `unit`, as "pts" or "h".
    pub fn new(
        id: impl Into<ElementId>,
        loads: impl IntoIterator<Item = Load>,
        unit: impl Into<SharedString>,
    ) -> Self {
        let loads: Vec<Load> = loads.into_iter().collect();
        for (ix, load) in loads.iter().enumerate() {
            let twice = loads[..ix]
                .iter()
                .any(|other| other.person.key == load.person.key);
            assert!(!twice, "workload: {} twice", load.person.key);
        }
        Self {
            id: id.into(),
            loads,
            unit: unit.into(),
        }
    }
}

impl RenderOnce for WorkloadView {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let rows = self.loads.into_iter().map(|load| {
            let over = load.over();
            let fill = if over {
                colors.danger
            } else {
                colors.hue(0, "workload")
            };
            let share = load.filled();
            let count = format!("{:.0} / {:.0} {}", load.assigned, load.capacity, self.unit);
            let key = load.person.key.clone();
            div()
                .debug_selector(move || format!("load {key}"))
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .w(relative(0.4))
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            Avatar::new(
                                (self.id.clone(), format!("who-{}", load.person.key)),
                                load.person.name.clone(),
                            )
                            .size(AvatarSize::Xs),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(Ellipsis::new(load.person.name)),
                        ),
                )
                .child(
                    div()
                        .flex_1()
                        .h_1p5()
                        .relative()
                        .rounded_full()
                        .bg(colors.sunken)
                        .child(
                            div()
                                .absolute()
                                .left_0()
                                .top_0()
                                .bottom_0()
                                .w(relative(share))
                                .rounded_full()
                                .bg(fill),
                        ),
                )
                .child(
                    tabular(div())
                        .w_20()
                        .flex_none()
                        .flex()
                        .justify_end()
                        .text_size(theme.text_size(TextSize::Sm))
                        .text_color(if over { colors.danger } else { colors.fg_muted })
                        .child(count),
                )
        });
        div()
            .debug_selector(|| "workload".into())
            .flex()
            .flex_col()
            .gap_3()
            .children(rows.collect::<Vec<_>>())
    }
}

#[cfg(test)]
mod tests {
    use super::Load;
    use crate::project::Person;

    #[test]
    fn a_bar_fills_its_share_and_stops_at_capacity() {
        let ana = Person::new("ana", "Ana Lima");
        assert_eq!(Load::new(ana.clone(), 190.0, 168.0).filled(), 1.0);
        assert_eq!(Load::new(ana.clone(), 7.0, 10.0).filled(), 0.7);
        assert!(Load::new(ana, 11.0, 10.0).over());
    }
}
