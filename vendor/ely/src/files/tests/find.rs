use gpui::{
    AppContext as _, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled,
    TestAppContext, VisualTestContext, Window, div, px,
};
use jiff::{SignedDuration, Timestamp, tz::TimeZone};

use super::{press, settle, setup, then};
use crate::{
    files::{
        ArchiveEntry, ArchiveViewer, DuplicateFinder, Duplicates, FilePreview, FileSearch,
        FoundFile, RecentFile, RecentFiles, archive::saving, duplicates::freed, preview::kind,
        recent::day_label,
    },
    forms::TextInput,
    lists::DirEntry,
};

/// Moves focus to the `nth` Tab stop from none.
fn tab_to(nth: usize, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.blur(cx);
        for _ in 0..nth {
            window.focus_next(cx);
        }
    });
    settle(cx);
}

#[test]
fn a_kind_reads_from_the_extension() {
    assert_eq!(kind(&DirEntry::file("Brief.md", 1, then())), "MD");
    assert_eq!(kind(&DirEntry::file("Makefile", 1, then())), "File");
    assert_eq!(
        kind(&DirEntry::file(".env", 1, then())),
        "ENV",
        "as the icon map reads it"
    );
    assert_eq!(kind(&DirEntry::folder("Design", then())), "Folder");
}

#[test]
fn a_day_reads_as_today_yesterday_or_its_date() {
    let today = jiff::civil::date(2026, 9, 22);
    assert_eq!(day_label(today, today), "Today");
    assert_eq!(
        day_label(jiff::civil::date(2026, 9, 21), today),
        "Yesterday"
    );
    assert_eq!(
        day_label(jiff::civil::date(2026, 9, 20), today),
        "Sunday, September 20"
    );
}

#[test]
fn packing_saves_a_whole_percent_and_marks_free_their_bytes() {
    assert_eq!(
        (saving(1000, 250), saving(0, 0), saving(10, 20)),
        (75, 0, 0)
    );
    let groups = [Duplicates {
        name: "a.jpg".into(),
        size: 100,
        paths: vec!["x/a.jpg".into(), "y/a.jpg".into(), "z/a.jpg".into()],
    }];
    assert_eq!(freed(&groups, &["y/a.jpg".into(), "z/a.jpg".into()]), 200);
}

/// A preview of a picture four wide over three, 400 across.
struct Looking {
    path: std::path::PathBuf,
}

impl Render for Looking {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let entry = DirEntry::file("a.png", 10, then());
        let preview = FilePreview::new("preview", entry).picture(self.path.clone(), 4.0 / 3.0);
        div().w(px(400.0)).child(preview)
    }
}

#[gpui::test]
fn a_picture_keeps_the_hosts_shape(cx: &mut TestAppContext) {
    setup(cx);
    let path = std::env::temp_dir().join("ely-files-preview.png");
    image::RgbaImage::from_pixel(40, 30, image::Rgba([90, 120, 150, 255]))
        .save(&path)
        .expect("the picture writes");
    let (_, cx) = cx.add_window_view(|_, _| Looking { path });
    settle(cx);
    let stage = cx
        .debug_bounds("file-preview-stage")
        .expect("the stage draws");
    let shape = stage.size.width / stage.size.height;
    assert!((shape - 4.0 / 3.0).abs() < 0.02, "{stage:?}");
}

/// Files opened today and yesterday in UTC, and the ones it opened.
struct Recalling {
    opened: Vec<SharedString>,
}

impl Render for Recalling {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        let midnight = Timestamp::now()
            .to_zoned(TimeZone::UTC)
            .start_of_day()
            .expect("a day starts")
            .timestamp();
        let file = |name: &str, minutes: i64| RecentFile {
            name: SharedString::from(name.to_string()),
            folder: "Home".into(),
            opened: midnight + SignedDuration::from_mins(minutes),
        };
        let files = [
            file("early.md", 5),
            file("late.md", -300),
            file("later.md", 9),
        ];
        div().w(px(320.0)).child(
            RecentFiles::new("recent", files)
                .zone(TimeZone::UTC)
                .on_open(move |file, _, cx| {
                    owner.update(cx, |host, _| host.opened.push(file.name.clone()))
                }),
        )
    }
}

#[gpui::test]
fn days_list_the_newest_first_and_enter_opens(cx: &mut TestAppContext) {
    setup(cx);
    let (host, cx) = cx.add_window_view(|_, _| Recalling { opened: Vec::new() });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    tab_to(1, cx);
    press("enter", cx);
    tab_to(2, cx);
    press("enter", cx);
    assert_eq!(
        host.read_with(cx, |host, _| host.opened.clone()),
        ["later.md", "late.md"]
    );
}

/// A file search over a field it owns, finding what the test says, and what it opened.
struct Seeking {
    query: Entity<TextInput>,
    found: Vec<FoundFile>,
    opened: Vec<SharedString>,
}

impl Render for Seeking {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        div().w(px(320.0)).child(
            FileSearch::new("search", &self.query, self.found.clone()).on_open(
                move |file, _, cx| owner.update(cx, |host, _| host.opened.push(file.name.clone())),
            ),
        )
    }
}

fn seeking<'a>(
    text: &str,
    found: Vec<FoundFile>,
    cx: &'a mut TestAppContext,
) -> (Entity<Seeking>, &'a mut VisualTestContext) {
    setup(cx);
    let text = text.to_string();
    let (host, cx) = cx.add_window_view(move |window, cx| {
        let query = cx.new(|cx| {
            let mut field = TextInput::new(window, cx);
            field.set_text(text, cx);
            field
        });
        Seeking {
            query,
            found,
            opened: Vec::new(),
        }
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (host, cx)
}

#[gpui::test]
fn a_search_with_nothing_found_says_so(cx: &mut TestAppContext) {
    let (_, cx) = seeking("zebra", Vec::new(), cx);
    assert!(cx.debug_bounds("file-search-note").is_some());
}

#[gpui::test]
fn a_found_file_opens_on_enter(cx: &mut TestAppContext) {
    let found = vec![FoundFile {
        name: "Brief.md".into(),
        folder: "Home/Atrium".into(),
    }];
    let (host, cx) = seeking("brief", found, cx);
    tab_to(3, cx);
    press("enter", cx);
    assert_eq!(
        host.read_with(cx, |host, _| host.opened.clone()),
        ["Brief.md"]
    );
}

/// Two copies of a file with the second marked, and what it was told.
struct Sparing {
    removed: Vec<SharedString>,
    marks: Vec<(SharedString, bool)>,
}

impl Render for Sparing {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (remove, mark) = (cx.entity(), cx.entity());
        let group = Duplicates {
            name: "a.jpg".into(),
            size: 100,
            paths: vec!["x/a.jpg".into(), "y/a.jpg".into()],
        };
        div().w(px(320.0)).child(
            DuplicateFinder::new("dupes", [group])
                .marked(["y/a.jpg"])
                .on_remove(move |paths, _, cx| {
                    remove.update(cx, |host, _| host.removed.extend(paths.iter().cloned()))
                })
                .on_mark(move |path, on, _, cx| {
                    mark.update(cx, |host, _| host.marks.push((path.clone(), on)))
                }),
        )
    }
}

#[gpui::test]
fn the_last_copy_stays_and_remove_takes_the_marked(cx: &mut TestAppContext) {
    setup(cx);
    let (host, cx) = cx.add_window_view(|_, _| Sparing {
        removed: Vec::new(),
        marks: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    tab_to(1, cx);
    press("space", cx);
    tab_to(2, cx);
    press("space", cx);
    let heard = host.read_with(cx, |host, _| (host.removed.clone(), host.marks.clone()));
    assert_eq!(
        heard,
        (vec!["y/a.jpg".into()], vec![("y/a.jpg".into(), false)]),
        "x/a.jpg is kept, its mark shut"
    );
}

/// An archive of four files in two folders, and the files it handed over.
struct Unpacking {
    opened: Vec<SharedString>,
}

impl Render for Unpacking {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        let entry = |path: &str| ArchiveEntry {
            path: SharedString::from(path.to_string()),
            size: 400,
            packed: 100,
        };
        let entries = ["docs/readme.md", "docs/guide.md", "src/main.rs", "LICENSE"].map(entry);
        div().w(px(320.0)).h(px(320.0)).child(
            ArchiveViewer::new("archive", "site.zip", entries).on_open(move |entry, _, cx| {
                owner.update(cx, |host, _| host.opened.push(entry.path.clone()))
            }),
        )
    }
}

#[gpui::test]
fn a_file_in_an_open_folder_goes_to_the_host_on_enter(cx: &mut TestAppContext) {
    setup(cx);
    let (host, cx) = cx.add_window_view(|_, _| Unpacking { opened: Vec::new() });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    tab_to(1, cx);
    press("down", cx);
    press("enter", cx);
    assert_eq!(
        host.read_with(cx, |host, _| host.opened.clone()),
        ["docs/guide.md"]
    );
}

#[test]
#[should_panic(expected = "LICENSE twice")]
fn an_archive_lists_a_path_once() {
    let entry = ArchiveEntry {
        path: "LICENSE".into(),
        size: 1,
        packed: 1,
    };
    let _ = ArchiveViewer::new("archive", "a.zip", [entry.clone(), entry]);
}

/// Three files opened today; an open moves a file to the top, as a host records it.
struct Reopening {
    files: Vec<RecentFile>,
    opened: Vec<SharedString>,
}

impl Render for Reopening {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        div().w(px(320.0)).child(
            RecentFiles::new("recent", self.files.clone())
                .zone(TimeZone::UTC)
                .on_open(move |file, _, cx| {
                    owner.update(cx, |host, cx| {
                        host.opened.push(file.name.clone());
                        let now = Timestamp::now();
                        for kept in &mut host.files {
                            if kept.name == file.name {
                                kept.opened = now;
                            }
                        }
                        cx.notify();
                    })
                }),
        )
    }
}

#[gpui::test]
fn the_cursor_stays_on_a_file_an_open_moves(cx: &mut TestAppContext) {
    setup(cx);
    let midnight = Timestamp::now()
        .to_zoned(TimeZone::UTC)
        .start_of_day()
        .expect("a day starts")
        .timestamp();
    let file = |name: &str, minutes: i64| RecentFile {
        name: SharedString::from(name.to_string()),
        folder: "Home".into(),
        opened: midnight + SignedDuration::from_mins(minutes),
    };
    let files = vec![file("a.md", 3), file("b.md", 2), file("c.md", 1)];
    let (host, cx) = cx.add_window_view(move |_, _| Reopening {
        files,
        opened: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    tab_to(1, cx);
    press("down", cx);
    press("enter", cx);
    press("enter", cx);
    assert_eq!(
        host.read_with(cx, |host, _| host.opened.clone()),
        ["b.md", "b.md"]
    );
}

#[test]
#[should_panic(expected = "Home/a.md twice")]
fn a_recent_file_is_listed_once() {
    let file = RecentFile {
        name: "a.md".into(),
        folder: "Home".into(),
        opened: then(),
    };
    let _ = RecentFiles::new("recent", [file.clone(), file]);
}

#[gpui::test]
#[should_panic(expected = "Home/a.md twice")]
fn a_found_file_is_listed_once(cx: &mut TestAppContext) {
    let file = FoundFile {
        name: "a.md".into(),
        folder: "Home".into(),
    };
    let _ = seeking("a", vec![file.clone(), file], cx);
}
