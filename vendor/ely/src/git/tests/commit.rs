use crate::{
    i18n,
    theme::{ActiveTheme, TextSize},
};
use gpui::{
    AppContext, Context, Entity, Focusable, IntoElement, Modifiers, ParentElement, Render, Styled,
    TestAppContext, VisualTestContext, Window, div, px,
};

use super::super::CommitInput;
use crate::forms::{Checkbox, TextInput};

struct Form {
    width: f32,
    standalone: bool,
    branch: String,
    subject: Entity<TextInput>,
    body: Entity<TextInput>,
    staged: usize,
    amend: bool,
    committed: usize,
}

impl Render for Form {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (flip, commit) = (cx.entity(), cx.entity());
        let root = div()
            .w(px(self.width))
            .font_family(cx.theme().font_family.clone());
        if self.standalone {
            return root.child(Checkbox::new("standalone", false).label(i18n::text(
                cx,
                "git.commit.amend",
                &[],
            )));
        }
        root.child(
            CommitInput::new(
                "test-commit",
                &self.subject,
                &self.body,
                self.branch.clone(),
                self.staged,
            )
            .amend(self.amend)
            .on_amend(move |on, _, cx| {
                flip.update(cx, |form, cx| {
                    form.amend = on;
                    cx.notify();
                })
            })
            .on_commit(move |_, cx| commit.update(cx, |form, _| form.committed += 1)),
        )
    }
}

fn open(cx: &mut TestAppContext) -> (Entity<Form>, &mut VisualTestContext) {
    cx.update(crate::init_for_tests);
    cx.add_window_view(|window, cx| Form {
        width: 960.0,
        standalone: false,
        branch: "main".into(),
        subject: cx.new(|cx| TextInput::new(window, cx).placeholder("Summary")),
        body: cx.new(|cx| TextInput::new(window, cx).multi_line(3, 6)),
        staged: 0,
        amend: false,
        committed: 0,
    })
}

fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

#[gpui::test]
fn resizing_keeps_amend_readable_and_commit_inside_the_footer(cx: &mut TestAppContext) {
    check_resizing(cx);
}

#[cfg(target_os = "linux")]
#[test]
fn resizing_with_native_inter_font_keeps_amend_readable() {
    let system = gpui_platform::current_platform(true).text_system();
    let mut cx = TestAppContext::build_with_text_system(gpui::TestDispatcher::new(0), None, system);
    cx.update(|cx| crate::assets::load_fonts(cx).expect("bundled fonts"));
    check_resizing(&mut cx);
    cx.quit();
}

fn check_resizing(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    settle(cx);
    let label_width = cx.update(|window, cx| {
        let text = i18n::text(cx, "git.commit.amend", &[]);
        let size = cx
            .theme()
            .text_size(TextSize::Base)
            .to_pixels(window.rem_size());
        let mut run = window.text_style().to_run(text.len());
        run.font = gpui::font(cx.theme().font_family.clone());
        window
            .text_system()
            .shape_line(text, size, &[run], None)
            .width
    });
    let branches = [
        "main".to_string(),
        "feature/ui-amend-layout-with-a-very-long-branch-name".to_string(),
        format!("feature/{}", "x".repeat(200)),
        format!("feature/{}", "改进布局".repeat(30)),
    ];
    for branch in branches {
        for amend in [false, true] {
            for width in [
                960.0, 560.0, 320.0, 240.0, 160.0, 144.0, 143.0, 240.0, 960.0,
            ] {
                view.update(cx, |form, cx| {
                    form.width = width;
                    form.standalone = true;
                    cx.notify();
                });
                settle(cx);
                let reference = cx.debug_bounds("checkbox-label").expect("standalone label");
                view.update(cx, |form, cx| {
                    form.standalone = false;
                    form.branch = branch.clone();
                    form.amend = amend;
                    cx.notify();
                });
                settle(cx);
                let footer = cx.debug_bounds("commit-footer").expect("footer");
                let amend = cx.debug_bounds("commit-amend").expect("amend control");
                let current = cx.debug_bounds("checkbox-label").expect("amend label");
                let button = cx.debug_bounds("commit-button").expect("commit button");
                assert_eq!(
                    current.size.height, reference.size.height,
                    "{branch} at {width}"
                );
                assert!(
                    current.size.width >= label_width.min(reference.size.width),
                    "{current:?} at {width}"
                );
                for bounds in [amend, current, button] {
                    assert!(bounds.left() >= footer.left(), "{bounds:?}");
                    assert!(bounds.right() <= footer.right(), "{bounds:?} at {width}");
                }
                if branch == "main" && width >= 320.0 {
                    assert!(
                        button.top() < amend.bottom(),
                        "short controls should share a row"
                    );
                }
                if width <= 240.0 && branch.len() > 50 {
                    assert!(button.top() >= amend.bottom(), "long branch should wrap");
                }
                if button.top() < amend.bottom() {
                    assert!(
                        button.left() >= amend.right(),
                        "{amend:?} overlaps {button:?}"
                    );
                } else {
                    assert_eq!(button.left(), footer.left());
                    assert_eq!(button.right(), footer.right());
                }
                assert!(
                    footer.size.height <= reference.size.height + button.size.height + px(8.0),
                    "{footer:?} at {width}"
                );
            }
        }
    }
}

#[gpui::test]
fn amend_and_commit_keep_their_click_and_submit_behavior(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    view.update(cx, |form, cx| {
        form.width = 143.0;
        form.branch = format!("feature/{}", "long".repeat(30));
        cx.notify();
    });
    settle(cx);
    let click_commit = |cx: &mut VisualTestContext| {
        let button = cx.debug_bounds("commit-button").expect("commit button");
        cx.simulate_click(button.center(), Modifiers::none());
        settle(cx);
    };
    click_commit(cx);
    assert_eq!(view.read_with(cx, |form, _| form.committed), 0);
    let subject = view.read_with(cx, |form, _| form.subject.clone());
    subject.update(cx, |input, cx| input.set_text("A summary", cx));
    settle(cx);
    click_commit(cx);
    assert_eq!(view.read_with(cx, |form, _| form.committed), 0);
    let amend = cx.debug_bounds("commit-amend").expect("amend control");
    cx.simulate_click(amend.center(), Modifiers::none());
    settle(cx);
    assert!(view.read_with(cx, |form, _| form.amend));
    click_commit(cx);
    assert_eq!(view.read_with(cx, |form, _| form.committed), 1);
    cx.update(|window, cx| window.focus(&subject.read(cx).focus_handle(cx), cx));
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert_eq!(view.read_with(cx, |form, _| form.committed), 1);
    cx.simulate_keystrokes(if cfg!(target_os = "macos") {
        "cmd-enter"
    } else {
        "ctrl-enter"
    });
    settle(cx);
    assert_eq!(view.read_with(cx, |form, _| form.committed), 2);
    subject.update(cx, |input, cx| input.set_text("", cx));
    settle(cx);
    click_commit(cx);
    assert_eq!(view.read_with(cx, |form, _| form.committed), 2);
    view.update(cx, |form, cx| {
        form.amend = false;
        form.staged = 1;
        cx.notify();
    });
    subject.update(cx, |input, cx| input.set_text("A staged change", cx));
    settle(cx);
    click_commit(cx);
    assert_eq!(view.read_with(cx, |form, _| form.committed), 3);
}
