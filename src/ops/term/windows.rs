extern crate kernel32;
extern crate winapi;

use std::mem::size_of;
use std::ptr::null_mut;
use std::path::PathBuf;
use std::ffi::{OsString, OsStr};
use std::os::windows::ffi::{OsStringExt, OsStrExt};
use self::kernel32::{SetConsoleCursorPosition, GetConsoleScreenBufferInfo, GetConsoleCursorInfo, SetConsoleCursorInfo, GetStdHandle};
use self::winapi::{CONSOLE_SCREEN_BUFFER_INFO, CONSOLE_CURSOR_INFO, STD_OUTPUT_HANDLE, OFN_NOCHANGEDIR, LPOPENFILENAMEW, OPENFILENAMEW, HANDLE, SMALL_RECT,
                   COORD, DWORD, WCHAR, BOOL};

#[link(name="comdlg32")]
extern "system" {
    fn GetSaveFileNameW(lpofn: LPOPENFILENAMEW) -> BOOL;
    fn GetOpenFileNameW(lpofn: LPOPENFILENAMEW) -> BOOL;
}


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

/// Show a file picker to let user choose where to save a file with the specified filename and optional extension.
pub fn save_file_picker(filename: &OsStr, extension: Option<&OsStr>) -> Option<PathBuf> {
    let ext: Vec<WCHAR> = if let Some(extension) = extension {
        extension.encode_wide().chain([0].iter().cloned()).collect()
    } else {
        vec![]
    };
    let filter: Vec<WCHAR> = if let Some(extension) = extension {
        OsStr::new(&format!("All Files\0*.*\0{0} files\0*.{0}\0\0", extension.to_string_lossy())[..]).encode_wide().collect()
    } else {
        OsStr::new("All Files\0*.*\0\0").encode_wide().collect()
    };
    let mut file = [0u16; 1024];
    for (i, b) in filename.encode_wide().enumerate() {
        file[i] = b;
    }

    picker(GetSaveFileNameW,
           ofnw(filter.as_ptr(),
                if extension.is_some() { 2 } else { 1 },
                file.as_mut_ptr(),
                file.len() as DWORD,
                if extension.is_some() {
                    ext.as_ptr()
                } else {
                    null_mut()
                }),
           &file)
}

/// Show a file picker to let user choose a file.
pub fn open_file_picker() -> Option<PathBuf> {
    let filter: Vec<WCHAR> = OsStr::new("All Files\0*.*\0\0").encode_wide().collect();
    let mut file = [0u16; 1024];
    picker(GetOpenFileNameW,
           ofnw(filter.as_ptr(), 1, file.as_mut_ptr(), file.len() as DWORD, null_mut()),
           &file)
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

fn ofnw(filter: *const WCHAR, filter_index: DWORD, file: *mut WCHAR, file_len: DWORD, default_extension: *const WCHAR) -> OPENFILENAMEW {
    OPENFILENAMEW {
        lStructSize: size_of::<OPENFILENAMEW>() as DWORD,
        hwndOwner: null_mut(),
        hInstance: null_mut(),
        lpstrFilter: filter,
        lpstrCustomFilter: null_mut(),
        nMaxCustFilter: 0,
        nFilterIndex: filter_index,
        lpstrFile: file,
        nMaxFile: file_len,
        lpstrFileTitle: null_mut(),
        nMaxFileTitle: 0,
        lpstrInitialDir: null_mut(),
        lpstrTitle: null_mut(),
        Flags: OFN_NOCHANGEDIR,
        nFileOffset: 0,
        nFileExtension: 0,
        lpstrDefExt: default_extension,
        lCustData: 0,
        lpfnHook: None,
        lpTemplateName: null_mut(),
        pvReserved: null_mut(),
        dwReserved: 0,
        FlagsEx: 0,
    }
}

fn picker(f: unsafe extern "system" fn(LPOPENFILENAMEW) -> BOOL, mut ofw: OPENFILENAMEW, file: &[WCHAR]) -> Option<PathBuf> {
    if unsafe { f(&mut ofw as LPOPENFILENAMEW) } == 0 {
        None
    } else {
        Some(OsString::from_wide(&file[0..file.iter().position(|&b| b == 0).unwrap_or(file.len())]).into())
    }
}
