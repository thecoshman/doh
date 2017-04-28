extern crate reqwest;


fn main() {
    println!("{:#?}", reqwest::get("https://google.com"));
}
