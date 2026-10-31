//! Finds where a screenshot has text: PP-OCRv5's detector gives each
//! pixel how likely it is to be inside a line of text, and the lines are
//! the regions where that's likely, each widened to take in its edges, as
//! `PaddleOCR`'s own DB post-processing does.
//!
//! The NPU takes one fixed shape, so the picture is read at its own scale
//! in tiles of that shape, overlapping so a line across a tile's edge is
//! whole in one of them, and the tiles' maps are joined into one. Reading
//! at its own scale keeps a screenshot's small text as large as it was.

use dictation::openvino::{Compiled, Core, Input, Priority, Request};

use crate::picture::{Picture, dimension, real, whole};

/// The detector's one shape, a square tile.
pub const TILE: usize = 640;

/// How far tiles overlap: more than a line of screen text is tall.
const OVERLAP: usize = 96;

/// The colour scale `ImageNet`'s models learnt, which `PaddleOCR` gives its
/// pixels in the order it reads them: blue, green and red.
const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const DEVIATION: [f32; 3] = [0.229, 0.224, 0.225];

/// How likely a pixel must be to be in a line, how likely a line's box on
/// the whole, and how much a line's region is widened, `PaddleOCR`'s own.
const LIKELY: f32 = 0.3;
const BOX_LIKELY: f32 = 0.6;
const WIDENED: f32 = 1.5;

/// The fewest pixels a line may be across, before and after widening.
const SMALLEST: usize = 3;

/// Where a line is, in pixels: its left, top, right and bottom, the right
/// and bottom past its last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Place {
    pub left: usize,
    pub top: usize,
    pub right: usize,
    pub bottom: usize,
}

impl Place {
    pub fn width(&self) -> usize {
        self.right - self.left
    }

    pub fn height(&self) -> usize {
        self.bottom - self.top
    }
}

pub struct Finder {
    _compiled: Compiled,
    request: Request,
}

impl Finder {
    pub fn load(core: &Core, model: &std::path::Path, device: &str) -> Result<Self, String> {
        let mut read = core.read(model)?;
        read.fix_input(&[1, 3, dimension(TILE), dimension(TILE)])?;
        let compiled = core.compile(&read, device, Priority::Background)?;
        let request = compiled.request()?;
        Ok(Self {
            _compiled: compiled,
            request,
        })
    }

    /// Where every line of text in the picture is.
    pub fn find(&mut self, picture: &Picture) -> Result<Vec<Place>, String> {
        let map = self.map(picture)?;
        Ok(places(&map, picture.width, picture.height))
    }

    /// How likely each pixel is to be in a line, the tiles' maps joined,
    /// the likelier where two overlap.
    fn map(&mut self, picture: &Picture) -> Result<Vec<f32>, String> {
        let mut map = vec![0.0_f32; picture.width * picture.height];
        let mut tile = vec![0.0; 3 * TILE * TILE];
        for top in starts(picture.height) {
            for left in starts(picture.width) {
                fill(&mut tile, picture, left, top);
                let shape = [1, 3, dimension(TILE), dimension(TILE)];
                let (_, likely) =
                    self.request
                        .run(&[Input::F32("x", &shape, &tile)], "fetch_name_0", None)?;
                if likely.len() != TILE * TILE {
                    return Err(format!("the detector gave {} values a tile", likely.len()));
                }
                for y in 0..TILE.min(picture.height - top) {
                    for x in 0..TILE.min(picture.width - left) {
                        let at = (top + y) * picture.width + left + x;
                        map[at] = map[at].max(likely[y * TILE + x]);
                    }
                }
            }
        }
        Ok(map)
    }
}

/// Where each tile starts along a side, the last flush with its end.
fn starts(side: usize) -> Vec<usize> {
    if side <= TILE {
        return vec![0];
    }
    let mut starts: Vec<usize> = (0..side - TILE).step_by(TILE - OVERLAP).collect();
    starts.push(side - TILE);
    starts
}

/// A tile of the picture as the detector takes it, blue, green and red
/// planes scaled as it learnt; past the picture's edge, the mean.
fn fill(tile: &mut [f32], picture: &Picture, left: usize, top: usize) {
    tile.fill(0.0);
    for y in 0..TILE.min(picture.height - top) {
        for x in 0..TILE.min(picture.width - left) {
            let [red, green, blue] = picture.at(left + x, top + y);
            for (plane, value) in [blue, green, red].into_iter().enumerate() {
                tile[plane * TILE * TILE + y * TILE + x] =
                    (f32::from(value) / 255.0 - MEAN[plane]) / DEVIATION[plane];
            }
        }
    }
}

/// The lines in a map of how likely each pixel is to be in one: each
/// region of likely pixels touching side by side, kept if its box is
/// likely on the whole, and widened by its area over its perimeter.
pub fn places(map: &[f32], width: usize, height: usize) -> Vec<Place> {
    let mut seen = vec![false; map.len()];
    let mut places = Vec::new();
    let mut stack = Vec::new();
    for start in 0..map.len() {
        if seen[start] || map[start] <= LIKELY {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        let (mut left, mut top, mut right, mut bottom) = (width, height, 0, 0);
        while let Some(at) = stack.pop() {
            let (x, y) = (at % width, at / width);
            (left, top, right, bottom) = (left.min(x), top.min(y), right.max(x), bottom.max(y));
            let beside = [
                (x > 0).then(|| at - 1),
                (x + 1 < width).then(|| at + 1),
                (y > 0).then(|| at - width),
                (y + 1 < height).then(|| at + width),
            ];
            for next in beside.into_iter().flatten() {
                if !seen[next] && map[next] > LIKELY {
                    seen[next] = true;
                    stack.push(next);
                }
            }
        }
        let found = Place {
            left,
            top,
            right: right + 1,
            bottom: bottom + 1,
        };
        if found.width().min(found.height()) < SMALLEST
            || likeliness(map, width, found) < BOX_LIKELY
        {
            continue;
        }
        let widened = widen(found, width, height);
        if widened.width().min(widened.height()) >= SMALLEST + 2 {
            places.push(widened);
        }
    }
    places
}

/// How likely a box's pixels are, on average.
fn likeliness(map: &[f32], width: usize, place: Place) -> f32 {
    let sum: f32 = (place.top..place.bottom)
        .flat_map(|y| &map[y * width + place.left..y * width + place.right])
        .sum();
    sum / real(place.width() * place.height())
}

/// A box widened on every side by its area over its perimeter, times
/// `PaddleOCR`'s ratio, since the detector learnt lines shrunk by as much.
fn widen(place: Place, width: usize, height: usize) -> Place {
    let (across, down) = (real(place.width()), real(place.height()));
    let by = whole((across * down * WIDENED / (2.0 * (across + down))).round());
    Place {
        left: place.left.saturating_sub(by),
        top: place.top.saturating_sub(by),
        right: (place.right + by).min(width),
        bottom: (place.bottom + by).min(height),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_cover_a_side_and_end_flush_with_it() {
        assert_eq!(starts(300), [0]);
        assert_eq!(starts(TILE), [0]);
        assert_eq!(starts(1000), [0, 360]);
        assert_eq!(starts(1500), [0, 544, 860]);
    }

    #[test]
    fn a_likely_region_is_one_line_widened() {
        let (width, height) = (40, 20);
        let mut map = vec![0.0; width * height];
        for y in 8..12 {
            for x in 10..30 {
                map[y * width + x] = 0.9;
            }
        }
        // 20 by 4: widened by 20 * 4 * 1.5 / 48, 2.5, which rounds to 3.
        assert_eq!(
            places(&map, width, height),
            [Place {
                left: 7,
                top: 5,
                right: 33,
                bottom: 15
            }]
        );
    }

    #[test]
    fn two_regions_apart_are_two_lines() {
        let (width, height) = (40, 30);
        let mut map = vec![0.0; width * height];
        for (top, bottom) in [(4, 9), (18, 23)] {
            for y in top..bottom {
                for x in 5..35 {
                    map[y * width + x] = 0.8;
                }
            }
        }
        assert_eq!(places(&map, width, height).len(), 2);
    }

    #[test]
    fn a_faint_or_tiny_region_is_no_line() {
        let (width, height) = (20, 20);
        let mut map = vec![0.0; width * height];
        for y in 2..8 {
            for x in 2..18 {
                map[y * width + x] = 0.4;
            }
        }
        map[15 * width + 10] = 0.99;
        assert!(places(&map, width, height).is_empty());
    }
}
