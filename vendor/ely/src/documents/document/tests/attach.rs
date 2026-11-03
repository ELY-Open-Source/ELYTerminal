use std::{cell::RefCell, path::PathBuf, rc::Rc};

use gpui::{
    ClipboardItem, Entity, ExternalPaths, FileDropEvent, Image, ImageFormat, Modifiers,
    TestAppContext, VisualTestContext, point, px,
};

use super::{MarkdownDocument, MarkdownMode, open, opened, press, text};
use crate::documents::Attachment;

/// Attachments the document hands its owner.
fn heard(
    document: &Entity<MarkdownDocument>,
    cx: &mut VisualTestContext,
) -> Rc<RefCell<Vec<Attachment>>> {
    let heard = Rc::new(RefCell::new(Vec::new()));
    let kept = heard.clone();
    cx.update(|_, cx| {
        cx.subscribe(document, move |_, attached: &Attachment, _| {
            kept.borrow_mut().push(attached.clone())
        })
        .detach()
    });
    heard
}

fn picture() -> Image {
    Image::from_bytes(ImageFormat::Png, vec![1, 2, 3])
}

#[gpui::test]
fn dropped_files_go_to_the_owner(cx: &mut TestAppContext) {
    let (document, cx) = open("a\n", MarkdownMode::Visual, cx);
    let heard = heard(&document, cx);
    let at = point(px(100.0), px(100.0));
    let paths = ExternalPaths(vec![PathBuf::from("shot.png")].into());
    cx.simulate_event(FileDropEvent::Entered {
        position: at,
        paths,
    });
    cx.simulate_event(FileDropEvent::Submit { position: at });
    cx.run_until_parked();
    assert_eq!(
        *heard.borrow(),
        [Attachment::Files(vec![PathBuf::from("shot.png")])]
    );
}

#[gpui::test]
fn a_pasted_picture_goes_to_the_owner_and_text_pastes(cx: &mut TestAppContext) {
    let (document, cx) = open("a\n", MarkdownMode::Visual, cx);
    let heard = heard(&document, cx);
    press(0, Modifiers::none(), cx);
    cx.write_to_clipboard(ClipboardItem::new_image(&picture()));
    cx.simulate_keystrokes("secondary-v");
    assert_eq!(*heard.borrow(), [Attachment::Picture(picture())]);
    assert_eq!(opened(&document, cx), "a", "the picture typed nothing");
    cx.write_to_clipboard(ClipboardItem::new_string("b".into()));
    cx.simulate_keystrokes("secondary-v");
    assert_eq!(opened(&document, cx), "ab", "text pastes as text");
}

#[gpui::test]
fn text_goes_in_at_the_caret_but_not_in_preview(cx: &mut TestAppContext) {
    let (document, cx) = open("ab\n", MarkdownMode::Source, cx);
    document.update(cx, |document, cx| {
        document.set_caret(0, 1, cx);
        assert!(document.insert("X", cx));
    });
    assert_eq!(text(&document, cx), "aXb\n");
    document.update_in(cx, |document, window, cx| {
        document.set_mode(MarkdownMode::Preview, window, cx)
    });
    assert!(!document.update(cx, |document, cx| document.insert("Y", cx)));
    assert_eq!(text(&document, cx), "aXb\n");
}
