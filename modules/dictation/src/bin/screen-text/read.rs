//! Reads each line found: `PP-OCRv5`'s English recognizer takes the line
//! scaled to 48 pixels tall and gives, for each step along it, how likely
//! each of its characters is; the likeliest at each step, repeats and
//! blanks dropped, is the line's text (CTC's decoding).
//!
//! The NPU takes fixed shapes, so the recognizer is compiled at three
//! widths, and a line goes to the narrowest that holds it, the rest left
//! blank as `PaddleOCR` leaves it; a line wider than the widest is squeezed
//! into it.

use std::path::Path;

use dictation::openvino::{Compiled, Core, Input, Priority, Request};

use crate::find::Place;
use crate::picture::{Picture, dimension, real, whole};

/// The height the recognizer reads a line at.
const HEIGHT: usize = 48;

/// The widths it's compiled at: a word or two, a sentence, and the widest
/// `PaddleOCR` reads.
const WIDTHS: [usize; 3] = [320, 960, 3200];

/// How sure the recognizer must be of a line, on average, for it to be
/// kept, `PaddleOCR`'s own.
const SURE: f32 = 0.5;

/// A line read, and how sure the recognizer was of it.
#[derive(Debug, PartialEq)]
pub struct Read {
    pub text: String,
    pub sure: f32,
}

pub struct Reader {
    /// One request a width, narrowest first, each with what it runs.
    widths: Vec<(usize, Compiled, Request)>,
    /// The characters the recognizer knows, in its order, after the blank.
    characters: Vec<String>,
}

impl Reader {
    pub fn load(
        core: &Core,
        model: &Path,
        characters: &Path,
        device: &str,
    ) -> Result<Self, String> {
        let characters = std::fs::read_to_string(characters)
            .map_err(|why| format!("reading {}: {why}", characters.display()))?;
        // The recognizer knows a space past the characters it lists.
        let characters = characters
            .lines()
            .map(str::to_owned)
            .chain([" ".to_owned()])
            .collect();
        let mut widths = Vec::new();
        for width in WIDTHS {
            let mut read = core.read(model)?;
            read.fix_input(&[1, 3, dimension(HEIGHT), dimension(width)])?;
            let compiled = core.compile(&read, device, Priority::Background)?;
            let request = compiled.request()?;
            widths.push((width, compiled, request));
        }
        Ok(Self { widths, characters })
    }

    /// The line at `place` in the picture, if the recognizer is sure of it.
    pub fn read(&mut self, picture: &Picture, place: Place) -> Result<Option<Read>, String> {
        let scale = real(HEIGHT) / real(place.height());
        let scaled = whole((real(place.width()) * scale).ceil()).max(1);
        let widest = WIDTHS[WIDTHS.len() - 1];
        let (width, _, request) = self
            .widths
            .iter_mut()
            .find(|(width, _, _)| *width >= scaled.min(widest))
            .ok_or("the recognizer has no widths")?;
        let width = *width;
        let drawn = scaled.min(width);
        let across = real(place.width()) / real(drawn);
        let down = real(place.height()) / real(HEIGHT);
        let mut line = vec![0.0; 3 * HEIGHT * width];
        for y in 0..HEIGHT {
            for x in 0..drawn {
                let [red, green, blue] = picture.between(
                    real(place.left) + (real(x) + 0.5) * across - 0.5,
                    real(place.top) + (real(y) + 0.5) * down - 0.5,
                );
                for (plane, value) in [blue, green, red].into_iter().enumerate() {
                    line[plane * HEIGHT * width + y * width + x] = value / 127.5 - 1.0;
                }
            }
        }
        let shape = [1, 3, dimension(HEIGHT), dimension(width)];
        let (dims, likely) =
            request.run(&[Input::F32("x", &shape, &line)], "fetch_name_0", None)?;
        let known = self.characters.len() + 1;
        if dims.last().copied() != Some(dimension(known)) {
            return Err(format!(
                "the recognizer gave {dims:?}, not {known} characters a step"
            ));
        }
        let read = decode(&likely, known, &self.characters);
        Ok((!read.text.trim().is_empty() && read.sure >= SURE).then_some(read))
    }
}

/// CTC's decoding: the likeliest character at each step, a run of the same
/// one kept once, and the blank, the first, dropped; how sure is the mean
/// of the kept steps' likelihoods.
fn decode(likely: &[f32], known: usize, characters: &[String]) -> Read {
    let mut text = String::new();
    let mut sure = Vec::new();
    let mut last = 0;
    for step in likely.chunks_exact(known) {
        let (best, chance) =
            step.iter()
                .copied()
                .enumerate()
                .fold((0, f32::MIN), |kept, (at, chance)| {
                    if chance > kept.1 { (at, chance) } else { kept }
                });
        if best != 0 && best != last {
            text.push_str(&characters[best - 1]);
            sure.push(chance);
        }
        last = best;
    }
    let sure = if sure.is_empty() {
        0.0
    } else {
        sure.iter().sum::<f32>() / real(sure.len())
    };
    Read { text, sure }
}

/// Lines in reading order: rows top to bottom, a row's lines left to
/// right; two lines are in one row when their middles are nearer than
/// half the shorter's height.
pub fn in_order(mut places: Vec<Place>) -> Vec<Place> {
    places.sort_by_key(|place| place.top + place.bottom);
    let mut rows: Vec<Vec<Place>> = Vec::new();
    for place in places {
        let middle = place.top + place.bottom;
        match rows.last_mut() {
            Some(row)
                if row.iter().any(|other| {
                    middle.abs_diff(other.top + other.bottom) < place.height().min(other.height())
                }) =>
            {
                row.push(place);
            }
            _ => rows.push(vec![place]),
        }
    }
    rows.into_iter()
        .flat_map(|mut row| {
            row.sort_by_key(|place| place.left);
            row
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn characters() -> Vec<String> {
        ["a", "b", " "].map(str::to_owned).to_vec()
    }

    #[test]
    fn a_run_is_one_character_and_a_blank_parts_two() {
        // Steps: a, a, blank, a, b, b; four classes, the blank first.
        let likely = [
            [0.1, 0.8, 0.05, 0.05],
            [0.1, 0.7, 0.1, 0.1],
            [0.9, 0.05, 0.03, 0.02],
            [0.1, 0.6, 0.2, 0.1],
            [0.0, 0.1, 0.9, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ]
        .concat();
        let read = decode(&likely, 4, &characters());
        assert_eq!(read.text, "aab");
        assert!((read.sure - (0.8 + 0.6 + 0.9) / 3.0).abs() < 1e-6);
    }

    #[test]
    fn nothing_but_blanks_reads_nothing() {
        let read = decode(&[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0], 4, &characters());
        assert_eq!(
            read,
            Read {
                text: String::new(),
                sure: 0.0
            }
        );
    }

    fn at(left: usize, top: usize) -> Place {
        Place {
            left,
            top,
            right: left + 50,
            bottom: top + 20,
        }
    }

    #[test]
    fn lines_read_by_rows_then_across() {
        let order = in_order(vec![at(200, 52), at(10, 50), at(10, 10), at(300, 12)]);
        assert_eq!(order, [at(10, 10), at(300, 12), at(10, 50), at(200, 52)]);
    }
}
