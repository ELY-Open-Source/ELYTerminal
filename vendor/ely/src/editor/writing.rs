//! Settings for a writing view over the code.

use gpui::Context;

use super::CodeEditor;

impl CodeEditor {
    /// Text size against the theme's, for a writing view.
    pub fn set_scale(&mut self, scale: f32, cx: &mut Context<Self>) {
        self.options.scale = scale;
        cx.notify();
    }

    /// Fades every line but the caret's.
    pub fn set_focus_line(&mut self, on: bool, cx: &mut Context<Self>) {
        self.options.focus_line = on;
        cx.notify();
    }

    /// Keeps the caret's line mid-view.
    pub fn set_typewriter(&mut self, on: bool, cx: &mut Context<Self>) {
        self.options.typewriter = on;
        self.centered = None;
        cx.notify();
    }
}
