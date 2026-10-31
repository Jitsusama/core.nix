//! A screenshot as the models take it: red, green and blue bytes a pixel,
//! row after row, whatever colours and depth the PNG was saved in.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use png::{ColorType, Decoder, Transformations};

/// A count of pixels as a real number, to scale by.
#[expect(
    clippy::cast_precision_loss,
    reason = "a picture's sides are thousands of pixels, far inside f32's 2^24"
)]
pub fn real(pixels: usize) -> f32 {
    pixels as f32
}

/// A real number of pixels, at least none, as a count.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "what's given is a scaled side, a few thousand pixels, and none below none"
)]
pub fn whole(pixels: f32) -> usize {
    pixels.max(0.0) as usize
}

/// A side as a model's shape gives it.
pub fn dimension(side: usize) -> i64 {
    i64::try_from(side).unwrap_or(i64::MAX)
}

pub struct Picture {
    pub width: usize,
    pub height: usize,
    rgb: Vec<u8>,
}

impl Picture {
    /// The PNG at `path`.
    pub fn read(path: &Path) -> Result<Self, String> {
        let file = File::open(path).map_err(|why| format!("opening {}: {why}", path.display()))?;
        let mut decoder = Decoder::new(BufReader::new(file));
        // Palettes and fewer than eight bits widened, sixteen narrowed, so
        // every picture comes out as bytes of grey or colour.
        decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);
        let unreadable = |why| format!("reading {}: {why}", path.display());
        let mut reader = decoder.read_info().map_err(unreadable)?;
        let size = reader
            .output_buffer_size()
            .ok_or_else(|| format!("{} is too big to read", path.display()))?;
        let mut bytes = vec![0; size];
        let frame = reader.next_frame(&mut bytes).map_err(unreadable)?;
        bytes.truncate(frame.buffer_size());
        let channels = match frame.color_type {
            ColorType::Grayscale => 1,
            ColorType::GrayscaleAlpha => 2,
            ColorType::Rgb => 3,
            ColorType::Rgba => 4,
            ColorType::Indexed => return Err(format!("{} kept its palette", path.display())),
        };
        let rgb = bytes
            .chunks_exact(channels)
            .flat_map(|pixel| match channels {
                1 | 2 => [pixel[0]; 3],
                _ => [pixel[0], pixel[1], pixel[2]],
            })
            .collect();
        Ok(Self::from_rgb(
            frame.width as usize,
            frame.height as usize,
            rgb,
        ))
    }

    pub fn from_rgb(width: usize, height: usize, rgb: Vec<u8>) -> Self {
        Self { width, height, rgb }
    }

    /// The pixel at a column and row, as red, green and blue.
    pub fn at(&self, x: usize, y: usize) -> [u8; 3] {
        let at = 3 * (y * self.width + x);
        [self.rgb[at], self.rgb[at + 1], self.rgb[at + 2]]
    }

    /// The colour at a point between pixels, mixed from the four around it,
    /// for a line scaled to the recognizer's height.
    pub fn between(&self, x: f32, y: f32) -> [f32; 3] {
        let largest = |size: usize| real(size - 1);
        let (x, y) = (
            x.clamp(0.0, largest(self.width)),
            y.clamp(0.0, largest(self.height)),
        );
        let (left, top) = (whole(x.floor()), whole(y.floor()));
        let (right, bottom) = (
            (left + 1).min(self.width - 1),
            (top + 1).min(self.height - 1),
        );
        let (across, down) = (x - real(left), y - real(top));
        let mix = |a: [u8; 3], b: [u8; 3], by: f32| -> [f32; 3] {
            std::array::from_fn(|c| f32::from(a[c]) * (1.0 - by) + f32::from(b[c]) * by)
        };
        let upper = mix(self.at(left, top), self.at(right, top), across);
        let lower = mix(self.at(left, bottom), self.at(right, bottom), across);
        std::array::from_fn(|c| upper[c] * (1.0 - down) + lower[c] * down)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(colour: [f32; 3], want: [f32; 3]) -> bool {
        colour
            .iter()
            .zip(want)
            .all(|(got, want)| (got - want).abs() < 1e-4)
    }

    #[test]
    fn a_point_between_pixels_is_their_mix() {
        let picture = Picture::from_rgb(2, 1, vec![0, 0, 0, 200, 100, 50]);
        assert!(near(picture.between(0.5, 0.0), [100.0, 50.0, 25.0]));
        assert!(near(picture.between(1.0, 0.0), [200.0, 100.0, 50.0]));
    }

    #[test]
    fn a_point_past_the_edge_takes_the_edge() {
        let picture = Picture::from_rgb(1, 1, vec![9, 8, 7]);
        assert!(near(picture.between(-3.0, 5.0), [9.0, 8.0, 7.0]));
    }
}
