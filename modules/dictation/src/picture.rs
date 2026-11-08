//! A picture as the models take it: red, green and blue bytes a pixel,
//! row after row, whatever colours and depth it was saved in. Screenshots
//! are PNGs, which screen-text reads the words of and meaning looks at;
//! the camera's pictures are JPEGs, which meaning looks at too.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use png::{ColorType, Decoder, Transformations};
use zune_jpeg::JpegDecoder;
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;

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

/// Whether a file is a picture this reads, by its name.
pub fn readable(path: &Path) -> bool {
    path.extension()
        .and_then(|kind| kind.to_str())
        .is_some_and(|kind| {
            ["png", "jpg", "jpeg"]
                .iter()
                .any(|known| kind.eq_ignore_ascii_case(known))
        })
}

pub struct Picture {
    pub width: usize,
    pub height: usize,
    rgb: Vec<u8>,
}

impl Picture {
    /// The PNG or JPEG at `path`, by its name.
    pub fn read(path: &Path) -> Result<Self, String> {
        let named = |kind: &str| {
            path.extension()
                .is_some_and(|given| given.eq_ignore_ascii_case(kind))
        };
        if named("jpg") || named("jpeg") {
            Self::read_jpeg(path)
        } else {
            Self::read_png(path)
        }
    }

    fn read_png(path: &Path) -> Result<Self, String> {
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

    fn read_jpeg(path: &Path) -> Result<Self, String> {
        let file = File::open(path).map_err(|why| format!("opening {}: {why}", path.display()))?;
        // Grey and CMYK pictures alike come out as red, green and blue.
        let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGB);
        let mut decoder = JpegDecoder::new_with_options(BufReader::new(file), options);
        let rgb = decoder
            .decode()
            .map_err(|why| format!("reading {}: {why:?}", path.display()))?;
        let info = decoder
            .info()
            .ok_or_else(|| format!("{} has no size", path.display()))?;
        let (width, height) = (usize::from(info.width), usize::from(info.height));
        if rgb.len() != width * height * 3 {
            return Err(format!("{} didn't come out as colour", path.display()));
        }
        Ok(Self::from_rgb(width, height, rgb))
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

    /// The picture shrunk or stretched to `width` by `height`, as red,
    /// green and blue from 0 to 255 a pixel, row after row, the way
    /// Pillow's bilinear resize makes it: each pixel a tent-weighted mix of
    /// those it covers, widened by how far the picture shrinks, so a
    /// screenshot many times wider is averaged rather than sampled. That
    /// is how the models that look at pictures were trained (SigLIP's
    /// processor resizes with Pillow), so a picture means to them here
    /// what it meant there.
    pub fn resized(&self, width: usize, height: usize) -> Vec<f32> {
        let across = Tent::new(self.width, width);
        let down = Tent::new(self.height, height);
        // Across first, every row, then down, as Pillow does.
        let mut wide = vec![0.0_f32; self.height * width * 3];
        for y in 0..self.height {
            for (x, (first, weights)) in across.taps.iter().enumerate() {
                let mut colour = [0.0_f32; 3];
                for (i, weight) in weights.iter().enumerate() {
                    let pixel = self.at(first + i, y);
                    for c in 0..3 {
                        colour[c] += f32::from(pixel[c]) * weight;
                    }
                }
                wide[3 * (y * width + x)..3 * (y * width + x) + 3].copy_from_slice(&colour);
            }
        }
        let mut out = vec![0.0_f32; height * width * 3];
        for (y, (first, weights)) in down.taps.iter().enumerate() {
            for x in 0..width {
                let mut colour = [0.0_f32; 3];
                for (i, weight) in weights.iter().enumerate() {
                    let at = 3 * ((first + i) * width + x);
                    for c in 0..3 {
                        colour[c] += wide[at + c] * weight;
                    }
                }
                out[3 * (y * width + x)..3 * (y * width + x) + 3].copy_from_slice(&colour);
            }
        }
        out
    }
}

/// Which pixels along one side each new pixel mixes, from the first, and
/// by how much each: Pillow's bilinear filter, a tent one pixel wide at
/// either side, widened by the shrink.
struct Tent {
    taps: Vec<(usize, Vec<f32>)>,
}

impl Tent {
    fn new(from: usize, to: usize) -> Self {
        let scale = real(from) / real(to.max(1));
        let widened = scale.max(1.0);
        let taps = (0..to)
            .map(|at| {
                let centre = (real(at) + 0.5) * scale;
                let first = whole((centre - widened + 0.5).floor());
                let last = whole((centre + widened + 0.5).floor()).min(from);
                let mut weights: Vec<f32> = (first..last)
                    .map(|i| (1.0 - ((real(i) - centre + 0.5) / widened).abs()).max(0.0))
                    .collect();
                let sum: f32 = weights.iter().sum();
                if sum > 0.0 {
                    for weight in &mut weights {
                        *weight /= sum;
                    }
                }
                (first, weights)
            })
            .collect();
        Self { taps }
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

    #[test]
    fn one_colour_stays_that_colour_at_any_size() {
        let picture = Picture::from_rgb(7, 5, [10, 120, 250].repeat(35));
        for (width, height) in [(3, 2), (7, 5), (16, 9)] {
            let made = picture.resized(width, height);
            assert_eq!(made.len(), width * height * 3);
            assert!(
                made.chunks_exact(3)
                    .all(|pixel| near([pixel[0], pixel[1], pixel[2]], [10.0, 120.0, 250.0]))
            );
        }
    }

    #[test]
    fn a_row_resized_is_what_pillow_makes_of_it() {
        // Grey rows resized across, against what Pillow 12's
        // `Image.resize((to, 1), Resampling.BILINEAR)` gave for each,
        // which rounds to whole bytes where this keeps the fraction. A
        // half may round either way in Pillow's fixed-point weights, and
        // sits a float's error past a half here, so a hair more is allowed.
        let cases: [(&[u8], &[f32]); 4] = [
            (&[0, 255, 0, 255], &[109.0, 146.0]),
            (&[0, 50, 100, 150, 200, 250], &[19.0, 89.0, 161.0, 231.0]),
            (&[0, 255], &[0.0, 25.0, 128.0, 230.0, 255.0]),
            (&[10, 20, 30, 40, 50, 60, 70, 80, 90], &[32.0, 68.0]),
        ];
        for (row, pillow) in cases {
            let grey = row.iter().flat_map(|&v| [v; 3]).collect();
            let made = Picture::from_rgb(row.len(), 1, grey).resized(pillow.len(), 1);
            let reds: Vec<f32> = made.chunks_exact(3).map(|pixel| pixel[0]).collect();
            assert!(
                reds.iter()
                    .zip(pillow)
                    .all(|(got, want)| (got - want).abs() <= 0.501),
                "{row:?} came to {reds:?}, where Pillow made {pillow:?}"
            );
        }
    }

    #[test]
    fn a_jpeg_and_a_png_are_both_pictures() {
        assert!(readable(Path::new("/a/b.png")));
        assert!(readable(Path::new("/a/b.JPG")));
        assert!(readable(Path::new("/a/b.jpeg")));
        assert!(!readable(Path::new("/a/b.mp4")));
        assert!(!readable(Path::new("/a/png")));
    }
}
