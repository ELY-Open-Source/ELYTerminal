use gpui::{Axis, EntityId};

use super::{Layout, Toward};

fn id(n: u64) -> EntityId {
    EntityId::from(1 << 32 | n)
}

#[test]
fn a_split_puts_the_new_pane_after_its_own_and_nests() {
    let mut layout = Layout::Pane(id(1));
    assert!(layout.split(id(1), Axis::Horizontal, id(2)));
    assert!(layout.split(id(2), Axis::Vertical, id(3)));
    assert!(!layout.split(id(9), Axis::Vertical, id(4)), "no such pane");
    assert_eq!(layout.order(), [id(1), id(2), id(3)]);
    let Layout::Split(Axis::Horizontal, _, _, right) = &layout else {
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
            0.5,
            Box::new(Layout::Pane(id(1))),
            Box::new(Layout::Pane(id(3)))
        )
    );
    assert!(layout.remove(id(1)));
    assert_eq!(layout, Layout::Pane(id(3)));
    assert!(!layout.remove(id(3)), "the last pane stays");
    assert!(!layout.remove(id(7)));
}

#[test]
fn a_split_keeps_the_share_a_drag_gives_it() {
    let mut layout = Layout::Pane(id(1));
    layout.split(id(1), Axis::Horizontal, id(2));
    layout.split(id(2), Axis::Vertical, id(3));
    assert!(layout.resize((id(2), id(3)), 0.3), "the inner split");
    assert!(layout.resize((id(1), id(2)), 0.7), "the outer one");
    assert!(!layout.resize((id(1), id(3)), 0.2), "no split starts so");
    let Layout::Split(_, 0.7, _, right) = &layout else {
        panic!("{layout:?}");
    };
    assert!(matches!(
        right.as_ref(),
        Layout::Split(Axis::Vertical, 0.3, ..)
    ));
}

#[test]
fn a_pane_trades_places_with_its_neighbor_toward_a_side() {
    // 1 on the left; 2 over 3 on the right, 2 the upper third.
    let mut layout = Layout::Pane(id(1));
    layout.split(id(1), Axis::Horizontal, id(2));
    layout.split(id(2), Axis::Vertical, id(3));
    layout.resize((id(2), id(3)), 1.0 / 3.0);
    assert_eq!(
        layout.neighbor(id(1), Toward::Right),
        Some(id(3)),
        "most alongside"
    );
    assert_eq!(layout.neighbor(id(3), Toward::Left), Some(id(1)));
    assert_eq!(layout.neighbor(id(2), Toward::Down), Some(id(3)));
    assert_eq!(layout.neighbor(id(3), Toward::Up), Some(id(2)));
    assert_eq!(layout.neighbor(id(1), Toward::Left), None, "the edge");
    assert_eq!(layout.neighbor(id(1), Toward::Up), None);
    layout.swap(id(1), id(3));
    assert_eq!(layout.order(), [id(3), id(2), id(1)]);
}
