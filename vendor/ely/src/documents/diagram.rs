use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, OnceLock},
};

use gpui::{
    App, Asset, ElementId, Hsla, IntoElement, ParentElement, Pixels, RenderImage, RenderOnce,
    SharedString, Size, Styled, Window, div, img, px, size,
};
use mermaid_rs_renderer::{LayoutConfig, RenderOptions, Theme, render_strict};

use crate::{
    primitives::hold,
    theme::{ActiveTheme, Radius, TextSize},
};

/// A Mermaid diagram; a failure shows why and the source.
#[derive(IntoElement)]
pub struct Diagram {
    id: ElementId,
    source: SharedString,
}

impl Diagram {
    pub fn new(id: impl Into<ElementId>, source: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
        }
    }
}

/// What a drawn diagram depends on.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Chart {
    source: SharedString,
    dark: bool,
    /// Background and text, as `#rrggbb`.
    ground: SharedString,
    ink: SharedString,
    /// Window scale, in hundredths.
    scale: u32,
}

type Drawn = Result<(Arc<RenderImage>, Size<Pixels>), SharedString>;

enum Drawer {}

impl Asset for Drawer {
    type Source = Chart;
    type Output = Drawn;

    fn load(chart: Chart, cx: &mut App) -> impl Future<Output = Drawn> + Send + 'static {
        let renderer = cx.svg_renderer();
        async move {
            let svg = svg(&chart)?;
            let tree = usvg::Tree::from_data(svg.as_bytes(), &options())
                .map_err(|error| SharedString::from(format!("{error}")))?;
            let shape = size(px(tree.size().width()), px(tree.size().height()));
            let picture = renderer
                .render_single_frame(svg.as_bytes(), chart.scale as f32 / 100.0)
                .map_err(|error| SharedString::from(format!("{error}")))?;
            Ok((picture, shape))
        }
    }
}

/// The diagram as SVG in Mermaid's theme for the mode.
fn svg(chart: &Chart) -> Result<String, SharedString> {
    let mut theme = match chart.dark {
        true => Theme::dark(),
        false => Theme::modern(),
    };
    let (ground, ink) = (chart.ground.to_string(), chart.ink.to_string());
    theme.background = ground.clone();
    theme.edge_label_background = ground;
    theme.primary_text_color = ink.clone();
    theme.text_color = ink.clone();
    theme.pie_title_text_color = ink.clone();
    theme.pie_section_text_color = ink.clone();
    theme.pie_legend_text_color = ink;
    let options = RenderOptions {
        theme,
        layout: LayoutConfig::default(),
    };
    // The renderer may panic on input it parsed.
    let drawn = catch_unwind(AssertUnwindSafe(|| render_strict(&chart.source, options)));
    match drawn {
        Ok(Ok(svg)) => Ok(svg),
        Ok(Err(error)) => Err(error.to_string().into()),
        Err(_) => {
            log::error!("diagram: the renderer failed on a parsed diagram");
            Err("the Mermaid renderer could not draw this diagram".into())
        }
    }
}

/// `source` as light SVG of plain paths, for export.
pub fn diagram_svg(source: &str) -> Result<String, SharedString> {
    let chart = Chart {
        source: source.to_owned().into(),
        dark: false,
        ground: "#ffffff".into(),
        ink: "#000000".into(),
        scale: 100,
    };
    flatten(&svg(&chart)?)
}

/// usvg reading with the system's fonts, chosen as gpui draws.
pub(crate) fn options() -> usvg::Options<'static> {
    let named = usvg::FontResolver::default_font_selector();
    let select_font = Box::new(
        move |font: &usvg::Font, db: &mut Arc<usvg::fontdb::Database>| {
            // A missing family falls to sans-serif, then any face.
            let sans = usvg::fontdb::Query {
                families: &[usvg::fontdb::Family::SansSerif],
                ..Default::default()
            };
            named(font, db)
                .or_else(|| db.query(&sans))
                .or_else(|| db.faces().next().map(|face| face.id))
        },
    );
    usvg::Options {
        fontdb: fonts(),
        font_resolver: usvg::FontResolver {
            select_font,
            select_fallback: usvg::FontResolver::default_fallback_selector(),
        },
        ..usvg::Options::default()
    }
}

/// `svg` with markers and text as paths any reader draws.
pub(crate) fn flatten(svg: &str) -> Result<String, SharedString> {
    let tree = usvg::Tree::from_data(svg.as_bytes(), &options())
        .map_err(|error| SharedString::from(format!("{error}")))?;
    // A letter without a font vanishes: say so.
    let (mut kept, mut lost) = (0, 0);
    texts(tree.root(), &mut kept, &mut lost);
    let written = labels(svg)?;
    if kept < written || lost > 0 {
        log::error!("svg: {kept} of {written} texts set, {lost} letters without a font");
        return Err("no installed font has every letter here".into());
    }
    Ok(tree.to_string(&usvg::WriteOptions::default()))
}

/// The `<text>` elements holding words.
fn labels(svg: &str) -> Result<usize, SharedString> {
    let doc = usvg::roxmltree::Document::parse(svg)
        .map_err(|error| SharedString::from(format!("{error}")))?;
    let worded = |node: &usvg::roxmltree::Node| {
        node.descendants()
            .any(|word| word.text().is_some_and(|text| !text.trim().is_empty()))
    };
    Ok(doc
        .descendants()
        .filter(|node| node.has_tag_name("text") && worded(node))
        .count())
}

/// Text nodes usvg set; letters it found no glyph for.
fn texts(group: &usvg::Group, kept: &mut usize, lost: &mut usize) {
    for node in group.children() {
        match node {
            usvg::Node::Group(inner) => texts(inner, kept, lost),
            usvg::Node::Text(text) => {
                *kept += 1;
                let glyphs = text
                    .layouted()
                    .iter()
                    .flat_map(|span| &span.positioned_glyphs);
                // Glyph zero is a font's blank for a missing letter.
                *lost += glyphs
                    .filter(|glyph| glyph.id.0 == 0 && !glyph.text.trim().is_empty())
                    .count();
            }
            _ => {}
        }
    }
}

/// The system's fonts and gpui's bundled ones, read once.
fn fonts() -> Arc<usvg::fontdb::Database> {
    static FONTS: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            use gpui::AssetSource as _;
            let mut fonts = usvg::fontdb::Database::new();
            fonts.load_system_fonts();
            // The faces gpui's SVG renderer adds, from the same assets.
            for path in [
                "fonts/ibm-plex-sans/IBMPlexSans-Regular.ttf",
                "fonts/lilex/Lilex-Regular.ttf",
            ] {
                match crate::Assets.load(path) {
                    Ok(Some(data)) => fonts.load_font_data(data.into_owned()),
                    Ok(None) => log::debug!("diagram: no bundled {path}"),
                    Err(error) => log::error!("diagram: bundled {path} did not load: {error}"),
                }
            }
            // As gpui: unanswered generic families take bundled faces.
            use usvg::fontdb::{Family, Query};
            for (family, bundled) in [
                (Family::SansSerif, "IBM Plex Sans"),
                (Family::Serif, "IBM Plex Sans"),
                (Family::Monospace, "Lilex"),
                (Family::Cursive, "IBM Plex Sans"),
                (Family::Fantasy, "IBM Plex Sans"),
            ] {
                let query = Query {
                    families: &[family],
                    ..Default::default()
                };
                if fonts.query(&query).is_some() {
                    continue;
                }
                match family {
                    Family::SansSerif => fonts.set_sans_serif_family(bundled),
                    Family::Serif => fonts.set_serif_family(bundled),
                    Family::Monospace => fonts.set_monospace_family(bundled),
                    Family::Cursive => fonts.set_cursive_family(bundled),
                    _ => fonts.set_fantasy_family(bundled),
                }
            }
            log::info!(
                "diagram: {} font faces for measuring and export",
                fonts.len()
            );
            Arc::new(fonts)
        })
        .clone()
}

/// `#rrggbb` of a theme color.
fn hex(color: Hsla) -> SharedString {
    let rgb = color.to_rgb();
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "#{:02x}{:02x}{:02x}",
        channel(rgb.r),
        channel(rgb.g),
        channel(rgb.b)
    )
    .into()
}

impl RenderOnce for Diagram {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let chart = Chart {
            source: self.source.clone(),
            dark: theme.is_dark(),
            ground: hex(colors.surface),
            ink: hex(colors.fg),
            scale: (window.scale_factor() * 100.0).round() as u32,
        };
        let (radius, small, mono) = (
            theme.radius(Radius::Md),
            theme.text_size(TextSize::Sm),
            theme.mono_family.clone(),
        );
        let picture = |drawn: &Drawn| drawn.as_ref().ok().map(|(image, _)| image.clone());
        match hold::<Drawer>(self.id, &chart, picture, window, cx) {
            Some(Ok((picture, shape))) => div()
                .flex()
                .justify_center()
                .child(img(picture).w(shape.width).h(shape.height).max_w_full())
                .into_any_element(),
            Some(Err(why)) => div()
                .flex()
                .flex_col()
                .gap_1()
                .p_3()
                .rounded(radius)
                .bg(colors.sunken)
                .text_size(small)
                .child(div().text_color(colors.danger).child(why))
                .child(
                    div()
                        .font_family(mono)
                        .text_color(colors.fg_muted)
                        .child(self.source),
                )
                .into_any_element(),
            None => div().h_12().into_any_element(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chart(source: &str, dark: bool) -> Chart {
        Chart {
            source: source.to_string().into(),
            dark,
            ground: "#2c2c2c".into(),
            ink: "#eeeeee".into(),
            scale: 100,
        }
    }

    #[test]
    fn a_flowchart_draws_with_the_ground_and_ink() {
        let svg = svg(&chart("flowchart LR\n  A[Start] --> B", true)).expect("draws");
        let svg = svg.to_lowercase();
        assert!(svg.contains("#2c2c2c"), "the ground: {svg}");
        let label = svg.find(">start<").expect("the label");
        let tag = &svg[svg[..label].rfind("<text").expect("its tag")..label];
        assert!(tag.contains("fill=\"#eeeeee\""), "the label's ink: {tag}");
    }

    #[test]
    fn a_broken_diagram_says_why() {
        let why = svg(&chart("flowchart LR\n  A -->", false)).expect_err("an error");
        assert!(!why.is_empty());
    }

    #[test]
    fn an_exported_diagram_is_plain_paths() {
        let svg = diagram_svg(
            "flowchart LR
  A[Start] --> B",
        )
        .expect("draws");
        assert!(svg.contains("<path"), "{svg}");
        assert!(!svg.contains("<marker") && !svg.contains("<text"), "{svg}");
    }

    #[test]
    fn a_letter_no_font_has_is_an_error() {
        let why = diagram_svg("flowchart LR\n  A[ABC \u{F0000}] --> B").expect_err("no glyph");
        assert!(why.contains("font"), "{why}");
    }

    #[test]
    fn colors_write_as_hex() {
        assert_eq!(hex(gpui::rgb(0x2c2c2c).into()), "#2c2c2c");
    }
}
