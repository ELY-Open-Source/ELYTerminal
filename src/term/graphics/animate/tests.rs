use std::time::{Duration, Instant};

use base64::{Engine, engine::general_purpose::STANDARD};

use super::super::{
    frames::{State, compose},
    place::tests::{Screen, screen},
    store::{Image, Store},
};

const A: [u8; 4] = [1, 2, 3, 255];
const B: [u8; 4] = [4, 5, 6, 255];

/// Image 5, pixels A then B, placed top-left.
fn animated() -> Screen {
    let mut s = screen();
    let pixels = STANDARD.encode([A, B].concat());
    s.feed(&format!(
        "\x1b_Gi=5,s=2,v=1,q=2;{pixels}\x1b\\\x1b_Ga=p,i=5,C=1,q=2\x1b\\"
    ));
    s
}

fn send(s: &mut Screen, keys: &str, pixels: &[u8]) {
    s.feed(&format!("\x1b_G{keys};{}\x1b\\", STANDARD.encode(pixels)));
}

fn image(s: &Screen, id: u32) -> Image {
    s.shared
        .lock()
        .unwrap()
        .screen()
        .store
        .get(id)
        .cloned()
        .expect("stored")
}

fn reply(keys: &str, message: &str) -> String {
    format!("\x1b_G{keys};{message}\x1b\\")
}

#[test]
fn a_frame_composes_onto_its_canvas_and_answers_its_number() {
    let mut s = animated();
    send(&mut s, "a=f,i=5,s=1,v=1,x=1", &[9, 9, 9, 255]);
    send(&mut s, "a=f,i=5,c=1,s=1,v=1", &[7, 7, 7, 255]);
    // Y fills a ground, red its top byte.
    send(&mut s, "a=f,i=5,Y=4278190335,X=1,s=1,v=1", &[0; 4]);
    // An edit blends onto frame 2; z=-1 makes it gapless.
    send(&mut s, "a=f,i=5,r=2,z=-1,s=1,v=1", &[255, 255, 255, 128]);
    let ok = |frame| reply(&format!("i=5,r={frame}"), "OK");
    assert_eq!(s.heard(), [ok(2), ok(3), ok(4), ok(2)]);
    let image = image(&s, 5);
    let frame = |number| image.frame(number).expect("a frame").to_vec();
    assert_eq!(frame(2), [[255, 255, 255, 128], [9, 9, 9, 255]].concat());
    assert_eq!(frame(3), [[7, 7, 7, 255], B].concat());
    assert_eq!(frame(4), [[0, 0, 0, 0], [255, 0, 0, 255]].concat());
    let gaps: Vec<u32> = image
        .animation
        .as_ref()
        .unwrap()
        .frames
        .iter()
        .map(|it| it.gap)
        .collect();
    assert_eq!(gaps, [0, 40, 40]);
    assert_eq!(image.size(), 8 * 4, "four frames of two pixels");
    // A frame in chunks, its first command's keys holding.
    let pixels = STANDARD.encode([A, B].concat());
    s.feed(&format!("\x1b_Ga=f,i=5,s=2,v=1,m=1;{}\x1b\\", &pixels[..8]));
    s.feed(&format!("\x1b_Ga=f,m=0;{}\x1b\\", &pixels[8..]));
    assert_eq!(s.heard(), [ok(5)]);
}

#[test]
fn a_frame_names_an_image_there_and_fits_it() {
    let mut s = animated();
    send(&mut s, "a=f,i=5,s=3,v=1", &[0; 12]);
    send(&mut s, "a=f,i=5,c=9,s=1,v=1", &[0; 4]);
    send(&mut s, "a=f,i=8,s=1,v=1", &[0; 4]);
    send(&mut s, "a=f,s=1,v=1", &[0; 4]);
    let failed = [
        reply("i=5", "EINVAL: frame dimensions exceed image"),
        reply("i=5", "EINVAL: base frame not found"),
        reply("i=8", "ENOENT: image not found"),
    ];
    assert_eq!(s.heard(), failed);
    assert!(image(&s, 5).animation.is_none_or(|it| it.frames.is_empty()));
}

#[test]
fn the_clock_steps_frames_by_their_gaps_as_kitty_does() {
    let mut s = animated();
    send(&mut s, "a=f,i=5,s=1,v=1", &A);
    send(&mut s, "a=f,i=5,s=1,v=1", &B);
    assert_eq!(s.heard().len(), 2);
    s.feed("\x1b_Ga=a,i=5,r=1,z=100\x1b\\\x1b_Ga=a,i=5,s=3\x1b\\");
    assert!(s.heard().is_empty(), "a=a answers nothing done");
    let start = Instant::now();
    let at = |ms| start + Duration::from_millis(ms);
    let tick = |ms| {
        let mut shared = s.shared.lock().unwrap();
        let next = shared.tick(at(ms));
        let image = shared.screen().store.get(5).unwrap();
        (
            image.animation.as_ref().unwrap().current,
            next,
            image.version,
        )
    };
    let (first, next, version) = tick(0);
    assert_eq!((first, next), (0, Some(Duration::from_millis(100))));
    let (second, next, moved) = tick(100);
    assert_eq!((second, next), (1, Some(Duration::from_millis(40))));
    assert_ne!(moved, version, "a new frame is a new texture");
    assert_eq!(tick(140).0, 2);
    assert_eq!(tick(180).0, 0, "it loops");
    // v=2 plays one loop more; gapless frames are passed over.
    s.feed("\x1b_Ga=a,i=5,r=2,z=-1,v=2,s=3\x1b\\");
    let tick = |ms| {
        let mut shared = s.shared.lock().unwrap();
        let next = shared.tick(at(ms));
        (
            shared
                .screen()
                .store
                .get(5)
                .unwrap()
                .animation
                .as_ref()
                .unwrap()
                .current,
            next,
        )
    };
    assert_eq!(tick(280), (2, Some(Duration::from_millis(40))));
    assert_eq!(tick(320), (2, None), "its one loop is done");
}

#[test]
fn loading_waits_at_the_last_frame_and_stopped_moves_by_hand() {
    let mut s = animated();
    send(&mut s, "a=f,i=5,s=1,v=1", &A);
    s.feed("\x1b_Ga=a,i=5,r=1,z=10\x1b\\\x1b_Ga=a,i=5,s=2\x1b\\");
    let start = Instant::now();
    let current = |s: &Screen, ms| {
        let mut shared = s.shared.lock().unwrap();
        shared.tick(start + Duration::from_millis(ms));
        shared
            .screen()
            .store
            .get(5)
            .unwrap()
            .animation
            .as_ref()
            .unwrap()
            .current
    };
    assert_eq!(current(&s, 0), 0);
    assert_eq!(current(&s, 10), 1);
    assert_eq!(current(&s, 100), 1, "it waits for more");
    send(&mut s, "a=f,i=5,s=1,v=1,q=1", &B);
    assert_eq!(current(&s, 140), 2);
    s.feed("\x1b_Ga=a,i=5,s=1\x1b\\\x1b_Ga=a,i=5,c=1\x1b\\");
    assert_eq!(current(&s, 1000), 0, "stopped, c moves it");
    let state = image(&s, 5).animation.unwrap().state;
    assert_eq!(state, State::Stopped);
    // An image no placement shows never steps.
    send(&mut s, "a=t,i=6,s=1,v=1,q=2", &A);
    send(&mut s, "a=f,i=6,s=1,v=1,q=2", &B);
    s.feed("\x1b_Ga=a,i=6,r=1,z=10,s=3\x1b\\");
    s.shared
        .lock()
        .unwrap()
        .tick(start + Duration::from_secs(5));
    assert_eq!(image(&s, 6).animation.unwrap().current, 0);
}

#[test]
fn a_hidden_session_s_gaps_start_afresh() {
    let mut s = animated();
    send(&mut s, "a=f,i=5,s=1,v=1", &A);
    s.feed("\x1b_Ga=a,i=5,r=1,z=100,s=3\x1b\\");
    let start = Instant::now();
    s.shared.lock().unwrap().tick(start);
    s.shared.lock().unwrap().resume();
    // Shown again later, the frame shows its whole gap.
    let later = start + Duration::from_secs(60);
    let next = s.shared.lock().unwrap().tick(later);
    assert_eq!(next, Some(Duration::from_millis(100)));
    assert_eq!(image(&s, 5).animation.unwrap().current, 0);
}

#[test]
fn compose_copies_a_rectangle_between_frames() {
    let mut s = animated();
    send(&mut s, "a=f,i=5,s=1,v=1,q=1", &[0; 4]);
    s.feed("\x1b_Ga=c,i=5,r=1,c=2,w=1,h=1,X=1,x=0,C=1\x1b\\");
    assert_eq!(s.heard(), [reply("i=5", "OK")]);
    assert_eq!(image(&s, 5).frame(2).unwrap(), [B, [0; 4]].concat());
    for (keys, message) in [
        ("r=9,c=2", "ENOENT: source frame not found"),
        ("r=1,c=9", "ENOENT: destination frame not found"),
        (
            "r=1,c=2,w=2,x=1",
            "EINVAL: destination rectangle out of bounds",
        ),
        ("r=1,c=2,w=2,X=1", "EINVAL: source rectangle out of bounds"),
        (
            "r=1,c=1,w=1",
            "EINVAL: source and destination rectangles overlap",
        ),
    ] {
        s.feed(&format!("\x1b_Ga=c,i=5,{keys}\x1b\\"));
        assert_eq!(s.heard(), [reply("i=5", message)], "{keys}");
    }
    // Apart, one frame composes onto itself.
    s.feed("\x1b_Ga=c,i=5,r=1,c=1,w=1,X=1,x=0,C=1,q=1\x1b\\");
    assert_eq!(image(&s, 5).pixels, [B, B].concat());
}

#[test]
fn a_frame_delete_takes_its_frame_and_a_capital_takes_a_still_image() {
    let mut s = animated();
    send(&mut s, "a=f,i=5,c=1,X=1,s=1,v=1,q=1", &[7; 4]);
    send(&mut s, "a=f,i=5,c=1,X=1,s=1,v=1,q=1", &[8; 4]);
    s.feed("\x1b_Ga=a,i=5,c=3\x1b\\\x1b_Ga=d,d=f,i=5,r=2\x1b\\");
    let shown = |s: &Screen| image(s, 5).shown().to_vec();
    assert_eq!(shown(&s), [[8; 4], B].concat(), "frame 3 stays shown");
    assert_eq!(image(&s, 5).size(), 4 * 4);
    // r=0 takes the first; the next becomes the image.
    s.feed("\x1b_Ga=d,d=f,i=5\x1b\\");
    assert_eq!(image(&s, 5).pixels, [[8; 4], B].concat());
    assert_eq!(shown(&s), [[8; 4], B].concat());
    s.feed("\x1b_Ga=d,d=f,i=5\x1b\\");
    assert_eq!(
        image(&s, 5).pixels.len(),
        8,
        "a still image keeps its pixels"
    );
    s.feed("\x1b_Ga=d,d=F,i=5\x1b\\");
    assert!(!s.stored(5) && s.placed().is_empty());
}

#[test]
fn a_blend_is_source_over_as_ghostty_s() {
    let blend = |under: [u8; 4], over: [u8; 4]| {
        let mut canvas = under.to_vec();
        compose(&mut canvas, (1, 1), &over, (1, 1), (0, 0), false);
        canvas
    };
    assert_eq!(
        blend([10, 20, 30, 40], [100, 110, 120, 255]),
        [100, 110, 120, 255]
    );
    assert_eq!(
        blend([10, 20, 30, 40], [100, 110, 120, 0]),
        [10, 20, 30, 40]
    );
    assert_eq!(
        blend([0, 0, 0, 255], [255, 255, 255, 128]),
        [128, 128, 128, 255]
    );
    assert_eq!(
        blend([0, 0, 0, 0], [200, 100, 50, 128]),
        [200, 100, 50, 128]
    );
    // A rectangle past the canvas is cut.
    let mut canvas = vec![0; 16];
    compose(
        &mut canvas,
        (2, 2),
        &[1, 2, 3, 4, 5, 6, 7, 8],
        (2, 1),
        (1, 1),
        true,
    );
    assert_eq!(canvas, [[0; 12].as_slice(), &[1, 2, 3, 4]].concat());
}

#[test]
fn a_frame_past_the_quota_takes_its_evicted_placements_along() {
    let mut s = screen();
    s.shared.lock().unwrap().screen().store = Store::new(20);
    let pixels = STANDARD.encode([A, B].concat());
    s.feed(&format!("\x1b_Gi=5,s=2,v=1,q=2;{pixels}\x1b\\"));
    send(&mut s, "a=f,i=5,s=2,v=1,q=2", &[A, B].concat());
    send(&mut s, "a=T,i=6,s=1,v=1,C=1,q=2", &A);
    assert_eq!(s.placed().len(), 1);
    send(&mut s, "a=f,i=5,s=2,v=1", &[A, B].concat());
    let full = reply("i=5", "ENOSPC: animation frame storage full");
    assert_eq!(s.heard(), [full]);
    assert!(!s.stored(6) && s.placed().is_empty(), "image 6 went whole");
    // Sent again past the quota, it takes its placement.
    send(&mut s, "a=T,i=6,s=1,v=1,C=1,q=2", &A);
    send(&mut s, "i=6,s=3,v=2", &[0; 24]);
    assert_eq!(s.heard(), [reply("i=6", "ENOMEM: out of memory")]);
    assert!(!s.stored(6) && s.placed().is_empty());
}

#[test]
fn a_hidden_session_gives_its_textures_back() {
    let s = animated();
    let mut pictures = super::super::Pictures::default();
    let mut shown = || {
        let shared = s.shared.lock().unwrap();
        let (shown, _) = pictures.shown(&shared, &[], 0, (10, 5));
        shown[0].picture.id
    };
    let (first, again) = (shown(), shown());
    assert_eq!(first, again, "a shown texture is kept");
    assert_eq!(pictures.release().len(), 1);
    assert!(pictures.release().is_empty());
    let shared = s.shared.lock().unwrap();
    let (rebuilt, _) = pictures.shown(&shared, &[], 0, (10, 5));
    assert_ne!(rebuilt[0].picture.id, first, "shown again, it builds anew");
}
