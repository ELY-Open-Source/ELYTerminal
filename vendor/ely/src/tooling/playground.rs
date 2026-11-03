use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled, Window, div,
};

use crate::{
    forms::{Choice, Select, Slider, Switch},
    layout::seeded::use_seeded,
    theme::{ActiveTheme, Radius, TextSize},
    typography::{format, tabular},
};

type Preview = Rc<dyn Fn(&Settings, &mut Window, &mut App) -> AnyElement>;

/// A property a playground edits: a switch, a choice among names, or a number in a range, each by its name.
#[derive(Clone, Debug, PartialEq)]
pub enum Knob {
    Toggle(SharedString, bool),
    Choice(SharedString, Vec<SharedString>, usize),
    Number(SharedString, f64, (f64, f64), f64),
}

impl Knob {
    fn name(&self) -> &SharedString {
        match self {
            Knob::Toggle(name, _) | Knob::Choice(name, _, _) | Knob::Number(name, _, _, _) => name,
        }
    }

    /// The setting it starts on.
    fn first(&self) -> Setting {
        match self {
            Knob::Toggle(_, on) => Setting::On(*on),
            Knob::Choice(name, options, at) => Setting::Picked(
                options
                    .get(*at)
                    .unwrap_or_else(|| panic!("playground knob {name}: no option {at}"))
                    .clone(),
            ),
            Knob::Number(_, value, _, _) => Setting::Number(*value),
        }
    }
}

/// Where a knob stands.
#[derive(Clone, Debug, PartialEq)]
pub enum Setting {
    On(bool),
    Picked(SharedString),
    Number(f64),
}

/// The knobs' settings by name; asking for a name the knobs lack, or in another kind, fails.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings(Vec<(SharedString, Setting)>);

impl Settings {
    fn get(&self, name: &str) -> &Setting {
        self.0
            .iter()
            .find(|(each, _)| each.as_ref() == name)
            .map(|(_, setting)| setting)
            .unwrap_or_else(|| panic!("playground: no knob named {name}"))
    }

    pub fn on(&self, name: &str) -> bool {
        match self.get(name) {
            Setting::On(on) => *on,
            other => panic!("playground knob {name} is {other:?}, not a switch"),
        }
    }

    pub fn picked(&self, name: &str) -> &SharedString {
        match self.get(name) {
            Setting::Picked(option) => option,
            other => panic!("playground knob {name} is {other:?}, not a choice"),
        }
    }

    pub fn number(&self, name: &str) -> f64 {
        match self.get(name) {
            Setting::Number(value) => *value,
            other => panic!("playground knob {name} is {other:?}, not a number"),
        }
    }

    /// Sets the knob named `name`; a knob painted before the owner's knobs changed sets nothing.
    fn set(&mut self, name: &str, setting: Setting) {
        let Some(slot) = self.0.iter_mut().find(|(each, _)| each.as_ref() == name) else {
            log::error!("playground: no knob named {name} now; {setting:?} is dropped");
            return;
        };
        log::info!("playground: {name} is {setting:?}");
        slot.1 = setting;
    }
}

/// A component beside the knobs that shape it: each knob edits one property as the preview redraws with the settings. The knobs are the owner's: when any changes, every setting starts over from them. Narrow, the knobs drop below.
#[derive(IntoElement)]
pub struct Playground {
    id: ElementId,
    knobs: Vec<Knob>,
    preview: Preview,
}

impl Playground {
    /// Fails on no knobs, two knobs of one name, or a number that starts outside its range.
    pub fn new(
        id: impl Into<ElementId>,
        knobs: impl IntoIterator<Item = Knob>,
        preview: impl Fn(&Settings, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        let (id, knobs): (ElementId, Vec<Knob>) = (id.into(), knobs.into_iter().collect());
        assert!(!knobs.is_empty(), "playground {id:?} has no knobs");
        for (ix, knob) in knobs.iter().enumerate() {
            assert!(
                knobs[..ix]
                    .iter()
                    .all(|before| before.name() != knob.name()),
                "playground {id:?} has two knobs named {}",
                knob.name()
            );
            if let Knob::Number(name, value, (least, most), _) = knob {
                assert!(
                    (*least..=*most).contains(value),
                    "playground {id:?}: {name} starts at {value}, outside {least}..={most}"
                );
            }
        }
        Self {
            id,
            knobs,
            preview: Rc::new(preview),
        }
    }
}

impl RenderOnce for Playground {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let firsts = Settings(
            self.knobs
                .iter()
                .map(|knob| (knob.name().clone(), knob.first()))
                .collect(),
        );
        // The knobs join the seed, so a changed option list or range starts over too.
        let settings = use_seeded(
            (id.clone(), "settings"),
            (self.knobs.clone(), firsts),
            window,
            cx,
        );
        let now = settings.read(cx).value.1.clone();
        let preview = (self.preview)(&now, window, cx);
        let theme = cx.theme();
        let colors = &theme.colors;
        let rows = self.knobs.iter().map(|knob| {
            let (name, set) = (knob.name().clone(), settings.clone());
            let key = (id.clone(), format!("knob-{name}"));
            let control = match knob {
                Knob::Toggle(..) => Switch::new(key, now.on(&name))
                    .on_change(move |on, _, cx| {
                        set.update(cx, |settings, cx| {
                            settings.value.1.set(&name, Setting::On(on));
                            cx.notify();
                        })
                    })
                    .into_any_element(),
                Knob::Choice(_, options, _) => Select::new(
                    key,
                    options
                        .iter()
                        .map(|option| Choice::new(option.clone(), option.clone())),
                )
                .selected(now.picked(&name).clone())
                .on_change(move |option, _, cx| {
                    set.update(cx, |settings, cx| {
                        settings.value.1.set(&name, Setting::Picked(option.clone()));
                        cx.notify();
                    })
                })
                .into_any_element(),
                Knob::Number(_, _, (least, most), step) => div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div().flex_1().min_w_0().child(
                            Slider::new(key, now.number(&name))
                                .range(*least, *most)
                                .step(*step)
                                .on_change(move |value, _, cx| {
                                    set.update(cx, |settings, cx| {
                                        settings.value.1.set(&name, Setting::Number(value));
                                        cx.notify();
                                    })
                                }),
                        ),
                    )
                    .child({
                        let shown = format::number(
                            now.number(knob.name()),
                            format::decimals(*step),
                            format::Separators::EN,
                        );
                        tabular(div())
                            .debug_selector(|| format!("knob-readout-{}-{shown}", knob.name()))
                            .flex_none()
                            .text_color(colors.fg_muted)
                            .child(shown.clone())
                    })
                    .into_any_element(),
            };
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(theme.text_size(TextSize::Xs))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(colors.fg_muted)
                        .child(knob.name().clone()),
                )
                .child(control)
        });
        div()
            .flex()
            .flex_wrap()
            .gap_4()
            .child(
                div()
                    .flex_1()
                    .min_w(theme.label_width())
                    .flex()
                    .items_center()
                    .justify_center()
                    .p_6()
                    .rounded(theme.radius(Radius::Lg))
                    .border_1()
                    .border_color(colors.border)
                    .bg(colors.sunken)
                    .child(preview),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(theme.label_width())
                    .flex()
                    .flex_col()
                    .gap_3()
                    .children(rows),
            )
    }
}

#[cfg(test)]
mod tests {
    use gpui::IntoElement;

    use super::{Knob, Setting, Settings};

    #[test]
    fn settings_answer_by_name_and_kind() {
        let knobs = [
            Knob::Toggle("disabled".into(), true),
            Knob::Choice("size".into(), vec!["Sm".into(), "Md".into()], 1),
            Knob::Number("share".into(), 0.4, (0.0, 1.0), 0.1),
        ];
        let mut settings = Settings(
            knobs
                .iter()
                .map(|knob| (knob.name().clone(), knob.first()))
                .collect(),
        );
        assert!(settings.on("disabled"));
        assert_eq!(settings.picked("size").as_ref(), "Md");
        settings.set("share", Setting::Number(0.7));
        assert_eq!(settings.number("share"), 0.7);
        let before = settings.clone();
        settings.set("gone", Setting::On(true));
        assert_eq!(settings, before, "a knob no longer there sets nothing");
    }

    #[test]
    #[should_panic(expected = "has no knobs")]
    fn a_playground_without_knobs_fails() {
        super::Playground::new("p", [], |_, _, _| gpui::div().into_any_element());
    }

    #[test]
    #[should_panic(expected = "has two knobs named size")]
    fn twin_knob_names_fail() {
        let size = || Knob::Toggle("size".into(), true);
        super::Playground::new("p", [size(), size()], |_, _, _| {
            gpui::div().into_any_element()
        });
    }

    #[test]
    #[should_panic(expected = "share starts at 5, outside 0..=1")]
    fn a_number_outside_its_range_fails() {
        let share = Knob::Number("share".into(), 5.0, (0.0, 1.0), 0.1);
        super::Playground::new("p", [share], |_, _, _| gpui::div().into_any_element());
    }

    #[test]
    #[should_panic(expected = "no knob named color")]
    fn a_name_the_knobs_lack_fails() {
        Settings(vec![("size".into(), Setting::Picked("Md".into()))]).on("color");
    }

    #[test]
    #[should_panic(expected = "playground knob size is Picked")]
    fn a_setting_asked_in_another_kind_fails() {
        let settings = Settings(vec![("size".into(), Setting::Picked("Md".into()))]);
        settings.on("size");
    }
}
