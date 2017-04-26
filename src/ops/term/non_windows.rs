extern crate tinyfiledialogs;

use std::path::Path;
use self::tinyfiledialogs::save_file_dialog;


/// Move the cursor `n` lines up.
pub fn move_cursor_up(n: usize) -> String {
    format!("\x1B[{}A", n)
}

/// Move the cursor `n` lines down.
pub fn move_cursor_down(n: usize) -> String {
    format!("\x1B[{}B", n)
}

/// Move the cursor `n` characters back (left).
pub fn move_cursor_back(n: usize) -> String {
    format!("\x1B[{}D", n)
}

/// Show/hide the cursor/caret.
pub fn show_cursor(show: bool) -> &'static str {
    if show { "\x1B[?25h" } else { "\x1B[?25l" }
}

/// Show a file picker to let user choose where to save a file with the pecified filename and optional extension.
pub fn save_file_picker(filename: &OsStr, extension: Option<&OsStr>) -> Option<PathBuf> {
    let _ = extension;
    save_file_dialog("Pick save location.", &Path::new(filename).display().to_string()).map(|s| s.into())
}
