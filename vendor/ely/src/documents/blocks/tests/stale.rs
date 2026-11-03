use gpui::{Focusable, TestAppContext};

use super::{blocks, caret, open, settle, text};
use crate::{
    documents::blocks::{Align, BlockData, BlockKind, Media},
    forms,
};

#[gpui::test]
fn a_key_the_user_deleted_changes_nothing(cx: &mut TestAppContext) {
    let paragraph = |text: &str| BlockData::new(BlockKind::Paragraph, [text]);
    let (editor, cx) = open(vec![paragraph("one"), paragraph("two")], cx);
    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let gone = 999;
            editor.set_kind(gone, BlockKind::Bullet, cx);
            editor.turn_into(gone, BlockKind::Bullet, window, cx);
            editor.remove(gone, window, cx);
            editor.duplicate(gone, window, cx);
            editor.move_block(9, 0, cx);
            editor.insert(Some(gone), paragraph("three"), window, cx);
        })
    });
    let texts: Vec<String> = editor.read_with(cx, |editor, cx| {
        editor
            .blocks(cx)
            .into_iter()
            .map(|block| block.texts[0].clone())
            .collect()
    });
    assert_eq!(
        texts,
        ["one", "two", "three"],
        "an insert after a gone block lands at the end"
    );
}

#[gpui::test]
fn keys_in_a_field_whose_block_left_change_nothing(cx: &mut TestAppContext) {
    let paragraph = |text: &str| BlockData::new(BlockKind::Paragraph, [text]);
    let (editor, cx) = open(vec![paragraph("one"), paragraph("two")], cx);
    caret(&editor, 0, 0, cx);
    cx.update(|window, cx| {
        let key = editor.read(cx).blocks[0].key;
        editor.update(cx, |editor, cx| editor.remove(key, window, cx));
        window.dispatch_action(Box::new(forms::Enter), cx);
    });
    settle(cx);
    assert_eq!(blocks(&editor, cx), [text(BlockKind::Paragraph, "two")]);
}

#[gpui::test]
fn up_from_a_body_folded_away_changes_nothing(cx: &mut TestAppContext) {
    let toggle = BlockData::new(BlockKind::Toggle(true), ["title", "body"]);
    let (editor, cx) = open(vec![toggle], cx);
    cx.update(|window, cx| {
        let body = editor.read(cx).blocks[0].fields[1].clone();
        window.focus(&body.focus_handle(cx), cx);
    });
    settle(cx);
    cx.update(|window, cx| {
        let key = editor.read(cx).blocks[0].key;
        editor.update(cx, |editor, cx| {
            editor.set_kind(key, BlockKind::Toggle(false), cx)
        });
        window.dispatch_action(Box::new(forms::Up), cx);
    });
    settle(cx);
    assert_eq!(blocks(&editor, cx)[0].0, BlockKind::Toggle(false));
}

#[gpui::test]
fn table_and_media_tools_on_a_block_turned_paragraph_change_nothing(cx: &mut TestAppContext) {
    let table = BlockKind::Table {
        columns: 2,
        merged: vec![],
    };
    let image = BlockKind::Image(Media::new("missing.png"));
    let (editor, cx) = open(
        vec![
            BlockData::new(table, ["h1", "h2", "a", "b"]),
            BlockData::new(image, ["caption"]),
        ],
        cx,
    );
    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let (table, image) = (editor.blocks[0].key, editor.blocks[1].key);
            editor.turn_into(table, BlockKind::Paragraph, window, cx);
            editor.turn_into(image, BlockKind::Paragraph, window, cx);
            editor.add_row(table, window, cx);
            editor.add_column(table, window, cx);
            editor.remove_line(table, 0, true, cx);
            editor.merge_right(table, 0, cx);
            editor.align(image, Align::Left, cx);
        })
    });
    settle(cx);
    let kinds: Vec<BlockKind> = blocks(&editor, cx)
        .into_iter()
        .map(|(kind, _)| kind)
        .collect();
    assert_eq!(kinds, [BlockKind::Paragraph, BlockKind::Paragraph]);
}

#[gpui::test]
fn a_cell_whose_row_left_is_not_removed_or_merged_again(cx: &mut TestAppContext) {
    let table = BlockKind::Table {
        columns: 2,
        merged: vec![],
    };
    let cells = ["h1", "h2", "a", "b", "c", "d", "e", "f"];
    let (editor, cx) = open(vec![BlockData::new(table, cells)], cx);
    cx.update(|_, cx| {
        editor.update(cx, |editor, cx| {
            let key = editor.blocks[0].key;
            editor.remove_line(key, 6, false, cx);
            editor.remove_line(key, 6, false, cx);
            editor.merge_right(key, 6, cx);
        })
    });
    settle(cx);
    let texts: Vec<String> = blocks(&editor, cx).remove(0).1;
    assert_eq!(texts, ["h1", "h2", "a", "b", "c", "d"]);
}

#[gpui::test]
fn a_drag_on_a_picture_turned_paragraph_leaves_undo_alone(cx: &mut TestAppContext) {
    let image = BlockKind::Image(Media::new("missing.png"));
    let (editor, cx) = open(vec![BlockData::new(image.clone(), ["caption"])], cx);
    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let key = editor.blocks[0].key;
            editor.turn_into(key, BlockKind::Paragraph, window, cx);
            editor.resize(key, 0.5, true, cx);
            editor.undo(window, cx);
        })
    });
    settle(cx);
    assert_eq!(blocks(&editor, cx)[0].0, image);
}
