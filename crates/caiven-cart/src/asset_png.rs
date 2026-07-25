//! PNG codec for the sprite sheet, palette, and map assets — the visual
//! counterpart to `text.rs`'s hex encoding, so a project's images can be
//! opened and edited in any image tool (Aseprite, GIMP, a file browser
//! preview) instead of only inside Caiven Studio.
//!
//! Sprite sheet: 256 sprites arranged as a 16×16 grid of 8×8 tiles ->
//! 128×128 8-bit indexed PNG (PLTE = the cart's 16-color palette). Palette:
//! 16×1 8-bit RGB PNG (pixel `i` = palette color `i`). Map: 64×64 8-bit
//! grayscale PNG (pixel value = the tile/sprite id stored at that cell).

use caiven_core::memory::{MAP_H, MAP_LEN, MAP_W, PALETTE_SIZE, SPRITE_BYTES, SPRITE_SIZE};

const SHEET_COLS: u32 = 16;
const SHEET_W: u32 = SHEET_COLS * SPRITE_SIZE;
const SHEET_H: u32 = SHEET_COLS * SPRITE_SIZE;
const SHEET_LEN: usize = (SHEET_W * SHEET_H) as usize;

/// Encodes the sprite sheet RAM section (sprite-major: `sheet[id*64 + sy*8 +
/// sx]`) as a 128×128 indexed PNG using `palette` (16×RGB bytes) as PLTE.
pub fn sprites_to_png(sheet: &[u8], palette: &[u8]) -> Result<Vec<u8>, String> {
    let mut indices = vec![0u8; SHEET_LEN];
    for py in 0..SHEET_H {
        for px in 0..SHEET_W {
            let tile_x = px / SPRITE_SIZE;
            let tile_y = py / SPRITE_SIZE;
            let id = (tile_y * SHEET_COLS + tile_x) as usize;
            let local = ((py % SPRITE_SIZE) * SPRITE_SIZE + (px % SPRITE_SIZE)) as usize;
            let byte = sheet.get(id * SPRITE_BYTES + local).copied().unwrap_or(0);
            indices[(py * SHEET_W + px) as usize] = byte;
        }
    }
    encode_indexed(SHEET_W, SHEET_H, &indices, palette)
}

/// Decodes a sprite sheet PNG back into the sprite-major RAM byte order.
/// Accepts an indexed PNG (uses its pixel indices directly) or an RGB/RGBA
/// PNG (maps each pixel to its nearest `palette` entry) — an external editor
/// may flatten the indexed PLTE away on save.
pub fn png_to_sprites(bytes: &[u8], palette: &[u8]) -> Result<Vec<u8>, String> {
    let (w, h, indices) = decode_to_indices(bytes, palette)?;
    if w != SHEET_W || h != SHEET_H {
        return Err(format!(
            "sprite sheet PNG must be {SHEET_W}x{SHEET_H}, got {w}x{h}"
        ));
    }
    let mut out = vec![0u8; SHEET_LEN];
    for py in 0..SHEET_H {
        for px in 0..SHEET_W {
            let tile_x = px / SPRITE_SIZE;
            let tile_y = py / SPRITE_SIZE;
            let id = (tile_y * SHEET_COLS + tile_x) as usize;
            let local = ((py % SPRITE_SIZE) * SPRITE_SIZE + (px % SPRITE_SIZE)) as usize;
            out[id * SPRITE_BYTES + local] = indices[(py * SHEET_W + px) as usize];
        }
    }
    Ok(out)
}

/// Encodes a 16-slot RGB palette section as a 16×1 RGB PNG.
pub fn palette_to_png(palette: &[u8]) -> Result<Vec<u8>, String> {
    let mut rgb = vec![0u8; PALETTE_SIZE * 3];
    let n = rgb.len().min(palette.len());
    rgb[..n].copy_from_slice(&palette[..n]);
    encode_rgb(PALETTE_SIZE as u32, 1, &rgb)
}

/// Decodes a 16×1 (or 4×4, or any 16-pixel layout) RGB/RGBA PNG back into a
/// flat 16×RGB palette section, reading pixels in row-major order.
pub fn png_to_palette(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let (w, h, rgb) = decode_to_rgb(bytes)?;
    if (w * h) as usize != PALETTE_SIZE {
        return Err(format!(
            "palette PNG must have exactly {PALETTE_SIZE} pixels, got {w}x{h}"
        ));
    }
    Ok(rgb)
}

/// Encodes the map RAM section (row-major, `map[y*64 + x]`, byte = tile id)
/// as a 64×64 grayscale PNG.
pub fn map_to_png(map: &[u8]) -> Result<Vec<u8>, String> {
    let mut gray = vec![0u8; MAP_LEN];
    let n = gray.len().min(map.len());
    gray[..n].copy_from_slice(&map[..n]);
    encode_gray(MAP_W as u32, MAP_H as u32, &gray)
}

/// Decodes a 64×64 grayscale (or indexed/RGB, using the red channel) PNG
/// back into the row-major map byte order.
pub fn png_to_map(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let (w, h, gray) = decode_to_gray(bytes)?;
    if w as usize != MAP_W || h as usize != MAP_H {
        return Err(format!("map PNG must be {MAP_W}x{MAP_H}, got {w}x{h}"));
    }
    Ok(gray)
}

fn encode_indexed(w: u32, h: u32, indices: &[u8], palette: &[u8]) -> Result<Vec<u8>, String> {
    let mut plte = vec![0u8; PALETTE_SIZE * 3];
    let n = plte.len().min(palette.len());
    plte[..n].copy_from_slice(&palette[..n]);

    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(png::ColorType::Indexed);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_palette(plte);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer
        .write_image_data(indices)
        .map_err(|e| e.to_string())?;
    drop(writer);
    Ok(out)
}

fn encode_rgb(w: u32, h: u32, rgb: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(rgb).map_err(|e| e.to_string())?;
    drop(writer);
    Ok(out)
}

fn encode_gray(w: u32, h: u32, gray: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(gray).map_err(|e| e.to_string())?;
    drop(writer);
    Ok(out)
}

struct Decoded {
    width: u32,
    height: u32,
    color: png::ColorType,
    bit_depth: png::BitDepth,
    palette: Option<Vec<u8>>,
    /// Raw pixel samples in the PNG's native bit depth.
    data: Vec<u8>,
}

fn decode(bytes: &[u8]) -> Result<Decoded, String> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    // Keep indexed images indexed so `decode_to_indices` reads their exact
    // palette indices rather than re-matching expanded RGB values.
    decoder.set_transformations(png::Transformations::IDENTITY);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let buf_size = reader
        .output_buffer_size()
        .ok_or_else(|| "PNG has no decodable frame".to_string())?;
    let mut buf = vec![0u8; buf_size];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let palette = reader.info().palette.as_ref().map(|p| p.to_vec());
    Ok(Decoded {
        width: info.width,
        height: info.height,
        color: info.color_type,
        bit_depth: info.bit_depth,
        palette,
        data: buf[..info.buffer_size()].to_vec(),
    })
}

impl Decoded {
    /// Returns one byte per sample. Sub-byte grayscale/indexed samples are
    /// unpacked from each scanline, while 16-bit samples use their high byte
    /// (the PNG crate's `STRIP_16` behavior).
    fn samples(&self) -> Result<Vec<u8>, String> {
        match self.bit_depth {
            png::BitDepth::Eight => Ok(self.data.clone()),
            png::BitDepth::Sixteen => {
                if self.data.len() % 2 != 0 {
                    return Err("PNG has an incomplete 16-bit sample".to_string());
                }
                Ok(self.data.chunks_exact(2).map(|sample| sample[0]).collect())
            }
            png::BitDepth::One | png::BitDepth::Two | png::BitDepth::Four => {
                if !matches!(
                    self.color,
                    png::ColorType::Indexed | png::ColorType::Grayscale
                ) {
                    return Err(format!(
                        "unsupported {:?}-bit {:?} PNG",
                        self.bit_depth, self.color
                    ));
                }

                let bits = self.bit_depth as usize;
                let width = self.width as usize;
                let height = self.height as usize;
                let row_bytes = width
                    .checked_mul(bits)
                    .and_then(|bits| bits.checked_add(7))
                    .map(|bits| bits / 8)
                    .ok_or_else(|| "PNG dimensions are too large".to_string())?;
                let expected_len = row_bytes
                    .checked_mul(height)
                    .ok_or_else(|| "PNG dimensions are too large".to_string())?;
                if self.data.len() != expected_len {
                    return Err("PNG packed sample buffer has an unexpected length".to_string());
                }

                let samples_per_byte = 8 / bits;
                let mask = (1u8 << bits) - 1;
                let mut samples = Vec::with_capacity(width * height);
                for row in self.data.chunks_exact(row_bytes) {
                    for x in 0..width {
                        let shift = 8 - bits * (x % samples_per_byte + 1);
                        samples.push((row[x / samples_per_byte] >> shift) & mask);
                    }
                }
                Ok(samples)
            }
        }
    }

    /// Grayscale is a shade in RGB contexts, so expand a packed grayscale
    /// sample across its full 8-bit range. Index/map contexts keep the raw
    /// sample value instead.
    fn rgb_samples(&self) -> Result<Vec<u8>, String> {
        let samples = self.samples()?;
        let scale = match self.bit_depth {
            png::BitDepth::One => Some(255),
            png::BitDepth::Two => Some(85),
            png::BitDepth::Four => Some(17),
            _ => None,
        };
        Ok(match scale {
            Some(scale) if self.color == png::ColorType::Grayscale => {
                samples.into_iter().map(|sample| sample * scale).collect()
            }
            _ => samples,
        })
    }
}

/// Decodes to a flat index-per-pixel buffer. Indexed PNGs return their
/// stored indices verbatim; true-color PNGs are matched to the nearest
/// `fallback_palette` entry per pixel.
fn decode_to_indices(bytes: &[u8], fallback_palette: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let d = decode(bytes)?;
    let samples = d.samples()?;
    match d.color {
        png::ColorType::Indexed => Ok((d.width, d.height, samples)),
        png::ColorType::Rgb => {
            let indices = samples
                .chunks_exact(3)
                .map(|p| nearest_index(p[0], p[1], p[2], fallback_palette))
                .collect();
            Ok((d.width, d.height, indices))
        }
        png::ColorType::Rgba => {
            let indices = samples
                .chunks_exact(4)
                .map(|p| nearest_index(p[0], p[1], p[2], fallback_palette))
                .collect();
            Ok((d.width, d.height, indices))
        }
        png::ColorType::Grayscale => Ok((d.width, d.height, samples)),
        png::ColorType::GrayscaleAlpha => {
            let indices = samples.chunks_exact(2).map(|p| p[0]).collect();
            Ok((d.width, d.height, indices))
        }
    }
}

fn nearest_index(r: u8, g: u8, b: u8, palette: &[u8]) -> u8 {
    let mut best = 0u8;
    let mut best_dist = u32::MAX;
    for (i, chunk) in palette.chunks_exact(3).enumerate() {
        let dr = r as i32 - chunk[0] as i32;
        let dg = g as i32 - chunk[1] as i32;
        let db = b as i32 - chunk[2] as i32;
        let dist = (dr * dr + dg * dg + db * db) as u32;
        if dist < best_dist {
            best_dist = dist;
            best = i as u8;
        }
    }
    best
}

/// Decodes to a flat RGB-triples buffer, expanding indexed/gray input
/// through their palette/shade.
fn decode_to_rgb(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let d = decode(bytes)?;
    let samples = d.rgb_samples()?;
    let rgb = match d.color {
        png::ColorType::Rgb => samples,
        png::ColorType::Rgba => samples
            .chunks_exact(4)
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect(),
        png::ColorType::Indexed => {
            let palette = d.palette.unwrap_or_default();
            samples
                .iter()
                .flat_map(|&i| {
                    let base = i as usize * 3;
                    palette
                        .get(base..base + 3)
                        .map(|c| [c[0], c[1], c[2]])
                        .unwrap_or([0, 0, 0])
                })
                .collect()
        }
        png::ColorType::Grayscale => samples.iter().flat_map(|&v| [v, v, v]).collect(),
        png::ColorType::GrayscaleAlpha => samples
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0]])
            .collect(),
    };
    Ok((d.width, d.height, rgb))
}

/// Decodes to a flat one-byte-per-pixel buffer using the red/gray channel
/// (map tile ids are stored as grayscale, but tolerate other color types).
fn decode_to_gray(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let d = decode(bytes)?;
    let samples = d.samples()?;
    let gray = match d.color {
        png::ColorType::Grayscale => samples,
        png::ColorType::GrayscaleAlpha => samples.chunks_exact(2).map(|p| p[0]).collect(),
        png::ColorType::Rgb => samples.chunks_exact(3).map(|p| p[0]).collect(),
        png::ColorType::Rgba => samples.chunks_exact(4).map(|p| p[0]).collect(),
        png::ColorType::Indexed => samples,
    };
    Ok((d.width, d.height, gray))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn test_palette() -> Vec<u8> {
        (0..PALETTE_SIZE)
            .flat_map(|i| [i as u8 * 10, i as u8 * 5, i as u8 * 3])
            .collect()
    }

    #[test]
    fn sprite_sheet_roundtrips_through_png() {
        let palette = test_palette();
        let mut sheet = vec![0u8; SHEET_LEN];
        // Sprite 0's top-left pixel and sprite 5's bottom-right pixel.
        sheet[0] = 7;
        sheet[5 * SPRITE_BYTES + (SPRITE_BYTES - 1)] = 3;

        let png = sprites_to_png(&sheet, &palette).unwrap();
        let back = png_to_sprites(&png, &palette).unwrap();
        assert_eq!(back, sheet);
    }

    #[test]
    fn palette_roundtrips_through_png() {
        let palette = test_palette();
        let png = palette_to_png(&palette).unwrap();
        let back = png_to_palette(&png).unwrap();
        assert_eq!(back, palette);
    }

    #[test]
    fn map_roundtrips_through_png() {
        let mut map = vec![0u8; MAP_LEN];
        map[0] = 1;
        map[MAP_LEN - 1] = 255;
        map[100] = 42;

        let png = map_to_png(&map).unwrap();
        let back = png_to_map(&png).unwrap();
        assert_eq!(back, map);
    }

    #[test]
    fn wrong_sprite_sheet_dimensions_are_rejected() {
        let png = encode_gray(8, 8, &[0u8; 64]).unwrap();
        assert!(png_to_sprites(&png, &test_palette()).is_err());
    }

    #[test]
    fn packed_four_bit_indexed_sprite_sheet_decodes_without_panicking() {
        let palette = test_palette();
        let mut pixels = Vec::with_capacity(SHEET_LEN);
        let mut expected = vec![0u8; SHEET_LEN];
        for py in 0..SHEET_H {
            for px in 0..SHEET_W {
                let tile_x = px / SPRITE_SIZE;
                let tile_y = py / SPRITE_SIZE;
                let id = (tile_y * SHEET_COLS + tile_x) as usize;
                let local = ((py % SPRITE_SIZE) * SPRITE_SIZE + (px % SPRITE_SIZE)) as usize;
                let index = ((px + py) % PALETTE_SIZE as u32) as u8;
                pixels.push(index);
                expected[id * SPRITE_BYTES + local] = index;
            }
        }

        let mut packed = Vec::with_capacity(SHEET_LEN / 2);
        for pair in pixels.chunks_exact(2) {
            packed.push(pair[0] << 4 | pair[1]);
        }
        let mut png = Vec::new();
        let mut encoder = png::Encoder::new(&mut png, SHEET_W, SHEET_H);
        encoder.set_color(png::ColorType::Indexed);
        encoder.set_depth(png::BitDepth::Four);
        encoder.set_palette(palette.clone());
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&packed).unwrap();
        drop(writer);

        assert_eq!(png_to_sprites(&png, &palette).unwrap(), expected);
    }

    #[test]
    fn sixteen_bit_grayscale_map_uses_high_bytes() {
        let expected: Vec<u8> = (0..MAP_LEN).map(|index| index as u8).collect();
        let mut samples = Vec::with_capacity(MAP_LEN * 2);
        for sample in &expected {
            samples.extend_from_slice(&[*sample, 0]);
        }

        let mut png = Vec::new();
        let mut encoder = png::Encoder::new(&mut png, MAP_W as u32, MAP_H as u32);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Sixteen);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&samples).unwrap();
        drop(writer);

        assert_eq!(png_to_map(&png).unwrap(), expected);
    }
}
