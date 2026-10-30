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

#[test]
fn zsh_keeps_a_continued_command_s_first_line_through_a_resize() {
    let mut run = ready();
    run.until("line editor", |run| run.pty.reads_keys());
    run.pty.write(b"for x in one two; do\r".to_vec());
    run.until("the continuation", |run| run.shows("for>"));
    run.pty.write(b"echo $x".to_vec());
    run.until("the typed line", |run| run.shows("echo $x"));
    for columns in [70, 40, 90] {
        resize(&mut run, columns);
    }
    assert!(
        text(&run).contains("for x in one two; do"),
        "{}",
        text(&run)
    );
    run.pty.write(b"\rdone\r".to_vec());
    run.until("the loop's output", |run| run.shows("one\ntwo"));
    run.finish();
}

#[test]
fn zsh_without_its_line_editor_promises_no_redraw() {
    let rc = format!("unsetopt zle\n{THEME}");
    let mut run = Run::start(&installed("zsh"), &[(".zshrc", &rc)]);
    run.until("the first prompt", |run| run.shows("~/project main"));
    run.pty.write(b"echo hello".to_vec());
    run.until("the typed line", |run| run.shows("echo hello"));
    run.pty.resize((70, 30), (8, 16));
    std::thread::sleep(std::time::Duration::from_millis(300));
    let shown = text(&run);
    assert!(shown.contains("echo hello"), "{shown}");
    assert!(shown.contains("~/project main"), "{shown}");
}

#[test]
fn zsh_keeps_pipestatus_when_a_hook_moves_itself_last() {
    let rc = format!(
        "{THEME}\nf() {{ precmd_functions=(${{precmd_functions:#f}} f) }}\nprecmd_functions+=(f)\n"
    );
    let mut run = Run::start(&installed("zsh"), &[(".zshrc", &rc)]);
    run.until("the first prompt", |run| run.shows("~/project main"));
    run.ends("false | true", &ended(0));
    run.ends("print -r -- PIPE=$pipestatus", &ended(0));
    run.until("the pipeline's statuses", |run| run.shows("PIPE=1 0"));
    run.finish();
}

#[test]
fn zsh_shows_pipestatus_in_a_prompt_a_theme_sets_last() {
    let rc = "setopt promptsubst\nf() {\n  PROMPT='STATUS=%? PIPE=${pipestatus} > '\n  precmd_functions=(${precmd_functions:#f} f)\n}\nprecmd_functions+=(f)\n";
    let mut run = Run::start(&installed("zsh"), &[(".zshrc", rc)]);
    run.until("the first prompt", |run| run.shows("STATUS="));
    run.ends("false | false", &ended(1));
    run.until("the prompt's statuses", |run| {
        run.shows("STATUS=1 PIPE=1 1 >")
    });
    run.finish();
}

#[test]
fn zsh_promises_no_redraw_once_its_line_editor_goes() {
    let mut run = ready();
    run.ends("unsetopt zle", &ended(0));
    let typed = format!("echo {}", "y".repeat(70));
    run.pty.write(typed.clone().into_bytes());
    run.until("the typed line", |run| {
        text(run).replace('\n', "").contains(&typed)
    });
    run.pty.resize((40, 30), (8, 16));
    std::thread::sleep(std::time::Duration::from_millis(300));
    let shown = text(&run);
    assert!(shown.replace('\n', "").contains(&typed), "{shown}");
}
