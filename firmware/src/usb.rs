use core::fmt::Write;
use embedded_hal::{delay::DelayNs, digital::OutputPin};
use heapless::String;
use panic_halt as _;
use usb_device::{
    bus::UsbBusAllocator,
    device::{StringDescriptors, UsbDevice, UsbDeviceBuilder, UsbVidPid},
};
use usbd_serial::SerialPort;

use waveshare_rp2040_epaper_73::{
    hal::{usb::UsbBus, Timer},
    LedActivity,
};

pub struct Usb<'a> {
    said_hello: bool,
    serial: SerialPort<'a, UsbBus>,
    device: UsbDevice<'a, UsbBus>,
}

impl<'a> Usb<'a> {
    pub fn new(bus: &'a UsbBusAllocator<UsbBus>) -> Result<Self, crate::error::Infallible> {
        let serial = SerialPort::new(bus);

        let device = UsbDeviceBuilder::new(bus, UsbVidPid(0x16c0, 0x27dd))
            .strings(&[StringDescriptors::default()
                .manufacturer("Fake company")
                .product("Serial port")
                .serial_number("TEST")])
            .unwrap()
            .device_class(2) // from: https://www.usb.org/defined-class-codes
            .build();

        Ok(Self {
            said_hello: false,
            serial,
            device,
        })
    }

    pub fn poll(
        &mut self,
        timer: &mut Timer,
        activity: &mut LedActivity,
    ) -> Result<bool, crate::error::Infallible> {
        // A welcome message at the beginning
        if !self.said_hello && timer.get_counter().ticks() >= 2_000_000 {
            activity.set_high()?;
            timer.delay_ms(500);

            self.said_hello = true;
            let _ = self.serial.write(b"Hello, World!\n");

            timer.delay_ms(500);

            let time = timer.get_counter().ticks();
            let mut text: String<64> = String::new();
            let _ = writeln!(&mut text, "Current timer ticks: {}", time);

            // This only works reliably because the number of bytes written to
            // the serial port is smaller than the buffers available to the USB
            // peripheral. In general, the return value should be handled, so that
            // bytes not transferred yet don't get lost.
            let _ = self.serial.write(text.as_bytes());

            timer.delay_ms(500);
            activity.set_low()?;
        }

        let mut next = false;
        if self.device.poll(&mut [&mut self.serial]) {
            let mut buf = [0u8; 64];
            match self.serial.read(&mut buf) {
                Err(_e) => {
                    // Do nothing
                }
                Ok(0) => {
                    // Do nothing
                }
                Ok(count) => {
                    activity.set_high()?;
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
                        match self.serial.write(wr_ptr) {
                            Ok(len) => wr_ptr = &wr_ptr[len..],
                            // On error, just drop unwritten data.
                            // One possible error is Err(WouldBlock), meaning the USB
                            // write buffer is full.
                            Err(_) => break,
                        };
                    }
                    activity.set_low()?;
                }
            }
        }

        Ok(next)
    }
}
