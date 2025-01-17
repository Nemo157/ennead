use std::vec::Vec;

use image::{GenericImageView, Rgb};

use super::{Chunk, Color, Command, HEIGHT, WIDTH};

const WHITE: Rgb<u8> = image::Rgb([255, 255, 255]);
const BLACK: Rgb<u8> = image::Rgb([0, 0, 0]);
const GREEN: Rgb<u8> = image::Rgb([0, 255, 0]);
const BLUE: Rgb<u8> = image::Rgb([0, 0, 255]);
const RED: Rgb<u8> = image::Rgb([255, 0, 0]);
const YELLOW: Rgb<u8> = image::Rgb([255, 255, 0]);
const ORANGE: Rgb<u8> = image::Rgb([255, 128, 0]);

pub const PALETTE: [image::Rgb<u8>; 7] = [WHITE, BLACK, GREEN, BLUE, RED, YELLOW, ORANGE];

impl Command {
    pub fn from_image(image: &impl GenericImageView<Pixel = Rgb<u8>>) -> Vec<Self> {
        assert!(image.dimensions() == (WIDTH, HEIGHT));

        [Self::Start { _unused: [0; 62] }]
            .into_iter()
            .chain(
                image
                    .pixels()
                    .map(|(_, _, pixel)| pixel)
                    .array_chunks()
                    .zip(0..)
                    .map(|(pixels, counter)| {
                        Self::Chunk(Chunk::new(
                            counter,
                            pixels
                                .map(|pixel| Color::try_from(pixel).expect("non-palettized image")),
                        ))
                    }),
            )
            .chain([Self::End { _unused: [0; 62] }])
            .collect()
    }
}

impl From<Color> for Rgb<u8> {
    fn from(color: Color) -> Self {
        match color {
            Color::White => WHITE,
            Color::Black => BLACK,
            Color::Green => GREEN,
            Color::Blue => BLUE,
            Color::Red => RED,
            Color::Yellow => YELLOW,
            Color::Orange => ORANGE,
        }
    }
}

impl TryFrom<Rgb<u8>> for Color {
    type Error = ();

    fn try_from(rgb: Rgb<u8>) -> Result<Self, ()> {
        Ok(match rgb {
            WHITE => Color::White,
            BLACK => Color::Black,
            GREEN => Color::Green,
            BLUE => Color::Blue,
            RED => Color::Red,
            YELLOW => Color::Yellow,
            ORANGE => Color::Orange,
            _ => return Err(()),
        })
    }
}
