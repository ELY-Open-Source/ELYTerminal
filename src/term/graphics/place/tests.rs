use std::sync::{Arc, Mutex};

use alacritty_terminal::{
    event::{Event, EventListener},
    index::{Column, Line, Point},
    term::{Config, Term, test::TermSize},
    vte::ansi::Processor,
};
use base64::{Engine, engine::general_purpose::STANDARD};

use super::super::{Graphics, Layer, Pictures, Shared, runs};

#[derive(Clone, Default)]
pub(in crate::term::graphics) struct Replies(Arc<Mutex<Vec<String>>>);

impl EventListener for Replies {
    fn send_event(&self, event: Event) {
        if let Event::PtyWrite(text) = event {
            self.0.lock().unwrap().push(text);
        }
    }
}

pub(in crate::term::graphics) type Placed = (u32, u32, i32, usize, (u32, u32));

pub(in crate::term::graphics) struct Screen {
    pub term: Term<Replies>,
    parser: Processor,
    replies: Replies,
    pub shared: Arc<Mutex<Shared>>,
}

/// Five lines of ten 10 by 20 cells; image 1.
pub(in crate::term::graphics) fn screen() -> Screen {
    let replies = Replies::default();
    let mut term = Term::new(Config::default(), &TermSize::new(10, 5), replies.clone());
    let graphics = Graphics::new(0, (10, 5), (10, 20));
    let shared = graphics.shared();
    term.set_graphics(Box::new(graphics));
    let mut screen = Screen {
        term,
        parser: Processor::new(),
        replies,
        shared,
    };
    screen.transmit(1, "");
    screen
}

impl Screen {
    pub fn feed(&mut self, text: &str) {
        self.parser.advance(&mut self.term, text.as_bytes());
    }

    pub fn transmit(&mut self, id: u32, keys: &str) {
        let pixels = STANDARD.encode([0u8; 20 * 40 * 4]);
        self.feed(&format!("\x1b_Gi={id},s=20,v=40,q=2{keys};{pixels}\x1b\\"));
    }

    pub fn heard(&self) -> Vec<String> {
        std::mem::take(&mut *self.replies.0.lock().unwrap())
    }

    pub fn cursor(&self) -> (i32, usize) {
        let point = self.term.grid().cursor.point;
        (point.line.0, point.column.0)
    }

    /// Each placement's image, id, line, column and cuts.
    pub fn placed(&self) -> Vec<Placed> {
        let mut shared = self.shared.lock().unwrap();
        let screen = shared.screen();
        let each = |kept: &super::Placement| {
            let [_, y, _, h] = kept.display.source;
            (
                kept.image,
                kept.id,
                kept.line,
                kept.column,
                (y as u32, h as u32),
            )
        };
        screen.placements.iter().map(each).collect()
    }

    pub fn stored(&self, id: u32) -> bool {
        self.shared.lock().unwrap().screen().store.get(id).is_some()
    }
}

#[test]
fn a_placement_stands_at_the_cursor_and_the_cursor_moves_past_it() {
    let mut s = screen();
    s.feed("\x1b[2;3H\x1b_Ga=p,i=1\x1b\\");
    assert_eq!(s.heard(), ["\x1b_Gi=1;OK\x1b\\"]);
    assert_eq!(s.placed(), [(1, 0, 1, 2, (0, 0))]);
    // Down a row, past two columns.
    assert_eq!(s.cursor(), (2, 4));
    // C=1 leaves the cursor; a placement id replaces its own.
    s.feed("\x1b_Ga=p,i=1,p=7,C=1\x1b\\\x1b[1;1H\x1b_Ga=p,i=1,p=7,C=1\x1b\\");
    assert_eq!(s.cursor(), (0, 0));
    assert_eq!(s.heard(), ["\x1b_Gi=1,p=7;OK\x1b\\"; 2]);
    assert_eq!(s.placed(), [(1, 0, 1, 2, (0, 0)), (1, 7, 0, 0, (0, 0))]);
    // Past the right edge, it wraps to a line's start.
    s.feed("\x1b[1;9H\x1b_Ga=p,i=1,q=1\x1b\\");
    assert_eq!(s.cursor(), (2, 0));
    // Columns and rows given; a=T transmits and places.
    let rgb = STANDARD.encode([1u8, 2, 3]);
    s.feed(&format!(
        "\x1b[4;1H\x1b_Ga=T,i=2,f=24,s=1,v=1,c=3,r=1;{rgb}\x1b\\"
    ));
    assert_eq!(s.heard(), ["\x1b_Gi=2;OK\x1b\\"]);
    assert_eq!(s.cursor(), (3, 3));
}

#[test]
fn a_placement_names_an_image_that_is_there() {
    let mut s = screen();
    s.feed("\x1b_Ga=p\x1b\\\x1b_Ga=p,i=9\x1b\\\x1b_Ga=p,I=4\x1b\\");
    let none = "ENOENT: image not found";
    let failed = [
        format!("\x1b_Gi=9;{none}\x1b\\"),
        format!("\x1b_GI=4;{none}\x1b\\"),
    ];
    assert_eq!(s.heard(), failed);
    s.feed(&format!(
        "\x1b_GI=4,s=1,v=1;{}\x1b\\",
        STANDARD.encode([0u8; 4])
    ));
    s.feed("\x1b_Ga=p,I=4,C=1\x1b\\");
    assert_eq!(s.heard(), ["\x1b_Gi=2,I=4;OK\x1b\\"; 2]);
    assert!(s.placed().iter().any(|placed| placed.0 == 2));
}

#[test]
fn scrolls_move_placements_and_a_region_cuts_them() {
    let mut s = screen();
    s.feed("\x1b[4;1H\x1b_Ga=p,i=1,C=1\x1b\\\x1b[5;1H\n");
    assert_eq!(s.placed(), [(1, 0, 2, 0, (0, 0))]);
    s.feed("\n\n\n");
    assert_eq!(s.placed(), [(1, 0, -1, 0, (0, 0))]);
    let mut s = screen();
    // One inside rows 2 to 4, one past its bottom.
    s.feed("\x1b[2;4r\x1b[3;1H\x1b_Ga=p,i=1,p=1,C=1\x1b\\\x1b[4;5H\x1b_Ga=p,i=1,p=2,C=1\x1b\\");
    s.feed("\x1b[4;1H\n");
    assert_eq!(s.placed(), [(1, 1, 1, 0, (0, 0)), (1, 2, 3, 4, (0, 0))]);
    s.feed("\n");
    // The top row went; the source: y 20, 20 high.
    assert_eq!(s.placed(), [(1, 1, 1, 0, (20, 20)), (1, 2, 3, 4, (0, 0))]);
    s.feed("\n");
    assert_eq!(s.placed(), [(1, 2, 3, 4, (0, 0))]);
}

#[test]
fn a_cut_stays_in_source_pixels_whatever_the_cell() {
    let mut s = screen();
    s.feed("\x1b[2;4r\x1b[2;1H\x1b_Ga=p,i=1,C=1\x1b\\\x1b[4;1H\n");
    assert_eq!(s.placed(), [(1, 0, 1, 0, (20, 20))]);
    s.shared.lock().unwrap().cell = (5, 10);
    s.term.resize(TermSize::new(10, 6));
    assert_eq!(s.placed(), [(1, 0, 1, 0, (20, 20))]);
    let mut pictures = Pictures::default();
    let (shown, _) = pictures.shown(&s.shared.lock().unwrap(), &runs(&s.term), 0, (10, 6));
    // Twenty source rows show in two 10-pixel rows.
    assert_eq!(
        (shown[0].clip.size.height, shown[0].whole.origin.y),
        (2., -1.)
    );
}

#[test]
fn a_cut_keeps_the_part_of_a_source_pixel_a_row_shows() {
    let mut s = screen();
    let white = STANDARD.encode([255u8; 4]);
    s.feed(&format!(
        "\x1b[2;4r\x1b[3;1H\x1b_Ga=T,i=2,s=1,v=1,c=2,r=2,C=1;{white}\x1b\\"
    ));
    s.feed("\x1b[2;1H\x1bM");
    let mut pictures = Pictures::default();
    let (shown, _) = pictures.shown(&s.shared.lock().unwrap(), &runs(&s.term), 0, (10, 5));
    // Half the pixel shows in line 3; half is cut.
    let drawn = (
        shown[0].clip.origin.y,
        shown[0].clip.size.height,
        shown[0].whole.size.height,
    );
    assert_eq!(drawn, (3., 1., 2.));
}

#[test]
fn clears_drop_placements_and_ed_2_moves_the_history_s() {
    let mut s = screen();
    s.feed("\x1b_Ga=p,i=1,p=1,C=1\x1b\\\x1b[5;1H\n\n");
    s.feed("\x1b[2;1H\x1b_Ga=p,i=1,p=2,C=1\x1b\\");
    assert_eq!(s.placed(), [(1, 1, -2, 0, (0, 0)), (1, 2, 1, 0, (0, 0))]);
    // Text on the last line scrolls all five lines away.
    s.feed("\x1b[5;1Hx\x1b[2J");
    assert_eq!(s.placed(), [(1, 1, -7, 0, (0, 0))]);
    s.feed("\x1b[3J");
    assert!(s.placed().is_empty());
    assert!(s.stored(1), "a clear keeps the image");
    s.feed("\x1b_Ga=p,i=1\x1b\\\x1bc");
    assert!(s.placed().is_empty());
}

#[test]
fn the_alternate_screen_keeps_its_own_images_and_starts_empty() {
    let mut s = screen();
    s.feed("\x1b_Ga=p,i=1,C=1\x1b\\\x1b[?1049h");
    assert!(s.placed().is_empty());
    s.feed("\x1b_Ga=p,i=1\x1b\\");
    assert_eq!(
        s.heard(),
        [
            "\x1b_Gi=1;OK\x1b\\",
            "\x1b_Gi=1;ENOENT: image not found\x1b\\"
        ]
    );
    s.transmit(3, ",a=T,C=1");
    assert_eq!(s.placed().len(), 1);
    s.feed("\x1b[?1049l");
    assert_eq!(s.placed(), [(1, 0, 0, 0, (0, 0))]);
    s.feed("\x1b[?1049h");
    assert!(s.placed().is_empty() && !s.stored(3));
}

#[test]
fn a_scroll_it_starts_moves_it_and_a_region_from_the_top_cuts_it() {
    let mut s = screen();
    s.feed("\x1b[5;1H\x1b_Ga=p,i=1\x1b\\");
    assert_eq!(s.placed(), [(1, 0, 3, 0, (0, 0))]);
    let mut s = screen();
    s.feed("\x1b[1;4r\x1b_Ga=p,i=1,C=1\x1b\\\x1b[4;1H\n");
    assert_eq!(s.placed(), [(1, 0, 0, 0, (20, 20))]);
}

#[test]
fn a_reflow_keeps_a_placement_on_its_text() {
    for text in ["abcdef", "abcd\u{754c}"] {
        let mut s = screen();
        s.feed(&format!("{text}\x1b_Ga=p,i=1,C=1\x1b\\X"));
        for size in [TermSize::new(5, 5), TermSize::new(10, 5)] {
            s.term.resize(size);
            let (_, _, line, column, _) = s.placed()[0];
            let point = Point::new(Line(line), Column(column));
            assert_eq!(s.term.grid()[point].c, 'X', "{text} {point:?}");
        }
    }
}

#[test]
fn a_resize_keeps_a_placement_its_screen_and_history_still_hold() {
    let mut s = screen();
    s.feed("\x1b_Ga=p,i=1,C=1\x1b\\X\x1b[5;1HY\x1b[1;1H");
    s.term.resize(TermSize::new(10, 3));
    assert_eq!(s.placed(), [(1, 0, 0, 0, (0, 0))]);
    let replies = Replies::default();
    let config = Config {
        scrolling_history: 3,
        ..Config::default()
    };
    let mut term = Term::new(config, &TermSize::new(10, 5), replies.clone());
    let graphics = Graphics::new(0, (10, 5), (10, 20));
    let shared = graphics.shared();
    term.set_graphics(Box::new(graphics));
    let mut s = Screen {
        term,
        parser: Processor::new(),
        replies,
        shared,
    };
    s.transmit(1, "");
    s.feed(&format!("{}\x1b_Ga=p,i=1,C=1\x1b\\X", "a".repeat(36)));
    s.term.resize(TermSize::new(5, 5));
    let (_, _, line, column, _) = s.placed()[0];
    assert_eq!(s.term.grid()[Point::new(Line(line), Column(column))].c, 'X');
}

#[test]
fn a_placement_the_history_s_clear_cut_keeps_its_rows_through_a_resize() {
    let mut s = screen();
    s.feed("\x1b_Ga=p,i=1,C=1\x1b\\\x1b[5;1H\n\x1b[3J");
    assert_eq!(s.placed(), [(1, 0, 0, 0, (20, 20))]);
    s.term.resize(TermSize::new(10, 6));
    assert_eq!(s.placed(), [(1, 0, 0, 0, (20, 20))]);
}

#[test]
fn a_resize_moves_placements_with_the_text() {
    let mut s = screen();
    s.feed("\x1b[5;1H\x1b_Ga=p,i=1,C=1\x1b\\");
    s.term.resize(TermSize::new(10, 3));
    assert_eq!(s.placed(), [(1, 0, 2, 0, (0, 0))]);
}

#[test]
fn deletes_pick_placements_and_capitals_free_their_images() {
    let mut s = screen();
    let place = "\x1b[1;1H\x1b_Ga=p,i=1,p=1,C=1\x1b\\\x1b[4;6H\x1b_Ga=p,i=1,p=2,z=-1,C=1\x1b\\";
    s.feed(&format!("{place}\x1b[2;9H\x1b_Ga=p,i=1,p=3,C=1\x1b\\"));
    s.feed("\x1b_Ga=d,d=p,x=6,y=4\x1b\\");
    let ids = |s: &Screen| s.placed().iter().map(|placed| placed.1).collect::<Vec<_>>();
    assert_eq!(ids(&s), [1, 3]);
    s.feed("\x1b_Ga=d,d=x,x=10\x1b\\");
    assert_eq!(ids(&s), [1]);
    s.feed("\x1b_Ga=d,d=y,y=2\x1b\\");
    assert!(ids(&s).is_empty());
    s.feed(&format!("{place}\x1b_Ga=d,d=z,z=-1\x1b\\"));
    assert_eq!(ids(&s), [1]);
    s.feed("\x1b[1;2H\x1b_Ga=d,d=c\x1b\\");
    assert!(ids(&s).is_empty());
    s.feed(&format!("{place}\x1b_Ga=d\x1b\\"));
    assert!(
        ids(&s).is_empty() && s.stored(1),
        "lower case keeps the image"
    );
    s.feed(&format!("{place}\x1b_Ga=d,d=I,i=1,p=2\x1b\\"));
    assert_eq!(ids(&s), [1]);
    assert!(s.stored(1), "a placement left keeps it");
    s.feed("\x1b_Ga=d,d=A\x1b\\");
    assert!(ids(&s).is_empty() && !s.stored(1));
}

#[test]
fn drawing_takes_the_layer_the_cuts_the_view_and_the_screen() {
    let mut s = screen();
    for (column, z) in [(1, ""), (4, ",z=-1"), (10, ",z=-1073741825")] {
        s.feed(&format!("\x1b[2;{column}H\x1b_Ga=p,i=1,C=1{z}\x1b\\"));
    }
    let mut pictures = Pictures::default();
    let (shown, stale) = pictures.shown(&s.shared.lock().unwrap(), &runs(&s.term), 0, (10, 5));
    let layers: Vec<Layer> = shown.iter().map(|shown| shown.layer).collect();
    assert_eq!(layers, [Layer::Under, Layer::Between, Layer::Over]);
    assert!(stale.is_empty());
    let clips: Vec<_> = shown
        .iter()
        .map(|shown| (shown.clip.origin.x, shown.clip.size.width))
        .collect();
    // The lowest starts on the last column; its second goes.
    assert_eq!(clips, [(9., 1.), (3., 2.), (0., 2.)]);
    // Scrolled back a line, it shows a line lower.
    let (shown, _) = pictures.shown(&s.shared.lock().unwrap(), &runs(&s.term), 1, (10, 5));
    assert_eq!(shown[0].clip.origin.y, 2.);
    s.feed("\x1b_Ga=d,d=A\x1b\\");
    let (shown, stale) = pictures.shown(&s.shared.lock().unwrap(), &runs(&s.term), 0, (10, 5));
    assert!(shown.is_empty());
    assert_eq!(stale.len(), 1, "its texture goes");
}
