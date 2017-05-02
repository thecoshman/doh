extern crate kernel32;
extern crate winapi;

use self::winapi::{CONSOLE_SCREEN_BUFFER_INFO, CONSOLE_CURSOR_INFO, STD_OUTPUT_HANDLE, HANDLE, SMALL_RECT, COORD, BOOL};
use self::kernel32::{SetConsoleCursorPosition, GetConsoleScreenBufferInfo, GetConsoleCursorInfo, SetConsoleCursorInfo, GetStdHandle};


/// Move the cursor `n` lines up.
pub fn move_cursor_up(n: usize) -> String {
    move_cursor(0, -(n as i16))
}

/// Move the cursor `n` lines down.
pub fn move_cursor_down(n: usize) -> String {
    move_cursor(0, n as i16)
}

/// Move the cursor `n` characters back (left).
pub fn move_cursor_back(n: usize) -> String {
    move_cursor(-(n as i16), 0)
}

/// Show/hide the cursor/caret.
pub fn show_cursor(show: bool) -> &'static str {
    let hand = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
    let mut cursor = CONSOLE_CURSOR_INFO {
        dwSize: 0,
        bVisible: 0,
    };

    unsafe { GetConsoleCursorInfo(hand, &mut cursor) };
    cursor.bVisible = show as BOOL;
    unsafe { SetConsoleCursorInfo(hand, &mut cursor) };

    ""
}


fn move_cursor(x: i16, y: i16) -> String {
    if let Some((hand, csbi)) = get_csbi() {
        unsafe {
            SetConsoleCursorPosition(hand,
                                     COORD {
                                         X: csbi.dwCursorPosition.X + x,
                                         Y: csbi.dwCursorPosition.Y + y,
                                     });
        }
    }
    String::new()
}

fn get_csbi() -> Option<(HANDLE, CONSOLE_SCREEN_BUFFER_INFO)> {
    let hand = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };

    let zc = COORD { X: 0, Y: 0 };
    let mut csbi = CONSOLE_SCREEN_BUFFER_INFO {
        dwSize: zc,
        dwCursorPosition: zc,
        wAttributes: 0,
        srWindow: SMALL_RECT {
            Left: 0,
            Top: 0,
            Right: 0,
            Bottom: 0,
        },
        dwMaximumWindowSize: zc,
    };
    match unsafe { GetConsoleScreenBufferInfo(hand, &mut csbi) } {
        0 => None,
        _ => Some((hand, csbi)),
    }
}
