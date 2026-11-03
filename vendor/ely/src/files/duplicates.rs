use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::*,
};

use super::FileIcon;
use crate::{
    buttons::{Button, ButtonVariant},
    forms::Checkbox,
    theme::{ActiveTheme, ControlSize, TextSize},
    typography::{MiddleEllipsis, format},
};

type OnMark = Rc<dyn Fn(&SharedString, bool, &mut Window, &mut App)>;
type OnRemove = Rc<dyn Fn(&[SharedString], &mut Window, &mut App)>;

/// Copies of one file: its name, its size in bytes, and where each copy lives.
#[derive(Clone, Debug, PartialEq)]
pub struct Duplicates {
    pub name: SharedString,
    pub size: u64,
    pub paths: Vec<SharedString>,
}

/// The bytes the marked copies take.
pub(crate) fn freed(groups: &[Duplicates], marked: &[SharedString]) -> u64 {
    groups
        .iter()
        .map(|group| {
            let copies = group
                .paths
                .iter()
                .filter(|path| marked.contains(path))
                .count();
            group.size * copies as u64
        })
        .sum()
}

/// Files found more than once, a group each: every copy with a mark to remove it; the one copy a group has left unmarked stays, its mark shut. What the marked copies free sits above, beside Remove.
#[derive(IntoElement)]
pub struct DuplicateFinder {
    id: ElementId,
    groups: Vec<Duplicates>,
    marked: Vec<SharedString>,
    on_mark: Option<OnMark>,
    on_remove: Option<OnRemove>,
}

impl DuplicateFinder {
    pub fn new(id: impl Into<ElementId>, groups: impl IntoIterator<Item = Duplicates>) -> Self {
        let groups: Vec<Duplicates> = groups.into_iter().collect();
        for group in &groups {
            assert!(group.paths.len() >= 2, "{} has one copy", group.name);
        }
        let paths: Vec<&SharedString> = groups.iter().flat_map(|group| &group.paths).collect();
        for (ix, path) in paths.iter().enumerate() {
            assert!(!paths[..ix].contains(path), "{path} twice");
        }
        Self {
            id: id.into(),
            groups,
            marked: Vec::new(),
            on_mark: None,
            on_remove: None,
        }
    }

    /// The copies marked to remove, by path; each group keeps one.
    pub fn marked(mut self, paths: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        let mut paths: Vec<SharedString> = paths.into_iter().map(Into::into).collect();
        paths.retain(|path| {
            let listed = self.groups.iter().any(|group| group.paths.contains(path));
            if !listed {
                log::error!("duplicate finder: no copy {path}; its mark is dropped");
            }
            listed
        });
        for group in &self.groups {
            if group.paths.iter().all(|path| paths.contains(path)) {
                log::error!(
                    "duplicate finder: every copy of {} is marked; none of them stays marked",
                    group.name
                );
                paths.retain(|path| !group.paths.contains(path));
            }
        }
        self.marked = paths;
        self
    }

    /// Gets a copy's path and whether it is marked now.
    pub fn on_mark(
        mut self,
        handler: impl Fn(&SharedString, bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_mark = Some(Rc::new(handler));
        self
    }

    /// Gets the marked copies to remove.
    pub fn on_remove(
        mut self,
        handler: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_remove = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for DuplicateFinder {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let small = theme.text_size(TextSize::Sm);
        let freed = freed(&self.groups, &self.marked);
        let marked: Rc<[SharedString]> = self.marked.into();
        let remove = Button::new((self.id.clone(), "remove"), "Remove marked")
            .variant(ButtonVariant::Danger)
            .size(ControlSize::Sm)
            .disabled(marked.is_empty() || self.on_remove.is_none());
        let remove = match self.on_remove {
            Some(on_remove) => {
                let (id, marked) = (self.id.clone(), marked.clone());
                remove.on_click(move |_, window, cx| {
                    log::info!("duplicate finder {id:?}: remove {} copies", marked.len());
                    on_remove(&marked, window, cx)
                })
            }
            None => remove,
        };
        let header = div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap_2()
            .child(
                div()
                    .flex_1()
                    .min_w(theme.label_width())
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.fg)
                            .child(match self.groups.len() {
                                1 => "1 file found more than once".to_string(),
                                count => format!("{count} files found more than once"),
                            }),
                    )
                    .child(
                        div()
                            .text_size(small)
                            .text_color(colors.fg_muted)
                            .child(format!("{} to free", format::file_size(freed, false))),
                    ),
            )
            .child(div().flex_none().child(remove));
        let groups = self.groups.iter().enumerate().map(|(g, group)| {
            let left = group
                .paths
                .iter()
                .filter(|path| !marked.contains(path))
                .count();
            let copies = group.paths.iter().enumerate().map(|(c, path)| {
                let on = marked.contains(path);
                let check = Checkbox::new((self.id.clone(), format!("copy-{g}-{c}")), on)
                    .label(path.clone())
                    .disabled(self.on_mark.is_none() || (!on && left == 1));
                match self.on_mark.clone() {
                    Some(on_mark) => {
                        let (id, path) = (self.id.clone(), path.clone());
                        check.on_change(move |to, window, cx| {
                            log::info!("duplicate finder {id:?}: {path} marked {to}");
                            on_mark(&path, to, window, cx)
                        })
                    }
                    None => check,
                }
            });
            div()
                .flex()
                .flex_col()
                .gap_1p5()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(FileIcon::file(&group.name))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_color(colors.fg)
                                .child(MiddleEllipsis::new(group.name.clone())),
                        )
                        .child(
                            div()
                                .flex_none()
                                .text_size(small)
                                .text_color(colors.fg_muted)
                                .child(format!(
                                    "{} copies · {} each",
                                    group.paths.len(),
                                    format::file_size(group.size, false)
                                )),
                        ),
                )
                .child(div().pl_6().flex().flex_col().gap_1().children(copies))
        });
        div()
            .debug_selector(|| "duplicate-finder".into())
            .flex()
            .flex_col()
            .gap_4()
            .child(header)
            .children(groups)
    }
}

#[cfg(test)]
mod tests {
    use super::{DuplicateFinder, Duplicates};

    #[test]
    fn marks_on_no_copy_or_on_every_copy_are_dropped() {
        let group = |name: &str, paths: [&str; 2]| Duplicates {
            name: name.to_string().into(),
            size: 1,
            paths: paths.map(Into::into).to_vec(),
        };
        let finder = DuplicateFinder::new(
            "dupes",
            [group("a.jpg", ["x", "y"]), group("b.jpg", ["p", "q"])],
        )
        .marked(["x", "y", "p", "gone"]);
        assert_eq!(finder.marked, ["p"]);
    }
}
