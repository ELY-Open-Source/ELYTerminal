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

/// Resizes, then waits out a redraw that shows no mark.
fn resize_bare(run: &mut Run, columns: usize) {
    run.pty.resize((columns, 30), (8, 16));
    std::thread::sleep(std::time::Duration::from_millis(500));
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

/// Types `line` at a prompt marked only at its start.
fn ends_bare(run: &mut Run, line: &str, status: i32) {
    let prompts = run.lines + 1;
    run.until("the prompt's mark", |run| {
        let marked = run.marks.iter();
        marked
            .filter(|mark| matches!(mark, Mark::Prompt(_)))
            .count()
            >= prompts
    });
    let from = run.marks.len();
    run.send(line);
    // A theme leaving PS1 alone shows two starts.
    run.until(&format!("the marks of {line}"), |run| {
        let heard = &run.marks[from..];
        let done = heard
            .iter()
            .position(|mark| *mark == Mark::Done(Some(status)));
        done.is_some_and(|at| {
            heard[..at].contains(&Mark::Output)
                && heard[at..]
                    .iter()
                    .any(|mark| matches!(mark, Mark::Prompt(_)))
        })
    });
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
    ends_bare(&mut run, "printf 'FIRST\\n'", 0);
    ends_bare(&mut run, "printf 'SECOND\\n'", 0);
    resize_bare(&mut run, 70);
    assert_eq!(
        text(&run).matches("~/project main").count(),
        3,
        "{}",
        text(&run)
    );
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
    ends_bare(&mut run, "false | true", 0);
    ends_bare(&mut run, "print -r -- PIPE=$pipestatus", 0);
    run.until("the pipeline's statuses", |run| run.shows("PIPE=1 0"));
    run.finish();
}

/// A theme that sets its prompt last, showing the statuses.
const LAST: &str = "setopt promptsubst\nf() {\n  PROMPT='STATUS=%? PIPE=${pipestatus[@]} > '\n  precmd_functions=(${precmd_functions[@]:#f} f)\n}\nprecmd_functions+=(f)\n";

#[test]
fn zsh_shows_pipestatus_in_a_prompt_a_theme_sets_last() {
    let mut run = Run::start(&installed("zsh"), &[(".zshrc", LAST)]);
    run.until("the first prompt", |run| run.shows("STATUS="));
    ends_bare(&mut run, "false | false", 1);
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

/// Runs `line` on a cleared screen, then waits for `shown`.
fn shows_after(run: &mut Run, line: &str, status: i32, shown: &str) {
    ends_bare(run, &format!("clear; {line}"), status);
    run.until(shown, |run| run.shows(shown));
}

#[test]
fn zsh_shows_every_status_under_ksh_arrays() {
    let rc = format!("{LAST}setopt ksh_arrays\n");
    let mut run = Run::start(&installed("zsh"), &[(".zshrc", &rc)]);
    run.until("the first prompt", |run| run.shows("STATUS="));
    shows_after(&mut run, "false | true", 0, "STATUS=0 PIPE=1 0 >");
    run.finish();
}

#[test]
fn zsh_shows_statuses_past_128_as_they_were() {
    let mut run = Run::start(&installed("zsh"), &[(".zshrc", LAST)]);
    run.until("the first prompt", |run| run.shows("STATUS="));
    for _ in 0..10 {
        shows_after(&mut run, "(exit 130) | (exit 137)", 137, "PIPE=130 137 >");
    }
    run.finish();
}

#[test]
fn zsh_marks_a_failed_pipeline_s_prompt_under_err_return() {
    let rc = format!("{LAST}setopt err_return\n");
    let mut run = Run::start(&installed("zsh"), &[(".zshrc", &rc)]);
    run.until("the first prompt", |run| run.shows("STATUS="));
    shows_after(&mut run, "false | false", 1, "PIPE=1 1 >");
    ends_bare(&mut run, "true", 0);
    run.finish();
}

#[test]
fn zsh_marks_a_theme_s_prompt_without_the_global_files() {
    let files = [(".zshenv", "unsetopt globalrcs\n"), (".zshrc", LAST)];
    let mut run = Run::start(&installed("zsh"), &files);
    run.until("the first prompt", |run| run.shows("STATUS="));
    ends_bare(&mut run, "true", 0);
    ends_bare(&mut run, "true", 0);
    run.finish();
}

#[test]
fn zsh_leaves_an_unset_ps2_alone_under_nounset() {
    let rc = "setopt nounset\nunset PS2\nPROMPT='PS1 > '\n";
    let mut run = Run::start(&installed("zsh"), &[(".zshrc", rc)]);
    run.until("the first prompt", |run| run.shows("PS1 >"));
    run.ends("print -r -- ${+PS2}", &ended(0));
    run.until("PS2 still unset", |run| run.shows("\n0\n"));
    assert!(!run.shows("parameter not set"), "{}", text(&run));
    run.finish();
}

#[test]
fn zsh_marks_no_new_prompt_when_a_widget_runs_the_precmd_hooks() {
    let rc = "PROMPT_EOL_MARK=''\nsetopt promptsubst\nf() {\n  PROMPT='TOP ${(l:$((COLUMNS-10))::-:)}\n> '\n  precmd_functions=(${precmd_functions:#f} f)\n}\nprecmd_functions+=(f)\nw() {\n  local h\n  for h in $precmd_functions; do $h; done\n  zle -R\n}\nzle -N w\nbindkey '^G' w\n";
    let mut run = Run::start(&installed("zsh"), &[(".zshrc", rc)]);
    run.until("the first prompt", |run| run.shows("TOP"));
    ends_bare(&mut run, "true", 0);
    run.pty.write(b"abcdef\x07gh".to_vec());
    run.until("the typed line", |run| run.shows("abcdefgh"));
    resize_bare(&mut run, 40);
    resize_bare(&mut run, 100);
    let shown = text(&run);
    assert_eq!(shown.matches("TOP").count(), 2, "{shown}");
}
