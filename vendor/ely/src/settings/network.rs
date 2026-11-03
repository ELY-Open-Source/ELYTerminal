use std::rc::Rc;

use gpui::{
    App, AppContext as _, ElementId, Entity, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div,
};

use crate::{
    buttons::{Button, ButtonVariant, SegmentedControl},
    forms::{FormField, Input, TextInput},
};

/// How requests reach the network: straight, through the system's proxy, or through one given here.
#[derive(Clone, Debug, PartialEq)]
pub enum Proxy {
    Off,
    System,
    Manual {
        host: SharedString,
        port: u16,
        bypass: Vec<SharedString>,
    },
}

/// A manual proxy read from its fields: a host, a port from 1 to 65535, and hosts to reach straight, split at commas.
pub fn manual(host: &str, port: &str, bypass: &str) -> Option<Proxy> {
    let host = host.trim();
    let port: u16 = port.trim().parse().ok().filter(|port| *port > 0)?;
    (!host.is_empty()).then(|| Proxy::Manual {
        host: host.to_string().into(),
        port,
        bypass: bypass
            .split(',')
            .map(str::trim)
            .filter(|each| !each.is_empty())
            .map(|each| SharedString::from(each.to_string()))
            .collect(),
    })
}

/// The form's own fields and way, filled from the proxy it was given.
struct Fields {
    seed: Proxy,
    way: SharedString,
    host: Entity<TextInput>,
    port: Entity<TextInput>,
    bypass: Entity<TextInput>,
}

type OnProxy = Rc<dyn Fn(Proxy, &mut Window, &mut App)>;

/// How requests reach the network: off, the system's proxy, or a manual one with its host, port and the hosts to reach straight. Apply hands the owner the proxy and waits for a host and a good port when manual; a new proxy from the owner refills it.
#[derive(IntoElement)]
pub struct ProxySettings {
    id: ElementId,
    proxy: Proxy,
    on_apply: OnProxy,
}

impl ProxySettings {
    pub fn new(
        id: impl Into<ElementId>,
        proxy: Proxy,
        on_apply: impl Fn(Proxy, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            proxy,
            on_apply: Rc::new(on_apply),
        }
    }
}

impl RenderOnce for ProxySettings {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (id, proxy) = (self.id, self.proxy);
        let fill = |proxy: &Proxy, window: &mut Window, cx: &mut App| {
            let (way, host, port, bypass) = match proxy {
                Proxy::Off => ("off", String::new(), String::new(), String::new()),
                Proxy::System => ("system", String::new(), String::new(), String::new()),
                Proxy::Manual { host, port, bypass } => (
                    "manual",
                    host.to_string(),
                    port.to_string(),
                    bypass
                        .iter()
                        .map(|each| each.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                ),
            };
            let mut field = |text: String| {
                cx.new(|cx| {
                    let mut input = TextInput::new(window, cx);
                    input.set_text(text, cx);
                    input
                })
            };
            Fields {
                seed: proxy.clone(),
                way: way.into(),
                host: field(host),
                port: field(port),
                bypass: field(bypass),
            }
        };
        let fields = window.use_keyed_state((id.clone(), "fields"), cx, |window, cx| {
            fill(&proxy, window, cx)
        });
        if fields.read(cx).seed != proxy {
            log::info!("proxy settings: refilled");
            let fresh = fill(&proxy, window, cx);
            fields.update(cx, |fields, _| *fields = fresh);
        }
        let (way, host, port, bypass) = {
            let fields = fields.read(cx);
            (
                fields.way.clone(),
                fields.host.clone(),
                fields.port.clone(),
                fields.bypass.clone(),
            )
        };
        let text = |input: &Entity<TextInput>| input.read(cx).text().to_string();
        let read = match way.as_ref() {
            "off" => Some(Proxy::Off),
            "system" => Some(Proxy::System),
            "manual" => manual(&text(&host), &text(&port), &text(&bypass)),
            other => panic!("proxy settings: no way named {other}"),
        };
        let on_apply = self.on_apply;
        let switched = fields.clone();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                SegmentedControl::new((id.clone(), "way"), way.clone())
                    .segment("off", "No proxy", None)
                    .segment("system", "System", None)
                    .segment("manual", "Manual", None)
                    .on_change(move |value, _, cx| {
                        switched.update(cx, |fields, cx| {
                            fields.way = value.clone();
                            cx.notify();
                        })
                    }),
            )
            .children((way == "manual").then(|| {
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_3()
                            .child(
                                div().flex_1().min_w_32().child(
                                    FormField::new((id.clone(), "host"), "Host")
                                        .child(Input::new(&host)),
                                ),
                            )
                            .child(
                                div().w_24().flex_none().child(
                                    FormField::new((id.clone(), "port"), "Port")
                                        .child(Input::new(&port)),
                                ),
                            ),
                    )
                    .child(
                        FormField::new((id.clone(), "bypass"), "Reach straight")
                            .description("Hosts split by commas, such as localhost, *.internal")
                            .child(Input::new(&bypass)),
                    )
            }))
            .child(
                div().flex().child(
                    Button::new((id, "apply"), "Apply")
                        .variant(ButtonVariant::Primary)
                        .disabled(read.is_none())
                        .on_click(move |_, window, cx| {
                            let proxy = read.clone().expect("Apply rests until the proxy reads");
                            log::info!("proxy settings: apply {proxy:?}");
                            on_apply(proxy, window, cx);
                        }),
                ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{Proxy, manual};

    #[test]
    fn a_manual_proxy_needs_a_host_and_a_good_port() {
        assert_eq!(
            manual(" proxy.corp ", "8080", "localhost, *.internal,"),
            Some(Proxy::Manual {
                host: "proxy.corp".into(),
                port: 8080,
                bypass: vec!["localhost".into(), "*.internal".into()]
            })
        );
        assert_eq!(manual("", "8080", ""), None);
        assert_eq!(manual("proxy.corp", "0", ""), None);
        assert_eq!(manual("proxy.corp", "99999", ""), None);
    }
}
