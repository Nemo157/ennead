use core::fmt::Write;
use embedded_hal::{delay::DelayNs, digital::OutputPin};
use heapless::String;
use panic_halt as _;
use usb_device::{
    bus::UsbBusAllocator,
    device::{StringDescriptors, UsbDevice, UsbDeviceBuilder, UsbVidPid},
    UsbError,
};
use usbd_serial::{CdcAcmClass, SerialPort};

use waveshare_rp2040_epaper_73::{
    hal::{usb::UsbBus, Timer},
    LedActivity,
};

pub struct Usb<'a> {
    said_hello: bool,
    serial: SerialPort<'a, UsbBus>,
    commands: CommandPort<'a>,
    device: UsbDevice<'a, UsbBus>,
}

impl<'a> Usb<'a> {
    pub fn new(
        bus: &'a UsbBusAllocator<UsbBus>,
        serial_number: &'a str,
    ) -> Result<Self, crate::error::Infallible> {
        let serial = SerialPort::new_with_interface_names(bus, None, Some("ἐννεάς-log"));
        let commands = CommandPort::new(bus);

        let device = UsbDeviceBuilder::new(bus, UsbVidPid(0xf055, 0xcf82))
            .strings(&[StringDescriptors::default()
                .manufacturer("Nullus157")
                .product("ἐννεάς")
                .serial_number(serial_number)])
            .unwrap()
            .device_class(0) // generic device with multi-class interfaces ??
            .build();

        Ok(Self {
            said_hello: false,
            serial,
            commands,
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
        if self
            .device
            .poll(&mut [&mut self.serial, &mut self.commands.class])
        {
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

            match self.commands.read() {
                Err(err) => {
                    let mut text: String<64> = String::new();
                    let _ = writeln!(&mut text, "Received command error: {err:?}");
                    let _ = self.serial.write(text.as_bytes());
                }
                Ok(None) => {}
                Ok(Some(command)) => {
                    let mut text: String<64> = String::new();
                    let _ = writeln!(&mut text, "Received command: {}", command[0]);
                    let _ = self.serial.write(text.as_bytes());
                }
            }
        }

        Ok(next)
    }
}

struct CommandPort<'a> {
    class: CdcAcmClass<'a, UsbBus>,
}

impl<'a> CommandPort<'a> {
    pub fn new(bus: &'a UsbBusAllocator<UsbBus>) -> Self {
        Self {
            class: CdcAcmClass::new_with_interface_names(bus, 64, None, Some("ἐννεάς-commands")),
        }
    }

    pub fn read(&mut self) -> Result<Option<[u8; 63]>, UsbError> {
        let mut packet = [0; 63];
        match self.class.read_packet(&mut packet) {
            Ok(63) => Ok(Some(packet)),
            Err(UsbError::WouldBlock) => Ok(None),
            Ok(_) => Err(UsbError::ParseError),
            Err(err) => Err(err),
        }
    }
}
