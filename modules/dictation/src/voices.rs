//! Telling a call's far-end voices apart: TitaNet small (titanet.nix), run
//! through OpenVINO on the CPU, gives each take one embedding as it's
//! heard, and at the call's end the takes are grouped by how alike their
//! embeddings are. No sound is kept for it, only 192 numbers a take.
//!
//! A take is the cutter's (takes.rs), cut where the far end pauses, so
//! now and then one holds two voices, and a group may be a mix; grouping
//! takes by average linkage up to `APART` and naming only groups that hold
//! at least `LEAST_SHARE` of the far end's speech left those mixes unnamed.
//! On three AMI meetings' far ends, so grouped, of the time a voice was
//! named 93 to 96% was named right.
//!
//! The features are the ones sherpa-onnx feeds TitaNet: 80 log mel bands
//! on Slaney's scale from 0 to 7600 Hz, over 25 ms Hann windows every
//! 10 ms, each pre-emphasised and padded to 512 points, and each band then
//! normalised across the take. Worked out so, TitaNet's embeddings are
//! sherpa-onnx's to five places.

use std::f64::consts::PI;
use std::path::Path;

use crate::mel::{Complex, hz_to_mel, mel_to_hz, transform};
use crate::openvino::{Core, Input, Request, Result};

/// The rate it hears at.
pub const RATE: usize = 16_000;
/// How far apart, in cosine distance, two groups of takes may be and still
/// be one voice. Further, AMI's voices merged; nearer, more of each voice
/// was left unnamed.
pub const APART: f32 = 0.6;
/// The least of the far end's speech a group holds to be named as a
/// voice: smaller ones were mostly takes holding two voices.
pub const LEAST_SHARE: f64 = 0.1;

/// A window's samples, 25 ms, and how far each is from the last, 10 ms.
const LENGTH: usize = 400;
const SHIFT: usize = 160;
/// The transform's points, the window padded, and the bins it gives.
const POINTS: usize = 512;
const BINS: usize = POINTS / 2 + 1;
/// TitaNet's bands, and the highest frequency they reach: 400 Hz under
/// half the rate, as the model was trained.
pub const BANDS: usize = 80;
const TOP_HZ: f64 = 7600.0;
const PREEMPHASIS: f64 = 0.97;
/// What a band's energy is floored at before its log, as sherpa-onnx
/// floors it: a single float's epsilon.
const FLOOR: f64 = f32::EPSILON as f64;
/// What a band's spread is padded by before it divides, so a band that
/// never moves isn't divided by nothing.
const SPREAD_PAD: f64 = 1e-5;

/// TitaNet, compiled, and what every take's features are worked out with.
pub struct Voices {
    request: Request,
    bands: Bands,
    _core: Core,
}

impl Voices {
    /// TitaNet from its ONNX `model`, compiled for the CPU on one thread,
    /// since a take's embedding is small and comes while Parakeet hears.
    ///
    /// # Errors
    ///
    /// When OpenVINO can't read or compile it.
    pub fn load(model: &Path) -> Result<Self> {
        let core = Core::new(None)?;
        let compiled = core.compile_on_one_thread(&core.read(model)?)?;
        Ok(Self {
            request: compiled.request()?,
            bands: Bands::new(),
            _core: core,
        })
    }

    /// The embedding of `samples`, at `RATE`, or None when they're shorter
    /// than a window.
    ///
    /// # Errors
    ///
    /// When the model refuses them or doesn't run.
    pub fn embed(&mut self, samples: &[f32]) -> Result<Option<Vec<f32>>> {
        let Some((frames, features)) = self.bands.features(samples) else {
            return Ok(None);
        };
        let frames = i64::try_from(frames).map_err(|_| "a take too long to embed".to_owned())?;
        let bands = i64::try_from(BANDS).expect("80 bands fit in an i64");
        let (_, embedding) = self.request.run(
            &[
                Input::F32("audio_signal", &[1, bands, frames], &features),
                Input::I64("length", &[1], &[frames]),
            ],
            "embs",
            None,
        )?;
        Ok(Some(embedding))
    }
}

/// The window, the bands' triangles and the transform's roots, made once.
struct Bands {
    window: Vec<f64>,
    filters: Vec<f64>,
    roots: Vec<Complex>,
}

impl Bands {
    fn new() -> Self {
        let window = (0..LENGTH)
            .map(|i| 0.5 - 0.5 * (2.0 * PI * as_f64(i) / as_f64(LENGTH)).cos())
            .collect();
        let roots = (0..POINTS)
            .map(|j| {
                let angle = -2.0 * PI * as_f64(j) / as_f64(POINTS);
                (angle.cos(), angle.sin())
            })
            .collect();
        Self {
            window,
            filters: filters(),
            roots,
        }
    }

    /// Each window's log band energies, a row of `BANDS` a window, or None
    /// when there's not a window's worth.
    fn log_bands(&self, samples: &[f32]) -> Option<(usize, Vec<f64>)> {
        let frames = samples.len().checked_sub(LENGTH)? / SHIFT + 1;
        let mut out = vec![0.0; frames * BANDS];
        let mut frame = vec![(0.0, 0.0); POINTS];
        let mut spectrum = vec![(0.0, 0.0); POINTS];
        let mut power = [0.0; BINS];
        let (rows, _) = out.as_chunks_mut::<BANDS>();
        for (t, row) in rows.iter_mut().enumerate() {
            let window = &samples[t * SHIFT..t * SHIFT + LENGTH];
            // Each sample less most of the one before, the first less most
            // of itself, as Kaldi pre-emphasises.
            let mut before = f64::from(window[0]);
            for (i, (slot, &sample)) in frame.iter_mut().zip(window).enumerate() {
                let sample = f64::from(sample);
                let emphasised = sample - PREEMPHASIS * if i == 0 { sample } else { before };
                before = sample;
                *slot = (emphasised * self.window[i], 0.0);
            }
            frame[LENGTH..].fill((0.0, 0.0));
            transform(&frame, 0, 1, POINTS, &mut spectrum, &self.roots, 1);
            for (p, &(re, im)) in power.iter_mut().zip(&spectrum) {
                *p = re * re + im * im;
            }
            for (band, energy) in row.iter_mut().enumerate() {
                let filter = &self.filters[band * BINS..(band + 1) * BINS];
                let sum: f64 = filter.iter().zip(&power).map(|(w, p)| w * p).sum();
                *energy = sum.max(FLOOR).ln();
            }
        }
        Some((frames, out))
    }

    /// TitaNet's input for `samples`: each band normalised across them and
    /// laid out band by band, `BANDS` rows of a value a window, with how
    /// many windows there are.
    fn features(&self, samples: &[f32]) -> Option<(usize, Vec<f32>)> {
        let (frames, logs) = self.log_bands(samples)?;
        let mut out = vec![0.0; frames * BANDS];
        for band in 0..BANDS {
            let values = || logs.iter().skip(band).step_by(BANDS);
            let mean = values().sum::<f64>() / as_f64(frames);
            let spread =
                (values().map(|v| (v - mean).powi(2)).sum::<f64>() / as_f64(frames)).sqrt();
            for (t, value) in values().enumerate() {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "TitaNet takes its features as 32-bit floats"
                )]
                let normal = ((value - mean) / (spread + SPREAD_PAD)) as f32;
                out[band * frames + t] = normal;
            }
        }
        Some((frames, out))
    }
}

/// `BANDS` triangles over the bins from 0 to `TOP_HZ`, evenly spaced on
/// Slaney's scale, each scaled by the width it spans.
fn filters() -> Vec<f64> {
    let top = hz_to_mel(TOP_HZ);
    let edge = |i: usize| mel_to_hz(top * as_f64(i) / as_f64(BANDS + 1));
    let mut weights = vec![0.0; BANDS * BINS];
    for band in 0..BANDS {
        let (left, centre, right) = (edge(band), edge(band + 1), edge(band + 2));
        for bin in 0..BINS {
            let hz = as_f64(bin * RATE) / as_f64(POINTS);
            let weight = if hz > left && hz <= centre {
                (hz - left) / (centre - left)
            } else if hz > centre && hz < right {
                (right - hz) / (right - centre)
            } else {
                0.0
            };
            weights[band * BINS + bin] = weight * 2.0 / (right - left);
        }
    }
    weights
}

fn as_f64(n: usize) -> f64 {
    f64::from(u32::try_from(n).expect("a count of samples or bins fits in 32 bits"))
}

/// A take's embedding, and how many samples it holds.
pub struct Heard {
    pub embedding: Vec<f32>,
    pub samples: usize,
}

/// Which voice each take is, in the order the voices first spoke, or None
/// for a take that's no voice for sure. Takes are grouped by average
/// linkage on cosine distance until the nearest two groups are further
/// than `APART`; a group is a voice when it holds at least `LEAST_SHARE`
/// of the samples, and when `people` says how many others were on the
/// call, only that many of the largest are. One voice is named only when
/// at least `LEAST_SHARE` of the speech is left outside it, so there's
/// someone else to tell it apart from; a far end of one person whose odd
/// takes fall outside its voice stays Others. `heard` is in the order it
/// was said.
#[must_use]
pub fn group(heard: &[Heard], people: Option<usize>) -> Vec<Option<usize>> {
    let spoken = |group: &[usize]| group.iter().map(|&i| heard[i].samples).sum::<usize>();
    let all: usize = heard.iter().map(|h| h.samples).sum();
    let mut voices: Vec<Vec<usize>> = groups(heard)
        .into_iter()
        .filter(|g| as_f64(spoken(g)) >= LEAST_SHARE * as_f64(all))
        .take(people.unwrap_or(usize::MAX))
        .collect();
    let mut out = vec![None; heard.len()];
    let alone = |voice: &[usize]| as_f64(all - spoken(voice)) < LEAST_SHARE * as_f64(all);
    match voices.as_slice() {
        [] => return out,
        [voice] if alone(voice) => return out,
        _ => {}
    }
    voices.sort_by_key(|g| g.iter().min().copied());
    for (voice, group) in voices.iter().enumerate() {
        for &i in group {
            out[i] = Some(voice);
        }
    }
    out
}

/// The takes grouped by average linkage on cosine distance until the
/// nearest two groups are further than `APART`, each group the takes it
/// holds, the group with the most samples first; apart.rs reports them,
/// so a measurement says why a voice wasn't named.
#[must_use]
pub fn groups(heard: &[Heard]) -> Vec<Vec<usize>> {
    let unit: Vec<Vec<f32>> = heard.iter().map(|h| unit(&h.embedding)).collect();
    let mut groups: Vec<Vec<usize>> = (0..heard.len()).map(|i| vec![i]).collect();
    let mut apart: Vec<Vec<f32>> = unit
        .iter()
        .map(|a| unit.iter().map(|b| 1.0 - dot(a, b)).collect())
        .collect();
    while let Some((a, b)) = nearest(&apart).filter(|&(a, b)| apart[a][b] <= APART) {
        // Average linkage: the merged group is as far from each other as
        // its takes are on average, each part weighed by its size.
        let (na, nb) = (as_f32(groups[a].len()), as_f32(groups[b].len()));
        let merged: Vec<f32> = apart[a]
            .iter()
            .zip(&apart[b])
            .map(|(from_a, from_b)| (na * from_a + nb * from_b) / (na + nb))
            .collect();
        for (row, &distance) in apart.iter_mut().zip(&merged) {
            row[a] = distance;
        }
        apart[a] = merged;
        apart.remove(b);
        for row in &mut apart {
            row.remove(b);
        }
        let moved = groups.remove(b);
        groups[a].extend(moved);
    }
    groups.sort_by_key(|g| std::cmp::Reverse(g.iter().map(|&i| heard[i].samples).sum::<usize>()));
    groups
}

/// The two nearest groups, the first before the second, or None for fewer
/// than two.
fn nearest(apart: &[Vec<f32>]) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize)> = None;
    for a in 0..apart.len() {
        for b in a + 1..apart.len() {
            if best.is_none_or(|(x, y)| apart[a][b] < apart[x][y]) {
                best = Some((a, b));
            }
        }
    }
    best
}

fn unit(v: &[f32]) -> Vec<f32> {
    let length = dot(v, v).sqrt();
    if length == 0.0 {
        return v.to_vec();
    }
    v.iter().map(|x| x / length).collect()
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn as_f32(n: usize) -> f32 {
    f32::from(u16::try_from(n).unwrap_or(u16::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tenth of a second of 440 Hz and 3 kHz together, which sherpa-onnx's
    /// features, worked out in numpy and checked against its embeddings,
    /// put into eight windows.
    fn two_tones() -> Vec<f32> {
        (0..1600)
            .map(|i| {
                let t = f64::from(i) / 16_000.0;
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "a sample is a 32-bit float"
                )]
                let sample = (0.5 * (2.0 * PI * 440.0 * t).sin()
                    + 0.25 * (2.0 * PI * 3000.0 * t).sin()) as f32;
                sample
            })
            .collect()
    }

    fn near(got: f64, want: f64, what: &str) {
        assert!(
            (got - want).abs() < 1e-4,
            "{what} is {got}, where sherpa-onnx's is {want}"
        );
    }

    #[test]
    fn the_bands_are_sherpa_onnxs() {
        let (frames, logs) = Bands::new().log_bands(&two_tones()).expect("eight windows");
        assert_eq!(
            frames, 8,
            "a tenth of a second is eight 25 ms windows 10 ms apart"
        );
        let at = |band: usize| logs[3 * BANDS + band];
        near(at(0), -15.204_901_136_534_215, "the lowest band");
        near(at(10), -0.198_521_347_838_674, "the band about 440 Hz");
        near(at(55), 2.062_606_308_330_201_5, "the band about 3 kHz");
        near(
            at(79),
            f64::from(f32::EPSILON).ln(),
            "the top band, with nothing in it",
        );
    }

    // Held to what normalising promises rather than to numpy's numbers:
    // sherpa-onnx normalises in 32-bit floats, and a steady tone's bands
    // barely move, so dividing by their spread magnifies its rounding. The
    // tones swell here, so every band moves by more than its padding.
    #[test]
    fn the_features_are_each_band_normalised_and_laid_out_by_band() {
        let swelling: Vec<f32> = two_tones()
            .iter()
            .enumerate()
            .map(|(i, &sample)| {
                sample * (1.0 + 4.0 * f32::from(u16::try_from(i).unwrap_or(0)) / 1600.0)
            })
            .collect();
        let bands = Bands::new();
        let (frames, logs) = bands.log_bands(&swelling).expect("eight windows");
        let (_, features) = bands.features(&swelling).expect("eight windows");
        for band in [0, 10, 55] {
            let row = &features[band * frames..(band + 1) * frames];
            let mean = row.iter().map(|&v| f64::from(v)).sum::<f64>() / as_f64(frames);
            let spread = (row
                .iter()
                .map(|&v| (f64::from(v) - mean).powi(2))
                .sum::<f64>()
                / as_f64(frames))
            .sqrt();
            assert!(mean.abs() < 1e-5, "band {band}'s mean is {mean}, not 0");
            assert!(
                (spread - 1.0).abs() < 1e-3,
                "band {band}'s spread is {spread}, not 1"
            );
            let (first, second) = (logs[band], logs[BANDS + band]);
            assert_eq!(
                f64::from(row[0]) < f64::from(row[1]),
                first < second,
                "band {band}'s first two windows aren't laid out one after the other"
            );
        }
    }

    #[test]
    fn less_than_a_window_has_no_features() {
        assert!(
            Bands::new().features(&[0.0; LENGTH - 1]).is_none(),
            "a take shorter than one window gave features"
        );
    }

    fn heard(embedding: &[f32], seconds: usize) -> Heard {
        Heard {
            embedding: embedding.to_vec(),
            samples: seconds * RATE,
        }
    }

    #[test]
    fn takes_alike_are_one_voice_and_voices_are_numbered_as_they_first_speak() {
        let takes = [
            heard(&[0.0, 1.0, 0.1], 4),
            heard(&[1.0, 0.0, 0.0], 5),
            heard(&[0.9, 0.1, 0.0], 5),
            heard(&[0.1, 1.0, 0.0], 4),
        ];
        assert_eq!(
            group(&takes, None),
            [Some(0), Some(1), Some(1), Some(0)],
            "the two voices weren't told apart, or weren't numbered in the order they spoke"
        );
    }

    #[test]
    fn a_group_with_little_of_the_speech_is_no_voice() {
        let takes = [
            heard(&[1.0, 0.0, 0.0], 10),
            heard(&[0.0, 1.0, 0.0], 10),
            heard(&[0.0, 0.0, 1.0], 1),
        ];
        assert_eq!(
            group(&takes, None),
            [Some(0), Some(1), None],
            "a take with a twenty-first of the speech was named as a voice"
        );
    }

    #[test]
    fn no_more_voices_are_named_than_people_were_on_the_call() {
        let takes = [
            heard(&[1.0, 0.0, 0.0], 3),
            heard(&[0.0, 1.0, 0.0], 10),
            heard(&[0.0, 0.0, 1.0], 10),
        ];
        assert_eq!(
            group(&takes, Some(2)),
            [None, Some(0), Some(1)],
            "with two others on the call, the third and smallest group was named"
        );
    }

    #[test]
    fn one_voice_alone_is_none() {
        let takes = [
            heard(&[1.0, 0.0, 0.0], 10),
            heard(&[0.9, 0.1, 0.0], 10),
            heard(&[0.0, 0.0, 1.0], 1),
        ];
        assert_eq!(
            group(&takes, None),
            [None, None, None],
            "a far end of one voice and a stray take was split off as Voice 1"
        );
    }

    #[test]
    fn one_voice_beside_enough_that_isnt_it_is_named() {
        let takes = [
            heard(&[1.0, 0.0, 0.0, 0.0], 16),
            heard(&[0.0, 1.0, 0.0, 0.0], 1),
            heard(&[0.0, 0.0, 1.0, 0.0], 1),
            heard(&[0.0, 0.0, 0.0, 1.0], 1),
        ];
        assert_eq!(
            group(&takes, None),
            [Some(0), None, None, None],
            "a voice with most of the speech wasn't told from the scattered rest"
        );
    }

    #[test]
    fn nothing_heard_is_no_voices() {
        assert!(group(&[], None).is_empty(), "no takes gave voices");
    }
}
