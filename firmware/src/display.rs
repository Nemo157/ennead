use embedded_hal::digital::OutputPin;
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use panic_halt as _;
use ἐννεάς_protocol::Chunk;

use waveshare_rp2040_epaper_73::{
    hal::{pac, spi, Timer},
    EpdBusy, EpdDc, EpdReset, EpdSpiClock, EpdSpiCs, EpdSpiTx, LedActivity,
};

use embedded_graphics::draw_target::DrawTarget;

use epd_waveshare::{
    color::OctColor,
    epd7in3f::{Display7in3f, Epd7in3f},
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
        })
    }

    pub fn clear(&mut self) {
        self.display.clear(OctColor::White).unwrap();
    }

    pub fn update(&mut self, chunk: Chunk) {
        self.display.draw_iter(chunk.oct_pixels()).unwrap();
    }

    pub fn show(
        &mut self,
        timer: &mut Timer,
        activity: &mut LedActivity,
    ) -> Result<(), crate::error::Infallible> {
        activity.set_high()?;

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
