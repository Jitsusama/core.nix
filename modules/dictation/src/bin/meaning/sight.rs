//! What a picture shows and what words say of one, as points in SigLIP 2's
//! one space for both: a picture and words that say what's in it point
//! alike, so a screenshot is found by typing what it's of, from what the
//! model saw in it rather than from any caption.
//!
//! The model is Google's siglip2-base-patch16-224, as onnx-community
//! exported its two towers at sixteen bits (modules/meaning/models.nix).
//! Each runs at one fixed shape, as the NPU needs: a picture shrunk to 224
//! pixels a side, as SigLIP's processor shrinks it, and words cut or
//! padded to 64 tokens, lowercased, as SigLIP was trained. What each tower
//! pools is the point, made a unit long, so a dot product is the cosine.
//!
//! Words go as they're typed, with no template around them: "a screenshot
//! of" lifts every screenshot alike, a pizza's as much as a chart's, where
//! bare words keep them apart (measured on the NPU, 2026-11-07).

use std::path::Path;

use dictation::openvino::{Compiled, Core, Input, Priority, Request};
use dictation::picture::{Picture, dimension};
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams};

use crate::text::unit;

/// The side a picture is shrunk to.
pub const SIDE: usize = 224;

/// The tokens words are cut or padded to.
const TOKENS: usize = 64;

/// How near a picture has to be to words to be answered, as SigLIP 2's
/// cosine. Judged by eye over 89 screenshots and 7 camera pictures on the
/// NPU: what was asked for came at 0.12 to 0.16 ("a terminal full of
/// code", "an error message", a keyboard), and what wasn't came under
/// 0.12 (a video call asked of chats, 0.116; a map asked of a donut
/// chart, 0.084). SigLIP's own even odds, 0.149 from its learned bias and
/// scale, would lose the web pages and chats it got right.
pub const FLOOR: f32 = 0.12;

/// Words, as SigLIP's text tower reads them.
pub struct Words {
    tokenizer: Tokenizer,
    request: Request,
    // After the request, so the request goes first.
    _compiled: Compiled,
}

impl Words {
    /// The text tower from `folder` (its `text.onnx` and
    /// `tokenizer.json`), compiled for `device`.
    ///
    /// # Errors
    ///
    /// When the files can't be read or the device can't run the tower.
    pub fn load(core: &Core, folder: &Path, device: &str) -> Result<Self, String> {
        let mut tokenizer = Tokenizer::from_file(folder.join("tokenizer.json"))
            .map_err(|why| format!("reading SigLIP's tokenizer: {why}"))?;
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length: TOKENS,
                ..TruncationParams::default()
            }))
            .map_err(|why| format!("cutting at {TOKENS} tokens: {why}"))?;
        tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::Fixed(TOKENS),
            pad_id: 0,
            pad_token: "<pad>".to_owned(),
            ..PaddingParams::default()
        }));
        let mut model = core.read(&folder.join("text.onnx"))?;
        model.fix_inputs(&[("input_ids", &[1, dimension(TOKENS)])])?;
        // Typed words wait on it, as on bge.
        let compiled = core.compile(&model, device, Priority::Waited)?;
        let request = compiled.request()?;
        Ok(Self {
            tokenizer,
            request,
            _compiled: compiled,
        })
    }

    /// What `words` say of a picture, a unit long.
    ///
    /// # Errors
    ///
    /// When they can't be tokenized or the tower fails.
    pub fn embed(&mut self, words: &str) -> Result<Vec<f32>, String> {
        let encoding = self
            .tokenizer
            .encode(words.to_lowercase(), true)
            .map_err(|why| format!("tokenizing: {why}"))?;
        let ids: Vec<i64> = encoding.get_ids().iter().map(|&n| i64::from(n)).collect();
        self.request
            .infer(&[Input::I64("input_ids", &[1, dimension(TOKENS)], &ids)])?;
        let (_, pooled) = self.request.output("pooler_output", None)?;
        Ok(unit(&pooled))
    }
}

/// Pictures, as SigLIP's vision tower sees them.
pub struct Looks {
    request: Request,
    // After the request, so the request goes first.
    _compiled: Compiled,
}

impl Looks {
    /// The vision tower from `folder` (its `vision.onnx`), compiled for
    /// `device`.
    ///
    /// # Errors
    ///
    /// When the file can't be read or the device can't run the tower.
    pub fn load(core: &Core, folder: &Path, device: &str) -> Result<Self, String> {
        let mut model = core.read(&folder.join("vision.onnx"))?;
        let side = dimension(SIDE);
        model.fix_inputs(&[("pixel_values", &[1, 3, side, side])])?;
        // Nobody waits on a picture being looked at: it comes after a
        // call's captions and dictation, and after every query.
        let compiled = core.compile(&model, device, Priority::Background)?;
        let request = compiled.request()?;
        Ok(Self {
            request,
            _compiled: compiled,
        })
    }

    /// What `picture` shows, a unit long.
    ///
    /// # Errors
    ///
    /// When the tower fails.
    pub fn look(&mut self, picture: &Picture) -> Result<Vec<f32>, String> {
        let side = dimension(SIDE);
        self.request.infer(&[Input::F32(
            "pixel_values",
            &[1, 3, side, side],
            &pixels(picture),
        )])?;
        let (_, pooled) = self.request.output("pooler_output", None)?;
        Ok(unit(&pooled))
    }
}

/// A picture as SigLIP's processor gives it to the tower: shrunk to the
/// side, each colour from -1 to 1, and each colour's plane whole before
/// the next, red, then green, then blue.
pub fn pixels(picture: &Picture) -> Vec<f32> {
    let shrunk = picture.resized(SIDE, SIDE);
    let mut planes = vec![0.0; shrunk.len()];
    let area = SIDE * SIDE;
    for (at, pixel) in shrunk.as_chunks::<3>().0.iter().enumerate() {
        for (colour, value) in pixel.iter().enumerate() {
            planes[colour * area + at] = value / 127.5 - 1.0;
        }
    }
    planes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_is_given_as_planes_of_colour_from_minus_one_to_one() {
        let red = Picture::from_rgb(2, 2, [255, 0, 0].repeat(4));
        let planes = pixels(&red);
        let area = SIDE * SIDE;
        assert_eq!(planes.len(), 3 * area);
        assert!(planes[..area].iter().all(|&v| (v - 1.0).abs() < 1e-5));
        assert!(planes[area..].iter().all(|&v| (v + 1.0).abs() < 1e-5));
    }
}
