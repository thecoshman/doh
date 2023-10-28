extern crate term_size;
extern crate getch;
extern crate doh;

use getch::Getch;
use std::process::exit;
use std::io::{Write, stdout, stderr};


fn main() {
    let ec = handler_main();
    exit(ec);
}

fn handler_main() -> i32 {
    if let Err((msg, errc)) = real_main() {
        let _ = writeln!(stderr(), "{}", msg);
        errc
    } else {
        0
    }
}

fn real_main() -> Result<(), (String, i32)> {
    let opts = doh::Options::parse();
    let termsize = term_size::dimensions().ok_or_else(|| ("Unknown terminal dimensions.".to_string(), 1))?;
    let _cursor = doh::util::RaiiGuard::new(|| print!("{}", doh::ops::term::show_cursor(false)),
                                            || print!("{}", doh::ops::term::show_cursor(true)));

    let input = Getch::new();
    let mut ctx = doh::ops::ListContext::new(opts.remote_dir.clone());
    while ctx.one_loop(&mut stdout(), &input, termsize).map_err(|e| (format!("Listing failure: {}", e), 3))? {
        println!();
    }

    Ok(())
}
