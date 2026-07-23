//! PNG/GIF encoding for the "export the running game" File-menu actions.
//! Frames are already-composed RGBA buffers (world+UI layers, same as the
//! game preview texture) — no headless VM replay involved, this module only
//! encodes bytes the caller already produced.

use anyhow::{Context, Result};
use image::codecs::gif::GifEncoder;
use image::{Delay, Frame, ImageBuffer, ImageFormat, Rgba};
use std::time::Duration;

fn to_image(width: u32, height: u32, rgba: Vec<u8>) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    ImageBuffer::from_raw(width, height, rgba).context("composed frame size mismatch")
}

pub fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>> {
    let img = to_image(width, height, rgba.to_vec())?;
    let mut bytes = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut bytes), ImageFormat::Png)
        .context("failed to encode PNG")?;
    Ok(bytes)
}

pub fn encode_gif(width: u32, height: u32, frames: &[Vec<u8>], delay_ms: u64) -> Result<Vec<u8>> {
    let delay = Delay::from_saturating_duration(Duration::from_millis(delay_ms));
    let mut bytes = Vec::new();
    {
        let mut encoder = GifEncoder::new(&mut bytes);
        for rgba in frames {
            let img = to_image(width, height, rgba.clone())?;
            encoder
                .encode_frame(Frame::from_parts(img, 0, 0, delay))
                .context("failed to encode GIF frame")?;
        }
    }
    Ok(bytes)
}
