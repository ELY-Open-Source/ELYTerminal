use std::rc::Rc;

use gpui::{
    App, Div, ElementId, ImageSource, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, img, prelude::*,
};

use crate::{
    forms::{Choice, OnValue, Select},
    primitives::{Icon, IconName, checked_ratio, framed},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::Ellipsis,
};

/// A name's chip over a frame's lower corner, light on a dark wash.
pub(crate) fn name_chip(cx: &App) -> Div {
    let theme = cx.theme();
    let colors = &theme.colors;
    div()
        .absolute()
        .left_2()
        .bottom_2()
        .max_w(gpui::relative(0.8))
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_0p5()
        .rounded(theme.radius(Radius::Sm))
        .bg(colors.media_backdrop.alpha(0.6))
        .text_color(colors.on_media)
        .text_size(theme.text_size(TextSize::Xs))
}

/// What a camera sees, in its own shape, from frames the host hands in; the host mirrors them if it will. With no frame it shows the camera off.
#[derive(IntoElement)]
pub struct CameraPreview {
    id: ElementId,
    ratio: f32,
    frame: Option<ImageSource>,
    name: Option<SharedString>,
}

impl CameraPreview {
    /// `ratio` is the camera's width over its height.
    pub fn new(id: impl Into<ElementId>, ratio: f32) -> Self {
        Self {
            id: id.into(),
            ratio: checked_ratio(ratio),
            frame: None,
            name: None,
        }
    }

    /// The frame to show now.
    pub fn frame(mut self, source: impl Into<ImageSource>) -> Self {
        self.frame = Some(source.into());
        self
    }

    /// The camera's name, over the frame's lower corner.
    pub fn name(mut self, name: impl Into<SharedString>) -> Self {
        self.name = Some(name.into());
        self
    }
}

impl RenderOnce for CameraPreview {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let round = theme.radius(Radius::Lg);
        let off = self.frame.is_none();
        let frame = self.frame.map(|frame| {
            img(frame)
                .id((self.id.clone(), "frame"))
                .size_full()
                .rounded(round)
        });
        let name = self.name.map(|name| {
            name_chip(cx)
                .debug_selector(|| "camera-name".into())
                .child(Ellipsis::new(name))
        });
        framed(self.ratio, cx)
            .debug_selector(|| "camera-preview".into())
            .rounded(round)
            .children(frame)
            .when(off, |stage| {
                stage.child(
                    div()
                        .debug_selector(|| "camera-off".into())
                        .absolute()
                        .inset_0()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap_2()
                        .text_size(theme.text_size(TextSize::Sm))
                        .text_color(colors.fg_muted)
                        .child(
                            Icon::new(IconName::VideoOff)
                                .size(IconSize::Lg)
                                .color(colors.fg_subtle),
                        )
                        .child("Camera off"),
                )
            })
            .children(name)
    }
}

/// What a device is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceKind {
    Microphone,
    Camera,
    Speaker,
}

impl DeviceKind {
    fn icon(self) -> IconName {
        match self {
            Self::Microphone => IconName::Mic,
            Self::Camera => IconName::Video,
            Self::Speaker => IconName::Volume2,
        }
    }

    fn noun(self) -> &'static str {
        match self {
            Self::Microphone => "microphone",
            Self::Camera => "camera",
            Self::Speaker => "speaker",
        }
    }
}

/// A device to pick: its key and its name.
#[derive(Clone, Debug, PartialEq)]
pub struct Device {
    pub key: SharedString,
    pub name: SharedString,
}

/// A choice of microphone, camera or speaker, each with its kind's icon. With none attached it says so and stays shut.
#[derive(IntoElement)]
pub struct DeviceSelector {
    id: ElementId,
    kind: DeviceKind,
    devices: Vec<Device>,
    chosen: Option<SharedString>,
    on_change: Option<OnValue>,
}

impl DeviceSelector {
    pub fn new(
        id: impl Into<ElementId>,
        kind: DeviceKind,
        devices: impl IntoIterator<Item = Device>,
    ) -> Self {
        let devices: Vec<Device> = devices.into_iter().collect();
        for (ix, device) in devices.iter().enumerate() {
            assert!(
                !devices[..ix].iter().any(|other| other.key == device.key),
                "device {} twice",
                device.key
            );
        }
        Self {
            id: id.into(),
            kind,
            devices,
            chosen: None,
            on_change: None,
        }
    }

    /// The device in use, by key.
    pub fn chosen(mut self, key: impl Into<SharedString>) -> Self {
        let key = key.into();
        if !self.devices.iter().any(|device| device.key == key) {
            log::error!("device selector: no device {key}; none chosen");
            self.chosen = None;
            return self;
        }
        self.chosen = Some(key);
        self
    }

    /// Gets the key of the device to use.
    pub fn on_change(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for DeviceSelector {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let (kind, empty) = (self.kind, self.devices.is_empty());
        let choices = self
            .devices
            .into_iter()
            .map(|device| Choice::new(device.key, device.name).icon(kind.icon()));
        let placeholder = match empty {
            true => format!("No {} found", kind.noun()),
            false => format!("Choose a {}", kind.noun()),
        };
        let select = Select::new(self.id, choices)
            .placeholder(placeholder)
            .disabled(empty || self.on_change.is_none());
        let select = match self.chosen {
            Some(key) => select.selected(key),
            None => select,
        };
        match self.on_change {
            Some(change) => select.on_change(move |key, window, cx| {
                log::info!("device selector: {} {key}", kind.noun());
                change(key, window, cx)
            }),
            None => select,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Device, DeviceKind, DeviceSelector};

    #[test]
    fn a_device_gone_from_the_list_leaves_none_chosen() {
        let device = Device {
            key: "a".into(),
            name: "Device a".into(),
        };
        let selector = DeviceSelector::new("mics", DeviceKind::Microphone, [device]);
        assert_eq!(selector.chosen("a").chosen("gone").chosen, None);
    }
}
