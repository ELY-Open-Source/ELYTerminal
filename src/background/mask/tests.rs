use super::*;

/// A mask read from a whole style.
fn mask(style: &str) -> Mask {
    crate::background::parse(&format!("background-image: url(\"C:/a.png\"); {style}"))
        .expect("a style")
        .mask
        .expect("a mask")
}

#[test]
fn a_mask_fades_along_its_gradient() {
    let faded = mask(
        "mask-image: linear-gradient(90deg, transparent, black); \
         mask-size: 40% 100%; mask-position: right top;",
    );
    let window = (100.0, 100.0);
    assert_eq!(faded.alpha(50.0, 50.0, window), 0.0, "left of the box");
    assert!(
        faded.alpha(60.0, 50.0, window) < 0.001,
        "the box's left edge"
    );
    assert!((faded.alpha(80.0, 50.0, window) - 0.5).abs() < 0.01);
    assert!((faded.alpha(99.0, 50.0, window) - 0.975).abs() < 0.01);
}

#[test]
fn mask_stops_keep_their_positions() {
    let faded = mask(
        "mask-image: linear-gradient(90deg, transparent, black, black); \
         mask-size: 100% 100%;",
    );
    let window = (100.0, 100.0);
    assert!((faded.alpha(25.0, 50.0, window) - 0.5).abs() < 0.01);
    assert!((faded.alpha(50.0, 50.0, window) - 1.0).abs() < 0.01);
    assert_eq!(faded.alpha(75.0, 50.0, window), 1.0);
}

#[test]
fn a_vertical_mask_fades_down_the_window() {
    let faded = mask(
        "mask-image: linear-gradient(180deg, transparent, black); \
         mask-size: 100% 100%;",
    );
    let window = (100.0, 100.0);
    assert!((faded.alpha(50.0, 25.0, window) - 0.25).abs() < 0.01);
    assert!((faded.alpha(50.0, 75.0, window) - 0.75).abs() < 0.01);
}

#[test]
fn a_mask_sits_at_the_left_top_unless_it_says() {
    let faded = mask("mask-image: linear-gradient(90deg, transparent, black);");
    let window = (100.0, 100.0);
    assert!((faded.alpha(50.0, 50.0, window) - 0.5).abs() < 0.01);
}
