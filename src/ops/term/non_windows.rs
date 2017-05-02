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
