use std::{path::PathBuf, rc::Rc};

use gpui::{
    App, ElementId, ImageAssetLoader, ImageSource, IntoElement, ParentElement, RenderOnce,
    Resource, SharedString, Styled, Window, div, prelude::*,
};

use super::{Crop, ImageCropper, cropper::OnCrop};
use crate::{
    buttons::{Button, ButtonVariant},
    forms::{DropZone, PICTURES, Run, browse},
    motion::Spinner,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, TextSize},
    typography::Ellipsis,
};

type OnPath = Rc<dyn Fn(PathBuf, &mut Window, &mut App)>;

/// What the upload has said about a file: its fitted crop to the host, its failure to the log.
#[derive(Default)]
struct Told {
    fitted: bool,
    failed: bool,
}

/// A picture to send: a drop zone until one is chosen, then the picture to crop, its name, Replace and Remove. Only pictures are taken; the picture's shape comes from the file. The host keeps the file and its crop; with none yet, the upload reports the largest centered box of its aspect once the picture opens.
#[derive(IntoElement)]
pub struct ImageUpload {
    id: ElementId,
    picked: Option<PathBuf>,
    crop: Option<Crop>,
    aspect: Option<f32>,
    on_pick: Option<OnPath>,
    on_crop: Option<OnCrop>,
    on_remove: Option<Run>,
}

impl ImageUpload {
    /// `picked` is the chosen file, none before one is; `crop` is none until the upload first reports one.
    pub fn new(id: impl Into<ElementId>, picked: Option<PathBuf>, crop: Option<Crop>) -> Self {
        Self {
            id: id.into(),
            picked,
            crop,
            aspect: None,
            on_pick: None,
            on_crop: None,
            on_remove: None,
        }
    }

    /// The crop's width over its height, in pixels.
    pub fn aspect(mut self, aspect: Option<f32>) -> Self {
        self.aspect = aspect;
        self
    }

    /// Gets a picture dropped or chosen, to keep in place of any before it.
    pub fn on_pick(mut self, handler: impl Fn(PathBuf, &mut Window, &mut App) + 'static) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }

    pub fn on_crop(mut self, handler: impl Fn(Crop, &mut Window, &mut App) + 'static) -> Self {
        self.on_crop = Some(Rc::new(handler));
        self
    }

    pub fn on_remove(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_remove = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ImageUpload {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let pick = self.on_pick.clone();
        let Some(path) = self.picked else {
            return DropZone::new((self.id.clone(), "drop"))
                .hint("PNG, JPEG, GIF or WebP")
                .kinds(&PICTURES)
                .when_some(pick, |zone, pick| {
                    zone.on_drop(move |mut paths, window, cx| pick(paths.remove(0), window, cx))
                })
                .into_any_element();
        };
        let decoded =
            window.use_asset::<ImageAssetLoader>(&Resource::Path(path.clone().into()), cx);
        let told = window.use_keyed_state(
            (self.id.clone(), path.to_string_lossy().into_owned()),
            cx,
            |_, _| Told::default(),
        );
        let name = path
            .file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy()
            .into_owned();
        let opened = match &decoded {
            Some(Ok(image)) => {
                let size = image.size(0);
                let ratio = size.width.0 as f32 / size.height.0 as f32;
                let fitted = self
                    .aspect
                    .map_or(Crop::WHOLE, |aspect| Crop::WHOLE.fitted(aspect, ratio));
                Some((ratio, fitted))
            }
            _ => None,
        };
        if let (Some((_, fitted)), None, Some(on_crop)) = (opened, self.crop, self.on_crop.clone())
            && !told.read(cx).fitted
        {
            log::info!("image upload: {name} opened, crop fitted to {fitted:?}");
            told.update(cx, |told, _| told.fitted = true);
            window.defer(cx, move |window, cx| on_crop(fitted, window, cx));
        }
        if let Some(Err(error)) = &decoded
            && !told.read(cx).failed
        {
            log::error!("image upload: {name} can't be opened: {error}");
            told.update(cx, |told, _| told.failed = true);
        }
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let replace = pick.map(|pick| {
            let what = SharedString::from(format!("image upload {:?}", self.id));
            Button::new((self.id.clone(), "replace"), "Replace")
                .size(ControlSize::Sm)
                .on_click(move |_, window, cx| {
                    let pick = pick.clone();
                    browse(
                        what.clone(),
                        false,
                        &PICTURES,
                        window,
                        cx,
                        move |mut paths, window, cx| pick(paths.remove(0), window, cx),
                    );
                })
        });
        let remove = self.on_remove.map(|remove| {
            Button::new((self.id.clone(), "remove"), "Remove")
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .on_click(move |_, window, cx| {
                    log::info!("image upload: removed");
                    remove(window, cx)
                })
        });
        let body = match decoded {
            Some(Ok(image)) => {
                let (ratio, fitted) = opened.expect("an opened picture has its shape");
                let cropper = ImageCropper::new(
                    (self.id.clone(), "crop"),
                    ImageSource::Render(image),
                    ratio,
                    self.crop.unwrap_or(fitted),
                )
                .aspect(self.aspect);
                match self.on_crop {
                    Some(on_crop) => {
                        cropper.on_change(move |crop, window, cx| on_crop(crop, window, cx))
                    }
                    None => cropper,
                }
                .into_any_element()
            }
            Some(Err(_)) => div()
                .flex()
                .items_center()
                .gap_2()
                .py_6()
                .justify_center()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(colors.danger)
                .child(
                    Icon::new(IconName::ImageOff)
                        .size(IconSize::Md)
                        .color(colors.danger),
                )
                .child("This picture can't be opened")
                .into_any_element(),
            None => div()
                .flex()
                .justify_center()
                .py_6()
                .child(Spinner::new((self.id.clone(), "loading")).size(IconSize::Md))
                .into_any_element(),
        };
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(body)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w(theme.label_width())
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(colors.fg_muted)
                            .child(Ellipsis::new(name)),
                    )
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .gap_2()
                            .children(replace)
                            .children(remove),
                    ),
            )
            .into_any_element()
    }
}
