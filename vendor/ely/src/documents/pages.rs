use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ObjectFit, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, relative,
};

use crate::{
    buttons::{Button, ButtonVariant},
    documents::blocks::source,
    forms::{EmojiPicker, OnValue},
    overlays::Popover,
    primitives::{IconName, Image},
    theme::{ActiveTheme, ControlSize, Radius},
};

type OnCover = Rc<dyn Fn(Option<SharedString>, &mut Window, &mut App)>;

/// A page's cover, a picture across its top; on hover, covers to switch to and a way to take it off.
#[derive(IntoElement)]
pub struct PageCover {
    id: ElementId,
    source: SharedString,
    choices: Vec<SharedString>,
    on_change: Option<OnCover>,
}

impl PageCover {
    /// `source` is a file or a web address.
    pub fn new(id: impl Into<ElementId>, source: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
            choices: Vec::new(),
            on_change: None,
        }
    }

    /// Offers `choices` to switch to, and a way to take the cover off: `on_change` gets the new cover, or none.
    pub fn choices(
        mut self,
        choices: impl IntoIterator<Item = impl Into<SharedString>>,
        on_change: impl Fn(Option<SharedString>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.choices = choices.into_iter().map(Into::into).collect();
        self.on_change = Some(Rc::new(on_change));
        self
    }
}

impl RenderOnce for PageCover {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let group = SharedString::from(format!("cover-{}", self.id));
        let tools =
            self.on_change.clone().map(|on_change| {
                let (choices, pick, remove) = (self.choices.clone(), on_change.clone(), on_change);
                let id = self.id.clone();
                div()
                    .absolute()
                    .bottom_2()
                    .right_2()
                    .flex()
                    .gap_1()
                    .opacity(0.0)
                    .group_hover(group.clone(), |tools| tools.opacity(1.0))
                    .child(
                        Popover::new((self.id.clone(), "change"), "Change cover", move |_, cx| {
                            let theme = cx.theme();
                            div().grid().grid_cols(3).gap_2().children(
                                choices.iter().enumerate().map(|(ix, choice)| {
                                    let (pick, choice) = (pick.clone(), choice.clone());
                                    div()
                                        .id((id.clone(), format!("choice-{ix}")))
                                        .w(theme.grid_column())
                                        .h(theme.grid_column() * 0.6)
                                        .rounded(theme.radius(Radius::Sm))
                                        .overflow_hidden()
                                        .cursor_pointer()
                                        .on_click(move |_, window, cx| {
                                            log::info!("page cover: {choice}");
                                            pick(Some(choice.clone()), window, cx)
                                        })
                                        .child(
                                            Image::new(
                                                (id.clone(), format!("choice-picture-{ix}")),
                                                source(&choices[ix]),
                                            )
                                            .size_full(),
                                        )
                                }),
                            )
                        })
                        .icon(IconName::Image)
                        .variant(ButtonVariant::Secondary),
                    )
                    .child(
                        Button::new((self.id.clone(), "remove"), "Remove")
                            .variant(ButtonVariant::Secondary)
                            .size(ControlSize::Sm)
                            .on_click(move |_, window, cx| {
                                log::info!("page cover: removed");
                                remove(None, window, cx)
                            }),
                    )
            });
        div()
            .id(self.id.clone())
            .group(group.clone())
            .relative()
            .h(theme.page_cover())
            .child(
                Image::new((self.id.clone(), "picture"), source(&self.source))
                    .fit(ObjectFit::Cover)
                    .size_full(),
            )
            .children(tools)
    }
}

/// A page's icon, an emoji set large; on hover, a way to pick another.
#[derive(IntoElement)]
pub struct PageIcon {
    id: ElementId,
    emoji: SharedString,
    on_change: Option<OnValue>,
}

impl PageIcon {
    pub fn new(id: impl Into<ElementId>, emoji: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            emoji: emoji.into(),
            on_change: None,
        }
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for PageIcon {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let group = SharedString::from(format!("icon-{}", self.id));
        let picker = self.on_change.map(|on_change| {
            let (id, emoji) = (self.id.clone(), self.emoji.clone());
            div()
                .opacity(0.0)
                .group_hover(group.clone(), |picker| picker.opacity(1.0))
                .child(
                    Popover::new((self.id.clone(), "change"), "Change icon", move |_, _| {
                        EmojiPicker::new((id.clone(), "picker"))
                            .selected(emoji.clone())
                            .on_change(move |glyph, window, cx| {
                                log::info!("page icon: {glyph}");
                                on_change(&SharedString::from(glyph.to_string()), window, cx)
                            })
                    })
                    .icon(IconName::Smile)
                    .variant(ButtonVariant::Ghost),
                )
        });
        div()
            .id(self.id.clone())
            .group(group.clone())
            .flex()
            .items_end()
            .gap_2()
            .child(
                div()
                    .text_size(theme.page_icon())
                    .line_height(relative(1.0))
                    .child(self.emoji.clone()),
            )
            .children(picker)
    }
}
