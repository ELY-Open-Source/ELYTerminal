use gpui::{
    AnyElement, App, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement, Pixels,
    Rems, RenderOnce, Rgba, Styled, Window, div, relative, rems,
};

use crate::{
    forms::hex,
    primitives::{Icon, IconName},
    theme::{
        ActiveTheme, AvatarSize, ContainerSize, ControlSize, Elevation, HUE_NAMES, IconSize,
        Palette, Radius, Syntax, TextSize,
    },
    typography::{Ellipsis, literal, tabular},
};

const ANSI: [&str; 16] = [
    "black",
    "red",
    "green",
    "yellow",
    "blue",
    "magenta",
    "cyan",
    "white",
    "bright black",
    "bright red",
    "bright green",
    "bright yellow",
    "bright blue",
    "bright magenta",
    "bright cyan",
    "bright white",
];

/// gpui's spacing steps shown, each a quarter rem, as `gap_2` reads.
const STEPS: [f32; 6] = [1.0, 2.0, 3.0, 4.0, 6.0, 8.0];

fn pixels(value: Pixels) -> String {
    ((f32::from(value) * 10.0).round() / 10.0).to_string()
}

/// A titled group of tokens.
fn group(title: &'static str, cx: &App) -> gpui::Div {
    div().flex().flex_col().gap_3().child(
        div()
            .text_size(cx.theme().text_size(TextSize::Sm))
            .font_weight(FontWeight::SEMIBOLD)
            .child(title),
    )
}

/// A color's tile: its swatch, its name as code reads it, and its hex.
fn swatch(name: String, color: Hsla, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let sizes = theme.tooling();
    let code = hex(Rgba::from(color));
    let shown = format!("token-{name}-{code}");
    div()
        .debug_selector(move || shown)
        .flex()
        .items_center()
        .gap_2()
        .w(sizes.tile)
        .min_w_0()
        .child(
            div()
                .flex_none()
                .size(sizes.swatch)
                .rounded(theme.radius(Radius::Md))
                .border_1()
                .border_color(theme.colors.border)
                .bg(color),
        )
        .child(
            literal(div())
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .text_size(theme.text_size(TextSize::Xs))
                .child(Ellipsis::new(name))
                .child(div().text_color(theme.colors.fg_muted).child(code)),
        )
        .into_any_element()
}

/// A measure's row: its name as code reads it and its size in pixels, then a sample of it.
fn measure(name: String, size: Pixels, sample: impl IntoElement, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let value = pixels(size);
    let shown = format!("token-{name}-{value}");
    div()
        .debug_selector(move || shown)
        .flex()
        .flex_wrap()
        .items_center()
        .gap_x_4()
        .gap_y_1()
        .child(
            div()
                .flex_none()
                .w(theme.tooling().tile)
                .flex()
                .justify_between()
                .gap_2()
                .text_size(theme.text_size(TextSize::Xs))
                .child(literal(div()).child(name))
                .child(
                    tabular(div())
                        .text_color(theme.colors.fg_muted)
                        .child(format!("{value} px")),
                ),
        )
        .child(div().flex_1().min_w_0().flex().items_center().child(sample))
        .into_any_element()
}

/// The theme's shared colors and scales in the mode shown, each named as code reads it: every palette, syntax, chart and terminal color with its hex, and the type, radius, control, icon, avatar and container sizes, the elevations and gpui's spacing steps. A chapter's own measures stay with the chapter.
#[derive(IntoElement, Default)]
pub struct DesignTokenViewer;

impl DesignTokenViewer {
    pub fn new() -> Self {
        Self
    }
}

impl RenderOnce for DesignTokenViewer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let rem = window.rem_size();
        let at = |size: Rems| size.to_pixels(rem);
        let theme = cx.theme();
        let (colors, sizes) = (&theme.colors, theme.tooling());
        let tiles = |title, tiles: Vec<AnyElement>| {
            group(title, cx).child(div().flex().flex_wrap().gap_3().children(tiles))
        };
        let palette = Palette::NAMES
            .iter()
            .map(|name| swatch(format!("colors.{name}"), colors.token(name), cx));
        let syntax = Syntax::NAMES.iter().map(|name| {
            swatch(
                format!("colors.syntax.{name}"),
                colors.syntax.token(name),
                cx,
            )
        });
        let chart = HUE_NAMES.iter().enumerate().map(|(ix, name)| {
            swatch(
                format!("colors.hue({ix}) {name}"),
                colors.hue(ix, "token viewer"),
                cx,
            )
        });
        let ansi = ANSI
            .iter()
            .zip(colors.ansi)
            .enumerate()
            .map(|(ix, (name, color))| swatch(format!("colors.ansi[{ix}] {name}"), color, cx));
        let text = [
            (TextSize::Xs, "Xs"),
            (TextSize::Sm, "Sm"),
            (TextSize::Base, "Base"),
            (TextSize::Md, "Md"),
            (TextSize::Lg, "Lg"),
            (TextSize::Xl, "Xl"),
            (TextSize::Xxl, "Xxl"),
            (TextSize::Display, "Display"),
        ]
        .map(|(size, name)| {
            let sample = div()
                .min_w_0()
                .text_size(theme.text_size(size))
                .child(Ellipsis::new("Sphinx of black quartz"));
            measure(
                format!("TextSize::{name}"),
                at(theme.text_size(size)),
                sample,
                cx,
            )
        });
        let radii = [
            (Radius::Sm, "Sm"),
            (Radius::Md, "Md"),
            (Radius::Lg, "Lg"),
            (Radius::Xl, "Xl"),
        ]
        .map(|(radius, name)| {
            let sample = div()
                .size(sizes.swatch)
                .rounded(theme.radius(radius))
                .border_1()
                .border_color(colors.border_strong)
                .bg(colors.sunken);
            measure(
                format!("Radius::{name}"),
                at(theme.radius(radius)),
                sample,
                cx,
            )
        });
        let controls = [
            (ControlSize::Sm, "Sm"),
            (ControlSize::Md, "Md"),
            (ControlSize::Lg, "Lg"),
        ]
        .map(|(size, name)| {
            let height = theme.control_height(size);
            let sample = div()
                .h(height)
                .w(sizes.tile)
                .rounded(theme.radius(Radius::Md))
                .border_1()
                .border_color(colors.border_strong)
                .bg(colors.surface);
            measure(format!("ControlSize::{name}"), at(height), sample, cx)
        });
        let avatars = [
            (AvatarSize::Xs, "Xs"),
            (AvatarSize::Sm, "Sm"),
            (AvatarSize::Md, "Md"),
            (AvatarSize::Lg, "Lg"),
            (AvatarSize::Xl, "Xl"),
        ]
        .map(|(size, name)| {
            let side = theme.avatar_size(size);
            let sample = div()
                .size(side)
                .rounded_full()
                .border_1()
                .border_color(colors.border_strong)
                .bg(colors.sunken);
            measure(format!("AvatarSize::{name}"), at(side), sample, cx)
        });
        let containers = [
            (ContainerSize::Sm, "Sm"),
            (ContainerSize::Md, "Md"),
            (ContainerSize::Lg, "Lg"),
        ]
        .map(|(size, name)| {
            let width = theme.container_width(size);
            let widest = theme.container_width(ContainerSize::Lg);
            // Drawn at its share of the widest, so the bar stays inside the row.
            let sample = div().relative().flex_1().h_2().child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .h_full()
                    .w(relative(width.0 / widest.0))
                    .rounded(theme.radius(Radius::Sm))
                    .bg(colors.border_strong),
            );
            measure(format!("ContainerSize::{name}"), at(width), sample, cx)
        });
        let icons = [
            (IconSize::Xs, "Xs"),
            (IconSize::Sm, "Sm"),
            (IconSize::Md, "Md"),
            (IconSize::Lg, "Lg"),
            (IconSize::Xl, "Xl"),
            (IconSize::Xxl, "Xxl"),
        ]
        .map(|(size, name)| {
            let sample = Icon::new(IconName::Star).size(size).color(colors.fg);
            measure(
                format!("IconSize::{name}"),
                at(theme.icon_size(size)),
                sample,
                cx,
            )
        });
        let elevations = [
            (Elevation::Raised, "Raised"),
            (Elevation::Floating, "Floating"),
            (Elevation::Modal, "Modal"),
        ]
        .map(|(level, name)| {
            let card = div()
                .w(sizes.tile)
                .h(sizes.swatch)
                .rounded(theme.radius(Radius::Md))
                .bg(colors.surface)
                .shadow(theme.elevation(level));
            let shown = format!("token-Elevation::{name}");
            div()
                .debug_selector(move || shown)
                .flex()
                .flex_col()
                .gap_2()
                .child(card)
                .child(
                    literal(div())
                        .text_size(theme.text_size(TextSize::Xs))
                        .child(format!("Elevation::{name}")),
                )
        });
        let steps = STEPS.map(|step| {
            let width = rems(step / 4.0);
            let sample = div()
                .h_2()
                .w(width)
                .rounded(theme.radius(Radius::Sm))
                .bg(colors.accent);
            measure(format!("gap_{step}"), at(width), sample, cx)
        });
        div()
            .flex()
            .flex_col()
            .gap_8()
            .child(tiles("Colors", palette.collect()))
            .child(tiles("Syntax", syntax.collect()))
            .child(tiles("Chart", chart.collect()))
            .child(tiles("Terminal", ansi.collect()))
            .child(group("Type", cx).children(text))
            .child(group("Radii", cx).children(radii))
            .child(group("Controls", cx).children(controls))
            .child(group("Icons", cx).children(icons))
            .child(group("Avatars", cx).children(avatars))
            .child(group("Containers", cx).children(containers))
            .child(
                group("Elevation", cx)
                    .child(div().flex().flex_wrap().gap_6().p_2().children(elevations)),
            )
            .child(group("Spacing", cx).children(steps))
    }
}
