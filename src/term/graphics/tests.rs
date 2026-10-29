use std::io::Write;

use alacritty_terminal::{
    index::{Column, Line, Point},
    term::Graphics as _,
};
use base64::{Engine, engine::general_purpose::STANDARD};

use super::*;

fn encode(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}

fn send(graphics: &mut Graphics, control: &str, payload: &[u8]) -> Option<String> {
    raw(graphics, &format!("{control};{}", encode(payload)))
}

fn raw(graphics: &mut Graphics, text: &str) -> Option<String> {
    graphics
        .command(text.as_bytes(), Point::new(Line(0), Column(0)))
        .reply
}

fn reply(keys: &str, message: &str) -> Option<String> {
    Some(format!("\x1b_G{keys};{message}\x1b\\"))
}

fn ids(graphics: &Graphics) -> Vec<u32> {
    graphics.ids()
}

#[test]
fn a_direct_image_is_stored_as_rgba_and_answered() {
    let mut graphics = Graphics::new(0, (80, 24), (10, 20));
    let rgb = [1, 2, 3, 4, 5, 6];
    assert_eq!(
        send(&mut graphics, "i=31,f=24,s=2,v=1", &rgb),
        reply("i=31", "OK")
    );
    let image = graphics.image(31).expect("stored");
    assert_eq!((image.width, image.height), (2, 1));
    assert_eq!(image.pixels, [1, 2, 3, 255, 4, 5, 6, 255]);
    assert_eq!(
        send(&mut graphics, "i=2,p=4,s=1,v=1", &[9, 8, 7, 6]),
        reply("i=2,p=4", "OK")
    );
    assert_eq!(graphics.image(2).expect("stored").pixels, [9, 8, 7, 6]);
    // Without an id or a number, it is stored unanswered.
    assert_eq!(send(&mut graphics, "s=1,v=1", &[0; 4]), None);
    assert_eq!(ids(&graphics).len(), 3);
}

#[test]
fn a_number_takes_the_lowest_free_id_and_the_newest_answers_to_it() {
    let mut graphics = Graphics::new(0, (80, 24), (10, 20));
    send(&mut graphics, "i=1,s=1,v=1", &[0; 4]);
    assert_eq!(
        send(&mut graphics, "I=13,s=1,v=1", &[0; 4]),
        reply("i=2,I=13", "OK")
    );
    assert_eq!(
        send(&mut graphics, "I=13,s=1,v=1", &[0; 4]),
        reply("i=3,I=13", "OK")
    );
    assert_eq!(raw(&mut graphics, "a=d,d=N,I=13"), None);
    assert_eq!(ids(&graphics), [1, 2]);
    let both = "EINVAL: image ID and number are mutually exclusive";
    assert_eq!(
        send(&mut graphics, "i=5,I=6,s=1,v=1", &[0; 4]),
        reply("i=5,I=6", both)
    );
    assert_eq!(raw(&mut graphics, "a=d,d=I,i=1,I=13,q=2"), None);
    assert_eq!(ids(&graphics), [1, 2]);
}

#[test]
fn quiet_keeps_back_what_it_names() {
    let mut graphics = Graphics::new(0, (80, 24), (10, 20));
    assert_eq!(send(&mut graphics, "i=1,q=1,s=1,v=1", &[0; 4]), None);
    let short = reply("i=2", "EINVAL: invalid data");
    assert_eq!(send(&mut graphics, "i=2,q=1,s=1,v=1", &[0; 3]), short);
    assert_eq!(send(&mut graphics, "i=3,q=2,s=1,v=1", &[0; 3]), None);
    assert_eq!(send(&mut graphics, "i=4,q=9,s=1,v=1", &[0; 3]), None);
    assert_eq!(ids(&graphics), [1]);
}

#[test]
fn chunks_join_answer_once_at_the_end_and_keep_their_first_quiet() {
    let mut graphics = Graphics::new(0, (80, 24), (10, 20));
    let pixels: Vec<u8> = (0..16).collect();
    let text = encode(&pixels);
    let (first, rest) = text.split_at(8);
    let (second, last) = rest.split_at(8);
    assert_eq!(
        raw(&mut graphics, &format!("i=5,s=2,v=2,m=1;{first}")),
        None
    );
    assert_eq!(raw(&mut graphics, &format!("m=1;{second}")), None);
    assert_eq!(
        raw(&mut graphics, &format!("m=0;{last}")),
        reply("i=5", "OK")
    );
    assert_eq!(graphics.image(5).expect("stored").pixels, pixels);
    // q=1 on the first chunk holds for the last.
    raw(
        &mut graphics,
        &format!("i=6,q=1,s=2,v=2,m=1;{first}{second}"),
    );
    assert_eq!(raw(&mut graphics, &format!("m=0;{last}")), None);
    assert!(graphics.image(6).is_some());
    // A file transmission drops one under way.
    raw(&mut graphics, &format!("i=7,s=2,v=2,m=1;{first}"));
    let missing = encode(b"/nonexistent/ely");
    raw(&mut graphics, &format!("i=8,t=f,q=2;{missing}"));
    assert_eq!(raw(&mut graphics, &format!("m=0;{last}")), None);
    assert_eq!(ids(&graphics), [5, 6]);
}

#[test]
fn zlib_and_png_decode_to_rgba() {
    let mut graphics = Graphics::new(0, (80, 24), (10, 20));
    let rgb = [10, 20, 30, 40, 50, 60];
    let mut zlib = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    zlib.write_all(&rgb).expect("it compresses");
    let zlib = zlib.finish().expect("it ends");
    assert_eq!(
        send(&mut graphics, "i=1,f=24,s=2,v=1,o=z", &zlib),
        reply("i=1", "OK")
    );
    assert_eq!(
        graphics.image(1).expect("stored").pixels,
        [10, 20, 30, 255, 40, 50, 60, 255]
    );
    let rgba = image::RgbaImage::from_raw(1, 2, vec![1, 2, 3, 4, 5, 6, 7, 8]).expect("pixels");
    let mut png = std::io::Cursor::new(Vec::new());
    rgba.write_to(&mut png, image::ImageFormat::Png)
        .expect("it encodes");
    let png = png.into_inner();
    assert_eq!(send(&mut graphics, "i=2,f=100", &png), reply("i=2", "OK"));
    let image = graphics.image(2).expect("stored");
    assert_eq!((image.width, image.height), (1, 2));
    assert_eq!(image.pixels, [1, 2, 3, 4, 5, 6, 7, 8]);
    let mut zlib = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    zlib.write_all(&png).expect("it compresses");
    let both = zlib.finish().expect("it ends");
    let keys = format!("i=3,f=100,o=z,S={}", png.len());
    assert_eq!(send(&mut graphics, &keys, &both), reply("i=3", "OK"));
}

#[test]
fn each_failure_answers_by_name() {
    let mut graphics = Graphics::new(0, (80, 24), (10, 20));
    let cases = [
        ("i=1,s=2,v=1", &[0u8; 7][..], "EINVAL: invalid data"),
        ("i=1,f=99,s=1,v=1", &[0; 4], "EINVAL: unsupported format"),
        ("i=1,v=1", &[0; 4], "EINVAL: dimensions required"),
        ("i=1,s=10001,v=1", &[], "EINVAL: dimensions too large"),
        (
            "i=1,s=1,v=1,o=z",
            &[1, 2, 3],
            "EINVAL: decompression failed",
        ),
        ("i=1,t=x", &[], "EINVAL: unsupported medium"),
        ("i=1,s=1,v=1,o=q", &[0; 4], "EINVAL: invalid data"),
        ("i=1,f=100", b"no png", "EINVAL: invalid data"),
    ];
    for (keys, payload, message) in cases {
        assert_eq!(
            send(&mut graphics, keys, payload),
            reply("i=1", message),
            "{keys}"
        );
    }
    // A malformed command drops without a word.
    for text in [
        "i=1,s;AAAA",
        "i=x;AAAA",
        "a=tt",
        "i=1;not base64!",
        "i=99999999999",
    ] {
        assert_eq!(raw(&mut graphics, text), None, "{text}");
    }
    assert!(ids(&graphics).is_empty());
}

#[test]
fn a_query_loads_and_keeps_nothing() {
    let mut graphics = Graphics::new(0, (80, 24), (10, 20));
    assert_eq!(
        raw(&mut graphics, "i=31,s=1,v=1,a=q,t=d,f=24;AAAA"),
        reply("i=31", "OK")
    );
    assert_eq!(
        raw(&mut graphics, "i=31,s=2,v=1,a=q,f=24;AAAA"),
        reply("i=31", "EINVAL: invalid data")
    );
    assert_eq!(raw(&mut graphics, "s=1,v=1,a=q,f=24;AAAA"), None);
    assert!(ids(&graphics).is_empty());
}

#[test]
fn a_reused_id_replaces_its_image() {
    let mut graphics = Graphics::new(0, (80, 24), (10, 20));
    send(&mut graphics, "i=4,s=1,v=1", &[1; 4]);
    send(&mut graphics, "i=4,s=1,v=1", &[2; 4]);
    assert_eq!(graphics.image(4).expect("stored").pixels, [2; 4]);
    // The old image goes as the new starts, even failing.
    send(&mut graphics, "i=4,s=1,v=1", &[3; 3]);
    assert!(ids(&graphics).is_empty());
}

#[test]
fn deletes_free_image_data_in_capitals_only() {
    let mut graphics = Graphics::new(0, (80, 24), (10, 20));
    for id in 1..=5 {
        send(&mut graphics, &format!("i={id},q=1,s=1,v=1"), &[0; 4]);
    }
    for text in [
        "a=d",
        "a=d,d=A",
        "a=d,d=i,i=1",
        "a=d,d=I,i=1,p=3",
        "a=d,d=Z,z=-1",
    ] {
        assert_eq!(raw(&mut graphics, text), None, "{text}");
    }
    assert_eq!(ids(&graphics), [1, 2, 3, 4, 5]);
    raw(&mut graphics, "a=d,d=I,i=1");
    raw(&mut graphics, "a=d,d=R,x=3,y=4");
    raw(&mut graphics, "a=d,d=R,x=5,y=0");
    assert_eq!(ids(&graphics), [2, 5]);
    assert_eq!(raw(&mut graphics, "a=d,d=w"), None);
    // A delete drops a transmission under way.
    let pixels = encode(&[0; 8]);
    raw(&mut graphics, &format!("i=9,s=1,v=2,m=1;{}", &pixels[..4]));
    raw(&mut graphics, "a=d,d=a");
    raw(&mut graphics, &format!("m=0;{}", &pixels[4..]));
    assert_eq!(ids(&graphics), [2, 5]);
}

#[test]
fn the_quota_evicts_transient_images_then_the_oldest() {
    let mut graphics = Graphics::limited(0, (80, 24), (10, 20), 40);
    send(&mut graphics, "i=1,s=2,v=2", &[0; 16]);
    send(&mut graphics, "i=2,s=2,v=2,N=1", &[0; 16]);
    send(&mut graphics, "i=3,s=2,v=2", &[0; 16]);
    assert_eq!(ids(&graphics), [1, 3]);
    send(&mut graphics, "i=4,s=2,v=2", &[0; 16]);
    assert_eq!(ids(&graphics), [3, 4]);
    let past = send(&mut graphics, "i=5,s=4,v=3", &[0; 48]);
    assert_eq!(past, reply("i=5", "ENOMEM: out of memory"));
    assert_eq!(ids(&graphics), [3, 4]);
}

#[test]
fn simd_base64_takes_what_the_old_decoder_took() {
    use base64::{
        alphabet,
        engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig},
    };
    let old = GeneralPurpose::new(
        &alphabet::STANDARD,
        GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
    );
    // Trailing bits, padding, a stray symbol and a newline.
    let symbols = b"AQgw/+=_\n";
    let mut text = Vec::new();
    for length in 0..=6u32 {
        for index in 0..symbols.len().pow(length) {
            text.clear();
            let mut rest = index;
            for _ in 0..length {
                text.push(symbols[rest % symbols.len()]);
                rest /= symbols.len();
            }
            let new = command::decode(&text).ok();
            assert_eq!(
                new,
                old.decode(&text).ok(),
                "{:?}",
                String::from_utf8_lossy(&text)
            );
        }
    }
    let long = encode(&(0..=255).collect::<Vec<u8>>());
    assert_eq!(
        command::decode(long.as_bytes()).unwrap(),
        (0..=255).collect::<Vec<u8>>()
    );
    let unpadded = long.trim_end_matches('=');
    assert_eq!(command::decode(unpadded.as_bytes()).unwrap().len(), 256);
}
