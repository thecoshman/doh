//! Module containing various utility functions.


use time::{self, Tm};
use std::borrow::Cow;
use std::{iter, f64, cmp};
use url::{percent_encoding, Url};


/// App name and version to use with User-Agent request header.
pub static USER_AGENT: &str = concat!("D'Oh/", env!("CARGO_PKG_VERSION"));

/// Byte returned by `getch()` for Enter.
pub const GETCH_ENTER: u8 = b'\r';
/// Byte returned by `getch()` for Escape.
pub const GETCH_ESC: u8 = b'\x1B';
/// First byte returned by `getch()` for special characters.
pub const GETCH_SPECIAL_PREFIX: u8 = 224;
/// Second byte returned by `getch()` for up arrow key.
pub const GETCH_ARROW_UP: u8 = 72;
/// Second byte returned by `getch()` for down arrow key.
pub const GETCH_ARROW_DOWN: u8 = 80;
/// Second byte returned by `getch()` for left arrow key.
pub const GETCH_ARROW_LEFT: u8 = 75;
/// Second byte returned by `getch()` for right arrow key.
pub const GETCH_ARROW_RIGHT: u8 = 77;
/// Second byte returned by `getch()` for Delete.
pub const GETCH_DELETE: u8 = 83;

/// Amount of spaces to expand tabs to.
///
/// Provided by @Ell, so flame him.
/// <span title=":noel:">![:noel:](https://cdn.discordapp.com/emojis/230277422006796288.png)</span>
pub const TAB_WIDTH: usize = 4;
lazy_static! {
    /// Filler to replace tabs with of length [`TAB_WIDTH`](constant.TAB_WIDTH.html).
    pub static ref TAB_SPACING: String = iter::repeat(' ').take(TAB_WIDTH).collect();
}


/// A RAII guard object, calling a function when it's created and when it's destroyed.
///
/// # Examples
///
/// Turn off cursor for the object's lifetime, then turn it back on.
///
/// ```
/// # use doh::util::RaiiGuard;
/// # fn cursor_visibility(_: bool) {}
/// let _cursor = RaiiGuard::new(|| cursor_visibility(false),
///                              || cursor_visibility(true));
///
/// println!("There is no cursor here.");
/// println!("Normally you'd use this if you had nonechoing keyboard input.");
///
/// println!("After this scope ends, the cursor will be restored,
///           the exit method and point nonaffecting.");
/// ```
pub struct RaiiGuard<E: FnOnce()> {
    end: Option<E>,
}

impl<E: FnOnce()> RaiiGuard<E> {
    /// Create a guard, calling the `start` function immediately, delaying the `end` function till the object is dropped.
    pub fn new<S: FnOnce()>(start: S, end: E) -> Self {
        start();
        RaiiGuard { end: Some(end) }
    }
}

impl<E: FnOnce()> Drop for RaiiGuard<E> {
    /// Call the `end` function supplied in [`new()`](#method.new).
    fn drop(&mut self) {
        self.end.take().unwrap()();
    }
}


/// Parse an RFC3339 string into a timespec.
///
/// Note: due to the specificity of the `tm` struct some fields are not
/// preserved, but have no impact on the correctness of the result:
///
/// * `tm_wday` – weekday
/// * `tm_yday` – day of the year
/// * `tm_isdst` – daylight savings time applied/not applied
///
/// # Examples
///
/// ```
/// # extern crate time;
/// # extern crate doh;
/// # use time::Tm;
/// # use doh::util::parse_rfc3339;
/// # fn main() {
/// assert_eq!(parse_rfc3339("2012-02-22T07:53:18-07:00"),
///            Ok(Tm {
///                tm_sec: 18,
///                tm_min: 53,
///                tm_hour: 7,
///                tm_mday: 22,
///                tm_mon: 1,
///                tm_year: 112,
///                tm_wday: 0,
///                tm_yday: 0,
///                tm_isdst: 0,
///                tm_utcoff: -25200,
///                tm_nsec: 0,
///            }));
/// assert_eq!(parse_rfc3339("2012-02-22T14:53:18.42Z"),
///            Ok(Tm {
///                tm_sec: 18,
///                tm_min: 53,
///                tm_hour: 14,
///                tm_mday: 22,
///                tm_mon: 1,
///                tm_year: 112,
///                tm_wday: 0,
///                tm_yday: 0,
///                tm_isdst: 0,
///                tm_utcoff: 0,
///                tm_nsec: 420000000,
///            }));
/// # }
/// ```
pub fn parse_rfc3339<S: AsRef<str>>(from: S) -> Result<Tm, time::ParseError> {
    let utc = from.as_ref().chars().last() == Some('Z');
    let fractional = from.as_ref().len() > if utc { 20 } else { 25 };
    time::strptime(from.as_ref(),
                   match (utc, fractional) {
                       (true, false) => "%Y-%m-%dT%H:%M:%SZ",
                       (true, true) => "%Y-%m-%dT%H:%M:%S.%fZ",
                       (false, true) => "%Y-%m-%dT%H:%M:%S.%f%z",
                       (false, false) => "%Y-%m-%dT%H:%M:%S%z",
                   })
}

/// Construct string representing a human-readable size.
///
/// Stolen, adapted and inlined from [fielsize.js](http://filesizejs.com).
pub fn human_readable_size(s: u64) -> String {
    lazy_static! {
        static ref LN_KIB: f64 = 1024f64.log(f64::consts::E);
    }

    if s == 0 {
        "0B".to_string()
    } else {
        let num = s as f64;
        let exp = cmp::min(cmp::max((num.log(f64::consts::E) / *LN_KIB) as i32, 0), 8);

        let val = num / 2f64.powi(exp * 10);

        if exp > 0 {
                (val * 10f64).round() / 10f64
            } else {
                val.round()
            }
            .to_string() + ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB"][cmp::max(exp, 0) as usize]
    }
}

/// Get a URL that is canonically considered a "parent" to the specified one, or the specified URL if it is a root URL.
///
/// # Examples
///
/// ```
/// # use doh::util::parent_url;
/// assert_eq!(parent_url(&"https://google.com/search/capitalism/".parse().unwrap()),
///            "https://google.com/search".parse().unwrap());
/// assert_eq!(parent_url(&"https://google.com/search".parse().unwrap()),
///            "https://google.com".parse().unwrap());
/// assert_eq!(parent_url(&"https://google.com".parse().unwrap()),
///            "https://google.com".parse().unwrap());
/// ```
pub fn parent_url(u: &Url) -> Url {
    let p = u.to_string();
    p[0..p[0..p.len() - ((p.chars().last() == Some('/')) as usize)].rfind('/').unwrap()].parse().unwrap_or_else(|_| u.clone())
}

/// Decode a percent-encoded string (like a part of a URL).
///
/// # Example
///
/// ```
/// # use doh::util::percent_decode;
/// # use std::borrow::Cow;
/// assert_eq!(percent_decode("%D0%B0%D1%81%D0%B4%D1%84%20fdsa"), Some(Cow::from("асдф fdsa")));
/// assert_eq!(percent_decode("%D0%D1%81%D0%B4%D1%84%20fdsa"), None);
/// ```
pub fn percent_decode(s: &str) -> Option<Cow<str>> {
    percent_encoding::percent_decode(s.as_bytes()).decode_utf8().ok()
}

/// Get max amount of listed elements per page, based on term height
///
/// Leaves out 1 line for header and 1 line for spacing
pub fn max_listing_lines(term_h: usize) -> usize {
    term_h - 2
}
