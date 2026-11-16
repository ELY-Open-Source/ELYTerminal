use super::*;

#[test]
fn each_mark_asks_her_mood() {
    assert_eq!(mood(Look::Idle), Mood::Calm);
    assert_eq!(mood(Look::Done), Mood::Done);
    assert_eq!(mood(Look::Working(Some(40))), Mood::Working);
    assert_eq!(mood(Look::Working(None)), Mood::Working);
    assert_eq!(mood(Look::Error), Mood::Error);
    assert_eq!(mood(Look::Waiting), Mood::Waiting);
}

/// Her pose `held` seconds into `mood`, long after she came, unpressed.
fn at(mood: Mood, held: f64) -> Pose {
    pose(mood, held, 10.0, None)
}

#[test]
fn she_moves_in_every_mood_even_after_weeks() {
    let weeks = 3.0 * 7.0 * 86_400.0;
    for (mood, held) in [
        (Mood::Calm, 30.0),
        (Mood::Calm, 400.0),
        (Mood::Done, 50.0),
        (Mood::Working, 30.0),
        (Mood::Waiting, 30.0),
        (Mood::Error, 30.0),
        (Mood::Working, weeks),
        (Mood::Calm, weeks),
    ] {
        let (now, soon) = (at(mood, held), at(mood, held + 0.3));
        assert_ne!(now.bob, soon.bob, "{mood:?} at {held}: her breath");
        assert_ne!(now.ahoge, soon.ahoge, "{mood:?} at {held}: her ahoge");
        assert_ne!(now.sway, soon.sway, "{mood:?} at {held}: her locks");
    }
}

#[test]
fn she_blinks_on_her_own_twice_at_the_short_gap() {
    let open = |held: f64| at(Mood::Calm, held).open;
    assert_eq!(open(2.0), [1.0, 1.0], "open between blinks");
    assert!(open(3.48)[0] < 0.1, "shut in the blink at 3.4 s");
    assert_eq!(open(3.7), [1.0, 1.0], "open again");
    assert!(
        open(8.08)[0] < 0.1 && open(8.4)[0] < 0.1,
        "twice at the short gap"
    );
    assert!(
        open(3.43)[0] > open(3.46)[0],
        "she shuts them, not snaps them"
    );
    assert!(
        open(0.0)[0] < 0.1 && open(0.2) == [1.0, 1.0],
        "her eyes open as an act begins"
    );
}

#[test]
fn calm_glances_left_then_right_with_her_head() {
    let calm = |held: f64| at(Mood::Calm, held);
    assert_eq!(calm(5.0).look, (0.0, 0.0));
    assert_eq!(calm(9.6).look, (-6.0, 0.0));
    assert_eq!(calm(16.6).look, (6.0, 0.0));
    assert!(
        calm(16.6).tilt > 0.0 && calm(9.6).tilt < 0.0,
        "her head follows"
    );
    assert!(calm(9.1).look.0 > -6.0, "her eyes ease over");
    let breath = idle(Act::Calm, 12.6, 2.6, 4.0).ahoge;
    assert!(
        (calm(12.6).ahoge - breath).abs() > 1.0,
        "her ahoge wiggles after her glances"
    );
}

#[test]
fn calm_dozes_after_five_minutes_with_zs() {
    let dozing = at(Mood::Calm, 301.0);
    assert_eq!((dozing.act, dozing.eyes), (Act::Doze, Eyes::Lines));
    let marks: Vec<Mark> = dozing.marks.iter().map(|(mark, _)| *mark).collect();
    assert_eq!(marks, [Mark::Z1, Mark::Z2]);
    assert!(
        at(Mood::Calm, 300.3).open[0] < 1.0,
        "her eyes fall shut first"
    );
    let z = |held: f64| at(Mood::Calm, held).marks[0].1;
    assert!(z(305.6).at.1 < z(305.0).at.1, "the zs float up");
}

#[test]
fn a_finish_cheers_her_with_a_hop_then_she_calms() {
    let cheer = at(Mood::Done, 0.3);
    assert_eq!(
        (cheer.act, cheer.eyes, cheer.mouth),
        (Act::Cheer, Eyes::Arcs, Mouth::Grin)
    );
    assert!(cheer.lift > 10.0, "a hop");
    assert_eq!(cheer.marks.len(), 2, "two sparkles");
    let calm = at(Mood::Done, 5.0);
    assert_eq!(
        (calm.act, calm.eyes, calm.mouth),
        (Act::Calm, Eyes::Open, Mouth::Smile)
    );
    assert!(
        at(Mood::Done, 4.0).open[0] < 0.1,
        "her eyes open again as she calms"
    );
}

#[test]
fn work_runs_her_eyes_along_the_lines_and_taps_her_fingers() {
    let work = |held: f64| at(Mood::Working, held);
    assert_eq!(work(1.0).act, Act::Work);
    assert_eq!(work(1.0).look.1, 5.0, "her eyes on the terminal");
    assert!(work(0.4).look.0 < work(1.6).look.0, "along a line");
    assert!(
        work(0.03).hands[0] > 0.0 && work(0.33).hands[1] > 0.0,
        "by turns"
    );
    assert_eq!(work(2.5).hands, [0.0, 0.0], "then a rest");
}

#[test]
fn a_wait_widens_her_eyes_and_hops_her_twice_every_round() {
    let wait = |held: f64| at(Mood::Waiting, held);
    assert!(wait(1.0).wide > 1.1);
    assert_eq!((wait(1.0).act, wait(1.0).mouth), (Act::Wait, Mouth::Gasp));
    assert!(wait(0.15).lift > 20.0 && wait(0.45).lift > 20.0, "two hops");
    assert!(wait(2.75).lift > 20.0, "again next round");
    assert_eq!(wait(1.5).lift, 0.0);
    assert_eq!(wait(1.0).marks[0].0, Mark::Bang);
}

#[test]
fn an_error_frets_her_with_a_shake_and_a_drop() {
    let fret = |held: f64| at(Mood::Error, held);
    assert_eq!(fret(1.0).brows, 1.0);
    assert_eq!((fret(1.0).act, fret(1.0).mouth), (Act::Fret, Mouth::Frown));
    assert!(fret(0.15).shake.abs() > 1.0, "a shake");
    assert_eq!(fret(1.0).shake, 0.0);
    assert!(fret(4.05).shake.abs() > 1.0, "a smaller one later");
    assert!(
        fret(1.5).marks[0].1.at.1 > fret(0.5).marks[0].1.at.1,
        "the drop runs down"
    );
}

#[test]
fn a_press_winks_with_a_heart_that_rises() {
    let pressed = |since: f64| pose(Mood::Calm, 30.0, 30.0, Some(since));
    assert_eq!(
        (pressed(0.5).act, pressed(0.5).eyes),
        (Act::Wink, Eyes::Wink)
    );
    assert_eq!(pressed(0.5).marks[0].0, Mark::Heart);
    assert!(pressed(1.0).marks[0].1.at.1 < pressed(0.5).marks[0].1.at.1);
    assert_eq!(
        (pressed(1.6).act, pressed(1.6).eyes),
        (Act::Calm, Eyes::Open),
        "the wink lasts 1.5 s"
    );
    assert!(pressed(1.55).open[0] < 0.5, "her eye opens after it");
    let asleep = pose(Mood::Calm, 400.0, 400.0, Some(0.5));
    assert!(asleep.open[1] > 0.9, "a press wakes her other eye");
}

#[test]
fn a_press_wakes_her_from_her_doze_for_five_minutes_more() {
    assert_eq!(at(Mood::Calm, 400.0).act, Act::Doze, "asleep, unpressed");
    let pressed = |since: f64| pose(Mood::Calm, 400.0 + since, 410.0 + since, Some(since));
    assert_eq!(pressed(0.5).act, Act::Wink);
    assert_eq!(
        (pressed(2.0).act, pressed(2.0).eyes),
        (Act::Calm, Eyes::Open),
        "awake after the wink"
    );
    assert_eq!(pressed(299.0).act, Act::Calm, "awake five minutes");
    assert_eq!(pressed(301.0).act, Act::Doze, "then asleep again");
    assert!(pressed(300.3).open[0] < 1.0, "her eyes fall shut again");
    let finished = pose(Mood::Done, 400.0, 410.0, Some(2.0));
    assert_eq!(finished.act, Act::Calm, "a finish long past wakes alike");
}

#[test]
fn she_rises_from_behind_the_edge_when_she_comes() {
    assert!(pose(Mood::Calm, 0.0, 0.0, None).lift <= -DEPTH);
    assert!(pose(Mood::Calm, 0.3, 0.3, None).lift < 0.0);
    assert_eq!(pose(Mood::Calm, 5.0, 0.6, None).lift, 0.0);
}

#[test]
fn a_change_of_act_eases_her_from_her_last_pose() {
    let working = at(Mood::Working, 1.0);
    let early = blend(&working, at(Mood::Calm, 0.1), 0.1);
    assert!(
        early.tilt < 0.0 && early.tilt > working.tilt,
        "her head turns back, not at once: {} from {}",
        early.tilt,
        working.tilt
    );
    let calm = at(Mood::Calm, EASE as f64);
    let eased = blend(&working, calm.clone(), EASE);
    assert!(
        (eased.tilt - calm.tilt).abs() < 1e-5 && (eased.look.1 - calm.look.1).abs() < 1e-5,
        "eased in {EASE} s"
    );
    let waiting = at(Mood::Waiting, 1.0);
    let leaving = blend(&waiting, at(Mood::Calm, 0.1), 0.1);
    let bang = (leaving.marks.iter())
        .find(|(mark, _)| *mark == Mark::Bang)
        .expect("the mark she leaves fades, not vanishes")
        .1;
    assert!(bang.alpha > 0.0 && bang.alpha < 1.0);
    assert_eq!(leaving.eyes, Eyes::Open, "the shapes are the new act's");
    let fretting = at(Mood::Error, 1.0);
    let calming = blend(&fretting, at(Mood::Calm, 0.1), 0.1);
    assert!(
        calming.brows > 0.0 && calming.brows < 1.0,
        "her brows ease down"
    );
}
