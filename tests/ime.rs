#![cfg(target_os = "linux")]
//! Compound text as libX11 writes it for ibus-x11 and fcitx5.

use xim_ctext::{DecodeError, compound_text_to_utf8};

fn assert_decodes(cases: &[(&[u8], &str)]) {
    for (bytes, text) in cases {
        let decoded = compound_text_to_utf8(bytes);
        assert_eq!(
            decoded.as_deref().ok(),
            Some(*text),
            "{bytes:x?}: {decoded:x?}"
        );
    }
}

#[test]
fn simplified_chinese_decodes_as_typed() {
    assert_decodes(&[
        (b"\x1b$(AVPND", "中文"),
        (b"abc\x1b$(AVPND\x1b(B123", "abc中文123"),
        (b"\x1b$(AVP\x1b(B \x1b$(AND", "中 文"),
        (b"\x1b$(A:\x5c8_PKHOJ6Dc", "很高兴认识你"),
        (
            b"2026\x1b$(ADj\x1b(B07\x1b$(ATB\x1b(B16\x1b$(AHU",
            "2026年07月16日",
        ),
    ]);
}

#[test]
fn traditional_chinese_and_cantonese_decode_as_typed() {
    assert_decodes(&[
        (b"\x1b$(ADc:C\x1b$(D5|", "你好嗎"),
        (b"abc\x1b$(D5|\x1b(B123", "abc嗎123"),
        (
            b"\x1b%/2\x80\x89big5-0\x02\xca\x5c\x1b%/2\x80\x8agbk-0\x02\x86O\x86\xd5\x1b$(A6H",
            "佢哋喺度",
        ),
        (b"\x1b%/2\x80\x8bbig5-0\x02\xa4\xa4\xa4\xe5", "中文"),
    ]);
}

#[test]
fn sets_hold_across_utf8_and_extended_segments() {
    assert_decodes(&[
        (
            b"\x1b$(D<=\x1b$(ANR\x1b%/2\x80\x89big5-0\x02\xb0\xf7SC",
            "幫我夠用",
        ),
        (b"\x1b$(AVP\x1b%G\xf0\x9f\x98\x80\x1b%@ND", "中😀文"),
        (b"\x1b%G\xe6\x9d\xb1\x1b%@\x1b$(A>)", "東京"),
        (
            b"\x1b$(AVPND\x1b(B English \x1b%G\xf0\x9f\x98\x80\x1b%@",
            "中文 English 😀",
        ),
        (b"\xe9\x1b-T\xc0\x1b-A\xe9", "éภé"),
    ]);
}

#[test]
fn other_scripts_decode_as_typed() {
    assert_decodes(&[
        (b"hello 123", "hello 123"),
        (b"\x1b$(BEl5~", "東京"),
        (b"\x1b$(BEl\x1b$(A>)", "東京"),
        (b"\x1b$(AV*$C$F$k$D$b$j", "知ってるつもり"),
        (b"\x1b$(B_ZS(\x1b$(ADc", "炸哦你"),
        (b"\x1b%/2\x80\x8agbk-0\x02\xf3{\xfeD", "髙﨑"),
        (b"\x1b)I\xb6\xc0\xb6\xc5", "ｶﾀｶﾅ"),
        (b"a\x1b(J~\x1b(Bb", "a‾b"),
        (b"\x1b$(C3MCV0m>_", "넌최고야"),
        (b"\x1b-T\xc0\xd2\xc9\xd2", "ภาษา"),
        (b"\x1b-L\xbf\xe0\xd8\xd2\xd5\xe2", "Привет"),
        (b"\xa1\xb8\xc0\xd1\xe2\xf3", "¡¸ÀÑâó"),
        (b"\x1b-B\xa1\xa3\xa5\xa6\xa9\xab", "ĄŁĽŚŠŤ"),
    ]);
}

/// The web's tables differ here; IBus and fcitx5 use libX11's.
#[test]
fn each_set_decodes_through_libx11s_own_table() {
    assert_decodes(&[
        (b"\x1b$(A!$", "・"),
        (b"\x1b$(A!*", "―"),
        (b"\x1b$(B!A", "〜"),
        (b"\x1b$(B!]", "−"),
        (b"\x1b%/2\x80\x89big5-0\x02\xa1\xf3", "☉"),
        (b"\x1b%/2\x80\x89big5-0\x02\xc6\xa6", "あ"),
        (b"\x1b%/2\x80\x8ebig5hkscs-0\x02\xc8\xf4", "\u{f83e}"),
    ]);
}

/// ibus-x11 sends unconverted text as UTF-8, left open.
#[test]
fn an_unclosed_utf8_segment_runs_to_the_end() {
    assert_decodes(&[(b"\x1b%GTi\xe1\xba\xbfng", "Tiếng")]);
}

#[test]
fn what_cannot_be_read_fails_with_its_bytes() {
    let unsupported = |bytes: &[u8], named: &[u8]| {
        let decoded = compound_text_to_utf8(bytes);
        assert!(
            matches!(&decoded, Err(DecodeError::UnsupportedEncoding(b)) if b == named),
            "{decoded:x?}"
        );
    };
    unsupported(b"\x1b$(GD!", b"\x1b$(G");
    unsupported(b"\x1b%/2\x80\x88koi8-r\x02\xc1", b"koi8-r");
    let invalid = |bytes: &[u8], named: &[u8]| {
        let decoded = compound_text_to_utf8(bytes);
        assert!(
            matches!(&decoded, Err(DecodeError::InvalidEncoding(b)) if b == named),
            "{decoded:x?}"
        );
    };
    invalid(b"\x1b$(", b"\x1b$(");
    invalid(b"\x1b$(AVPN", b"VPN");
    invalid(b"\x1b$(A\"!", b"\"!");
    invalid(b"\x1b%/2\x80\x89big5-0\x02\xc8\x40", b"\xc8\x40");
    invalid(b"\x1b%G\xff\x1b%@", b"\xff");
    invalid(b"\x1b%/2\x80\x89big5-0", b"\x80\x89big5-0");
}
