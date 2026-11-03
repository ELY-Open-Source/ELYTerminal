//! Components Ely ships say nothing in fixed English.

/// Sources of components the Ely apps draw.
const SHIPPED: &[(&str, &str)] = &[
    ("editor/find.rs", include_str!("../../editor/find.rs")),
    ("editor/hints.rs", include_str!("../../editor/hints.rs")),
    ("forms/input.rs", include_str!("../../forms/input.rs")),
    ("forms/select.rs", include_str!("../../forms/select.rs")),
    (
        "tables/builders.rs",
        include_str!("../../tables/builders.rs"),
    ),
    ("tables/rules.rs", include_str!("../../tables/rules.rs")),
    (
        "navigation/editor_tabs.rs",
        include_str!("../../navigation/editor_tabs.rs"),
    ),
    (
        "settings/appearance.rs",
        include_str!("../../settings/appearance.rs"),
    ),
];

#[test]
fn shipped_components_show_no_fixed_english() {
    let shown = [".tooltip(\"", ".child(\"", "SharedString::from(\"", "=> \""];
    // After an id: words, not a lowercase id.
    let labels = ["), \"", "\", \"", "(\""];
    let mut found = Vec::new();
    for (path, source) in SHIPPED {
        let code = source.split("#[cfg(test)]").next().unwrap_or(source);
        for (no, line) in code.lines().enumerate() {
            let checks = ["assert", "panic!", "log::", ".expect("];
            if checks.iter().any(|check| line.contains(check)) {
                continue;
            }
            let literals = |calls: &[&str]| -> Vec<&str> {
                (calls.iter())
                    .flat_map(|call| {
                        line.match_indices(call)
                            .map(|(at, _)| &line[at + call.len()..])
                    })
                    .map(|rest| rest.split('"').next().unwrap_or(rest))
                    .filter(|literal| literal.starts_with(|ch: char| ch.is_ascii_alphabetic()))
                    .collect()
            };
            let key = |literal: &str| literal.contains('.') && !literal.contains(' ');
            let words =
                |literal: &str| literal.contains(' ') || literal.starts_with(char::is_uppercase);
            let english = (literals(&shown).into_iter().filter(|literal| !key(literal)))
                .chain(
                    literals(&labels)
                        .into_iter()
                        .filter(|literal| words(literal)),
                )
                .chain(
                    line.trim()
                        .strip_prefix('"')
                        .and_then(|rest| rest.split('"').next()),
                )
                .filter(|literal| {
                    literal.starts_with(char::is_uppercase) || !line.trim().starts_with('"')
                });
            if english.count() > 0 {
                found.push(format!("{path}:{}: {}", no + 1, line.trim()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "English outside the catalog:\n{}",
        found.join("\n")
    );
}
