use embedded_graphics_core::{Pixel, geometry::Point};
use epd_waveshare::color::OctColor;

use super::{Color, Chunk};

pub const PALETTE: [OctColor; 7] = [OctColor::White, OctColor::Black, OctColor::Green, OctColor::Blue, OctColor::Red, OctColor::Yellow, OctColor::Orange];

impl Chunk {
    pub fn oct_pixels(&self) -> impl Iterator<Item = Pixel<OctColor>> {
        self.pixels().map(|((x, y), color)| Pixel(Point::new(i32::from(x), i32::from(y)), OctColor::from(color)))
    }
}

impl From<Color> for OctColor {
    fn from(color: Color) -> Self {
        match color {
            Color::White => Self::White,
            Color::Black => Self::Black,
            Color::Green => Self::Green,
            Color::Blue => Self::Blue,
            Color::Red => Self::Red,
            Color::Yellow => Self::Yellow,
            Color::Orange => Self::Orange,
        }
    }
}
