use std::{fs::OpenOptions, os::fd::OwnedFd, path::Path};

use anyhow::Context;

fn main() -> anyhow::Result<()> {
    let mut enumerator = udev::Enumerator::new()?;

    enumerator.match_subsystem("usb")?;
    enumerator.match_attribute("interface", "ἐννεάς-commands")?;

    for device in enumerator.scan_devices()? {
        println!("{}", device.syspath().display());
        println!("  [properties]");
        for property in device.properties() {
            println!("    - {:?} {:?}", property.name(), property.value());
        }

        println!("  [attributes]");
        for attribute in device.attributes() {
            println!("    - {:?} {:?}", attribute.name(), attribute.value());
        }

        let tty = device.syspath().join("tty").read_dir()?.next().context("missing tty dir entry")??.file_name();
        dbg!(&tty);
        let fd: OwnedFd = OpenOptions::new().read(true).write(true).open(Path::new("/dev").join(tty))?.into();

        let mut command = [0; 63];
        command[0] = 0xf2;
        nix::unistd::write(fd, &command)?;
    }

    Ok(())
}
