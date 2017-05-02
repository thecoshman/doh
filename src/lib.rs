pub extern crate rfsapi;

#[macro_use]
extern crate lazy_static;
extern crate itertools;
extern crate tabwriter;
extern crate reqwest;
extern crate getch;
#[macro_use]
extern crate clap;
extern crate time;

mod options;

pub mod ops;
pub mod util;

pub use self::options::Options;
