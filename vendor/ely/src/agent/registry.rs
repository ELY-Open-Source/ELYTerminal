use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::*,
};

use crate::{
    buttons::SegmentedControl,
    theme::{ActiveTheme, ControlSize, TextSize},
    typography::Ellipsis,
};

/// What an agent may do with a tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolAccess {
    Allow,
    Ask,
    Deny,
}

impl ToolAccess {
    const ALL: [(ToolAccess, &'static str, &'static str); 3] = [
        (ToolAccess::Allow, "allow", "Allow"),
        (ToolAccess::Ask, "ask", "Ask"),
        (ToolAccess::Deny, "deny", "Deny"),
    ];

    fn key(self) -> &'static str {
        Self::ALL
            .iter()
            .find(|(access, _, _)| *access == self)
            .expect("every access has a key")
            .1
    }
}

/// A tool the agent can call: its name, where it comes from, what it does, and what the agent may do with it.
#[derive(Clone, Debug, PartialEq)]
pub struct RegisteredTool {
    pub name: SharedString,
    pub source: SharedString,
    pub description: SharedString,
    pub access: ToolAccess,
}

/// The tools whose name or description holds every word of `query`, ignoring case, by index.
pub(crate) fn found(tools: &[RegisteredTool], query: &str) -> Vec<usize> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    (0..tools.len())
        .filter(|ix| {
            let text = format!("{} {}", tools[*ix].name, tools[*ix].description).to_lowercase();
            words.iter().all(|word| text.contains(word))
        })
        .collect()
}

/// The kept tools gathered by source, sources in the order they first appear.
pub(crate) fn grouped(tools: &[RegisteredTool], kept: &[usize]) -> Vec<(SharedString, Vec<usize>)> {
    let mut groups: Vec<(SharedString, Vec<usize>)> = Vec::new();
    for &ix in kept {
        let source = &tools[ix].source;
        match groups.iter_mut().find(|(name, _)| name == source) {
            Some((_, items)) => items.push(ix),
            None => groups.push((source.clone(), vec![ix])),
        }
    }
    groups
}

type OnAccess = Rc<dyn Fn(usize, ToolAccess, &mut Window, &mut App)>;

/// Every tool the agent may call, grouped by where it comes from and found by the owner's query: each allowed, asked for first, or denied.
#[derive(IntoElement)]
pub struct ToolRegistry {
    id: ElementId,
    tools: Vec<RegisteredTool>,
    query: SharedString,
    on_access: Option<OnAccess>,
}

impl ToolRegistry {
    pub fn new(id: impl Into<ElementId>, tools: impl IntoIterator<Item = RegisteredTool>) -> Self {
        Self {
            id: id.into(),
            tools: tools.into_iter().collect(),
            query: SharedString::default(),
            on_access: None,
        }
    }

    /// Keeps the tools that hold every word, such as a `forms::SearchInput`'s.
    pub fn query(mut self, query: impl Into<SharedString>) -> Self {
        self.query = query.into();
        self
    }

    /// Gets a tool's index and its new access.
    pub fn on_access(
        mut self,
        handler: impl Fn(usize, ToolAccess, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_access = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ToolRegistry {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let groups = grouped(&self.tools, &found(&self.tools, &self.query));
        let empty = groups.is_empty();
        div()
            .flex()
            .flex_col()
            .gap_4()
            .text_size(theme.text_size(TextSize::Sm))
            .when(empty, |list| {
                list.child(div().text_color(colors.fg_subtle).child("No tools match"))
            })
            .children(groups.into_iter().map(|(source, items)| {
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .pb_1()
                            .text_size(theme.text_size(TextSize::Xs))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.fg_subtle)
                            .child(format!("{source} · {}", items.len())),
                    )
                    .children(items.into_iter().map(|ix| {
                        let tool = &self.tools[ix];
                        let access = self.on_access.clone();
                        let control = ToolAccess::ALL.iter().fold(
                            SegmentedControl::new(
                                (self.id.clone(), format!("access-{ix}")),
                                tool.access.key(),
                            )
                            .size(ControlSize::Sm),
                            |control, (_, key, label)| control.segment(*key, *label, None),
                        );
                        let control = match access {
                            Some(access) => control.on_change(move |key, window, cx| {
                                let next = ToolAccess::ALL
                                    .iter()
                                    .find(|(_, name, _)| *name == key.as_ref())
                                    .expect("a segment names an access")
                                    .0;
                                log::info!("tool registry: tool {ix} {next:?}");
                                access(ix, next, window, cx)
                            }),
                            None => control,
                        };
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap_x_3()
                            .gap_y_1()
                            .py_2()
                            .border_t_1()
                            .border_color(colors.border)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(theme.label_width())
                                    .child(
                                        div()
                                            .font_family(theme.mono_family.clone())
                                            .child(Ellipsis::new(tool.name.clone())),
                                    )
                                    .child(
                                        div()
                                            .text_size(theme.text_size(TextSize::Xs))
                                            .text_color(colors.fg_muted)
                                            .child(Ellipsis::new(tool.description.clone())),
                                    ),
                            )
                            .child(div().flex_none().child(control))
                    }))
            }))
    }
}

#[cfg(test)]
mod tests {
    use gpui::SharedString;

    use super::{RegisteredTool, ToolAccess, found, grouped};

    fn tool(name: &str, description: &str) -> RegisteredTool {
        RegisteredTool {
            name: name.to_string().into(),
            source: "files".into(),
            description: description.to_string().into(),
            access: ToolAccess::Ask,
        }
    }

    #[test]
    fn a_query_keeps_tools_that_hold_every_word() {
        let tools = [
            tool("read_file", "Reads a file"),
            tool("write_file", "Writes a file"),
            tool("web_search", "Searches the web"),
        ];
        assert_eq!(found(&tools, ""), [0, 1, 2]);
        assert_eq!(found(&tools, "FILE"), [0, 1]);
        assert_eq!(found(&tools, "writes file"), [1]);
        assert_eq!(found(&tools, "web file"), Vec::<usize>::new());
    }

    #[test]
    fn kept_tools_gather_by_source_in_first_seen_order() {
        let from = |source: &str| RegisteredTool {
            source: source.to_string().into(),
            ..tool("tool", "")
        };
        let tools = [from("A"), from("B"), from("A")];
        let (a, b) = (SharedString::from("A"), SharedString::from("B"));
        assert_eq!(
            grouped(&tools, &[0, 1, 2]),
            [(a.clone(), vec![0, 2]), (b.clone(), vec![1])]
        );
        assert_eq!(grouped(&tools, &[1, 2]), [(b, vec![1]), (a, vec![2])]);
    }
}
