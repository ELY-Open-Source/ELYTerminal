use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use base64::Engine as _;
use gpui::{
    AnyElement, App, Asset, ElementId, Hsla, Image, ImageAssetLoader, ImageFormat,
    InteractiveElement, IntoElement, ParentElement, RenderImage, RenderOnce, Resource,
    SMOOTH_SVG_SCALE_FACTOR, SharedString, Styled, Window, div, img, px,
};

use crate::{
    i18n,
    primitives::{Icon, IconName, hold},
    theme::{ActiveTheme, IconSize, Radius},
};

/// Where a picture's bytes come from.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Origin {
    File(PathBuf),
    /// Bytes inline in the address, and their type.
    Data(ImageFormat, Arc<[u8]>),
    Web,
    /// A relative path with no folder to read it in.
    Loose,
    /// An address that names no picture, and why.
    Bad(SharedString),
}

/// `%XX` decoded; a broken escape stays as written.
fn decoded(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let (mut out, mut at) = (Vec::with_capacity(bytes.len()), 0);
    while at < bytes.len() {
        let pair = bytes.get(at + 1..at + 3);
        let hex =
            pair.and_then(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok());
        match (bytes[at], hex) {
            (b'%', Some(byte)) => (out.push(byte), at += 3),
            (byte, _) => (out.push(byte), at += 1),
        };
    }
    out
}

/// A path written in a link, `%XX` decoded.
fn path(written: &str) -> Result<PathBuf, SharedString> {
    String::from_utf8(decoded(written))
        .map(PathBuf::from)
        .map_err(|_| "the path is not UTF-8".into())
}

/// Where `url` points, read from the folder `base`.
pub(crate) fn origin(url: &str, base: Option<&Path>) -> Origin {
    let url = url.trim();
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || url.starts_with("//") {
        return Origin::Web;
    }
    if lower.starts_with("data:") {
        return inline(url);
    }
    if lower.starts_with("file:") {
        return file(&url[5..]);
    }
    let scheme = url.split_once(':').is_some_and(|(scheme, _)| {
        scheme.len() > 1
            && scheme
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || "+-.".contains(ch))
    });
    if scheme {
        return Origin::Bad(format!("{url} is no picture address").into());
    }
    let written = url.split(['?', '#']).next().unwrap_or_default();
    match path(written) {
        Ok(path) if path.is_absolute() => Origin::File(path),
        Ok(path) => base.map_or(Origin::Loose, |base| Origin::File(base.join(path))),
        Err(why) => Origin::Bad(why),
    }
}

/// A `file:` address after its scheme: host, path, no query.
fn file(rest: &str) -> Origin {
    let rest = rest.split(['?', '#']).next().unwrap_or_default();
    let (host, written) = match rest.strip_prefix("//") {
        Some(after) => after.split_at(after.find('/').unwrap_or(after.len())),
        None => ("", rest),
    };
    let local = host.is_empty() || host.eq_ignore_ascii_case("localhost");
    let written = match (local, written.as_bytes()) {
        // Windows writes file:///C:/…
        (true, [b'/', _, b':', ..]) if cfg!(windows) => written[1..].to_string(),
        (true, _) => written.to_string(),
        (false, _) if cfg!(windows) => format!("//{host}{written}"),
        (false, _) => return Origin::Bad(format!("files on {host} are not read").into()),
    };
    path(&written).map_or_else(Origin::Bad, Origin::File)
}

/// A `data:` address: base64 bytes of an image type.
fn inline(url: &str) -> Origin {
    let Some((head, payload)) = url[5..].split_once(',') else {
        return Origin::Bad("the data address has no comma".into());
    };
    let Some(mime) = head.strip_suffix(";base64") else {
        return Origin::Bad("only base64 data addresses draw".into());
    };
    let Some(format) = ImageFormat::from_mime_type(&mime.to_ascii_lowercase()) else {
        return Origin::Bad(format!("{mime} is no image type").into());
    };
    match base64::engine::general_purpose::STANDARD.decode(payload.trim()) {
        Ok(bytes) => Origin::Data(format, bytes.into()),
        Err(error) => Origin::Bad(format!("{error}").into()),
    }
}

/// Inline bytes decoded as gpui decodes files.
enum Unpacker {}

impl Asset for Unpacker {
    type Source = (ImageFormat, Arc<[u8]>);
    type Output = Result<Arc<RenderImage>, SharedString>;

    fn load(
        (format, bytes): Self::Source,
        cx: &mut App,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let renderer = cx.svg_renderer();
        async move {
            Image::from_bytes(format, bytes.to_vec())
                .to_image_data(renderer)
                .map_err(|error| format!("{error:#}").into())
        }
    }
}

/// The picture at `url`, natural size up to the column.
#[derive(IntoElement)]
pub(crate) struct Picture {
    pub id: ElementId,
    pub alt: String,
    pub url: String,
    /// The folder relative addresses start from.
    pub base: Option<Arc<Path>>,
}

impl RenderOnce for Picture {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        picture(self, window, cx)
    }
}

fn picture(shown: Picture, window: &mut Window, cx: &mut App) -> AnyElement {
    let Picture { id, alt, url, base } = shown;
    let (alt, url) = (alt.as_str(), url.as_str());
    let colors = cx.theme().colors.clone();
    let (loaded, svg) = match origin(url, base.as_deref()) {
        Origin::File(path) => {
            let svg = path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"));
            let resource = Resource::Path(path.into());
            let image = |loaded: &Result<Arc<RenderImage>, _>| loaded.as_ref().ok().cloned();
            let loaded = hold::<ImageAssetLoader>(id, &resource, image, window, cx)
                .map(|loaded| loaded.map_err(|error| SharedString::from(error.to_string())));
            (loaded, svg)
        }
        Origin::Data(format, bytes) => {
            let svg = format == ImageFormat::Svg;
            let image = |loaded: &Result<Arc<RenderImage>, _>| loaded.as_ref().ok().cloned();
            (
                hold::<Unpacker>(id, &(format, bytes), image, window, cx),
                svg,
            )
        }
        Origin::Web => {
            let said = i18n::text(cx, "markdown.picture.web", &[]);
            return note(alt, said, colors.fg_subtle, cx);
        }
        Origin::Loose => {
            let said = i18n::text(cx, "markdown.picture.folder", &[]);
            return note(alt, said, colors.fg_subtle, cx);
        }
        Origin::Bad(why) => return note(alt, why, colors.danger, cx),
    };
    match loaded {
        Some(Ok(image)) => {
            let natural = image.size(0);
            // The picture's own height follows its ratio.
            let smooth = if svg { SMOOTH_SVG_SCALE_FACTOR } else { 1.0 };
            let width = i32::from(natural.width) as f32 / smooth;
            div()
                .debug_selector(|| "markdown-picture".into())
                .w(px(width))
                .max_w_full()
                .child(img(image).size_full())
                .into_any_element()
        }
        Some(Err(why)) => {
            log::warn!("markdown: a picture did not load");
            note(alt, why, colors.danger, cx)
        }
        None => note(alt, url.to_string().into(), colors.fg_subtle, cx),
    }
}

/// A picture not drawn: its alt text and why.
fn note(alt: &str, said: SharedString, tone: Hsla, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let colors = &theme.colors;
    div()
        .flex()
        .items_center()
        .gap_2()
        .p_3()
        .rounded(theme.radius(Radius::Md))
        .border_1()
        .border_color(colors.border)
        .text_color(colors.fg_muted)
        .child(
            Icon::new(IconName::Image)
                .size(IconSize::Sm)
                .color(colors.fg_subtle),
        )
        .child(div().flex_1().child(alt.to_string()))
        .child(div().text_color(tone).child(said))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_resolve_against_the_folder() {
        let base = Path::new("/notes");
        assert_eq!(
            origin("assets/a%20b.png?x#y", Some(base)),
            Origin::File(base.join("assets/a b.png"))
        );
        assert_eq!(origin("assets/a.png", None), Origin::Loose);
        assert_eq!(origin("https://e.x/a.png", Some(base)), Origin::Web);
        assert_eq!(origin("//e.x/a.png", Some(base)), Origin::Web);
        assert!(matches!(origin("mailto:a@b", Some(base)), Origin::Bad(_)));
        let absolute = std::env::current_dir().expect("a dir").join("a.png");
        assert_eq!(
            origin(&absolute.display().to_string(), Some(base)),
            Origin::File(absolute)
        );
    }

    #[test]
    fn file_addresses_decode_without_query_or_fragment() {
        for url in [
            "file:///tmp/a%23b.png",
            "file://localhost/tmp/a%23b.png#x",
            "FILE:/tmp/a%23b.png?y",
        ] {
            assert_eq!(
                origin(url, None),
                Origin::File(PathBuf::from("/tmp/a#b.png")),
                "{url}"
            );
        }
        if cfg!(windows) {
            assert_eq!(
                origin("file:///C:/notes/a.png#preview", None),
                Origin::File(PathBuf::from("C:/notes/a.png"))
            );
        }
    }

    #[test]
    fn a_file_on_another_host_is_its_share_or_refused() {
        let found = origin("file://server/share/a.png", Some(Path::new("/notes")));
        match cfg!(windows) {
            true => assert_eq!(found, Origin::File(PathBuf::from("//server/share/a.png"))),
            false => assert!(matches!(found, Origin::Bad(_)), "{found:?}"),
        }
    }

    #[test]
    fn data_addresses_carry_their_bytes() {
        assert_eq!(
            origin("data:image/png;base64,cG5n", None),
            Origin::Data(ImageFormat::Png, Arc::from(&b"png"[..]))
        );
        assert!(matches!(origin("data:image/png,raw", None), Origin::Bad(_)));
        assert!(matches!(
            origin("data:text/plain;base64,eA==", None),
            Origin::Bad(_)
        ));
        assert!(matches!(
            origin("data:image/png;base64,***", None),
            Origin::Bad(_)
        ));
    }
}
