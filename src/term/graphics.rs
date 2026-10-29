//! Kitty graphics commands, as the terminal hands them over.

use alacritty_terminal::index::Point;

/// One session's kitty graphics commands.
pub struct Graphics {
    pub pid: u32,
}

impl alacritty_terminal::term::Graphics for Graphics {
    fn command(&mut self, command: &[u8], cursor: Point) -> Option<String> {
        let shown = &command[..command.len().min(64)];
        log::debug!(
            "graphics {}: {} bytes at {}, {}: {}",
            self.pid,
            command.len(),
            cursor.line.0,
            cursor.column.0,
            String::from_utf8_lossy(shown)
        );
        None
    }
}
