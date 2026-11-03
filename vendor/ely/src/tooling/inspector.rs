use std::fmt;

use gpui::{
    AbsoluteLength, AnyElement, App, BorderStyle, Bounds, Context, Corners, DefiniteLength,
    DivInspectorState, Edges, FontWeight, Inspector, InteractiveElement, IntoElement, Length,
    ParentElement, Pixels, StatefulInteractiveElement, StyleRefinement, Styled, TextAlign, TextRun,
    Window, canvas, div, fill, point, quad, size, transparent_black,
};

use crate::{
    buttons::{Button, ButtonVariant},
    primitives::{FocusScope, raise},
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
    typography::{Ellipsis, LEADING, literal, tabular},
};

// Chart hues Rust, Ochre, Green and Blue, as browsers color a box's layers.
const MARGIN: usize = 6;
const BORDER: usize = 2;
const PADDING: usize = 5;
const CONTENT: usize = 0;

/// One side of a box as written: pixels, `auto`, or a share of the parent's width.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Side {
    Px(f32),
    Auto,
    Share(f32),
}

impl Side {
    /// What the overlay draws: `auto` and shares only layout knows, so they draw nothing.
    fn drawn(self) -> Pixels {
        match self {
            Side::Px(px) => Pixels::from(px),
            Side::Auto | Side::Share(_) => Pixels::ZERO,
        }
    }
}

fn number(value: f32) -> String {
    ((value * 10.0).round() / 10.0).to_string()
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Side::Px(px) => write!(f, "{}", number(*px)),
            Side::Auto => write!(f, "auto"),
            Side::Share(share) => write!(f, "{}%", number(share * 100.0)),
        }
    }
}

/// A box's margin, border and padding, each top, right, bottom, left.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Layers {
    margin: [Side; 4],
    border: [Side; 4],
    padding: [Side; 4],
}

/// The layers as the style writes them, rems at `rem`.
fn layers(style: &StyleRefinement, rem: Pixels) -> Layers {
    let definite = |length: DefiniteLength| match length {
        DefiniteLength::Absolute(length) => Side::Px(length.to_pixels(rem).into()),
        DefiniteLength::Fraction(share) => Side::Share(share),
    };
    let margin = |length: Option<Length>| match length {
        None => Side::Px(0.0),
        Some(Length::Auto) => Side::Auto,
        Some(Length::Definite(length)) => definite(length),
    };
    let padding = |length: Option<DefiniteLength>| length.map_or(Side::Px(0.0), definite);
    let border = |length: Option<AbsoluteLength>| {
        Side::Px(length.map_or(0.0, |length| length.to_pixels(rem).into()))
    };
    let (m, b, p) = (&style.margin, &style.border_widths, &style.padding);
    Layers {
        margin: [m.top, m.right, m.bottom, m.left].map(margin),
        border: [b.top, b.right, b.bottom, b.left].map(border),
        padding: [p.top, p.right, p.bottom, p.left].map(padding),
    }
}

fn edges(sides: [Side; 4]) -> Edges<Pixels> {
    let [top, right, bottom, left] = sides.map(Side::drawn);
    Edges {
        top,
        right,
        bottom,
        left,
    }
}

fn shrink(bounds: Bounds<Pixels>, by: Edges<Pixels>) -> Bounds<Pixels> {
    let inner = bounds.extend(Edges {
        top: -by.top,
        right: -by.right,
        bottom: -by.bottom,
        left: -by.left,
    });
    Bounds::new(
        inner.origin,
        inner.size.max(&size(Pixels::ZERO, Pixels::ZERO)),
    )
}

/// Paints the layers over the page at the box's bounds, with a tag that gives its size.
fn paint_layers(bounds: Bounds<Pixels>, layers: Layers, window: &mut Window, cx: &mut App) {
    let theme = cx.theme();
    let colors = &theme.colors;
    let hue = |hue| colors.hue(hue, "inspector");
    let band = |bounds, widths, hue| {
        quad(
            bounds,
            Corners::default(),
            transparent_black(),
            widths,
            colors.hue(hue, "inspector").alpha(0.4),
            BorderStyle::Solid,
        )
    };
    let (margin, border, padding) = (
        edges(layers.margin),
        edges(layers.border),
        edges(layers.padding),
    );
    let (outer, inner) = (bounds.extend(margin), shrink(bounds, border));
    window.paint_quad(band(outer, margin, MARGIN));
    window.paint_quad(band(bounds, border, BORDER));
    window.paint_quad(band(inner, padding, PADDING));
    window.paint_quad(fill(shrink(inner, padding), hue(CONTENT).alpha(0.25)));
    let text = theme.text_size(TextSize::Xs).to_pixels(window.rem_size());
    let label = format!(
        "{} × {}",
        number(bounds.size.width.into()),
        number(bounds.size.height.into())
    );
    let run = TextRun {
        len: label.len(),
        font: window.text_style().font(),
        color: colors.bg,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let line = window
        .text_system()
        .shape_line(label.into(), text, &[run], None);
    let (height, pad) = (text * LEADING, text / 2.0);
    let top = (outer.top() - height).max(Pixels::ZERO);
    let tag = Bounds::new(
        point(outer.left(), top),
        size(line.width + pad * 2.0, height),
    );
    let radius = theme.radius(Radius::Sm).to_pixels(window.rem_size());
    window.paint_quad(fill(tag, colors.fg).corner_radii(radius));
    if let Err(error) = line.paint(
        point(tag.left() + pad, tag.top()),
        height,
        TextAlign::Left,
        None,
        window,
        cx,
    ) {
        log::error!("inspector: size tag did not paint: {error:#}");
    }
}

/// One layer of the box model: its name, its four sides around what it holds.
fn ring(
    name: &'static str,
    sides: [Side; 4],
    hue: usize,
    inside: AnyElement,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let color = theme.colors.hue(hue, "inspector");
    let value = |side: Side| tabular(div()).child(side.to_string());
    let shown = sides.map(|side| side.to_string()).join("-");
    div()
        .debug_selector(move || format!("inspector-{name}-{shown}"))
        .relative()
        .flex()
        .flex_col()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .rounded(theme.radius(Radius::Sm))
        .border_1()
        .border_color(color)
        .bg(color.alpha(0.12))
        .child(
            div()
                .absolute()
                .top_1()
                .left_2()
                .text_color(theme.colors.fg_muted)
                .child(name),
        )
        .child(value(sides[0]))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(value(sides[3]))
                .child(inside)
                .child(value(sides[1])),
        )
        .child(value(sides[2]))
        .into_any_element()
}

/// A picked box's model, sizes and place, and its layers drawn over the page.
fn box_model(state: &DivInspectorState, window: &mut Window, cx: &mut App) -> AnyElement {
    let (bounds, children) = (state.bounds, state.content_size);
    let layers = layers(&state.base_style, window.rem_size());
    let content = shrink(shrink(bounds, edges(layers.border)), edges(layers.padding)).size;
    let pair = |w: Pixels, h: Pixels| format!("{} × {}", number(w.into()), number(h.into()));
    let theme = cx.theme();
    let fact = |name: &'static str, value: String| {
        div()
            .flex()
            .justify_between()
            .gap_2()
            .child(div().text_color(theme.colors.fg_muted).child(name))
            .child(tabular(div()).child(value))
    };
    let sized = format!(
        "{}x{}",
        number(bounds.size.width.into()),
        number(bounds.size.height.into())
    );
    let core = div()
        .debug_selector(move || {
            format!(
                "inspector-content-{}x{}",
                number(content.width.into()),
                number(content.height.into())
            )
        })
        .px_2()
        .py_1()
        .rounded(theme.radius(Radius::Sm))
        .border_1()
        .border_color(theme.colors.hue(CONTENT, "inspector"))
        .bg(theme.colors.hue(CONTENT, "inspector").alpha(0.12))
        .child(tabular(div()).child(pair(content.width, content.height)))
        .into_any_element();
    let padded = ring("padding", layers.padding, PADDING, core, cx);
    let bordered = ring("border", layers.border, BORDER, padded, cx);
    let model = ring("margin", layers.margin, MARGIN, bordered, cx);
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .gap_3()
        .text_size(theme.text_size(TextSize::Xs))
        // Raised, so the panel's scroll box does not clip it off the page.
        .child(raise(
            "ely-inspector-overlay",
            canvas(
                |_, _, _| {},
                move |_, _, window, cx| paint_layers(bounds, layers, window, cx),
            )
            .absolute()
            .top_0()
            .left_0(),
        ))
        .child(model)
        .child(
            div()
                .debug_selector(move || format!("inspector-size-{sized}"))
                .flex()
                .flex_col()
                .gap_1()
                .child(fact(
                    "Place",
                    format!(
                        "{}, {}",
                        number(bounds.origin.x.into()),
                        number(bounds.origin.y.into())
                    ),
                ))
                .child(fact("Size", pair(bounds.size.width, bounds.size.height)))
                .child(fact("Children", pair(children.width, children.height))),
        )
        .into_any_element()
}

/// The inspector's panel: how to pick, the picked box's source and ids, and its model.
fn panel(
    inspector: &mut Inspector,
    window: &mut Window,
    cx: &mut Context<Inspector>,
) -> AnyElement {
    let picking = inspector.is_picking();
    if picking {
        // gpui holds a pick on a press without a redraw, so the panel draws each frame while picking.
        window.request_animation_frame();
    }
    let place = inspector.active_element_id().map(|id| {
        let location = id.path.source_location;
        let ids: Vec<String> = id.path.global_id.iter().map(ToString::to_string).collect();
        (
            format!("{}:{}", location.file(), location.line()),
            ids[ids.len().saturating_sub(3)..].join(" › "),
        )
    });
    let states = inspector.render_inspector_states(window, cx);
    let focus = window
        .use_keyed_state("ely-inspector-focus", cx, |_, cx| cx.focus_handle())
        .read(cx)
        .clone();
    let owner = cx.entity();
    let theme = cx.theme();
    let colors = &theme.colors;
    let hint = match picking {
        true => {
            "Hover picks a box, the wheel steps out to the boxes around it, and a press holds it."
        }
        false => "Held. Pick chooses another box.",
    };
    let body = div()
        .id("ely-inspector")
        .size_full()
        .flex()
        .flex_col()
        .gap_4()
        .p_4()
        .overflow_y_scroll()
        .bg(colors.surface)
        .border_l_1()
        .border_color(colors.border)
        .text_color(colors.fg)
        .text_size(theme.text_size(TextSize::Sm))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(div().font_weight(FontWeight::SEMIBOLD).child("Inspector"))
                .children((!picking).then(|| {
                    div().debug_selector(|| "inspector-pick".into()).child(
                        Button::new("ely-inspector-pick", "Pick")
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Sm)
                            .on_click(move |_, window, cx| {
                                owner.update(cx, |inspector, _| inspector.start_picking());
                                log::info!("inspector: picking");
                                window.refresh();
                            }),
                    )
                })),
        )
        .child(
            div()
                .debug_selector(move || format!("inspector-picking-{picking}"))
                .text_color(colors.fg_muted)
                .child(hint),
        )
        .children(place.map(|(source, ids)| {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(literal(div()).child(Ellipsis::new(source)))
                .child(
                    literal(div())
                        .text_color(colors.fg_muted)
                        .child(Ellipsis::new(ids)),
                )
        }))
        .children(states);
    // A second root, out of the app's scope: it owns Tab itself.
    FocusScope::new(&focus)
        .size_full()
        .child(body)
        .into_any_element()
}

/// Draws gpui's inspector, which debug builds have, in Ely's look. `window.toggle_inspector` opens it at the window's right, and the page gives up its width. Hovering picks the box under the pointer, the wheel steps out to the boxes around it, and a press holds it; the picked box shows its margin, border, padding and size over the page, and its model in the panel.
pub fn install_inspector(cx: &mut App) {
    cx.set_inspector_renderer(Box::new(panel));
    cx.register_inspector_element(|_, _| {
        |_, state: &DivInspectorState, window: &mut Window, cx: &mut App| {
            box_model(state, window, cx)
        }
    });
    log::info!("inspector: installed");
}

#[cfg(test)]
mod tests {
    use gpui::{StyleRefinement, Styled, div, px, relative, rems};

    use super::{Side, layers};

    #[test]
    fn layers_read_as_written() {
        let mut style = div().mt(px(4.0)).mx_auto().p(rems(0.5)).border_2();
        let written = layers(style.style(), px(16.0));
        assert_eq!(
            written.margin,
            [Side::Px(4.0), Side::Auto, Side::Px(0.0), Side::Auto]
        );
        assert_eq!(written.padding, [Side::Px(8.0); 4]);
        assert_eq!(written.border, [Side::Px(2.0); 4]);
        let bare = layers(&StyleRefinement::default(), px(16.0));
        assert_eq!(
            bare.border,
            [Side::Px(0.0); 4],
            "no border written, none read"
        );
        let mut share = StyleRefinement::default();
        share.padding.left = Some(relative(0.25));
        let shown = layers(&share, px(16.0))
            .padding
            .map(|side| side.to_string());
        assert_eq!(shown, ["0", "0", "0", "25%"]);
        assert_eq!(Side::Share(0.25).drawn(), px(0.0), "a share draws nothing");
    }
}
