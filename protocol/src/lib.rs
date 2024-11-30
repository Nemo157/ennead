#![no_std]
#![feature(iter_array_chunks, array_try_map)]

use zerocopy::{IntoBytes, TryFromBytes, byteorder::little_endian as le};

#[cfg(feature = "std")]
pub mod std;

#[cfg(feature = "embedded")]
pub mod embedded;

pub const WIDTH: u32 = 800;
pub const HEIGHT: u32 = 480;

#[derive(IntoBytes, TryFromBytes, Copy, Clone)]
#[repr(C)]
pub struct SubChunk {
    data: [u8; 3],
}

#[derive(IntoBytes, TryFromBytes, Copy, Clone)]
#[repr(C)]
pub struct Chunk {
    counter: le::U16,
    subchunks: [SubChunk; 20],
}

#[derive(IntoBytes, TryFromBytes, Copy, Clone)]
#[repr(u8)]
pub enum Command {
    Start { _unused: [u8; 62] } = 0,
    End { _unused: [u8; 62] } = 1,
    Chunk(Chunk) = 2,
}

#[derive(Copy, Clone)]
pub enum Color {
    White,
    Black,
    Green,
    Blue,
    Red,
    Yellow,
    Orange,
}

const _: () = assert!(core::mem::size_of::<Command>() == 63);

impl Chunk {
    pub fn new(counter: u16, pixels: [Color; 160]) -> Self {
        let pixels: [[Color; 8]; 20] = core::array::from_fn({
            let mut iter = pixels.into_iter().array_chunks();
            move |_| iter.next().unwrap()
        });

        Self {
            counter: counter.into(),
            subchunks: pixels.map(SubChunk::from),
        }
    }
}

impl Chunk {
    pub fn pixels(self) -> impl Iterator<Item = Color> {
        // TODO: how to handle error here
        self.subchunks.into_iter().flat_map(|subchunk| <[Color; 8]>::try_from(subchunk).unwrap())
    }
}

impl From<[Color; 8]> for SubChunk {
    fn from(pixels: [Color; 8]) -> Self {
        let [a, b, c, d, e, f, g, h] = pixels.map(u8::from);

        Self {
            data: [
                (a << 5) | (b << 2) | (c >> 1),
                (c << 7) | (d << 4) | (e << 1) | (f >> 2),
                (f << 6) | (g << 3) | h,
            ]
        }
    }
}

impl TryFrom<SubChunk> for [Color; 8] {
    type Error = <Color as TryFrom<u8>>::Error;

    fn try_from(subchunk: SubChunk) -> Result<Self, Self::Error> {
        let [a, b, c] = subchunk.data;

        let pixels = [
            (a >> 5) & 0b111,
            (a >> 2) & 0b111,
            (a << 1) & 0b110 | (b >> 7) & 0b001,
            (b >> 4) & 0b111,
            (b >> 1) & 0b111,
            (b << 2) & 0b100 | (c >> 6) & 0b011,
            (c >> 3) & 0b111,
            (c >> 0) & 0b111,
        ];

        pixels.try_map(Color::try_from)
    }
}

impl From<Color> for u8 {
    fn from(pixel: Color) -> Self {
        match pixel {
            Color::White => 0,
            Color::Black => 1,
            Color::Green => 2,
            Color::Blue => 3,
            Color::Red => 4,
            Color::Yellow => 5,
            Color::Orange => 6,
        }
    }
}

impl TryFrom<u8> for Color {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::White,
            1 => Self::Black,
            2 => Self::Green,
            3 => Self::Blue,
            4 => Self::Red,
            5 => Self::Yellow,
            6 => Self::Orange,
            _ => return Err(()),
        })
    }
}
