use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ObjectFit, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, Window, div,
};

use super::viewer::DocPage;
use crate::{
    buttons::{Button, ButtonVariant, SegmentedControl},
    documents::blocks::source,
    forms::{Choice, NumberInput, Run, Select},
    primitives::Image,
    theme::{ActiveTheme, ControlSize, Elevation, Radius, TextSize},
    typography::{format::plural, tabular},
};

/// Sheets the app can print on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Paper {
    #[default]
    Letter,
    A4,
    Legal,
}

/// Space kept around what prints.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Margins {
    None,
    Narrow,
    #[default]
    Normal,
}

/// How a print goes: paper, turn, margins and copies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrintSettings {
    pub paper: Paper,
    pub landscape: bool,
    pub margins: Margins,
    pub copies: u32,
}

impl Default for PrintSettings {
    fn default() -> Self {
        Self {
            paper: Paper::default(),
            landscape: false,
            margins: Margins::default(),
            copies: 1,
        }
    }
}

const PAPERS: [(Paper, &str); 3] = [
    (Paper::Letter, "Letter"),
    (Paper::A4, "A4"),
    (Paper::Legal, "Legal"),
];
const MARGINS: [(Margins, &str); 3] = [
    (Margins::None, "None"),
    (Margins::Narrow, "Narrow"),
    (Margins::Normal, "Normal"),
];

/// Most copies one print makes.
const MOST_COPIES: f64 = 99.0;

type OnSettings = Rc<dyn Fn(PrintSettings, &mut Window, &mut App)>;

/// A print before it goes: the pages the app laid out for the settings, small on a desk, and the settings beside, paper, turn, margins and copies; each change asks the app for new pages.
#[derive(IntoElement)]
pub struct PrintPreview {
    id: ElementId,
    pages: Rc<Vec<DocPage>>,
    settings: PrintSettings,
    on_change: OnSettings,
    on_print: OnSettings,
    on_cancel: Run,
}

impl PrintPreview {
    pub fn new(
        id: impl Into<ElementId>,
        pages: impl Into<Rc<Vec<DocPage>>>,
        settings: PrintSettings,
        on_change: impl Fn(PrintSettings, &mut Window, &mut App) + 'static,
        on_print: impl Fn(PrintSettings, &mut Window, &mut App) + 'static,
        on_cancel: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            pages: pages.into(),
            settings,
            on_change: Rc::new(on_change),
            on_print: Rc::new(on_print),
            on_cancel: Rc::new(on_cancel),
        }
    }
}

fn row(label: &'static str, control: impl IntoElement, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_size(cx.theme().text_size(TextSize::Sm))
                .text_color(cx.theme().colors.fg_muted)
                .child(label),
        )
        .child(control)
}

impl RenderOnce for PrintPreview {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let settings = self.settings;
        let paper = {
            let on_change = self.on_change.clone();
            Select::new(
                (self.id.clone(), "paper"),
                PAPERS.map(|(_, name)| Choice::new(name, name)),
            )
            .selected(
                PAPERS
                    .iter()
                    .find(|(paper, _)| *paper == settings.paper)
                    .expect("a known paper")
                    .1,
            )
            .size(ControlSize::Sm)
            .on_change(move |name, window, cx| {
                let (paper, _) = PAPERS
                    .into_iter()
                    .find(|(_, named)| *named == name.as_ref())
                    .expect("a listed paper");
                on_change(PrintSettings { paper, ..settings }, window, cx);
            })
        };
        let turn = {
            let on_change = self.on_change.clone();
            SegmentedControl::new(
                (self.id.clone(), "turn"),
                if settings.landscape {
                    "landscape"
                } else {
                    "portrait"
                },
            )
            .size(ControlSize::Sm)
            .segment("portrait", "Portrait", None)
            .segment("landscape", "Landscape", None)
            .on_change(move |value, window, cx| {
                on_change(
                    PrintSettings {
                        landscape: value.as_ref() == "landscape",
                        ..settings
                    },
                    window,
                    cx,
                )
            })
        };
        let margins = {
            let on_change = self.on_change.clone();
            Select::new(
                (self.id.clone(), "margins"),
                MARGINS.map(|(_, name)| Choice::new(name, name)),
            )
            .selected(
                MARGINS
                    .iter()
                    .find(|(margins, _)| *margins == settings.margins)
                    .expect("known margins")
                    .1,
            )
            .size(ControlSize::Sm)
            .on_change(move |name, window, cx| {
                let (margins, _) = MARGINS
                    .into_iter()
                    .find(|(_, named)| *named == name.as_ref())
                    .expect("listed margins");
                on_change(
                    PrintSettings {
                        margins,
                        ..settings
                    },
                    window,
                    cx,
                );
            })
        };
        let copies = {
            let on_change = self.on_change.clone();
            NumberInput::new((self.id.clone(), "copies"), settings.copies as f64)
                .range(1.0, MOST_COPIES)
                .step(1.0)
                .size(ControlSize::Sm)
                .on_change(move |copies, window, cx| {
                    on_change(
                        PrintSettings {
                            copies: copies as u32,
                            ..settings
                        },
                        window,
                        cx,
                    )
                })
        };
        let (print, cancel) = (self.on_print.clone(), self.on_cancel.clone());
        let thumb = theme.label_width();
        div()
            .flex()
            .size_full()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .overflow_hidden()
            .child(
                div()
                    .id((self.id.clone(), "desk"))
                    .flex_1()
                    .min_w_0()
                    .overflow_y_scroll()
                    .bg(colors.sunken)
                    .p_6()
                    .flex()
                    .flex_wrap()
                    .content_start()
                    .gap_4()
                    .children(self.pages.iter().enumerate().map(|(ix, page)| {
                        let (width, height) = sheet(settings);
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .w(thumb)
                                    .h(thumb * (height / width))
                                    .bg(colors.paper)
                                    .shadow(theme.elevation(Elevation::Raised))
                                    .p(thumb * margin(settings.margins))
                                    .child(
                                        Image::new(
                                            (self.id.clone(), format!("sheet-{ix}")),
                                            source(&page.source),
                                        )
                                        .fit(ObjectFit::Contain)
                                        .size_full(),
                                    ),
                            )
                            .child(
                                tabular(
                                    div()
                                        .text_size(theme.text_size(TextSize::Xs))
                                        .text_color(colors.fg_subtle),
                                )
                                .child(format!("{}", ix + 1)),
                            )
                    })),
            )
            .child(
                div()
                    .flex_none()
                    .w(theme.sidebar_width(false))
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p_4()
                    .border_l_1()
                    .border_color(colors.border)
                    .bg(colors.surface)
                    .child(div().text_color(colors.fg_muted).child(format!(
                        "{} · {}",
                        plural(self.pages.len() as u64, "page", "pages"),
                        plural(settings.copies as u64, "copy", "copies")
                    )))
                    .child(row("Paper", paper, cx))
                    .child(row("Turn", turn, cx))
                    .child(row("Margins", margins, cx))
                    .child(row("Copies", copies, cx))
                    .child(div().flex_1())
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new((self.id.clone(), "cancel"), "Cancel")
                                    .variant(ButtonVariant::Ghost)
                                    .on_click(move |_, window, cx| cancel(window, cx)),
                            )
                            .child(
                                Button::new((self.id.clone(), "print"), "Print")
                                    .variant(ButtonVariant::Primary)
                                    .on_click(move |_, window, cx| {
                                        log::info!("print preview: print {settings:?}");
                                        print(settings, window, cx)
                                    }),
                            ),
                    ),
            )
    }
}

/// The share of a sheet's width each margin keeps.
fn margin(margins: Margins) -> f32 {
    match margins {
        Margins::None => 0.0,
        Margins::Narrow => 0.04,
        Margins::Normal => 0.08,
    }
}

/// The paper's size in points, turned when landscape, for the app laying out pages.
pub fn sheet(settings: PrintSettings) -> (f32, f32) {
    let (width, height) = match settings.paper {
        Paper::Letter => (612.0, 792.0),
        Paper::A4 => (595.0, 842.0),
        Paper::Legal => (612.0, 1008.0),
    };
    if settings.landscape {
        (height, width)
    } else {
        (width, height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sheet_turns_on_its_side() {
        let settings = PrintSettings {
            paper: Paper::A4,
            landscape: true,
            ..PrintSettings::default()
        };
        assert_eq!(sheet(settings), (842.0, 595.0));
        assert_eq!(sheet(PrintSettings::default()), (612.0, 792.0));
    }
}
