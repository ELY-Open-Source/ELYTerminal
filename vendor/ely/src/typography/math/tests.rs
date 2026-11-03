use gpui::AppContext as _;

use super::*;

fn formula(source: &str, display: bool) -> Formula {
    Formula {
        source: source.to_string().into(),
        display,
        rgba: 0x112233ff,
        font: 1600,
        scale: 200,
    }
}

#[test]
fn a_formula_typesets_to_outlined_svg_in_its_color() {
    let (svg, shape) = svg(&formula(r"\frac{a}{b} + \sqrt{x}", false)).expect("typesets");
    assert!(svg.starts_with("<svg") && !svg.contains("<text"), "{svg}");
    assert!(svg.contains("17,34,51"), "the color: {svg}");
    assert!(shape.width > px(16.) && shape.height > px(16.), "{shape:?}");
}

#[test]
fn display_style_sets_limits_larger() {
    let (_, text) = svg(&formula(r"\sum_{k=1}^{n} k", false)).expect("typesets");
    let (_, display) = svg(&formula(r"\sum_{k=1}^{n} k", true)).expect("typesets");
    assert!(display.height > text.height, "{display:?} over {text:?}");
}

#[test]
fn every_glyph_lies_inside_the_picture() {
    for source in [
        r"\text{私密} x",
        "x",
        r"\int_0^1 f(x)\,dx",
        r"\left(\frac{a}{b}\right)",
    ] {
        let (svg, shape) = svg(&formula(source, false)).expect("typesets");
        // Measured with the fonts it draws with.
        let options = crate::documents::svg_options();
        let tree = usvg::Tree::from_data(svg.as_bytes(), &options).expect("svg");
        let ink = tree.root().abs_stroke_bounding_box();
        let (w, h) = (shape.width.as_f32(), shape.height.as_f32());
        let near = |a: f32, b: f32| (a - b).abs() < 0.01;
        assert!(
            near(tree.size().width(), w) && near(tree.size().height(), h),
            "{source}"
        );
        assert!(
            ink.left() >= -0.01 && ink.top() >= -0.01,
            "{source}: ink {ink:?} starts before the picture"
        );
        assert!(
            ink.right() <= w + 0.01 && ink.bottom() <= h + 0.01,
            "{source}: ink {ink:?} passes {w}×{h}"
        );
    }
}

#[test]
fn bad_tex_says_why_and_where() {
    let Err(error) = Latex::parse("x", r"x^2^3") else {
        panic!("a double superscript parses")
    };
    assert_eq!(error.to_string(), "Double superscript at 3");
    assert!(Latex::parse("x", r"x^{2").is_err(), "an open brace");
}

#[test]
fn a_layout_panic_is_an_error_without_the_panic_text() {
    let laid: Result<(), MathError> = guarded(|| panic!("private formula text"));
    let error = laid.expect_err("the panic is an error");
    assert!(!error.to_string().contains("private"), "{error}");
}

/// Formulas drawn by id; the test changes them.
struct Shown(Vec<&'static str>);

impl gpui::Render for Shown {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        div().children(
            self.0
                .iter()
                .enumerate()
                .map(|(ix, source)| Latex::parse(ix, source).expect("parses")),
        )
    }
}

fn open<'a>(
    sources: Vec<&'static str>,
    cx: &'a mut gpui::TestAppContext,
) -> (gpui::Entity<Shown>, &'a mut gpui::VisualTestContext) {
    cx.update(crate::theme::Theme::init);
    let (view, cx) = cx.add_window_view(|_, _| Shown(sources));
    settle(cx);
    (view, cx)
}

fn settle(cx: &mut gpui::VisualTestContext) {
    for _ in 0..3 {
        cx.run_until_parked();
        cx.update(|window, _| window.refresh());
    }
    cx.run_until_parked();
}

/// Draws, then lets gpui release what the frame dropped.
fn flush(cx: &mut gpui::VisualTestContext) {
    cx.run_until_parked();
    cx.update(|_, _| ());
}

/// Whether `source`'s picture sits in the asset cache.
fn cached(source: &str, cx: &mut gpui::VisualTestContext) -> bool {
    cx.update(|window, cx| {
        let font = cx
            .theme()
            .text_size(TextSize::Lg)
            .to_pixels(window.rem_size());
        let formula = super::formula((&source.to_string().into(), false), font, window, cx);
        cx.has_asset::<Typesetter>(&formula)
    })
}

#[gpui::test]
fn a_formula_frees_its_picture_when_it_changes_or_leaves(cx: &mut gpui::TestAppContext) {
    let (view, cx) = open(vec!["a^2"], cx);
    assert!(cached("a^2", cx), "drawn once");
    view.update(cx, |view, cx| {
        view.0 = vec!["b^2"];
        cx.notify();
    });
    settle(cx);
    assert!(!cached("a^2", cx) && cached("b^2", cx), "the old one freed");
    view.update(cx, |view, cx| {
        view.0.clear();
        cx.notify();
    });
    settle(cx);
    assert!(!cached("b^2", cx), "gone with its element");
}

/// One formula in a view of its own.
struct One;

impl gpui::Render for One {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        Latex::parse("one", "c^2").expect("parses")
    }
}

/// Views of one formula, each drawn from gpui's view cache.
struct Pair(Vec<gpui::Entity<One>>);

impl gpui::Render for Pair {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        div().children(
            self.0.iter().map(|one| {
                gpui::AnyView::from(one.clone()).cached(gpui::StyleRefinement::default())
            }),
        )
    }
}

#[gpui::test]
fn a_shared_formula_stays_until_its_last_holder_leaves(cx: &mut gpui::TestAppContext) {
    cx.update(crate::theme::Theme::init);
    let (pair, cx) = cx.add_window_view(|_, cx| Pair(vec![cx.new(|_| One), cx.new(|_| One)]));
    settle(cx);
    assert!(cached("c^2", cx), "drawn");
    pair.update(cx, |pair, cx| {
        pair.0.pop();
        cx.notify();
    });
    flush(cx);
    assert!(cached("c^2", cx), "the cached survivor still holds it");
    pair.update(cx, |pair, cx| {
        pair.0.clear();
        cx.notify();
    });
    flush(cx);
    assert!(!cached("c^2", cx), "the last holder freed it");
}

#[test]
fn an_exported_formula_is_black_outlines() {
    let svg = formula_svg(r"\sqrt{x}", true).expect("typesets");
    assert!(svg.starts_with("<svg") && !svg.contains("<text"), "{svg}");
    assert!(svg.contains("#000000"), "black: {svg}");
    assert!(formula_svg(r"\frac{", false).is_err());
}

#[test]
fn an_exported_formula_outlines_its_text_or_says_why() {
    let svg = formula_svg(r"\text{中文} + x", false).expect("a system font sets it");
    assert!(!svg.contains("<text"), "{svg}");
    let why = formula_svg("\\text{\u{F0000}}", false).expect_err("no font");
    assert!(why.to_string().contains("font"), "{why}");
}

#[test]
fn an_exported_formula_frames_its_outlined_text() {
    // Wide or tall system faces, where installed.
    for source in [r"\text{한글}", r"\text{𓀀} + x"] {
        let Ok(svg) = formula_svg(source, false) else {
            continue;
        };
        let tree =
            usvg::Tree::from_data(svg.as_bytes(), &usvg::Options::default()).expect("parses");
        let (ink, shape) = (tree.root().abs_stroke_bounding_box(), tree.size());
        let inside = ink.left() >= -0.01
            && ink.top() >= -0.01
            && ink.right() <= shape.width() + 0.01
            && ink.bottom() <= shape.height() + 0.01;
        assert!(inside, "{source}: {ink:?} in {shape:?}");
    }
}

#[test]
fn drawn_fallback_text_stays_inside_the_frame() {
    let renderer = gpui::SvgRenderer::new(std::sync::Arc::new(crate::Assets));
    let ink = |svg: &str| {
        let image = renderer
            .render_single_frame(svg.as_bytes(), 4.0)
            .expect("draws");
        let pixels = image.as_bytes(0).expect("pixels");
        pixels.chunks_exact(4).filter(|pixel| pixel[3] > 16).count()
    };
    // The last is in gpui's bundled IBM Plex Sans alone.
    for source in [r"\text{中文}", r"\text{한글}", "\\text{\u{ECE0}}"] {
        let (svg, _) = svg(&formula(source, false)).expect("typesets");
        let tree = usvg::roxmltree::Document::parse(&svg).expect("xml");
        let view: Vec<f32> = tree
            .root_element()
            .attribute("viewBox")
            .expect("a view box")
            .split_whitespace()
            .map(|number| number.parse().expect("a number"))
            .collect();
        // The same ink in a frame wider all round.
        let wide = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" width=\"{}\" height=\"{}\">{}",
            view[0] - 64.0,
            view[1] - 64.0,
            view[2] + 128.0,
            view[3] + 128.0,
            view[2] + 128.0,
            view[3] + 128.0,
            &svg[svg.find('>').expect("a tag") + 1..]
        );
        let (framed, all) = (ink(&svg), ink(&wide));
        assert!(
            framed as f32 >= all as f32 * 0.95,
            "{source}: {framed} of {all} pixels"
        );
    }
}

#[test]
fn a_picture_spans_its_formula_not_a_point_rescale() {
    let formula = super::Formula {
        source: "x^2+y^2=r^2".into(),
        display: false,
        rgba: 0xff,
        font: 1600,
        scale: 100,
    };
    let (_, width, _) = super::laid(&formula).expect("laid");
    let (_, shape) = super::svg(&formula).expect("framed");
    let ratio = shape.width.as_f32() / width;
    assert!((0.95..1.05).contains(&ratio), "{} for {width}", shape.width);
}
