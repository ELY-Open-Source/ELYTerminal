use std::ops::Index;

use gpui::{Hsla, Rgba};
use vte::ansi::{Color, NamedColor, Rgb};

use crate::theme::Palette;

/// What a terminal paints with: the theme's sixteen colors, its text and its ground.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Ink {
    pub ansi: [Hsla; 16],
    pub fg: Hsla,
    pub bg: Hsla,
}

impl Ink {
    pub(crate) fn new(colors: &Palette) -> Self {
        Self {
            ansi: colors.ansi,
            fg: colors.fg,
            bg: colors.surface,
        }
    }

    /// A color as painted, after any the program set with OSC 4, 10 or 11.
    pub(crate) fn resolve(&self, color: Color, set: &impl Set) -> Hsla {
        match color {
            Color::Spec(spec) => hsla(spec),
            Color::Indexed(index) => self.indexed(index as usize, set),
            Color::Named(name) => self.named(name, set),
        }
    }

    fn named(&self, name: NamedColor, set: &impl Set) -> Hsla {
        if let Some(spec) = set[name as usize] {
            return hsla(spec);
        }
        match name {
            NamedColor::Foreground | NamedColor::BrightForeground | NamedColor::Cursor => self.fg,
            NamedColor::Background => self.bg,
            NamedColor::DimForeground => self.fg.opacity(0.7),
            dim if (NamedColor::DimBlack as usize..=NamedColor::DimWhite as usize)
                .contains(&(dim as usize)) =>
            {
                self.named(dim.to_bright(), set).opacity(0.7)
            }
            ansi => self.ansi[ansi as usize],
        }
    }

    fn indexed(&self, index: usize, set: &impl Set) -> Hsla {
        if let Some(spec) = set[index] {
            return hsla(spec);
        }
        match index {
            0..16 => self.ansi[index],
            16..232 => {
                let step = |level: usize| if level == 0 { 0 } else { 55 + 40 * level as u8 };
                let cube = index - 16;
                hsla(Rgb {
                    r: step(cube / 36),
                    g: step(cube / 6 % 6),
                    b: step(cube % 6),
                })
            }
            _ => {
                let gray = 8 + 10 * (index - 232) as u8;
                hsla(Rgb {
                    r: gray,
                    g: gray,
                    b: gray,
                })
            }
        }
    }

    /// Color `index` as an answer to a program's query.
    #[cfg(not(target_family = "wasm"))]
    pub(crate) fn query(&self, index: usize, set: &impl Set) -> Rgb {
        let color = match index {
            256 => self.named(NamedColor::Foreground, set),
            257 => self.named(NamedColor::Background, set),
            258 => self.named(NamedColor::Cursor, set),
            index => self.indexed(index.min(255), set),
        };
        let rgba = color.to_rgb();
        let byte = |channel: f32| (channel * 255.0).round() as u8;
        Rgb {
            r: byte(rgba.r),
            g: byte(rgba.g),
            b: byte(rgba.b),
        }
    }
}

/// Colors a program set through OSC 4, 10, 11.
pub(crate) trait Set: Index<usize, Output = Option<Rgb>> {}

impl<T: Index<usize, Output = Option<Rgb>>> Set for T {}

fn hsla(spec: Rgb) -> Hsla {
    let channel = |value: u8| f32::from(value) / 255.0;
    Rgba {
        r: channel(spec.r),
        g: channel(spec.g),
        b: channel(spec.b),
        a: 1.0,
    }
    .into()
}

#[cfg(test)]
mod tests {
    use alacritty_terminal::term::color::Colors;
    use gpui::{black, white};

    use super::*;

    fn ink() -> Ink {
        let mut ansi = [black(); 16];
        ansi[1] = hsla(Rgb { r: 200, g: 0, b: 0 });
        Ink {
            ansi,
            fg: black(),
            bg: white(),
        }
    }

    #[test]
    fn indexed_colors_follow_the_xterm_cube_and_ramp() {
        let (ink, set) = (ink(), Colors::default());
        assert_eq!(ink.query(196, &set), Rgb { r: 255, g: 0, b: 0 });
        assert_eq!(
            ink.query(244, &set),
            Rgb {
                r: 128,
                g: 128,
                b: 128
            }
        );
        assert_eq!(
            ink.query(1, &set),
            Rgb { r: 200, g: 0, b: 0 },
            "the theme's red"
        );
    }

    #[test]
    fn a_color_the_program_set_wins() {
        let (ink, mut set) = (ink(), Colors::default());
        set[1] = Some(Rgb { r: 1, g: 2, b: 3 });
        assert_eq!(
            ink.resolve(Color::Named(NamedColor::Red), &set),
            hsla(Rgb { r: 1, g: 2, b: 3 })
        );
        assert_eq!(
            ink.resolve(Color::Named(NamedColor::Background), &set),
            white()
        );
    }
}
