extern crate reqwest;
extern crate doh;


fn main() {
    let opts = doh::Options::parse();
    println!("{:#?}", opts);
    println!("{:#?}", reqwest::get(opts.remote_dir));
}
