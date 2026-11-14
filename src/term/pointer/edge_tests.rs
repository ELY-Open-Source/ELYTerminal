use gpui::px;

use super::overflow;

#[test]
fn a_drag_inside_the_grid_scrolls_nothing() {
    assert_eq!(overflow(px(0.), px(16.), 24), 0);
    assert_eq!(overflow(px(383.9), px(16.), 24), 0);
}

#[test]
fn a_drag_above_the_grid_scrolls_up_faster_the_farther_it_lies() {
    assert_eq!(overflow(px(-0.5), px(16.), 24), 1);
    assert_eq!(overflow(px(-16.), px(16.), 24), 2);
    assert_eq!(overflow(px(-40.), px(16.), 24), 3);
}

#[test]
fn a_drag_below_the_grid_scrolls_down() {
    assert_eq!(overflow(px(384.), px(16.), 24), -1);
    assert_eq!(overflow(px(400.), px(16.), 24), -2);
}

#[test]
fn an_unmeasured_cell_scrolls_nothing() {
    assert_eq!(overflow(px(-40.), px(0.), 24), 0);
}
