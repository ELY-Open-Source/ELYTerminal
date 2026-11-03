use std::sync::atomic::{AtomicBool, Ordering};

use barcoders::sym::{code128::Code128, ean13::EAN13};
use gpui::{
    App, Bounds, Div, IntoElement, ParentElement, Pixels, Refineable, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, canvas, div, fill, point, size,
};

pub use barcoders::error::Error as BarcodeError;
pub use qrcode::types::QrError;

use crate::theme::{ActiveTheme, Radius, TextSize};

/// Whether a QR code has already said its tile is too small to draw it sharply.
static SMALL: AtomicBool = AtomicBool::new(false);

/// The quiet margin around a QR code, in modules, as the standard asks.
const QUIET: usize = 4;

/// A module's side: whole pixels for crisp edges, or the exact share when whole pixels would floor to nothing.
fn module(side: Pixels, span: f32) -> Pixels {
    let fit = side / span;
    if f32::from(fit) >= 1.0 {
        return fit.floor();
    }
    if !SMALL.swap(true, Ordering::Relaxed) {
        log::warn!("qr code: {span} modules share {fit:?} each; give the tile more room");
    }
    fit
}

/// The dark runs in a row of modules, as start and length.
fn runs(row: &[bool]) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut start = None;
    for (ix, dark) in row.iter().chain([&false]).enumerate() {
        match (start, *dark) {
            (None, true) => start = Some(ix),
            (Some(from), false) => {
                runs.push((from, ix - from));
                start = None;
            }
            _ => {}
        }
    }
    runs
}

/// Paints each row's dark runs as bars `module` wide, from `origin`.
fn paint_runs(
    rows: &[Vec<(usize, usize)>],
    origin: gpui::Point<Pixels>,
    module: Pixels,
    height: Pixels,
    color: gpui::Hsla,
    window: &mut Window,
) {
    for (y, row) in rows.iter().enumerate() {
        for (x, length) in row {
            let corner = point(origin.x + module * *x as f32, origin.y + height * y as f32);
            window.paint_quad(fill(
                Bounds::new(corner, size(module * *length as f32, height)),
                color,
            ));
        }
    }
}

/// Text as a QR code: dark modules on a light tile in both themes, so any camera reads it, inside the standard quiet margin.
#[derive(IntoElement)]
pub struct QrCode {
    base: Div,
    rows: Vec<Vec<(usize, usize)>>,
}

impl QrCode {
    pub fn new(data: impl AsRef<[u8]>) -> Result<Self, QrError> {
        let code = qrcode::QrCode::new(data)?;
        let dark: Vec<bool> = code
            .to_colors()
            .into_iter()
            .map(|color| color == qrcode::Color::Dark)
            .collect();
        Ok(Self {
            base: div(),
            rows: dark.chunks(code.width()).map(runs).collect(),
        })
    }
}

impl Styled for QrCode {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for QrCode {
    fn render(mut self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (paper, ink) = (theme.colors.paper, theme.colors.ink);
        let rows = self.rows;
        let span = (rows.len() + QUIET * 2) as f32;
        let need = theme.barcode_module() * span;
        let side = if need.0 > theme.qr_code().0 {
            need
        } else {
            theme.qr_code()
        };
        let mut tile = div()
            .flex_none()
            .size(side)
            .rounded(theme.radius(Radius::Sm))
            .bg(paper);
        tile.style().refine(self.base.style());
        tile.child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    let module = module(bounds.size.width.min(bounds.size.height), span);
                    let inset = (rows.len() as f32 * module) / 2.0;
                    let origin = bounds.center() - point(inset, inset);
                    paint_runs(&rows, origin, module, module, ink, window);
                },
            )
            .size_full(),
        )
    }
}

/// EAN's check digit: weights of one and three from the left, then what lifts the sum to a multiple of ten.
fn check_digit(digits: &str) -> u32 {
    let sum: u32 = digits
        .chars()
        .enumerate()
        .map(|(ix, digit)| {
            digit.to_digit(10).expect("barcoders checked the digits")
                * if ix % 2 == 0 { 1 } else { 3 }
        })
        .sum();
    (10 - sum % 10) % 10
}

/// Text as a barcode, bars on a light tile in both themes with the text under them: Code 128 for printable ASCII, EAN-13 for retail numbers.
#[derive(IntoElement)]
pub struct Barcode {
    base: Div,
    bars: Vec<(usize, usize)>,
    modules: usize,
    text: SharedString,
}

impl Barcode {
    fn new(modules: Vec<u8>, text: String) -> Self {
        let dark: Vec<bool> = modules.iter().map(|module| *module == 1).collect();
        Self {
            base: div(),
            bars: runs(&dark),
            modules: dark.len(),
            text: text.into(),
        }
    }

    /// Code 128 in its set B: letters, digits and printable ASCII marks.
    pub fn code128(text: &str) -> Result<Self, BarcodeError> {
        if !text.chars().all(|letter| (' '..='~').contains(&letter)) {
            return Err(BarcodeError::Character);
        }
        let modules = Code128::new(format!("\u{0181}{text}"))?.encode();
        Ok(Self::new(modules, text.to_string()))
    }

    /// EAN-13 from twelve digits, or thirteen whose check digit must match.
    pub fn ean13(digits: &str) -> Result<Self, BarcodeError> {
        let modules = EAN13::new(digits)?.encode();
        let text = match digits.len() {
            12 => format!("{digits}{}", check_digit(digits)),
            _ => digits.to_string(),
        };
        Ok(Self::new(modules, text))
    }
}

impl Styled for Barcode {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Barcode {
    fn render(mut self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (paper, ink) = (theme.colors.paper, theme.colors.ink);
        let bars = vec![self.bars];
        let mut tile = div()
            .flex()
            .flex_none()
            .flex_col()
            .items_center()
            .gap_1()
            .px_4()
            .pt_3()
            .pb_2()
            .rounded(theme.radius(Radius::Sm))
            .bg(paper)
            .text_color(ink);
        tile.style().refine(self.base.style());
        tile.child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    let module = bounds.size.width / self.modules as f32;
                    paint_runs(
                        &bars,
                        bounds.origin,
                        module,
                        bounds.size.height,
                        ink,
                        window,
                    );
                },
            )
            .w(theme.barcode_module() * self.modules as f32)
            .h(theme.barcode_height()),
        )
        .child(
            div()
                .font_family(theme.mono_family.clone())
                .text_size(theme.text_size(TextSize::Xs))
                .child(self.text),
        )
    }
}

#[cfg(test)]
mod tests {
    use gpui::px;

    use super::{Barcode, BarcodeError, check_digit, module, runs};

    #[test]
    fn dark_modules_merge_into_runs() {
        assert_eq!(runs(&[true, true, false, true]), [(0, 2), (3, 1)]);
        assert_eq!(runs(&[false, false]), []);
        assert_eq!(runs(&[true]), [(0, 1)]);
    }

    #[test]
    fn an_ean_gains_its_check_digit_and_refuses_a_wrong_one() {
        assert_eq!(check_digit("400638133393"), 1);
        assert_eq!(check_digit("590123412345"), 7);
        assert_eq!(
            Barcode::ean13("400638133393").map(|code| code.text).ok(),
            Some("4006381333931".into())
        );
        assert!(matches!(
            Barcode::ean13("4006381333932"),
            Err(BarcodeError::Checksum)
        ));
        assert!(matches!(
            Barcode::code128("caf\u{e9}"),
            Err(BarcodeError::Character)
        ));
        assert!(
            matches!(Barcode::code128("\u{c0}123"), Err(BarcodeError::Character)),
            "barcoders reads \u{c0} as a switch to set A"
        );
    }

    #[test]
    fn a_dense_qr_keeps_modules_it_can_draw() {
        assert_eq!(
            module(px(254.0), 25.0),
            px(10.0),
            "whole pixels when they fit"
        );
        assert!(
            module(px(100.0), 185.0) > px(0.0),
            "a share under a pixel stays drawn"
        );
    }

    #[test]
    fn code_128_spends_eleven_modules_a_character() {
        let code = Barcode::code128("ELY-2419").expect("printable ASCII");
        assert_eq!(
            code.modules,
            11 + 8 * 11 + 11 + 13,
            "start, eight characters, check, stop"
        );
        assert_eq!(
            code.bars.first(),
            Some(&(0, 2)),
            "set B's start opens with two dark modules"
        );
    }
}
