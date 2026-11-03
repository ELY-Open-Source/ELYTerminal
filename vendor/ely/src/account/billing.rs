use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div, prelude::*,
};

use super::login::Run;
use crate::{
    buttons::{Button, ButtonVariant, ConfirmButton, ConfirmMode, IconButton},
    data_display::{Badge, Meter, Tone},
    layout::Card,
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, TextSize},
    typography::{
        Ellipsis,
        format::{Separators, currency, number},
        tabular,
    },
};

type OnKey = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// Where a subscription stands, each with the date that matters, as words.
#[derive(Clone, Debug, PartialEq)]
pub enum Standing {
    Active { renews: SharedString },
    Trial { ends: SharedString },
    Canceled { ends: SharedString },
    PastDue,
}

impl Standing {
    fn badge(&self) -> (&'static str, Tone) {
        match self {
            Standing::Active { .. } => ("Active", Tone::Success),
            Standing::Trial { .. } => ("Trial", Tone::Info),
            Standing::Canceled { .. } => ("Canceled", Tone::Neutral),
            Standing::PastDue => ("Past due", Tone::Danger),
        }
    }

    fn line(&self) -> String {
        match self {
            Standing::Active { renews } => format!("Renews on {renews}."),
            Standing::Trial { ends } => format!("The trial ends on {ends}."),
            Standing::Canceled { ends } => format!("Ends on {ends}. Nothing more will be charged."),
            Standing::PastDue => {
                "The last payment failed. Update the card to keep the plan.".into()
            }
        }
    }
}

/// A plan paid for: its name, its price per period in a currency, and where it stands.
#[derive(Clone, Debug, PartialEq)]
pub struct Subscription {
    pub plan: SharedString,
    pub price: f64,
    pub currency: SharedString,
    pub yearly: bool,
    pub standing: Standing,
}

/// A plan at a glance: its name and where it stands, its price, what happens next, and a way to change it, cancel it after a second press, or take it back.
#[derive(IntoElement)]
pub struct SubscriptionCard {
    id: ElementId,
    subscription: Subscription,
    on_change: Option<Run>,
    on_cancel: Option<Run>,
    on_resume: Option<Run>,
}

impl SubscriptionCard {
    pub fn new(id: impl Into<ElementId>, subscription: Subscription) -> Self {
        Self {
            id: id.into(),
            subscription,
            on_change: None,
            on_cancel: None,
            on_resume: None,
        }
    }

    /// Shows "Change plan".
    pub fn on_change(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    /// Shows "Cancel plan" while the plan runs.
    pub fn on_cancel(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_cancel = Some(Rc::new(handler));
        self
    }

    /// Shows "Keep plan" once canceled.
    pub fn on_resume(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_resume = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SubscriptionCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (id, plan) = (self.id, self.subscription);
        let theme = cx.theme();
        let (state, tone) = plan.standing.badge();
        let canceled = matches!(plan.standing, Standing::Canceled { .. });
        let period = if plan.yearly { "per year" } else { "per month" };
        let change = self.on_change.map(|run| {
            Button::new((id.clone(), "change"), "Change plan")
                .on_click(move |_, window, cx| run(window, cx))
                .into_any_element()
        });
        let end = match canceled {
            true => self.on_resume.map(|run| {
                Button::new((id.clone(), "resume"), "Keep plan")
                    .variant(ButtonVariant::Primary)
                    .on_click(move |_, window, cx| run(window, cx))
                    .into_any_element()
            }),
            false => self.on_cancel.map(|run| {
                ConfirmButton::new((id.clone(), "cancel"), "Cancel plan", ConfirmMode::Twice)
                    .on_confirm(move |window, cx| {
                        log::info!("subscription card: cancel");
                        run(window, cx);
                    })
                    .into_any_element()
            }),
        };
        let actions: Vec<AnyElement> = change.into_iter().chain(end).collect();
        Card::new().child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .text_size(theme.text_size(TextSize::Sm))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_size(theme.text_size(TextSize::Md))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(plan.plan),
                        )
                        .child(Badge::new(state).tone(tone)),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_baseline()
                        .gap_1()
                        .child(tabular(
                            div()
                                .text_size(theme.text_size(TextSize::Xl))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(currency(plan.price, &plan.currency)),
                        ))
                        .child(div().text_color(theme.colors.fg_muted).child(period)),
                )
                .child(
                    div().flex().child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(theme.colors.fg_muted)
                            .child(plan.standing.line()),
                    ),
                )
                .children(
                    (!actions.is_empty())
                        .then(|| div().flex().flex_wrap().gap_2().pt_1().children(actions)),
                ),
        )
    }
}

/// A limit on a plan: its key and name, how much is used of how much, and the unit.
#[derive(Clone, Debug, PartialEq)]
pub struct Quota {
    pub key: SharedString,
    pub name: SharedString,
    pub used: f64,
    pub limit: f64,
    pub unit: SharedString,
}

/// An amount to a tenth, without a trailing zero: 8, 8.5, 1,200.
fn amount(value: f64) -> String {
    let text = number(value, 1, Separators::EN);
    text.strip_suffix(".0").map_or(text.clone(), str::to_string)
}

/// The words under a quota's bar: "8.5 of 10 GB", or how far past its limit.
pub fn quota_line(quota: &Quota) -> String {
    assert!(quota.limit > 0.0, "quota {}: no limit", quota.key);
    match quota.used > quota.limit {
        true => format!(
            "{} past the {} {} limit",
            amount(quota.used - quota.limit),
            amount(quota.limit),
            quota.unit
        ),
        false => format!(
            "{} of {} {}",
            amount(quota.used),
            amount(quota.limit),
            quota.unit
        ),
    }
}

/// How much of each limit is used, a bar for each that turns amber near it and red at it, and when they start over.
#[derive(IntoElement)]
pub struct UsageQuota {
    id: ElementId,
    quotas: Vec<Quota>,
    resets: Option<SharedString>,
}

impl UsageQuota {
    pub fn new(id: impl Into<ElementId>, quotas: impl IntoIterator<Item = Quota>) -> Self {
        Self {
            id: id.into(),
            quotas: quotas.into_iter().collect(),
            resets: None,
        }
    }

    /// When the counts start over, as words.
    pub fn resets(mut self, when: impl Into<SharedString>) -> Self {
        self.resets = Some(when.into());
        self
    }
}

impl RenderOnce for UsageQuota {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let theme = cx.theme();
        div()
            .flex()
            .flex_col()
            .gap_4()
            .children(self.quotas.iter().map(|quota| {
                let share = (quota.used / quota.limit).clamp(0.0, 1.0) as f32;
                Meter::new(
                    (id.clone(), format!("quota-{}", quota.key)),
                    quota.name.clone(),
                    share,
                )
                .detail(quota_line(quota))
            }))
            .children(self.resets.map(|when| {
                div()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(theme.colors.fg_muted)
                    .child(format!("Resets on {when}."))
            }))
    }
}

/// Where an invoice stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvoiceState {
    Paid,
    Due,
    Failed,
    Refunded,
}

/// A bill: its key, date and what it was for as words, its amount in a currency, and where it stands.
#[derive(Clone, Debug, PartialEq)]
pub struct Invoice {
    pub key: SharedString,
    pub date: SharedString,
    pub description: SharedString,
    pub amount: f64,
    pub currency: SharedString,
    pub state: InvoiceState,
}

/// Past bills, newest first as given: what each was for and its amount, its date and state below, and a way to download it.
#[derive(IntoElement)]
pub struct BillingHistory {
    id: ElementId,
    invoices: Vec<Invoice>,
    on_download: Option<OnKey>,
}

impl BillingHistory {
    pub fn new(id: impl Into<ElementId>, invoices: impl IntoIterator<Item = Invoice>) -> Self {
        Self {
            id: id.into(),
            invoices: invoices.into_iter().collect(),
            on_download: None,
        }
    }

    /// Shows a download button on each.
    pub fn on_download(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_download = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for BillingHistory {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let theme = cx.theme();
        let rows = self.invoices.iter().enumerate().map(|(ix, invoice)| {
            let (state, tone) = match invoice.state {
                InvoiceState::Paid => ("Paid", Tone::Success),
                InvoiceState::Due => ("Due", Tone::Warning),
                InvoiceState::Failed => ("Failed", Tone::Danger),
                InvoiceState::Refunded => ("Refunded", Tone::Neutral),
            };
            let download = self.on_download.clone().map(|run| {
                let key = invoice.key.clone();
                div().flex_none().child(
                    IconButton::new(
                        (id.clone(), format!("download-{}", invoice.key)),
                        IconName::Download,
                    )
                    .size(ControlSize::Sm)
                    .tooltip("Download")
                    .on_click(move |_, window, cx| {
                        log::info!("billing history: download {key}");
                        run(&key, window, cx);
                    }),
                )
            });
            div()
                .flex()
                .items_center()
                .gap_3()
                .py_2p5()
                .when(ix > 0, |row| {
                    row.border_t_1().border_color(theme.colors.border)
                })
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .child(Ellipsis::new(invoice.description.clone())),
                                )
                                .child(tabular(
                                    div()
                                        .flex_none()
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(currency(invoice.amount, &invoice.currency)),
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .text_size(theme.text_size(TextSize::Xs))
                                .text_color(theme.colors.fg_muted)
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .child(Ellipsis::new(invoice.date.clone())),
                                )
                                .child(div().flex_none().child(Badge::new(state).tone(tone))),
                        ),
                )
                .children(download)
        });
        let empty = self.invoices.is_empty().then(|| {
            div()
                .py_2()
                .text_color(theme.colors.fg_muted)
                .child("No bills yet.")
        });
        div()
            .flex()
            .flex_col()
            .text_size(theme.text_size(TextSize::Sm))
            .children(rows)
            .children(empty)
    }
}

#[cfg(test)]
mod tests {
    use super::{Quota, quota_line};

    fn quota(used: f64, limit: f64) -> Quota {
        Quota {
            key: "storage".into(),
            name: "Storage".into(),
            used,
            limit,
            unit: "GB".into(),
        }
    }

    #[test]
    fn a_quota_reads_its_use_or_how_far_past_it_went() {
        assert_eq!(quota_line(&quota(8.5, 10.0)), "8.5 of 10 GB");
        assert_eq!(quota_line(&quota(1_250.0, 2_000.0)), "1,250 of 2,000 GB");
        assert_eq!(quota_line(&quota(12.4, 10.0)), "2.4 past the 10 GB limit");
    }
}
