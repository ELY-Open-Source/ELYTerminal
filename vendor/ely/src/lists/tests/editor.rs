use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, Render, Styled, TestAppContext,
    Window, div, px,
};

use super::{settle, setup};
use crate::lists::{List, ListItem};

/// Two rows, the first renamed in place.
struct Renaming;

impl Render for Renaming {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let field = div()
            .debug_selector(|| "row-editor".into())
            .h(px(12.0))
            .w_full();
        div().w(px(240.0)).child(
            List::new()
                .child(
                    ListItem::new("renamed", "Old name")
                        .description("~/work")
                        .selected(true)
                        .editor(field),
                )
                .child(ListItem::new("other", "Other").description("~/other")),
        )
    }
}

#[gpui::test]
fn an_editor_takes_the_title_s_place_in_its_row(cx: &mut TestAppContext) {
    setup(cx);
    let (_, cx) = cx.add_window_view(|_, _| Renaming);
    settle(cx);
    let row = cx
        .debug_bounds("item-selected-Old name")
        .expect("the renamed row");
    let field = cx.debug_bounds("row-editor").expect("its editor");
    let line = cx
        .debug_bounds("item-description-~/work")
        .expect("its description");
    assert!(row.contains(&field.center()), "{field:?} in {row:?}");
    assert!(
        field.bottom() <= line.top(),
        "the editor stands over the description"
    );
    assert!(cx.debug_bounds("item-description-~/other").is_some());
}
