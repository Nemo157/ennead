extern crate ennead_protocol as ἐννεάς_protocol;

use anyhow::Context;
use clap::{Parser, ValueEnum};
use dither::Dither as _;
use image::{ImageReader, imageops::FilterType};
use indicatif::{ProgressBar, ProgressStyle};
use nusb::DeviceInfo;
use zerocopy::IntoBytes;
use ἐννεάς_protocol::{Command, HEIGHT, WIDTH, image::PALETTE};

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

fn dither_dither(
    image: image::RgbImage,
    ditherer: dither::ditherer::Ditherer<'static>,
) -> image::RgbImage {
    let img = dither::Img::new(
        image
            .pixels()
            .map(|&image::Rgb([r, g, b])| dither::color::RGB(r as f64, g as f64, b as f64)),
        image.width(),
    )
    .unwrap();

    let palette = PALETTE.map(|image::Rgb([r, g, b])| dither::color::RGB(r, g, b));

    let img = ditherer.dither(img, dither::color::palette::quantize(&palette));

    image::RgbImage::from_vec(
        image.width(),
        image.height(),
        img.iter()
            .flat_map(|&dither::color::RGB(r, g, b)| [r as u8, g as u8, b as u8])
            .collect(),
    )
    .unwrap()
}

fn bayer(image: image::RgbImage) -> image::RgbImage {
    use image_effects::effect::Effect;

    let palette = PALETTE
        .iter()
        .map(|&image::Rgb([r, g, b])| {
            palette::rgb::Srgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
        })
        .collect();

    let img = image
        .rows()
        .map(|row| row.map(|&image::Rgb([r, g, b])| [r, g, b]).collect())
        .collect();

    let img: Vec<Vec<[u8; 3]>> = image_effects::dither::bayer::Bayer::new(4, palette).affect(img);

    image::RgbImage::from_vec(
        image.width(),
        image.height(),
        img.into_iter().flatten().flatten().collect(),
    )
    .unwrap()
}

fn blue_noise(mut image: image::RgbImage) -> image::RgbImage {
    fn dist(x: &image::Rgb<u8>, y: &image::Rgb<u8>) -> u32 {
        (x.0[0].abs_diff(y.0[0]) as u32).pow(2)
            + (x.0[1].abs_diff(y.0[1]) as u32).pow(2)
            + (x.0[2].abs_diff(y.0[2]) as u32).pow(2)
    }

    static NOISE: &[u8] = include_bytes!("./blue-noise.png");
    let noise = ImageReader::with_format(std::io::Cursor::new(NOISE), image::ImageFormat::Png)
        .decode()
        .unwrap()
        .to_rgb8();

    for (pixel, noise) in image
        .rows_mut()
        .zip(noise.rows())
        .flat_map(|(row, noise)| row.zip(noise))
    {
        let dith = image::Rgb(std::array::from_fn(|c| {
            let offset = pixel[c] as i16 - noise[c] as i16 + 128 as i16;
            offset.clamp(0, 255) as u8
        }));
        *pixel = *PALETTE.iter().min_by_key(|p| dist(&dith, p)).unwrap();
    }

    image
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Dither {
    Atkinson,
    Burkes,
    FloydSteinberg,
    JarvisJudiceNinke,
    Sierra3,
    Stucki,
    Bayer,
    BlueNoise,
}

impl Dither {
    fn apply(self, image: image::RgbImage) -> image::RgbImage {
        match self {
            Self::Atkinson => dither_dither(image, dither::ditherer::ATKINSON),
            Self::Burkes => dither_dither(image, dither::ditherer::BURKES),
            Self::FloydSteinberg => dither_dither(image, dither::ditherer::FLOYD_STEINBERG),
            Self::JarvisJudiceNinke => dither_dither(image, dither::ditherer::JARVIS_JUDICE_NINKE),
            Self::Sierra3 => dither_dither(image, dither::ditherer::SIERRA_3),
            Self::Stucki => dither_dither(image, dither::ditherer::STUCKI),
            Self::Bayer => bayer(image),
            Self::BlueNoise => blue_noise(image),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Scale {
    Fit,
    Fill,
    Stretch,
}

#[derive(Parser)]
struct Args {
    /// Image to send to the display
    image: String,

    /// Dither algorithm to apply to image
    #[arg(long, value_enum)]
    dither: Dither,

    /// Scaling to apply to fit image to frame
    #[arg(long, value_enum)]
    scale: Scale,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

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
        .with_message(args.image.clone());

    let image = ImageReader::open(&args.image)?
        .with_guessed_format()?
        .decode()?;
    image.save("/tmp/ἐννεάς.original.png").unwrap();

    let image = match args.scale {
        Scale::Fit => image.resize(WIDTH, HEIGHT, FilterType::CatmullRom),
        Scale::Fill => image.resize_to_fill(WIDTH, HEIGHT, FilterType::CatmullRom),
        Scale::Stretch => image.resize_exact(WIDTH, HEIGHT, FilterType::CatmullRom),
    }
    .to_rgb8();
    image.save("/tmp/ἐννεάς.resized.png").unwrap();

    let image = args.dither.apply(image);
    image.save("/tmp/ἐννεάς.dithered.png").unwrap();

    let image = match args.scale {
        Scale::Fit => {
            let mut base = image::RgbImage::from_pixel(WIDTH, HEIGHT, image::Rgb([255, 255, 255]));
            image::imageops::overlay(
                &mut base,
                &image,
                i64::from((WIDTH - image.width()) / 2),
                i64::from((HEIGHT - image.height()) / 2),
            );
            base.save("/tmp/ἐννεάς.padded.png").unwrap();
            base
        }
        Scale::Fill | Scale::Stretch => image,
    };

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
