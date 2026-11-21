//! Telling a call's far-end voices apart: TitaNet small (titanet.nix), run
//! through OpenVINO on the CPU, gives each 3 s piece of a take
//! (pieces.rs) one embedding as it's heard, and at the call's end the
//! pieces are grouped by how alike their embeddings are; each is a
//! `Heard`, called a take below. No sound is kept for it, only 192
//! numbers a piece.
//!
//! A take is the cutter's (takes.rs), cut where the far end pauses, so
//! now and then one holds two voices, and a group may be a mix; grouping
//! takes by average linkage up to `APART` and naming only groups that hold
//! at least `LEAST_SHARE` of the far end's speech left those mixes unnamed.
//! The voices check (apart.rs) holds what that reaches on three AMI
//! meetings' far ends, cut and coded as a call's are.
//!
//! The features are the ones sherpa-onnx feeds this export of TitaNet:
//! 80 log mel bands on Slaney's scale from 0 to 7600 Hz, over 25 ms
//! periodic Hann windows every 10 ms, each pre-emphasised and padded to
//! 512 points, and each band then normalised across the take. Worked out
//! so, TitaNet's embeddings were sherpa-onnx's to five places. NeMo's own
//! front end, which it was trained with, differs a little: its bands reach
//! 8000 Hz, a guard is added before the log rather than floored, and a
//! band's spread divides by one fewer. The check measures what this one
//! gives, so trying NeMo's is a change the check can judge.

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
/// The most takes a call groups, its longest: grouping holds every take's
/// distance from every other, 36 MB at this many, and a far end with
/// chatter on it can cut a take a second for hours. The shortest are left
/// out, saying least of how a voice sounds, and of the same length the
/// latest. Cut in 3 s pieces, about 2.5 hours of their speech fills it,
/// and past that a take whose pieces are all left out stays under Others.
pub const MOST_TAKES: usize = 3000;

/// A window's samples, 25 ms, and how far each is from the last, 10 ms.
const LENGTH: usize = 400;
const SHIFT: usize = 160;
/// The transform's points, the window padded, and the bins it gives.
const POINTS: usize = 512;
const BINS: usize = POINTS / 2 + 1;
/// TitaNet's bands, and the highest frequency they reach: 400 Hz under
/// half the rate, as sherpa-onnx has it for this export.
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
    /// since a take's embedding is small and comes while Parakeet hears,
    /// and kept compiled in `cache`, where there's one, so a call's first
    /// take needn't wait for it.
    ///
    /// # Errors
    ///
    /// When OpenVINO can't read or compile it.
    pub fn load(model: &Path, cache: Option<&Path>) -> Result<Self> {
        let core = Core::new(cache)?;
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
        // A number that isn't one would make every distance to the take
        // none, so the take is left unembedded, under Others.
        Ok(embedding.iter().all(|x| x.is_finite()).then_some(embedding))
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

/// A take's embedding, how many samples it holds, and whether words were
/// heard in it, so it has a line.
pub struct Heard {
    pub embedding: Vec<f32>,
    pub samples: usize,
    pub worded: bool,
}

/// Which voice each take is, in the order the voices first spoke, or None
/// for a take that's no voice for sure. Takes are grouped by average
/// linkage on cosine distance until the nearest two groups are further
/// than `APART`; a group is a voice when it holds at least `LEAST_SHARE`
/// of the speech words were heard in, and when `people` says how many
/// others were on the call, only that many of those holding the most
/// are. A take with no words helps group the rest, but counts for nothing
/// in naming them: a group of those alone, such as a far end's hold music
/// or a room's chatter, takes no voice from a person, and a voice is
/// numbered from its first take with words, so the transcript opens on
/// Voice 1. One voice is named only when at least `LEAST_SHARE` of the
/// speech is left outside it, so there's someone else to tell it apart
/// from; a far end of one person whose odd takes fall outside its voice
/// stays Others. `heard` is in the order it was said.
#[must_use]
pub fn group(heard: &[Heard], people: Option<usize>) -> Vec<Option<usize>> {
    let said = |group: &[usize]| {
        group
            .iter()
            .filter(|&&i| heard[i].worded)
            .map(|&i| heard[i].samples)
            .sum::<usize>()
    };
    let all: usize = heard.iter().filter(|h| h.worded).map(|h| h.samples).sum();
    let mut out = vec![None; heard.len()];
    if all == 0 {
        return out;
    }
    let mut voices: Vec<Vec<usize>> = groups(heard)
        .into_iter()
        .filter(|g| as_f64(said(g)) >= LEAST_SHARE * as_f64(all))
        .collect();
    voices.sort_by_key(|g| std::cmp::Reverse(said(g)));
    voices.truncate(people.unwrap_or(usize::MAX));
    let alone = |voice: &[usize]| as_f64(all - said(voice)) < LEAST_SHARE * as_f64(all);
    match voices.as_slice() {
        [] => return out,
        [voice] if alone(voice) => return out,
        _ => {}
    }
    voices.sort_by_key(|g| g.iter().copied().filter(|&i| heard[i].worded).min());
    for (voice, group) in voices.iter().enumerate() {
        for &i in group {
            out[i] = Some(voice);
        }
    }
    out
}

/// The takes grouped by average linkage on cosine distance until the
/// nearest two groups are further than `APART`, each group the takes it
/// holds in the order said, the group with the most samples first; only
/// the `MOST_TAKES` longest are grouped. apart.rs reports them, so a
/// measurement says why a voice wasn't named.
#[must_use]
pub fn groups(heard: &[Heard]) -> Vec<Vec<usize>> {
    let kept = longest(heard);
    let unit: Vec<Vec<f32>> = kept.iter().map(|&i| unit(&heard[i].embedding)).collect();
    let mut apart: Vec<Vec<f32>> = unit
        .iter()
        .map(|a| unit.iter().map(|b| 1.0 - dot(a, b)).collect())
        .collect();
    let mut groups: Vec<Vec<usize>> = kept.iter().map(|&i| vec![i]).collect();
    let mut open = vec![true; kept.len()];
    // Each group in the chain is the nearest to the one below it, and two
    // that are each other's nearest are joined. Average linkage never
    // brings a joined group nearer anything than the nearer of its parts
    // was, so this joins what joining the nearest pair of all each time
    // would, in about the square of the takes rather than the cube: it
    // runs as a call ends, on the thread that hears the next one.
    let mut chain: Vec<usize> = Vec::new();
    loop {
        let Some(&top) = chain.last() else {
            let Some(first) = open.iter().position(|&o| o) else {
                break;
            };
            chain.push(first);
            continue;
        };
        let below = chain.len().checked_sub(2).map(|at| chain[at]);
        match nearest(top, below, &apart, &open) {
            Some((next, distance)) if distance <= APART => {
                if Some(next) == below {
                    chain.truncate(chain.len() - 2);
                    join(
                        top.min(next),
                        top.max(next),
                        &mut apart,
                        &mut groups,
                        &mut open,
                    );
                } else {
                    chain.push(next);
                }
            }
            // Nothing is within APART of the top, so nothing is of any
            // group in the chain, each as near the one above it as
            // anything; and since joining brings no group nearer, none of
            // them will ever join another.
            _ => {
                for done in chain.drain(..) {
                    open[done] = false;
                }
            }
        }
    }
    let mut groups: Vec<Vec<usize>> = groups.into_iter().filter(|g| !g.is_empty()).collect();
    for group in &mut groups {
        group.sort_unstable();
    }
    groups.sort_by_key(|g| std::cmp::Reverse(g.iter().map(|&i| heard[i].samples).sum::<usize>()));
    groups
}

/// The takes grouped, in the order said: every one, or the `MOST_TAKES`
/// longest.
fn longest(heard: &[Heard]) -> Vec<usize> {
    let mut kept: Vec<usize> = (0..heard.len()).collect();
    if kept.len() > MOST_TAKES {
        kept.sort_by_key(|&i| std::cmp::Reverse(heard[i].samples));
        kept.truncate(MOST_TAKES);
        kept.sort_unstable();
    }
    kept
}

/// The open group nearest group `a`, and how far: `below`, the group that
/// led to it, where that's as near as any, so two groups each other's
/// nearest are found to be, else the first of any tied.
fn nearest(
    a: usize,
    below: Option<usize>,
    apart: &[Vec<f32>],
    open: &[bool],
) -> Option<(usize, f32)> {
    let best = apart[a]
        .iter()
        .enumerate()
        .filter(|&(b, _)| b != a && open[b])
        .min_by(|x, y| x.1.total_cmp(y.1))
        .map(|(b, &d)| (b, d))?;
    Some(match below {
        Some(b) if apart[a][b] <= best.1 => (b, apart[a][b]),
        _ => best,
    })
}

/// Joins group `b` into `a`, the joined group as far from each other as
/// its takes are on average, each part weighed by its size.
fn join(a: usize, b: usize, apart: &mut [Vec<f32>], groups: &mut [Vec<usize>], open: &mut [bool]) {
    let (na, nb) = (as_f32(groups[a].len()), as_f32(groups[b].len()));
    let joined: Vec<f32> = apart[a]
        .iter()
        .zip(&apart[b])
        .map(|(from_a, from_b)| (na * from_a + nb * from_b) / (na + nb))
        .collect();
    for (row, &distance) in apart.iter_mut().zip(&joined) {
        row[a] = distance;
    }
    apart[a] = joined;
    open[b] = false;
    let moved = std::mem::take(&mut groups[b]);
    groups[a].extend(moved);
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
            worded: true,
        }
    }

    fn wordless(embedding: &[f32], seconds: usize) -> Heard {
        Heard {
            worded: false,
            ..heard(embedding, seconds)
        }
    }

    #[test]
    fn a_voice_is_numbered_from_its_first_take_with_words() {
        let takes = [
            wordless(&[1.0, 0.0], 2),
            heard(&[0.0, 1.0], 5),
            heard(&[1.0, 0.0], 5),
        ];
        assert_eq!(
            group(&takes, None),
            [Some(1), Some(0), Some(1)],
            "a voice whose first take had no words was numbered from it, so the transcript \
             would open on Voice 2"
        );
    }

    #[test]
    fn takes_with_no_words_are_no_voice_and_take_no_ones_place() {
        let takes = [
            wordless(&[1.0, 0.0, 0.0], 20),
            heard(&[0.0, 1.0, 0.0], 5),
            heard(&[0.0, 0.0, 1.0], 3),
        ];
        assert_eq!(
            group(&takes, Some(2)),
            [None, Some(0), Some(1)],
            "a group of takes with no words, such as hold music, took a person's voice"
        );
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

    /// A take whose embedding is `degrees` round from the first axis, so
    /// two takes' cosine distance is one less the cosine between them.
    fn at(degrees: f64) -> Heard {
        let radians = degrees.to_radians();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a unit vector's parts fit a float"
        )]
        heard(&[radians.cos() as f32, radians.sin() as f32], 1)
    }

    #[test]
    fn two_takes_are_one_voice_only_within_apart() {
        // 1 - cos 63.26 degrees is 0.55, and 1 - cos 69.51 is 0.65.
        assert_eq!(
            groups(&[at(0.0), at(63.26)]),
            [vec![0, 1]],
            "two takes 0.55 apart weren't one group"
        );
        assert_eq!(
            groups(&[at(0.0), at(69.51)]).len(),
            2,
            "two takes 0.65 apart were made one group"
        );
    }

    #[test]
    fn a_group_is_as_far_from_a_take_as_its_takes_are_on_average() {
        // A chain 0.5 and 0.58 apart, its ends 1.57 apart: the nearest, as
        // single linkage takes it, would join all three; on average the
        // pair is 1.07 from the third.
        let chain = groups(&[at(0.0), at(60.0), at(125.0)]);
        assert_eq!(
            chain,
            [vec![0, 1], vec![2]],
            "a chain of takes was joined end to end"
        );
        // The same chain said the other way round, so the pair joined is
        // the last two, and the first take's distance to it is the one
        // the join worked out.
        let backwards = groups(&[at(125.0), at(60.0), at(0.0)]);
        assert_eq!(
            backwards,
            [vec![1, 2], vec![0]],
            "a chain said backwards was joined end to end"
        );
        // A pair 0.7 and 0.4 from a third, so 0.55 on average: the
        // furthest, as complete linkage takes it, would leave it out.
        let pair = groups(&[at(0.0), at(19.41), at(72.54)]);
        assert_eq!(
            pair,
            [vec![0, 1, 2]],
            "a take near a group on average was left out of it"
        );
    }

    /// Numbers from -1 to 1 that are the same every run, from a seed.
    struct Seeded(u64);

    impl Seeded {
        fn next(&mut self) -> f32 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            let top = u16::try_from(self.0 >> 48).expect("the top 16 bits fit in a u16");
            f32::from(top) / 32_768.0 - 1.0
        }
    }

    /// Average linkage the slow way: each time, the pair of groups whose
    /// takes are nearest on average, worked out afresh, joined while
    /// they're within `APART`.
    fn by_brute_force(heard: &[Heard]) -> Vec<Vec<usize>> {
        let unit: Vec<Vec<f32>> = heard.iter().map(|h| unit(&h.embedding)).collect();
        let between = |a: &[usize], b: &[usize]| {
            let sum: f64 = a
                .iter()
                .flat_map(|&i| b.iter().map(move |&j| (i, j)))
                .map(|(i, j)| f64::from(1.0 - dot(&unit[i], &unit[j])))
                .sum();
            sum / (as_f64(a.len()) * as_f64(b.len()))
        };
        let mut groups: Vec<Vec<usize>> = (0..heard.len()).map(|i| vec![i]).collect();
        loop {
            let mut best: Option<(f64, usize, usize)> = None;
            for (a, first) in groups.iter().enumerate() {
                for (b, second) in groups.iter().enumerate().skip(a + 1) {
                    let distance = between(first, second);
                    if best.is_none_or(|(nearest, ..)| distance < nearest) {
                        best = Some((distance, a, b));
                    }
                }
            }
            match best {
                Some((distance, a, b)) if distance <= f64::from(APART) => {
                    let moved = groups.remove(b);
                    groups[a].extend(moved);
                }
                _ => break,
            }
        }
        groups
    }

    fn sorted(mut groups: Vec<Vec<usize>>) -> Vec<Vec<usize>> {
        for group in &mut groups {
            group.sort_unstable();
        }
        groups.sort();
        groups
    }

    #[test]
    fn grouping_joins_what_joining_the_nearest_pair_each_time_does() {
        for seed in 1..=6 {
            let mut seeded = Seeded(seed);
            let centres: Vec<Vec<f32>> = (0..5)
                .map(|_| (0..6).map(|_| seeded.next()).collect())
                .collect();
            let takes: Vec<Heard> = (0..60)
                .map(|take| {
                    let centre = &centres[take % centres.len()];
                    let embedding: Vec<f32> =
                        centre.iter().map(|x| x + 0.5 * seeded.next()).collect();
                    heard(&embedding, 1)
                })
                .collect();
            let slow = sorted(by_brute_force(&takes));
            assert!(
                slow.len() > 1 && slow.len() < takes.len(),
                "seed {seed} made {} groups of 60 takes, which tests no joining",
                slow.len()
            );
            assert_eq!(
                sorted(groups(&takes)),
                slow,
                "seed {seed}'s takes weren't grouped as joining the nearest pair each time \
                 groups them"
            );
        }
    }

    #[test]
    fn past_the_most_takes_only_the_longest_are_grouped() {
        let mut takes = vec![Heard {
            samples: 1,
            ..heard(&[1.0, 0.0], 0)
        }];
        takes.extend((0..MOST_TAKES).map(|_| Heard {
            samples: 2,
            ..heard(&[1.0, 0.0], 0)
        }));
        let grouped = groups(&takes);
        let held: Vec<usize> = grouped.iter().flatten().copied().collect();
        assert_eq!(
            held.len(),
            MOST_TAKES,
            "more takes than the most were grouped"
        );
        assert!(
            !held.contains(&0),
            "the shortest take was grouped in place of a longer one"
        );
    }

    #[test]
    fn nothing_heard_is_no_voices() {
        assert!(group(&[], None).is_empty(), "no takes gave voices");
    }
}
