use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
};

use super::panel::{OnEdit, editing, heading, pair};
use crate::{
    buttons::{Button, ButtonVariant, IconButton},
    forms::{Choice, Select},
    primitives::IconName,
    theme::{ActiveTheme, TextSize},
    typography::Ellipsis,
};

/// A kind of file to export to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Png,
    Jpg,
    Svg,
    Pdf,
}

impl ExportFormat {
    pub const ALL: [ExportFormat; 4] = [
        ExportFormat::Png,
        ExportFormat::Jpg,
        ExportFormat::Svg,
        ExportFormat::Pdf,
    ];

    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Png => "png",
            ExportFormat::Jpg => "jpg",
            ExportFormat::Svg => "svg",
            ExportFormat::Pdf => "pdf",
        }
    }

    /// Whether it keeps shapes as lines, so no scale applies.
    pub fn vector(self) -> bool {
        matches!(self, ExportFormat::Svg | ExportFormat::Pdf)
    }
}

/// One file to export: a scale and a format.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExportSetting {
    pub scale: f32,
    pub format: ExportFormat,
}

/// The scales a setting offers, in the order plus adds them.
const SCALES: [f32; 5] = [1.0, 2.0, 3.0, 4.0, 0.5];

impl ExportSetting {
    /// The file it writes for a layer called `name`: a scale other than one after an @, then the extension. A vector format takes no scale.
    pub fn file(&self, name: &str) -> String {
        let extension = self.format.extension();
        if self.format.vector() || self.scale == 1.0 {
            format!("{name}.{extension}")
        } else {
            format!("{name}@{}x.{extension}", self.scale)
        }
    }
}

/// The files `settings` write for `name`, each once, in order.
pub fn export_files(name: &str, settings: &[ExportSetting]) -> Vec<String> {
    settings.iter().fold(Vec::new(), |mut files, setting| {
        let file = setting.file(name);
        if !files.contains(&file) {
            files.push(file);
        }
        files
    })
}

/// What plus adds: the first scale no PNG has yet.
fn added(settings: &[ExportSetting]) -> Option<ExportSetting> {
    SCALES
        .iter()
        .find(|scale| {
            !settings
                .iter()
                .any(|setting| setting.format == ExportFormat::Png && setting.scale == **scale)
        })
        .map(|scale| ExportSetting {
            scale: *scale,
            format: ExportFormat::Png,
        })
}

/// What a layer exports to. Each row picks a scale and a format and minus drops it; plus adds the next scale as PNG; Export hands the owner the files, each once, and rests disabled with none.
#[derive(IntoElement)]
pub struct ExportPanel {
    id: ElementId,
    name: SharedString,
    settings: Vec<ExportSetting>,
    on_change: OnEdit<Vec<ExportSetting>>,
    on_export: OnEdit<Vec<String>>,
}

impl ExportPanel {
    /// `name` is the layer's, which the files take.
    pub fn new(
        id: impl Into<ElementId>,
        name: impl Into<SharedString>,
        settings: impl IntoIterator<Item = ExportSetting>,
        on_change: impl Fn(Vec<ExportSetting>, &mut Window, &mut App) + 'static,
        on_export: impl Fn(Vec<String>, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            settings: settings.into_iter().collect(),
            on_change: Rc::new(on_change),
            on_export: Rc::new(on_export),
        }
    }
}

impl RenderOnce for ExportPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_change = self.on_change;
        let on_export = self.on_export;
        let (id, settings, on_change) = (self.id, self.settings, &on_change);
        let files = export_files(&self.name, &settings);
        let next = added(&settings);
        let add = editing("export panel", &settings, on_change, move |all, _: ()| {
            all.extend(next)
        });
        let rows = settings.iter().enumerate().map(|(ix, setting)| {
            let scale = editing(
                "export panel",
                &settings,
                on_change,
                move |all, scale: f32| all[ix].scale = scale,
            );
            let format = editing(
                "export panel",
                &settings,
                on_change,
                move |all, format: ExportFormat| all[ix].format = format,
            );
            let remove = editing("export panel", &settings, on_change, move |all, _: ()| {
                all.remove(ix);
            });
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div().flex_1().min_w_0().child(pair(
                        Select::new(
                            (id.clone(), format!("scale-{ix}")),
                            SCALES
                                .iter()
                                .map(|scale| Choice::new(scale.to_string(), format!("{scale}x"))),
                        )
                        .selected(setting.scale.to_string())
                        .disabled(setting.format.vector())
                        .on_change(move |value, window, cx| {
                            scale(
                                value.parse().expect("a scale the select offers"),
                                window,
                                cx,
                            )
                        }),
                        Select::new(
                            (id.clone(), format!("format-{ix}")),
                            ExportFormat::ALL.iter().map(|format| {
                                Choice::new(format.extension(), format.extension().to_uppercase())
                            }),
                        )
                        .selected(setting.format.extension())
                        .on_change(move |value, window, cx| {
                            let chosen = *ExportFormat::ALL
                                .iter()
                                .find(|format| format.extension() == value.as_ref())
                                .expect("a format the select offers");
                            format(chosen, window, cx)
                        }),
                    )),
                )
                .child(
                    IconButton::new((id.clone(), format!("remove-{ix}")), IconName::Minus)
                        .tooltip("Remove the size")
                        .on_click(move |_, window, cx| remove((), window, cx)),
                )
        });
        let theme = cx.theme();
        let (exported, on_export) = (files.clone(), on_export);
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(heading(
                "Export",
                IconButton::new((id.clone(), "add"), IconName::Plus)
                    .tooltip("Add a size")
                    .disabled(next.is_none())
                    .on_click(move |_, window, cx| add((), window, cx)),
                cx,
            ))
            .children(rows)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(theme.colors.fg_subtle)
                    .children(files.into_iter().map(Ellipsis::new)),
            )
            .child(
                Button::new((id, "export"), format!("Export {}", self.name))
                    .variant(ButtonVariant::Primary)
                    .disabled(exported.is_empty())
                    .on_click(move |_, window, cx| {
                        log::info!("export panel: {exported:?}");
                        on_export(exported.clone(), window, cx);
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{ExportFormat, ExportSetting, added, export_files};

    fn at(scale: f32, format: ExportFormat) -> ExportSetting {
        ExportSetting { scale, format }
    }

    #[test]
    fn files_name_their_scale_and_come_once() {
        let settings = [
            at(1.0, ExportFormat::Png),
            at(2.0, ExportFormat::Png),
            at(0.5, ExportFormat::Jpg),
            at(2.0, ExportFormat::Svg),
            at(3.0, ExportFormat::Svg),
            at(2.0, ExportFormat::Png),
        ];
        assert_eq!(
            export_files("Card", &settings),
            ["Card.png", "Card@2x.png", "Card@0.5x.jpg", "Card.svg"]
        );
    }

    #[test]
    fn plus_adds_the_first_scale_no_png_has() {
        assert_eq!(added(&[]), Some(at(1.0, ExportFormat::Png)));
        assert_eq!(
            added(&[at(1.0, ExportFormat::Png), at(2.0, ExportFormat::Jpg)]),
            Some(at(2.0, ExportFormat::Png))
        );
        let every: Vec<_> = [1.0, 2.0, 3.0, 4.0, 0.5]
            .iter()
            .map(|scale| at(*scale, ExportFormat::Png))
            .collect();
        assert_eq!(added(&every), None);
    }
}
