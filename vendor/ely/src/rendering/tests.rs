use std::{cell::RefCell, rc::Rc};

use gpui::{
    Context, DevicePixels, IntoElement, ParentElement, Render, RenderImage, Size, Styled,
    TestAppContext, VisualTestContext, Window, div, px, size,
};
use image::{Frame, RgbaImage};

use super::RenderSurface;
use crate::{layout::tests::narrow_width, theme::Theme};

type Asked = Rc<RefCell<Vec<Size<DevicePixels>>>>;

/// A surface `width` wide at 2:1, its revision set by the test.
struct Host {
    width: f32,
    revision: u64,
    asked: Asked,
    off_by: i32,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (asked, off_by) = (self.asked.clone(), self.off_by);
        div().w(px(self.width)).child(
            RenderSurface::new("surface", 2.0, move |size, _| {
                asked.borrow_mut().push(size);
                frame(size.width.0 + off_by, size.height.0)
            })
            .revision(self.revision),
        )
    }
}

fn frame(width: i32, height: i32) -> RenderImage {
    RenderImage::new(vec![Frame::new(RgbaImage::new(
        width as u32,
        height as u32,
    ))])
}

fn hosted(
    cx: &mut TestAppContext,
    width: f32,
    off_by: i32,
) -> (gpui::Entity<Host>, &mut VisualTestContext, Asked) {
    cx.update(Theme::init);
    let asked = Asked::default();
    let seen = asked.clone();
    let (host, cx) = cx.add_window_view(move |_, _| Host {
        width,
        revision: 0,
        asked: seen,
        off_by,
    });
    redraw(cx);
    (host, cx, asked)
}

fn redraw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

fn px_size(width: i32, height: i32) -> Size<DevicePixels> {
    size(DevicePixels(width), DevicePixels(height))
}

#[gpui::test]
fn draws_once_at_the_box_size_in_device_pixels(cx: &mut TestAppContext) {
    let (_, cx, asked) = hosted(cx, 200.0, 0);
    assert_eq!(cx.update(|window, _| window.scale_factor()), 2.0);
    redraw(cx);
    redraw(cx);
    assert_eq!(
        *asked.borrow(),
        [px_size(400, 200)],
        "one frame for one size"
    );
    cx.update(|window, _| window.set_scale_factor(1.0));
    redraw(cx);
    assert_eq!(
        asked.borrow().last(),
        Some(&px_size(200, 100)),
        "a coarser screen asks for fewer pixels"
    );
}

#[gpui::test]
fn a_new_revision_or_width_draws_again(cx: &mut TestAppContext) {
    let (host, cx, asked) = hosted(cx, 200.0, 0);
    host.update(cx, |host, cx| {
        host.revision = 1;
        cx.notify();
    });
    redraw(cx);
    host.update(cx, |host, cx| {
        host.width = 120.0;
        cx.notify();
    });
    redraw(cx);
    assert_eq!(
        *asked.borrow(),
        [px_size(400, 200), px_size(400, 200), px_size(240, 120)]
    );
}

#[gpui::test]
fn an_empty_box_asks_for_nothing(cx: &mut TestAppContext) {
    let (_, _, asked) = hosted(cx, 0.0, 0);
    assert!(asked.borrow().is_empty(), "no frame for no pixels");
}

#[gpui::test]
#[should_panic(expected = "the host drew the wrong size")]
fn a_frame_of_the_wrong_size_fails(cx: &mut TestAppContext) {
    hosted(cx, 200.0, 1);
}

#[gpui::test]
fn the_surface_fills_its_column(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let width = narrow_width(cx, "render-surface", |_, _| {
        RenderSurface::new("surface", 2.0, |size, _| frame(size.width.0, size.height.0))
            .into_any_element()
    });
    assert_eq!(width, px(240.0));
}
