use std::{cell::Cell, rc::Rc};

use gpui::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
};

use crate::{
    buttons::{Button, ButtonVariant},
    forms::{RadioCard, Run},
    overlays::Dialog,
    primitives::IconName,
};

type OnExport = Rc<dyn Fn(ExportFormat, &mut Window, &mut App)>;

/// A way to save data out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Pdf,
    Png,
    Csv,
    Json,
}

impl ExportFormat {
    /// The file extension it takes, without the dot.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Png => "png",
            Self::Csv => "csv",
            Self::Json => "json",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Pdf => "PDF",
            Self::Png => "PNG",
            Self::Csv => "CSV",
            Self::Json => "JSON",
        }
    }

    fn detail(self) -> &'static str {
        match self {
            Self::Pdf => "A document to print or share",
            Self::Png => "A picture of what shows",
            Self::Csv => "Rows for a spreadsheet",
            Self::Json => "Data for other programs",
        }
    }

    fn icon(self) -> IconName {
        match self {
            Self::Pdf => IconName::FileText,
            Self::Png => IconName::FileImage,
            Self::Csv => IconName::FileSpreadsheet,
            Self::Json => IconName::FileJson,
        }
    }
}

/// Saves data out: the owner's formats as cards, the first chosen, then Export or Cancel. Each way out answers once: Export hands the chosen format to the owner, who writes the file; Cancel, Escape and a press on the scrim cancel. Render it while open.
#[derive(IntoElement)]
pub struct ExportDialog {
    id: ElementId,
    title: SharedString,
    formats: Vec<ExportFormat>,
    on_export: OnExport,
    on_cancel: Run,
}

impl ExportDialog {
    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        formats: impl IntoIterator<Item = ExportFormat>,
        on_export: impl Fn(ExportFormat, &mut Window, &mut App) + 'static,
        on_cancel: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            formats: formats.into_iter().collect(),
            on_export: Rc::new(on_export),
            on_cancel: Rc::new(on_cancel),
        }
    }
}

impl RenderOnce for ExportDialog {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let first = *self
            .formats
            .first()
            .unwrap_or_else(|| panic!("export dialog {id:?} has no formats"));
        let chosen = window.use_keyed_state((id.clone(), "chosen"), cx, move |_, _| first);
        let mut format = *chosen.read(cx);
        if !self.formats.contains(&format) {
            log::error!("export dialog {id:?}: {format:?} left the formats; {first:?} is chosen");
            format = first;
            chosen.update(cx, |chosen, _| *chosen = first);
        }
        let cards = self.formats.iter().map(|&each| {
            let pick = chosen.clone();
            RadioCard::new((id.clone(), each.label()), each == format, each.label())
                .description(each.detail())
                .icon(each.icon())
                .on_select(move |_, cx| {
                    pick.update(cx, |chosen, cx| {
                        *chosen = each;
                        cx.notify();
                    })
                })
        });
        let (cancel, export) = (self.on_cancel, self.on_export);
        let (cancelled, exported) = (id.clone(), id.clone());
        let answered = Rc::new(Cell::new(false));
        let answer = answered.clone();
        Dialog::new(id.clone(), self.title, move |window, cx| {
            if !answered.get() {
                log::info!("export dialog {cancelled:?}: cancelled");
                cancel(window, cx)
            }
        })
        .child(div().flex().flex_col().gap_2().children(cards))
        .action({
            let id = id.clone();
            move |close| {
                Button::new((id, "cancel"), "Cancel")
                    .variant(ButtonVariant::Ghost)
                    .on_click(move |_, window, cx| close(window, cx))
            }
        })
        .action(move |close| {
            Button::new((id, "export"), "Export")
                .primary()
                .on_click(move |_, window, cx| {
                    log::info!("export dialog {exported:?}: {format:?}");
                    answer.set(true);
                    close(window, cx);
                    export(format, window, cx);
                })
        })
    }
}
