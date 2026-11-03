use gpui::SharedString;

use super::view::Frame;

/// A topic of a mind map, with the topics under it.
#[derive(Clone, Debug, PartialEq)]
pub struct Topic {
    pub key: SharedString,
    pub text: SharedString,
    pub children: Vec<Topic>,
}

impl Topic {
    pub fn new(key: impl Into<SharedString>, text: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            text: text.into(),
            children: Vec::new(),
        }
    }

    pub fn child(mut self, topic: Topic) -> Self {
        self.children.push(topic);
        self
    }

    /// The map with `topic` added last under `parent`; none, logged, when the map holds no `parent`.
    pub fn adding(&self, parent: &str, topic: Topic) -> Option<Topic> {
        let mut next = self.clone();
        let Some(under) = next.find(parent) else {
            log::error!("mind map: no topic {parent} to add under");
            return None;
        };
        under.children.push(topic);
        Some(next)
    }

    /// The map without `key` and what lies under it; none, logged, for the root or a topic the map lacks.
    pub fn removing(&self, key: &str) -> Option<Topic> {
        let mut next = self.clone();
        let Some(parent) = next.parent_of(key) else {
            log::error!("mind map: no topic {key} below the root to remove");
            return None;
        };
        next.find(&parent)
            .expect("the parent is in the map")
            .children
            .retain(|topic| topic.key != key);
        Some(next)
    }

    /// The map with `key` reading `text`; none, logged, when the map holds no `key`.
    pub fn renaming(&self, key: &str, text: impl Into<SharedString>) -> Option<Topic> {
        let mut next = self.clone();
        let Some(topic) = next.find(key) else {
            log::error!("mind map: no topic {key} to rename");
            return None;
        };
        topic.text = text.into();
        Some(next)
    }

    fn find(&mut self, key: &str) -> Option<&mut Topic> {
        if self.key == key {
            return Some(self);
        }
        self.children.iter_mut().find_map(|topic| topic.find(key))
    }

    /// The key of the topic `key` sits under.
    pub fn parent_of(&self, key: &str) -> Option<SharedString> {
        self.children.iter().find_map(|topic| {
            if topic.key == key {
                Some(self.key.clone())
            } else {
                topic.parent_of(key)
            }
        })
    }
}

/// A topic's box, in canvas units.
pub(crate) const TOPIC: (f32, f32) = (160.0, 36.0);
/// Between a topic and its children's column, and between siblings, in canvas units.
const GAP: (f32, f32) = (56.0, 12.0);

/// A topic laid out: its key, its frame, the key it sits under, and whether it grows to the left.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Placed {
    pub key: SharedString,
    pub text: SharedString,
    pub frame: Frame,
    pub parent: Option<SharedString>,
    pub left: bool,
}

fn tall(topic: &Topic) -> f32 {
    let children: f32 = topic.children.iter().map(tall).sum::<f32>()
        + GAP.1 * topic.children.len().saturating_sub(1) as f32;
    children.max(TOPIC.1)
}

/// Every topic laid out: the root centered on the origin, the first half of its children to its right and the rest to its left, each topic's children a column beside it centered on it.
pub(crate) fn laid(root: &Topic) -> Vec<Placed> {
    let frame = Frame::new(-TOPIC.0 / 2.0, -TOPIC.1 / 2.0, TOPIC.0, TOPIC.1);
    let mut out = vec![Placed {
        key: root.key.clone(),
        text: root.text.clone(),
        frame,
        parent: None,
        left: false,
    }];
    let right = root.children.len().div_ceil(2);
    let (east, west) = root.children.split_at(right);
    column(east, &root.key, &frame, false, &mut out);
    column(west, &root.key, &frame, true, &mut out);
    out
}

fn column(
    topics: &[Topic],
    parent: &SharedString,
    beside: &Frame,
    left: bool,
    out: &mut Vec<Placed>,
) {
    let total: f32 =
        topics.iter().map(tall).sum::<f32>() + GAP.1 * topics.len().saturating_sub(1) as f32;
    let x = if left {
        beside.x - GAP.0 - TOPIC.0
    } else {
        beside.right() + GAP.0
    };
    let mut top = beside.center().1 - total / 2.0;
    for topic in topics {
        let room = tall(topic);
        let frame = Frame::new(x, top + (room - TOPIC.1) / 2.0, TOPIC.0, TOPIC.1);
        out.push(Placed {
            key: topic.key.clone(),
            text: topic.text.clone(),
            frame,
            parent: Some(parent.clone()),
            left,
        });
        column(&topic.children, &topic.key, &frame, left, out);
        top += room + GAP.1;
    }
}

/// The topic an arrow key reaches from `at`: Up and Down walk its siblings, and the key toward its children goes to the first of them, the other key back to its parent.
pub(crate) fn stepped(placed: &[Placed], at: &str, key: &str) -> Option<SharedString> {
    let here = placed.iter().find(|topic| topic.key == at)?;
    let outward = |left: bool| {
        placed
            .iter()
            .find(|topic| {
                topic
                    .parent
                    .as_ref()
                    .is_some_and(|parent| parent.as_ref() == at)
                    && topic.left == left
            })
            .map(|topic| topic.key.clone())
    };
    let siblings: Vec<&Placed> = placed
        .iter()
        .filter(|topic| topic.parent == here.parent && topic.left == here.left)
        .collect();
    let ix = siblings.iter().position(|topic| topic.key == at)?;
    match (key, here.parent.is_some(), here.left) {
        ("up", true, _) => ix.checked_sub(1).map(|ix| siblings[ix].key.clone()),
        ("down", true, _) => siblings.get(ix + 1).map(|topic| topic.key.clone()),
        ("right", false, _) => outward(false),
        ("left", false, _) => outward(true),
        ("right", true, false) | ("left", true, true) => outward(here.left),
        ("left", true, false) | ("right", true, true) => here.parent.clone(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{GAP, TOPIC, Topic, laid, stepped};

    fn map() -> Topic {
        Topic::new("root", "Launch")
            .child(
                Topic::new("a", "Site")
                    .child(Topic::new("a1", "Copy"))
                    .child(Topic::new("a2", "Pictures")),
            )
            .child(Topic::new("b", "Press"))
            .child(Topic::new("c", "Budget"))
    }

    #[test]
    fn half_the_children_go_right_and_each_column_centers_on_its_parent() {
        let placed = laid(&map());
        let at = |key: &str| {
            placed
                .iter()
                .find(|topic| topic.key == key)
                .expect("placed")
        };
        assert!(!at("a").left && !at("b").left && at("c").left);
        assert_eq!(at("a").frame.x, TOPIC.0 / 2.0 + GAP.0);
        assert_eq!(at("c").frame.right(), -TOPIC.0 / 2.0 - GAP.0);
        assert_eq!(
            at("c").frame.center().1,
            0.0,
            "a lone child sits level with its parent"
        );
        let (a1, a2) = (at("a1").frame.center().1, at("a2").frame.center().1);
        assert_eq!((a1 + a2) / 2.0, at("a").frame.center().1);
        assert_eq!(a2 - a1, TOPIC.1 + GAP.1);
    }

    #[test]
    fn arrows_walk_siblings_and_step_in_and_out() {
        let placed = laid(&map());
        let step = |at: &str, key: &str| stepped(&placed, at, key).map(|key| key.to_string());
        assert_eq!(step("root", "right").as_deref(), Some("a"));
        assert_eq!(step("root", "left").as_deref(), Some("c"));
        assert_eq!(step("a", "down").as_deref(), Some("b"));
        assert_eq!(step("a", "right").as_deref(), Some("a1"));
        assert_eq!(step("a1", "left").as_deref(), Some("a"));
        assert_eq!(step("c", "right").as_deref(), Some("root"));
        assert_eq!(step("b", "down"), None);
    }

    #[test]
    fn edits_add_remove_and_rename_by_key() {
        let map = map()
            .adding("b", Topic::new("b1", "Release"))
            .and_then(|map| map.renaming("c", "Money"))
            .expect("both topics are in the map");
        assert_eq!(map.children[1].children[0].key, "b1");
        assert_eq!(map.children[2].text, "Money");
        assert_eq!(map.adding("gone", Topic::new("x", "X")), None);
        assert_eq!(map.renaming("gone", "X"), None);
        assert_eq!(map.removing(&map.key), None, "the root stays");
        assert_eq!(map.removing("gone"), None);
        let map = map.removing("a").expect("a is in the map");
        assert_eq!(
            map.children
                .iter()
                .map(|topic| topic.key.as_ref())
                .collect::<Vec<_>>(),
            ["b", "c"]
        );
    }
}
