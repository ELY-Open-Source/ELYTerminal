use std::rc::Rc;

use gpui::{
    App, ElementId, Entity, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use crate::{
    buttons::{ButtonVariant, IconButton, SegmentedControl},
    forms::{Choice, JsonInput, NumberInput, Select, Switch, TextInput},
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
    typography::format::decimals,
};

/// A setting's value and the form it takes.
#[derive(Clone, Debug, PartialEq)]
pub enum SettingValue {
    Bool(bool),
    Number {
        value: f64,
        min: f64,
        max: f64,
        step: f64,
    },
    Choice {
        value: SharedString,
        choices: Vec<SharedString>,
    },
}

impl SettingValue {
    fn json(&self) -> serde_json::Value {
        match self {
            SettingValue::Bool(on) => (*on).into(),
            SettingValue::Number { value, .. } => serde_json::Number::from_f64(*value)
                .map(serde_json::Value::Number)
                .expect("a setting's number is finite"),
            SettingValue::Choice { value, .. } => value.to_string().into(),
        }
    }
}

/// One setting: its dotted key, title, what it does, its value and the value it starts from.
#[derive(Clone, Debug, PartialEq)]
pub struct Setting {
    pub key: SharedString,
    pub title: SharedString,
    pub description: SharedString,
    pub value: SettingValue,
    pub default: SettingValue,
}

impl Setting {
    pub fn modified(&self) -> bool {
        self.value != self.default
    }
}

/// The settings changed from their defaults, as a settings file holds them.
pub fn settings_json(groups: &[(SharedString, Vec<Setting>)]) -> String {
    let map: serde_json::Map<String, serde_json::Value> = groups
        .iter()
        .flat_map(|(_, settings)| settings)
        .filter(|setting| setting.modified())
        .map(|setting| (setting.key.to_string(), setting.value.json()))
        .collect();
    serde_json::to_string_pretty(&serde_json::Value::Object(map)).expect("settings serialize")
}

type OnChange = Rc<dyn Fn(&SharedString, SettingValue, &mut Window, &mut App)>;
type OnIndex = Rc<dyn Fn(usize, &mut Window, &mut App)>;
type OnFlag = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// Settings two ways: as rows of controls by section, each changed one marked with a way back, or as the JSON a settings file holds.
#[derive(IntoElement)]
pub struct SettingsEditor {
    id: ElementId,
    groups: Vec<(SharedString, Vec<Setting>)>,
    section: usize,
    json: Option<Entity<TextInput>>,
    on_change: Option<OnChange>,
    on_section: Option<OnIndex>,
    on_json: Option<OnFlag>,
}

impl SettingsEditor {
    pub fn new(
        id: impl Into<ElementId>,
        groups: impl IntoIterator<Item = (impl Into<SharedString>, Vec<Setting>)>,
    ) -> Self {
        let groups: Vec<(SharedString, Vec<Setting>)> = groups
            .into_iter()
            .map(|(title, settings)| (title.into(), settings))
            .collect();
        assert!(!groups.is_empty(), "settings come in at least one section");
        Self {
            id: id.into(),
            groups,
            section: 0,
            json: None,
            on_change: None,
            on_section: None,
            on_json: None,
        }
    }

    pub fn section(mut self, section: usize) -> Self {
        self.section = section;
        self
    }

    /// Shows the JSON, held in `field`; seed it with `settings_json`.
    pub fn json(mut self, field: &Entity<TextInput>) -> Self {
        self.json = Some(field.clone());
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(&SharedString, SettingValue, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    pub fn on_section(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_section = Some(Rc::new(handler));
        self
    }

    /// Gets whether the JSON view is wanted.
    pub fn on_json(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_json = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SettingsEditor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let mode = SegmentedControl::new(
            (self.id.clone(), "mode"),
            if self.json.is_some() { "json" } else { "ui" },
        )
        .size(ControlSize::Sm)
        .segment("ui", "Settings", None)
        .segment("json", "JSON", None);
        let mode = match self.on_json.clone() {
            Some(on_json) => {
                mode.on_change(move |key, window, cx| on_json(key.as_ref() == "json", window, cx))
            }
            None => mode,
        };
        let header = div().flex().items_center().justify_between().child(
            div()
                .text_size(theme.text_size(TextSize::Md))
                .font_weight(FontWeight::SEMIBOLD)
                .child("Settings"),
        );
        let header = header.child(mode);
        if let Some(field) = self.json {
            return div()
                .flex()
                .flex_col()
                .gap_3()
                .child(header)
                .child(JsonInput::new(&field))
                .into_any_element();
        }
        let section = self.section.min(self.groups.len() - 1);
        let nav = self
            .groups
            .iter()
            .enumerate()
            .map(|(ix, (title, settings))| {
                let pick = self.on_section.clone();
                let changed = settings.iter().filter(|setting| setting.modified()).count();
                div()
                    .id((self.id.clone(), format!("section-{ix}")))
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_2()
                    .py_1()
                    .rounded(theme.radius(Radius::Sm))
                    .cursor_pointer()
                    .when(ix == section, |row| {
                        row.bg(colors.active).text_color(colors.fg)
                    })
                    .when(ix != section, |row| {
                        row.text_color(colors.fg_muted)
                            .hover(|row| row.bg(colors.hover))
                    })
                    .when_some(pick, |row, pick| {
                        row.on_click(move |_, window, cx| pick(ix, window, cx))
                    })
                    .child(title.clone())
                    .when(changed > 0, |row| {
                        row.child(
                            div()
                                .text_size(theme.text_size(TextSize::Xs))
                                .text_color(colors.accent)
                                .child(changed.to_string()),
                        )
                    })
            });
        let rows = self.groups[section]
            .1
            .iter()
            .enumerate()
            .map(|(ix, setting)| {
                let id = ElementId::from((self.id.clone(), format!("{section}-{ix}")));
                let (change, key) = (self.on_change.clone(), setting.key.clone());
                let tell = move |value: SettingValue, window: &mut Window, cx: &mut App| {
                    log::info!("settings: {key} = {value:?}");
                    if let Some(change) = &change {
                        change(&key, value, window, cx);
                    }
                };
                let tell = Rc::new(tell);
                let control = match &setting.value {
                    SettingValue::Bool(on) => {
                        let tell = tell.clone();
                        Switch::new(id.clone(), *on)
                            .on_change(move |on, window, cx| {
                                tell(SettingValue::Bool(on), window, cx)
                            })
                            .into_any_element()
                    }
                    SettingValue::Number {
                        value,
                        min,
                        max,
                        step,
                    } => {
                        let (tell, min, max, step) = (tell.clone(), *min, *max, *step);
                        div()
                            .w(theme.label_width())
                            .child(
                                NumberInput::new(id.clone(), *value)
                                    .range(min, max)
                                    .step(step)
                                    .precision(places(*value, step))
                                    .size(ControlSize::Sm)
                                    .on_change(move |value, window, cx| {
                                        tell(
                                            SettingValue::Number {
                                                value,
                                                min,
                                                max,
                                                step,
                                            },
                                            window,
                                            cx,
                                        )
                                    }),
                            )
                            .into_any_element()
                    }
                    SettingValue::Choice { value, choices } => {
                        let (tell, all) = (tell.clone(), choices.clone());
                        div()
                            .w(theme.label_width() * 1.5)
                            .child(
                                Select::new(
                                    id.clone(),
                                    choices
                                        .iter()
                                        .map(|choice| Choice::new(choice.clone(), choice.clone())),
                                )
                                .selected(value.clone())
                                .size(ControlSize::Sm)
                                .on_change(
                                    move |picked, window, cx| {
                                        let value = SettingValue::Choice {
                                            value: picked.clone(),
                                            choices: all.clone(),
                                        };
                                        tell(value, window, cx)
                                    },
                                ),
                            )
                            .into_any_element()
                    }
                };
                let reset = setting.modified().then(|| {
                    let (tell, default) = (tell.clone(), setting.default.clone());
                    IconButton::new(
                        (self.id.clone(), format!("reset-{section}-{ix}")),
                        IconName::RotateCcw,
                    )
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .tooltip("Reset to default")
                    .on_click(move |_, window, cx| tell(default.clone(), window, cx))
                });
                div()
                    .flex()
                    .items_start()
                    .gap_3()
                    .py_3()
                    .pl_3()
                    .border_l_2()
                    .border_color(if setting.modified() {
                        colors.accent
                    } else {
                        gpui::transparent_black()
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(
                                div()
                                    .flex()
                                    .items_baseline()
                                    .gap_2()
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(colors.fg)
                                            .child(setting.title.clone()),
                                    )
                                    .child(
                                        div()
                                            .font_family(theme.mono_family.clone())
                                            .text_size(theme.text_size(TextSize::Xs))
                                            .text_color(colors.fg_subtle)
                                            .child(setting.key.clone()),
                                    ),
                            )
                            .child(
                                div()
                                    .text_color(colors.fg_muted)
                                    .child(setting.description.clone()),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .children(reset)
                            .child(control),
                    )
            });
        div()
            .flex()
            .flex_col()
            .gap_3()
            .text_size(theme.text_size(TextSize::Sm))
            .child(header)
            .child(
                div()
                    .flex()
                    .gap_6()
                    .child(
                        div()
                            .w(theme.label_width() * 1.25)
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .children(nav),
                    )
                    .child(div().flex_1().min_w_0().flex().flex_col().children(rows)),
            )
            .into_any_element()
    }
}

/// Places a number setting shows: enough for its step and its value.
fn places(value: f64, step: f64) -> usize {
    let own = if value == 0.0 {
        0
    } else {
        decimals(value.abs())
    };
    decimals(step).max(own)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_show_the_places_their_value_and_step_need() {
        assert_eq!(places(0.5, 0.1), 1);
        assert_eq!(places(14.0, 1.0), 0);
        assert_eq!(places(0.25, 0.05), 2);
        assert_eq!(places(0.0, 0.5), 1);
    }

    #[test]
    fn json_holds_only_what_changed() {
        let groups = vec![(
            SharedString::from("Editor"),
            vec![
                Setting {
                    key: "editor.fontSize".into(),
                    title: "Font size".into(),
                    description: "".into(),
                    value: SettingValue::Number {
                        value: 15.0,
                        min: 8.0,
                        max: 32.0,
                        step: 1.0,
                    },
                    default: SettingValue::Number {
                        value: 14.0,
                        min: 8.0,
                        max: 32.0,
                        step: 1.0,
                    },
                },
                Setting {
                    key: "editor.minimap".into(),
                    title: "Minimap".into(),
                    description: "".into(),
                    value: SettingValue::Bool(true),
                    default: SettingValue::Bool(true),
                },
            ],
        )];
        let json: serde_json::Value = serde_json::from_str(&settings_json(&groups)).expect("json");
        assert_eq!(json, serde_json::json!({ "editor.fontSize": 15.0 }));
    }
}
