//! Windows and non-Windows terminal utilities.
//!
//! All functions in this module return something printable, and you should print their results,
//! but do *not* assume that it's the returned text that does stuff,
//! i.e. do *not* mix terminal manipulation and printing into the same call,
//! and *always* flush after printing and before using the functions.
//!
//! # Examples
//!
//! ```
//! # use doh::ops::term::{move_cursor_back, move_cursor_down, move_cursor_up};
//! # use std::io::{Write, stdout};
//! print!("{}{}", move_cursor_up(5), move_cursor_back(3));
//! print!("HLO");
//! stdout().flush().unwrap();
//! print!("{}", move_cursor_down(5));
//! ```


#[cfg(target_os="windows")]
mod windows;
#[cfg(not(target_os="windows"))]
mod non_windows;

#[cfg(target_os="windows")]
pub use self::windows::*;
#[cfg(not(target_os="windows"))]
pub use self::non_windows::*;
