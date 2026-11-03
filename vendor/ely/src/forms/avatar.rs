use std::{path::PathBuf, rc::Rc};

use gpui::{
    App, ElementId, ExternalPaths, InteractiveElement, IntoElement, ParentElement,
    PathPromptOptions, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div,
};

use super::{files::hovering, path::choose};
use crate::{
    data_display::Avatar,
    primitives::{FocusRing, Icon, IconName, tab_stop},
    theme::{ActiveTheme, IconSize, Radius},
};

/// File kinds an avatar takes.
const PICTURES: [&str; 5] = ["png", "jpg", "jpeg", "gif", "webp"];

/// The one picture a drop or a pick brings, or why it is refused.
pub(crate) fn picture(paths: &[PathBuf]) -> Result<PathBuf, &'static str> {
    let [path] = paths else {
        return Err(if paths.is_empty() {
            "nothing came"
        } else {
            "one picture at a time"
        });
    };
    if !path.is_file() {
        return Err("files only, not folders");
    }
    let kind = path
        .extension()
        .and_then(|kind| kind.to_str())
        .map(str::to_lowercase);
    if !kind.is_some_and(|kind| PICTURES.contains(&kind.as_str())) {
        return Err("pictures only: png, jpg, gif or webp");
    }
    Ok(path.clone())
}

type OnPicture = Rc<dyn Fn(PathBuf, &mut Window, &mut App)>;

/// An avatar you replace: a hover shows a camera, a press or Enter opens the picture dialog, and a picture dropped on it works too. `on_change` gets its path.
#[derive(IntoElement)]
pub struct AvatarUpload {
    id: ElementId,
    avatar: Avatar,
    on_change: Option<OnPicture>,
}

impl AvatarUpload {
    pub fn new(id: impl Into<ElementId>, avatar: Avatar) -> Self {
        Self {
            id: id.into(),
            avatar,
            on_change: None,
        }
    }

    pub fn on_change(mut self, handler: impl Fn(PathBuf, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for AvatarUpload {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus = tab_stop((self.id.clone(), "focus").into(), true, window, cx);
        let what = SharedString::from(format!("avatar upload {:?}", self.id));
        let take: OnPicture = {
            let (what, on_change) = (what.clone(), self.on_change);
            Rc::new(move |path, window, cx| {
                log::info!("{what}: took {}", path.display());
                if let Some(on_change) = &on_change {
                    on_change(path, window, cx);
                }
            })
        };
        let theme = cx.theme();
        let corner = |element: gpui::Div, square: bool| {
            if square {
                element.rounded(theme.radius(Radius::Md))
            } else {
                element.rounded_full()
            }
        };
        let square = self.avatar.is_square();
        let group = SharedString::from(format!("{:?}", self.id));
        let (pick, drop_in, what_drop) = (take.clone(), take, what.clone());
        corner(div(), square)
            .id(self.id)
            .group(group.clone())
            .relative()
            .flex_none()
            .border_1()
            .border_color(gpui::transparent_black())
            .cursor_pointer()
            .track_focus(&focus)
            .focus_ring(cx)
            .child(self.avatar)
            .child(
                corner(div(), square)
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(theme.colors.media_backdrop.opacity(0.5))
                    .opacity(0.0)
                    .group_hover(group, |style| style.opacity(1.0))
                    .child(
                        Icon::new(IconName::Camera)
                            .size(IconSize::Md)
                            .color(theme.colors.on_media),
                    ),
            )
            .on_click(move |_, window, cx| {
                let pick = pick.clone();
                let what = what.clone();
                choose(
                    what.clone(),
                    PathPromptOptions {
                        files: true,
                        directories: false,
                        multiple: false,
                        prompt: Some("Choose".into()),
                    },
                    window,
                    cx,
                    move |paths, window, cx| match picture(&paths) {
                        Ok(path) => pick(path, window, cx),
                        Err(reason) => log::warn!("{what}: refused a pick: {reason}"),
                    },
                );
            })
            .drag_over::<ExternalPaths>(|style, paths, _, cx| {
                hovering(style, picture(paths.paths()).is_ok(), cx)
            })
            .on_drop(
                move |paths: &ExternalPaths, window, cx| match picture(paths.paths()) {
                    Ok(path) => drop_in(path, window, cx),
                    Err(reason) => log::info!("{what_drop}: refused a drop: {reason}"),
                },
            )
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::picture;

    #[test]
    fn an_avatar_takes_one_picture_file() {
        let dir = std::env::temp_dir().join("ely-avatar-picture");
        fs::create_dir_all(&dir).expect("a temp dir");
        let (face, notes) = (dir.join("face.PNG"), dir.join("notes.txt"));
        fs::write(&face, b"png").expect("a picture");
        fs::write(&notes, b"text").expect("a note");
        assert_eq!(picture(std::slice::from_ref(&face)), Ok(face.clone()));
        assert_eq!(
            picture(&[notes]),
            Err("pictures only: png, jpg, gif or webp")
        );
        assert_eq!(picture(&[face.clone(), face]), Err("one picture at a time"));
        assert_eq!(picture(&[dir]), Err("files only, not folders"));
        assert_eq!(picture(&[]), Err("nothing came"));
    }
}
