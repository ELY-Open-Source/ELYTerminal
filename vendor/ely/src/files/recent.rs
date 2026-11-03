use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::*,
};
use jiff::{Timestamp, civil::Date, tz::TimeZone};

use super::FileIcon;
use crate::{
    lists::{ListItem, SelectableList},
    theme::{ActiveTheme, TextSize},
    typography::format::{datetime, system_zone},
};

type OnRecent = Rc<dyn Fn(&RecentFile, &mut Window, &mut App)>;

/// A file opened lately: its name, the folder it lives in, and when it was opened.
#[derive(Clone, Debug, PartialEq)]
pub struct RecentFile {
    pub name: SharedString,
    pub folder: SharedString,
    pub opened: Timestamp,
}

impl RecentFile {
    /// Where it lives: its folder and name.
    fn place(&self) -> SharedString {
        format!("{}/{}", self.folder, self.name).into()
    }
}

/// The header over a day's files: Today, Yesterday, or the weekday and date.
pub(crate) fn day_label(day: Date, today: Date) -> String {
    if day == today {
        "Today".into()
    } else if today.yesterday().ok() == Some(day) {
        "Yesterday".into()
    } else {
        day.strftime("%A, %B %-d").to_string()
    }
}

/// Files opened lately, newest first, under a header for each day: Today, Yesterday, then the weekday and date. Each day is a list; Tab moves between days, and Enter or a double press opens a file.
#[derive(IntoElement)]
pub struct RecentFiles {
    id: ElementId,
    files: Vec<RecentFile>,
    zone: Option<TimeZone>,
    on_open: Option<OnRecent>,
}

impl RecentFiles {
    pub fn new(id: impl Into<ElementId>, files: impl IntoIterator<Item = RecentFile>) -> Self {
        let files: Vec<RecentFile> = files.into_iter().collect();
        for (ix, file) in files.iter().enumerate() {
            let place = file.place();
            assert!(
                !files[..ix].iter().any(|other| other.place() == place),
                "{place} twice"
            );
        }
        Self {
            id: id.into(),
            files,
            zone: None,
            on_open: None,
        }
    }

    /// The zone days turn in; the system's by default.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    pub fn on_open(
        mut self,
        handler: impl Fn(&RecentFile, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for RecentFiles {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let zone = self.zone.unwrap_or_else(|| system_zone("recent files"));
        let today = Timestamp::now().to_zoned(zone.clone()).date();
        let mut files = self.files;
        files.sort_by_key(|file| std::cmp::Reverse(file.opened));
        let files: Rc<[RecentFile]> = files.into();
        let theme = cx.theme();
        let colors = &theme.colors;
        let mut days: Vec<(Date, Vec<usize>)> = Vec::new();
        for (ix, file) in files.iter().enumerate() {
            let day = file.opened.to_zoned(zone.clone()).date();
            match days.last_mut() {
                Some((last, those)) if *last == day => those.push(ix),
                _ => days.push((day, vec![ix])),
            }
        }
        let groups = days.into_iter().map(|(day, those)| {
            let list = those.iter().fold(
                SelectableList::new((self.id.clone(), format!("day-{day}"))),
                |list, ix| {
                    let file = &files[*ix];
                    let time = datetime(file.opened, &zone, "%H:%M").expect("a time formats");
                    let row =
                        ListItem::new((self.id.clone(), format!("recent-{ix}")), file.name.clone())
                            .leading(FileIcon::file(&file.name))
                            .description(file.folder.clone())
                            .trailing(time);
                    list.row(file.place(), row)
                },
            );
            let (id, files, on_open) = (self.id.clone(), files.clone(), self.on_open.clone());
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .px_3()
                        .text_size(theme.text_size(TextSize::Sm))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(colors.fg_muted)
                        .child(day_label(day, today)),
                )
                .child(list.on_activate(move |key, window, cx| {
                    let file = files
                        .iter()
                        .find(|file| file.place() == *key)
                        .expect("a listed file");
                    log::info!("recent files {id:?}: open {}", file.name);
                    if let Some(on_open) = &on_open {
                        on_open(file, window, cx);
                    }
                }))
        });
        div()
            .debug_selector(|| "recent-files".into())
            .flex()
            .flex_col()
            .gap_4()
            .children(groups)
    }
}
