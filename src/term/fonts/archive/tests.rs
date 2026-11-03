//! The bundled archive unpacks into the faces the terminal registers.

use super::*;

#[test]
fn every_face_unpacks_from_the_bundled_archive() {
    let fonts = fonts().expect("the bundled archive unpacks");
    assert_eq!(fonts.len(), WEIGHTS.len());
    for (font, name) in fonts.iter().zip(WEIGHTS) {
        assert!(font.len() > 1_000_000, "{name} is {} bytes", font.len());
        // Every TTF starts with a version tag or `true`; a wrong part would not.
        assert!(
            font.starts_with(&[0x00, 0x01, 0x00, 0x00]) || font.starts_with(b"true"),
            "{name} does not start like a font",
        );
    }
}

#[test]
fn the_archive_holds_eight_weights_and_their_italics() {
    let upright: Vec<&str> = WEIGHTS
        .iter()
        .copied()
        .filter(|name| !name.contains("Italic"))
        .collect();
    let italic: Vec<&str> = WEIGHTS
        .iter()
        .copied()
        .filter(|name| name.contains("Italic"))
        .collect();
    assert_eq!(upright.len(), 8);
    assert_eq!(italic.len(), 8);
    for weight in [
        "Thin",
        "ExtraLight",
        "Light",
        "Regular",
        "Medium",
        "SemiBold",
        "Bold",
        "ExtraBold",
    ] {
        let name = format!("JetBrainsMonoNerdFontMono-{weight}.ttf");
        assert!(WEIGHTS.contains(&name.as_str()), "{name} is missing");
    }
    // Regular's italic drops "Regular", as Nerd Fonts names it.
    assert!(WEIGHTS.contains(&"JetBrainsMonoNerdFontMono-Italic.ttf"));
}

#[test]
fn a_stream_that_stops_mid_header_names_what_it_lacks() {
    let mut picker = Picker::new();
    // A tar with one header and no body: the walk ends without the faces.
    let mut header = vec![0u8; 512];
    header[0..5].copy_from_slice(b"x.ttf");
    header[124..136].copy_from_slice(b"00000000012\0");
    picker.write_all(&header).expect("the header writes");
    picker.write_all(b"short").expect("the body writes");
    let error = picker.finish().expect_err("no face was kept");
    assert!(error.contains("Thin"), "{error}");
}
