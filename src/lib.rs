#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate lazy_static;
extern crate itertools;
extern crate tabwriter;
extern crate reqwest;
extern crate getch;
extern crate serde;
#[macro_use]
extern crate clap;
extern crate time;
extern crate mime;
extern crate url;

mod options;

pub mod ops;
pub mod util;

pub use self::options::Options;
