use std::rc::Rc;

use gpui::{
    App, ElementId, Entity, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window,
    div, rems,
};

use super::{datum::read_json, document::JsonViewer};
use crate::{
    buttons::{Button, ButtonVariant},
    editor::CodeEditor,
    feedback::{EmptyState, InlineMessage},
    forms::TextInput,
    lists::{Tree, TreeNode},
    primitives::{IconName, Severity},
    theme::{ActiveTheme, Radius, TextSize},
};

/// What kind of type a GraphQL schema declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeKind {
    Object,
    Input,
    Enum,
    Scalar,
}

impl TypeKind {
    fn word(self) -> &'static str {
        match self {
            TypeKind::Object => "type",
            TypeKind::Input => "input",
            TypeKind::Enum => "enum",
            TypeKind::Scalar => "scalar",
        }
    }
}

/// A type in a schema: its name, its kind, and its fields with their types, or an enum's values.
#[derive(Clone, Debug, PartialEq)]
pub struct SchemaType {
    pub name: SharedString,
    pub kind: TypeKind,
    pub fields: Vec<(SharedString, SharedString)>,
}

type OnRun = Rc<dyn Fn(String, String, &mut Window, &mut App)>;
type OnKey = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// A GraphQL schema to browse beside a query and its variables to write, over the answer to read. The schema lists each type's fields with their types; Enter or a double press on a field hands the owner its path, `Type.field`. Run hands the owner the query and the variables; it rests while it runs and while the variables do not read as JSON, which says where. The answer shows as a tree.
#[derive(IntoElement)]
pub struct GraphQLExplorer {
    id: ElementId,
    types: Vec<SchemaType>,
    query: Entity<CodeEditor>,
    variables: Entity<CodeEditor>,
    answer: Option<SharedString>,
    search: Entity<TextInput>,
    running: bool,
    on_run: OnRun,
    on_pick: Option<OnKey>,
}

impl GraphQLExplorer {
    /// The owner keeps the query's and the variables' editors, and the answer's find field.
    pub fn new(
        id: impl Into<ElementId>,
        types: impl IntoIterator<Item = SchemaType>,
        query: &Entity<CodeEditor>,
        variables: &Entity<CodeEditor>,
        search: &Entity<TextInput>,
        on_run: impl Fn(String, String, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            types: types.into_iter().collect(),
            query: query.clone(),
            variables: variables.clone(),
            answer: None,
            search: search.clone(),
            running: false,
            on_run: Rc::new(on_run),
            on_pick: None,
        }
    }

    /// The last answer, as JSON.
    pub fn answer(mut self, json: impl Into<SharedString>) -> Self {
        self.answer = Some(json.into());
        self
    }

    pub fn running(mut self, running: bool) -> Self {
        self.running = running;
        self
    }

    pub fn on_pick(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for GraphQLExplorer {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let theme = cx.theme();
        let nodes = self.types.iter().map(|ty| {
            TreeNode::new(ty.name.clone(), ty.name.clone())
                .note(ty.kind.word())
                .icon(match ty.kind {
                    TypeKind::Object | TypeKind::Input => IconName::Braces,
                    TypeKind::Enum => IconName::ListOrdered,
                    TypeKind::Scalar => IconName::Hash,
                })
                .children(ty.fields.iter().map(|(field, of)| {
                    TreeNode::new(format!("{}.{field}", ty.name), field.clone()).note(of.clone())
                }))
        });
        let mut schema = Tree::new((id.clone(), "schema"), nodes).size_full();
        if let Some(on_pick) = self.on_pick {
            schema = schema.on_activate(move |path, window, cx| {
                log::info!("graphql explorer: pick {path}");
                on_pick(path, window, cx);
            });
        }
        let boxed = |height: gpui::Rems| {
            div()
                .h(height)
                .rounded(theme.radius(Radius::Md))
                .border_1()
                .border_color(theme.colors.border)
                .overflow_hidden()
        };
        let caption = |text: &'static str| {
            div()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(theme.colors.fg_subtle)
                .child(text)
        };
        let tall = theme.list_max_height();
        let on_run = self.on_run;
        let (query, variables) = (self.query.clone(), self.variables.clone());
        let typed = variables.read(cx).text().to_string();
        let unread = (!typed.trim().is_empty())
            .then(|| read_json(&typed).err())
            .flatten();
        let rests = self.running || unread.is_some();
        let answer = match self.answer {
            Some(json) => {
                JsonViewer::new((id.clone(), "answer"), json, &self.search).into_any_element()
            }
            None => EmptyState::new((id.clone(), "none"), IconName::Play, "No answer yet")
                .body("Run the query to see what comes back.")
                .into_any_element(),
        };
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_4()
                    .child(
                        div()
                            .flex_1()
                            .min_w(theme.label_width())
                            .max_w(rems(theme.label_width().0 * 2.0))
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(caption("Schema"))
                            .child(boxed(tall).p_1().child(schema)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(theme.label_width())
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(caption("Query"))
                            .child(boxed(rems(tall.0 * 0.6)).child(self.query))
                            .child(caption("Variables"))
                            .child(boxed(rems(tall.0 * 0.3)).child(self.variables))
                            .children(unread.map(|unread| {
                                InlineMessage::new(Severity::Danger, unread.to_string())
                            }))
                            .child(
                                div().flex().child(
                                    Button::new(
                                        (id, "run"),
                                        if self.running { "Running…" } else { "Run" },
                                    )
                                    .variant(ButtonVariant::Primary)
                                    .disabled(rests)
                                    .on_click(
                                        move |_, window, cx| {
                                            let (query, variables) = (
                                                query.read(cx).text().to_string(),
                                                variables.read(cx).text().to_string(),
                                            );
                                            log::info!(
                                                "graphql explorer: run {} bytes",
                                                query.len()
                                            );
                                            on_run(query, variables, window, cx);
                                        },
                                    ),
                                ),
                            ),
                    ),
            )
            .child(answer)
    }
}
