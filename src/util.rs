//! Module containing various utility functions.


use time::{self, Tm};
use std::{f64, cmp};
use reqwest::Url;


/// The app name and version to use with User-Agent request header.
pub static USER_AGENT: &'static str = concat!("D'Oh/", env!("CARGO_PKG_VERSION"));


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
/// assert_eq!(parent_url(&"https://google.com/search/capitalism".parse().unwrap()),
///            "https://google.com/search".parse().unwrap());
/// assert_eq!(parent_url(&"https://google.com/search".parse().unwrap()),
///            "https://google.com".parse().unwrap());
/// assert_eq!(parent_url(&"https://google.com".parse().unwrap()),
///            "https://google.com".parse().unwrap());
/// ```
pub fn parent_url(u: &Url) -> Url {
    let p = u.to_string();
    p[0..p.rfind('/').unwrap()].parse().unwrap()
}
