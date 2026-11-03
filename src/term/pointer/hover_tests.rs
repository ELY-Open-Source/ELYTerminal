use super::*;
use gpui::{point, px, size};

#[test]
fn hover_uses_only_cells_inside_the_grid() {
    let pointer = Pointer {
        origin: point(px(10.), px(20.)),
        cell: size(px(8.), px(16.)),
        ..Pointer::default()
    };
    let grid = (12, 4);
    assert_eq!(pointer.hovered(point(px(10.), px(20.)), grid), Some((0, 0)));
    assert_eq!(pointer.hovered(point(px(25.), px(51.)), grid), Some((1, 1)));
    for position in [
        point(px(9.), px(20.)),
        point(px(10.), px(19.)),
        point(px(106.), px(20.)),
        point(px(10.), px(84.)),
    ] {
        assert_eq!(pointer.hovered(position, grid), None);
    }
    assert_eq!(
        Pointer::default().hovered(point(px(0.), px(0.)), grid),
        None
    );
}
