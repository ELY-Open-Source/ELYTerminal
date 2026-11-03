use gpui::{
    Context, Entity, IntoElement, Modifiers, MouseButton, ParentElement, Pixels, Point, Render,
    TestAppContext, VisualTestContext, Window, div, point, prelude::*, px,
};

use crate::shell::{ColumnShell, ShellColumn};
use crate::theme::{Platform, Theme};

/// Two set columns round a filling one; this view keeps their widths.
struct Sided {
    platform: Platform,
    width: Pixels,
    widths: [Pixels; 2],
    heard: Vec<Pixels>,
}

impl Render for Sided {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let side = |id: &'static str, ix: usize| {
            let view = view.clone();
            ShellColumn::new(id).title(id).resizable(
                self.widths[ix],
                px(100.0),
                px(400.0),
                move |width, _, cx| {
                    view.update(cx, |view, cx| {
                        view.widths[ix] = width;
                        view.heard.push(width);
                        cx.notify();
                    })
                },
            )
        };
        div().w(self.width).h(px(400.0)).flex().flex_col().child(
            ColumnShell::new("shell")
                .platform(self.platform)
                .column(side("left", 0))
                .column(ShellColumn::new("main").title("main"))
                .column(side("right", 1)),
        )
    }
}

fn sided(
    platform: Platform,
    widths: [Pixels; 2],
    cx: &mut TestAppContext,
) -> (Entity<Sided>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(|_, _| Sided {
        platform,
        width: px(1000.0),
        widths,
        heard: Vec::new(),
    });
    cx.run_until_parked();
    (view, cx)
}

fn column(ix: usize, cx: &mut VisualTestContext) -> gpui::Bounds<Pixels> {
    let name: &'static str = ["column-shell-0", "column-shell-1", "column-shell-2"][ix];
    cx.debug_bounds(name).expect("a column")
}

/// Presses at `from`, moves past gpui's drag threshold, then to each offset `path` names, and lets go.
fn drag_path(from: Point<Pixels>, path: &[Pixels], cx: &mut VisualTestContext) {
    let none = Modifiers::none();
    cx.simulate_mouse_down(from, MouseButton::Left, none);
    let set_off = from + point(px(0.0), px(4.0));
    cx.simulate_mouse_move(set_off, Some(MouseButton::Left), none);
    cx.run_until_parked();
    let mut to = set_off;
    for by in path {
        to = set_off + point(*by, px(0.0));
        cx.simulate_mouse_move(to, Some(MouseButton::Left), none);
        cx.run_until_parked();
    }
    cx.simulate_mouse_up(to, MouseButton::Left, none);
    cx.run_until_parked();
}

fn drag(from: Point<Pixels>, by: Pixels, cx: &mut VisualTestContext) {
    drag_path(from, &[by], cx);
}

fn heard(view: &Entity<Sided>, cx: &mut VisualTestContext) -> Option<Pixels> {
    view.read_with(cx, |view, _| view.heard.last().copied())
}

/// A column before the filling one drags by its right border, one after it by its left.
#[gpui::test]
fn a_border_toward_the_filling_column_drags_its_width(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (view, cx) = sided(Platform::Linux, [px(200.0), px(200.0)], cx);
    let left = cx.debug_bounds("column-shell-border-0").expect("a border");
    let right = cx.debug_bounds("column-shell-border-2").expect("a border");
    assert_eq!(left.right(), column(0, cx).right());
    assert_eq!(right.left(), column(2, cx).left());
    drag(left.center(), px(50.0), cx);
    assert_eq!(heard(&view, cx), Some(px(250.0)));
    assert_eq!(column(0, cx).size.width, px(250.0));
    drag(right.center(), px(-50.0), cx);
    assert_eq!(heard(&view, cx), Some(px(250.0)));
    let (main, panel) = (column(1, cx), column(2, cx));
    assert_eq!((panel.size.width, panel.right()), (px(250.0), px(1000.0)));
    assert_eq!((main.left(), main.right()), (px(250.0), px(750.0)));
}

/// A drag stops at the column's least and greatest width.
#[gpui::test]
fn a_border_drag_keeps_within_the_limits(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (view, cx) = sided(Platform::Linux, [px(200.0), px(200.0)], cx);
    let border = |cx: &mut VisualTestContext| {
        cx.debug_bounds("column-shell-border-0")
            .expect("a border")
            .center()
    };
    drag(border(cx), px(900.0), cx);
    assert_eq!(heard(&view, cx), Some(px(400.0)));
    drag(border(cx), px(-900.0), cx);
    assert_eq!(heard(&view, cx), Some(px(100.0)));
}

/// In a header the border takes the press, so no system's drag moves the window, which panics here.
#[gpui::test]
fn a_border_drag_in_a_header_moves_no_window(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    for platform in [Platform::Windows, Platform::Mac, Platform::Linux] {
        let (view, cx) = sided(platform, [px(200.0), px(200.0)], cx);
        for (ix, by) in [(0, px(30.0)), (2, px(-30.0))] {
            let border = cx
                .debug_bounds(["column-shell-border-0", "", "column-shell-border-2"][ix])
                .expect("a border");
            let header = column(ix, cx).top() + px(4.0);
            drag(point(border.center().x, header), by, cx);
            assert_eq!(heard(&view, cx), Some(px(230.0)), "{platform:?}");
        }
    }
}

/// A header's own parts hold a column past the owner's width; a drag sets off from the drawn one.
#[gpui::test]
fn a_border_drag_sets_off_from_the_drawn_width(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (view, cx) = sided(Platform::Windows, [px(200.0), px(100.0)], cx);
    let (panel, close) = (column(2, cx), cx.debug_bounds("caption-Close"));
    let close = close.expect("a close button");
    assert!(close.right() <= panel.right(), "{close:?} in {panel:?}");
    let drawn = panel.size.width;
    assert!(drawn > px(100.0), "{drawn:?} holds the caption buttons");
    let border = cx.debug_bounds("column-shell-border-2").expect("a border");
    drag(border.center(), px(-10.0), cx);
    assert_eq!(heard(&view, cx), Some(drawn + px(10.0)));
}

/// Two set columns and nothing that fills.
struct Unfilled;

impl Render for Unfilled {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let column = |id: &'static str| ShellColumn::new(id).width(px(200.0));
        ColumnShell::new("shell")
            .platform(Platform::Linux)
            .column(column("first"))
            .column(column("last").resizable(px(200.0), px(100.0), px(400.0), |_, _, _| {}))
    }
}

/// The last column of a shell that nothing fills has no border to drag.
#[gpui::test]
#[should_panic(expected = "no border lies toward a filling column")]
fn a_resizing_column_needs_a_border_toward_a_filling_one(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| Unfilled);
    cx.run_until_parked();
}

/// A short shell squeezes both columns; a drag moves a border with the pointer, from the owner's width.
#[gpui::test]
fn a_squeezed_column_drags_from_the_owner_s_width(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (view, cx) = cx.add_window_view(|_, _| Sided {
        platform: Platform::Linux,
        width: px(700.0),
        widths: [px(350.0), px(350.0)],
        heard: Vec::new(),
    });
    cx.run_until_parked();
    let left = column(0, cx);
    assert!(left.size.width < px(350.0), "{left:?} is squeezed");
    let border = |ix: usize, cx: &mut VisualTestContext| {
        let name = ["column-shell-border-0", "", "column-shell-border-2"][ix];
        cx.debug_bounds(name).expect("a border").center()
    };
    drag(border(0, cx), px(0.0), cx);
    assert_eq!(heard(&view, cx), None, "a drag with no sideways move");
    drag(border(0, cx), px(20.0), cx);
    assert_eq!(heard(&view, cx), Some(px(370.0)));
    assert!(
        column(0, cx).right() > left.right(),
        "the left border moves right"
    );
    let right = column(2, cx);
    drag(border(2, cx), px(-20.0), cx);
    assert_eq!(heard(&view, cx), Some(px(370.0)));
    assert!(
        column(2, cx).left() < right.left(),
        "the right border moves left"
    );
}

/// A drag that comes back where it set off gives the width the owner had, a held or squeezed one too.
#[gpui::test]
fn a_drag_back_where_it_set_off_gives_the_width_it_had(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let cases = [
        (
            Platform::Linux,
            px(1000.0),
            [px(200.0), px(200.0)],
            0,
            px(50.0),
        ),
        (
            Platform::Linux,
            px(1000.0),
            [px(200.0), px(200.0)],
            2,
            px(-50.0),
        ),
        (
            Platform::Windows,
            px(1000.0),
            [px(200.0), px(100.0)],
            2,
            px(-10.0),
        ),
        (
            Platform::Linux,
            px(700.0),
            [px(350.0), px(350.0)],
            0,
            px(20.0),
        ),
    ];
    for (platform, width, widths, ix, by) in cases {
        let (view, cx) = cx.add_window_view(move |_, _| Sided {
            platform,
            width,
            widths,
            heard: Vec::new(),
        });
        cx.run_until_parked();
        let name = ["column-shell-border-0", "", "column-shell-border-2"][ix];
        let border = cx.debug_bounds(name).expect("a border").center();
        let drawn = column(ix, cx);
        drag_path(border, &[by, px(0.0)], cx);
        let heard = view.read_with(cx, |view, _| view.heard.clone());
        assert_eq!(heard.len(), 2, "{platform:?} {ix}: out and back, {heard:?}");
        assert_eq!(heard[1], widths[ix / 2], "{platform:?} {ix}");
        assert_eq!(column(ix, cx), drawn, "{platform:?} {ix}: drawn as before");
    }
}
