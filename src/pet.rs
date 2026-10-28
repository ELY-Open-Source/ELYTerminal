//! Ely, the icon's girl, peeking over the main window's top edge from a window of her own, or up from its bottom edge where no room lies above.

mod look;
#[cfg(target_os = "macos")]
mod macos;
mod rig;
#[cfg(test)]
mod tests;
mod wayland;
#[cfg(windows)]
mod windows;
#[cfg(target_os = "linux")]
mod x11;

use std::{cell::RefCell, rc::Rc, time::Instant};

use gpui::{
    App, AppContext as _, Bounds, Context, DispatchPhase, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point,
    Render, Styled, Subscription, Window, WindowBackgroundAppearance, WindowBounds, WindowHandle,
    WindowKind, WindowOptions, canvas, div, point, popup, px, size,
};

#[cfg(target_os = "macos")]
use self::macos as platform;
#[cfg(windows)]
use self::windows as platform;
#[cfg(target_os = "linux")]
use self::x11 as platform;
use self::{
    look::{Act, EASE, Mood, Pose, blend, mood, pose},
    rig::Rig,
};
use crate::{settings::SettingsStore, workspace::Workspace};

/// Her head's box, in points, and the room above it her hops take.
const WIDTH: f32 = 112.0;
const HEAD: f32 = 107.4;
const ROOM: f32 = 16.0;
/// The pane edge her bottom stands on: the top, or the bottom, inside the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Edge {
    Top,
    Bottom,
}

/// The window she shows in, which the settings open and close.
type Shown = Rc<RefCell<Option<WindowHandle<Pet>>>>;

/// Ely in her window: she moves all the time, her acts follow the sessions' marks, and a press pets her.
pub struct Pet {
    rig: Rc<Rig>,
    mood: Mood,
    /// When the mood began, when she came, and when she was last pressed.
    since: Instant,
    shown: Instant,
    petted: Option<Instant>,
    /// What she was doing, her last pose, and the pose a change of act eases her from, with its time.
    act: Act,
    last: Option<Pose>,
    eased: Option<(Pose, Instant)>,
    /// The edge she stands on, and on Wayland the anchor her popup opened at, from the pane's top left, which never moves.
    edge: Edge,
    anchor: Option<Point<Pixels>>,
    /// How far in from the pane's left edge she stands, and a drag under way: the pointer's x where it began and where she stood then.
    at: f32,
    drag: Option<(f32, f32)>,
    main: WindowHandle<Workspace>,
    /// What keeps her window on the main window's edge, where the platform does not.
    glue: Option<platform::Glue>,
    _watch: Vec<Subscription>,
}

/// Shows Ely over `main` while the settings want her.
pub fn follow(main: WindowHandle<Workspace>, cx: &mut App) {
    let shown: Shown = Rc::default();
    let sync = {
        let shown = shown.clone();
        move |cx: &mut App| present(main, &shown, cx)
    };
    sync(cx);
    cx.observe_global::<SettingsStore>(sync).detach();
    if main
        .update(cx, |_, window, _| wayland(window))
        .is_ok_and(|wayland| wayland)
    {
        wayland::follow(main, shown, cx);
    }
}

/// Opens or closes her per settings; on Wayland, per activation.
fn present(main: WindowHandle<Workspace>, shown: &Shown, cx: &mut App) {
    let state = main.update(cx, |_, window, _| {
        (wayland(window), window.is_window_active())
    });
    let Ok((wayland, active)) = state else {
        return log::debug!("pet: the main window had gone");
    };
    let wanted = SettingsStore::get(cx).ely;
    let open = *shown.borrow();
    match (wanted && (active || !wayland), open) {
        (true, None) => *shown.borrow_mut() = open_pet(main, cx),
        (false, Some(pet)) => {
            *shown.borrow_mut() = None;
            let why = if wanted {
                "while the main window is inactive"
            } else {
                "as the settings say"
            };
            close(pet, why, cx);
        }
        _ => {}
    }
}

/// Opens her window, logging why when it does not open.
fn open_pet(main: WindowHandle<Workspace>, cx: &mut App) -> Option<WindowHandle<Pet>> {
    show(main, cx)
        .inspect_err(|error| log::error!("pet: Ely does not show: {error:#}"))
        .ok()
}

fn close(pet: WindowHandle<Pet>, why: &str, cx: &mut App) {
    log::info!("pet: Ely goes, {why}");
    if pet
        .update(cx, |_, window, _| window.remove_window())
        .is_err()
    {
        log::debug!("pet: her window had gone");
    }
}

/// How far in from the pane's left edge she stands on a pane `width` points wide: `at`, or less on a narrow pane, so she holds its edge.
fn left(width: Pixels, at: f32) -> f32 {
    at.min(f32::from(width) - WIDTH).max(0.0)
}

/// The edge she stands on with `room` points free above the pane, up to the top of the work area of the main window's display.
fn edge(room: f32) -> Edge {
    if room >= HEAD + ROOM {
        Edge::Top
    } else {
        Edge::Bottom
    }
}

/// Opens her window on `main`'s pane: a popup Wayland's compositor places, else a window kept there.
fn show(main: WindowHandle<Workspace>, cx: &mut App) -> anyhow::Result<WindowHandle<Pet>> {
    let kept = SettingsStore::get(cx).ely_left;
    let (bounds, pane, wayland, (edge, spot)) = main.update(cx, |_, window, _| {
        (
            window.bounds(),
            pane(window),
            wayland(window),
            wayland::spot(window, kept),
        )
    })?;
    let at = left(pane.size.width, kept);
    let room = size(px(WIDTH), px(HEAD + ROOM));
    let (edge, kind) = if wayland {
        let anchor = match edge {
            Edge::Top => popup::PopupAnchor::TopLeft,
            Edge::Bottom => popup::PopupAnchor::BottomLeft,
        };
        let kind = WindowKind::AnchoredPopup(popup::PopupOptions {
            parent: main.into(),
            anchor_rect: Bounds::new(pane.origin + spot, size(px(1.), px(1.))),
            anchor,
            gravity: popup::PopupGravity::TopRight,
            constraint_adjustment: popup::PopupConstraintAdjustment::empty(),
            offset: point(px(0.), px(0.)),
            grab: false,
        });
        (edge, kind)
    } else {
        (Edge::Top, WindowKind::PopUp)
    };
    let origin = bounds.origin + pane.origin + point(px(at), -room.height);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(origin, room))),
        titlebar: None,
        focus: false,
        show: true,
        kind,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    };
    let workspace = main.entity(cx)?;
    let feeling = |workspace: &Workspace, cx: &App| {
        mood(
            workspace
                .sessions()
                .iter()
                .filter_map(|session| session.looks(cx)),
        )
    };
    let start = feeling(workspace.read(cx), cx);
    let pet = cx.open_window(options, |_, cx| {
        cx.new(|cx| {
            let feel = cx.observe(&workspace, move |pet: &mut Pet, workspace, cx| {
                let mood = feeling(workspace.read(cx), cx);
                if mood != pet.mood {
                    pet.feel(mood, Instant::now());
                    cx.notify();
                }
            });
            Pet {
                rig: Rc::new(Rig::new()),
                mood: start,
                since: Instant::now(),
                shown: Instant::now(),
                petted: None,
                act: Act::Calm,
                last: None,
                eased: None,
                edge,
                anchor: wayland.then_some(spot),
                at: kept,
                drag: None,
                main,
                glue: None,
                _watch: vec![feel],
            }
        })
    })?;
    let (way, edge) = if wayland {
        ("a Wayland popup", edge)
    } else {
        let glued = glue(main, pet, cx);
        let edge = glued.inspect_err(|_| close(pet, "as her window cannot be kept there", cx))?;
        ("a window kept there", edge)
    };
    let edge = format!("{edge:?}").to_lowercase();
    log::info!(
        "pet: Ely peeks over the main window, {way}, on the pane's {edge} edge, feeling {start:?}"
    );
    Ok(pet)
}

/// Keeps her window on `main`'s pane as it moves, comes forward and hides, and gives the edge she first stands on.
fn glue(
    main: WindowHandle<Workspace>,
    pet: WindowHandle<Pet>,
    cx: &mut App,
) -> anyhow::Result<Edge> {
    main.update(cx, |_, main, cx| {
        pet.update(cx, |pet, window, cx| {
            let glue = platform::Glue::new(window, main)?;
            pet.edge = glue.place(main, pet.at)?;
            pet.glue = Some(glue);
            pet._watch
                .push(cx.observe_window_bounds(main, |pet, main, _| pet.place(main)));
            pet._watch
                .push(cx.observe_window_activation(main, |pet, main, _| pet.place(main)));
            #[cfg(target_os = "linux")]
            pet._watch.push(
                cx.observe_window_visibility(main, |pet, visibility, main, _| {
                    pet.shown(visibility.is_visible(), main)
                }),
            );
            anyhow::Ok(pet.edge)
        })?
    })?
}

/// The main window's pane, in its own points.
fn pane(window: &Window) -> Bounds<Pixels> {
    crate::blur::pane(
        window.viewport_size(),
        window.client_inset(),
        window.window_decorations(),
    )
}

/// Whether `window` is a Wayland surface, where the compositor places windows.
#[cfg(target_os = "linux")]
fn wayland(window: &Window) -> bool {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    HasWindowHandle::window_handle(window)
        .is_ok_and(|handle| matches!(handle.as_raw(), RawWindowHandle::Wayland(_)))
}

#[cfg(not(target_os = "linux"))]
fn wayland(_: &Window) -> bool {
    false
}

impl Pet {
    /// Takes up `mood` at `now`, its act beginning there, eased from where she is.
    fn feel(&mut self, mood: Mood, now: Instant) {
        log::debug!("pet: Ely feels {mood:?}, was {:?}", self.mood);
        (self.mood, self.since) = (mood, now);
        self.ease(now);
    }

    /// A press at `now`, which winks her, eased from where she is.
    fn press(&mut self, now: Instant) {
        log::debug!("pet: Ely is pressed");
        self.petted = Some(now);
        self.ease(now);
    }

    /// Eases her from her last pose, from `now`, as a move restarts.
    fn ease(&mut self, now: Instant) {
        self.eased = self.last.clone().map(|last| (last, now));
    }

    /// Her pose now, eased from where she was while a change of act or mood, or a press, is fresh.
    fn pose(&mut self, now: Instant) -> Pose {
        let since = |at: Instant| now.saturating_duration_since(at).as_secs_f64();
        let mut pose = pose(
            self.mood,
            since(self.since),
            since(self.shown),
            self.petted.map(since),
        );
        if pose.act != self.act {
            log::debug!("pet: Ely goes from {:?} to {:?}", self.act, pose.act);
            self.act = pose.act;
            self.ease(now);
        }
        if let Some((from, at)) = &self.eased {
            let into = since(*at) as f32;
            if into < EASE {
                pose = blend(from, pose, into);
            } else {
                self.eased = None;
            }
        }
        self.last = Some(pose.clone());
        pose
    }

    fn place(&mut self, main: &Window) {
        let Some(glue) = &self.glue else {
            return;
        };
        match glue.place(main, self.at) {
            Ok(edge) if edge != self.edge => {
                let name = format!("{edge:?}").to_lowercase();
                log::info!("pet: Ely moves to the pane's {name} edge");
                self.edge = edge;
            }
            Ok(_) => {}
            Err(error) => log::error!("{error:#}"),
        }
    }

    /// Takes hold of her for a drag along her edge, the pointer `local` in her window.
    fn grab(&mut self, local: Pixels, scale: f32) {
        match self.pointer(local, scale) {
            Ok(x) => self.drag = Some((x, self.at)),
            Err(error) => log::error!("{error:#}"),
        }
    }

    /// Moves her along her edge after the pointer, inside the pane.
    fn slide(&mut self, local: Pixels, scale: f32, cx: &mut App) {
        let Some((from, at)) = self.drag else {
            return;
        };
        let x = match self.pointer(local, scale) {
            Ok(x) => x,
            Err(error) => {
                self.drag = None;
                return log::error!("{error:#}");
            }
        };
        let main = self.main;
        let slid = main.update(cx, |_, main, _| {
            let room = (f32::from(pane(main).size.width) - WIDTH).max(0.0);
            // Whole points, so the kept place carries no float noise.
            self.at = (at + x - from).round().clamp(0.0, room.floor());
            self.place(main);
        });
        if let Err(error) = slid {
            log::debug!("pet: the main window does not read: {error:#}");
        }
    }

    /// Lets go of her, the settings keeping where the drag left her, where a Wayland popup then opens again.
    fn release(&mut self, cx: &mut App) {
        if self.drag.take().is_none() || SettingsStore::get(cx).ely_left == self.at {
            return;
        }
        let at = self.at;
        match SettingsStore::update(cx, |settings| settings.ely_left = at) {
            Ok(()) => log::info!("pet: Ely stands {at} points in from the pane's left edge"),
            Err(error) => log::error!("pet: where Ely stands is not kept: {error:#}"),
        }
    }

    /// The pointer's x in points: on the screen where her glue moves her window, else in her window, which a drag leaves in place.
    fn pointer(&self, local: Pixels, scale: f32) -> anyhow::Result<f32> {
        match &self.glue {
            Some(glue) => glue.pointer(scale),
            None => Ok(local.into()),
        }
    }

    #[cfg(target_os = "linux")]
    fn shown(&mut self, visible: bool, main: &Window) {
        if let Some(glue) = &self.glue {
            match glue.shown(visible) {
                Ok(()) => log::debug!("pet: Ely shows {visible} with the main window"),
                Err(error) => log::error!("{error:#}"),
            }
        }
        if visible {
            self.place(main);
        }
    }
}

impl Render for Pet {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pose = self.pose(Instant::now());
        // gpui gives a hidden window no frames, so she rests while unseen.
        window.request_animation_frame();
        let rig = self.rig.clone();
        let this = cx.entity();
        let drawn = canvas(
            |_, _, _| {},
            move |bounds, _, window, cx| {
                let head = point(bounds.origin.x, bounds.origin.y + px(ROOM));
                rig.paint(head, &pose, window, cx);
                // Her window keeps the pointer while the button is held, inside it or not.
                let held = this.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                    if phase == DispatchPhase::Bubble && event.dragging() {
                        let scale = window.scale_factor();
                        held.update(cx, |pet, cx| pet.slide(event.position.x, scale, cx));
                    }
                });
                window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                    if phase == DispatchPhase::Bubble && event.button == MouseButton::Left {
                        this.update(cx, |pet, cx| pet.release(cx));
                    }
                });
            },
        );
        div().size_full().child(drawn.size_full()).on_mouse_down(
            MouseButton::Left,
            cx.listener(|pet, event: &MouseDownEvent, window, cx| {
                pet.press(Instant::now());
                pet.grab(event.position.x, window.scale_factor());
                cx.notify();
            }),
        )
    }
}
