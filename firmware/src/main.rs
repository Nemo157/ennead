#![no_std]
#![no_main]

use embedded_hal::digital::OutputPin;
use embedded_hal_bus::spi::ExclusiveDevice;
use panic_halt as _;
use usb_device::bus::UsbBusAllocator;

use fugit::RateExtU32;
use waveshare_rp2040_epaper_73::{
    hal::{
        clocks::init_clocks_and_plls, pac, timer::Timer, usb::UsbBus, watchdog::Watchdog, Clock,
        Sio, Spi,
    },
    EpdPowerEnable, LedActivity, LedPower, Pins, XOSC_CRYSTAL_FREQ,
};

mod display;
mod error;
mod usb;

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

    let usb_bus = UsbBusAllocator::new(UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let mut epd_power_enable: EpdPowerEnable = pins.epd_power_enable.reconfigure();

    epd_power_enable.set_high().unwrap();

    let mut usb = usb::Usb::new(&usb_bus).unwrap();

    let mut display = display::Display::new(
        ExclusiveDevice::new_no_delay(
            Spi::new(
                pac.SPI1,
                (
                    pins.epd_spi_tx.reconfigure(),
                    pins.epd_spi_clock.reconfigure(),
                ),
            )
            .init(
                &mut pac.RESETS,
                clocks.peripheral_clock.freq(),
                8.MHz(),
                embedded_hal::spi::MODE_0,
            ),
            pins.epd_spi_cs.reconfigure(),
        )
        .unwrap(),
        pins.epd_busy.reconfigure(),
        pins.epd_dc.reconfigure(),
        pins.epd_reset.reconfigure(),
        &mut timer,
    )
    .unwrap();

    led_power.set_high().unwrap();

    loop {
        let next = usb.poll(&mut timer, &mut led_activity).unwrap();

        if next {
            display.next(&mut timer, &mut led_activity).unwrap();
        }
    }
}
