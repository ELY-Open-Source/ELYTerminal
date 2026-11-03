use std::path::PathBuf;

use gpui::{
    Context, Entity, InteractiveElement, IntoElement, Modifiers, MouseButton, ParentElement,
    Pixels, Point, Render, Styled, TestAppContext, VisualTestContext, Window, div, point, px,
};

use super::{picture, press, settle, setup};
use crate::media::{Crop, ImageAnnotator, ImageCropper, ImageUpload, Mark};

const HALF: Crop = Crop {
    x: 0.25,
    y: 0.25,
    w: 0.5,
    h: 0.5,
};

fn near(a: Crop, b: Crop) -> bool {
    [(a.x, b.x), (a.y, b.y), (a.w, b.w), (a.h, b.h)]
        .iter()
        .all(|(a, b)| (a - b).abs() < 1e-3)
}

/// Where a share of the picture sits in the window.
fn spot(selector: &'static str, share: (f32, f32), cx: &mut VisualTestContext) -> Point<Pixels> {
    let picture = cx.debug_bounds(selector).expect("the picture draws");
    picture.origin + point(picture.size.width * share.0, picture.size.height * share.1)
}

fn picture_width(cx: &mut VisualTestContext) -> f32 {
    f32::from(
        cx.debug_bounds("image-cropper")
            .expect("the picture draws")
            .size
            .width,
    )
}

/// A drag from `from` through `path`, the pointer let go at the last place.
fn drag(from: Point<Pixels>, path: &[Point<Pixels>], cx: &mut VisualTestContext) {
    let none = Modifiers::none();
    cx.simulate_mouse_down(from, MouseButton::Left, none);
    for at in path {
        cx.simulate_mouse_move(*at, MouseButton::Left, none);
    }
    cx.simulate_mouse_up(*path.last().unwrap_or(&from), MouseButton::Left, none);
    settle(cx);
}

/// A cropper 400 wide over a picture twice as wide as tall; with `editable`, it keeps each crop it reports.
struct Cropping {
    path: PathBuf,
    crop: Crop,
    aspect: Option<f32>,
    editable: bool,
}

impl Render for Cropping {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        let cropper =
            ImageCropper::new("crop", self.path.clone(), 2.0, self.crop).aspect(self.aspect);
        let cropper = if self.editable {
            cropper.on_change(move |crop, _, cx| {
                owner.update(cx, |view, cx| {
                    view.crop = crop;
                    cx.notify();
                })
            })
        } else {
            cropper
        };
        div().w(px(400.0)).child(cropper)
    }
}

fn cropping(
    crop: Crop,
    aspect: Option<f32>,
    editable: bool,
    cx: &mut TestAppContext,
) -> (Entity<Cropping>, &mut VisualTestContext) {
    setup(cx);
    let path = picture("crop", 200, 100);
    let (view, cx) = cx.add_window_view(|_, _| Cropping {
        path,
        crop,
        aspect,
        editable,
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (view, cx)
}

#[gpui::test]
fn a_corner_drag_sizes_the_crop_and_stops_at_the_picture_edge(cx: &mut TestAppContext) {
    let (view, cx) = cropping(HALF, None, true, cx);
    let corner = spot("image-cropper", (0.75, 0.75), cx);
    drag(
        corner,
        &[
            corner + point(px(20.0), px(10.0)),
            corner + point(px(600.0), px(600.0)),
        ],
        cx,
    );
    let crop = view.read_with(cx, |view, _| view.crop);
    assert!(
        near(
            crop,
            Crop {
                w: 0.75,
                h: 0.75,
                ..HALF
            }
        ),
        "{crop:?}"
    );
}

#[gpui::test]
fn the_arrows_move_the_crop_a_hundredth_and_shift_a_tenth(cx: &mut TestAppContext) {
    let (view, cx) = cropping(HALF, None, true, cx);
    let middle = spot("image-cropper", (0.5, 0.5), cx);
    cx.simulate_click(middle, Modifiers::none());
    settle(cx);
    press("right", cx);
    press("shift-right", cx);
    let crop = view.read_with(cx, |view, _| view.crop);
    assert!(near(crop, Crop { x: 0.36, ..HALF }), "{crop:?}");
}

#[gpui::test]
fn a_shaped_crop_keeps_its_shape_as_a_corner_moves(cx: &mut TestAppContext) {
    let square = Crop { w: 0.25, ..HALF };
    let (view, cx) = cropping(square, Some(1.0), true, cx);
    let corner = spot("image-cropper", (0.5, 0.75), cx);
    drag(
        corner,
        &[
            corner + point(px(20.0), px(0.0)),
            corner + point(px(40.0), px(0.0)),
        ],
        cx,
    );
    let (crop, width) = (view.read_with(cx, |view, _| view.crop), picture_width(cx));
    let grown = 0.25 + 40.0 / width;
    let shaped = Crop {
        w: grown,
        h: grown * 2.0,
        ..HALF
    };
    assert!(near(crop, shaped), "{crop:?}");
}

#[gpui::test]
fn a_cropper_with_no_handler_takes_no_tab(cx: &mut TestAppContext) {
    let (_, cx) = cropping(HALF, None, false, cx);
    cx.update(|window, cx| window.focus_next(cx));
    assert!(cx.update(|window, cx| window.focused(cx)).is_none());
}

/// An upload over a picked picture, the crops it heard, kept or not, and how often it was emptied.
struct Uploading {
    picked: Option<PathBuf>,
    crop: Option<Crop>,
    keeps: bool,
    heard: usize,
    removed: usize,
}

impl Render for Uploading {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (owner, cropped) = (cx.entity(), cx.entity());
        div().w(px(420.0)).child(
            ImageUpload::new("upload", self.picked.clone(), self.crop)
                .aspect(Some(1.0))
                .on_crop(move |crop, _, cx| {
                    cropped.update(cx, |view, cx| {
                        view.heard += 1;
                        if view.keeps {
                            view.crop = Some(crop);
                        }
                        cx.notify();
                    })
                })
                .on_remove(move |_, cx| {
                    owner.update(cx, |view, cx| {
                        (view.picked, view.removed) = (None, view.removed + 1);
                        cx.notify();
                    })
                }),
        )
    }
}

#[gpui::test]
fn a_picked_picture_crops_from_a_fitted_box_and_remove_empties_it(cx: &mut TestAppContext) {
    setup(cx);
    let picked = Some(picture("upload", 300, 100));
    let (view, cx) = cx.add_window_view(|_, _| Uploading {
        picked,
        crop: None,
        keeps: true,
        heard: 0,
        removed: 0,
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    let shown = cx
        .debug_bounds("image-cropper")
        .expect("the cropper draws")
        .size;
    let snapped = (shown.height - shown.width / 3.0).abs() <= px(0.5);
    assert!(
        snapped,
        "the picture's own shape, to the half point: {shown:?}"
    );
    let (crop, heard) = view.read_with(cx, |view, _| (view.crop, view.heard));
    let square = Crop {
        x: 1.0 / 3.0,
        y: 0.0,
        w: 1.0 / 3.0,
        h: 1.0,
    };
    let fitted = crop.is_some_and(|crop| near(crop, square));
    assert!(fitted && heard == 1, "{crop:?} after {heard}");
    cx.update(|window, cx| {
        for _ in 0..2 {
            window.focus_next(cx);
        }
    });
    press("enter", cx);
    assert_eq!(view.read_with(cx, |view, _| view.removed), 1);
}

#[gpui::test]
fn a_host_that_keeps_no_crop_still_hears_the_fitted_one_once(cx: &mut TestAppContext) {
    setup(cx);
    let picked = Some(picture("ignored", 300, 100));
    let (view, cx) = cx.add_window_view(|_, _| Uploading {
        picked,
        crop: None,
        keeps: false,
        heard: 0,
        removed: 0,
    });
    settle(cx);
    for _ in 0..3 {
        view.update(cx, |_, cx| cx.notify());
        settle(cx);
    }
    assert_eq!(view.read_with(cx, |view, _| view.heard), 1);
}

/// An annotator 400 wide over a picture twice as wide as tall, and its marks.
struct Marking {
    path: PathBuf,
    marks: Vec<Mark>,
    escapes: usize,
}

impl Render for Marking {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (owner, page) = (cx.entity(), cx.entity());
        div()
            .w(px(400.0))
            .on_key_down(move |event, _, cx| {
                if event.keystroke.key == "escape" {
                    page.update(cx, |view, _| view.escapes += 1);
                }
            })
            .child(
                ImageAnnotator::new("marks", self.path.clone(), 2.0, self.marks.clone()).on_change(
                    move |marks, _, cx| {
                        owner.update(cx, |view, cx| {
                            view.marks = marks.to_vec();
                            cx.notify();
                        })
                    },
                ),
            )
    }
}

fn marking(marks: Vec<Mark>, cx: &mut TestAppContext) -> (Entity<Marking>, &mut VisualTestContext) {
    setup(cx);
    let path = picture("marks", 200, 100);
    let (view, cx) = cx.add_window_view(|_, _| Marking {
        path,
        marks,
        escapes: 0,
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (view, cx)
}

#[gpui::test]
fn a_drag_with_the_box_tool_adds_a_box_and_a_tap_adds_nothing(cx: &mut TestAppContext) {
    let (view, cx) = marking(Vec::new(), cx);
    let tap = spot("image-annotator", (0.8, 0.8), cx);
    cx.simulate_click(tap, Modifiers::none());
    settle(cx);
    assert!(
        view.read_with(cx, |view, _| view.marks.is_empty()),
        "a tap makes no box"
    );
    let from = spot("image-annotator", (0.1, 0.2), cx);
    let to = spot("image-annotator", (0.4, 0.6), cx);
    drag(from, &[from + point(px(10.0), px(10.0)), to], cx);
    let marks = view.read_with(cx, |view, _| view.marks.clone());
    let [Mark::Box { from, to, color: 0 }] = marks.as_slice() else {
        panic!("one box: {marks:?}");
    };
    let close = |a: (f32, f32), b: (f32, f32)| (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01;
    assert!(
        close(*from, (0.1, 0.2)) && close(*to, (0.4, 0.6)),
        "{marks:?}"
    );
}

#[gpui::test]
fn select_and_delete_remove_a_mark_and_undo_brings_it_back(cx: &mut TestAppContext) {
    let pin = Mark::Pin {
        at: (0.5, 0.5),
        color: 1,
    };
    let (view, cx) = marking(vec![pin.clone()], cx);
    let corner = spot("image-annotator", (0.1, 0.1), cx);
    cx.simulate_click(corner, Modifiers::none());
    settle(cx);
    press("v", cx);
    let middle = spot("image-annotator", (0.5, 0.5), cx);
    cx.simulate_click(middle, Modifiers::none());
    settle(cx);
    press("escape", cx);
    let escapes = view.read_with(cx, |view, _| view.escapes);
    assert_eq!(escapes, 0, "Escape lets the chosen pin go and stops there");
    cx.simulate_click(middle, Modifiers::none());
    settle(cx);
    press("backspace", cx);
    assert!(
        view.read_with(cx, |view, _| view.marks.is_empty()),
        "the pin is gone"
    );
    press("escape", cx);
    let escapes = view.read_with(cx, |view, _| view.escapes);
    assert_eq!(escapes, 1, "with nothing chosen, Escape passes to the page");
    press("cmd-z", cx);
    assert_eq!(view.read_with(cx, |view, _| view.marks.clone()), [pin]);
}
