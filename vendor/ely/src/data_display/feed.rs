use std::{iter, ops::Range};

use gpui::{
    AnyElement, App, ElementId, FontWeight, HighlightStyle, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, StyledText, Window, div,
};
use jiff::Timestamp;

use super::{Avatar, TimelineItem};
use crate::{
    motion::AnimatePresence,
    primitives::IconName,
    theme::{ActiveTheme, AvatarSize},
    typography::RelativeTime,
};

/// Who did what, to what, and when: "Grace Hopper commented on Design review".
pub struct Activity {
    key: SharedString,
    actor: SharedString,
    verb: SharedString,
    object: Option<SharedString>,
    at: Timestamp,
    avatar: Option<Avatar>,
    icon: Option<IconName>,
    body: Option<AnyElement>,
}

impl Activity {
    /// `key` follows the entry as the feed changes; a key not seen before arrives with motion.
    pub fn new(
        key: impl Into<SharedString>,
        actor: impl Into<SharedString>,
        verb: impl Into<SharedString>,
        at: Timestamp,
    ) -> Self {
        Self {
            key: key.into(),
            actor: actor.into(),
            verb: verb.into(),
            object: None,
            at,
            avatar: None,
            icon: None,
            body: None,
        }
    }

    /// What it was done to, set in the actor's weight.
    pub fn object(mut self, object: impl Into<SharedString>) -> Self {
        self.object = Some(object.into());
        self
    }

    /// The actor's avatar, such as one with a picture; by default, their initials.
    pub fn avatar(mut self, avatar: Avatar) -> Self {
        self.avatar = Some(avatar);
        self
    }

    /// An icon in place of an avatar, for what a system did.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Detail below, such as the comment itself.
    pub fn body(mut self, body: impl IntoElement) -> Self {
        self.body = Some(body.into_any_element());
        self
    }
}

/// The sentence, where the actor sits in it, and where the object does.
fn sentence(
    actor: &str,
    verb: &str,
    object: Option<&str>,
) -> (String, Range<usize>, Option<Range<usize>>) {
    let text = format!("{actor} {verb}");
    let Some(object) = object else {
        return (text, 0..actor.len(), None);
    };
    let start = text.len() + 1;
    let text = format!("{text} {object}");
    let end = text.len();
    (text, 0..actor.len(), Some(start..end))
}

/// A stream of what people did, in the order given. An entry with a new key folds in; the times stay fresh.
#[derive(IntoElement)]
pub struct ActivityFeed {
    id: ElementId,
    entries: Vec<Activity>,
}

impl ActivityFeed {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            entries: Vec::new(),
        }
    }

    pub fn entry(mut self, activity: Activity) -> Self {
        self.entries.push(activity);
        self
    }
}

impl RenderOnce for ActivityFeed {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = &cx.theme().colors;
        let strong = HighlightStyle {
            color: Some(colors.fg),
            font_weight: Some(FontWeight::MEDIUM),
            ..Default::default()
        };
        let (id, last) = (self.id.clone(), self.entries.len().saturating_sub(1));
        let muted = colors.fg_muted;
        self.entries.into_iter().enumerate().fold(
            AnimatePresence::new(self.id),
            |feed, (ix, activity)| {
                let (text, actor, object) = sentence(
                    &activity.actor,
                    &activity.verb,
                    activity.object.as_ref().map(|object| object.as_ref()),
                );
                let title =
                    div()
                        .text_color(muted)
                        .child(StyledText::new(text).with_highlights(
                            iter::once(actor).chain(object).map(|range| (range, strong)),
                        ));
                let item = TimelineItem::new(title)
                    .time(RelativeTime::new(
                        (id.clone(), format!("time-{}", activity.key)),
                        activity.at,
                    ))
                    .tail(ix < last)
                    .children(activity.body);
                let item = match (activity.icon, activity.avatar) {
                    (Some(icon), _) => item.icon(icon),
                    (None, Some(avatar)) => item.marker(avatar.size(AvatarSize::Sm)),
                    (None, None) => item.marker(
                        Avatar::new(
                            (id.clone(), format!("face-{}", activity.key)),
                            activity.actor.clone(),
                        )
                        .size(AvatarSize::Sm),
                    ),
                };
                feed.row(activity.key, true, item)
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::sentence;

    #[test]
    fn an_activity_sets_its_actor_and_object_apart() {
        let (text, actor, object) = sentence("Lucía Romero", "commented on", Some("Design review"));
        assert_eq!(text, "Lucía Romero commented on Design review");
        assert_eq!(&text[actor], "Lucía Romero");
        assert_eq!(&text[object.expect("an object")], "Design review");
        let (text, actor, object) = sentence("Build bot", "deployed", None);
        assert_eq!(
            (text.as_str(), actor, object),
            ("Build bot deployed", 0..9, None)
        );
    }
}
