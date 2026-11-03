use gpui::{
    Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div,
    prelude::FluentBuilder,
};

use super::setup;
use crate::{
    navigation::Pagination,
    theme::{ActiveTheme, ControlSize, TextSize},
    typography::text_width,
};

struct Paged {
    size: ControlSize,
    page: usize,
    jump: bool,
    picked: Vec<usize>,
}

impl Render for Paged {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let text = cx.theme().text_size(TextSize::Base);
        div().text_size(text).child(
            Pagination::new("pages", self.page, 100)
                .size(self.size)
                .when(self.jump, Pagination::jump)
                .on_change(move |to, _, cx| {
                    view.update(cx, |paged, cx| {
                        paged.picked.push(to);
                        paged.page = to;
                        cx.notify();
                    })
                }),
        )
    }
}

#[gpui::test]
fn the_jump_field_goes_to_the_page_typed_within_range(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Paged {
        size: ControlSize::Sm,
        page: 3,
        jump: true,
        picked: Vec::new(),
    });
    cx.update(|window, cx| {
        window.activate_window();
        window.focus_prev(cx);
    });
    for typed in ["78", "500"] {
        cx.simulate_keystrokes("secondary-a");
        cx.simulate_input(typed);
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
    }
    cx.simulate_keystrokes("secondary-a");
    cx.simulate_input("88");
    cx.update(|window, cx| window.blur(cx));
    cx.run_until_parked();
    assert_eq!(
        view.read_with(cx, |paged, _| paged.picked.clone()),
        [78, 100, 88]
    );
}

#[gpui::test]
fn every_control_takes_the_size_asked(cx: &mut TestAppContext) {
    setup(cx);
    for size in [ControlSize::Sm, ControlSize::Md, ControlSize::Lg] {
        let (_, cx) = cx.add_window_view(move |_, _| Paged {
            size,
            page: 3,
            jump: false,
            picked: Vec::new(),
        });
        cx.run_until_parked();
        let tall =
            cx.update(|window, cx| cx.theme().control_height(size).to_pixels(window.rem_size()));
        for control in [
            "pagination-previous",
            "pagination-page-3",
            "pagination-page-4",
            "pagination-next",
        ] {
            let bounds = cx.debug_bounds(control).expect("drawn");
            assert_eq!(bounds.size.height, tall, "{control} at {size:?}");
        }
    }
}

#[gpui::test]
fn the_jump_field_holds_the_last_page_at_every_size(cx: &mut TestAppContext) {
    setup(cx);
    let sizes = [
        (ControlSize::Sm, TextSize::Sm),
        (ControlSize::Md, TextSize::Base),
        (ControlSize::Lg, TextSize::Md),
    ];
    for (size, text) in sizes {
        let (_, cx) = cx.add_window_view(move |_, _| Paged {
            size,
            page: 100,
            jump: true,
            picked: Vec::new(),
        });
        cx.run_until_parked();
        let slot = cx.debug_bounds("input-text").expect("the field draws");
        let digits = cx.update(|window, cx| {
            let text = cx.theme().text_size(text);
            text_width("100", text.to_pixels(window.rem_size()), window)
        });
        assert!(
            slot.size.width >= digits,
            "{size:?}: {slot:?} under {digits:?}"
        );
    }
}

/// Pages the owner has not caught up with: `page` of `pages`.
struct Behind(usize, usize);

impl Render for Behind {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().child(Pagination::new("behind", self.0, self.1).jump())
    }
}

#[gpui::test]
fn a_page_past_the_last_or_no_pages_marks_none(cx: &mut TestAppContext) {
    setup(cx);
    let (_, cx) = cx.add_window_view(|_, _| Behind(7, 3));
    cx.run_until_parked();
    assert!(cx.debug_bounds("pagination-page-3").is_some());
    assert!(cx.debug_bounds("pagination-page-7").is_none());
    let (_, cx) = cx.add_window_view(|_, _| Behind(1, 0));
    cx.run_until_parked();
    assert!(cx.debug_bounds("pagination-page-1").is_none());
    assert!(
        cx.debug_bounds("number-root").is_none(),
        "no field to jump with"
    );
}
