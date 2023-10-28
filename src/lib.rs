pub extern crate rfsapi;

extern crate itertools;
extern crate tabwriter;
extern crate reqwest;
extern crate getch;
#[macro_use]
extern crate clap;
extern crate time;
extern crate url;
extern crate pbr;

mod options;

pub mod ops;
pub mod util;

pub use self::options::Options;
