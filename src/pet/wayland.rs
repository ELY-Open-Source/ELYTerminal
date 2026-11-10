//! Ely's Wayland popup, open while the main window is active.

use std::time::Duration;

use gpui::{App, Decorations, Pixels, Point, Window, WindowHandle, point, px};

use super::{Edge, HEAD, ROOM, Shown, close, left, open_pet, pane, present};
use crate::{settings::SettingsStore, workspace::Workspace};

/// How often she checks her anchor against the main window.
const CHECK: Duration = Duration::from_millis(500);

/// KWin snaps a moving window to a popup's edge inside its snap zone, 100 points at most.
const SKIRT: f32 = 100.0;

/// Whether `points` cover whole device pixels at `scale`, which Wayland counts in 120ths.
fn whole(points: u32, scale: f32) -> bool {
    let parts = (scale * 120.0).round() as u32;
    (points * parts).is_multiple_of(120)
}

/// The fewest whole points, `least` or more, that cover whole device pixels: a scale counts in 120ths, so 120 points always do.
fn up(least: f32, scale: f32) -> f32 {
    let first = least.ceil() as u32;
    let found = (first..first + 120).find(|points| whole(*points, scale));
    found.expect("120 points in a row hold a whole one") as f32
}

/// The most whole points, `most` or fewer, that cover whole device pixels.
fn down(most: f32, scale: f32) -> f32 {
    let found = (0..=most.floor() as u32)
        .rev()
        .find(|points| whole(*points, scale));
    found.expect("no points cover no pixels") as f32
}

/// Her popup's rise above the edge and its clear skirt below it, each whole device pixels, so a compositor that rounds each window's place apart never shifts her; the skirt keeps a top popup's edges out of KWin's snap zone.
pub fn reach(edge: Edge, scale: f32) -> (f32, f32) {
    let skirt = match edge {
        Edge::Top => up(SKIRT, scale),
        Edge::Bottom => 0.0,
    };
    (up(HEAD + ROOM, scale), skirt)
}

/// Follows `main`'s activation, and reopens her where her anchor moves.
pub fn follow(main: WindowHandle<Workspace>, shown: Shown, cx: &mut App) {
    let activation = {
        let shown = shown.clone();
        main.update(cx, |_, window, cx| {
            cx.observe_window_activation(window, move |_, _, cx| {
                let shown = shown.clone();
                // gpui opens no window inside another window's update.
                cx.defer(move |cx| present(main, &shown, cx));
            })
            .detach();
        })
    };
    if let Err(error) = activation {
        log::error!("pet: the main window's activation is not followed: {error:#}");
    }
    // gpui hears a change of state alone as no change of bounds, and sends an unseen popup no frames.
    cx.spawn(async move |cx| {
        while cx.update(|cx| reanchor(main, &shown, cx)).is_ok() {
            cx.background_executor().timer(CHECK).await;
        }
    })
    .detach();
}

/// Where she stands over a Wayland window: the edge, and her anchor, her popup's bottom left, counted from the pane's top left, as gpui's popups count, on whole device pixels.
pub fn spot(window: &Window, at: f32) -> (Edge, Point<Pixels>) {
    let pane = pane(window);
    let edge = edge(window);
    let scale = window.scale_factor();
    let y = match edge {
        Edge::Top => px(reach(edge, scale).1),
        Edge::Bottom => pane.size.height - px(1.),
    };
    let x = down(left(pane.size.width, at), scale);
    (edge, point(px(x), y))
}

/// The edge she stands on over a Wayland window, whose place no client knows: the bottom while it is full screen, maximized or tiled at its top.
fn edge(window: &Window) -> Edge {
    let tiled = matches!(window.window_decorations(), Decorations::Client { tiling } if tiling.top);
    if tiled || window.is_fullscreen() || window.is_maximized() {
        Edge::Bottom
    } else {
        Edge::Top
    }
}

/// Reopens her once `main`'s size or state moves her anchor.
fn reanchor(main: WindowHandle<Workspace>, shown: &Shown, cx: &mut App) -> anyhow::Result<()> {
    let at = SettingsStore::get(cx).ely_left;
    let (spot, rise) = main.update(cx, |_, window, _| {
        let spot = spot(window, at);
        (spot, reach(spot.0, window.scale_factor()).0)
    })?;
    let Some(pet) = *shown.borrow() else {
        return Ok(());
    };
    if pet
        .read(cx)
        .is_ok_and(|pet| (pet.edge, pet.anchor, pet.rise) == (spot.0, Some(spot.1), rise))
    {
        return Ok(());
    }
    close(
        pet,
        "to stand where the window's size, state or scale, or a drag, puts her",
        cx,
    );
    *shown.borrow_mut() = open_pet(main, cx);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn her_reach_covers_whole_device_pixels_at_every_scale() {
        for (scale, rise) in [
            (1.0, 124.),
            (1.1, 130.),
            (1.25, 124.),
            (1.5, 124.),
            (1.75, 124.),
            (2.0, 124.),
        ] {
            let (up, skirt) = reach(Edge::Top, scale);
            assert_eq!(up, rise, "at {scale}");
            assert!(
                skirt >= SKIRT && whole(skirt as u32, scale),
                "{skirt} at {scale}"
            );
            assert_eq!(reach(Edge::Bottom, scale), (rise, 0.0));
        }
        // 121 parts of 120: only a multiple of 120 points is whole.
        assert_eq!(reach(Edge::Top, 121.0 / 120.0), (240., 120.));
        assert_eq!(
            (down(141.0, 1.25), down(141.0, 1.0), down(3.0, 1.1)),
            (140., 141., 0.)
        );
    }
}
