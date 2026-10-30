use super::*;

const THEME: &str = r#"setopt promptsubst
PROMPT='~/project main ${(l:$((COLUMNS-24))::-:)} 15:49:28
> '
RPROMPT=
"#;

fn resize(run: &mut Run, columns: usize) {
    let before = run.marks.len();
    run.pty.resize((columns, 30), (8, 16));
    run.until("the resized prompt", |run| {
        run.marks[before..].contains(&Mark::Input)
    });
}

fn text(run: &Run) -> String {
    let term = run.pty.term.lock();
    (-(term.grid().history_size() as i32)..term.screen_lines() as i32)
        .map(|line| {
            term.grid()[Line(line)]
                .into_iter()
                .map(|cell| cell.c)
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn ready() -> Run {
    let mut run = Run::start(&installed("zsh"), &[(".zshrc", THEME)]);
    run.until("the first prompt", |run| run.shows("~/project main"));
    run.ends("printf 'OUTPUT\\n'", &ended(0));
    run
}

#[test]
fn zsh_redraws_a_full_width_prompt_without_adding_it_to_history() {
    let mut run = ready();
    for columns in (30..100).rev() {
        resize(&mut run, columns);
    }
    let shrunk = text(&run);
    assert_eq!(
        shrunk.matches("~/project main").count(),
        2,
        "shrinking added prompt copies to history:\n{shrunk}"
    );
    for columns in 31..=100 {
        resize(&mut run, columns);
    }
    let grown = text(&run);
    assert_eq!(
        grown.matches("~/project main").count(),
        2,
        "growing added prompt copies to history:\n{grown}"
    );
    assert!(grown.contains("OUTPUT\n"), "the command's output remains");
    run.finish();
}

#[test]
fn zsh_keeps_a_wrapped_command_through_prompt_resizes() {
    let mut run = ready();
    let value = "x".repeat(140);
    run.until("line editor", |run| run.pty.reads_keys());
    run.pty.write(format!("printf '{value}\\n'").into_bytes());
    run.until("the wrapped command", |run| {
        text(run).replace('\n', "").contains(&value)
    });
    for columns in [70, 40, 30, 60, 100] {
        resize(&mut run, columns);
    }
    let before = run.marks.len();
    run.pty.write(b"\r".to_vec());
    run.until("the wrapped command's next prompt", |run| {
        ends_like(&run.marks[before..], &ended(0))
    });
    let shown = text(&run);
    assert_eq!(shown.replace('\n', "").matches(&value).count(), 2);
    assert_eq!(
        shown.matches("~/project main").count(),
        3,
        "the command left one prompt and printed one:\n{shown}"
    );
    assert!(shown.contains("OUTPUT\n"), "earlier output remains");
    run.finish();
}

#[test]
fn zsh_marks_prompts_when_a_theme_keeps_its_hook_last() {
    let theme = format!(
        "{THEME}\n_ely_test_theme() {{\n  {THEME}\n  precmd_functions=(${{precmd_functions:#_ely_test_theme}} _ely_test_theme)\n}}\nprecmd_functions+=(_ely_test_theme)\n"
    );
    let mut run = Run::start(&installed("zsh"), &[(".zshrc", &theme)]);
    run.until("the first prompt", |run| run.shows("~/project main"));
    run.ends("printf 'FIRST\\n'", &ended(0));
    run.ends("printf 'SECOND\\n'", &ended(0));
    resize(&mut run, 70);
    assert_eq!(text(&run).matches("~/project main").count(), 3);
    run.finish();
}
