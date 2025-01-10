extern crate ennead_protocol as ἐννεάς_protocol;

use anyhow::Context;
use dither::Dither as _;
use image::{imageops::FilterType, ImageReader};
use indicatif::{ProgressBar, ProgressStyle};
use nusb::DeviceInfo;
use zerocopy::IntoBytes;
use ἐννεάς_protocol::{image::PALETTE, Command, HEIGHT, WIDTH};

fn find_device() -> anyhow::Result<(DeviceInfo, u8)> {
    let mut interface_number = None;
    for device in nusb::list_devices()? {
        for interface in device.interfaces() {
            if interface.interface_string() == Some("ἐννεάς-commands") {
                interface_number = Some(interface.interface_number());
            }
        }
        if let Some(interface_number) = interface_number {
            return Ok((device, interface_number));
        }
    }

    anyhow::bail!("device not found")
}

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .skip(1)
        .next()
        .context("missing image filename")?;

    let spinner = ProgressStyle::with_template("{prefix:>40.cyan} {spinner} {msg}")?;
    let success = ProgressStyle::with_template("{prefix:>40.green} {spinner} {msg}")?;
    let bar_style = ProgressStyle::with_template(
        "{prefix:>40.cyan} {spinner} [{bar:27}] {pos:>9}/{len:9}  {per_sec} {elapsed:>4}/{eta:4}",
    )?;

    let bar = ProgressBar::no_length()
        .with_style(spinner.clone())
        .with_prefix("finding ἐννεάς device");
    let (device, interface_number) = find_device()?;
    let interface = device
        .open()
        .context("opening usb device")?
        .detach_and_claim_interface(interface_number)
        .context("claiming usb interface")?;
    bar.with_style(success.clone())
        .with_prefix("found device")
        .finish_with_message(format!(
            "{}/{} {}",
            device.manufacturer_string().unwrap_or("<unknown>"),
            device.product_string().unwrap_or("<unknown>"),
            device.serial_number().unwrap_or("<unknown>")
        ));

    let bar = ProgressBar::no_length()
        .with_style(spinner.clone())
        .with_prefix("loading image")
        .with_message(image.clone());

    let image = ImageReader::open(&image)?.with_guessed_format()?.decode()?;
    image.save("/tmp/ἐννεάς.original.png").unwrap();

    let image = image.resize(WIDTH, HEIGHT, FilterType::CatmullRom);
    let mut base = image::RgbaImage::from_pixel(WIDTH, HEIGHT, image::Rgba([255, 255, 255, 255]));
    image::imageops::overlay(
        &mut base,
        &image,
        i64::from((WIDTH - image.width()) / 2),
        i64::from((HEIGHT - image.height()) / 2),
    );
    let image = base;
    image.save("/tmp/ἐννεάς.resized.png").unwrap();

    let img = dither::Img::new(
        image
            .pixels()
            .map(|&image::Rgba([r, g, b, _])| dither::color::RGB(r as f64, g as f64, b as f64)),
        image.width(),
    )
    .unwrap();

    let palette = PALETTE.map(|image::Rgb([r, g, b])| dither::color::RGB(r, g, b));

    let img = dither::ditherer::BURKES.dither(img, dither::color::palette::quantize(&palette));

    let image = image::RgbImage::from_vec(
        WIDTH,
        HEIGHT,
        img.iter()
            .flat_map(|&dither::color::RGB(r, g, b)| [r as u8, g as u8, b as u8])
            .collect(),
    )
    .unwrap();
    image.save("/tmp/ἐννεάς.dithered.png").unwrap();

    let image = {
        let mut image = image;
        image::imageops::rotate180_in_place(&mut image);
        image
    };
    let commands = Command::from_image(&image);

    bar.with_style(success.clone())
        .with_prefix("loaded image")
        .finish();

    let bar = ProgressBar::new(u64::try_from(commands.len())?)
        .with_style(bar_style.clone())
        .with_prefix("sending commands");

    let mut output = interface.bulk_out_queue(0x02);
    for command in &commands {
        output.submit(Vec::from(command.as_bytes()));
    }

    while output.pending() > 0 {
        futures::executor::block_on(output.next_complete()).into_result()?;
        bar.inc(1);
    }

    bar.with_style(success.clone())
        .with_prefix("sent commands")
        .finish_with_message("image should be refreshing now");

    Ok(())
}
