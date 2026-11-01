use gpui::{Hsla, Rgba};

use super::*;

/// `color`'s bytes, for comparison.
fn bytes(color: Hsla) -> [u8; 4] {
    let rgba = Rgba::from(color);
    [rgba.r, rgba.g, rgba.b, rgba.a].map(|channel| (channel * 255.0).round() as u8)
}

#[test]
fn an_empty_style_paints_nothing() {
    assert!(parse("").expect("empty").is_empty());
    assert!(parse("  \n ").expect("blank").is_empty());
    assert!(parse("/* nothing */").expect("a comment").is_empty());
}

#[test]
fn colors_read_as_hex_rgb_and_hsl() {
    assert_eq!(bytes(color("#ff8000").expect("hex")), [255, 128, 0, 255]);
    assert_eq!(bytes(color("#f80").expect("short hex")), [255, 136, 0, 255]);
    assert_eq!(bytes(color("#00000080").expect("alpha")), [0, 0, 0, 128]);
    assert_eq!(
        bytes(color("rgb(255, 128, 0)").expect("rgb")),
        [255, 128, 0, 255]
    );
    assert_eq!(
        bytes(color("rgba(255, 128, 0, 0.5)").expect("rgba")),
        [255, 128, 0, 128]
    );
    assert_eq!(
        bytes(color("hsl(0, 100%, 50%)").expect("hsl")),
        [255, 0, 0, 255]
    );
    assert_eq!(
        bytes(color("hsla(120, 100%, 50%, 50%)").expect("hsla")),
        [0, 255, 0, 128]
    );
    assert_eq!(bytes(color("transparent").expect("clear")), [0, 0, 0, 0]);
    assert!(color("mauve").is_err());
    assert!(color("#ff800").is_err());
    assert!(color("rgb(1, 2)").is_err());
}

#[test]
fn a_background_reads_its_color_and_layers() {
    let background = parse(
        "background-color: #112233;\n\
         background-image: linear-gradient(90deg, #000 0%, #fff 100%);\n\
         background-size: contain;",
    )
    .expect("a background");
    assert_eq!(bytes(background.color.expect("a color")), [17, 34, 51, 255]);
    let [layer] = &background.layers[..] else {
        panic!("one layer: {:?}", background.layers);
    };
    assert_eq!(layer.size, Size::Contain);
    let Paint::Gradient(gradient) = &layer.paint else {
        panic!("a gradient: {:?}", layer.paint);
    };
    assert_eq!(gradient.angle, 90.0);
    assert_eq!(gradient.stops.len(), 2);
    assert_eq!(bytes(gradient.stops[0].color), [0, 0, 0, 255]);
    assert_eq!(gradient.stops[0].at, 0.0);
    assert_eq!(gradient.stops[1].at, 1.0);
}

#[test]
fn a_gradient_takes_sides_and_places_its_two_stops() {
    let gradient = |text: &str| match layer_paint(text).expect("a layer") {
        Paint::Gradient(gradient) => gradient,
        other => panic!("a gradient: {other:?}"),
    };
    assert_eq!(
        gradient("linear-gradient(to bottom right, #000, #fff)").angle,
        135.0
    );
    assert_eq!(gradient("linear-gradient(#000, #fff)").angle, 180.0);
    let plain = gradient("linear-gradient(#000, #fff)");
    assert_eq!(
        plain.stops.iter().map(|stop| stop.at).collect::<Vec<_>>(),
        [0.0, 1.0]
    );
    let placed = gradient("linear-gradient(#000 20%, #fff 80%)");
    assert_eq!(placed.stops[0].at, 0.2);
    assert_eq!(placed.stops[1].at, 0.8);
    assert!(layer_paint("linear-gradient(#000)").is_err());
    assert!(
        layer_paint("linear-gradient(#000, #f00, #fff)").is_err(),
        "gpui draws two stops"
    );
    assert!(layer_paint("radial-gradient(#000, #fff)").is_err());
}

#[test]
fn layers_are_listed_topmost_first_and_parsed_in_any_case() {
    let background =
        parse("background-image: LINEAR-GRADIENT(0deg, #000, #fff), URL(\"C:/pictures/ely.png\");")
            .expect("two layers");
    assert_eq!(background.layers.len(), 2);
    assert!(matches!(&background.layers[0].paint, Paint::Gradient(_)));
    assert_eq!(
        background.layers[1].paint,
        Paint::Image(PathBuf::from("C:/pictures/ely.png"))
    );
    assert!(
        parse("background-image: url(ely.png);").is_err(),
        "not a full path"
    );
    assert!(
        parse("background-image: none;")
            .expect("none")
            .layers
            .is_empty()
    );
}

#[test]
fn size_position_and_repeat_cycle_over_the_layers() {
    let background = parse(
        "background-image: url(\"C:/a.png\"), url(\"C:/b.png\");\n\
         background-size: 640px 480px, cover;\n\
         background-position: left top, 80% 20%;\n\
         background-repeat: no-repeat, repeat-x;",
    )
    .expect("a background");
    let [first, second] = &background.layers[..] else {
        panic!("two layers");
    };
    assert_eq!(
        first.size,
        Size::Explicit(Length::Px(640.0), Length::Px(480.0))
    );
    assert_eq!(first.position.x, Axis::Start);
    assert_eq!(first.position.y, Axis::Start);
    assert_eq!(first.repeat, Repeat::NoRepeat);
    assert_eq!(second.size, Size::Cover);
    assert_eq!(second.position.x, Axis::Percent(0.8));
    assert_eq!(second.position.y, Axis::Percent(0.2));
    assert_eq!(second.repeat, Repeat::RepeatX);
}

#[test]
fn sizes_read_keywords_lengths_and_percentages() {
    let one = |style: &str| {
        parse(&format!(
            "background-image: url(\"C:/a.png\"); background-size: {style};"
        ))
        .expect(style)
        .layers[0]
            .size
    };
    assert_eq!(one("cover"), Size::Cover);
    assert_eq!(one("contain"), Size::Contain);
    assert_eq!(one("fill"), Size::Fill);
    assert_eq!(one("auto"), Size::Auto);
    assert_eq!(
        one("50%"),
        Size::Explicit(Length::Percent(0.5), Length::Auto)
    );
    assert_eq!(
        one("auto 30%"),
        Size::Explicit(Length::Auto, Length::Percent(0.3))
    );
    assert!(parse("background-image: url(\"C:/a.png\"); background-size: 10;").is_ok());
    assert!(parse("background-image: url(\"C:/a.png\"); background-size: -5px;").is_err());
    assert!(parse("background-image: url(\"C:/a.png\"); background-size: 1px 2px 3px;").is_err());
}

#[test]
fn positions_read_sides_centers_and_lengths() {
    let one = |style: &str| {
        parse(&format!(
            "background-image: url(\"C:/a.png\"); background-position: {style};"
        ))
        .expect(style)
        .layers[0]
            .position
    };
    assert_eq!(one("center"), Position::default());
    assert_eq!(
        one("top"),
        Position {
            x: Axis::Center,
            y: Axis::Start
        }
    );
    assert_eq!(
        one("bottom right"),
        Position {
            x: Axis::End,
            y: Axis::End
        }
    );
    assert_eq!(
        one("10px 20%"),
        Position {
            x: Axis::Px(10.0),
            y: Axis::Percent(0.2)
        }
    );
    assert!(
        parse("background-image: url(\"C:/a.png\"); background-position: 1px 2px 3px;").is_err()
    );
}

#[test]
fn comments_and_blank_declarations_pass_by() {
    let background = parse(
        "/* a comment; with a separator */\n\
         background-color: #000;\n\n\
         ; background-size: fill;",
    )
    .expect("a background");
    assert!(background.color.is_some());
}

#[test]
fn an_unknown_property_is_an_error_naming_it() {
    let error = parse("background-glow: #fff;").expect_err("unknown");
    assert!(
        format!("{error:#}").contains("background-glow"),
        "{error:#}"
    );
    let error = parse("background-color #fff").expect_err("no colon");
    assert!(
        format!("{error:#}").contains("property: value"),
        "{error:#}"
    );
    let error = parse("/* open").expect_err("open comment");
    assert!(format!("{error:#}").contains("never ends"), "{error:#}");
    assert!(parse("background-repeat: sideways;").is_err());
}
