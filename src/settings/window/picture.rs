//! The picture's rows: its file, scope, size, position, repeat and gradient.

use std::str::FromStr;

use ely_gpui_component::{
    feedback::InlineMessage,
    forms::{Input, InputEvent, NumberInput, PathInput, TextInput},
    i18n::text,
    primitives::Severity,
    settings::SettingsRow,
    theme::{ActiveTheme, ControlSize},
};
use gpui::{
    App, Context, Div, Entity, IntoElement, ParentElement, SharedString, Styled, Subscription,
    Window, div, prelude::*,
};

use super::{
    SettingsWindow, choice, set, switch,
    terminal::{chosen, path_field},
};
use crate::settings::{BackgroundScope, GRADIENT_ANGLE, PictureRepeat, Settings, SettingsStore};

/// Each picture scope, its key and its name's key.
const SCOPES: [(BackgroundScope, &str, &str); 2] = [
    (
        BackgroundScope::Terminal,
        "terminal",
        "et.appearance.scope_terminal",
    ),
    (
        BackgroundScope::Window,
        "window",
        "et.appearance.scope_window",
    ),
];

/// Each repeat, its key and its name's key.
const REPEATS: [(PictureRepeat, &str, &str); 4] = [
    (
        PictureRepeat::NoRepeat,
        "no_repeat",
        "et.appearance.picture_repeat_no_repeat",
    ),
    (
        PictureRepeat::Repeat,
        "repeat",
        "et.appearance.picture_repeat_repeat",
    ),
    (
        PictureRepeat::RepeatX,
        "repeat_x",
        "et.appearance.picture_repeat_x",
    ),
    (
        PictureRepeat::RepeatY,
        "repeat_y",
        "et.appearance.picture_repeat_y",
    ),
];

/// The picture's fields: its file, size, position, gradient colors, and why each is not saved.
pub(super) struct Fields {
    pub(super) image: Entity<TextInput>,
    size: Entity<TextInput>,
    position: Entity<TextInput>,
    from: Entity<TextInput>,
    to: Entity<TextInput>,
    pub(super) refused_image: Option<SharedString>,
    refused_size: Option<SharedString>,
    refused_position: Option<SharedString>,
    refused_from: Option<SharedString>,
    refused_to: Option<SharedString>,
    _typed: Vec<Subscription>,
}

impl Fields {
    /// The fields as the settings hold them, each saved as it is typed.
    pub(super) fn new(window: &mut Window, cx: &mut Context<SettingsWindow>) -> Self {
        let settings = SettingsStore::get(cx).clone();
        let image = cx.new(|cx| {
            let mut input = TextInput::new(window, cx);
            input.set_text(settings.background_image.clone(), cx);
            input
        });
        let size = cx.new(|cx| {
            let mut input = TextInput::new(window, cx);
            input.set_text(settings.background_size.to_string(), cx);
            input
        });
        let position = cx.new(|cx| {
            let mut input = TextInput::new(window, cx);
            input.set_text(settings.background_position.to_string(), cx);
            input
        });
        let from = cx.new(|cx| {
            let mut input = TextInput::new(window, cx);
            input.set_text(settings.background_gradient_from.clone(), cx);
            input
        });
        let to = cx.new(|cx| {
            let mut input = TextInput::new(window, cx);
            input.set_text(settings.background_gradient_to.clone(), cx);
            input
        });
        let mut fields = Self {
            image,
            size,
            position,
            from,
            to,
            refused_image: None,
            refused_size: None,
            refused_position: None,
            refused_from: None,
            refused_to: None,
            _typed: Vec::new(),
        };
        fields.watch(cx);
        fields
    }

    /// Saves each field as the user types, or says why it is not saved.
    fn watch(&mut self, cx: &mut Context<SettingsWindow>) {
        self._typed.push(
            cx.subscribe(&self.image, |this, input, event: &InputEvent, cx| {
                if let InputEvent::Changed = event {
                    let text = input.read(cx).text().to_string();
                    let put = |settings: &mut Settings, text| settings.background_image = text;
                    this.picture.refused_image = this.save_typed("background_image", text, put, cx);
                }
            }),
        );
        self._typed.push(
            cx.subscribe(&self.size, |this, input, event: &InputEvent, cx| {
                if let InputEvent::Changed = event {
                    let text = input.read(cx).text().to_string();
                    let refused = save(
                        this,
                        "background_size",
                        text,
                        |settings, size| settings.background_size = size,
                        cx,
                    );
                    this.picture.refused_size = refused;
                }
            }),
        );
        self._typed.push(
            cx.subscribe(&self.position, |this, input, event: &InputEvent, cx| {
                if let InputEvent::Changed = event {
                    let text = input.read(cx).text().to_string();
                    let refused = save(
                        this,
                        "background_position",
                        text,
                        |settings, position| settings.background_position = position,
                        cx,
                    );
                    this.picture.refused_position = refused;
                }
            }),
        );
        self._typed.push(
            cx.subscribe(&self.from, |this, input, event: &InputEvent, cx| {
                if let InputEvent::Changed = event {
                    let text = input.read(cx).text().to_string();
                    let put = |settings: &mut Settings, text| {
                        settings.background_gradient_from = text;
                    };
                    this.picture.refused_from =
                        this.save_typed("background_gradient_from", text, put, cx);
                }
            }),
        );
        self._typed.push(
            cx.subscribe(&self.to, |this, input, event: &InputEvent, cx| {
                if let InputEvent::Changed = event {
                    let text = input.read(cx).text().to_string();
                    let put = |settings: &mut Settings, text| {
                        settings.background_gradient_to = text;
                    };
                    this.picture.refused_to =
                        this.save_typed("background_gradient_to", text, put, cx);
                }
            }),
        );
    }

    /// The rows the picture fills: its file, scope, size, position and repeat.
    pub(super) fn rows(
        &self,
        this: &Entity<SettingsWindow>,
        settings: &Settings,
        cx: &App,
    ) -> Vec<SettingsRow> {
        let word = |key: &str| text(cx, key, &[]);
        let image = PathInput::new("settings-background-image", &self.image)
            .on_choose(chosen(this, "background_image"));
        let image = path_field(image, self.refused_image.clone(), cx);
        let size = field(Input::new(&self.size), self.refused_size.clone(), cx);
        let position = field(
            Input::new(&self.position),
            self.refused_position.clone(),
            cx,
        );
        let angle = NumberInput::new(
            "background-gradient-angle",
            f64::from(settings.background_gradient_angle),
        )
        .range(
            f64::from(*GRADIENT_ANGLE.start()),
            f64::from(*GRADIENT_ANGLE.end()),
        )
        .step(15.0)
        .size(ControlSize::Sm)
        .on_commit({
            let this = this.clone();
            move |angle, _, cx| {
                let angle = angle as f32;
                set(
                    &this,
                    format!("background_gradient_angle is {angle}"),
                    cx,
                    |settings| settings.background_gradient_angle = angle,
                );
            }
        });
        let from = field(Input::new(&self.from), self.refused_from.clone(), cx);
        let to = field(Input::new(&self.to), self.refused_to.clone(), cx);
        vec![
            SettingsRow::new(word("et.appearance.image"))
                .description(word("et.appearance.image_hint"))
                .control(image),
            SettingsRow::new(word("et.appearance.scope")).control(choice(
                this,
                "background_scope",
                &SCOPES,
                settings.background_scope,
                |settings, scope| settings.background_scope = scope,
                cx,
            )),
            SettingsRow::new(word("et.appearance.picture_size"))
                .description(word("et.appearance.picture_size_hint"))
                .control(size),
            SettingsRow::new(word("et.appearance.picture_position"))
                .description(word("et.appearance.picture_position_hint"))
                .control(position),
            SettingsRow::new(word("et.appearance.picture_repeat")).control(choice(
                this,
                "background_repeat",
                &REPEATS,
                settings.background_repeat,
                |settings, repeat| settings.background_repeat = repeat,
                cx,
            )),
            SettingsRow::new(word("et.appearance.gradient")).control(switch(
                this,
                "background_gradient",
                settings.background_gradient,
                |settings, on| settings.background_gradient = on,
            )),
            SettingsRow::new(word("et.appearance.gradient_angle"))
                .description(word("et.appearance.gradient_angle_hint"))
                .control(angle),
            SettingsRow::new(word("et.appearance.gradient_from"))
                .description(word("et.appearance.gradient_color_hint"))
                .control(from),
            SettingsRow::new(word("et.appearance.gradient_to")).control(to),
        ]
    }
}

/// A field the picture's text fills, with why it is not saved.
fn field(input: impl IntoElement, refused: Option<SharedString>, cx: &App) -> Div {
    let refused = refused.map(|why| InlineMessage::new(Severity::Danger, why));
    div()
        .w(cx.theme().label_width())
        .flex()
        .flex_col()
        .gap_1()
        .child(input)
        .children(refused)
}

/// Saves `text` as `T` through `put`, or returns why it is not one.
fn save<T: FromStr<Err = anyhow::Error>>(
    this: &mut SettingsWindow,
    field: &'static str,
    text: String,
    put: impl FnOnce(&mut Settings, T),
    cx: &mut Context<SettingsWindow>,
) -> Option<SharedString> {
    match text.trim().parse::<T>() {
        Ok(value) => this.save_typed(field, text, move |settings, _| put(settings, value), cx),
        Err(error) => Some(format!("{error:#}").into()),
    }
}
