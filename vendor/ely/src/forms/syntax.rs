//! How one language's code reads, by file name.

/// A language's comments, keywords and constants.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Syntax {
    /// The mark that opens a line comment.
    pub comment: &'static str,
    /// The mark counts only where a word starts (Shell).
    pub word_comment: bool,
    pub keywords: &'static [&'static str],
    pub constants: &'static [&'static str],
}

impl Syntax {
    const fn new(
        comment: &'static str,
        keywords: &'static [&'static str],
        constants: &'static [&'static str],
    ) -> Self {
        Self {
            comment,
            word_comment: false,
            keywords,
            constants,
        }
    }
}

#[rustfmt::skip]
const RUST: Syntax = Syntax::new("//", &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
    "return", "static", "struct", "super", "trait", "type", "unsafe", "use", "where", "while",
], &["true", "false", "self", "Self", "None", "Some", "Ok", "Err"]);

#[rustfmt::skip]
const SCRIPT: Syntax = Syntax::new("//", &[
    "as", "async", "await", "break", "case", "catch", "class", "const", "continue", "default",
    "delete", "do", "else", "export", "extends", "finally", "for", "from", "function", "if",
    "import", "in", "instanceof", "interface", "let", "new", "of", "return", "static", "switch",
    "throw", "try", "type", "typeof", "var", "void", "while", "yield",
], &["true", "false", "null", "undefined", "this", "NaN"]);

/// C, C++, Java, Go, Swift, Kotlin, C# and kin.
#[rustfmt::skip]
pub(crate) const C_LIKE: Syntax = Syntax::new("//", &[
    "break", "case", "catch", "class", "const", "continue", "default", "defer", "do", "else",
    "enum", "extends", "final", "for", "func", "fun", "go", "if", "implements", "import",
    "interface", "let", "namespace", "new", "package", "private", "protected", "public", "return",
    "static", "struct", "switch", "throw", "try", "typedef", "union", "using", "val", "var",
    "void", "while",
], &["true", "false", "null", "nil", "nullptr", "this", "self"]);

#[rustfmt::skip]
const PYTHON: Syntax = Syntax::new("#", &[
    "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif",
    "else", "except", "finally", "for", "from", "global", "if", "import", "in", "is", "lambda",
    "nonlocal", "not", "or", "pass", "raise", "return", "try", "while", "with", "yield",
], &["True", "False", "None", "self"]);

#[rustfmt::skip]
const RUBY: Syntax = Syntax::new("#", &[
    "alias", "and", "begin", "break", "case", "class", "def", "do", "else", "elsif", "end",
    "ensure", "for", "if", "in", "module", "next", "not", "or", "redo", "rescue", "retry",
    "return", "then", "unless", "until", "when", "while", "yield",
], &["true", "false", "nil", "self"]);

#[rustfmt::skip]
const SHELL: Syntax = Syntax { word_comment: true, ..Syntax::new("#", &[
    "case", "do", "done", "elif", "else", "esac", "export", "fi", "for", "function", "if", "in",
    "local", "return", "then", "until", "while",
], &["true", "false"]) };

/// `#` languages, CMake and Make: no word list.
const HASH: Syntax = Syntax::new("#", &[], &["true", "false"]);

/// YAML, whose `#` starts a comment only after a space.
const YAML: Syntax = Syntax {
    word_comment: true,
    ..Syntax::new("#", &[], &["true", "false", "null", "yes", "no"])
};

const INI: Syntax = Syntax::new(";", &[], &["true", "false", "yes", "no", "on", "off"]);

#[rustfmt::skip]
const SQL: Syntax = Syntax::new("--", &[
    "SELECT", "FROM", "WHERE", "JOIN", "LEFT", "RIGHT", "INNER", "OUTER", "ON", "AND", "OR",
    "NOT", "AS", "GROUP", "BY", "ORDER", "HAVING", "LIMIT", "OFFSET", "INSERT", "INTO", "VALUES",
    "UPDATE", "SET", "DELETE", "CREATE", "TABLE", "INDEX", "DROP", "ALTER", "DESC", "ASC",
    "DISTINCT", "UNION", "WITH", "CASE", "WHEN", "THEN", "ELSE", "END", "IS", "IN", "LIKE",
    "BETWEEN", "EXISTS",
], &["NULL", "TRUE", "FALSE"]);

#[rustfmt::skip]
const LUA: Syntax = Syntax::new("--", &[
    "and", "break", "do", "else", "elseif", "end", "for", "function", "goto", "if", "in", "local",
    "not", "or", "repeat", "return", "then", "until", "while",
], &["true", "false", "nil"]);

#[rustfmt::skip]
const HASKELL: Syntax = Syntax::new("--", &[
    "case", "class", "data", "deriving", "do", "else", "if", "import", "in", "instance", "let",
    "module", "newtype", "of", "then", "type", "where",
], &["True", "False"]);

#[rustfmt::skip]
const LISP: Syntax = Syntax::new(";", &[
    "defun", "defn", "define", "lambda", "let", "if", "cond",
], &["nil", "t"]);

#[rustfmt::skip]
const ERLANG: Syntax = Syntax::new("%", &[
    "after", "begin", "case", "catch", "end", "fun", "if", "of", "receive", "try", "when",
], &["true", "false"]);

const TEX: Syntax = Syntax::new("%", &[], &[]);

/// Extensions of prose and plain text, which read plain.
#[rustfmt::skip]
const PROSE: &[&str] = &[
    "md", "markdown", "mdown", "mkd", "txt", "text", "rst", "adoc", "asciidoc", "org", "csv", "tsv",
    "log",
];

/// Languages by extension; any other code reads C-like.
#[rustfmt::skip]
const BY_EXTENSION: &[(&Syntax, &[&str])] = &[
    (&RUST, &["rs"]),
    (&SCRIPT, &["js", "mjs", "cjs", "jsx", "ts", "tsx", "mts"]),
    (&PYTHON, &["py", "pyi", "pyw"]),
    (&RUBY, &["rb", "rake", "gemspec"]),
    (&SHELL, &["sh", "bash", "zsh", "fish", "ksh"]),
    (&HASH, &["pl", "r", "cmake", "mk", "nix", "ex", "exs", "jl", "nim", "cr", "tcl", "ps1",
        "toml", "conf", "cfg", "gitignore", "env", "properties"]),
    (&YAML, &["yaml", "yml"]),
    (&INI, &["ini"]),
    (&SQL, &["sql"]),
    (&LUA, &["lua"]),
    (&HASKELL, &["hs", "lhs", "elm"]),
    (&LISP, &["clj", "cljs", "lisp", "el", "scm"]),
    (&ERLANG, &["erl", "hrl"]),
    (&TEX, &["tex", "sty", "bib"]),
];

/// How code fenced as `name` reads; none for prose.
pub(crate) fn syntax_fenced(name: &str) -> Option<&'static Syntax> {
    let name = name.to_ascii_lowercase();
    let extension = match name.as_str() {
        "rust" => "rs",
        "javascript" | "typescript" => "js",
        "python" => "py",
        "ruby" => "rb",
        "shell" | "console" => "sh",
        "haskell" => "hs",
        "clojure" => "clj",
        "scheme" | "elisp" => "lisp",
        "erlang" => "erl",
        "latex" => "tex",
        "plaintext" | "plain" => "txt",
        other => other,
    };
    syntax_of(&format!("fence.{extension}"))
}

/// How the file at `path` reads; none for prose.
pub(crate) fn syntax_of(path: &str) -> Option<&'static Syntax> {
    let name = path.rsplit_once('/').map_or(path, |(_, name)| name);
    // Named files first: CMakeLists.txt is not prose.
    match name {
        "Makefile" | "GNUmakefile" | "Dockerfile" | "CMakeLists.txt" => return Some(&HASH),
        _ => {}
    }
    let ext = name.rsplit_once('.')?.1.to_ascii_lowercase();
    if PROSE.contains(&ext.as_str()) {
        return None;
    }
    let found = BY_EXTENSION
        .iter()
        .find(|(_, exts)| exts.contains(&ext.as_str()));
    Some(found.map_or(&C_LIKE, |(syntax, _)| *syntax))
}

#[cfg(test)]
mod tests {
    use super::{PYTHON, RUST, SHELL, syntax_fenced};

    #[test]
    fn fences_read_by_their_language() {
        assert_eq!(syntax_fenced("Rust"), Some(&RUST));
        assert_eq!(syntax_fenced("python"), Some(&PYTHON));
        assert_eq!(syntax_fenced("bash"), Some(&SHELL));
        assert_eq!(syntax_fenced("text"), None, "prose stays plain");
    }
}
