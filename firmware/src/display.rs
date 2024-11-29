use embedded_hal::digital::OutputPin;
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use panic_halt as _;

use waveshare_rp2040_epaper_73::{
    hal::{pac, spi, Timer},
    EpdBusy, EpdDc, EpdReset, EpdSpiClock, EpdSpiCs, EpdSpiTx, LedActivity,
};

use embedded_graphics::{
    draw_target::{DrawTarget, DrawTargetExt},
    geometry::{AnchorPoint, Dimensions, Point, Size},
    mono_font::{ascii::FONT_9X18_BOLD, MonoTextStyle},
    primitives::{Primitive, PrimitiveStyle, Rectangle, Triangle},
    text::Text,
    Drawable,
};

use epd_waveshare::{
    color::OctColor,
    epd7in3f::{Display7in3f, Epd7in3f},
    graphics::DisplayRotation,
    prelude::WaveshareDisplay,
};

type Spi = ExclusiveDevice<
    spi::Spi<spi::Enabled, pac::SPI1, (EpdSpiTx, EpdSpiClock), 8>,
    EpdSpiCs,
    NoDelay,
>;
type Device = Epd7in3f<Spi, EpdBusy, EpdDc, EpdReset, Timer>;

pub struct Display {
    spi: Spi,
    device: Device,
    display: Display7in3f,
    rotations: core::iter::Cycle<core::array::IntoIter<DisplayRotation, 4>>,
    colors: core::iter::Cycle<core::array::IntoIter<OctColor, 5>>,
}

impl Display {
    pub fn new(
        mut spi: Spi,
        epd_busy: EpdBusy,
        epd_dc: EpdDc,
        epd_reset: EpdReset,
        timer: &mut Timer,
    ) -> Result<Self, crate::error::Infallible> {
        Ok(Self {
            device: Epd7in3f::new(&mut spi, epd_busy, epd_dc, epd_reset, timer, None)?,
            display: Display7in3f::default(),
            spi,
            rotations: [
                DisplayRotation::Rotate0,
                DisplayRotation::Rotate90,
                DisplayRotation::Rotate180,
                DisplayRotation::Rotate270,
            ]
            .into_iter()
            .cycle(),
            colors: [
                OctColor::Green,
                OctColor::Blue,
                OctColor::Red,
                OctColor::Yellow,
                OctColor::Orange,
            ]
            .into_iter()
            .cycle(),
        })
    }

    pub fn next(
        &mut self,
        timer: &mut Timer,
        activity: &mut LedActivity,
    ) -> Result<(), crate::error::Infallible> {
        activity.set_high()?;
        self.display.set_rotation(self.rotations.next().unwrap());
        self.display.clear(OctColor::White)?;

        // We draw to a square section in the center of the display
        {
            let mut display = self.display.cropped(&self.display.bounding_box().resized(
                Size {
                    width: 480,
                    height: 480,
                },
                AnchorPoint::Center,
            ));

            let w2 = 30;
            let w4 = 60;
            let w8 = 120;
            let w16 = 240;

            Triangle::new(
                Point::new(w8, w8),
                Point::new(w8 + w4, w4),
                Point::new(w8 + w8, w8),
            )
            .into_styled(PrimitiveStyle::with_fill(self.colors.next().unwrap()))
            .draw(&mut display)?;

            Triangle::new(
                Point::new(w16, w8),
                Point::new(w16 + w4, w4),
                Point::new(w16 + w8, w8),
            )
            .into_styled(PrimitiveStyle::with_fill(self.colors.next().unwrap()))
            .draw(&mut display)?;

            Rectangle::new(
                Point::new(w8 + w2, w8),
                Size::new(w4 as u32, (w8 + w4) as u32),
            )
            .into_styled(PrimitiveStyle::with_fill(self.colors.next().unwrap()))
            .draw(&mut display)?;

            Rectangle::new(
                Point::new(w16 + w2, w8),
                Size::new(w4 as u32, (w8 + w4) as u32),
            )
            .into_styled(PrimitiveStyle::with_fill(self.colors.next().unwrap()))
            .draw(&mut display)?;

            let text_style = MonoTextStyle::new(&FONT_9X18_BOLD, self.colors.next().unwrap());
            Text::new("UP", Point::new(w16 - 9, w16 + w4 + w2), text_style).draw(&mut display)?;
        }

        self.device.wake_up(&mut self.spi, timer)?;

        // Display updated frame
        self.device
            .update_frame(&mut self.spi, &self.display.buffer(), timer)?;

        self.device.display_frame(&mut self.spi, timer)?;

        self.device.sleep(&mut self.spi, timer)?;

        activity.set_low()?;

        Ok(())
    }
}
