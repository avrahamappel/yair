//! GIF format
//! <https://en.wikipedia.org/wiki/GIF>
#![allow(unused)]

use nom::branch::alt;
use nom::bytes::complete::{is_a, tag, take, take_until1};
use nom::combinator::{map, map_res};
use nom::multi::{count, many1};
use nom::sequence::{pair, separated_pair, terminated, tuple};
use nom::{bits, IResult, Parser};

pub trait Parse
where
    Self: Sized,
{
    fn parse(input: &[u8]) -> IResult<&[u8], Self>;
}

macro_rules! le_int_from_bytes {
    ($int:tt, $bytes:expr) => {
        $int::from_le_bytes($bytes.try_into().expect("conversion from le failed"))
    };
}

#[derive(Debug, PartialEq, Eq)]
enum Version {
    Gif87a,
    Gif89a,
}

impl Parse for Version {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        map(
            alt((tag(b"GIF87a"), tag(b"GIF89a"))),
            |version: &[u8]| match version {
                b"GIF87a" => Self::Gif87a,
                b"GIF89a" => Self::Gif89a,
                _ => unreachable!(),
            },
        )(input)
    }
}

#[derive(Debug)]
struct ColorTable {
    colors: Vec<Vec<u8>>,
}

impl ColorTable {
    fn parse(input: &[u8], size: usize) -> IResult<&[u8], Self> {
        map(count(take(size), 256), |colors| Self {
            colors: colors.into_iter().map(|c: &[u8]| c.to_vec()).collect(),
        })(input)
    }
}

/// Highest bit indicates presence, lowest three bits indicate length
fn color_table_spec(byte: u8) -> Option<usize> {
    ((byte | 0b10000000) >> 7 == 1).then_some(((byte | 0b00000111) as usize * 255) + 1)
}

#[derive(Debug)]
struct LogicalScreenDescriptor {
    width: u16,
    height: u16,
    global_color_table: Option<ColorTable>,
    bg_color: u8,
    pixel_aspect_ratio: u8,
}

impl Parse for LogicalScreenDescriptor {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        tuple((
            // width
            take(2usize),
            // height
            take(2usize),
            // GCT indicator
            take(1usize),
            // background color
            take(1usize),
            // pixel aspect ratio
            take(1usize),
        ))(input)
        .and_then(|(rest, (w, h, gct, bg, px))| {
            let (rest, global_color_table) = match color_table_spec(gct[0]) {
                Some(size) => ColorTable::parse(rest, size).map(|(r, ct)| (r, Some(ct)))?,
                None => (rest, None),
            };

            let lsd = Self {
                width: le_int_from_bytes!(u16, w),
                height: le_int_from_bytes!(u16, h),
                global_color_table,
                bg_color: bg[0],
                pixel_aspect_ratio: px[0],
            };

            Ok((rest, lsd))
        })
    }
}

#[derive(Debug)]
struct ImageDescriptor {
    position: (u16, u16),
    width: u16,
    height: u16,
    local_color_table: Option<ColorTable>,
}

impl Parse for ImageDescriptor {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        tuple((
            take(2usize),
            take(2usize),
            take(2usize),
            take(2usize),
            take(1usize),
        ))(input)
        .and_then(|(rest, (x, y, w, h, lct))| {
            let (rest, local_color_table) = match color_table_spec(lct[0]) {
                Some(size) => ColorTable::parse(input, size).map(|(r, ct)| (r, Some(ct)))?,
                None => (rest, None),
            };

            let id = Self {
                position: (le_int_from_bytes!(u16, x), le_int_from_bytes!(u16, y)),
                width: le_int_from_bytes!(u16, w),
                height: le_int_from_bytes!(u16, h),
                local_color_table,
            };

            Ok((rest, id))
        })
    }
}

#[derive(Debug)]
struct Header {
    version: Version,
    screen_descriptor: LogicalScreenDescriptor,
}

impl Parse for Header {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        map(
            pair(Version::parse, LogicalScreenDescriptor::parse),
            |(version, screen_descriptor)| Self {
                version,
                screen_descriptor,
            },
        )(input)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct SubBlock {
    // Should be generated
    // length: u8,
    data: Vec<u8>,
    // Null block
    // end: u8,
}

impl Parse for SubBlock {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        map(take_until1(b"\0".as_slice()), |data: &[u8]| Self {
            data: data.to_vec(),
        })(input)
    }
}

type SubBlocks = Vec<SubBlock>;

#[derive(Debug)]
struct ImageData {
    bit_width: u8,
    data: SubBlocks,
}

impl Parse for ImageData {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        map(
            pair(take(1usize), many1(SubBlock::parse)),
            |(bit_width, data)| Self {
                bit_width: bit_width[0],
                data,
            },
        )(input)
    }
}

#[derive(Debug)]
struct Image {
    image_descriptor: ImageDescriptor,
    image_data: ImageData,
}

impl Parse for Image {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        map(
            pair(ImageDescriptor::parse, ImageData::parse),
            |(image_descriptor, image_data)| Self {
                image_descriptor,
                image_data,
            },
        )(input)
    }
}

#[derive(Debug)]
enum ExtensionType {
    GraphicControl,
    Unknown,
}

impl From<u8> for ExtensionType {
    fn from(byte: u8) -> Self {
        match byte {
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug)]
struct Extension {
    ext_type: ExtensionType,
    data: SubBlocks,
}

impl Parse for Extension {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        map(
            pair(take(1usize), many1(SubBlock::parse)),
            |(type_byte, data)| Self {
                ext_type: type_byte[0].into(),
                data,
            },
        )(input)
    }
}

#[derive(Debug)]
enum Block {
    Image(Image),
    Extension(Extension),
}

impl Parse for Block {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        map_res(
            pair(is_a(b",!".as_slice()), take_until1(b",!;".as_slice())),
            |(sent, block): (&[u8], &[u8])| match sent[0] {
                b',' => Image::parse(block).map(|(_, img)| Self::Image(img)),
                b'!' => Extension::parse(block).map(|(_, ext)| Self::Extension(ext)),
                _ => unreachable!(),
            },
        )(input)
    }
}

impl Block {
    fn sentinel(&self) -> u8 {
        match self {
            Self::Image(_) => b',',
            Self::Extension(_) => b'!',
        }
    }
}

#[derive(Debug)]
pub struct Gif {
    header: Header,
    blocks: Vec<Block>,
}

impl Parse for Gif {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        map(
            terminated(pair(Header::parse, many1(Block::parse)), tag(b";")),
            |(header, blocks)| Self { header, blocks },
        )(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version() {
        for (input, expected) in [(b"GIF87a", Version::Gif87a), (b"GIF89a", Version::Gif89a)] {
            assert_eq!(Version::parse(input).unwrap().1, expected);
        }
    }

    #[test]
    fn parse_sub_block() {
        assert_eq!(
            SubBlock {
                data: vec![b'a', b'b', b'c']
            },
            SubBlock::parse(b"abc\0").unwrap().1
        );
    }
}
