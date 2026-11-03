use std::rc::Rc;

use gpui::{App, ElementId, Hsla, IntoElement, ParentElement, RenderOnce, Styled, Window, div};

use super::{DensitySelector, SettingsRow, SettingsSection};
use crate::{
    canvas::color_well,
    forms::{Slider, Switch, hex},
    theme::{ActiveTheme, Density, Palette, TextSize, Theme},
    typography::literal,
};

/// What a theme editor changes: the look's measures and switches, and the colors of the mode shown.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeDraft {
    pub density: Density,
    pub radius_scale: f32,
    pub font_scale: f32,
    pub high_contrast: bool,
    pub reduced_motion: bool,
    pub colors: Palette,
}

impl ThemeDraft {
    /// The theme as it stands, its palette the one the shown mode fades to.
    pub fn of(theme: &Theme) -> Self {
        Self {
            density: theme.density,
            radius_scale: theme.radius_scale,
            font_scale: theme.font_scale,
            high_contrast: theme.high_contrast(),
            reduced_motion: theme.reduced_motion,
            colors: theme.palette(),
        }
    }
}

/// The colors an editor lays open, each by its palette name and the name a person reads.
const COLORS: [(&str, &str); 12] = [
    ("bg", "Background"),
    ("surface", "Surface"),
    ("border", "Border"),
    ("fg", "Text"),
    ("fg_muted", "Quiet text"),
    ("accent", "Accent"),
    ("on_accent", "On accent"),
    ("focus", "Focus"),
    ("link", "Link"),
    ("success", "Success"),
    ("warning", "Warning"),
    ("danger", "Danger"),
];

type OnDraft = Rc<dyn Fn(&ThemeDraft, &mut Window, &mut App)>;

/// A theme laid open: density, corner rounding, text size, high contrast and reduced motion, then the main colors of the mode shown, each a swatch that opens a picker beside its hex. Each change hands the owner a changed draft, which it applies through `Theme::update`, `Theme::set_high_contrast` and `Theme::set_palette`.
#[derive(IntoElement)]
pub struct ThemeEditor {
    id: ElementId,
    draft: ThemeDraft,
    on_change: OnDraft,
}

impl ThemeEditor {
    pub fn new(
        id: impl Into<ElementId>,
        draft: ThemeDraft,
        on_change: impl Fn(&ThemeDraft, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            draft,
            on_change: Rc::new(on_change),
        }
    }
}

impl RenderOnce for ThemeEditor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let on_change = self.on_change;
        let draft = Rc::new(self.draft);
        let edit = {
            let (draft, on_change) = (draft.clone(), on_change);
            move |change: &dyn Fn(&mut ThemeDraft), window: &mut Window, cx: &mut App| {
                let mut next = (*draft).clone();
                change(&mut next);
                log::info!("theme editor: changed");
                on_change(&next, window, cx)
            }
        };
        let edit = Rc::new(edit);
        let theme = cx.theme();
        let row = |title: &'static str, control: gpui::AnyElement| {
            SettingsRow::new(title).control(control)
        };
        let [density, corners, text, contrast, motion] = [(); 5].map(|_| edit.clone());
        let look = SettingsSection::new("Look")
            .row(row(
                "Density",
                DensitySelector::new(
                    (id.clone(), "density"),
                    draft.density,
                    move |next, window, cx| {
                        density(&|d: &mut ThemeDraft| d.density = next, window, cx)
                    },
                )
                .into_any_element(),
            ))
            .row(row(
                "Corners",
                div()
                    .w(theme.label_width())
                    .child(
                        Slider::new((id.clone(), "corners"), draft.radius_scale as f64)
                            .range(0.0, 2.0)
                            .step(0.25)
                            .on_change(move |next, window, cx| {
                                corners(
                                    &|d: &mut ThemeDraft| d.radius_scale = next as f32,
                                    window,
                                    cx,
                                )
                            }),
                    )
                    .into_any_element(),
            ))
            .row(row(
                "Text size",
                div()
                    .w(theme.label_width())
                    .child(
                        Slider::new((id.clone(), "text"), draft.font_scale as f64)
                            .range(0.85, 1.3)
                            .step(0.05)
                            .on_change(move |next, window, cx| {
                                text(&|d: &mut ThemeDraft| d.font_scale = next as f32, window, cx)
                            }),
                    )
                    .into_any_element(),
            ))
            .row(row(
                "High contrast",
                Switch::new((id.clone(), "contrast"), draft.high_contrast)
                    .on_change(move |on, window, cx| {
                        contrast(&|d: &mut ThemeDraft| d.high_contrast = on, window, cx)
                    })
                    .into_any_element(),
            ))
            .row(row(
                "Reduce motion",
                Switch::new((id.clone(), "motion"), draft.reduced_motion)
                    .on_change(move |on, window, cx| {
                        motion(&|d: &mut ThemeDraft| d.reduced_motion = on, window, cx)
                    })
                    .into_any_element(),
            ));
        let mut colors = draft.colors.clone();
        let tokens = COLORS.iter().map(|(token, name)| {
            let color: Hsla = *colors.token_mut(token);
            let (recolor, token) = (edit.clone(), *token);
            row(
                name,
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        literal(div())
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(theme.colors.fg_muted)
                            .child(hex(color.to_rgb())),
                    )
                    .child(color_well(
                        (id.clone(), format!("color-{token}")).into(),
                        color,
                        move |next, window, cx| {
                            recolor(
                                &|d: &mut ThemeDraft| *d.colors.token_mut(token) = next,
                                window,
                                cx,
                            )
                        },
                    ))
                    .into_any_element(),
            )
        });
        let palette = tokens.fold(
            SettingsSection::new("Colors").description("The colors of the mode shown."),
            |section, row| section.row(row),
        );
        div().flex().flex_col().gap_6().child(look).child(palette)
    }
}
