use std::rc::Rc;

use gpui::{
    App, ElementId, FontStyle, HighlightStyle, Hsla, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, Styled, StyledText, Window, div,
};

use crate::{
    forms::RadioCard,
    theme::{ActiveTheme, Mode, Radius, Syntax, SyntaxTheme, TextSize},
    typography::literal,
};

/// A line of code in `syntax`'s colors.
fn sample(syntax: &Syntax, cx: &App) -> impl IntoElement + use<> {
    let theme = cx.theme();
    let text = "fn answer() -> u32 { 42 } // truth";
    let ink = |color: Hsla| HighlightStyle {
        color: Some(color),
        ..Default::default()
    };
    let spans = [
        (0..2, ink(syntax.keyword)),
        (3..9, ink(syntax.function)),
        (9..11, ink(syntax.punctuation)),
        (12..14, ink(syntax.operator)),
        (15..18, ink(syntax.type_name)),
        (19..20, ink(syntax.punctuation)),
        (21..23, ink(syntax.number)),
        (24..25, ink(syntax.punctuation)),
        (
            26..34,
            HighlightStyle {
                color: Some(syntax.comment),
                font_style: Some(FontStyle::Italic),
                ..Default::default()
            },
        ),
    ];
    literal(div())
        .debug_selector(|| "syntax-sample".into())
        .px_3()
        .py_2()
        .rounded(theme.radius(Radius::Md))
        .bg(theme.colors.sunken)
        .font_family(theme.mono_family.clone())
        .text_size(theme.text_size(TextSize::Sm))
        .text_color(theme.colors.fg)
        .child(StyledText::new(text).with_highlights(spans))
}

type OnName = Rc<dyn Fn(&'static str, &mut Window, &mut App)>;

/// Code palettes to choose from, each a card with a line of code in its colors for the mode shown.
#[derive(IntoElement)]
pub struct SyntaxThemePicker {
    id: ElementId,
    themes: Vec<SyntaxTheme>,
    selected: SharedString,
    on_change: OnName,
}

impl SyntaxThemePicker {
    /// `selected` names one of `themes`.
    pub fn new(
        id: impl Into<ElementId>,
        themes: impl IntoIterator<Item = SyntaxTheme>,
        selected: impl Into<SharedString>,
        on_change: impl Fn(&'static str, &mut Window, &mut App) + 'static,
    ) -> Self {
        let themes: Vec<SyntaxTheme> = themes.into_iter().collect();
        let selected = selected.into();
        if !themes.iter().any(|theme| theme.name == selected.as_ref()) {
            log::error!("syntax theme picker: no theme {selected}; none chosen");
        }
        Self {
            id: id.into(),
            themes,
            selected,
            on_change: Rc::new(on_change),
        }
    }
}

impl RenderOnce for SyntaxThemePicker {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let on_change = self.on_change;
        let mode: Mode = cx.theme().mode();
        let cards = self.themes.iter().map(|theme| {
            let (pick, name) = (on_change.clone(), theme.name);
            RadioCard::new(
                (id.clone(), format!("theme-{name}")),
                self.selected.as_ref() == name,
                name,
            )
            .preview(sample(&theme.of(mode), cx))
            .on_select(move |window, cx| {
                log::info!("syntax theme picker: {name}");
                pick(name, window, cx)
            })
        });
        div().flex().flex_col().gap_2().children(cards)
    }
}
