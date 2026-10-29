//! The picture under the terminal, as the settings name it.

use std::{path::Path, sync::Arc};

use gpui::{
    AnyElement, Context, ImageAssetLoader, IntoElement, ObjectFit, Resource, Styled, StyledImage,
    Window, img,
};

use super::view::TerminalView;
use crate::settings::Settings;

impl TerminalView {
    /// The picture at the background's opacity; a failed load logs once.
    pub(super) fn backdrop(
        &mut self,
        settings: &Settings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if settings.background_image.is_empty() {
            return None;
        }
        let path: Arc<Path> = Path::new(&settings.background_image).into();
        let source = Resource::Path(path.clone());
        match window.use_asset::<ImageAssetLoader>(&source, cx)? {
            Ok(image) => Some(
                img(image)
                    .absolute()
                    .size_full()
                    .object_fit(ObjectFit::Cover)
                    .opacity(settings.background_opacity)
                    .into_any_element(),
            ),
            Err(error) => {
                if self.backdrop_failed.as_deref() != Some(&*path) {
                    log::error!(
                        "terminal {}: background image {}: {error}",
                        self.pty.pid,
                        path.display()
                    );
                    self.backdrop_failed = Some(path.to_path_buf());
                }
                None
            }
        }
    }
}
