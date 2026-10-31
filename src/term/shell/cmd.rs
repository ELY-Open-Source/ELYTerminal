//! cmd's marks and folder, ConEmu's raw OSC 9;9, through `PROMPT`.

/// cmd's marked `PROMPT`; `own` when the session's shell.
pub(crate) fn prompt(user: Option<&str>, own: bool) -> String {
    let user = user.unwrap_or("$P$G");
    // Ours, inherited, is marked again for this place.
    let user = [true, false]
        .into_iter()
        .find_map(|own| user.strip_prefix(&opening(own)))
        .and_then(|inner| inner.strip_suffix(CLOSING))
        .unwrap_or(user);
    // Another's marks stay as they are.
    if user.contains("133;A") {
        return user.to_string();
    }
    format!("{}{user}{CLOSING}", opening(own))
}

/// What ends a marked prompt.
const CLOSING: &str = "$E]133;B$E\\";

/// What starts it; nested, a program's prompt, unnamed.
fn opening(own: bool) -> String {
    let aid = if own { ";aid=elyterminal" } else { "" };
    format!("$E]133;A{aid}$E\\$E]9;9;$P$E\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_s_default_prompt_is_marked_with_its_folder() {
        assert_eq!(
            prompt(None, true),
            "$E]133;A;aid=elyterminal$E\\$E]9;9;$P$E\\$P$G$E]133;B$E\\"
        );
        let nested = prompt(None, false);
        assert!(
            nested.starts_with("$E]133;A$E\\"),
            "no shell's name: {nested}"
        );
    }

    #[test]
    fn a_user_s_prompt_is_kept_inside_the_marks_once() {
        let marked = prompt(Some("$T $P$G"), true);
        assert!(marked.contains("$E\\$T $P$G$E]133;B"), "{marked}");
        assert_eq!(prompt(Some(&marked), true), marked);
    }

    #[test]
    fn an_inherited_marked_prompt_is_marked_again_for_its_place() {
        for own in [true, false] {
            for user in [None, Some("$T $P$G")] {
                let inherited = prompt(user, !own);
                assert_eq!(prompt(Some(&inherited), own), prompt(user, own));
            }
        }
        let foreign = "$E]133;A;aid=other$E\\$P$G";
        assert_eq!(prompt(Some(foreign), true), foreign, "another's marks stay");
    }

    #[test]
    fn cmd_s_prompt_ends_no_command() {
        assert!(
            !prompt(None, true).contains("133;D"),
            "cmd sends no C to end"
        );
    }
}
