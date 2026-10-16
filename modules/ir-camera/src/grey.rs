//! Turning a frame of 16-bit greyscale samples into 8-bit grey, the format
//! every program that reads a camera knows.

use std::fmt;

/// The raw greyscale formats the relay reads: each sample is two bytes,
/// little-endian, with the picture in its lowest bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Raw {
    Y10,
    Y12,
    Y16,
}

impl Raw {
    /// The format by its V4L2 name.
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "Y10" => Some(Self::Y10),
            "Y12" => Some(Self::Y12),
            "Y16" => Some(Self::Y16),
            _ => None,
        }
    }

    /// The format's V4L2 code.
    pub fn fourcc(self) -> u32 {
        match self {
            Self::Y10 => crate::v4l2::fourcc(*b"Y10 "),
            Self::Y12 => crate::v4l2::fourcc(*b"Y12 "),
            Self::Y16 => crate::v4l2::fourcc(*b"Y16 "),
        }
    }

    /// How many of each sample's bits hold the picture.
    fn depth(self) -> u32 {
        match self {
            Self::Y10 => 10,
            Self::Y12 => 12,
            Self::Y16 => 16,
        }
    }
}

/// The shape of a frame: how many pixels wide and high, and how many bytes
/// apart its lines start, which a driver may pad past the pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shape {
    pub width: usize,
    pub height: usize,
    pub stride: usize,
}

impl Shape {
    /// How many bytes a whole frame of this shape takes, its last line
    /// unpadded.
    fn length(self, bytes_per_pixel: usize) -> usize {
        match self.height {
            0 => 0,
            height => (height - 1) * self.stride + self.width * bytes_per_pixel,
        }
    }
}

/// A frame too short for the shape it's meant to have.
#[derive(Debug, PartialEq, Eq)]
pub struct Short {
    pub needed: usize,
    pub got: usize,
}

impl fmt::Display for Short {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "a frame of {} bytes, short of {}",
            self.got, self.needed
        )
    }
}

/// Writes `raw`, a frame in `format` shaped `from`, into `grey`, shaped
/// `to`, keeping each sample's highest eight bits. A sample with bits set
/// above its depth, which a sound sensor never sends, comes out white.
pub fn convert(
    format: Raw,
    raw: &[u8],
    from: Shape,
    grey: &mut [u8],
    to: Shape,
) -> Result<(), Short> {
    for (needed, got) in [(from.length(2), raw.len()), (to.length(1), grey.len())] {
        if got < needed {
            return Err(Short { needed, got });
        }
    }
    let shift = format.depth() - 8;
    let width = from.width.min(to.width);
    for line in 0..from.height.min(to.height) {
        let (samples, _) = raw[line * from.stride..][..width * 2].as_chunks::<2>();
        let pixels = &mut grey[line * to.stride..][..width];
        for (pixel, sample) in pixels.iter_mut().zip(samples) {
            let value = u16::from_le_bytes(*sample) >> shift;
            *pixel = value.min(255) as u8;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples(values: &[u16]) -> Vec<u8> {
        values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect()
    }

    #[test]
    fn keeps_the_highest_eight_bits_of_each_depth() {
        let packed = Shape {
            width: 3,
            height: 1,
            stride: 6,
        };
        let grey_shape = Shape {
            width: 3,
            height: 1,
            stride: 3,
        };
        let mut grey = [0; 3];
        for (format, values, expected) in [
            (Raw::Y10, [0, 0x3ff, 0x200], [0, 255, 128]),
            (Raw::Y12, [0, 0xfff, 0x800], [0, 255, 128]),
            (Raw::Y16, [0, 0xffff, 0x8000], [0, 255, 128]),
        ] {
            convert(format, &samples(&values), packed, &mut grey, grey_shape).unwrap();
            assert_eq!(grey, expected, "{format:?}");
        }
    }

    #[test]
    fn reads_lines_their_stride_apart() {
        // Two lines of two pixels, each line padded to eight bytes.
        let raw = samples(&[4, 8, 0xdead, 0xdead, 12, 16, 0xdead, 0xdead]);
        let from = Shape {
            width: 2,
            height: 2,
            stride: 8,
        };
        let to = Shape {
            width: 2,
            height: 2,
            stride: 2,
        };
        let mut grey = [0; 4];
        convert(Raw::Y10, &raw, from, &mut grey, to).unwrap();
        assert_eq!(grey, [1, 2, 3, 4]);
    }

    #[test]
    fn needs_no_padding_after_the_last_line() {
        let raw = samples(&[4, 0xdead, 8]);
        let from = Shape {
            width: 1,
            height: 2,
            stride: 4,
        };
        let to = Shape {
            width: 1,
            height: 2,
            stride: 1,
        };
        let mut grey = [0; 2];
        convert(Raw::Y10, &raw, from, &mut grey, to).unwrap();
        assert_eq!(grey, [1, 2]);
    }

    #[test]
    fn writes_grey_lines_their_stride_apart() {
        let raw = samples(&[4, 8]);
        let from = Shape {
            width: 1,
            height: 2,
            stride: 2,
        };
        let to = Shape {
            width: 1,
            height: 2,
            stride: 3,
        };
        let mut grey = [9; 4];
        convert(Raw::Y10, &raw, from, &mut grey, to).unwrap();
        assert_eq!(grey, [1, 9, 9, 2]);
    }

    #[test]
    fn a_sample_past_its_depth_comes_out_white() {
        // Its lowest eight bits, once shifted, are all zero.
        let raw = samples(&[0x0800]);
        let shape = Shape {
            width: 1,
            height: 1,
            stride: 2,
        };
        let mut grey = [0; 1];
        convert(
            Raw::Y10,
            &raw,
            shape,
            &mut grey,
            Shape { stride: 1, ..shape },
        )
        .unwrap();
        assert_eq!(grey, [255]);
    }

    #[test]
    fn refuses_a_short_frame() {
        let from = Shape {
            width: 2,
            height: 2,
            stride: 8,
        };
        let to = Shape {
            width: 2,
            height: 2,
            stride: 2,
        };
        let mut grey = [0; 4];
        let result = convert(Raw::Y10, &[0; 11], from, &mut grey, to);
        assert_eq!(
            result,
            Err(Short {
                needed: 12,
                got: 11
            })
        );
    }

    #[test]
    fn refuses_a_short_grey_frame() {
        let from = Shape {
            width: 2,
            height: 2,
            stride: 4,
        };
        let to = Shape {
            width: 2,
            height: 2,
            stride: 2,
        };
        let mut grey = [0; 3];
        let result = convert(Raw::Y10, &[0; 8], from, &mut grey, to);
        assert_eq!(result, Err(Short { needed: 4, got: 3 }));
    }

    #[test]
    fn knows_formats_by_their_v4l2_names() {
        assert_eq!(Raw::named("Y10"), Some(Raw::Y10));
        assert_eq!(Raw::named("Y12"), Some(Raw::Y12));
        assert_eq!(Raw::named("Y16"), Some(Raw::Y16));
        assert_eq!(Raw::named("GREY"), None);
        assert_eq!(Raw::Y10.fourcc(), 0x2030_3159);
        assert_eq!(Raw::Y12.fourcc(), 0x2032_3159);
        assert_eq!(Raw::Y16.fourcc(), 0x2036_3159);
    }
}
