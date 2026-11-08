//! What a text means, as a point in bge-base-en-v1.5's space: 768 numbers
//! whose direction is the meaning, so two texts meaning alike point alike
//! and their dot product, each made a unit long, says how near they are.
//!
//! The model is BERT's, as OpenVINO exported it at eight bits: the text's
//! tokens in, the hidden state of every token out, and the first token's,
//! the classifier's, is the text's meaning, as bge was trained. It runs
//! at one fixed shape, 64 tokens, as the NPU needs: a launcher's things
//! and what's typed into it are a few words each, so a longer text is cut
//! there rather than paying for a wider model on every key.
//!
//! A text searched for is asked with bge's instruction for a short query
//! looking for a longer passage; the things searched among are embedded
//! plainly, as bge's authors say.

use std::path::Path;

use dictation::openvino::{Compiled, Core, Input, Priority, Request};
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams};

/// The tokens every text is cut or padded to.
pub const TOKENS: usize = 64;

/// How many numbers a meaning is.
pub const WIDTH: usize = 768;

/// bge's instruction before a query searching passages.
const ASKING: &str = "Represent this sentence for searching relevant passages: ";

/// The tokenizer and the compiled model, ready to embed.
pub struct Texts {
    tokenizer: Tokenizer,
    request: Request,
    // After the request, so the request goes first.
    _compiled: Compiled,
}

impl Texts {
    /// bge from `folder` (its `tokenizer.json` and `openvino_model.xml`),
    /// compiled for `device` at the fixed shape.
    ///
    /// # Errors
    ///
    /// When the files can't be read or the device can't run the model.
    pub fn load(core: &Core, folder: &Path, device: &str) -> Result<Self, String> {
        let mut tokenizer = Tokenizer::from_file(folder.join("tokenizer.json"))
            .map_err(|why| format!("reading the tokenizer: {why}"))?;
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length: TOKENS,
                ..TruncationParams::default()
            }))
            .map_err(|why| format!("cutting at {TOKENS} tokens: {why}"))?;
        tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::Fixed(TOKENS),
            ..PaddingParams::default()
        }));
        let mut model = core.read(&folder.join("openvino_model.xml"))?;
        let shape = [1, i64::try_from(TOKENS).unwrap_or(64)];
        model.fix_inputs(&[
            ("input_ids", &shape),
            ("attention_mask", &shape),
            ("token_type_ids", &shape),
        ])?;
        // A typed query waits on it, and the things it's asked among are
        // embedded once each and kept, a few seconds of work in all.
        let compiled = core.compile(&model, device, Priority::Waited)?;
        let request = compiled.request()?;
        Ok(Self {
            tokenizer,
            request,
            _compiled: compiled,
        })
    }

    /// What `text` means, a unit long; `asking` for a query rather than
    /// a thing searched among.
    ///
    /// # Errors
    ///
    /// When it can't be tokenized or the model fails.
    pub fn embed(&mut self, text: &str, asking: bool) -> Result<Vec<f32>, String> {
        let text = if asking {
            format!("{ASKING}{text}")
        } else {
            text.to_owned()
        };
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|why| format!("tokenizing: {why}"))?;
        let wide = |values: &[u32]| values.iter().map(|&n| i64::from(n)).collect::<Vec<i64>>();
        let ids = wide(encoding.get_ids());
        let mask = wide(encoding.get_attention_mask());
        let segments = wide(encoding.get_type_ids());
        let shape = [1, i64::try_from(TOKENS).unwrap_or(64)];
        self.request.infer(&[
            Input::I64("input_ids", &shape, &ids),
            Input::I64("attention_mask", &shape, &mask),
            Input::I64("token_type_ids", &shape, &segments),
        ])?;
        let (_, hidden) = self.request.output("last_hidden_state", None)?;
        let first = hidden
            .get(..WIDTH)
            .ok_or_else(|| format!("the model gave {} numbers, not {WIDTH}", hidden.len()))?;
        Ok(unit(first))
    }
}

/// `values` scaled to a unit long, so a dot product is the cosine.
pub fn unit(values: &[f32]) -> Vec<f32> {
    let length = values.iter().map(|v| v * v).sum::<f32>().sqrt();
    if length == 0.0 {
        return values.to_vec();
    }
    values.iter().map(|v| v / length).collect()
}

/// How near two meanings are, each a unit long: 1 for the same, 0 for
/// nothing in common.
pub fn near(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_meaning_is_made_a_unit_long() {
        let made = unit(&[3.0, 4.0]);
        assert!((made[0] - 0.6).abs() < 1e-6 && (made[1] - 0.8).abs() < 1e-6);
        assert_eq!(unit(&[0.0, 0.0]), vec![0.0, 0.0]);
    }

    #[test]
    fn nearness_is_the_cosine_of_two_units() {
        let a = unit(&[1.0, 0.0]);
        let b = unit(&[1.0, 1.0]);
        assert!((near(&a, &a) - 1.0).abs() < 1e-6);
        assert!((near(&a, &b) - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
        assert!(near(&a, &unit(&[0.0, 1.0])).abs() < 1e-6);
    }
}
