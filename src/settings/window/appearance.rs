mod cards;

use std::ops::RangeInclusive;

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, IconButton},
    forms::{FontPicker, NumberInput, Slider, Switch, TextInput},
    i18n::text,
    primitives::IconName,
    settings::{SettingsRow, SettingsSection},
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
};
use gpui::{
    App, Context, Div, Entity, ParentElement, SharedString, Styled, Window, div, prelude::*, px,
    relative,
};

use self::cards::grid;
use super::{SettingsWindow, choice, set, switch};
use crate::{
    settings::{
        CursorStyle, FONT_SIZE, LINE_HEIGHT, Language, OPACITY, PADDING, Settings, SettingsStore,
    },
    themes::{self, TerminalTheme},
};

/// Each cursor style, its key and its name's key.
const CURSORS: [(CursorStyle, &str, &str); 4] = [
    (CursorStyle::Block, "block", "et.cursor.block"),
    (CursorStyle::Bar, "bar", "et.cursor.bar"),
    (CursorStyle::Underline, "underline", "et.cursor.underline"),
    (CursorStyle::Hollow, "hollow", "et.cursor.hollow"),
];

/// Each language, its key, and its own name's key.
const LANGUAGES: [(Language, &str, &str); 2] = [
    (Language::English, "en-US", "et.language.english"),
    (Language::SimplifiedChinese, "zh-CN", "et.language.chinese"),
];

impl SettingsWindow {
    /// The language, theme, font, cursor and window rows.
    pub(super) fn appearance(&self, window: &Window, cx: &Context<Self>) -> Div {
        let settings = SettingsStore::get(cx);
        let theme = cx.theme();
        let this = cx.entity();
        let word = |key: &str| text(cx, key, &[]);
        let language = choice(
            &this,
            "language",
            &LANGUAGES,
            settings.language,
            |settings, language| settings.language = language,
            cx,
        );
        let language = SettingsSection::new(word("et.appearance.language"))
            .row(SettingsRow::new(word("et.appearance.interface")).control(language));
        let separate = settings.dark_theme.is_some();
        let dark = Switch::new("dark_theme", separate).on_change({
            let this = this.clone();
            move |on, _, cx| {
                let theme = SettingsStore::get(cx).theme.clone();
                set(
                    &this,
                    format!("separate dark theme is {on}"),
                    cx,
                    |settings| settings.dark_theme = on.then_some(theme),
                );
            }
        });
        let light = separate.then(|| word("et.appearance.light"));
        let mut themes = SettingsSection::new(word("et.appearance.theme"))
            .row(grid(&this, light, false, settings, window, cx))
            .row(SettingsRow::new(word("et.appearance.separate_dark")).control(dark));
        if separate {
            let dark = Some(word("et.appearance.dark"));
            themes = themes.row(grid(&this, dark, true, settings, window, cx));
        }
        let family = FontPicker::new("font_family", &self.font)
            .selected(settings.font_family.clone())
            .on_change({
                let this = this.clone();
                move |family, window, cx| {
                    let family = family.to_string();
                    let saved = set(
                        &this,
                        format!("font_family is {family:?}"),
                        cx,
                        |settings| settings.font_family = family,
                    );
                    // A refused pick shows the saved family again, the list closed.
                    if !saved {
                        let saved = SettingsStore::get(cx).font_family.clone();
                        let input = this.read(cx).font.clone();
                        input.update(cx, |input, cx| input.set_text(saved, cx));
                        window.blur(cx);
                    }
                }
            });
        let fallbacks = SettingsRow::new(word("et.appearance.fallbacks"))
            .description(word("et.appearance.fallbacks_hint"))
            .control(self.fallback_list(cx));
        let family = div().w(theme.prose_width()).child(family);
        let font = SettingsSection::new(word("et.appearance.font"))
            .row(SettingsRow::new(word("et.appearance.family")).control(family))
            .row(fallbacks)
            .row(
                SettingsRow::new(word("et.appearance.size")).control(self.number(
                    "font_size",
                    settings.font_size,
                    &FONT_SIZE,
                    (1.0, 1),
                    |settings, size| settings.font_size = size,
                    cx,
                )),
            )
            .row(
                SettingsRow::new(word("et.appearance.line_height")).control(self.number(
                    "line_height",
                    settings.line_height,
                    &LINE_HEIGHT,
                    (0.05, 2),
                    |settings, height| settings.line_height = height,
                    cx,
                )),
            )
            .row(
                SettingsRow::new(word("et.appearance.ligatures")).control(switch(
                    &this,
                    "ligatures",
                    settings.ligatures,
                    |settings, on| settings.ligatures = on,
                )),
            );
        let style = choice(
            &this,
            "cursor_style",
            &CURSORS,
            settings.cursor_style,
            |settings, style| settings.cursor_style = style,
            cx,
        );
        let active = themes::active(settings, window.appearance(), cx);
        let lit = self.lit || !settings.cursor_blink;
        let preview = prompt(settings, &active, lit, cx);
        let cursor = SettingsSection::new(word("et.appearance.cursor"))
            .row(SettingsRow::new(word("et.appearance.style")).control(style))
            .row(
                SettingsRow::new(word("et.appearance.blink")).control(switch(
                    &this,
                    "cursor_blink",
                    settings.cursor_blink,
                    |settings, on| settings.cursor_blink = on,
                )),
            )
            .row(SettingsRow::new(word("et.appearance.preview")).control(preview));
        let opacity = percent(settings.background_opacity);
        let slider = Slider::new("background_opacity", opacity)
            .range(percent(*OPACITY.start()), percent(*OPACITY.end()))
            .on_change({
                let this = this.clone();
                move |percent, _, cx| {
                    let value = fraction(percent);
                    set(
                        &this,
                        format!("background_opacity is {value}"),
                        cx,
                        |settings| settings.background_opacity = value,
                    );
                }
            });
        let opacity = div()
            .flex()
            .items_center()
            .gap_2()
            .child(div().w(theme.label_width()).child(slider))
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Sm))
                    .child(format!("{opacity}%")),
            );
        let background = SettingsSection::new(word("et.appearance.window"))
            .row(SettingsRow::new(word("et.appearance.opacity")).control(opacity))
            .row(SettingsRow::new(word("et.appearance.blur")).control(switch(
                &this,
                "background_blur",
                settings.background_blur,
                |settings, on| settings.background_blur = on,
            )))
            .row(
                SettingsRow::new(word("et.appearance.padding_x")).control(self.number(
                    "padding_x",
                    settings.padding_x,
                    &PADDING,
                    (1.0, 0),
                    |settings, padding| settings.padding_x = padding,
                    cx,
                )),
            )
            .row(
                SettingsRow::new(word("et.appearance.padding_y")).control(self.number(
                    "padding_y",
                    settings.padding_y,
                    &PADDING,
                    (1.0, 0),
                    |settings, padding| settings.padding_y = padding,
                    cx,
                )),
            );
        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(language)
            .child(themes)
            .child(font)
            .child(cursor)
            .child(background)
    }

    /// An input for each fallback picker and the one that adds a family.
    pub(super) fn grow_fallbacks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let wanted = SettingsStore::get(cx).font_fallbacks.len() + 1;
        while self.fallbacks.len() < wanted {
            let input = cx.new(|cx| TextInput::new(window, cx));
            self.fallbacks.push(input);
        }
    }

    /// Each fallback family, a picker that replaces it and a button that removes it, then Add or the empty picker it opens.
    fn fallback_list(&self, cx: &Context<Self>) -> Div {
        let this = cx.entity();
        let families = SettingsStore::get(cx).font_fallbacks.clone();
        let width = cx.theme().prose_width();
        let row = |index: usize, picker: FontPicker, remove: IconButton| {
            let tip = if index < families.len() {
                "et.appearance.remove"
            } else {
                "et.appearance.cancel"
            };
            let tip = text(cx, tip, &[]);
            div()
                .flex()
                .items_start()
                .gap_2()
                .child(div().w(width).child(picker))
                .child(
                    remove
                        .variant(ButtonVariant::Ghost)
                        .size(ControlSize::Sm)
                        .tooltip(tip),
                )
        };
        let rows = families.iter().enumerate().map(|(index, family)| {
            let picker = FontPicker::new(keyed(&self.fallbacks[index]), &self.fallbacks[index])
                .selected(family.clone())
                .on_change(pick(&this, index));
            let remove = IconButton::new(("remove_fallback", index), IconName::X).on_click({
                let (this, family) = (this.clone(), family.clone());
                move |_, window, cx| {
                    let dropped = format!("font_fallbacks drops {family:?}");
                    let removed = set(&this, dropped, cx, |settings| {
                        settings.font_fallbacks.remove(index);
                    });
                    // Each input left keeps its own family, and none the dropped one.
                    if removed {
                        this.update(cx, |this, _| this.fallbacks.remove(index));
                        window.blur(cx);
                    }
                }
            });
            row(index, picker, remove)
        });
        let next = families.len();
        let add = if self.adding {
            let picker = FontPicker::new(keyed(&self.fallbacks[next]), &self.fallbacks[next])
                .on_change(pick(&this, next));
            let cancel = IconButton::new("cancel_fallback", IconName::X).on_click({
                let this = this.clone();
                move |_, window, cx| {
                    this.update(cx, |this, cx| this.leave_adding(cx));
                    window.blur(cx);
                }
            });
            row(next, picker, cancel)
        } else {
            let add = Button::new("add_fallback", text(cx, "et.appearance.add_fallback", &[]))
                .icon(IconName::Plus)
                .size(ControlSize::Sm)
                .on_click({
                    let this = this.clone();
                    move |_, _, cx| {
                        this.update(cx, |this, cx| {
                            this.adding = true;
                            cx.notify();
                        })
                    }
                });
            div().child(add)
        };
        div().flex().flex_col().gap_2().children(rows).child(add)
    }

    /// Closes the empty picker, its text gone.
    fn leave_adding(&mut self, cx: &mut Context<Self>) {
        let next = SettingsStore::get(cx).font_fallbacks.len();
        if let Some(input) = self.fallbacks.get(next) {
            input.update(cx, |input, cx| input.set_text("", cx));
        }
        self.adding = false;
        cx.notify();
    }

    /// A field over `range`, by `step` to `places`, that saves each committed number through `put`; a refusal starts it again from the settings.
    fn number(
        &self,
        field: &'static str,
        value: f32,
        range: &RangeInclusive<f32>,
        (step, places): (f64, usize),
        put: fn(&mut Settings, f32),
        cx: &Context<Self>,
    ) -> NumberInput {
        let this = cx.entity();
        NumberInput::new((field, self.drafts), f64::from(value))
            .range(f64::from(*range.start()), f64::from(*range.end()))
            .step(step)
            .precision(places)
            .size(ControlSize::Sm)
            .on_commit(move |value, _, cx| {
                let value = value as f32;
                set(&this, format!("{field} is {value}"), cx, |settings| {
                    put(settings, value)
                });
            })
    }
}

/// A fallback picker's id, its input's: Ely's combobox keeps state by id and listens to the input it met first.
fn keyed(input: &Entity<TextInput>) -> (&'static str, u64) {
    ("font_fallback", input.entity_id().as_u64())
}

/// Saves the family picked at fallback `index`, past the last adds one; a refusal shows the saved family again, the list closed.
fn pick(
    this: &Entity<SettingsWindow>,
    index: usize,
) -> impl Fn(&SharedString, &mut Window, &mut App) + 'static {
    let this = this.clone();
    move |family, window, cx| {
        let family = family.to_string();
        let adds = index == SettingsStore::get(cx).font_fallbacks.len();
        let saved = set(
            &this,
            format!("font_fallbacks[{index}] is {family:?}"),
            cx,
            |settings| {
                if adds {
                    settings.font_fallbacks.push(family)
                } else {
                    settings.font_fallbacks[index] = family
                }
            },
        );
        if adds && saved {
            this.update(cx, |this, cx| {
                this.adding = false;
                cx.notify();
            });
        } else if !saved {
            let shown = SettingsStore::get(cx)
                .font_fallbacks
                .get(index)
                .cloned()
                .unwrap_or_default();
            let input = this.read(cx).fallbacks[index].clone();
            input.update(cx, |input, cx| input.set_text(shown, cx));
            window.blur(cx);
        }
    }
}

/// `fraction` in whole percents; the slider moves by them, so a saved fraction carries no float noise.
fn percent(fraction: f32) -> f64 {
    (f64::from(fraction) * 100.).round()
}

fn fraction(percent: f64) -> f32 {
    (percent / 100.) as f32
}

/// A prompt in the terminal's theme and font, its cursor drawn as the terminal draws it.
fn prompt(settings: &Settings, theme: &TerminalTheme, lit: bool, cx: &App) -> Div {
    let rule = (px(settings.font_size) / 14.).max(px(1.));
    let mark = div().absolute().bg(theme.cursor);
    let cursor = match settings.cursor_style {
        CursorStyle::Block => mark.inset_0(),
        CursorStyle::Bar => mark.top_0().bottom_0().left_0().w(rule * 2.),
        CursorStyle::Underline => mark.left_0().right_0().bottom_0().h(rule * 2.),
        CursorStyle::Hollow => div()
            .absolute()
            .inset_0()
            .border_1()
            .border_color(theme.cursor),
    };
    let chrome = cx.theme();
    div()
        .flex()
        .p_2()
        .rounded(chrome.radius(Radius::Sm))
        .border_1()
        .border_color(chrome.colors.border)
        .bg(theme.background)
        .text_color(theme.foreground)
        .font_family(settings.font_family.clone())
        .text_size(px(settings.font_size))
        .line_height(relative(settings.line_height))
        .child(div().text_color(theme.ansi[2]).child("$"))
        .child(" ls")
        .child(
            div()
                .relative()
                .child(" ")
                .when(lit, |cell| cell.child(cursor)),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_percent_saves_as_its_own_short_fraction() {
        let (low, high) = (percent(*OPACITY.start()), percent(*OPACITY.end()));
        assert_eq!((low, high), (30., 100.));
        for whole in 30..=100 {
            let saved = fraction(f64::from(whole));
            assert_eq!(percent(saved), f64::from(whole));
            let text = serde_json::to_string(&saved).expect("a number");
            assert!(text.len() <= 4, "{whole}% saves as {text}");
        }
    }
}
