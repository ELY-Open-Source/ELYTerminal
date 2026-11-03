use std::{cmp::Reverse, rc::Rc};

use gpui::{
    App, ClipboardItem, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div,
};
use jiff::{Timestamp, tz::TimeZone};

use super::quotes::price;
use crate::{
    buttons::{ButtonVariant, IconButton},
    data_display::{Badge, QrCode, Tone},
    forms::{Combobox, NumberInput, TextInput},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::{
        Ellipsis,
        format::{self, system_zone},
        tabular,
    },
};

type OnConvert = Rc<dyn Fn(f64, SharedString, SharedString, &mut Window, &mut App)>;

/// An amount in one currency as another: the amount and both currencies to choose, a swap between them, the result, and the rate the owner gives.
#[derive(IntoElement)]
pub struct CurrencyConverter {
    id: ElementId,
    amount: f64,
    from: SharedString,
    to: SharedString,
    rate: f64,
    on_change: Option<OnConvert>,
}

impl CurrencyConverter {
    /// `rate` is how much of `to` one `from` buys; codes are ISO 4217.
    pub fn new(id: impl Into<ElementId>, amount: f64, (from, to): (&str, &str), rate: f64) -> Self {
        assert!(rate.is_finite() && rate > 0.0, "a rate is positive");
        Self {
            id: id.into(),
            amount,
            from: from.to_string().into(),
            to: to.to_string().into(),
            rate,
            on_change: None,
        }
    }

    /// Gets the amount and both codes after any of them changes.
    pub fn on_change(
        mut self,
        handler: impl Fn(f64, SharedString, SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for CurrencyConverter {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let from_field =
            window.use_keyed_state((self.id.clone(), "from-field"), cx, |window, cx| {
                TextInput::new(window, cx)
            });
        let to_field = window.use_keyed_state((self.id.clone(), "to-field"), cx, |window, cx| {
            TextInput::new(window, cx)
        });
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let (amount, from, to, on_change) = (
            self.amount,
            self.from.clone(),
            self.to.clone(),
            self.on_change.clone(),
        );
        let tell = move |amount: f64,
                         from: SharedString,
                         to: SharedString,
                         window: &mut Window,
                         cx: &mut App| {
            log::info!("currency converter: {amount} {from} to {to}");
            if let Some(on_change) = &on_change {
                on_change(amount, from, to, window, cx);
            }
        };
        let tell = Rc::new(tell);
        let (on_amount, on_from, on_to, on_swap) = (tell.clone(), tell.clone(), tell.clone(), tell);
        let (to_now, from_now) = (to.clone(), from.clone());
        let (_, places, _) = format::currency_parts(&self.to);
        let result = format::currency(self.amount * self.rate, &self.to);
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div().w_full().child(
                    NumberInput::new((self.id.clone(), "amount"), self.amount)
                        .range(0.0, f64::MAX)
                        .precision(2)
                        .on_change({
                            let (from, to) = (from.clone(), to.clone());
                            move |amount, window, cx| {
                                on_amount(amount, from.clone(), to.clone(), window, cx)
                            }
                        }),
                ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div().flex_1().min_w_0().child(
                            Combobox::currencies((self.id.clone(), "from"), &from_field)
                                .selected(self.from.clone())
                                .on_change(move |code, window, cx| {
                                    on_from(amount, code.clone(), to_now.clone(), window, cx)
                                }),
                        ),
                    )
                    .child(
                        IconButton::new((self.id.clone(), "swap"), IconName::ChevronsLeftRight)
                            .variant(ButtonVariant::Ghost)
                            .tooltip("Swap")
                            .on_click({
                                let (from, to) = (from.clone(), to.clone());
                                move |_, window, cx| {
                                    on_swap(amount, to.clone(), from.clone(), window, cx)
                                }
                            }),
                    )
                    .child(
                        div().flex_1().min_w_0().child(
                            Combobox::currencies((self.id.clone(), "to"), &to_field)
                                .selected(self.to.clone())
                                .on_change(move |code, window, cx| {
                                    on_to(amount, from_now.clone(), code.clone(), window, cx)
                                }),
                        ),
                    ),
            )
            .child(
                tabular(div())
                    .text_size(theme.text_size(TextSize::Xl))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.fg)
                    .child(result),
            )
            .child(
                tabular(div())
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(colors.fg_muted)
                    .child(format!(
                        "1 {} = {} {}",
                        self.from,
                        format::number(self.rate, places.max(4), format::Separators::EN),
                        self.to
                    )),
            )
    }
}

/// An address shortened to its ends, as wallets show them.
fn shortened(address: &str) -> String {
    let count = address.chars().count();
    if count <= 14 {
        return address.to_string();
    }
    let head: String = address.chars().take(6).collect();
    let tail: String = address.chars().skip(count - 4).collect();
    format!("{head}…{tail}")
}

/// A wallet at a glance: its name and network, the balance in its coin and in money, its address to copy, and a code to scan.
#[derive(IntoElement)]
pub struct CryptoWalletCard {
    id: ElementId,
    name: SharedString,
    network: SharedString,
    balance: (f64, SharedString),
    worth: (f64, SharedString),
    address: SharedString,
}

impl CryptoWalletCard {
    /// The balance in its coin, such as `(0.84, "BTC")`, and what it is worth in an ISO 4217 currency.
    pub fn new(
        id: impl Into<ElementId>,
        (name, network): (impl Into<SharedString>, impl Into<SharedString>),
        balance: (f64, &str),
        worth: (f64, &str),
        address: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            network: network.into(),
            balance: (balance.0, balance.1.to_string().into()),
            worth: (worth.0, worth.1.to_string().into()),
            address: address.into(),
        }
    }
}

impl RenderOnce for CryptoWalletCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let code = match QrCode::new(self.address.as_bytes()) {
            Ok(code) => code.size(theme.qr_code() * 0.6).into_any_element(),
            Err(error) => {
                log::error!("wallet card: no code for its address: {error:?}");
                div()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.danger)
                    .child("Address too long for a code")
                    .into_any_element()
            }
        };
        let address = self.address.clone();
        div()
            .flex()
            .gap_6()
            .p_4()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Md))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.fg)
                                    .child(self.name),
                            )
                            .child(Badge::new(self.network)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                tabular(div())
                                    .text_size(theme.text_size(TextSize::Xl))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.fg)
                                    .child(format!(
                                        "{} {}",
                                        format::number(self.balance.0, 4, format::Separators::EN),
                                        self.balance.1
                                    )),
                            )
                            .child(
                                tabular(div())
                                    .text_size(theme.text_size(TextSize::Sm))
                                    .text_color(colors.fg_muted)
                                    .child(format::currency(self.worth.0, &self.worth.1)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .font_family(theme.mono_family.clone())
                                    .text_size(theme.text_size(TextSize::Sm))
                                    .text_color(colors.fg_muted)
                                    .child(shortened(&self.address)),
                            )
                            .child(
                                IconButton::new((self.id.clone(), "copy"), IconName::Copy)
                                    .variant(ButtonVariant::Ghost)
                                    .tooltip("Copy the address")
                                    .on_click(move |_, _, cx| {
                                        log::info!("wallet card: address copied");
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            address.to_string(),
                                        ));
                                    }),
                            ),
                    ),
            )
            .child(code)
    }
}

/// Whether a transfer has settled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Settled {
    Pending,
    Done,
    Failed,
}

/// A transfer in or out: when, with whom, how much, positive when received, in what unit, and whether it settled.
#[derive(Clone, Debug, PartialEq)]
pub struct Transfer {
    pub time: Timestamp,
    pub party: SharedString,
    pub amount: f64,
    pub unit: SharedString,
    pub settled: Settled,
}

/// Transfers newest first: which way each went, with whom, when, how much, and whether it has settled.
#[derive(IntoElement)]
pub struct TransactionList {
    transfers: Vec<Transfer>,
    zone: Option<TimeZone>,
}

impl TransactionList {
    pub fn new(transfers: impl IntoIterator<Item = Transfer>) -> Self {
        let mut transfers: Vec<Transfer> = transfers.into_iter().collect();
        transfers.sort_by_key(|transfer| Reverse(transfer.time));
        Self {
            transfers,
            zone: None,
        }
    }

    /// The time zone its times read in; the system's unless set.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }
}

/// An amount in its unit: a currency's own places, or as many as a coin needs, up to eight.
fn amount(value: f64, unit: &str) -> String {
    match iso_currency::Currency::from_code(unit) {
        Some(currency) => price(value, currency.exponent().map_or(0, usize::from)),
        None => price(value, 8)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string(),
    }
}

impl RenderOnce for TransactionList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let zone = self
            .zone
            .clone()
            .unwrap_or_else(|| system_zone("TransactionList"));
        let theme = cx.theme();
        let colors = theme.colors.clone();
        div()
            .flex()
            .flex_col()
            .children(self.transfers.iter().map(|transfer| {
                let received = transfer.amount >= 0.0;
                let (icon, ink) = if received {
                    (IconName::ArrowDownLeft, colors.success)
                } else {
                    (IconName::ArrowUpRight, colors.fg_muted)
                };
                let status = match transfer.settled {
                    Settled::Pending => Some(Badge::new("Pending").tone(Tone::Warning)),
                    Settled::Failed => Some(Badge::new("Failed").tone(Tone::Danger)),
                    Settled::Done => None,
                };
                let sign = if received { "+" } else { "" };
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .py_2()
                    .border_b_1()
                    .border_color(colors.border.opacity(0.5))
                    .child(
                        div()
                            .size(theme.icon_size(IconSize::Xl))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .bg(colors.hover)
                            .child(Icon::new(icon).size(IconSize::Sm).color(ink)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Sm))
                                    .text_color(colors.fg)
                                    .child(Ellipsis::new(transfer.party.clone())),
                            )
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Xs))
                                    .text_color(colors.fg_muted)
                                    .child(
                                        format::datetime(transfer.time, &zone, "%b %-d, %H:%M")
                                            .expect("a fixed pattern"),
                                    ),
                            ),
                    )
                    .children(status)
                    .child(
                        tabular(div())
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(if received { colors.success } else { colors.fg })
                            .child(format!(
                                "{sign}{} {}",
                                amount(transfer.amount, &transfer.unit),
                                transfer.unit
                            )),
                    )
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::{amount, shortened};

    #[test]
    fn amounts_keep_their_units_places() {
        assert_eq!(amount(-500.0, "EUR"), "\u{2212}500.00");
        assert_eq!(amount(1250.0, "JPY"), "1,250");
        assert_eq!(amount(0.0421, "BTC"), "0.0421");
        assert_eq!(amount(2.0, "ETH"), "2");
    }

    #[test]
    fn addresses_shorten_to_their_ends() {
        assert_eq!(
            shortened("bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"),
            "bc1qxy…0wlh"
        );
        assert_eq!(shortened("short"), "short");
    }
}
