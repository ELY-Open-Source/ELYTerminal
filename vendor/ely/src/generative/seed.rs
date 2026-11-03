use std::{
    hash::{BuildHasher, Hasher, RandomState},
    ops::Range,
    rc::Rc,
};

use gpui::{App, ElementId, Entity, EntityId, IntoElement, RenderOnce, Window};

use crate::{
    buttons::IconButton,
    forms::{InputAddon, InputGroup, TextInput},
    primitives::IconName,
};

/// Fits an edit into a seed: digits only, and nothing past `u32::MAX`; a refused edit keeps the text.
pub(crate) fn digits(text: &str, range: Range<usize>, typed: &str) -> (String, usize) {
    let kept: String = typed.chars().filter(char::is_ascii_digit).collect();
    let next = format!("{}{kept}{}", &text[..range.start], &text[range.end..]);
    if next.is_empty() || next.parse::<u32>().is_ok() {
        (next, range.start + kept.len())
    } else {
        (text.to_string(), range.start)
    }
}

/// A seed from the system's source of randomness.
fn roll() -> u32 {
    RandomState::new().build_hasher().finish() as u32
}

/// A generation's seed in the owner's field: digits up to `u32::MAX`, empty for a new one each run, and a die that rolls one.
#[derive(IntoElement)]
pub struct SeedInput {
    id: ElementId,
    field: Entity<TextInput>,
}

impl SeedInput {
    pub fn new(id: impl Into<ElementId>, field: &Entity<TextInput>) -> Self {
        Self {
            id: id.into(),
            field: field.clone(),
        }
    }

    /// The seed in `field`, read after a `SeedInput` drew it; none while it is empty.
    pub fn read(field: &Entity<TextInput>, cx: &App) -> Option<u32> {
        let text = field.read(cx).committed();
        (!text.is_empty()).then(|| text.parse().expect("a seed field holds digits"))
    }
}

impl RenderOnce for SeedInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let fitted =
            window.use_keyed_state((self.id.clone(), "fitted"), cx, |_, _| None::<EntityId>);
        let field_id = self.field.entity_id();
        if *fitted.read(cx) != Some(field_id) {
            fitted.update(cx, |fitted, _| *fitted = Some(field_id));
            self.field.update(cx, |input, cx| {
                let text = input.text().to_string();
                let kept = match text.is_empty() || text.parse::<u32>().is_ok() {
                    true => text,
                    false => {
                        log::error!("seed input: {text:?} is no seed; cleared");
                        String::new()
                    }
                };
                input.set_fit(Rc::new(digits));
                input.set_text(kept, cx);
                input.set_placeholder("Random", cx);
            });
        }
        let field = self.field.clone();
        InputGroup::new(&self.field).after(InputAddon::new(
            IconButton::new((self.id.clone(), "roll"), IconName::Dices)
                .tooltip("Roll a seed")
                .on_click(move |_, _, cx| {
                    let seed = roll();
                    log::info!("seed input: rolled {seed}");
                    field.update(cx, |input, cx| input.set_text(seed.to_string(), cx));
                }),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::digits;

    #[test]
    fn a_seed_keeps_digits_up_to_the_largest() {
        assert_eq!(digits("12", 2..2, "3a4"), ("1234".to_string(), 4));
        assert_eq!(digits("", 0..0, "12,345"), ("12345".to_string(), 5));
        assert_eq!(
            digits("4294967295", 10..10, "1"),
            ("4294967295".to_string(), 10)
        );
        assert_eq!(digits("1234", 1..3, ""), ("14".to_string(), 1));
    }
}
