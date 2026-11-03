use std::rc::Rc;

use gpui::{
    App, ElementId, Entity, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div,
};

use crate::{
    buttons::{Button, ButtonVariant, IconButton},
    documents::MarkdownRenderer,
    forms::{Input, Run, TextInput},
    layout::on_axis,
    lists::{ListItem, SelectableList},
    primitives::{Icon, IconName, tab_stop},
    theme::{ActiveTheme, ControlSize, IconSize, TextSize},
};

/// A help article: its key, its title, a line on it, and its body in Markdown. A link to `help:<key>` in a body opens that article.
#[derive(Clone, Debug, PartialEq)]
pub struct HelpArticle {
    pub key: SharedString,
    pub title: SharedString,
    pub summary: SharedString,
    pub body: SharedString,
}

/// The articles whose title, line or body hold every word of `query`, in order.
pub fn found<'a>(articles: &'a [HelpArticle], query: &str) -> Vec<&'a HelpArticle> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    articles
        .iter()
        .filter(|article| {
            let text =
                format!("{} {} {}", article.title, article.summary, article.body).to_lowercase();
            words.iter().all(|word| text.contains(word.as_str()))
        })
        .collect()
}

/// Where a link in an article leads.
#[derive(Debug, PartialEq, Eq)]
enum Link<'a> {
    Article(&'a str),
    Web(&'a str),
}

fn link(url: &str) -> Link<'_> {
    match url.strip_prefix("help:") {
        Some(key) => Link::Article(key),
        None => Link::Web(url),
    }
}

type OnOpen = Rc<dyn Fn(Option<&SharedString>, &mut Window, &mut App)>;

/// Help beside the work: a search over the articles and their list, where Enter or a double press opens one, or one article with the way back to all. Focus follows: an article opened takes it to All articles, and the way back takes it to the search. Contact support and Keyboard shortcuts wait at its foot when the owner handles them. It fills its box, which the host sizes.
#[derive(IntoElement)]
pub struct HelpPanel {
    id: ElementId,
    articles: Vec<HelpArticle>,
    search: Entity<TextInput>,
    open: Option<SharedString>,
    on_open: OnOpen,
    on_close: Option<Run>,
    on_contact: Option<Run>,
    on_shortcuts: Option<Run>,
}

impl HelpPanel {
    /// `search` is the find field, which the owner keeps; `open` names the article shown, if any.
    pub fn new(
        id: impl Into<ElementId>,
        articles: impl IntoIterator<Item = HelpArticle>,
        search: &Entity<TextInput>,
        open: Option<SharedString>,
        on_open: impl Fn(Option<&SharedString>, &mut Window, &mut App) + 'static,
    ) -> Self {
        let articles: Vec<HelpArticle> = articles.into_iter().collect();
        if let Some(key) = &open
            && !articles.iter().any(|article| &article.key == key)
        {
            log::error!("help panel: no article {key}; none open");
        }
        Self {
            id: id.into(),
            articles,
            search: search.clone(),
            open,
            on_open: Rc::new(on_open),
            on_close: None,
            on_contact: None,
            on_shortcuts: None,
        }
    }

    /// Shows a close button.
    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }

    /// Shows Contact support at the foot.
    pub fn on_contact(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_contact = Some(Rc::new(handler));
        self
    }

    /// Shows Keyboard shortcuts at the foot.
    pub fn on_shortcuts(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_shortcuts = Some(Rc::new(handler));
        self
    }
}

/// A button at the panel's foot.
fn foot_button(
    id: &ElementId,
    key: &'static str,
    label: &'static str,
    icon: IconName,
    run: Run,
) -> Button {
    Button::new((id.clone(), key), label)
        .variant(ButtonVariant::Ghost)
        .size(ControlSize::Sm)
        .icon(icon)
        .on_click(move |_, window, cx| {
            log::info!("help panel: {key}");
            run(window, cx)
        })
}

impl RenderOnce for HelpPanel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let on_open = self.on_open;
        let back_focus = tab_stop((id.clone(), "back").into(), true, window, cx);
        let back_handle = back_focus.clone();
        let search_focus = self.search.read(cx).focus().clone();
        let on_open: OnOpen = Rc::new(move |key, window, cx| {
            match key {
                Some(_) => window.focus(&back_focus, cx),
                None => window.focus(&search_focus, cx),
            }
            on_open(key, window, cx)
        });
        let theme = cx.theme();
        let close = self.on_close.map(|run| {
            IconButton::new((id.clone(), "close"), IconName::X)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip("Close help")
                .on_click(move |_, window, cx| run(window, cx))
        });
        let head = div()
            .flex()
            .flex_none()
            .items_center()
            .justify_between()
            .gap_2()
            .px_4()
            .py_2()
            .border_b_1()
            .border_color(theme.colors.border)
            .child(div().font_weight(FontWeight::SEMIBOLD).child("Help"))
            .children(close);
        let shown = self
            .open
            .as_ref()
            .and_then(|key| self.articles.iter().find(|article| &article.key == key));
        let body = match shown {
            Some(article) => {
                let (back, follow) = (on_open.clone(), on_open.clone());
                let page = div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div().flex().child(
                            Button::new((id.clone(), "back"), "All articles")
                                .variant(ButtonVariant::Ghost)
                                .size(ControlSize::Sm)
                                .icon(IconName::ChevronLeft)
                                .focus_handle(&back_handle)
                                .on_click(move |_, window, cx| back(None, window, cx)),
                        ),
                    )
                    .child(
                        div().flex().child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(theme.text_size(TextSize::Lg))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(article.title.clone()),
                        ),
                    )
                    .child(
                        MarkdownRenderer::new(
                            (id.clone(), format!("article-{}", article.key)),
                            article.body.clone(),
                        )
                        .text_size(TextSize::Sm)
                        .on_link(move |url, window, cx| match link(url) {
                            Link::Article(key) => {
                                log::info!("help panel: link to {key}");
                                follow(Some(&key.to_string().into()), window, cx)
                            }
                            Link::Web(url) => cx.open_url(url),
                        }),
                    );
                on_axis(
                    div()
                        .id((id.clone(), "body"))
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .p_4()
                        .child(page),
                )
                .into_any_element()
            }
            None => {
                let query = self.search.read(cx).text().trim().to_string();
                let listed = found(&self.articles, &query);
                let none = listed.is_empty();
                let list = listed.iter().fold(
                    SelectableList::new((id.clone(), "articles")),
                    |list, article| {
                        list.row(
                            article.key.clone(),
                            ListItem::new(
                                (id.clone(), format!("row-{}", article.key)),
                                article.title.clone(),
                            )
                            .description(article.summary.clone()),
                        )
                    },
                );
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .child(
                        Input::new(&self.search)
                            .prefix(Icon::new(IconName::Search).size(IconSize::Sm)),
                    )
                    .children((!none).then(|| {
                        list.on_activate(move |key, window, cx| {
                            log::info!("help panel: open {key}");
                            on_open(Some(key), window, cx)
                        })
                    }))
                    .children(none.then(|| {
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(theme.colors.fg_subtle)
                            .child(format!("No article answers to \"{query}\"."))
                    }))
                    .into_any_element()
            }
        };
        let contact = self.on_contact.map(|run| {
            foot_button(
                &id,
                "contact",
                "Contact support",
                IconName::MessageSquare,
                run,
            )
        });
        let shortcuts = self.on_shortcuts.map(|run| {
            foot_button(
                &id,
                "shortcuts",
                "Keyboard shortcuts",
                IconName::Keyboard,
                run,
            )
        });
        let foot = (contact.is_some() || shortcuts.is_some()).then(|| {
            div()
                .debug_selector(|| "help-foot".into())
                .flex()
                .flex_none()
                .flex_wrap()
                .gap_1()
                .px_2()
                .py_2()
                .border_t_1()
                .border_color(theme.colors.border)
                .children(contact)
                .children(shortcuts)
        });
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(head)
            .child(body)
            .children(foot)
    }
}

#[cfg(test)]
mod tests {
    use super::{HelpArticle, Link, found, link};

    #[test]
    fn every_word_answers_in_the_title_the_line_or_the_body() {
        let article = |key: &str, title: &str, body: &str| HelpArticle {
            key: key.to_string().into(),
            title: title.to_string().into(),
            summary: "A line".into(),
            body: body.to_string().into(),
        };
        let articles = [
            article("start", "Getting started", "Make a project."),
            article("team", "Invite your team", "Send an invitation by email."),
        ];
        let keys = |query: &str| {
            found(&articles, query)
                .iter()
                .map(|article| article.key.to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            keys("invite EMAIL"),
            ["team"],
            "across the title and the body, in any case"
        );
        assert_eq!(keys("project team"), Vec::<String>::new());
        assert_eq!(keys(""), ["start", "team"]);
    }

    #[test]
    fn a_help_link_opens_an_article_and_the_rest_go_to_the_web() {
        assert_eq!(link("help:team"), Link::Article("team"));
        assert_eq!(
            link("https://example.com/help"),
            Link::Web("https://example.com/help")
        );
    }
}
