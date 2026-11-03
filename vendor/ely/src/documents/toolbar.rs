use gpui::{App, ElementId, Entity, IntoElement, ParentElement, RenderOnce, Window};

use super::format::{Format, active, format};
use crate::{
    buttons::{ToggleButton, ToggleItem, shortcut_text},
    forms::{Run, TextInput},
    primitives::IconName,
    shell::{Toolbar, ToolbarGroup, ToolbarSeparator},
    theme::{ActiveTheme, ControlSize, Platform},
};

/// A tool: the format it applies, its icon, its name and its key.
type Tool = (Format, IconName, &'static str, Option<&'static str>);

/// The inline styles, which the floating toolbar shows too.
pub(crate) const INLINE: &[Tool] = &[
    (Format::Bold, IconName::Bold, "Bold", Some("secondary-b")),
    (
        Format::Italic,
        IconName::Italic,
        "Italic",
        Some("secondary-i"),
    ),
    (
        Format::Strike,
        IconName::Strikethrough,
        "Strikethrough",
        None,
    ),
    (Format::Code, IconName::Code, "Code", None),
    (Format::Link, IconName::Link, "Link", Some("secondary-k")),
];

const HEADINGS: &[Tool] = &[
    (Format::Heading(1), IconName::Heading1, "Heading 1", None),
    (Format::Heading(2), IconName::Heading2, "Heading 2", None),
    (Format::Heading(3), IconName::Heading3, "Heading 3", None),
];

const BLOCKS: &[Tool] = &[
    (Format::Quote, IconName::Quote, "Quote", None),
    (Format::Bullet, IconName::List, "Bulleted list", None),
    (
        Format::Numbered,
        IconName::ListOrdered,
        "Numbered list",
        None,
    ),
    (Format::Task, IconName::ListTodo, "To-do list", None),
];

/// One tool as a toggle: pressed while `on`; a press applies its format, or opens the link editor.
pub(crate) fn tool(
    id: &ElementId,
    field: &Entity<TextInput>,
    (format_of, icon, name, key): Tool,
    on: bool,
    on_link: Option<Run>,
    platform: Platform,
) -> ToggleButton {
    let tip = match key {
        Some(key) => format!("{name}  {}", shortcut_text(key, platform)),
        None => name.to_string(),
    };
    let field = field.clone();
    ToggleButton::new(
        (id.clone(), name),
        ToggleItem::new(name).icon(icon).tooltip(tip),
        on,
    )
    .size(ControlSize::Sm)
    .on_toggle(move |_, window, cx| match (&on_link, format_of) {
        (Some(open), Format::Link) => open(window, cx),
        _ => format(&field, format_of, cx),
    })
}

/// The formats a markdown field takes, in a row above it: inline styles, headings, quote and lists. A tool shows pressed while the selection holds its format; a press applies it and leaves focus in the field.
#[derive(IntoElement)]
pub struct FixedFormatToolbar {
    id: ElementId,
    field: Entity<TextInput>,
    on_link: Option<Run>,
}

impl FixedFormatToolbar {
    pub fn new(id: impl Into<ElementId>, field: &Entity<TextInput>) -> Self {
        Self {
            id: id.into(),
            field: field.clone(),
            on_link: None,
        }
    }

    /// The link tool opens the owner's link editor instead of writing `[text]()`.
    pub fn on_link(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_link = Some(std::rc::Rc::new(handler));
        self
    }
}

impl RenderOnce for FixedFormatToolbar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let input = self.field.read(cx);
        let pressed = active(input.text(), input.selection());
        let platform = cx.theme().platform;
        let group = |tools: &[Tool]| {
            ToolbarGroup::new().children(tools.iter().map(|tool_of| {
                let on = pressed.contains(&tool_of.0);
                tool(
                    &self.id,
                    &self.field,
                    *tool_of,
                    on,
                    self.on_link.clone(),
                    platform,
                )
            }))
        };
        Toolbar::new()
            .child(group(INLINE))
            .child(ToolbarSeparator)
            .child(group(HEADINGS))
            .child(ToolbarSeparator)
            .child(group(BLOCKS))
    }
}
