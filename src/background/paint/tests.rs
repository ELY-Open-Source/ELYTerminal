use super::*;

const INTRINSIC: (f32, f32) = (200.0, 100.0);
const WINDOW: (f32, f32) = (100.0, 100.0);

#[test]
fn a_picture_fills_the_window_as_its_size_says() {
    assert_eq!(sized(Size::Cover, INTRINSIC, WINDOW), (200.0, 100.0));
    assert_eq!(sized(Size::Contain, INTRINSIC, WINDOW), (100.0, 50.0));
    assert_eq!(sized(Size::Fill, INTRINSIC, WINDOW), WINDOW);
    assert_eq!(sized(Size::Auto, INTRINSIC, WINDOW), INTRINSIC);
    assert_eq!(
        sized(
            Size::Explicit(Length::Percent(0.5), Length::Auto),
            INTRINSIC,
            WINDOW
        ),
        (50.0, 25.0)
    );
    assert_eq!(
        sized(
            Size::Explicit(Length::Auto, Length::Px(25.0)),
            INTRINSIC,
            WINDOW
        ),
        (50.0, 25.0)
    );
    assert_eq!(
        sized(
            Size::Explicit(Length::Px(10.0), Length::Px(20.0)),
            INTRINSIC,
            WINDOW
        ),
        (10.0, 20.0)
    );
}

#[test]
fn a_position_resolves_in_the_space_the_picture_leaves() {
    assert_eq!(offset(Axis::Start, 50.0), 0.0);
    assert_eq!(offset(Axis::Center, 50.0), 25.0);
    assert_eq!(offset(Axis::End, 50.0), 50.0);
    assert_eq!(offset(Axis::Percent(0.2), 50.0), 10.0);
    assert_eq!(offset(Axis::Px(5.0), 50.0), 5.0);
}

#[test]
fn repeat_tiles_cover_the_window_from_the_position() {
    assert_eq!(along(10.0, 30.0, 100.0), [-20.0, 10.0, 40.0, 70.0]);
    assert_eq!(along(-10.0, 30.0, 100.0), [-10.0, 20.0, 50.0, 80.0]);
    let one = placed(Repeat::NoRepeat, 10.0, 20.0, 30.0, 30.0, WINDOW);
    assert_eq!(one, [(10.0, 20.0)]);
    let across = placed(Repeat::RepeatX, 0.0, 20.0, 30.0, 30.0, WINDOW);
    assert_eq!(across.len(), 4);
    assert!(across.iter().all(|(_, y)| *y == 20.0));
    let grid = placed(Repeat::Repeat, 0.0, 0.0, 30.0, 50.0, WINDOW);
    assert_eq!(grid.len(), 4 * 2);
}

#[test]
fn a_repeat_past_the_cap_asks_for_one_tile() {
    let many = placed(Repeat::Repeat, 0.0, 0.0, 1.0, 1.0, WINDOW);
    assert!(many.is_empty(), "the cap turns it into one");
}
