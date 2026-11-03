use std::{ops::Range, rc::Rc};

use gpui::{
    AnyElement, App, Bounds, ElementId, Entity, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window,
    canvas, div, prelude::*,
};

use super::{
    Input, TextInput,
    options::{Choice, Popup},
};
use crate::{
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, Radius},
};

fn is_slot(slot: char) -> bool {
    matches!(slot, '9' | 'a' | '*')
}

/// Fits typed characters into `mask`: `9` a digit, `a` a letter, `*` either; the rest is literal.
pub(crate) fn apply_mask(mask: &str, raw: &str) -> String {
    let mut raw = raw.chars().filter(char::is_ascii_alphanumeric).peekable();
    let mut out = String::new();
    for slot in mask.chars() {
        let fits = |ch: &char| match slot {
            '9' => ch.is_ascii_digit(),
            'a' => ch.is_ascii_alphabetic(),
            _ => ch.is_ascii_alphanumeric(),
        };
        if raw.peek().is_none() {
            break;
        }
        if !is_slot(slot) {
            out.push(slot);
            continue;
        }
        while raw.peek().is_some_and(|ch| !fits(ch)) {
            raw.next();
        }
        match raw.next() {
            Some(ch) => out.push(ch),
            None => break,
        }
    }
    out
}

/// What `fitted` holds in the slots of `mask`, from char `from` to char `to`.
fn slot_chars(mask: &str, fitted: &str, from: usize, to: usize) -> String {
    mask.chars()
        .zip(fitted.chars())
        .enumerate()
        .filter(|(ix, (slot, _))| (from..to).contains(ix) && is_slot(*slot))
        .map(|(_, (_, ch))| ch)
        .collect()
}

/// Fits an edit into `mask`: `range` of `text` becomes `typed`. Gives the text and caret.
pub(crate) fn refit(mask: &str, text: &str, range: Range<usize>, typed: &str) -> (String, usize) {
    let at = |byte: usize| text[..byte].chars().count();
    let head = slot_chars(mask, text, 0, at(range.start)) + typed;
    let fitted = apply_mask(
        mask,
        &(head.clone() + &slot_chars(mask, text, at(range.end), usize::MAX)),
    );
    let caret = if typed.is_empty() {
        range.start
    } else {
        apply_mask(mask, &head).len()
    };
    let caret = caret.min(fitted.len());
    (fitted, caret)
}

/// The mask shown while empty: slots become underscores.
pub(crate) fn mask_hint(mask: &str) -> String {
    mask.chars()
        .map(|slot| {
            if matches!(slot, '9' | 'a' | '*') {
                '_'
            } else {
                slot
            }
        })
        .collect()
}

fn masking(
    id: &ElementId,
    state: &Entity<TextInput>,
    mask: &SharedString,
    window: &mut Window,
    cx: &mut App,
) {
    let applied = window.use_keyed_state(id.clone(), cx, |_, _| None::<SharedString>);
    let old = applied.read(cx).clone();
    if old.as_ref() != Some(mask) {
        applied.update(cx, |applied, _| *applied = Some(mask.clone()));
        let fit_mask = mask.clone();
        state.update(cx, |input, cx| {
            let raw = match &old {
                Some(old) => slot_chars(old, input.text(), 0, usize::MAX),
                None => input.text().to_string(),
            };
            input.set_fit(Rc::new(move |text, range, typed| {
                refit(&fit_mask, text, range, typed)
            }));
            input.set_text(raw, cx);
        });
        log::info!("masked input: mask {mask}");
    }
    let hint = mask_hint(mask);
    if state.read(cx).placeholder_text().as_ref() != hint {
        state.update(cx, |input, cx| input.set_placeholder(hint, cx));
    }
}

/// A field that fits typing into a pattern such as `(999) 999-9999`.
#[derive(IntoElement)]
pub struct MaskedInput {
    id: ElementId,
    state: Entity<TextInput>,
    mask: SharedString,
    size: ControlSize,
    prefix: Option<AnyElement>,
}

impl MaskedInput {
    pub fn new(
        id: impl Into<ElementId>,
        state: &Entity<TextInput>,
        mask: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            mask: mask.into(),
            size: ControlSize::default(),
            prefix: None,
        }
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }
}

impl RenderOnce for MaskedInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        masking(&self.id, &self.state, &self.mask, window, cx);
        let field = Input::new(&self.state).size(self.size);
        match self.prefix {
            Some(prefix) => field.prefix(prefix),
            None => field,
        }
    }
}

/// Dialing code, flag and number pattern of each country offered.
pub const COUNTRIES: [(&str, &str, &str, &str); 6] = [
    ("US", "+1", "🇺🇸", "(999) 999-9999"),
    ("GB", "+44", "🇬🇧", "9999 999999"),
    ("DE", "+49", "🇩🇪", "9999 9999999"),
    ("FR", "+33", "🇫🇷", "9 99 99 99 99"),
    ("JP", "+81", "🇯🇵", "99-9999-9999"),
    ("CN", "+86", "🇨🇳", "999 9999 9999"),
];

type OnCountry = Rc<dyn Fn(&'static str, &mut Window, &mut App)>;

/// A phone number: a country picker for the code, then its number pattern.
#[derive(IntoElement)]
pub struct PhoneInput {
    id: ElementId,
    state: Entity<TextInput>,
    country: &'static str,
    on_country: Option<OnCountry>,
}

impl PhoneInput {
    /// `country` is a code from `COUNTRIES`.
    pub fn new(id: impl Into<ElementId>, state: &Entity<TextInput>, country: &'static str) -> Self {
        assert!(
            COUNTRIES.iter().any(|(code, ..)| *code == country),
            "phone input has no country {country}"
        );
        Self {
            id: id.into(),
            state: state.clone(),
            country,
            on_country: None,
        }
    }

    pub fn on_country(
        mut self,
        handler: impl Fn(&'static str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_country = Some(Rc::new(handler));
        self
    }
}

struct Picker {
    open: bool,
    anchor: Bounds<Pixels>,
}

impl RenderOnce for PhoneInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let picker = window.use_keyed_state((self.id.clone(), "picker"), cx, |_, _| Picker {
            open: false,
            anchor: Bounds::default(),
        });
        let (open, anchor) = (picker.read(cx).open, picker.read(cx).anchor);
        let &(_, dial, flag, mask) = COUNTRIES
            .iter()
            .find(|(code, ..)| *code == self.country)
            .expect("checked at construction");
        let theme = cx.theme();
        let (hover, muted) = (theme.colors.hover, theme.colors.fg_muted);
        let rows: Vec<Choice> = COUNTRIES
            .iter()
            .map(|(code, dial, flag, _)| Choice::new(*code, format!("{flag}  {code}")).note(*dial))
            .collect();
        let (toggle, close, measure) = (picker.clone(), picker.clone(), picker.clone());
        let on_country = self.on_country;
        let pick = Rc::new(move |ix: usize, window: &mut Window, cx: &mut App| {
            let code = COUNTRIES[ix].0;
            log::info!("phone input: country {code}");
            close.update(cx, |picker, cx| {
                picker.open = false;
                cx.notify();
            });
            if let Some(on_country) = &on_country {
                on_country(code, window, cx);
            }
        });
        let dismiss = picker.clone();
        let code = div()
            .id((self.id.clone(), "code"))
            .flex()
            .items_center()
            .gap_1()
            .px_1()
            .rounded(theme.radius(Radius::Sm))
            .text_color(muted)
            .cursor_pointer()
            .hover(|style| style.bg(hover))
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .on_click(move |_, _, cx| {
                toggle.update(cx, |picker, cx| {
                    picker.open = !picker.open;
                    cx.notify();
                })
            })
            .child(format!("{flag} {dial}"))
            .child(
                Icon::new(IconName::ChevronDown)
                    .size(IconSize::Xs)
                    .color(muted),
            );
        div()
            .relative()
            .child(MaskedInput::new(self.id.clone(), &self.state, mask).prefix(code))
            .child(
                canvas(
                    move |bounds, _, cx| {
                        if measure.read(cx).anchor != bounds {
                            measure.update(cx, |picker, _| picker.anchor = bounds);
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .when(open, |field| {
                let current = [SharedString::from(self.country)];
                field.child(
                    Popup {
                        id: (self.id.clone(), "countries").into(),
                        anchor,
                        rows: &rows,
                        highlighted: None,
                        checked: Some(&current),
                        pick,
                        dismiss: Some(Rc::new(move |_, cx| {
                            dismiss.update(cx, |picker, cx| {
                                picker.open = false;
                                cx.notify();
                            })
                        })),
                        scroll: None,
                        reveal: None,
                    }
                    .render(window, cx),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_mask, mask_hint, refit};

    fn typed(mask: &str, keys: &str) -> String {
        keys.chars().fold(String::new(), |text, key| {
            let end = text.len();
            refit(mask, &text, end..end, &key.to_string()).0
        })
    }

    #[test]
    fn mask_fits_slots_skips_misfits_and_stops_with_the_input() {
        assert_eq!(apply_mask("(999) 999-9999", "5551234567"), "(555) 123-4567");
        assert_eq!(apply_mask("(999) 999-9999", "555"), "(555");
        assert_eq!(apply_mask("(999) 999-9999", "(55x5) 1"), "(555) 1");
        assert_eq!(apply_mask("aa-9999", "ab1234"), "ab-1234");
        assert_eq!(apply_mask("aa-9999", "12ab"), "ab");
        assert_eq!(mask_hint("(999) 999-9999"), "(___) ___-____");
    }

    #[test]
    fn typing_fills_slots_and_literals_never_eat_input() {
        assert_eq!(typed("+1 999", "234"), "+1 234");
        assert_eq!(typed("1-999", "123"), "1-123");
        assert_eq!(typed("(999) 999-9999", "5551234567"), "(555) 123-4567");
    }

    #[test]
    fn an_edit_replaces_its_range_and_places_the_caret() {
        let fit = |mask, text, range, typed| refit(mask, text, range, typed);
        assert_eq!(fit("1-999", "1-123", 0..5, "1"), ("1-1".into(), 3));
        assert_eq!(fit("+1 999", "+1 23", 2..3, ""), ("+1 23".into(), 2));
        assert_eq!(
            fit("(999) 999-9999", "(555) 123-4567", 8..8, "9"),
            ("(555) 129-3456".into(), 9)
        );
        assert_eq!(
            fit("(999) 999-9999", "(555) 1", 6..7, ""),
            ("(555".into(), 4)
        );
        assert_eq!(
            fit("(999) 999-9999", "", 0..0, "(555) 123-4567"),
            ("(555) 123-4567".into(), 14)
        );
    }
}
