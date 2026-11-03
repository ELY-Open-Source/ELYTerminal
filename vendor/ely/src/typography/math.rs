use std::{
    fmt,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Arc,
};

use gpui::{
    AnyElement, App, Asset, ElementId, InteractiveElement, IntoElement, ParentElement, Pixels,
    RenderImage, RenderOnce, SharedString, Size, Styled, Window, div, img, px, size,
};
use ratex_layout::{LayoutOptions, layout, to_display_list};
use ratex_svg::{SvgOptions, render_to_svg};
use ratex_types::{color::Color, math_style::MathStyle};

use crate::{
    primitives::hold,
    theme::{ActiveTheme, TextSize},
};

/// Why a formula did not typeset, and where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MathError {
    message: SharedString,
    /// The offset RaTeX names, when it names one.
    at: Option<usize>,
}

impl fmt::Display for MathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.at {
            Some(at) => write!(f, "{} at {at}", self.message),
            None => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for MathError {}

impl MathError {
    fn said(message: impl Into<SharedString>) -> Self {
        Self {
            message: message.into(),
            at: None,
        }
    }
}

fn parsed(source: &str) -> Result<Vec<ratex_parser::parse_node::ParseNode>, MathError> {
    ratex_parser::parse(source).map_err(|error| MathError {
        message: error.message.into(),
        at: error.loc.map(|loc| loc.start),
    })
}

/// TeX typeset by RaTeX; a failure shows why and source.
#[derive(IntoElement)]
pub struct Latex {
    id: ElementId,
    source: SharedString,
    size: TextSize,
    display: bool,
}

impl Latex {
    pub fn parse(id: impl Into<ElementId>, source: &str) -> Result<Self, MathError> {
        parsed(source)?;
        Ok(Self {
            id: id.into(),
            source: source.to_string().into(),
            size: TextSize::Lg,
            display: false,
        })
    }

    pub fn size(mut self, size: TextSize) -> Self {
        self.size = size;
        self
    }

    /// Display style: limits above and below, larger operators.
    pub fn display(mut self) -> Self {
        self.display = true;
        self
    }
}

/// What a typeset picture depends on.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Formula {
    source: SharedString,
    display: bool,
    rgba: u32,
    /// Font size and window scale, in hundredths.
    font: u32,
    scale: u32,
}

/// A typeset formula: its picture and logical size.
pub(crate) type Typeset = Result<(Arc<RenderImage>, Size<Pixels>), MathError>;

enum Typesetter {}

impl Asset for Typesetter {
    type Source = Formula;
    type Output = Typeset;

    fn load(formula: Formula, cx: &mut App) -> impl Future<Output = Typeset> + Send + 'static {
        let renderer = cx.svg_renderer();
        async move {
            let (svg, shape) = svg(&formula)?;
            let picture = renderer
                .render_single_frame(svg.as_bytes(), formula.scale as f32 / 100.0)
                .map_err(|error| MathError::said(format!("{error}")))?;
            Ok((picture, shape))
        }
    }
}

/// `source` as black outlined SVG, for export.
pub fn formula_svg(source: &str, display: bool) -> Result<String, MathError> {
    let formula = Formula {
        source: source.to_owned().into(),
        display,
        rgba: 0x0000_00ff,
        font: 1600,
        scale: 100,
    };
    let (svg, width, height) = laid(&formula)?;
    // RaTeX writes text where it has no outline.
    let outlined = crate::documents::flatten(&svg).map_err(MathError::said)?;
    framed(&outlined, width, height).map(|(svg, _)| svg)
}

/// The formula as outlined SVG holding all ink; its size.
fn svg(formula: &Formula) -> Result<(String, Size<Pixels>), MathError> {
    let (svg, width, height) = laid(formula)?;
    framed(&svg, width, height)
}

/// RaTeX's SVG and the layout box's width and height.
fn laid(formula: &Formula) -> Result<(String, f32, f32), MathError> {
    let ast = parsed(&formula.source)?;
    let [r, g, b, a] = formula
        .rgba
        .to_be_bytes()
        .map(|channel| f32::from(channel) / 255.0);
    let options = LayoutOptions::default()
        .with_style(match formula.display {
            true => MathStyle::Display,
            false => MathStyle::Text,
        })
        .with_color(Color::new(r, g, b, a));
    let font = f64::from(formula.font) / 100.0;
    let typeset = guarded(|| {
        let list = to_display_list(&layout(&ast, &options));
        let svg = render_to_svg(
            &list,
            &SvgOptions {
                font_size: font,
                padding: 0.0,
                stroke_width: (font / 26.0).max(0.5),
                embed_glyphs: true,
                font_dir: String::new(),
            },
        );
        (list.width * font, (list.height + list.depth) * font, svg)
    })?;
    let (width, height, svg) = typeset;
    let (width, height) = (width as f32, height as f32);
    // RaTeX sizes its svg in pt; px keeps one unit.
    Ok((wrapped(&svg, [0.0, 0.0, width, height])?, width, height))
}

/// Runs RaTeX's layout; a panic becomes an error.
fn guarded<T>(lay: impl FnOnce() -> T) -> Result<T, MathError> {
    catch_unwind(AssertUnwindSafe(lay)).map_err(|_| {
        // The formula is the user's text: not logged.
        log::error!("latex: RaTeX failed to lay out a formula");
        MathError::said("RaTeX could not lay out this formula")
    })
}

/// `svg` reframed around its ink and layout box.
fn framed(svg: &str, width: f32, height: f32) -> Result<(String, Size<Pixels>), MathError> {
    let tree = usvg::Tree::from_data(svg.as_bytes(), &crate::documents::svg_options())
        .map_err(|error| MathError::said(format!("{error}")))?;
    let ink = tree.root().abs_stroke_bounding_box();
    let left = ink.left().min(0.0);
    let top = ink.top().min(0.0);
    let right = ink.right().max(width);
    let bottom = ink.bottom().max(height);
    let (w, h) = (right - left, bottom - top);
    Ok((wrapped(svg, [left, top, w, h])?, size(px(w), px(h))))
}

/// `svg` in a px root viewing `[left, top, w, h]`.
fn wrapped(svg: &str, [left, top, w, h]: [f32; 4]) -> Result<String, MathError> {
    let open = svg
        .find('>')
        .ok_or_else(|| MathError::said("RaTeX wrote no svg tag"))?;
    let close = svg
        .rfind("</svg>")
        .ok_or_else(|| MathError::said("RaTeX left the svg open"))?;
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{left} {top} {w} {h}\" width=\"{w}\" height=\"{h}\">{}</svg>",
        &svg[open + 1..close]
    ))
}

/// The asset key for `source` at `font` in `window`.
fn formula(
    (source, display): (&SharedString, bool),
    font: Pixels,
    window: &Window,
    cx: &App,
) -> Formula {
    Formula {
        source: source.clone(),
        display,
        rgba: u32::from(cx.theme().colors.fg.to_rgb()),
        font: (font.as_f32() * 100.0).round() as u32,
        scale: (window.scale_factor() * 100.0).round() as u32,
    }
}

/// `source` typeset at `font` in `window`; None while it loads.
pub(crate) fn typeset(
    id: impl Into<ElementId>,
    (source, display): (&SharedString, bool),
    font: Pixels,
    window: &mut Window,
    cx: &mut App,
) -> Option<Typeset> {
    let formula = formula((source, display), font, window, cx);
    let picture = |typeset: &Typeset| typeset.as_ref().ok().map(|(image, _)| image.clone());
    hold::<Typesetter>(id, &formula, picture, window, cx)
}

impl RenderOnce for Latex {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let font = cx.theme().text_size(self.size).to_pixels(window.rem_size());
        match typeset(self.id, (&self.source, self.display), font, window, cx) {
            Some(Ok((picture, shape))) => img(picture)
                .w(shape.width)
                .h(shape.height)
                .into_any_element(),
            Some(Err(error)) => failed(&error, &self.source, cx),
            None => div().h(font).into_any_element(),
        }
    }
}

/// A formula that did not typeset: why, then its source.
pub(crate) fn failed(error: &MathError, source: &str, cx: &App) -> AnyElement {
    let theme = cx.theme();
    div()
        .debug_selector(|| "latex-error".into())
        .flex()
        .flex_col()
        .child(
            div()
                .text_color(theme.colors.danger)
                .child(error.to_string()),
        )
        .child(
            div()
                .font_family(theme.mono_family.clone())
                .text_color(theme.colors.fg_muted)
                .child(source.to_string()),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests;
