use gpui::{AppContext as _, Entity, Focusable, TestAppContext, VisualTestContext};

use super::{BlockData, BlockEditor, BlockKind};
use crate::{forms, theme::Theme};

mod markdown;
mod scroll;
mod span;
mod stale;

fn open(
    blocks: Vec<BlockData>,
    cx: &mut TestAppContext,
) -> (Entity<BlockEditor>, &mut VisualTestContext) {
    cx.update(|cx| {
        Theme::init(cx);
        forms::bind_keys(cx);
        crate::documents::bind_keys(cx);
    });
    let (full, cx) =
        cx.add_window_view(|window, cx| Full(cx.new(|cx| BlockEditor::new(blocks, window, cx))));
    let editor = full.read_with(cx, |full, _| full.0.clone());
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (editor, cx)
}

/// The editor as a window's content, in the box that fills the window.
struct Full(Entity<BlockEditor>);

impl gpui::Render for Full {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::{ParentElement, Styled};
        gpui::div().size_full().child(self.0.clone())
    }
}

fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

fn blocks(
    editor: &Entity<BlockEditor>,
    cx: &mut VisualTestContext,
) -> Vec<(BlockKind, Vec<String>)> {
    cx.update(|_, cx| {
        editor
            .read(cx)
            .blocks(cx)
            .into_iter()
            .map(|block| (block.kind, block.texts))
            .collect()
    })
}

/// Puts the caret in block `ix`'s first field at byte `at`.
fn caret(editor: &Entity<BlockEditor>, ix: usize, at: usize, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        let field = editor.read(cx).blocks[ix].fields[0].clone();
        field.update(cx, |field, cx| field.select(at..at, cx));
        window.focus(&field.focus_handle(cx), cx);
    });
    settle(cx);
}

fn text(kind: BlockKind, text: &str) -> (BlockKind, Vec<String>) {
    (kind, vec![text.to_string()])
}

#[gpui::test]
fn enter_splits_prose_at_the_caret(cx: &mut TestAppContext) {
    let (editor, cx) = open(vec![BlockData::new(BlockKind::Bullet, ["lift"])], cx);
    caret(&editor, 0, 2, cx);
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert_eq!(
        blocks(&editor, cx),
        [text(BlockKind::Bullet, "li"), text(BlockKind::Bullet, "ft")]
    );
}

#[gpui::test]
fn backspace_at_the_start_joins_up_and_undo_parts_them(cx: &mut TestAppContext) {
    let (editor, cx) = open(
        vec![
            BlockData::new(BlockKind::Paragraph, ["tone "]),
            BlockData::new(BlockKind::Paragraph, ["lift"]),
        ],
        cx,
    );
    caret(&editor, 1, 0, cx);
    cx.simulate_keystrokes("backspace");
    settle(cx);
    assert_eq!(
        blocks(&editor, cx),
        [text(BlockKind::Paragraph, "tone lift")]
    );
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    assert_eq!(
        blocks(&editor, cx),
        [
            text(BlockKind::Paragraph, "tone "),
            text(BlockKind::Paragraph, "lift")
        ],
        "undo brings the joined block back"
    );
}

#[gpui::test]
fn markdown_typed_at_a_paragraphs_start_changes_its_kind(cx: &mut TestAppContext) {
    let (editor, cx) = open(vec![BlockData::new(BlockKind::Paragraph, [""])], cx);
    caret(&editor, 0, 0, cx);
    cx.simulate_input("## Lift");
    settle(cx);
    assert_eq!(blocks(&editor, cx), [text(BlockKind::Heading(2), "Lift")]);
}

#[gpui::test]
fn backspace_turns_a_heading_back_into_text(cx: &mut TestAppContext) {
    let (editor, cx) = open(vec![BlockData::new(BlockKind::Heading(1), ["Lift"])], cx);
    caret(&editor, 0, 0, cx);
    cx.simulate_keystrokes("backspace");
    settle(cx);
    assert_eq!(blocks(&editor, cx), [text(BlockKind::Paragraph, "Lift")]);
}

#[gpui::test]
fn a_press_in_a_blocks_text_puts_the_caret_there(cx: &mut TestAppContext) {
    let (editor, cx) = open(vec![BlockData::new(BlockKind::Paragraph, ["lift"])], cx);
    cx.simulate_click(
        gpui::point(gpui::px(300.0), gpui::px(10.0)),
        gpui::Modifiers::none(),
    );
    settle(cx);
    let focused = cx.update(|window, cx| editor.read(cx).focused(window, cx));
    assert!(focused.is_some(), "the press focuses the block's field");
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert_eq!(blocks(&editor, cx).len(), 2);
}

/// The editor at a fixed width inside a row, as a page holds it.
struct Hosted(Entity<BlockEditor>);

impl gpui::Render for Hosted {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::{ParentElement, Styled};
        gpui::div()
            .flex()
            .child(gpui::div().w(gpui::px(760.0)).child(self.0.clone()))
    }
}

#[gpui::test]
fn a_press_past_the_words_still_lands_in_the_block(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        forms::bind_keys(cx);
        crate::documents::bind_keys(cx);
    });
    let (host, cx) = cx.add_window_view(|window, cx| {
        let editor = cx.new(|cx| {
            BlockEditor::new(
                vec![BlockData::new(BlockKind::Paragraph, ["lift"])],
                window,
                cx,
            )
        });
        Hosted(editor)
    });
    settle(cx);
    cx.simulate_click(
        gpui::point(gpui::px(700.0), gpui::px(10.0)),
        gpui::Modifiers::none(),
    );
    settle(cx);
    let editor = cx.update(|_, cx| host.read(cx).0.clone());
    let focused = cx.update(|window, cx| editor.read(cx).focused(window, cx));
    assert!(focused.is_some(), "the press focuses the block's field");
}

#[gpui::test]
fn a_press_beside_a_blocks_text_puts_the_caret_at_its_end(cx: &mut TestAppContext) {
    let (editor, cx) = open(vec![BlockData::new(BlockKind::Paragraph, ["lift"])], cx);
    cx.simulate_click(
        gpui::point(gpui::px(300.0), gpui::px(1.0)),
        gpui::Modifiers::none(),
    );
    settle(cx);
    let focused = cx.update(|window, cx| editor.read(cx).focused(window, cx));
    assert_eq!(
        focused,
        Some((editor.read_with(cx, |editor, _| editor.blocks[0].key), 0))
    );
}

/// Puts block `ix`'s field `field` in focus with `range` selected.
fn select(
    editor: &Entity<BlockEditor>,
    ix: usize,
    field: usize,
    range: std::ops::Range<usize>,
    cx: &mut VisualTestContext,
) {
    cx.update(|window, cx| {
        let field = editor.read(cx).blocks[ix].fields[field].clone();
        field.update(cx, |field, cx| field.select(range, cx));
        window.focus(&field.focus_handle(cx), cx);
    });
    settle(cx);
}

#[gpui::test]
fn enter_over_a_selection_drops_it_and_splits_there(cx: &mut TestAppContext) {
    let (editor, cx) = open(vec![BlockData::new(BlockKind::Paragraph, ["abcdef"])], cx);
    select(&editor, 0, 0, 2..4, cx);
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert_eq!(
        blocks(&editor, cx),
        [
            text(BlockKind::Paragraph, "ab"),
            text(BlockKind::Paragraph, "ef")
        ]
    );
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    assert_eq!(
        blocks(&editor, cx),
        [text(BlockKind::Paragraph, "abcdef")],
        "one step undoes it"
    );
}

#[gpui::test]
fn undo_puts_the_caret_back_where_the_change_began(cx: &mut TestAppContext) {
    let (editor, cx) = open(vec![BlockData::new(BlockKind::Paragraph, ["abcdef"])], cx);
    caret(&editor, 0, 3, cx);
    cx.simulate_keystrokes("enter");
    settle(cx);
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    let first = cx.update(|_, cx| editor.read(cx).blocks[0].key);
    let focused = cx.update(|window, cx| editor.read(cx).focused(window, cx));
    assert_eq!(focused, Some((first, 0)));
    cx.simulate_keystrokes("secondary-shift-z");
    settle(cx);
    assert_eq!(
        blocks(&editor, cx),
        [
            text(BlockKind::Paragraph, "abc"),
            text(BlockKind::Paragraph, "def")
        ],
        "redo splits again"
    );
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    cx.simulate_input("x");
    settle(cx);
    assert_eq!(
        blocks(&editor, cx),
        [text(BlockKind::Paragraph, "abcxdef")],
        "typing lands at the caret undo put back"
    );
}

#[gpui::test]
fn keys_into_a_merged_cell_land_on_the_cell_that_shows(cx: &mut TestAppContext) {
    let table = BlockKind::Table {
        columns: 2,
        merged: vec![(1, 1)],
    };
    let (editor, cx) = open(vec![BlockData::new(table, ["A", "B", "c", ""])], cx);
    select(&editor, 0, 0, 1..1, cx);
    cx.update(|window, cx| {
        let field = editor.read(cx).blocks[0].fields[1].clone();
        window.focus(&field.focus_handle(cx), cx);
    });
    settle(cx);
    cx.simulate_keystrokes("down");
    settle(cx);
    let key = cx.update(|_, cx| editor.read(cx).blocks[0].key);
    let focused = cx.update(|window, cx| editor.read(cx).focused(window, cx));
    assert_eq!(
        focused,
        Some((key, 2)),
        "down from B lands on the cell c spans"
    );
}

#[gpui::test]
fn undoing_a_kind_change_brings_back_its_styling(cx: &mut TestAppContext) {
    let (editor, cx) = open(vec![BlockData::new(BlockKind::Paragraph, ["**lift**"])], cx);
    cx.update(|window, cx| {
        let key = editor.read(cx).blocks[0].key;
        editor.update(cx, |editor, cx| {
            editor.turn_into(key, BlockKind::Code, window, cx)
        });
    });
    settle(cx);
    caret(&editor, 0, 0, cx);
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    let bold = cx.update(|_, cx| {
        let field = editor.read(cx).blocks[0].fields[0].clone();
        field
            .read(cx)
            .highlights(cx)
            .iter()
            .any(|(_, highlight)| highlight.weight.is_some())
    });
    assert_eq!(
        blocks(&editor, cx),
        [text(BlockKind::Paragraph, "**lift**")]
    );
    assert!(bold, "the paragraph styles its markdown again");
}

#[gpui::test]
fn undo_of_typing_restores_the_selection_it_replaced(cx: &mut TestAppContext) {
    let (editor, cx) = open(vec![BlockData::new(BlockKind::Paragraph, ["abcdef"])], cx);
    select(&editor, 0, 0, 2..4, cx);
    cx.simulate_keystrokes("x");
    settle(cx);
    assert_eq!(blocks(&editor, cx), [text(BlockKind::Paragraph, "abxef")]);
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    let selection = cx.update(|_, cx| editor.read(cx).blocks[0].fields[0].read(cx).selection());
    assert_eq!(blocks(&editor, cx), [text(BlockKind::Paragraph, "abcdef")]);
    assert_eq!(selection, 2..4, "undo selects what the typing replaced");
}

#[gpui::test]
fn up_into_a_table_skips_its_hidden_cells(cx: &mut TestAppContext) {
    let table = BlockKind::Table {
        columns: 2,
        merged: vec![(1, 1)],
    };
    let (editor, cx) = open(
        vec![
            BlockData::new(table, ["A", "B", "c", ""]),
            BlockData::new(BlockKind::Paragraph, ["after"]),
        ],
        cx,
    );
    caret(&editor, 1, 0, cx);
    cx.simulate_keystrokes("up");
    settle(cx);
    let key = cx.update(|_, cx| editor.read(cx).blocks[0].key);
    let focused = cx.update(|window, cx| editor.read(cx).focused(window, cx));
    assert_eq!(
        focused,
        Some((key, 2)),
        "up lands on the cell that shows in the last row"
    );
}

#[gpui::test]
fn undo_of_a_delete_or_a_paste_restores_the_selection(cx: &mut TestAppContext) {
    let (editor, cx) = open(vec![BlockData::new(BlockKind::Paragraph, ["abcdef"])], cx);
    let selection = |cx: &mut VisualTestContext| {
        cx.update(|_, cx| editor.read(cx).blocks[0].fields[0].read(cx).selection())
    };
    select(&editor, 0, 0, 2..4, cx);
    cx.simulate_keystrokes("backspace");
    settle(cx);
    assert_eq!(blocks(&editor, cx), [text(BlockKind::Paragraph, "abef")]);
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    assert_eq!(selection(cx), 2..4, "undo of a delete selects what it took");
    cx.update(|_, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string("xy".into())));
    select(&editor, 0, 0, 2..4, cx);
    cx.simulate_keystrokes("secondary-v");
    settle(cx);
    assert_eq!(blocks(&editor, cx), [text(BlockKind::Paragraph, "abxyef")]);
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    assert_eq!(
        selection(cx),
        2..4,
        "undo of a paste selects what it replaced"
    );
}
