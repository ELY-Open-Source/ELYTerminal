use std::{path::PathBuf, rc::Rc};

use gpui::{
    App, ElementId, Entity, IntoElement, PathPromptOptions, RenderOnce, SharedString, Window,
};

use super::{Input, TextInput};
use crate::{
    buttons::{Button, ButtonVariant},
    i18n,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize},
};

/// Opens the system's file dialog; `then` gets the chosen paths back in the window.
pub(crate) fn choose(
    what: SharedString,
    options: PathPromptOptions,
    window: &mut Window,
    cx: &mut App,
    then: impl FnOnce(Vec<PathBuf>, &mut Window, &mut App) + 'static,
) {
    let chosen = cx.prompt_for_paths(options);
    log::info!("{what}: dialog opened");
    window
        .spawn(cx, async move |cx| {
            let paths = match chosen.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) => {
                    log::info!("{what}: dialog cancelled");
                    return;
                }
                Ok(Err(error)) => {
                    log::error!("{what}: dialog failed: {error:#}");
                    return;
                }
                Err(_) => {
                    log::error!("{what}: dialog closed without an answer");
                    return;
                }
            };
            if paths.is_empty() {
                log::error!("{what}: dialog chose nothing");
                return;
            }
            log::info!("{what}: chose {} paths", paths.len());
            if let Err(error) = cx.update(|window, cx| then(paths, window, cx)) {
                log::error!("{what}: window closed first: {error:#}");
            }
        })
        .detach();
}

type OnChoose = Rc<dyn Fn(PathBuf, &mut Window, &mut App)>;

/// A path field with Browse, which opens the system's file dialog.
#[derive(IntoElement)]
pub struct PathInput {
    id: ElementId,
    state: Entity<TextInput>,
    directories: bool,
    on_choose: Option<OnChoose>,
}

impl PathInput {
    pub fn new(id: impl Into<ElementId>, state: &Entity<TextInput>) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            directories: false,
            on_choose: None,
        }
    }

    /// Hands the chosen path whole to `handler`, not the field.
    pub fn on_choose(mut self, handler: impl Fn(PathBuf, &mut Window, &mut App) + 'static) -> Self {
        self.on_choose = Some(Rc::new(handler));
        self
    }

    /// Chooses folders instead of files.
    pub fn directories(mut self) -> Self {
        self.directories = true;
        self
    }
}

impl RenderOnce for PathInput {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let subtle = cx.theme().colors.fg_subtle;
        let (state, directories, on_choose) =
            (self.state.clone(), self.directories, self.on_choose);
        let icon = if directories {
            IconName::Folder
        } else {
            IconName::File
        };
        Input::new(&self.state)
            .prefix(Icon::new(icon).size(IconSize::Sm).color(subtle))
            .suffix(
                Button::new(self.id, i18n::text(cx, "input.browse", &[]))
                    .size(ControlSize::Sm)
                    .variant(ButtonVariant::Ghost)
                    .on_click(move |_, window, cx| {
                        let (state, on_choose) = (state.clone(), on_choose.clone());
                        let options = PathPromptOptions {
                            files: !directories,
                            directories,
                            multiple: false,
                            prompt: Some(i18n::text(cx, "input.choose", &[])),
                        };
                        choose(
                            "path input".into(),
                            options,
                            window,
                            cx,
                            move |mut paths, window, cx| {
                                picked(paths.swap_remove(0), &state, on_choose, window, cx)
                            },
                        );
                    }),
            )
    }
}

/// `on_choose` takes the path whole; the field takes UTF-8 only.
fn picked(
    path: PathBuf,
    state: &Entity<TextInput>,
    on_choose: Option<OnChoose>,
    window: &mut Window,
    cx: &mut App,
) {
    if let Some(handler) = on_choose {
        return handler(path, window, cx);
    }
    match path.to_str() {
        Some(text) => state.update(cx, |input, cx| input.set_text(text, cx)),
        None => log::error!(
            "path input: {} is not UTF-8, so the field keeps its text",
            path.display()
        ),
    }
}
