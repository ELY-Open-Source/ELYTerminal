use std::time::{Duration, Instant};

use super::{PASTE_LIFE, Session};
use crate::term::clipboard::{Pass, Place, Read};

#[test]
fn a_paste_password_reads_once_its_place_soon_and_only_the_newest() {
    let mut session = Session::default();
    let asking = |word: &str, place| Read {
        place,
        mimes: vec!["text/plain".into()],
        id: String::new(),
        pass: Some(Pass {
            word: word.into(),
            name: "Paste event".into(),
        }),
    };
    let start = Instant::now();
    let soon = start + Duration::from_secs(1);
    let old = session.once(Place::Clipboard, start).unwrap();
    let word = session.once(Place::Clipboard, start).unwrap();
    assert_eq!(word.len(), 32);
    assert!(
        !session.granted(&asking(&old, Place::Clipboard), soon),
        "a newer paste's only"
    );
    assert!(
        !session.granted(&asking(&word, Place::Primary), soon),
        "its own place's only"
    );
    assert!(session.granted(&asking(&word, Place::Clipboard), soon));
    assert!(
        !session.granted(&asking(&word, Place::Clipboard), soon),
        "spent"
    );
    let late = session.once(Place::Clipboard, start).unwrap();
    assert!(
        !session.granted(&asking(&late, Place::Clipboard), start + PASTE_LIFE),
        "expired"
    );
    // A session's grant keeps its place and lasts.
    session.always("kept".into(), Place::Clipboard);
    assert!(session.granted(&asking("kept", Place::Clipboard), soon));
    assert!(session.granted(&asking("kept", Place::Clipboard), start + PASTE_LIFE * 100));
    assert!(!session.granted(&asking("kept", Place::Primary), soon));
    assert!(!session.granted(
        &Read {
            pass: None,
            ..asking("kept", Place::Clipboard)
        },
        soon
    ));
}
