use super::super::{
    Pictures,
    place::tests::{Screen, screen},
    runs,
    unicode::Run,
};

/// Each shown placement's clip, origin and size in cells.
fn clips(s: &Screen) -> Vec<(f32, f32, f32, f32)> {
    let mut pictures = Pictures::default();
    let (shown, _) = pictures.shown(&s.shared.lock().unwrap(), &runs(&s.term), 0, (10, 5));
    let clip = |shown: &super::super::Shown| {
        let (at, size) = (shown.clip.origin, shown.clip.size);
        (at.x, at.y, size.width, size.height)
    };
    shown.iter().map(clip).collect()
}

fn ok(keys: &str) -> String {
    format!("\x1b_G{keys};OK\x1b\\")
}

#[test]
fn a_relative_placement_follows_its_parent_and_leaves_the_cursor() {
    let mut s = screen();
    s.transmit(2, "");
    s.feed("\x1b[2;3H\x1b_Ga=p,i=1,p=1,C=1\x1b\\\x1b_Ga=p,i=2,p=1,P=1,Q=1,H=3,V=1\x1b\\");
    assert_eq!(s.heard(), [ok("i=1,p=1"), ok("i=2,p=1")]);
    assert_eq!(s.cursor(), (1, 2));
    assert_eq!(clips(&s), [(2., 1., 2., 2.), (5., 2., 2., 2.)]);
    s.feed("\x1b[5;1H\n");
    assert_eq!(clips(&s), [(2., 0., 2., 2.), (5., 1., 2., 2.)]);
    // A parent placed again keeps its children.
    s.feed("\x1b[1;1H\x1b_Ga=p,i=1,p=1,C=1,q=2\x1b\\");
    assert_eq!(clips(&s), [(0., 0., 2., 2.), (3., 1., 2., 2.)]);
}

#[test]
fn a_parent_must_be_there_and_no_chain_loops() {
    let mut s = screen();
    s.transmit(2, "");
    s.feed("\x1b_Ga=p,i=1,p=1,C=1,q=1\x1b\\");
    let failed = |keys: &str, message: &str| format!("\x1b_G{keys};{message}\x1b\\");
    for (keys, message) in [
        ("i=1,p=2,P=9", "ENOPARENT: parent image not found"),
        ("i=1,p=2,P=1,Q=9", "ENOPARENT: parent placement not found"),
        (
            "i=1,p=2,P=1,U=1",
            "EINVAL: virtual placement cannot refer to a parent",
        ),
        (
            "i=1,p=1,P=1,Q=1",
            "EINVAL: placement cannot be its own parent",
        ),
    ] {
        s.feed(&format!("\x1b_Ga=p,{keys}\x1b\\"));
        assert_eq!(s.heard(), [failed(&keys[..7], message)], "{keys}");
    }
    s.feed("\x1b_Ga=p,i=2,p=1,P=1,Q=1,q=1\x1b\\\x1b_Ga=p,i=1,p=1,P=2,Q=1\x1b\\");
    let cycle = "ECYCLE: parent chain creates a cycle";
    assert_eq!(s.heard(), [failed("i=1,p=1", cycle)]);
    // A chain holds eight links; a ninth is too deep.
    for link in 2..=10 {
        s.feed(&format!(
            "\x1b_Ga=p,i=1,p={link},P=1,Q={},q=1\x1b\\",
            link - 1
        ));
    }
    assert_eq!(
        s.heard(),
        [failed("i=1,p=10", "ETOODEEP: parent chain too deep")]
    );
}

#[test]
fn a_parent_s_delete_takes_its_children_and_capitals_their_images() {
    for (what, kept) in [("i", true), ("I", false)] {
        let mut s = screen();
        s.transmit(2, "");
        s.feed("\x1b_Ga=p,i=1,p=1,C=1\x1b\\\x1b_Ga=p,i=2,p=1,P=1,Q=1\x1b\\");
        s.feed("\x1b_Ga=p,i=2,p=2,P=2,Q=1\x1b\\");
        s.feed(&format!("\x1b_Ga=d,d={what},i=1,p=1\x1b\\"));
        assert!(s.placed().is_empty(), "{what}");
        assert_eq!((s.stored(1), s.stored(2)), (kept, kept), "{what}");
    }
}

#[test]
fn placeholder_cells_read_as_runs_by_kitty_s_rules() {
    let mut s = screen();
    // Row and column marks; then a row mark others continue.
    let first = "\u{10EEEE}\u{305}\u{305}\u{10EEEE}\u{305}\u{30D}";
    let second = "\u{10EEEE}\u{30D}\u{10EEEE}\u{10EEEE}\u{10EEEE}";
    // A third mark: the id's high byte; 58: the placement.
    let high = "\x1b[58;5;7m\u{10EEEE}\u{305}\u{305}\u{30E}\u{10EEEE}";
    s.feed(&format!(
        "\x1b[38;5;1m\x1b[2;3H{first}\x1b[3;3H{second}\x1b[5;1H{high}"
    ));
    let run = |row, column, image, placement, at, width| Run {
        row,
        column,
        image,
        placement,
        at,
        width,
    };
    assert_eq!(
        runs(&s.term),
        [
            run(1, 2, 1, 0, (0, 0), 2),
            run(2, 2, 1, 0, (1, 0), 4),
            run(4, 0, 1 | 2 << 24, 7, (0, 0), 2),
        ]
    );
}

#[test]
fn a_virtual_placement_shows_fitted_in_its_placeholders() {
    let mut s = screen();
    s.feed("\x1b_Ga=p,U=1,i=1,p=5,c=4,r=2\x1b\\");
    assert_eq!(s.heard(), [ok("i=1,p=5")]);
    assert_eq!(s.cursor(), (0, 0));
    assert!(clips(&s).is_empty(), "nothing shows without placeholders");
    let first = "\u{10EEEE}\u{305}\u{305}\u{10EEEE}";
    s.feed(&format!(
        "\x1b[38;5;1m\x1b[2;3H{first}\x1b[3;3H\u{10EEEE}\u{30D}\u{10EEEE}"
    ));
    assert_eq!(clips(&s), [(2., 1., 2., 1.), (2., 2., 2., 1.)]);
    let mut pictures = Pictures::default();
    let (shown, _) = pictures.shown(&s.shared.lock().unwrap(), &runs(&s.term), 0, (10, 5));
    // 2 by 4 in 4 by 2: height fits, centred.
    let whole = shown[0].whole;
    let at = (
        whole.origin.x,
        whole.origin.y,
        whole.size.width,
        whole.size.height,
    );
    assert_eq!(at, (3., 1., 2., 2.));
    // A relative placement stands at the placeholders' top-left.
    s.transmit(2, "");
    s.feed("\x1b_Ga=p,i=2,p=1,P=1,Q=5,H=1,V=1\x1b\\");
    assert_eq!(clips(&s)[2], (3., 2., 2., 2.));
}

#[test]
fn a_virtual_placement_goes_only_by_its_image() {
    let mut s = screen();
    s.feed("\x1b_Ga=p,U=1,i=1,c=4,r=2,q=2\x1b\\");
    for delete in [
        "d=a",
        "d=A",
        "d=z,z=0",
        "d=c",
        "d=p,x=1,y=1",
        "d=x,x=1",
        "d=y,y=1",
    ] {
        s.feed(&format!("\x1b_Ga=d,{delete}\x1b\\"));
        assert_eq!(s.placed().len(), 1, "{delete}");
    }
    s.feed("\x1b_Ga=d,d=i,i=1\x1b\\");
    assert!(s.placed().is_empty());
}

#[test]
fn a_relative_placement_goes_by_its_z_or_its_parent_alone() {
    let mut s = screen();
    s.transmit(2, "");
    // Children: off a virtual parent, a negative column, a positive.
    s.feed("\x1b_Ga=p,i=1,p=1,U=1,c=1,r=1,q=2\x1b\\\x1b[38;5;1m\x1b[1;1H\u{10EEEE}");
    s.feed("\x1b[2;3H\x1b_Ga=p,i=1,p=2,C=1,q=2\x1b\\");
    s.feed("\x1b_Ga=p,i=2,p=1,P=1,Q=1,H=2,c=1,r=1,z=-1,q=2\x1b\\");
    s.feed("\x1b_Ga=p,i=2,p=2,P=1,Q=2,H=-3,c=2,r=1,q=2\x1b\\");
    s.feed("\x1b_Ga=p,i=2,p=3,P=1,Q=2,H=3,c=2,r=1,q=2\x1b\\");
    let relative = |s: &Screen| s.placed().iter().filter(|placed| placed.0 == 2).count();
    // Each hits a child's cells alone, never a parent's.
    for delete in [
        "d=p,x=1,y=2",
        "d=p,x=3,y=1",
        "d=x,x=1",
        "d=y,y=1",
        "d=c",
        "d=p,x=6,y=2",
        "d=x,x=7",
    ] {
        s.feed(&format!("\x1b[1;3H\x1b_Ga=d,{delete}\x1b\\"));
    }
    assert_eq!(relative(&s), 3, "spatial deletes pass relatives by");
    // The grid parent and its children go; the virtual stays.
    s.feed("\x1b_Ga=d\x1b\\");
    assert_eq!(relative(&s), 1);
    s.feed("\x1b_Ga=d,d=z,z=-1\x1b\\");
    assert_eq!(relative(&s), 0);
}
