use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div,
};

use crate::{
    buttons::IconButton,
    forms::{Choice, Select},
    primitives::IconName,
    theme::{ActiveTheme, TextSize},
    typography::Ellipsis,
};

/// A named set of variables a request fills its `{{names}}` from; a secret's value shows masked.
#[derive(Clone, Debug, PartialEq)]
pub struct Environment {
    pub key: SharedString,
    pub name: SharedString,
    pub variables: Vec<(SharedString, SharedString)>,
    pub secrets: Vec<SharedString>,
}

type OnKey = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// The environment requests use, picked from a list, over its variables; a secret shows as dots until its eye is pressed.
#[derive(IntoElement)]
pub struct EnvironmentSelector {
    id: ElementId,
    environments: Vec<Environment>,
    selected: SharedString,
    on_change: OnKey,
}

impl EnvironmentSelector {
    pub fn new(
        id: impl Into<ElementId>,
        environments: impl IntoIterator<Item = Environment>,
        selected: impl Into<SharedString>,
        on_change: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            environments: environments.into_iter().collect(),
            selected: selected.into(),
            on_change: Rc::new(on_change),
        }
    }
}

/// What a row shows for a value: dots for a secret until it is opened.
fn shown_value(value: &SharedString, secret: bool, open: bool) -> SharedString {
    match secret && !open {
        true => "••••••••".into(),
        false => value.clone(),
    }
}

/// A secret by its environment's key and its name.
type Secret = (SharedString, SharedString);

/// Opens `secret` when it is shut, shuts it when it is open.
fn flipped(open: &mut Vec<Secret>, secret: &Secret) {
    match open.iter().position(|each| each == secret) {
        Some(ix) => {
            open.remove(ix);
        }
        None => open.push(secret.clone()),
    }
}

impl RenderOnce for EnvironmentSelector {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let chosen = self
            .environments
            .iter()
            .find(|environment| environment.key == self.selected)
            .cloned();
        if chosen.is_none() {
            log::error!(
                "environment selector {id:?}: no environment {}; none chosen",
                self.selected
            );
        }
        let shown = window.use_keyed_state((id.clone(), "shown"), cx, |_, _| Vec::<Secret>::new());
        let theme = cx.theme();
        let on_change = self.on_change;
        let rows = chosen.iter().flat_map(|chosen| {
            chosen.variables.iter().map(|(name, value)| {
                let secret = chosen.secrets.contains(name);
                let key: Secret = (chosen.key.clone(), name.clone());
                let open = shown.read(cx).contains(&key);
                let flip = shown.clone();
                let seen = format!(
                    "environment-{}-{name}-{}",
                    chosen.key,
                    if secret && !open { "masked" } else { "shown" }
                );
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .py_1()
                    .child(
                        div()
                            .flex_none()
                            .w(theme.label_width())
                            .font_family(theme.mono_family.clone())
                            .text_size(theme.text_size(TextSize::Sm))
                            .child(Ellipsis::new(format!("{{{{{name}}}}}"))),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(theme.colors.fg_muted)
                            .debug_selector(move || seen)
                            .child(Ellipsis::new(shown_value(value, secret, open))),
                    )
                    .children(secret.then(|| {
                        IconButton::new(
                            (id.clone(), format!("reveal-{name}")),
                            if open {
                                IconName::EyeOff
                            } else {
                                IconName::Eye
                            },
                        )
                        .tooltip(if open {
                            "Hide the value"
                        } else {
                            "Show the value"
                        })
                        .on_click(move |_, _, cx| {
                            flip.update(cx, |shown, cx| {
                                flipped(shown, &key);
                                cx.notify();
                            })
                        })
                    }))
            })
        });
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                Select::new(
                    (id.clone(), "select"),
                    self.environments.iter().map(|environment| {
                        Choice::new(environment.key.clone(), environment.name.clone())
                    }),
                )
                .selected(self.selected.clone())
                .on_change(move |key, window, cx| {
                    log::info!("environment selector: {key}");
                    on_change(key, window, cx);
                }),
            )
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(theme.colors.fg_subtle)
                    .font_weight(FontWeight::MEDIUM)
                    .child(match &chosen {
                        Some(chosen) => format!("{} variables", chosen.variables.len()),
                        None => "No environment chosen".into(),
                    }),
            )
            .child(div().flex().flex_col().children(rows))
    }
}

#[cfg(test)]
mod tests {
    use gpui::SharedString;

    use super::{flipped, shown_value};

    #[test]
    fn a_secret_shows_dots_until_its_eye_opens_it() {
        let value = SharedString::from("s3cret");
        assert_eq!(shown_value(&value, true, false), "••••••••");
        assert_eq!(shown_value(&value, true, true), "s3cret");
        assert_eq!(shown_value(&value, false, false), "s3cret");
        let secret = (SharedString::from("staging"), SharedString::from("token"));
        let mut open = Vec::new();
        flipped(&mut open, &secret);
        assert_eq!(open, std::slice::from_ref(&secret));
        flipped(&mut open, &secret);
        assert!(open.is_empty(), "a second press shuts it again");
    }
}
