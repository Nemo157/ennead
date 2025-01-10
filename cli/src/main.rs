extern crate ennead_protocol as ἐννεάς_protocol;

use std::{
    fs::OpenOptions,
    os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd},
    path::Path,
};

use anyhow::Context;
use dither::Dither as _;
use image::{imageops::FilterType, ImageReader};
use zerocopy::{IntoBytes, TryFromBytes};
use ἐννεάς_protocol::{image::PALETTE, Command, Response, HEIGHT, WIDTH};

fn find_device() -> anyhow::Result<OwnedFd> {
    let mut enumerator = udev::Enumerator::new()?;

    enumerator.match_subsystem("usb")?;
    enumerator.match_attribute("interface", "ἐννεάς-commands")?;

    for device in enumerator.scan_devices()? {
        let tty = device
            .syspath()
            .join("tty")
            .read_dir()?
            .next()
            .context("missing tty dir entry")??
            .file_name();
        let tty = Path::new("/dev").join(tty);

        eprintln!(
            "found device {}, command channel at {}",
            device.syspath().display(),
            tty.display()
        );

        return Ok(OpenOptions::new().read(true).write(true).open(tty)?.into());
    }

    anyhow::bail!("device not found")
}

fn send(device: BorrowedFd, command: &Command) -> anyhow::Result<()> {
    let bytes = command.as_bytes();
    let mut offset = 0;
    while offset < bytes.len() {
        offset += nix::unistd::write(&device, &bytes[offset..])?;
    }
    Ok(())
}

fn receive(device: RawFd) -> nix::Result<anyhow::Result<Response>> {
    let mut bytes = [0; 64];
    let mut offset = 0;
    while offset < bytes.len() {
        offset += nix::unistd::read(device.as_raw_fd(), &mut bytes[offset..])?;
    }
    Ok(Response::try_read_from_bytes(&bytes)
        .map_err(|e| anyhow::Error::from(e.map_src(|s| Vec::from(s)))))
}

fn main() -> anyhow::Result<()> {
    let device = find_device()?;

    let image = std::env::args()
        .skip(1)
        .next()
        .context("missing image filename")?;

    eprintln!("loading and preparing {image}");

    let image = ImageReader::open(image)?.with_guessed_format()?.decode()?;
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

    eprintln!("sending image with {} chunks", commands.len());

    let mut errors = 0;
    for command in commands {
        loop {
            send(device.as_fd(), &command)?;
            match receive(device.as_raw_fd())? {
                Ok(Response::Ok { .. }) => break,
                Ok(Response::Err { msg }) => {
                    eprintln!("error response {}", msg.to_str().unwrap());
                    errors += 1;
                }
                Err(err) => {
                    eprintln!("invalid response {err}");
                    errors += 1;
                }
            }
        }
    }

    eprintln!("completed sending image with {errors} errors");

    Ok(())
}
