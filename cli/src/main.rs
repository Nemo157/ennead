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
    }

    Ok(())
}
