use gpui::{
    App, AppContext as _, ElementId, Entity, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled, Window, div,
};

use crate::{
    buttons::IconButton,
    forms::{Choice, Input, Select, TextInput, parse_number},
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, Radius},
    typography::{
        format::{MINUS, Separators, significant},
        tabular,
    },
};

/// A unit as its kind's base times `scale`, plus `offset`: kelvin from celsius is 1 and 273.15.
pub(crate) struct Unit {
    name: &'static str,
    symbol: &'static str,
    scale: f64,
    offset: f64,
}

const fn unit(name: &'static str, symbol: &'static str, scale: f64) -> Unit {
    Unit {
        name,
        symbol,
        scale,
        offset: 0.0,
    }
}

/// A kind of measure: its name, its units, and the two it starts on.
pub(crate) struct Kind {
    name: &'static str,
    units: &'static [Unit],
    pair: (usize, usize),
}

/// Lengths in meters, areas in square meters, volumes in liters, masses in kilograms, temperatures in kelvin, speeds in meters a second, data in bytes.
pub(crate) static KINDS: [Kind; 7] = [
    Kind {
        name: "Length",
        units: &[
            unit("Millimeter", "mm", 0.001),
            unit("Centimeter", "cm", 0.01),
            unit("Meter", "m", 1.0),
            unit("Kilometer", "km", 1000.0),
            unit("Inch", "in", 0.0254),
            unit("Foot", "ft", 0.3048),
            unit("Yard", "yd", 0.9144),
            unit("Mile", "mi", 1609.344),
        ],
        pair: (2, 5),
    },
    Kind {
        name: "Area",
        units: &[
            unit("Square meter", "m²", 1.0),
            unit("Hectare", "ha", 1e4),
            unit("Square kilometer", "km²", 1e6),
            unit("Square foot", "ft²", 0.092_903_04),
            unit("Acre", "ac", 4_046.856_422_4),
            unit("Square mile", "mi²", 2_589_988.110_336),
        ],
        pair: (0, 3),
    },
    Kind {
        name: "Volume",
        units: &[
            unit("Milliliter", "mL", 0.001),
            unit("Liter", "L", 1.0),
            unit("Cubic meter", "m³", 1000.0),
            unit("US fluid ounce", "fl oz", 0.029_573_529_562_5),
            unit("US cup", "cup", 0.236_588_236_5),
            unit("US pint", "pt", 0.473_176_473),
            unit("US gallon", "gal", 3.785_411_784),
        ],
        pair: (1, 6),
    },
    Kind {
        name: "Mass",
        units: &[
            unit("Milligram", "mg", 1e-6),
            unit("Gram", "g", 0.001),
            unit("Kilogram", "kg", 1.0),
            unit("Tonne", "t", 1000.0),
            unit("Ounce", "oz", 0.028_349_523_125),
            unit("Pound", "lb", 0.453_592_37),
            unit("Stone", "st", 6.350_293_18),
        ],
        pair: (2, 5),
    },
    Kind {
        name: "Temperature",
        units: &[
            Unit {
                name: "Celsius",
                symbol: "°C",
                scale: 1.0,
                offset: 273.15,
            },
            Unit {
                name: "Fahrenheit",
                symbol: "°F",
                scale: 5.0 / 9.0,
                offset: 273.15 - 32.0 * 5.0 / 9.0,
            },
            unit("Kelvin", "K", 1.0),
        ],
        pair: (0, 1),
    },
    Kind {
        name: "Speed",
        units: &[
            unit("Meter per second", "m/s", 1.0),
            unit("Kilometer per hour", "km/h", 1.0 / 3.6),
            unit("Mile per hour", "mph", 0.447_04),
            unit("Knot", "kn", 1852.0 / 3600.0),
        ],
        pair: (1, 2),
    },
    Kind {
        name: "Data",
        units: &[
            unit("Bit", "b", 0.125),
            unit("Byte", "B", 1.0),
            unit("Kilobyte", "kB", 1e3),
            unit("Megabyte", "MB", 1e6),
            unit("Gigabyte", "GB", 1e9),
            unit("Terabyte", "TB", 1e12),
            unit("Kibibyte", "KiB", 1024.0),
            unit("Mebibyte", "MiB", 1_048_576.0),
            unit("Gibibyte", "GiB", 1_073_741_824.0),
        ],
        pair: (4, 8),
    },
];

/// `amount` in `from` as `to`, through their kind's base.
pub(crate) fn convert(amount: f64, from: &Unit, to: &Unit) -> f64 {
    (amount * from.scale + from.offset - to.offset) / to.scale
}

/// `amount` in `from` as it reads in `to`: six significant digits and the symbol.
fn read(amount: f64, from: &Unit, to: &Unit) -> String {
    let value = significant(convert(amount, from, to), 6, Separators::EN);
    format!("{value} {}", to.symbol)
}

/// The converter's own field, kind and units.
struct Desk {
    input: Entity<TextInput>,
    kind: usize,
    from: usize,
    to: usize,
}

/// Where a unit named `name` stands among its kind's.
fn place(units: &[Unit], name: &str) -> usize {
    units
        .iter()
        .position(|unit| unit.name == name)
        .unwrap_or_else(|| panic!("unit converter: no unit named {name}"))
}

/// An amount in one unit read in another: length, area, volume, mass, temperature, speed or data, with a swap of the two units. It shows six significant digits, wrapped in its box when long.
#[derive(IntoElement)]
pub struct UnitConverter {
    id: ElementId,
}

impl UnitConverter {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self { id: id.into() }
    }
}

impl RenderOnce for UnitConverter {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let desk = window.use_keyed_state((id.clone(), "desk"), cx, |window, cx| Desk {
            input: cx.new(|cx| {
                let mut input = TextInput::new(window, cx)
                    .filter(|ch| ch.is_ascii_digit() || matches!(ch, '.' | ',' | '-' | MINUS));
                input.set_text("1", cx);
                input
            }),
            kind: 0,
            from: KINDS[0].pair.0,
            to: KINDS[0].pair.1,
        });
        let (input, kind, from, to) = {
            let desk = desk.read(cx);
            (desk.input.clone(), &KINDS[desk.kind], desk.from, desk.to)
        };
        let text = input.read(cx).text().to_string();
        let amount = parse_number(&text);
        let result = match amount {
            Some(amount) => read(amount, &kind.units[from], &kind.units[to]),
            None => "—".to_string(),
        };
        let theme = cx.theme();
        let choices = || {
            kind.units
                .iter()
                .map(|unit| Choice::new(unit.name, unit.name))
                .collect::<Vec<_>>()
        };
        let pick = |edit: fn(&mut Desk, usize)| {
            let desk = desk.clone();
            move |name: &SharedString, _: &mut Window, cx: &mut App| {
                desk.update(cx, |desk, cx| {
                    edit(desk, place(KINDS[desk.kind].units, name));
                    log::info!("unit converter: {name}");
                    cx.notify();
                })
            }
        };
        let (kinds, swap) = (desk.clone(), desk.clone());
        let half = || div().flex_1().min_w_0();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        half().child(
                            Select::new(
                                (id.clone(), "kind"),
                                KINDS.iter().map(|kind| Choice::new(kind.name, kind.name)),
                            )
                            .selected(kind.name)
                            .on_change(move |name, _, cx| {
                                kinds.update(cx, |desk, cx| {
                                    desk.kind = KINDS
                                        .iter()
                                        .position(|kind| kind.name == name.as_ref())
                                        .unwrap_or_else(|| {
                                            panic!("unit converter: no kind {name}")
                                        });
                                    (desk.from, desk.to) = KINDS[desk.kind].pair;
                                    log::info!("unit converter: {name}");
                                    cx.notify();
                                })
                            }),
                        ),
                    )
                    .child(
                        IconButton::new((id.clone(), "swap"), IconName::ArrowDownUp)
                            .tooltip("Swap units")
                            .on_click(move |_, _, cx| {
                                swap.update(cx, |desk, cx| {
                                    (desk.from, desk.to) = (desk.to, desk.from);
                                    log::info!("unit converter: swapped");
                                    cx.notify();
                                })
                            }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(half().child(Input::new(&input).invalid(amount.is_none())))
                    .child(
                        half().child(
                            Select::new((id.clone(), "from"), choices())
                                .selected(kind.units[from].name)
                                .on_change(pick(|desk, at| desk.from = at)),
                        ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        half().child(
                            div()
                                .flex()
                                .items_center()
                                .min_h(theme.control_height(ControlSize::Md))
                                .px(theme.control_padding(ControlSize::Md))
                                .py_1()
                                .rounded(theme.radius(Radius::Md))
                                .bg(theme.colors.sunken)
                                .text_color(theme.colors.fg)
                                .child(
                                    tabular(div())
                                        .flex_1()
                                        .min_w_0()
                                        .debug_selector(|| format!("converted-{result}"))
                                        .child(result.clone()),
                                ),
                        ),
                    )
                    .child(
                        half().child(
                            Select::new((id, "to"), choices())
                                .selected(kind.units[to].name)
                                .on_change(pick(|desk, at| desk.to = at)),
                        ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{KINDS, Kind, convert, place, read};

    fn between(kind: &Kind, amount: f64, from: &str, to: &str) -> f64 {
        convert(
            amount,
            &kind.units[place(kind.units, from)],
            &kind.units[place(kind.units, to)],
        )
    }

    fn near(value: f64, expected: f64) -> bool {
        (value - expected).abs() <= 1e-9 * expected.abs().max(1.0)
    }

    #[test]
    fn units_convert_through_their_base() {
        let [length, area, volume, mass, temperature, speed, data] = &KINDS;
        assert!(near(between(length, 1.0, "Mile", "Kilometer"), 1.609_344));
        assert!(near(between(length, 1.0, "Foot", "Inch"), 12.0));
        assert!(near(between(area, 1.0, "Square mile", "Acre"), 640.0));
        assert!(near(between(volume, 1.0, "US gallon", "US cup"), 16.0));
        assert!(near(between(mass, 1.0, "Stone", "Pound"), 14.0));
        assert!(near(
            between(speed, 1.0, "Knot", "Kilometer per hour"),
            1.852
        ));
        assert!(near(
            between(data, 1.0, "Gibibyte", "Megabyte"),
            1_073.741_824
        ));
        assert!(near(
            between(temperature, 100.0, "Celsius", "Fahrenheit"),
            212.0
        ));
        assert!(near(
            between(temperature, -40.0, "Fahrenheit", "Celsius"),
            -40.0
        ));
        assert!(near(
            between(temperature, 0.0, "Kelvin", "Celsius"),
            -273.15
        ));
    }

    #[test]
    fn a_reading_keeps_six_significant_digits_and_the_symbol() {
        let length = &KINDS[0];
        let unit = |name: &str| &length.units[place(length.units, name)];
        assert_eq!(read(1.0, unit("Mile"), unit("Kilometer")), "1.60934 km");
        assert_eq!(
            read(1e9, unit("Meter"), unit("Millimeter")),
            "1,000,000,000,000 mm"
        );
    }

    #[test]
    fn each_kind_starts_on_two_units_of_its_own() {
        for kind in &KINDS {
            let (from, to) = kind.pair;
            assert!(from != to && to < kind.units.len(), "{}", kind.name);
        }
    }
}
