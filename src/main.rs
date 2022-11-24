mod bmp;
mod gif;

use crate::gif::{Gif, Parse};

fn main() {
    let bytes = include_bytes!("../GifSample.gif");

    for (i, byte) in bytes.iter().enumerate() {
        println!("{1:3X}: HEX: {0:02X} DEC: {0:3}", byte, i);
    }

    println!("Parsing...");

    let gif = Gif::parse(bytes).expect("Parse failed");

    println!("{:#?}", gif);
}
