//! What Ely feels from the shown session's mark, and her pose at each moment of it, moving as the films move her.

mod curves;
#[cfg(test)]
mod tests;

use std::f32::consts::PI;

use self::curves::{
    SHUT, between, blink, early, floating, fold, hops, opening, overshoot, popping, shaking,
    smooth, tap, twinkling, wiggle,
};
use crate::sessions::orb::Look;

/// What a mark asks of her.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mood {
    Calm,
    Done,
    Working,
    Error,
    Waiting,
}

/// The mood a mark's look asks for.
pub fn mood(look: Look) -> Mood {
    match look {
        Look::Waiting => Mood::Waiting,
        Look::Error => Mood::Error,
        Look::Working(_) => Mood::Working,
        Look::Done => Mood::Done,
        Look::Idle => Mood::Calm,
    }
}

/// Seconds she cheers a program that finished, and calm lasts, from her mood's start or the last press, before she dozes.
const CHEER: f64 = 4.0;
const DOZE: f64 = 300.0;
/// Seconds a press keeps her wink.
const PETTED: f32 = 1.5;
/// Seconds she takes to rise from behind the edge, from as deep as she is tall.
const RISE: f32 = 0.6;
const DEPTH: f32 = 280.0;
/// Units she sinks while she peeks low: her brows under the edge, at her highest hop too.
pub const LOW: f32 = 188.0;
/// Seconds she takes to ease from one act into the next.
pub const EASE: f32 = 0.3;

/// What she is doing; a change eases her from her last pose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Calm,
    Doze,
    Cheer,
    Work,
    Wait,
    Fret,
    Wink,
}

/// Her eyes: open, smiling arcs, sleeping lines, or a wink of the left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Eyes {
    Open,
    Arcs,
    Lines,
    Wink,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mouth {
    Smile,
    Grin,
    Gasp,
    Flat,
    Frown,
}

/// A mark beside her head.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Bang,
    Z1,
    Z2,
    Heart,
    SparkleLeft,
    SparkleRight,
    Drop,
}

/// How a mark moves from its place: by `at` units, grown by `scale`, turned `turn` degrees, at `alpha`.
#[derive(Clone, Copy, Debug)]
pub struct Motion {
    pub at: (f32, f32),
    pub scale: f32,
    pub turn: f32,
    pub alpha: f32,
}

/// Her pose at one moment, in the icon's units and degrees: lift and bob go up, marks and hands move as the icon's y does, down.
#[derive(Clone, Debug)]
pub struct Pose {
    pub act: Act,
    /// All of her up, and aside.
    pub lift: f32,
    pub shake: f32,
    /// Her head up with her breath, and turned.
    pub bob: f32,
    pub tilt: f32,
    pub sway: f32,
    pub ahoge: f32,
    pub look: (f32, f32),
    /// Each eye's height, left then right, and both eyes' size.
    pub open: [f32; 2],
    pub wide: f32,
    pub eyes: Eyes,
    pub mouth: Mouth,
    /// The mouth's size, which pops as it changes.
    pub pop: f32,
    /// How much her worried brows show.
    pub brows: f32,
    pub flush: f32,
    /// Each hand pressed down, left then right.
    pub hands: [f32; 2],
    pub marks: Vec<(Mark, Motion)>,
}

/// Her pose `held` seconds into `mood`, `shown` seconds after she came, and `petted` seconds after a press, which wakes her from a doze for `DOZE` more.
pub fn pose(mood: Mood, held: f64, shown: f64, petted: Option<f64>) -> Pose {
    let stirred = petted.map_or(held, |since| held.min(since));
    let mut pose = match mood {
        Mood::Waiting => waiting(held),
        Mood::Error => fretting(held),
        Mood::Working => working(held),
        Mood::Done if held < CHEER => cheering(held),
        Mood::Calm | Mood::Done if stirred >= DOZE => dozing(stirred - DOZE),
        Mood::Calm => calm(held),
        Mood::Done => calm(held - CHEER),
    };
    if shown < RISE as f64 {
        pose.lift -= DEPTH * (1.0 - shown as f32 / RISE).powi(3);
    }
    match petted.map(early) {
        Some(since) if since < PETTED => winking(&mut pose, since),
        Some(since) => pose.open[0] = pose.open[0].min(opening(since - PETTED)),
        None => {}
    }
    pose
}

/// `to`, `since` seconds after a change of act or mood, or a press, eased from `from`, her pose then: her place, her moves and her brows blend, the marks `to` lacks fade, and the shapes are `to`'s.
pub fn blend(from: &Pose, to: Pose, since: f32) -> Pose {
    let k = smooth((since / EASE).clamp(0.0, 1.0));
    let mix = |a: f32, b: f32| a + (b - a) * k;
    let fading: Vec<(Mark, Motion)> = (from.marks.iter())
        .filter(|(mark, _)| to.marks.iter().all(|(kept, _)| kept != mark))
        .map(|&(mark, motion)| {
            let alpha = motion.alpha * (1.0 - k);
            (mark, Motion { alpha, ..motion })
        })
        .collect();
    let mut pose = Pose {
        lift: mix(from.lift, to.lift),
        shake: mix(from.shake, to.shake),
        bob: mix(from.bob, to.bob),
        tilt: mix(from.tilt, to.tilt),
        sway: mix(from.sway, to.sway),
        ahoge: mix(from.ahoge, to.ahoge),
        look: (mix(from.look.0, to.look.0), mix(from.look.1, to.look.1)),
        wide: mix(from.wide, to.wide),
        brows: mix(from.brows, to.brows),
        flush: mix(from.flush, to.flush),
        hands: [
            mix(from.hands[0], to.hands[0]),
            mix(from.hands[1], to.hands[1]),
        ],
        ..to
    };
    pose.marks.extend(fading);
    pose
}

/// Life under every act, `t` seconds into it: a bob of `amp` units every `period` seconds, swaying locks, a breathing ahoge, and blinks, her eyes opening and her mouth popping as the act begins.
fn idle(act: Act, t: f64, period: f32, amp: f32) -> Pose {
    let phase = 2.0 * PI * fold(t, period) / period;
    Pose {
        act,
        lift: 0.0,
        shake: 0.0,
        bob: phase.sin() * amp,
        tilt: 0.0,
        sway: (PI * fold(t, 2.0 * period) / period + 1.1).sin() * 2.2,
        ahoge: (phase + 0.6).sin() * 4.0,
        look: (0.0, 0.0),
        open: [blink(t).min(opening(early(t))); 2],
        wide: 1.0,
        eyes: Eyes::Open,
        mouth: Mouth::Smile,
        pop: popping(early(t)),
        brows: 0.0,
        flush: 0.0,
        hands: [0.0; 2],
        marks: Vec::new(),
    }
}

/// Calm: she glances left, then right, her head after her eyes, and her ahoge wiggles.
fn calm(t: f64) -> Pose {
    let mut pose = idle(Act::Calm, t, 2.6, 4.0);
    let at = fold(t, 20.0);
    let look = 6.0 * (between(at, 16.0, 17.2) - between(at, 9.0, 10.2));
    pose.look = (look, 0.0);
    pose.tilt = look * 0.5;
    pose.ahoge += wiggle(at - 12.4);
    pose
}

/// Dozing: her eyes fall shut, her head and ahoge droop, and zs float up.
fn dozing(t: f64) -> Pose {
    let mut pose = idle(Act::Doze, t, 4.2, 3.0);
    let settle = smooth((early(t) / 0.8).min(1.0));
    if t < 0.6 {
        pose.open = [1.0 - (1.0 - SHUT) * smooth(early(t) / 0.6); 2];
    } else {
        pose.eyes = Eyes::Lines;
        pose.open = [1.0; 2];
    }
    pose.mouth = Mouth::Flat;
    pose.tilt = settle * 5.0 + (2.0 * PI * fold(t, 8.4) / 8.4).sin() * 1.5;
    pose.ahoge += settle * 14.0;
    pose.marks = vec![(Mark::Z1, floating(t, 0.0)), (Mark::Z2, floating(t, 0.5))];
    pose
}

/// Cheering: smiling eyes and mouth, a hop, and twinkling sparkles.
fn cheering(t: f64) -> Pose {
    let mut pose = idle(Act::Cheer, t, 2.2, 4.0);
    let t = early(t);
    pose.eyes = Eyes::Arcs;
    pose.mouth = Mouth::Grin;
    pose.flush = 0.6;
    pose.lift = hops(t, 1.0);
    pose.ahoge += wiggle(t - 0.1);
    pose.marks = vec![
        (Mark::SparkleLeft, twinkling(t, 0.0)),
        (Mark::SparkleRight, twinkling(t, 0.45)),
    ];
    pose
}

/// Working: her eyes run along the terminal's lines, and her fingers tap by turns.
fn working(t: f64) -> Pose {
    let mut pose = idle(Act::Work, t, 2.0, 2.5);
    let at = fold(t, 2.25);
    let scan = if at < 2.0 {
        -5.0 + 10.0 * smooth(at / 2.0)
    } else {
        5.0 - 10.0 * smooth((at - 2.0) / 0.25)
    };
    pose.look = (scan, 5.0);
    pose.mouth = Mouth::Flat;
    pose.tilt = -2.0;
    let beat = fold(t, 3.0);
    if beat < 1.8 {
        let n = (beat / 0.3) as usize;
        pose.hands[n % 2] = tap(beat - n as f32 * 0.3);
    }
    pose
}

/// Waiting on you: wide eyes, a gasp, a mark that rocks, and two hops every 2.6 s.
fn waiting(t: f64) -> Pose {
    let mut pose = idle(Act::Wait, t, 1.6, 3.0);
    let beat = 2.0 * PI * fold(t, 0.8) / 0.8;
    pose.wide = 1.0 + 0.12 * overshoot((early(t) / 0.12).min(1.0)) + 0.02 * beat.sin();
    pose.mouth = Mouth::Gasp;
    pose.look = (0.0, -2.0);
    let at = fold(t, 2.6);
    pose.lift = hops(at, 2.0);
    pose.ahoge += wiggle(at);
    let bang = Motion {
        at: (0.0, 0.0),
        scale: popping(early(t)),
        turn: beat.sin() * 8.0,
        alpha: 1.0,
    };
    pose.marks = vec![(Mark::Bang, bang)];
    pose
}

/// An error: brows up, a frown, a shake, a smaller one every 4.5 s, and a drop that runs down.
fn fretting(t: f64) -> Pose {
    let mut pose = idle(Act::Fret, t, 2.4, 1.5);
    pose.brows = 1.0;
    pose.mouth = Mouth::Frown;
    pose.look = (0.0, 3.0);
    pose.tilt = -3.0;
    pose.shake = shaking(early(t), 10.0) + shaking(fold(t, 4.5) - 3.9, 4.0);
    let f = fold(t, 2.0) / 2.0;
    let drop = Motion {
        at: (0.0, 18.0 * smooth(f)),
        scale: 1.0,
        turn: 0.0,
        alpha: (f * PI).sin(),
    };
    pose.marks = vec![(Mark::Drop, drop)];
    pose
}

/// A press, `since` seconds ago: a wink, a grin, a blush, a small hop, a wiggle and a heart that rises; an eye she slept with opens.
fn winking(pose: &mut Pose, since: f32) {
    let woke = pose.eyes != Eyes::Open;
    pose.act = Act::Wink;
    pose.eyes = Eyes::Wink;
    pose.open = [
        opening(since),
        if woke { opening(since) } else { pose.open[1] },
    ];
    pose.mouth = Mouth::Grin;
    pose.pop = popping(since);
    pose.flush = 1.0;
    pose.lift += hops(since, 1.0) * 0.6;
    pose.ahoge += wiggle(since);
    let heart = Motion {
        at: (0.0, -26.0 * since / PETTED),
        scale: popping(since),
        turn: 0.0,
        alpha: ((PETTED - since) / 0.4).min(1.0),
    };
    pose.marks.push((Mark::Heart, heart));
}
