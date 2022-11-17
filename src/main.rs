mod gif;
use crate::gif::{Gif, Parse};

fn main() {
    println!("Parsing...");

    let gif = Gif::parse(&include_bytes!("../sample_640×426.gif")[..7]).expect("Parse failed");

    println!("{:#?}", gif);
}
