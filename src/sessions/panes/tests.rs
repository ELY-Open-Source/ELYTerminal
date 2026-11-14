use gpui::{Axis, EntityId};

use super::Layout;

fn id(n: u64) -> EntityId {
    EntityId::from(n << 32 | 1)
}

#[test]
fn a_split_puts_the_new_pane_after_its_own_and_nests() {
    let mut layout = Layout::Pane(id(1));
    assert!(layout.split(id(1), Axis::Horizontal, id(2)));
    assert!(layout.split(id(2), Axis::Vertical, id(3)));
    assert!(!layout.split(id(9), Axis::Vertical, id(4)), "no such pane");
    assert_eq!(layout.order(), [id(1), id(2), id(3)]);
    let Layout::Split(Axis::Horizontal, _, right) = &layout else {
        panic!("{layout:?}");
    };
    assert!(matches!(right.as_ref(), Layout::Split(Axis::Vertical, ..)));
}

#[test]
fn a_pane_removed_leaves_its_sibling_in_its_place() {
    let mut layout = Layout::Pane(id(1));
    layout.split(id(1), Axis::Horizontal, id(2));
    layout.split(id(2), Axis::Vertical, id(3));
    assert!(layout.remove(id(2)));
    assert_eq!(
        layout,
        Layout::Split(
            Axis::Horizontal,
            Box::new(Layout::Pane(id(1))),
            Box::new(Layout::Pane(id(3)))
        )
    );
    assert!(layout.remove(id(1)));
    assert_eq!(layout, Layout::Pane(id(3)));
    assert!(!layout.remove(id(3)), "the last pane stays");
    assert!(!layout.remove(id(7)));
}
