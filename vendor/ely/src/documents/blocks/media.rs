use std::{path::Path, sync::Arc};

use gpui::{
    AnyElement, AppContext as _, Bounds, Context, EmptyView, EntityId, Image, ImageFormat,
    ImageSource, InteractiveElement, IntoElement, ParentElement, Pixels, SharedUri,
    StatefulInteractiveElement, Styled, Window, div, img, prelude::*,
};
use regex::Regex;

use super::{Align, Block, BlockEditor, BlockKind, Media, view::Look};
use crate::{
    buttons::{Button, ButtonVariant, IconButton},
    primitives::{Icon, IconName, Measure},
    theme::{ControlSize, IconSize},
};

/// Narrowest a picture gets, as a share of its column.
const NARROWEST: f32 = 0.2;

/// A picture's edge on its way, and whose.
struct Resize {
    owner: EntityId,
    key: u64,
}

/// A web address loads over the network; anything else is a file.
pub(crate) fn source(text: &str) -> ImageSource {
    if text.starts_with("http://") || text.starts_with("https://") {
        SharedUri::from(text.to_string()).into()
    } else {
        Path::new(text).into()
    }
}

/// The share of `column` a picture set by `align` spans when its edge is at `x`.
pub(crate) fn share(align: Align, column: Bounds<Pixels>, x: Pixels) -> f32 {
    let width = f32::from(column.size.width).max(1.0);
    let reach = match align {
        Align::Left => f32::from(x - column.left()),
        Align::Right => f32::from(column.right() - x),
        Align::Center => 2.0 * f32::from(x - column.center().x).abs(),
    };
    (reach / width).clamp(NARROWEST, 1.0)
}

/// A media block's media; a block turned into something else since its controls painted has none.
fn media_of(kind: &mut BlockKind) -> Option<&mut Media> {
    match kind {
        BlockKind::Image(media) | BlockKind::Video(media) | BlockKind::Embed(media) => Some(media),
        other => {
            log::error!("block editor: a {} block holds no media now", other.label());
            None
        }
    }
}

impl BlockEditor {
    /// Sets a picture's width while its edge is dragged; the drag's start records undo.
    pub(super) fn resize(&mut self, key: u64, width: f32, first: bool, cx: &mut Context<Self>) {
        let Some(ix) = self.index(key) else {
            return;
        };
        let mut kind = self.blocks[ix].kind.clone();
        let Some(media) = media_of(&mut kind) else {
            return;
        };
        media.width = width;
        if first {
            self.before_change(cx);
        }
        self.blocks[ix].kind = kind;
        self.after_change(cx);
    }

    pub(crate) fn align(&mut self, key: u64, align: Align, cx: &mut Context<Self>) {
        let Some(ix) = self.index(key) else {
            return;
        };
        let mut kind = self.blocks[ix].kind.clone();
        let Some(media) = media_of(&mut kind) else {
            return;
        };
        media.align = align;
        self.set_kind(key, kind, cx);
    }
}

/// A picture sized as a share of its column, set left, center or right, with an edge to drag and a caption below; a clip or a page shows as a card to open.
pub(super) fn media(
    editor: &BlockEditor,
    block: &Block,
    look: &Look,
    window: &mut Window,
    cx: &mut Context<BlockEditor>,
) -> AnyElement {
    let (key, id) = (block.key, BlockEditor::id(cx));
    let colors = look.colors.clone();
    let caption = div()
        .text_size(look.small)
        .text_color(colors.fg_muted)
        .child(editor.field_view(block, 0, window, cx));
    let (media, icon) = match &block.kind {
        BlockKind::Image(media) => (media.clone(), None),
        BlockKind::Video(media) => (media.clone(), Some(IconName::Video)),
        BlockKind::Embed(media) => (media.clone(), Some(IconName::Globe)),
        other => unreachable!("{} is no media", other.label()),
    };
    if let Some(icon) = icon {
        let address = media.source.clone();
        return div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .p_3()
                    .rounded(look.radius)
                    .border_1()
                    .border_color(colors.border)
                    .child(Icon::new(icon).size(IconSize::Md).color(colors.fg_muted))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(colors.fg_muted)
                            .child(media.source.clone()),
                    )
                    .child(
                        Button::new((id.clone(), format!("open-{key}")), "Open")
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Sm)
                            .on_click(move |_, _, cx| cx.open_url(&address)),
                    ),
            )
            .child(caption)
            .into_any_element();
    }
    let column = window.use_keyed_state((id.clone(), format!("column-{key}")), cx, |_, _| {
        Bounds::<Pixels>::default()
    });
    let bounds = *column.read(cx);
    let (align, width) = (media.align, media.width);
    let owner = cx.entity_id();
    let edge = |side: &'static str, cx: &mut Context<BlockEditor>| {
        let entity = cx.entity();
        div()
            .id((id.clone(), format!("edge-{key}-{side}")))
            .absolute()
            .top_0()
            .bottom_0()
            .map(|edge| {
                if side == "left" {
                    edge.left_0()
                } else {
                    edge.right_0()
                }
            })
            .w_2()
            .cursor_col_resize()
            .on_drag(Resize { owner, key }, move |_, _, _, cx| {
                entity.update(cx, |editor, cx| editor.resize(key, width, true, cx));
                log::info!("image {key}: resize");
                cx.new(|_| EmptyView)
            })
    };
    let aligned = |align_to: Align, icon: IconName, cx: &mut Context<BlockEditor>| {
        IconButton::new((id.clone(), format!("align-{key}-{align_to:?}")), icon)
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Sm)
            .tooltip(format!("Align {align_to:?}").to_lowercase())
            .on_click(cx.listener(move |editor, _, _, cx| editor.align(key, align_to, cx)))
    };
    let tools = div()
        .absolute()
        .top_1()
        .right_1()
        .flex()
        .gap_0p5()
        .p_0p5()
        .rounded(look.radius)
        .bg(colors.overlay)
        .opacity(0.0)
        .group_hover(format!("picture-{key}"), |tools| tools.opacity(1.0))
        .child(aligned(Align::Left, IconName::AlignLeft, cx))
        .child(aligned(Align::Center, IconName::AlignCenter, cx))
        .child(aligned(Align::Right, IconName::AlignRight, cx));
    let picture = div()
        .id((id.clone(), format!("picture-{key}")))
        .group(format!("picture-{key}"))
        .relative()
        .w(bounds.size.width * width)
        .child(
            img(source(&media.source))
                .id((id.clone(), format!("img-{key}")))
                .w(bounds.size.width * width)
                .rounded(look.radius),
        )
        .child(edge("left", cx))
        .child(edge("right", cx))
        .child(tools);
    let entity = cx.entity();
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .id((id.clone(), format!("column-{key}")))
                .w_full()
                .on_drag_move(move |event: &gpui::DragMoveEvent<Resize>, _, cx| {
                    if event.drag(cx).owner != owner || event.drag(cx).key != key {
                        return;
                    }
                    let width = share(align, event.bounds, event.event.position.x);
                    entity.update(cx, |editor, cx| editor.resize(key, width, false, cx));
                })
                .child(
                    Measure::new(
                        (id.clone(), format!("measure-{key}")),
                        move |measured, _, cx| {
                            column.update(cx, |column, cx| {
                                *column = measured;
                                cx.notify();
                            })
                        },
                    )
                    .w_full()
                    .flex()
                    .map(|row| match align {
                        Align::Left => row.justify_start(),
                        Align::Center => row.justify_center(),
                        Align::Right => row.justify_end(),
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .w(bounds.size.width * width)
                            .child(picture)
                            .child(caption),
                    ),
                ),
        )
        .into_any_element()
}

/// A diagram the host drew, and the width its SVG asks for.
#[derive(Clone)]
struct Drawn {
    image: Arc<Image>,
    width: Option<f32>,
}

/// The width an SVG's root asks for, in pixels, when it gives one.
pub(crate) fn svg_width(svg: &str) -> Option<f32> {
    let root = Regex::new(r#"<svg\b[^>]*?\swidth=['"]([0-9.]+)(px)?['"]"#)
        .expect("the SVG width pattern compiles");
    root.captures(svg)?.get(1)?.as_str().parse().ok()
}

/// A diagram's source above the picture the host draws from it.
pub(super) fn diagram(
    editor: &BlockEditor,
    block: &Block,
    look: &Look,
    window: &mut Window,
    cx: &mut Context<BlockEditor>,
) -> AnyElement {
    let (key, id) = (block.key, BlockEditor::id(cx));
    let text = block.fields[0].read(cx).text().to_string();
    let drawn = window.use_keyed_state((id.clone(), format!("drawn-{key}")), cx, |_, _| {
        None::<(String, Result<Drawn, String>)>
    });
    let stale = drawn
        .read(cx)
        .as_ref()
        .is_none_or(|(source, _)| *source != text);
    if stale && let Some(render) = &editor.diagram {
        let picture = render(&text).map(|svg| Drawn {
            width: svg_width(&svg),
            image: Arc::new(Image::from_bytes(ImageFormat::Svg, svg.into_bytes())),
        });
        log::info!(
            "diagram {key}: drawn, {}",
            if picture.is_ok() {
                "fine"
            } else {
                "with an error"
            }
        );
        drawn.update(cx, |drawn, _| *drawn = Some((text.clone(), picture)));
    }
    let colors = look.colors.clone();
    let shown: AnyElement = match (editor.diagram.is_some(), drawn.read(cx).clone()) {
        _ if text.trim().is_empty() => div().into_any_element(),
        (false, _) => div()
            .text_color(colors.fg_subtle)
            .child("Diagrams draw once the app gives a renderer")
            .into_any_element(),
        (true, Some((_, Ok(drawn)))) => {
            let column =
                window.use_keyed_state((id.clone(), format!("diagram-width-{key}")), cx, |_, _| {
                    Pixels::ZERO
                });
            let width = drawn.width.map_or(*column.read(cx), |natural| {
                Pixels::from(natural).min(*column.read(cx))
            });
            Measure::new(
                (id.clone(), format!("diagram-measure-{key}")),
                move |bounds, _, cx| {
                    column.update(cx, |column, cx| {
                        *column = bounds.size.width;
                        cx.notify();
                    })
                },
            )
            .w_full()
            .flex()
            .justify_center()
            .child(
                img(drawn.image)
                    .id((id.clone(), format!("diagram-{key}")))
                    .w(width),
            )
            .into_any_element()
        }
        (true, Some((_, Err(error)))) => div()
            .text_color(colors.danger)
            .child(error)
            .into_any_element(),
        (true, None) => unreachable!("a renderer draws before this shows"),
    };
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .rounded(look.radius)
        .border_1()
        .border_color(colors.border)
        .child(
            div()
                .font_family(look.mono.clone())
                .text_size(look.small)
                .text_color(colors.fg_muted)
                .child(editor.field_view(block, 0, window, cx)),
        )
        .child(shown)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, point, px, size};

    use super::*;

    #[test]
    fn an_svg_gives_its_width_when_it_has_one() {
        assert_eq!(
            svg_width("<svg xmlns='x' width='504' height='44'>"),
            Some(504.0)
        );
        assert_eq!(svg_width("<svg width=\"120px\">"), Some(120.0));
        assert_eq!(
            svg_width("<svg viewBox='0 0 10 10'><rect width='5'/></svg>"),
            None
        );
    }

    #[test]
    fn an_edge_sets_its_share_by_how_the_picture_sits() {
        let column = Bounds::new(point(px(100.0), px(0.0)), size(px(400.0), px(10.0)));
        assert_eq!(share(Align::Left, column, px(300.0)), 0.5);
        assert_eq!(share(Align::Right, column, px(300.0)), 0.5);
        assert_eq!(share(Align::Center, column, px(400.0)), 0.5);
        assert_eq!(
            share(Align::Center, column, px(300.0)),
            NARROWEST,
            "a picture keeps some width"
        );
    }
}
