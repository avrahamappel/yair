/// BMP file parser
/// https://en.wikipedia.org/wiki/BMP_file_format
#[allow(unused)]

struct Bmp {
    bitmap_file_header: BitmapFileHeader,
    dib_header: DibHeader,
    extra_bitmasks: Option<Vec<BitMask>>,
}
