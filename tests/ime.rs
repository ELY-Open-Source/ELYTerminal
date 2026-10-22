#![cfg(target_os = "linux")]

use xim_ctext::{compound_text_to_utf8, utf8_to_compound_text};

#[test]
fn chinese_xim_commit_decodes_without_panicking() {
    let text = compound_text_to_utf8(b"\x1b$(AVPND\x1b(B").expect("Encoding Error");
    assert_eq!(text, "中文");
}

#[test]
fn chinese_xim_commit_preserves_ascii_segments() {
    let text = compound_text_to_utf8(b"abc\x1b$(AVPND\x1b(B123").unwrap();
    assert_eq!(text, "abc中文123");
}

#[test]
fn utf8_xim_commit_preserves_chinese_and_emoji() {
    let text = "中文 English 😀";
    assert_eq!(
        compound_text_to_utf8(&utf8_to_compound_text(text)).unwrap(),
        text
    );
}

#[test]
fn plain_ascii_xim_commit_is_unchanged() {
    assert_eq!(compound_text_to_utf8(b"hello 123").unwrap(), "hello 123");
}
