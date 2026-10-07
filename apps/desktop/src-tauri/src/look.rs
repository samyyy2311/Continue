// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: GPL-3.0-only

//! Sends this computer's wallpaper and its average colour to connected phones.

use std::io::Cursor;
use std::path::PathBuf;
use std::time::Duration;

use device::Device;
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat};
use protocol::v1::DeviceLook;

/// The size the wallpaper is shrunk to fit; plenty for the phone's drawing of a laptop.
const WALLPAPER_WIDTH: u32 = 640;
const WALLPAPER_HEIGHT: u32 = 400;

/// How often the wallpaper is checked for a change.
const CHECK_EVERY: Duration = Duration::from_secs(60);

pub fn watch(device: Device) {
    tauri::async_runtime::spawn(async move {
        loop {
            if let Some(look) = tauri::async_runtime::spawn_blocking(own_look)
                .await
                .ok()
                .flatten()
            {
                device.sessions.report_look(look);
            }
            tokio::time::sleep(CHECK_EVERY).await;
        }
    });
}

fn own_look() -> Option<DeviceLook> {
    // Windows saves it without an extension, so the format comes from the bytes.
    let image = image::ImageReader::open(wallpaper_path()?)
        .ok()?
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?;
    let small = image.resize(WALLPAPER_WIDTH, WALLPAPER_HEIGHT, FilterType::Triangle);
    Some(DeviceLook {
        wallpaper_color: average_color(&small),
        wallpaper: jpeg(&small)?,
    })
}

/// Windows keeps a copy of the current wallpaper here, whatever it was set from. Other systems
/// aren't handled yet.
fn wallpaper_path() -> Option<PathBuf> {
    let path = PathBuf::from(std::env::var_os("APPDATA")?)
        .join("Microsoft")
        .join("Windows")
        .join("Themes")
        .join("TranscodedWallpaper");
    path.exists().then_some(path)
}

pub fn jpeg(image: &DynamicImage) -> Option<Vec<u8>> {
    let mut bytes = Cursor::new(Vec::new());
    image
        .to_rgb8()
        .write_to(&mut bytes, ImageFormat::Jpeg)
        .ok()?;
    Some(bytes.into_inner())
}

/// The mean of every pixel, as 0xRRGGBB.
fn average_color(image: &DynamicImage) -> u32 {
    let pixels = image.to_rgb8();
    let count = u64::from(pixels.width()) * u64::from(pixels.height());
    let [r, g, b] = pixels.pixels().fold([0u64; 3], |sum, p| {
        [
            sum[0] + u64::from(p[0]),
            sum[1] + u64::from(p[1]),
            sum[2] + u64::from(p[2]),
        ]
    });
    let channel = |total: u64| (total / count.max(1)) as u32;
    (channel(r) << 16) | (channel(g) << 8) | channel(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_average_colour_of_a_two_tone_image_is_halfway() {
        let mut image = image::RgbImage::new(2, 1);
        image.put_pixel(0, 0, image::Rgb([200, 0, 100]));
        image.put_pixel(1, 0, image::Rgb([0, 100, 100]));
        assert_eq!(average_color(&DynamicImage::ImageRgb8(image)), 0x643264);
    }
}
