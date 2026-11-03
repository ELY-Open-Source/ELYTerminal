use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div,
};

use crate::{
    buttons::{ButtonVariant, IconButton},
    menus::{Menu, MenuItem, SplitButton},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, TextSize},
};

/// A shell a terminal can start: its name and its program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellChoice {
    pub name: SharedString,
    pub program: SharedString,
}

type OnIndex = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// A new terminal in the default shell, with the others under its arrow, the default checked.
#[derive(IntoElement)]
pub struct ShellSelector {
    id: ElementId,
    shells: Vec<ShellChoice>,
    default: usize,
    on_pick: Option<OnIndex>,
}

impl ShellSelector {
    pub fn new(id: impl Into<ElementId>, shells: impl IntoIterator<Item = ShellChoice>) -> Self {
        let shells: Vec<ShellChoice> = shells.into_iter().collect();
        assert!(
            !shells.is_empty(),
            "a shell selector offers at least one shell"
        );
        Self {
            id: id.into(),
            shells,
            default: 0,
            on_pick: None,
        }
    }

    pub fn default_shell(mut self, default: usize) -> Self {
        if default >= self.shells.len() {
            log::error!(
                "shell selector: no shell {default} of {}; the default stays",
                self.shells.len()
            );
            return self;
        }
        self.default = default;
        self
    }

    /// Called with the shell to start: the default from the button, any from the menu.
    pub fn on_pick(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ShellSelector {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let menu = self
            .shells
            .iter()
            .enumerate()
            .fold(Menu::new(), |menu, (ix, shell)| {
                let pick = self.on_pick.clone();
                menu.item(
                    MenuItem::check(shell.name.clone(), ix == self.default).on_click(
                        move |window, cx| {
                            if let Some(pick) = &pick {
                                pick(ix, window, cx)
                            }
                        },
                    ),
                )
            });
        let (pick, default) = (self.on_pick, self.default);
        SplitButton::new(self.id, format!("New {}", self.shells[default].name), menu)
            .variant(ButtonVariant::Ghost)
            .on_click(move |_, window, cx| {
                if let Some(pick) = &pick {
                    pick(default, window, cx)
                }
            })
    }
}

type Run = Rc<dyn Fn(&mut Window, &mut App)>;

/// The row over a terminal: its title and how its program stands, the shell selector, and split, clear, stop and maximize.
#[derive(IntoElement)]
pub struct TerminalToolbar {
    id: ElementId,
    title: SharedString,
    exit: Option<i32>,
    shells: Option<AnyElement>,
    actions: Vec<(&'static str, IconName, &'static str, Run)>,
}

impl TerminalToolbar {
    /// `exit` is the program's exit code once it has ended.
    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        exit: Option<i32>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            exit,
            shells: None,
            actions: Vec::new(),
        }
    }

    pub fn shells(mut self, selector: ShellSelector) -> Self {
        self.shells = Some(selector.into_any_element());
        self
    }

    fn action(
        mut self,
        key: &'static str,
        icon: IconName,
        words: &'static str,
        run: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.actions.push((key, icon, words, Rc::new(run)));
        self
    }

    pub fn on_split(self, run: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.action("split", IconName::Columns2, "Split terminal", run)
    }

    pub fn on_clear(self, run: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.action("clear", IconName::Eraser, "Clear", run)
    }

    pub fn on_stop(self, run: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.action("stop", IconName::CircleStop, "Stop the program", run)
    }

    pub fn on_maximize(self, run: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.action("maximize", IconName::Maximize2, "Maximize", run)
    }
}

impl RenderOnce for TerminalToolbar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let (mark, color) = match self.exit {
            None => (IconName::Terminal, colors.fg_muted),
            Some(0) => (IconName::CircleCheck, colors.success),
            Some(_) => (IconName::CircleX, colors.danger),
        };
        div()
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .border_b_1()
            .border_color(colors.border)
            .text_size(theme.text_size(TextSize::Sm))
            .child(Icon::new(mark).size(IconSize::Sm).color(color))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.fg)
                    .child(self.title),
            )
            .children(self.exit.filter(|code| *code != 0).map(|code| {
                div()
                    .text_color(colors.danger)
                    .child(format!("Exit {code}"))
            }))
            .children(self.shells)
            .children(self.actions.into_iter().map(|(key, icon, words, run)| {
                IconButton::new((self.id.clone(), key), icon)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .tooltip(words)
                    .on_click(move |_, window, cx| run(window, cx))
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::{ShellChoice, ShellSelector};

    #[test]
    fn a_default_past_the_shells_keeps_the_one_before() {
        let shell = |name: &'static str| ShellChoice {
            name: name.into(),
            program: name.into(),
        };
        let picker = ShellSelector::new("shells", [shell("zsh"), shell("bash")])
            .default_shell(1)
            .default_shell(5);
        assert_eq!(picker.default, 1);
    }
}
