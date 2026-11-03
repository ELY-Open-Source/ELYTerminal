use std::rc::Rc;

use gpui::{
    App, ElementId, Entity, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window,
    div, prelude::*,
};

use super::FileIcon;
use crate::{
    forms::{SearchInput, TextInput},
    lists::{ListItem, SelectableList},
    theme::{ActiveTheme, TextSize},
};

type OnFound = Rc<dyn Fn(&FoundFile, &mut Window, &mut App)>;

/// A file a search found: its name and the folder it lives in.
#[derive(Clone, Debug, PartialEq)]
pub struct FoundFile {
    pub name: SharedString,
    pub folder: SharedString,
}

impl FoundFile {
    /// Where it lives: its folder and name.
    fn place(&self) -> SharedString {
        format!("{}/{}", self.folder, self.name).into()
    }
}

/// A search over files: a field the host owns, and what it found, each file with its folder and a count above. The host searches as the text changes; Enter or a double press opens a file.
#[derive(IntoElement)]
pub struct FileSearch {
    id: ElementId,
    query: Entity<TextInput>,
    found: Vec<FoundFile>,
    on_open: Option<OnFound>,
}

impl FileSearch {
    pub fn new(
        id: impl Into<ElementId>,
        query: &Entity<TextInput>,
        found: impl IntoIterator<Item = FoundFile>,
    ) -> Self {
        let found: Vec<FoundFile> = found.into_iter().collect();
        for (ix, file) in found.iter().enumerate() {
            let place = file.place();
            assert!(
                !found[..ix].iter().any(|other| other.place() == place),
                "{place} twice"
            );
        }
        Self {
            id: id.into(),
            query: query.clone(),
            found,
            on_open: None,
        }
    }

    pub fn on_open(
        mut self,
        handler: impl Fn(&FoundFile, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for FileSearch {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let text = self.query.read(cx).text().trim().to_string();
        let theme = cx.theme();
        let quiet = |line: String| {
            div()
                .debug_selector(|| "file-search-note".into())
                .px_3()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(theme.colors.fg_muted)
                .child(line)
        };
        let found: Rc<[FoundFile]> = self.found.into();
        let results = match (text.is_empty(), found.is_empty()) {
            (true, _) => quiet("Type a name to search.".into()).into_any_element(),
            (false, true) => quiet(format!("No files match “{text}”.")).into_any_element(),
            (false, false) => {
                let list = found.iter().enumerate().fold(
                    SelectableList::new((self.id.clone(), "found")),
                    |list, (ix, file)| {
                        let row = ListItem::new(
                            (self.id.clone(), format!("found-{ix}")),
                            file.name.clone(),
                        )
                        .leading(FileIcon::file(&file.name))
                        .description(file.folder.clone());
                        list.row(file.place(), row)
                    },
                );
                let (id, shown, on_open) = (self.id.clone(), found.clone(), self.on_open);
                let count = match found.len() {
                    1 => "1 file".to_string(),
                    count => format!("{count} files"),
                };
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(quiet(count))
                    .child(list.on_activate(move |key, window, cx| {
                        let file = shown
                            .iter()
                            .find(|file| file.place() == *key)
                            .expect("a listed file");
                        log::info!("file search {id:?}: open {}", file.name);
                        if let Some(on_open) = &on_open {
                            on_open(file, window, cx);
                        }
                    }))
                    .into_any_element()
            }
        };
        div()
            .debug_selector(|| "file-search".into())
            .flex()
            .flex_col()
            .gap_3()
            .child(SearchInput::new((self.id.clone(), "field"), &self.query))
            .child(results)
    }
}
