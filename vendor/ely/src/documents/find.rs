use gpui::{Pixels, ScrollHandle, point};

use super::viewer::PageHit;
use crate::editor::FindWidget;

/// The owner's find box over the pages, the hits it found and the current one.
pub(crate) struct Find {
    pub widget: FindWidget,
    pub hits: Vec<PageHit>,
    pub current: Option<usize>,
}

/// Scrolls `hit` a third of the way down the view; runs after the pages' prepaint, so their bounds are this frame's.
pub(crate) fn reveal(scroll: &ScrollHandle, hit: &PageHit, zoom: f32) {
    let page = scroll
        .bounds_for_item(hit.page)
        .expect("the hit's page is laid out");
    let view = scroll.bounds();
    let target = page.top() + Pixels::from(hit.bounds.origin.y * zoom);
    let offset = scroll.offset();
    scroll.set_offset(point(
        offset.x,
        view.top() + view.size.height / 3.0 - target,
    ));
}
