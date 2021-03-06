//! This module contains the configuration of the application.
//!
//! All options are passed individually to each function and are not bundled
//! together.
//!
//! # Examples
//!
//! ```no_run
//! # use doh::Options;
//! let options = Options::parse();
//! println!("Showing {}", options.remote_dir);
//! ```


use clap::{AppSettings, Arg};
use reqwest::Url;


/// Representation of the application's all configurable values.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Options {
    /// Remote directory to start on.
    pub remote_dir: Url,
}

impl Options {
    /// Parse `env`-wide command-line arguments into an `Options` instance
    pub fn parse() -> Options {
        let matches = app_from_crate!("\n")
            .setting(AppSettings::ColoredHelp)
            .arg(Arg::from_usage("<URL> 'Remote directory to browse'").validator(Options::url_validator))
            .get_matches();

        let u = matches.value_of("URL").unwrap();
        Options {
            remote_dir: Url::parse(u)
                .or_else(|_| Url::parse(&format!("http://{}", u)))
                .unwrap(),
        }
    }

    fn url_validator(s: String) -> Result<(), String> {
        Url::parse(&s)
            .or_else(|_| Url::parse(&format!("http://{}", s)))
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}
