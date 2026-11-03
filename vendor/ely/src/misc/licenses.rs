use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div,
};

use crate::{
    layout::on_axis,
    lists::{ListItem, SelectableList},
    theme::{ActiveTheme, Radius, TextSize},
    typography::literal,
};

/// A package a product ships: its name, version, license by SPDX name, and the license's text.
#[derive(Clone)]
pub struct Package {
    name: SharedString,
    version: SharedString,
    license: SharedString,
    text: SharedString,
}

impl Package {
    pub fn new(
        name: impl Into<SharedString>,
        version: impl Into<SharedString>,
        license: impl Into<SharedString>,
        text: impl Into<SharedString>,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            license: license.into(),
            text: text.into(),
        }
    }
}

/// The packages a product ships, a row each with its version and license, beside the chosen one's text, which scrolls; narrow, the text drops below the list. The first package shows until one is chosen.
#[derive(IntoElement)]
pub struct LicenseViewer {
    id: ElementId,
    packages: Vec<Package>,
}

impl LicenseViewer {
    pub fn new(id: impl Into<ElementId>, packages: impl IntoIterator<Item = Package>) -> Self {
        Self {
            id: id.into(),
            packages: packages.into_iter().collect(),
        }
    }
}

impl RenderOnce for LicenseViewer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        assert!(
            !self.packages.is_empty(),
            "license viewer {id:?} has no packages"
        );
        let chosen = window.use_keyed_state((id.clone(), "chosen"), cx, |_, _| 0);
        let mut at = *chosen.read(cx);
        if at >= self.packages.len() {
            log::error!(
                "license viewer {id:?}: package {at} of {}; the first is shown",
                self.packages.len()
            );
            at = 0;
            chosen.update(cx, |chosen, _| *chosen = 0);
        }
        let theme = cx.theme();
        let colors = &theme.colors;
        let list = self.packages.iter().enumerate().fold(
            SelectableList::new((id.clone(), "list"))
                .selected([at.to_string()])
                .on_change(move |keys, _, cx| {
                    let Some(key) = keys.first() else {
                        return;
                    };
                    let ix = key.parse().expect("a package's place");
                    log::info!("license viewer: package {ix}");
                    chosen.update(cx, |chosen, cx| {
                        *chosen = ix;
                        cx.notify();
                    })
                }),
            |list, (ix, package)| {
                list.row(
                    ix.to_string(),
                    ListItem::new((id.clone(), format!("row-{ix}")), package.name.clone())
                        .description(format!("{} · {}", package.version, package.license)),
                )
            },
        );
        let package = &self.packages[at];
        let detail = div()
            .flex_1()
            .min_w(theme.misc().licenses)
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .debug_selector(|| format!("license-{}", package.name))
                    .flex()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(colors.fg)
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(format!("{} {}", package.name, package.version)),
                    ),
            )
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(colors.fg_muted)
                    .child(package.license.clone()),
            )
            .child(on_axis(
                div()
                    .id((id.clone(), "text"))
                    .overflow_y_scroll()
                    .max_h(theme.misc().reading)
                    .p_3()
                    .rounded(theme.radius(Radius::Md))
                    .bg(colors.sunken)
                    .child(
                        literal(div())
                            .font_family(theme.mono_family.clone())
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg)
                            .child(package.text.clone()),
                    ),
            ));
        div()
            .flex()
            .flex_wrap()
            .items_start()
            .gap_4()
            .child(div().flex_none().w(theme.misc().licenses).child(list))
            .child(detail)
    }
}
