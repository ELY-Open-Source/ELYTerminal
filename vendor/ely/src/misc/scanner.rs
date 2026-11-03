use std::{rc::Rc, sync::Arc};

use gpui::{
    App, ElementId, ImageId, IntoElement, ParentElement, RenderImage, RenderOnce, SharedString,
    Styled, Task, Window, div, prelude::*, relative,
};

use crate::{
    media::CameraPreview,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::literal,
};

type OnScan = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// The text of the first QR code that decodes in a frame of gpui's BGRA pixels.
pub(crate) fn read(frame: &RenderImage) -> Option<String> {
    let Some(pixels) = frame.as_bytes(0) else {
        log::error!("qr code scanner: a frame with no pixels");
        return None;
    };
    let size = frame.size(0);
    let (width, height) = (size.width.0 as usize, size.height.0 as usize);
    let mut image = rqrr::PreparedImage::prepare_from_greyscale(width, height, |x, y| {
        let at = (y * width + x) * 4;
        let [blue, green, red] = [pixels[at], pixels[at + 1], pixels[at + 2]].map(u32::from);
        ((red * 299 + green * 587 + blue * 114) / 1000) as u8
    });
    image
        .detect_grids()
        .into_iter()
        .find_map(|grid| grid.decode().ok().map(|(_, text)| text))
}

/// The scanner's own: the frame last read, the text last found, the read under way, and the owner's handler.
#[derive(Default)]
struct Scan {
    seen: Option<ImageId>,
    found: Option<SharedString>,
    reading: Option<Task<()>>,
    on_scan: Option<OnScan>,
}

/// A camera's view with a square to aim at, reading QR codes off the main thread from the frames the host hands in; frames that come while one is read are passed over. Each code goes to the owner once, when it differs from the last, and shows under the view.
#[derive(IntoElement)]
pub struct QrCodeScanner {
    id: ElementId,
    ratio: f32,
    frame: Option<Arc<RenderImage>>,
    on_scan: OnScan,
}

impl QrCodeScanner {
    /// `ratio` is the camera's width over its height.
    pub fn new(
        id: impl Into<ElementId>,
        ratio: f32,
        on_scan: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            ratio,
            frame: None,
            on_scan: Rc::new(on_scan),
        }
    }

    /// The frame to show and read now, in gpui's BGRA pixels.
    pub fn frame(mut self, frame: Arc<RenderImage>) -> Self {
        self.frame = Some(frame);
        self
    }
}

impl RenderOnce for QrCodeScanner {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let on_scan = self.on_scan;
        let scan = window.use_keyed_state((id.clone(), "scan"), cx, |_, _| Scan::default());
        scan.update(cx, |scan, _| scan.on_scan = Some(on_scan));
        let fresh = self.frame.clone().filter(|frame| {
            let scan = scan.read(cx);
            scan.reading.is_none() && scan.seen != Some(frame.id)
        });
        if let Some(frame) = fresh {
            let (weak, seen) = (scan.downgrade(), frame.id);
            let task = window.spawn(cx, async move |cx| {
                let text = cx
                    .background_executor()
                    .spawn(async move { read(&frame) })
                    .await;
                let _ = cx.update(|window, cx| {
                    let Ok(Some((text, on_scan))) = weak.update(cx, |scan, cx| {
                        scan.reading = None;
                        cx.notify();
                        let text = SharedString::from(text?);
                        (scan.found.as_ref() != Some(&text)).then(|| {
                            scan.found = Some(text.clone());
                            (text, scan.on_scan.clone())
                        })
                    }) else {
                        return;
                    };
                    log::info!("qr code scanner: read {text}");
                    if let Some(on_scan) = on_scan {
                        on_scan(&text, window, cx);
                    }
                });
            });
            scan.update(cx, |scan, _| {
                scan.seen = Some(seen);
                scan.reading = Some(task);
            });
        }
        let theme = cx.theme();
        let colors = &theme.colors;
        let found = scan.read(cx).found.clone();
        let live = self.frame.is_some();
        let mut camera = CameraPreview::new((id, "camera"), self.ratio);
        if let Some(frame) = self.frame {
            camera = camera.frame(frame);
        }
        let mut aim = div()
            .h(relative(0.62))
            .border_2()
            .border_color(colors.on_media.alpha(0.9))
            .rounded(theme.radius(Radius::Md));
        aim.style().aspect_ratio = Some(1.0);
        let status = match found {
            Some(text) => div()
                .flex()
                .items_start()
                .gap_2()
                .child(
                    Icon::new(IconName::CircleCheck)
                        .size(IconSize::Sm)
                        .color(colors.success),
                )
                .child(
                    literal(div())
                        .flex_1()
                        .min_w_0()
                        .debug_selector(|| format!("scanned-{text}"))
                        .text_color(colors.fg)
                        .child(text),
                ),
            None => div()
                .text_color(colors.fg_muted)
                .child("Point the camera at a QR code"),
        };
        div()
            .flex()
            .flex_col()
            .gap_2()
            .text_size(theme.text_size(TextSize::Sm))
            .child(div().relative().child(camera).when(live, |view| {
                view.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(aim),
                )
            }))
            .child(status)
    }
}

/// A frame of gpui's BGRA pixels showing `text` as a QR code, four pixels a module, inside the quiet margin.
#[cfg(test)]
pub(crate) fn frame_of(text: &str) -> Arc<RenderImage> {
    let code = qrcode::QrCode::new(text).expect("the text fits a QR code");
    let (width, scale, quiet) = (code.width(), 4, 4);
    let side = ((width + quiet * 2) * scale) as u32;
    let mut pixels = image::RgbaImage::from_pixel(side, side, image::Rgba([255; 4]));
    for (ix, color) in code.to_colors().into_iter().enumerate() {
        if color == qrcode::Color::Dark {
            let (x, y) = ((ix % width + quiet) * scale, (ix / width + quiet) * scale);
            for (dx, dy) in (0..scale).flat_map(|dx| (0..scale).map(move |dy| (dx, dy))) {
                pixels.put_pixel(
                    (x + dx) as u32,
                    (y + dy) as u32,
                    image::Rgba([0, 0, 0, 255]),
                );
            }
        }
    }
    Arc::new(RenderImage::new(vec![image::Frame::new(pixels)]))
}

#[cfg(test)]
mod tests {
    use gpui::RenderImage;

    use super::{frame_of, read};

    #[test]
    fn a_picture_with_no_frame_reads_nothing() {
        assert_eq!(read(&RenderImage::new(Vec::new())), None);
    }

    #[test]
    fn a_frame_reads_as_the_code_it_shows() {
        assert_eq!(
            read(&frame_of("https://example.com/ely")).as_deref(),
            Some("https://example.com/ely")
        );
        let blank = image::RgbaImage::from_pixel(64, 64, image::Rgba([255; 4]));
        assert_eq!(
            read(&RenderImage::new(vec![image::Frame::new(blank)])),
            None
        );
    }
}
