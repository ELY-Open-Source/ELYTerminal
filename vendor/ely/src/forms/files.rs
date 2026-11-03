use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use gpui::{
    App, Div, ElementId, ExternalPaths, InteractiveElement, IntoElement, ParentElement,
    PathPromptOptions, RenderOnce, SharedString, Stateful, StatefulInteractiveElement,
    StyleRefinement, Styled, Window, div, prelude::*,
};

use super::{
    path::choose,
    select::{field_button, field_text},
};
use crate::{
    buttons::{Button, ButtonVariant, IconButton},
    primitives::{Icon, IconName, tab_stop},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
};

type OnPaths = Rc<dyn Fn(Vec<PathBuf>, &mut Window, &mut App)>;

/// The files a drop brings, or why it is refused.
pub(crate) fn dropped(paths: &[PathBuf], multiple: bool) -> Result<Vec<PathBuf>, &'static str> {
    if paths.is_empty() {
        return Err("nothing was dropped");
    }
    if !multiple && paths.len() > 1 {
        return Err("one file at a time");
    }
    if paths.iter().any(|path| !path.is_file()) {
        return Err("files only, not folders");
    }
    Ok(paths.to_vec())
}

/// Files a picture may be, by extension.
pub const PICTURES: [&str; 5] = ["png", "jpg", "jpeg", "gif", "webp"];

/// The files a drop or a dialog brings when each is of one of `kinds`, by extension in any case, or why they are refused; no kinds takes any file.
pub(crate) fn taken(
    paths: &[PathBuf],
    multiple: bool,
    kinds: &[&str],
) -> Result<Vec<PathBuf>, &'static str> {
    let paths = dropped(paths, multiple)?;
    let fits = |path: &PathBuf| {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                kinds
                    .iter()
                    .any(|kind| kind.eq_ignore_ascii_case(extension))
            })
    };
    if !kinds.is_empty() && !paths.iter().all(fits) {
        return Err("not a kind this takes");
    }
    Ok(paths)
}

/// Opens the file dialog; what it brings goes to `then` once `taken` lets it through.
pub(crate) fn browse(
    what: SharedString,
    multiple: bool,
    kinds: &'static [&'static str],
    window: &mut Window,
    cx: &mut App,
    then: impl Fn(Vec<PathBuf>, &mut Window, &mut App) + 'static,
) {
    let refused = what.clone();
    choose(
        what,
        files(multiple),
        window,
        cx,
        move |paths, window, cx| match taken(&paths, multiple, kinds) {
            Ok(paths) => then(paths, window, cx),
            Err(reason) => log::info!("{refused}: refused a choice: {reason}"),
        },
    );
}

/// A drop target while files hover: focus-tinted when it takes them, danger-tinted when not.
pub(super) fn hovering(style: StyleRefinement, takes: bool, cx: &App) -> StyleRefinement {
    let colors = &cx.theme().colors;
    if takes {
        style.border_color(colors.focus).bg(colors.selection)
    } else {
        style.border_color(colors.danger).bg(colors.danger_subtle)
    }
}

/// Wires a drop target: hover tint, the drop gate, and the handler.
fn target(
    element: Stateful<Div>,
    what: SharedString,
    (multiple, kinds): (bool, &'static [&'static str]),
    take: OnPaths,
) -> Stateful<Div> {
    element
        .drag_over::<ExternalPaths>(move |style, paths, _, cx| {
            hovering(style, taken(paths.paths(), multiple, kinds).is_ok(), cx)
        })
        .on_drop(move |paths: &ExternalPaths, window, cx| {
            match taken(paths.paths(), multiple, kinds) {
                Ok(paths) => take(paths, window, cx),
                Err(reason) => log::info!("{what}: refused a drop: {reason}"),
            }
        })
}

fn files(multiple: bool) -> PathPromptOptions {
    PathPromptOptions {
        files: true,
        directories: false,
        multiple,
        prompt: Some(SharedString::from("Choose")),
    }
}

/// The file's name, or the whole path for a root such as `C:\`.
fn file_name(path: &Path) -> String {
    match path.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => path.display().to_string(),
    }
}

/// Chosen files in a field. A click opens the dialog; files dropped on it count too.
#[derive(IntoElement)]
pub struct FileInput {
    id: ElementId,
    paths: Vec<PathBuf>,
    multiple: bool,
    on_change: Option<OnPaths>,
}

impl FileInput {
    pub fn new(id: impl Into<ElementId>, paths: impl IntoIterator<Item = PathBuf>) -> Self {
        Self {
            id: id.into(),
            paths: paths.into_iter().collect(),
            multiple: false,
            on_change: None,
        }
    }

    /// Takes several files at once.
    pub fn multiple(mut self) -> Self {
        self.multiple = true;
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(Vec<PathBuf>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for FileInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        assert!(
            self.multiple || self.paths.len() <= 1,
            "file input {:?} holds {} files but takes one",
            self.id,
            self.paths.len()
        );
        let focus = tab_stop((self.id.clone(), "focus").into(), true, window, cx);
        let what = SharedString::from(format!("file input {:?}", self.id));
        let set: OnPaths = {
            let (what, on_change) = (what.clone(), self.on_change);
            Rc::new(move |paths, window, cx| {
                log::info!("{what}: {} files", paths.len());
                if let Some(on_change) = &on_change {
                    on_change(paths, window, cx);
                }
            })
        };
        let text = match self.paths.as_slice() {
            [] => None,
            [one] => Some(file_name(one)),
            [first, rest @ ..] => Some(format!("{} +{}", file_name(first), rest.len())),
        };
        let placeholder = if self.multiple {
            "Choose files…"
        } else {
            "Choose a file…"
        };
        let (multiple, browse, clear) = (self.multiple, set.clone(), set.clone());
        let browse_what = what.clone();
        let subtle = cx.theme().colors.fg_subtle;
        let field = field_button(self.id.clone(), &focus, ControlSize::Md, false, window, cx)
            .cursor_pointer()
            .on_click(move |_, window, cx| {
                let browse = browse.clone();
                choose(
                    browse_what.clone(),
                    files(multiple),
                    window,
                    cx,
                    move |paths, window, cx| browse(paths, window, cx),
                );
            })
            .child(Icon::new(IconName::File).size(IconSize::Sm).color(subtle))
            .child(field_text(
                text.map(SharedString::from),
                placeholder.into(),
                false,
                cx,
            ))
            .when(!self.paths.is_empty(), |field| {
                field.child(
                    IconButton::new((self.id.clone(), "clear"), IconName::X)
                        .size(ControlSize::Sm)
                        .variant(ButtonVariant::Ghost)
                        .tooltip("Clear")
                        .on_click(move |_, window, cx| {
                            cx.stop_propagation();
                            clear(Vec::new(), window, cx);
                        }),
                )
            });
        target(field, what, (multiple, &[]), set)
    }
}

/// A dashed area that takes dropped files; Browse opens the dialog instead.
#[derive(IntoElement)]
pub struct DropZone {
    id: ElementId,
    hint: Option<SharedString>,
    multiple: bool,
    kinds: &'static [&'static str],
    on_drop: Option<OnPaths>,
}

impl DropZone {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            hint: None,
            multiple: false,
            kinds: &[],
            on_drop: None,
        }
    }

    /// Takes only files of these kinds, by extension, such as `PICTURES`.
    pub fn kinds(mut self, kinds: &'static [&'static str]) -> Self {
        self.kinds = kinds;
        self
    }

    /// A line under the prompt, such as the kinds and sizes taken.
    pub fn hint(mut self, text: impl Into<SharedString>) -> Self {
        self.hint = Some(text.into());
        self
    }

    /// Takes several files at once.
    pub fn multiple(mut self) -> Self {
        self.multiple = true;
        self
    }

    pub fn on_drop(
        mut self,
        handler: impl Fn(Vec<PathBuf>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_drop = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for DropZone {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let what = SharedString::from(format!("drop zone {:?}", self.id));
        let take: OnPaths = {
            let (what, on_drop) = (what.clone(), self.on_drop);
            Rc::new(move |paths, window, cx| {
                log::info!("{what}: took {} files", paths.len());
                if let Some(on_drop) = &on_drop {
                    on_drop(paths, window, cx);
                }
            })
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let (multiple, kinds, chosen, browse_what) =
            (self.multiple, self.kinds, take.clone(), what.clone());
        let zone = div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .px_6()
            .py_8()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_dashed()
            .border_color(colors.border_strong)
            .child(
                Icon::new(IconName::Upload)
                    .size(IconSize::Lg)
                    .color(colors.fg_muted),
            )
            .child(div().text_color(colors.fg).child(if multiple {
                "Drop files here"
            } else {
                "Drop a file here"
            }))
            .when_some(self.hint, |zone, hint| {
                zone.child(
                    div()
                        .text_size(theme.text_size(TextSize::Sm))
                        .text_color(colors.fg_subtle)
                        .child(hint),
                )
            })
            .child(
                div().pt_2().child(
                    Button::new((self.id, "browse"), "Browse…")
                        .size(ControlSize::Sm)
                        .on_click(move |_, window, cx| {
                            let take = chosen.clone();
                            browse(
                                browse_what.clone(),
                                multiple,
                                kinds,
                                window,
                                cx,
                                move |paths, window, cx| take(paths, window, cx),
                            );
                        }),
                ),
            );
        target(zone, what, (multiple, kinds), take)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{PICTURES, dropped, file_name, taken};

    #[test]
    fn a_root_shows_its_whole_path() {
        assert_eq!(file_name(std::path::Path::new("/")), "/");
        assert_eq!(file_name(std::path::Path::new("/notes/plan.md")), "plan.md");
    }

    #[test]
    fn drops_take_files_and_refuse_folders_extras_and_nothing() {
        let dir = std::env::temp_dir().join(format!("ely-drop-{}", std::process::id()));
        fs::create_dir_all(dir.join("folder")).unwrap();
        let (one, two) = (dir.join("one.txt"), dir.join("two.txt"));
        fs::write(&one, "1").unwrap();
        fs::write(&two, "2").unwrap();
        let both = [one.clone(), two.clone()];
        assert_eq!(dropped(&both, true), Ok(both.to_vec()));
        assert_eq!(dropped(&both, false), Err("one file at a time"));
        assert_eq!(
            dropped(std::slice::from_ref(&one), false),
            Ok(vec![one.clone()])
        );
        assert_eq!(
            dropped(&[one, dir.join("folder")], true),
            Err("files only, not folders")
        );
        assert_eq!(dropped(&[], true), Err("nothing was dropped"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_zone_of_pictures_takes_pictures_in_any_case_and_refuses_the_rest() {
        let dir = std::env::temp_dir().join(format!("ely-kinds-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let [photo, loud, note] = ["dunes.jpg", "STAIR.PNG", "notes.txt"].map(|name| {
            let path = dir.join(name);
            fs::write(&path, "x").unwrap();
            path
        });
        let both = [photo.clone(), loud.clone()];
        assert_eq!(taken(&both, true, &PICTURES), Ok(both.to_vec()));
        let mixed = [photo, note.clone()];
        assert_eq!(taken(&mixed, true, &PICTURES), Err("not a kind this takes"));
        let lone = std::slice::from_ref(&note);
        assert_eq!(
            taken(lone, false, &[]),
            Ok(lone.to_vec()),
            "no kinds takes any file"
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
