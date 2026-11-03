use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use crate::{
    buttons::ButtonVariant,
    forms::{EmojiPicker, OnValue},
    overlays::Popover,
    primitives::{FocusRing, IconName},
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
    typography::tabular,
};

/// Emoji most reactions use, offered before the full picker.
const QUICK: [&str; 6] = ["👍", "❤️", "🎉", "😄", "👀", "🙏"];

/// A reaction on a comment: its emoji, how many gave it, and whether you did.
#[derive(Clone, Debug, PartialEq)]
pub struct Reaction {
    pub emoji: SharedString,
    pub count: usize,
    pub mine: bool,
}

/// `reactions` after you toggle `emoji`: yours comes off, or goes on, a new one last.
pub fn toggled(reactions: &[Reaction], emoji: &str) -> Vec<Reaction> {
    let mut next: Vec<Reaction> = reactions.to_vec();
    match next
        .iter()
        .position(|reaction| reaction.emoji.as_ref() == emoji)
    {
        Some(ix) if next[ix].mine => {
            if next[ix].count == 0 {
                log::error!("reactions: {emoji} counts none yet is yours; it comes off");
            }
            next[ix].count = next[ix].count.saturating_sub(1);
            next[ix].mine = false;
            if next[ix].count == 0 {
                next.remove(ix);
            }
        }
        Some(ix) => {
            next[ix].count += 1;
            next[ix].mine = true;
        }
        None => next.push(Reaction {
            emoji: emoji.to_string().into(),
            count: 1,
            mine: true,
        }),
    }
    next
}

/// Picks an emoji to react with: a few most used, then every one.
#[derive(IntoElement)]
pub struct ReactionPicker {
    id: ElementId,
    on_pick: OnValue,
}

impl ReactionPicker {
    pub fn new(
        id: impl Into<ElementId>,
        on_pick: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            on_pick: Rc::new(on_pick),
        }
    }
}

impl RenderOnce for ReactionPicker {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let (id, on_pick) = (self.id.clone(), self.on_pick);
        Popover::new(self.id, "", move |_, cx| {
            let theme = cx.theme();
            let colors = theme.colors.clone();
            let (quick, all) = (on_pick.clone(), on_pick.clone());
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(div().flex().gap_1().children(QUICK.iter().map(|emoji| {
                    let (pick, emoji) = (quick.clone(), SharedString::from(*emoji));
                    let label = emoji.clone();
                    div()
                        .id((id.clone(), format!("quick-{emoji}")))
                        .p_1()
                        .rounded(theme.radius(Radius::Sm))
                        .cursor_pointer()
                        .text_size(theme.text_size(TextSize::Lg))
                        .hover(|face| face.bg(colors.hover))
                        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                        .on_click(move |_, window, cx| pick(&emoji, window, cx))
                        .child(label)
                })))
                .child(
                    EmojiPicker::new((id.clone(), "all")).on_change(move |glyph, window, cx| {
                        all(&SharedString::from(glyph.to_string()), window, cx)
                    }),
                )
        })
        .icon(IconName::SmilePlus)
        .variant(ButtonVariant::Ghost)
        .size(ControlSize::Sm)
    }
}

/// Reactions under a comment: each emoji with its count, yours outlined; a press toggles yours, and the picker adds another.
#[derive(IntoElement)]
pub struct Reactions {
    id: ElementId,
    reactions: Vec<Reaction>,
    on_toggle: Option<OnValue>,
}

impl Reactions {
    pub fn new(id: impl Into<ElementId>, reactions: impl IntoIterator<Item = Reaction>) -> Self {
        Self {
            id: id.into(),
            reactions: reactions.into_iter().collect(),
            on_toggle: None,
        }
    }

    /// Gets the emoji to toggle, from a chip or the picker.
    pub fn on_toggle(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Reactions {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .children(self.reactions.iter().map(|reaction| {
                let toggle = self.on_toggle.clone();
                let emoji = reaction.emoji.clone();
                div()
                    .id((self.id.clone(), format!("reaction-{emoji}")))
                    .flex()
                    .items_center()
                    .gap_1()
                    .px_1p5()
                    .py_0p5()
                    .rounded_full()
                    .border_1()
                    .border_color(if reaction.mine {
                        colors.focus
                    } else {
                        colors.border
                    })
                    .when(reaction.mine, |chip| chip.bg(colors.focus.opacity(0.1)))
                    .tab_index(0)
                    .focus_ring(cx)
                    .cursor_pointer()
                    .text_size(theme.text_size(TextSize::Xs))
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .when_some(toggle, |chip, toggle| {
                        chip.on_click(move |_, window, cx| {
                            log::info!("reactions: toggle {emoji}");
                            toggle(&emoji, window, cx)
                        })
                    })
                    .child(reaction.emoji.clone())
                    .child(
                        tabular(div().text_color(colors.fg_muted))
                            .child(format!("{}", reaction.count)),
                    )
            }))
            .children(self.on_toggle.map(|toggle| {
                ReactionPicker::new((self.id.clone(), "add"), move |emoji, window, cx| {
                    toggle(emoji, window, cx)
                })
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yours_at_no_count_comes_off() {
        assert_eq!(toggled(&[reaction("👍", 0, true)], "👍"), []);
    }

    fn reaction(emoji: &str, count: usize, mine: bool) -> Reaction {
        Reaction {
            emoji: emoji.to_string().into(),
            count,
            mine,
        }
    }

    #[test]
    fn a_toggle_adds_yours_or_takes_it_off() {
        let start = [reaction("👍", 2, false), reaction("🎉", 1, true)];
        assert_eq!(
            toggled(&start, "👍"),
            [reaction("👍", 3, true), reaction("🎉", 1, true)]
        );
        assert_eq!(
            toggled(&start, "🎉"),
            [reaction("👍", 2, false)],
            "the last one goes"
        );
        assert_eq!(toggled(&start, "👀").last(), Some(&reaction("👀", 1, true)));
    }
}
