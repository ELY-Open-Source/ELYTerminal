use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div,
};

use crate::{
    buttons::{ButtonVariant, IconButton},
    data_display::color_mark,
    forms::{ColorPalette, Enter, FormError, InlineEdit, Input, OnValue, Run, TextInput},
    mail::Label,
    overlays::Popover,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, HUE_NAMES, IconSize, Radius},
};

type OnRename = Rc<dyn Fn(&SharedString, &SharedString, &mut Window, &mut App)>;
type OnRecolor = Rc<dyn Fn(&SharedString, usize, &mut Window, &mut App)>;

/// Why `name` cannot name a label, if it cannot: blank, or another label's already, whatever the case.
pub(crate) fn refusal(
    name: &str,
    labels: &[Label],
    except: Option<&SharedString>,
) -> Option<&'static str> {
    let name = name.trim().to_lowercase();
    if name.is_empty() {
        return Some("A label needs a name.");
    }
    let taken = labels
        .iter()
        .any(|label| Some(&label.key) != except && label.name.to_lowercase() == name);
    taken.then_some("That name is taken.")
}

/// A team's labels to tend: each wears its hue, which a press on it changes, and its name, which edits in place; a label can go, and a field at the end makes a new one. Names stay unique, whatever their case.
#[derive(IntoElement)]
pub struct LabelManager {
    id: ElementId,
    labels: Vec<Label>,
    on_rename: Option<OnRename>,
    on_recolor: Option<OnRecolor>,
    on_delete: Option<OnValue>,
    on_create: Option<OnValue>,
}

impl LabelManager {
    pub fn new(id: impl Into<ElementId>, labels: impl IntoIterator<Item = Label>) -> Self {
        let labels: Vec<Label> = labels.into_iter().collect();
        for (ix, label) in labels.iter().enumerate() {
            let before = &labels[..ix];
            assert!(
                !before.iter().any(|other| other.key == label.key),
                "label {} twice",
                label.key
            );
            assert!(
                refusal(&label.name, before, None).is_none(),
                "label name {} blank or twice",
                label.name
            );
        }
        Self {
            id: id.into(),
            labels,
            on_rename: None,
            on_recolor: None,
            on_delete: None,
            on_create: None,
        }
    }

    /// Gets a label's key and its new name, trimmed.
    pub fn on_rename(
        mut self,
        handler: impl Fn(&SharedString, &SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_rename = Some(Rc::new(handler));
        self
    }

    /// Gets a label's key and its new hue among the chart colors.
    pub fn on_recolor(
        mut self,
        handler: impl Fn(&SharedString, usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_recolor = Some(Rc::new(handler));
        self
    }

    /// Gets the key of the label to delete.
    pub fn on_delete(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_delete = Some(Rc::new(handler));
        self
    }

    /// Gets a new label's name, trimmed; the owner picks its key and hue.
    pub fn on_create(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_create = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for LabelManager {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let refused = window.use_keyed_state((id.clone(), "refused"), cx, |_, _| {
            None::<(SharedString, &'static str)>
        });
        let field = window.use_keyed_state((id.clone(), "new"), cx, |window, cx| {
            TextInput::new(window, cx).placeholder("New label")
        });
        let refuse = {
            let refused = refused.clone();
            move |at: Option<(SharedString, &'static str)>, cx: &mut App| {
                refused.update(cx, |refused, cx| {
                    *refused = at;
                    cx.notify();
                })
            }
        };
        let shown = refused.read(cx).clone();
        let labels = Rc::new(self.labels);
        let theme = cx.theme();
        let chart = theme.colors.chart;
        let error = |at: &str| {
            shown
                .as_ref()
                .filter(|(key, _)| key.as_ref() == at)
                .map(|(key, why)| {
                    let named = key.clone();
                    div()
                        .debug_selector(move || format!("refused {named}"))
                        .child(FormError::new((id.clone(), format!("refused-{key}")), *why))
                })
        };
        let rows = labels.iter().map(|label| {
            let key = label.key.clone();
            let hue = theme
                .colors
                .hue(label.hue, format_args!("label {}", label.name));
            let swatch_id = (id.clone(), format!("hue-{key}"));
            let palette = {
                let (id, key, on_recolor) = (id.clone(), key.clone(), self.on_recolor.clone());
                move |_: Run, _: &mut Window, _: &mut App| {
                    let colors = HUE_NAMES
                        .iter()
                        .zip(chart)
                        .map(|(name, color)| (*name, color));
                    ColorPalette::new((id.clone(), format!("palette-{key}")), colors)
                        .columns(4)
                        .selected(hue)
                        .on_change(move |color, window, cx| {
                            let hue = chart
                                .iter()
                                .position(|each| *each == color)
                                .expect("a hue of the chart");
                            log::info!("label manager: {key} to hue {hue}");
                            if let Some(on_recolor) = &on_recolor {
                                on_recolor(&key, hue, window, cx);
                            }
                        })
                }
            };
            let (mark, radius) = (color_mark(hue, cx), theme.radius(Radius::Sm));
            let (ring, lit) = (theme.colors.focus, theme.colors.hover);
            let swatch = Popover::with_opener(
                (id.clone(), format!("recolor-{key}")),
                move |toggle| {
                    div()
                        .id(swatch_id)
                        .flex_none()
                        .p_1()
                        .rounded(radius)
                        .border_1()
                        .border_color(gpui::transparent_black())
                        .tab_index(0)
                        .focus(move |style| style.border_color(ring))
                        .cursor_pointer()
                        .hover(move |style| style.bg(lit))
                        .on_click(move |_, window, cx| toggle(window, cx))
                        .child(mark)
                },
                palette,
            );
            let name = {
                let (key, labels, refuse) = (key.clone(), labels.clone(), refuse.clone());
                let on_rename = self.on_rename.clone();
                InlineEdit::new((id.clone(), format!("name-{key}")), label.name.clone()).on_commit(
                    move |name, window, cx| match refusal(name, &labels, Some(&key)) {
                        Some(why) => {
                            log::info!("label manager: {key} not renamed: {why}");
                            refuse(Some((key.clone(), why)), cx);
                        }
                        None => {
                            refuse(None, cx);
                            let name = SharedString::from(name.trim().to_string());
                            log::info!("label manager: {key} renamed {name}");
                            if let Some(on_rename) = &on_rename {
                                on_rename(&key, &name, window, cx);
                            }
                        }
                    },
                )
            };
            let delete = {
                let (key, on_delete) = (key.clone(), self.on_delete.clone());
                IconButton::new((id.clone(), format!("delete-{key}")), IconName::X)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .tooltip("Delete label")
                    .on_click(move |_, window, cx| {
                        log::info!("label manager: delete {key}");
                        if let Some(on_delete) = &on_delete {
                            on_delete(&key, window, cx);
                        }
                    })
            };
            div()
                .debug_selector(move || format!("label-row {key}"))
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(swatch)
                        .child(div().flex_1().min_w_0().child(name))
                        .child(delete),
                )
                .children(error(label.key.as_ref()))
        });
        let rows: Vec<_> = rows.collect();
        let create = {
            let (labels, field, on_create) = (labels.clone(), field.clone(), self.on_create);
            move |_: &Enter, window: &mut Window, cx: &mut App| {
                let words = field.read(cx).text().to_string();
                cx.stop_propagation();
                if let Some(why) = refusal(&words, &labels, None) {
                    log::info!("label manager: not made: {why}");
                    refuse(Some(("+new".into(), why)), cx);
                    return;
                }
                refuse(None, cx);
                field.update(cx, |input, cx| input.set_text(String::new(), cx));
                let name = SharedString::from(words.trim().to_string());
                log::info!("label manager: make {name}");
                if let Some(on_create) = &on_create {
                    on_create(&name, window, cx);
                }
            }
        };
        let plus = Icon::new(IconName::Plus)
            .size(IconSize::Sm)
            .color(theme.colors.fg_subtle);
        div()
            .debug_selector(|| "label-manager".into())
            .flex()
            .flex_col()
            .gap_2()
            .children(rows)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .capture_action(create)
                    .child(Input::new(&field).prefix(plus))
                    .children(error("+new")),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::refusal;
    use crate::mail::Label;

    #[test]
    fn a_name_is_refused_when_blank_or_taken_in_any_case() {
        let labels = [Label::new("bug", "Bug", 3), Label::new("site", "Site", 1)];
        assert_eq!(refusal("  ", &labels, None), Some("A label needs a name."));
        assert_eq!(
            refusal(" SITE ", &labels, None),
            Some("That name is taken.")
        );
        assert_eq!(
            refusal("bug", &labels, Some(&"bug".into())),
            None,
            "its own name"
        );
        assert_eq!(refusal("Client", &labels, None), None);
    }
}
