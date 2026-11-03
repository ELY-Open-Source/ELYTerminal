use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, Keystroke, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use crate::{
    data_display::{Badge, Tone},
    forms::HotkeyInput,
    navigation::fuzzy,
    primitives::{Icon, IconName, Tooltip},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::KbdCombo,
};

/// Where a binding comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeySource {
    Default,
    User,
}

/// A command and the keys that run it, in gpui key syntax, when a context holds.
#[derive(Clone, Debug, PartialEq)]
pub struct Keybinding {
    pub command: SharedString,
    pub title: SharedString,
    pub keys: Option<SharedString>,
    pub when: Option<SharedString>,
    pub source: KeySource,
}

/// Commands sharing keys in the same context: for each binding, the others its keys also run.
pub fn conflicts(bindings: &[Keybinding], binding: &Keybinding) -> Vec<SharedString> {
    let Some(keys) = &binding.keys else {
        return Vec::new();
    };
    bindings
        .iter()
        .filter(|other| {
            other.command != binding.command
                && other.keys.as_ref() == Some(keys)
                && other.when == binding.when
        })
        .map(|other| other.title.clone())
        .collect()
}

type OnCommand = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
type OnBind = Rc<dyn Fn(&SharedString, Option<Keystroke>, &mut Window, &mut App)>;

/// Every command with its keys, context and source; a query narrows them, keys that clash are marked, and a double press records new keys.
#[derive(IntoElement)]
pub struct KeybindingsEditor {
    id: ElementId,
    bindings: Vec<Keybinding>,
    query: SharedString,
    recording: Option<SharedString>,
    on_record: Option<OnCommand>,
    on_bind: Option<OnBind>,
}

impl KeybindingsEditor {
    pub fn new(id: impl Into<ElementId>, bindings: impl IntoIterator<Item = Keybinding>) -> Self {
        Self {
            id: id.into(),
            bindings: bindings.into_iter().collect(),
            query: SharedString::default(),
            recording: None,
            on_record: None,
            on_bind: None,
        }
    }

    /// Narrows to commands whose title, command or keys fit.
    pub fn query(mut self, query: impl Into<SharedString>) -> Self {
        self.query = query.into();
        self
    }

    /// The command whose keys are being recorded.
    pub fn recording(mut self, command: Option<SharedString>) -> Self {
        self.recording = command;
        self
    }

    /// Gets the command a double press chose to record.
    pub fn on_record(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_record = Some(Rc::new(handler));
        self
    }

    /// Gets the keys recorded for a command, or none when cleared.
    pub fn on_bind(
        mut self,
        handler: impl Fn(&SharedString, Option<Keystroke>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_bind = Some(Rc::new(handler));
        self
    }
}

/// The four columns every row shares.
fn columns<E: Styled>(row: E) -> E {
    row.flex().items_center().gap_3().px_2()
}

impl RenderOnce for KeybindingsEditor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let query = self.query.trim().to_string();
        let fits = |binding: &Keybinding| {
            query.is_empty()
                || [
                    Some(&binding.title),
                    Some(&binding.command),
                    binding.keys.as_ref(),
                ]
                .into_iter()
                .flatten()
                .any(|text| fuzzy(&query, text).is_some())
        };
        let widths = [2.25, 1.5, 1.5, 0.75].map(|share| theme.label_width() * share);
        let head = columns(div())
            .py_1()
            .text_size(theme.text_size(TextSize::Xs))
            .text_color(colors.fg_subtle)
            .border_b_1()
            .border_color(colors.border)
            .child(div().w(widths[0]).child("Command"))
            .child(div().w(widths[1]).child("Keybinding"))
            .child(div().w(widths[2]).child("When"))
            .child(div().w(widths[3]).child("Source"));
        let rows = self
            .bindings
            .iter()
            .enumerate()
            .filter(|(_, binding)| fits(binding))
            .map(|(ix, binding)| {
                let clashes = conflicts(&self.bindings, binding);
                let recording = self.recording.as_ref() == Some(&binding.command);
                let keys = if recording {
                    let (bind, command) = (self.on_bind.clone(), binding.command.clone());
                    HotkeyInput::new((self.id.clone(), format!("keys-{ix}")), None)
                        .on_change(move |stroke, window, cx| {
                            log::info!("keybindings: {command} = {stroke:?}");
                            if let Some(bind) = &bind {
                                bind(&command, stroke, window, cx);
                            }
                        })
                        .into_any_element()
                } else {
                    match &binding.keys {
                        Some(keys) => div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .child(KbdCombo::new(keys))
                            .when(!clashes.is_empty(), |cell| {
                                let words = format!("Also runs {}", clashes.join(", "));
                                cell.child(
                                    div()
                                        .id((self.id.clone(), format!("clash-{ix}")))
                                        .tooltip(Tooltip::text(words))
                                        .child(
                                            Icon::new(IconName::TriangleAlert)
                                                .size(IconSize::Xs)
                                                .color(colors.warning),
                                        ),
                                )
                            })
                            .into_any_element(),
                        None => div()
                            .text_color(colors.fg_subtle)
                            .child("—")
                            .into_any_element(),
                    }
                };
                let record = self.on_record.clone();
                let command = binding.command.clone();
                columns(div().id((self.id.clone(), format!("row-{ix}"))))
                    .py_1p5()
                    .rounded(theme.radius(Radius::Sm))
                    .when(recording, |row| row.bg(colors.active))
                    .when(!recording, |row| row.hover(|row| row.bg(colors.hover)))
                    .when_some(record, |row, record| {
                        row.on_click(move |event, window, cx| {
                            if event.click_count() == 2 {
                                record(&command, window, cx)
                            }
                        })
                    })
                    .child(
                        div()
                            .w(widths[0])
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(colors.fg)
                                    .child(binding.title.clone()),
                            )
                            .child(
                                div()
                                    .font_family(theme.mono_family.clone())
                                    .text_size(theme.text_size(TextSize::Xs))
                                    .text_color(colors.fg_subtle)
                                    .child(binding.command.clone()),
                            ),
                    )
                    .child(div().w(widths[1]).child(keys))
                    .child(
                        div()
                            .w(widths[2])
                            .font_family(theme.mono_family.clone())
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg_muted)
                            .children(binding.when.clone()),
                    )
                    .child(div().w(widths[3]).flex().child(match binding.source {
                        KeySource::Default => Badge::new("Default"),
                        KeySource::User => Badge::new("User").tone(Tone::Info),
                    }))
            });
        div()
            .flex()
            .flex_col()
            .text_size(theme.text_size(TextSize::Sm))
            .child(head)
            .children(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding(command: &str, keys: &str, when: Option<&str>) -> Keybinding {
        Keybinding {
            command: command.to_string().into(),
            title: command.to_string().into(),
            keys: Some(keys.to_string().into()),
            when: when.map(|when| when.to_string().into()),
            source: KeySource::Default,
        }
    }

    #[test]
    fn keys_clash_only_in_the_same_context() {
        let all = [
            binding("save", "secondary-s", None),
            binding("sort", "secondary-s", None),
            binding("split", "secondary-s", Some("Terminal")),
        ];
        assert_eq!(conflicts(&all, &all[0]), ["sort"]);
        assert!(
            conflicts(&all, &all[2]).is_empty(),
            "another context does not clash"
        );
    }
}
