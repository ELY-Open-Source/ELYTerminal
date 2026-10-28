//! Ely's Wayland popup, open while the main window is active.

use std::time::Duration;

use gpui::{App, Decorations, Pixels, Point, Window, WindowHandle, point, px};

use super::{Edge, Shown, close, left, open_pet, pane, present};
use crate::{settings::SettingsStore, workspace::Workspace};

/// How often she checks her anchor against the main window.
const CHECK: Duration = Duration::from_millis(500);

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

/// Where she stands over a Wayland window: the edge, and her anchor counted from the pane's top left, as gpui's popups count.
pub fn spot(window: &Window, at: f32) -> (Edge, Point<Pixels>) {
    let pane = pane(window);
    let edge = edge(window);
    let y = match edge {
        Edge::Top => px(0.),
        Edge::Bottom => pane.size.height - px(1.),
    };
    (edge, point(px(left(pane.size.width, at)), y))
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
    let spot = main.update(cx, |_, window, _| spot(window, at))?;
    let Some(pet) = *shown.borrow() else {
        return Ok(());
    };
    if pet
        .read(cx)
        .is_ok_and(|pet| (pet.edge, pet.anchor) == (spot.0, Some(spot.1)))
    {
        return Ok(());
    }
    close(
        pet,
        "to stand where the window's size or state, or a drag, puts her",
        cx,
    );
    *shown.borrow_mut() = open_pet(main, cx);
    Ok(())
}
