#![no_std]
#![no_main]

use embedded_hal::{delay::DelayNs, digital::OutputPin};
use embedded_hal_bus::spi::ExclusiveDevice;
use panic_halt as _;
use usbd_serial::SerialPort;
use usb_device::{device::{UsbDeviceBuilder, StringDescriptors, UsbVidPid}, bus::UsbBusAllocator};
use heapless::String;
use core::fmt::Write;

use fugit::RateExtU32;
use waveshare_rp2040_epaper_73::{
    hal::{clocks::init_clocks_and_plls, pac, timer::Timer, watchdog::Watchdog, Clock, Sio, Spi, usb::UsbBus},
    EpdBusy, EpdDc, EpdPowerEnable, EpdReset, EpdSpiClock, EpdSpiCs, EpdSpiTx, LedActivity,
    LedPower, Pins, XOSC_CRYSTAL_FREQ,
};

use embedded_graphics::{
    geometry::{Point, Size, AnchorPoint, Dimensions},
    primitives::{Primitive, PrimitiveStyle, Triangle, Rectangle},
    text::Text,
    draw_target::{DrawTarget, DrawTargetExt},
    Drawable,
    mono_font::{MonoTextStyle, ascii::FONT_9X18_BOLD},
};

use epd_waveshare::{
    color::OctColor,
    epd7in3f::{Display7in3f, Epd7in3f},
    graphics::DisplayRotation,
    prelude::WaveshareDisplay,
};

#[cortex_m_rt::entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    let clocks = init_clocks_and_plls(
        XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut led_power: LedPower = pins.led_power.reconfigure();
    let mut led_activity: LedActivity = pins.led_activity.reconfigure();

    // Set up the USB driver
    let usb_bus = UsbBusAllocator::new(UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let mut serial = SerialPort::new(&usb_bus);

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .strings(&[StringDescriptors::default()
            .manufacturer("Fake company")
            .product("Serial port")
            .serial_number("TEST")])
        .unwrap()
        .device_class(2) // from: https://www.usb.org/defined-class-codes
        .build();

    let epd_busy: EpdBusy = pins.epd_busy.reconfigure();
    let epd_dc: EpdDc = pins.epd_dc.reconfigure();
    let mut epd_power_enable: EpdPowerEnable = pins.epd_power_enable.reconfigure();
    let epd_reset: EpdReset = pins.epd_reset.reconfigure();
    let epd_spi_clock: EpdSpiClock = pins.epd_spi_clock.reconfigure();
    let epd_spi_cs: EpdSpiCs = pins.epd_spi_cs.reconfigure();
    let epd_spi_tx: EpdSpiTx = pins.epd_spi_tx.reconfigure();

    epd_power_enable.set_high().unwrap();

    let spi = Spi::<_, _, _, 8>::new(pac.SPI1, (epd_spi_tx, epd_spi_clock));

    let spi = spi.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        8.MHz(),
        embedded_hal::spi::MODE_0,
    );

    let mut spi = ExclusiveDevice::new_no_delay(spi, epd_spi_cs).unwrap();

    let mut epd = Epd7in3f::new(&mut spi, epd_busy, epd_dc, epd_reset, &mut timer, None).unwrap();

    let mut display = Display7in3f::default();

    let mut rotations = [
        DisplayRotation::Rotate0,
        DisplayRotation::Rotate90,
        DisplayRotation::Rotate180,
        DisplayRotation::Rotate270,
    ].into_iter().cycle();

    let mut colors = [
        OctColor::Green,
        OctColor::Blue,
        OctColor::Red,
        OctColor::Yellow,
        OctColor::Orange,
    ].into_iter().cycle();

    led_power.set_high().unwrap();

    let mut said_hello = false;
    loop {
        // A welcome message at the beginning
        if !said_hello && timer.get_counter().ticks() >= 2_000_000 {
            led_activity.set_high().unwrap();
            timer.delay_ms(500);

            said_hello = true;
            let _ = serial.write(b"Hello, World!\n");

            timer.delay_ms(500);

            let time = timer.get_counter().ticks();
            let mut text: String<64> = String::new();
            writeln!(&mut text, "Current timer ticks: {}", time).unwrap();

            // This only works reliably because the number of bytes written to
            // the serial port is smaller than the buffers available to the USB
            // peripheral. In general, the return value should be handled, so that
            // bytes not transferred yet don't get lost.
            let _ = serial.write(text.as_bytes());

            timer.delay_ms(500);
            led_activity.set_low().unwrap();
        }

        let mut next = false;

        // Check for new data
        if usb_dev.poll(&mut [&mut serial]) {
            let mut buf = [0u8; 64];
            match serial.read(&mut buf) {
                Err(_e) => {
                    // Do nothing
                }
                Ok(0) => {
                    // Do nothing
                }
                Ok(count) => {
                    led_activity.set_high().unwrap();
                    // Convert to upper case
                    buf.iter_mut().take(count).for_each(|b| {
                        b.make_ascii_uppercase();
                        if *b == b'\t' {
                            next = true;
                        }
                    });
                    // Send back to the host
                    let mut wr_ptr = &buf[..count];
                    while !wr_ptr.is_empty() {
                        match serial.write(wr_ptr) {
                            Ok(len) => wr_ptr = &wr_ptr[len..],
                            // On error, just drop unwritten data.
                            // One possible error is Err(WouldBlock), meaning the USB
                            // write buffer is full.
                            Err(_) => break,
                        };
                    }
                    led_activity.set_low().unwrap();
                }
            }
        }

        if next {
            led_activity.set_high().unwrap();

            display.set_rotation(rotations.next().unwrap());
            display.clear(OctColor::White).unwrap();

            // We draw to a square section in the center of the display
            {
                let mut display = display.cropped(&display.bounding_box().resized(Size { width: 480, height: 480 }, AnchorPoint::Center));

                let w2 = 30;
                let w4 = 60;
                let w8 = 120;
                let w16 = 240;

                Triangle::new(
                    Point::new(w8, w8),
                    Point::new(w8 + w4, w4),
                    Point::new(w8 + w8, w8),
                )
                .into_styled(PrimitiveStyle::with_fill(colors.next().unwrap()))
                .draw(&mut display).unwrap();

                Triangle::new(
                    Point::new(w16, w8),
                    Point::new(w16 + w4, w4),
                    Point::new(w16 + w8, w8),
                )
                .into_styled(PrimitiveStyle::with_fill(colors.next().unwrap()))
                .draw(&mut display).unwrap();

                Rectangle::new(
                    Point::new(w8 + w2, w8),
                    Size::new(w4 as u32, (w8 + w4) as u32),
                )
                .into_styled(PrimitiveStyle::with_fill(colors.next().unwrap()))
                .draw(&mut display).unwrap();

                Rectangle::new(
                    Point::new(w16 + w2, w8),
                    Size::new(w4 as u32, (w8 + w4) as u32),
                )
                .into_styled(PrimitiveStyle::with_fill(colors.next().unwrap()))
                .draw(&mut display).unwrap();

                let text_style = MonoTextStyle::new(&FONT_9X18_BOLD, colors.next().unwrap());
                Text::new("UP", Point::new(w16 - 9, w16 + w4 + w2), text_style).draw(&mut display).unwrap();
            }

            epd.wake_up(&mut spi, &mut timer).unwrap();

            // Display updated frame
            epd.update_frame(&mut spi, &display.buffer(), &mut timer).unwrap();

            epd.display_frame(&mut spi, &mut timer).unwrap();

            epd.sleep(&mut spi, &mut timer).unwrap();

            led_activity.set_low().unwrap();
        }
    }
}
