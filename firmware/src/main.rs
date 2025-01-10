#![no_std]
#![no_main]

extern crate ennead_protocol as ἐννεάς_protocol;

use embedded_hal::digital::OutputPin;
use embedded_hal_bus::spi::ExclusiveDevice;
use heapless::String;
use panic_halt as _;
use usb_device::bus::UsbBusAllocator;
use ἐννεάς_protocol::{Command, Response};

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

fn read_serial() -> u32 {
    // TODO: The RP2040 doesn't have a unique id, the sdk reads the id from the flash chip, I don't
    // know if this configuration has a flash chip or how to read it though 😔.
    return 0xeeeeeeee;
}

fn aegean_u16(mut target: u16, result: &mut String<64>) {
    const NUMERALS: [[&str; 9]; 5] = [
        ["𐄇", "𐄈", "𐄉", "𐄊", "𐄋", "𐄌", "𐄍", "𐄎", "𐄏"],
        ["𐄐", "𐄑", "𐄒", "𐄓", "𐄔", "𐄕", "𐄖", "𐄗", "𐄘"],
        ["𐄙", "𐄚", "𐄛", "𐄜", "𐄝", "𐄞", "𐄟", "𐄠", "𐄡"],
        ["𐄢", "𐄣", "𐄤", "𐄥", "𐄦", "𐄧", "𐄨", "𐄩", "𐄪"],
        ["𐄫", "𐄬", "𐄭", "𐄮", "𐄯", "𐄰", "𐄱", "𐄲", "𐄳"],
    ];

    let mut numerals = NUMERALS
        .iter()
        .zip([1, 10, 100, 1000, 10000])
        .rev()
        .flat_map(|(inner, a)| {
            inner
                .iter()
                .zip(1..9)
                .rev()
                .map(move |(numeral, b)| (a * b, numeral))
        });

    while target > 0 {
        let (value, numeral) = numerals.next().unwrap();
        while target as u32 >= value {
            let _ = result.push_str(numeral);
            target -= value as u16;
        }
    }
}

fn aegean_u32(value: u32) -> String<64> {
    let mut result: String<64> = String::new();

    aegean_u16((value >> 16) as u16, &mut result);
    let _ = result.push_str(" ");
    aegean_u16(value as u16, &mut result);

    result
}

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

    let serial_number = aegean_u32(read_serial());
    let mut usb = usb::Usb::new(&usb_bus, &serial_number).unwrap();

    let mut epd_power_enable: EpdPowerEnable = pins.epd_power_enable.reconfigure();
    epd_power_enable.set_high().unwrap();

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
        let Some(command) = usb.poll(&mut timer, &mut led_activity).unwrap() else {
            continue;
        };

        match command {
            Ok(command) => {
                match command {
                    Command::Start { .. } => display.clear(),
                    Command::Chunk(chunk) => display.update(chunk),
                    Command::End { .. } => display.show(&mut timer, &mut led_activity).unwrap(),
                }
                // usb.send_response(Response::Ok { _unused: [0; 62] });
            }
            Err(msg) => {
                // usb.send_response(Response::Err { msg });
            }
        }
    }
}
