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
use ἐννεάς_protocol::Command;
use zerocopy::TryFromBytes;

use waveshare_rp2040_epaper_73::{
    hal::{usb::UsbBus, Timer},
    LedActivity,
};

pub struct Usb<'a> {
    said_hello: bool,
    serial: SerialPort<'a, UsbBus>,
    commands: CommandPort<'a>,
    device: UsbDevice<'a, UsbBus>,
    received_chunks: usize,
}

impl<'a> Usb<'a> {
    pub fn new(
        bus: &'a UsbBusAllocator<UsbBus>,
        serial_number: &'a str,
    ) -> Result<Self, crate::error::Infallible> {
        let serial = SerialPort::new_with_interface_names(bus, Some("ἐννεάς-log"), None);
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
            received_chunks: 0,
        })
    }

    pub fn poll(
        &mut self,
        timer: &mut Timer,
        activity: &mut LedActivity,
    ) -> Result<Option<Command>, crate::error::Infallible> {
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

            if let Ok(Some(command)) = self.commands.read() {
                if let Ok(command) = Command::try_read_from_bytes(&command) {
                    match command {
                        Command::Start { .. } => {
                            let mut text: String<64> = String::new();
                            let _ = writeln!(&mut text, "Start");
                            let _ = self.serial.write(text.as_bytes());
                            self.received_chunks = 0;
                        }
                        Command::Chunk { .. } => {
                            self.received_chunks += 1;
                        }
                        Command::End { .. } => {
                            let mut text: String<64> = String::new();
                            let _ = writeln!(&mut text, "End, received {} chunks", self.received_chunks);
                            let _ = self.serial.write(text.as_bytes());
                        }
                    }
                    return Ok(Some(command));
                }
            }
        }

        Ok(None)
    }
}

struct CommandPort<'a> {
    class: CdcAcmClass<'a, UsbBus>,
}

impl<'a> CommandPort<'a> {
    pub fn new(bus: &'a UsbBusAllocator<UsbBus>) -> Self {
        Self {
            class: CdcAcmClass::new_with_interface_names(bus, 64, Some("ἐννεάς-commands"), None),
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
