mod anchors;
mod buffer;
mod chat;
mod console;
mod cursor;
mod decor;
mod edit;
mod extensions;
mod filters;
mod find;
mod gutter;
mod hints;
mod hover;
mod ime;
mod keybindings;
mod keys;
mod layout;
mod line;
mod measure;
mod moves;
mod preview;
mod problems;
mod prose;
mod refactor;
mod row;
mod search;
mod settings;
mod state;
mod status;
mod symbols;
mod syntax;
mod tasks;
#[cfg(all(test, feature = "test-support"))]
mod tests;
mod view;
mod welcome;
mod writing;

pub use buffer::Buffer;
pub use chat::InlineChat;
pub use console::{ConsoleEntry, DebugConsole, OutputPanel};
pub use cursor::Selection;
pub use decor::{
    CodeLens, Diagnostic, DiffHunk, GhostText, GitMark, InlayHint, ReadOnlyBanner, VimMode,
    VimModeIndicator,
};
pub use extensions::{Extension, ExtensionAction, ExtensionState, ExtensionsPanel};
pub use filters::{SearchFilters, passes};
pub use find::{FindOptions, FindWidget, find_all};
pub use hints::{CompletionItem, CompletionMenu, Signature, SignatureHelp, completions};
pub use hover::HoverInfo;
pub use keybindings::{KeySource, Keybinding, KeybindingsEditor, conflicts};
pub(crate) use keys::{Newline, Paste, bind_keys};
pub(crate) use layout::stack;
pub use preview::ThemePreview;
pub use problems::{Problem, ProblemsPanel};
pub use refactor::{ActionKind, CodeAction, CodeActionMenu, RenameInput};
pub use search::{FileHits, Hit, ReferencesPanel, SearchPanel, SearchResultItem};
pub use settings::{Setting, SettingValue, SettingsEditor, settings_json};
pub use state::{CodeEditor, CursorShape, EditorEvent, LineNumbers};
pub use status::{
    BranchIndicator, CursorPosition, Encoding, IndentSettings, LanguageMode, LineEnding, LspState,
    LspStatus, NotificationBell,
};
pub use symbols::{Call, CallHierarchy, GoToSymbol, Symbol, SymbolKind, SymbolOutline};
pub(crate) use syntax::{colors as code_colors, colors_with as code_colors_with};
pub use tasks::{Task, TaskRunner, TaskState};
pub use welcome::{Project, ProjectSwitcher, RecentProjects, WelcomePage};
