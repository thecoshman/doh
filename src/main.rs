extern crate tabwriter;
extern crate doh;

use tabwriter::TabWriter;
use std::io::{self, Write, stdout};


fn main() {
    let opts = doh::Options::parse();

    let mut cururl = opts.remote_dir.clone();
    let mut fs;
    let mut selected = 0;
    loop {
        println!("Contents of {}:", cururl);
        let mut resp = doh::ops::download(cururl.clone());
        if !resp.status().is_success() {
            println!("<Got {}... Exiting.>", resp.status());
            break; // Don't actually break here
        }
        let data = match resp.json::<doh::ops::FilesetData>() {
            Ok(d) => d,
            Err(e) => {
                println!("<Couldn't parse server response: {}... Exiting.>", e);
                cururl = doh::util::parent_url(&cururl);
                continue;
            }
        };

        if data.is_file {
            io::copy(&mut doh::ops::download_raw(cururl.clone()), &mut stdout()).unwrap();
            cururl = doh::util::parent_url(&cururl);
            println!();
        } else {
            fs = doh::ops::RemoteFile::from_response(data);
            let mut out = TabWriter::new(stdout());
            for (i, f) in fs.iter().enumerate() {
                writeln!(out, "{}{}", if i == selected { ">" } else { " " }, f).unwrap();
            }
            out.flush().unwrap();
            break; // Don't actually break here
        }
    }
}
