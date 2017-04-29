extern crate reqwest;
extern crate doh;

use reqwest::Client;


fn main() {
    let opts = doh::Options::parse();
    println!("{:#?}", opts);
    let mut client = Client::new().unwrap();
    client.gzip(true);
    let resp = client.get(opts.remote_dir).header(doh::ops::RawFsApiHeader(true)).send();
    println!("{:#?}", resp);
    println!("{:#?}", resp.unwrap().json::<doh::ops::FilesetData>().unwrap());
}
