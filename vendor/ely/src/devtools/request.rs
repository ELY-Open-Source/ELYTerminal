use std::rc::Rc;

use gpui::{
    App, AppContext as _, ElementId, Entity, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div,
};

use crate::{
    buttons::{Button, ButtonVariant},
    editor::CodeEditor,
    feedback::InlineMessage,
    forms::{Choice, FormField, Input, KeyValueInput, PasswordInput, Select, TextInput},
    navigation::Tabs,
    primitives::Severity,
    theme::{ActiveTheme, Radius, TextSize},
    typography::Ellipsis,
};

/// An HTTP method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl Method {
    pub const ALL: [Method; 7] = [
        Method::Get,
        Method::Post,
        Method::Put,
        Method::Patch,
        Method::Delete,
        Method::Head,
        Method::Options,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Patch => "PATCH",
            Method::Delete => "DELETE",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
        }
    }

    fn named(name: &str) -> Method {
        *Method::ALL
            .iter()
            .find(|method| method.name() == name)
            .unwrap_or_else(|| panic!("no method {name}"))
    }
}

/// How a request signs in.
#[derive(Clone, Debug, PartialEq)]
pub enum Auth {
    None,
    Bearer(SharedString),
    Basic {
        user: SharedString,
        password: SharedString,
    },
}

/// A request to send: its method and address, its query's pairs, its headers, its body and how it signs in.
#[derive(Clone, Debug, PartialEq)]
pub struct ApiRequest {
    pub key: SharedString,
    pub method: Method,
    pub url: SharedString,
    pub params: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
    pub body: SharedString,
    pub auth: Auth,
}

impl ApiRequest {
    pub fn new(key: impl Into<SharedString>, method: Method, url: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            method,
            url: url.into(),
            params: Vec::new(),
            headers: Vec::new(),
            body: SharedString::default(),
            auth: Auth::None,
        }
    }

    /// Its address with its query's pairs encoded after it; pairs with no key are left out.
    pub fn address(&self) -> String {
        let pairs: Vec<String> = self
            .params
            .iter()
            .filter(|(key, _)| !key.trim().is_empty())
            .map(|(key, value)| format!("{}={}", percent(key.trim()), percent(value)))
            .collect();
        match (pairs.is_empty(), self.url.contains('?')) {
            (true, _) => self.url.to_string(),
            (false, true) => format!("{}&{}", self.url, pairs.join("&")),
            (false, false) => format!("{}?{}", self.url, pairs.join("&")),
        }
    }
}

/// Text made safe for a URL's query: letters, digits and `-_.~` stay, anything else becomes %XX of its UTF-8 bytes.
pub fn percent(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

/// `text` with each `{{name}}` swapped for its value, and the names that had none.
pub fn resolve(text: &str, variables: &[(SharedString, SharedString)]) -> (String, Vec<String>) {
    let (mut out, mut missing, mut rest) = (String::new(), Vec::new(), text);
    while let Some(open) = rest.find("{{") {
        let Some(close) = rest[open..].find("}}") else {
            break;
        };
        let name = rest[open + 2..open + close].trim();
        out.push_str(&rest[..open]);
        match variables.iter().find(|(key, _)| key == name) {
            Some((_, value)) => out.push_str(value),
            None => {
                out.push_str(&rest[open..open + close + 2]);
                missing.push(name.to_string());
            }
        }
        rest = &rest[open + close + 2..];
    }
    out.push_str(rest);
    (out, missing)
}

type OnRequest = Rc<dyn Fn(ApiRequest, &mut Window, &mut App)>;

/// The builder's own state, filled from the request named by `seed`.
struct Draft {
    seed: SharedString,
    method: Method,
    url: Entity<TextInput>,
    params: Vec<(String, String)>,
    headers: Vec<(String, String)>,
    body: Entity<CodeEditor>,
    auth: SharedString,
    token: Entity<TextInput>,
    user: Entity<TextInput>,
    password: Entity<TextInput>,
    tab: SharedString,
}

impl Draft {
    fn filled(request: &ApiRequest, window: &mut Window, cx: &mut App) -> Self {
        let mut field = |text: &str| {
            let text = text.to_string();
            cx.new(|cx| {
                let mut input = TextInput::new(window, cx);
                input.set_text(text, cx);
                input
            })
        };
        let (auth, token, user, password) = match &request.auth {
            Auth::None => ("none", "", "", ""),
            Auth::Bearer(token) => ("bearer", token.as_ref(), "", ""),
            Auth::Basic { user, password } => ("basic", "", user.as_ref(), password.as_ref()),
        };
        let (url, token, user, password) = (
            field(&request.url),
            field(token),
            field(user),
            field(password),
        );
        let body = request.body.to_string();
        Self {
            seed: request.key.clone(),
            method: request.method,
            url,
            params: request.params.clone(),
            headers: request.headers.clone(),
            body: cx.new(|cx| CodeEditor::new(body, window, cx).language("JSON")),
            auth: auth.into(),
            token,
            user,
            password,
            tab: "params".into(),
        }
    }

    fn read(&self, key: &SharedString, cx: &App) -> ApiRequest {
        let text =
            |input: &Entity<TextInput>| SharedString::from(input.read(cx).text().to_string());
        ApiRequest {
            key: key.clone(),
            method: self.method,
            url: SharedString::from(self.url.read(cx).text().trim().to_string()),
            params: self.params.clone(),
            headers: self.headers.clone(),
            body: SharedString::from(self.body.read(cx).text().to_string()),
            auth: match self.auth.as_ref() {
                "bearer" => Auth::Bearer(text(&self.token)),
                "basic" => Auth::Basic {
                    user: text(&self.user),
                    password: text(&self.password),
                },
                "none" => Auth::None,
                other => panic!("request builder: no sign-in named {other}"),
            },
        }
    }
}

/// A request to build: its method and address, then tabs for the query's pairs, headers, body and sign-in. Under the address, the one that goes out, with the environment's `{{variables}}` filled in, or the ones it lacks. Send hands the owner the request, waiting for an address; a new request from the owner refills it.
#[derive(IntoElement)]
pub struct ApiRequestBuilder {
    id: ElementId,
    request: ApiRequest,
    variables: Vec<(SharedString, SharedString)>,
    sending: bool,
    on_send: Option<OnRequest>,
}

impl ApiRequestBuilder {
    pub fn new(id: impl Into<ElementId>, request: ApiRequest) -> Self {
        Self {
            id: id.into(),
            request,
            variables: Vec::new(),
            sending: false,
            on_send: None,
        }
    }

    /// The environment's variables, which `{{name}}` in the address stands for.
    pub fn variables(
        mut self,
        variables: impl IntoIterator<Item = (impl Into<SharedString>, impl Into<SharedString>)>,
    ) -> Self {
        self.variables = variables
            .into_iter()
            .map(|(name, value)| (name.into(), value.into()))
            .collect();
        self
    }

    /// While the owner waits for an answer, Send rests.
    pub fn sending(mut self, sending: bool) -> Self {
        self.sending = sending;
        self
    }

    /// Gets the request as written, `{{names}}` and all; `resolve` fills them from the environment.
    pub fn on_send(
        mut self,
        handler: impl Fn(ApiRequest, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_send = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ApiRequestBuilder {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (id, request) = (self.id, self.request);
        let draft = window.use_keyed_state((id.clone(), "draft"), cx, |window, cx| {
            Draft::filled(&request, window, cx)
        });
        if draft.read(cx).seed != request.key {
            log::info!("request builder: filled from {}", request.key);
            let fresh = Draft::filled(&request, window, cx);
            draft.update(cx, |draft, _| *draft = fresh);
        }
        let now = draft.read(cx).read(&request.key, cx);
        let (tab, auth) = (draft.read(cx).tab.clone(), draft.read(cx).auth.clone());
        let (url, body, token, user, password) = {
            let draft = draft.read(cx);
            (
                draft.url.clone(),
                draft.body.clone(),
                draft.token.clone(),
                draft.user.clone(),
                draft.password.clone(),
            )
        };
        let (going, missing) = resolve(&now.address(), &self.variables);
        let theme = cx.theme();
        let update = |edit: fn(&mut Draft, SharedString)| {
            let draft = draft.clone();
            move |value: &SharedString, _: &mut Window, cx: &mut App| {
                draft.update(cx, |draft, cx| {
                    edit(draft, value.clone());
                    cx.notify();
                })
            }
        };
        let pairs = |edit: fn(&mut Draft, Vec<(String, String)>)| {
            let draft = draft.clone();
            move |rows: Vec<(String, String)>, _: &mut Window, cx: &mut App| {
                draft.update(cx, |draft, cx| {
                    edit(draft, rows);
                    cx.notify();
                })
            }
        };
        let panel = match tab.as_ref() {
            "params" => KeyValueInput::new(
                (id.clone(), format!("params-{}", request.key)),
                now.params.clone(),
            )
            .on_change(pairs(|draft, rows| draft.params = rows))
            .into_any_element(),
            "headers" => KeyValueInput::new(
                (id.clone(), format!("headers-{}", request.key)),
                now.headers.clone(),
            )
            .on_change(pairs(|draft, rows| draft.headers = rows))
            .into_any_element(),
            "body" => div()
                .h(theme.list_max_height())
                .rounded(theme.radius(Radius::Md))
                .border_1()
                .border_color(theme.colors.border)
                .overflow_hidden()
                .child(body)
                .into_any_element(),
            _ => div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    Select::new(
                        (id.clone(), "auth"),
                        [
                            Choice::new("none", "No sign-in"),
                            Choice::new("bearer", "Bearer token"),
                            Choice::new("basic", "Basic"),
                        ],
                    )
                    .selected(auth.clone())
                    .on_change(update(|draft, value| draft.auth = value)),
                )
                .children((auth == "bearer").then(|| {
                    FormField::new((id.clone(), "token"), "Token").child(PasswordInput::new(&token))
                }))
                .children(
                    (auth == "basic").then(|| {
                        FormField::new((id.clone(), "user"), "User").child(Input::new(&user))
                    }),
                )
                .children((auth == "basic").then(|| {
                    FormField::new((id.clone(), "password"), "Password")
                        .child(PasswordInput::new(&password))
                }))
                .into_any_element(),
        };
        let ready = !now.url.is_empty() && !self.sending;
        let (sent, on_send) = (draft.clone(), self.on_send);
        let key = request.key.clone();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        div().flex_none().w(theme.label_width()).child(
                            Select::new(
                                (id.clone(), "method"),
                                Method::ALL
                                    .iter()
                                    .map(|method| Choice::new(method.name(), method.name())),
                            )
                            .selected(now.method.name())
                            .on_change(update(|draft, value| draft.method = Method::named(&value))),
                        ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(theme.label_width())
                            .child(Input::new(&url)),
                    )
                    .child(
                        Button::new(
                            (id.clone(), "send"),
                            if self.sending { "Sending…" } else { "Send" },
                        )
                        .variant(ButtonVariant::Primary)
                        .disabled(!ready)
                        .on_click(move |_, window, cx| {
                            let request = sent.read(cx).read(&key, cx);
                            log::info!(
                                "request builder: send {} {}",
                                request.method.name(),
                                request.address()
                            );
                            if let Some(on_send) = &on_send {
                                on_send(request, window, cx);
                            }
                        }),
                    ),
            )
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(theme.colors.fg_subtle)
                    .child(Ellipsis::new(going)),
            )
            .children((!missing.is_empty()).then(|| {
                InlineMessage::new(
                    Severity::Warning,
                    format!("No value for {}", missing.join(", ")),
                )
            }))
            .child(
                Tabs::new(
                    (id, "tabs"),
                    [
                        Choice::new("params", "Params"),
                        Choice::new("headers", "Headers"),
                        Choice::new("body", "Body"),
                        Choice::new("auth", "Auth"),
                    ],
                    tab,
                )
                .on_change(update(|draft, value| draft.tab = value))
                .panel(panel),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{ApiRequest, Method, percent, resolve};

    #[test]
    fn an_address_carries_its_encoded_pairs() {
        let mut request = ApiRequest::new("r", Method::Get, "https://api.example.com/items");
        request.params = vec![
            ("q".into(), "red shoes".into()),
            ("".into(), "dropped".into()),
            ("page".into(), "2".into()),
        ];
        assert_eq!(
            request.address(),
            "https://api.example.com/items?q=red%20shoes&page=2"
        );
        assert_eq!(percent("café & co"), "caf%C3%A9%20%26%20co");
        request.url = "https://api.example.com/items?sort=new".into();
        request.params.truncate(1);
        assert_eq!(
            request.address(),
            "https://api.example.com/items?sort=new&q=red%20shoes",
            "pairs join a query already there"
        );
    }

    #[test]
    fn variables_fill_in_and_the_missing_are_named() {
        let variables = [("host".into(), "api.example.com".into())];
        assert_eq!(
            resolve("https://{{host}}/v1?key={{ token }}", &variables),
            (
                "https://api.example.com/v1?key={{ token }}".to_string(),
                vec!["token".to_string()]
            )
        );
        assert_eq!(
            resolve("{{host}}/{{open", &variables),
            ("api.example.com/{{open".to_string(), Vec::new()),
            "braces that never close stay as written"
        );
    }
}
