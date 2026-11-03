use std::{path::Path, rc::Rc};

use gpui::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled, Window, div,
};

use super::vscode::{VsCodeTheme, read_vscode_theme};
use crate::{
    buttons::{Button, ButtonVariant},
    data_display::Badge,
    feedback::InlineMessage,
    forms::DropZone,
    primitives::Severity,
    theme::{ActiveTheme, Mode, TextSize},
    typography::Ellipsis,
};

/// What the importer read last: a theme and the file it came from, or why the file did not read.
enum Read {
    Theme(Box<VsCodeTheme>, SharedString),
    Fault(SharedString),
}

/// A file's text, read as a VS Code color theme.
fn read(path: &Path) -> Read {
    let file: SharedString = path
        .file_name()
        .map_or(path.display().to_string(), |name| {
            name.to_string_lossy().to_string()
        })
        .into();
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => return Read::Fault(format!("{file} did not open: {error}").into()),
    };
    match read_vscode_theme(&text) {
        Ok(theme) => Read::Theme(Box::new(theme), file),
        Err(error) => Read::Fault(format!("{file}: {error:#}").into()),
    }
}

pub(super) type OnTheme = Rc<dyn Fn(&VsCodeTheme, &mut Window, &mut App)>;

/// A VS Code color theme taken in, dropped or browsed for: its name and mode, how many colors it names, and Apply, the rest kept from Ely's palette of that mode. A file that does not read says why.
#[derive(IntoElement)]
pub struct ThemeImporter {
    id: ElementId,
    on_apply: OnTheme,
}

impl ThemeImporter {
    pub fn new(
        id: impl Into<ElementId>,
        on_apply: impl Fn(&VsCodeTheme, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            on_apply: Rc::new(on_apply),
        }
    }
}

impl RenderOnce for ThemeImporter {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let on_apply = self.on_apply;
        let last = window.use_keyed_state((id.clone(), "read"), cx, |_, _| None::<Read>);
        let took = last.clone();
        let zone = DropZone::new((id.clone(), "zone"))
            .kinds(&["json"])
            .hint("A VS Code color theme, as .json")
            .on_drop(move |paths, _, cx| {
                let path = paths.first().expect("a drop brings a file");
                let result = read(path);
                if let Read::Fault(why) = &result {
                    log::warn!("theme importer: {why}");
                }
                took.update(cx, |last, cx| {
                    *last = Some(result);
                    cx.notify();
                });
            });
        let result = last.read(cx).as_ref().map(|result| match result {
            Read::Fault(why) => {
                InlineMessage::new(Severity::Danger, why.clone()).into_any_element()
            }
            Read::Theme(read, file) => read_row(read, file, &id, on_apply.clone(), cx),
        });
        div().flex().flex_col().gap_3().child(zone).children(result)
    }
}

/// A theme read: its name and mode, how many colors it names, and Apply.
pub(super) fn read_row(
    read: &VsCodeTheme,
    file: &SharedString,
    id: &ElementId,
    apply: OnTheme,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let named = read.named.len();
    let all = named + read.kept.len();
    let applied = read.clone();
    div()
        .debug_selector(|| "theme-import-read".into())
        .flex()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            div()
                .flex_1()
                .min_w(theme.label_width())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .font_weight(FontWeight::MEDIUM)
                                .child(Ellipsis::new(
                                    read.name.clone().map_or(file.clone(), SharedString::from),
                                )),
                        )
                        .child(
                            div()
                                .flex_none()
                                .debug_selector(|| "theme-import-mode".into())
                                .child(Badge::new(match read.mode {
                                    Mode::Light => "Light",
                                    Mode::Dark => "Dark",
                                })),
                        ),
                )
                .child(
                    div()
                        .text_size(theme.text_size(TextSize::Sm))
                        .text_color(theme.colors.fg_muted)
                        .child(format!(
                            "Names {named} of {all} colors; the rest keep Ely's."
                        )),
                ),
        )
        .child(
            Button::new((id.clone(), "apply"), "Apply")
                .variant(ButtonVariant::Primary)
                .on_click(move |_, window, cx| {
                    log::info!("theme importer: apply");
                    apply(&applied, window, cx)
                }),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{Read, read};

    #[test]
    fn a_file_reads_as_a_theme_or_says_why() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("ely-theme-{}.json", std::process::id()));
        std::fs::write(
            &path,
            r##"{"name": "Mist", "type": "light", "colors": {"editor.background": "#fafafa"}}"##,
        )
        .expect("a written theme");
        let taken = read(&path);
        std::fs::remove_file(&path).expect("the theme removed");
        match taken {
            Read::Theme(theme, file) => {
                assert_eq!(theme.name.as_deref(), Some("Mist"));
                assert!(file.ends_with(".json"), "{file}");
            }
            Read::Fault(why) => panic!("{why}"),
        }
        match read(&dir.join("ely-no-such-theme.json")) {
            Read::Fault(why) => assert!(why.contains("did not open"), "{why}"),
            Read::Theme(..) => panic!("a missing file read"),
        }
    }
}
