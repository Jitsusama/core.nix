//! Whisper's log-mel spectrogram: 30 s of 16 kHz audio as 80 mel bands by
//! 3000 frames, the encoder's input, worked out as Whisper's own feature
//! extractor does it.

/// Samples a second.
pub const RATE: usize = 16_000;
/// Samples in Whisper's window, 30 s.
pub const SAMPLES: usize = 30 * RATE;
/// Mel bands.
pub const MELS: usize = 80;
/// Frames in Whisper's window, one every hop.
pub const FRAMES: usize = SAMPLES / HOP;

const N_FFT: usize = 400;
const HOP: usize = 160;
const BINS: usize = N_FFT / 2 + 1;

type Complex = (f64, f64);

/// What every spectrogram is worked out with, made once.
pub struct Spectrogram {
    window: Vec<f64>,
    filters: Vec<f64>,
    twiddles: Vec<Complex>,
}

impl Default for Spectrogram {
    fn default() -> Self {
        Self::new()
    }
}

impl Spectrogram {
    #[must_use]
    pub fn new() -> Self {
        let window = (0..N_FFT)
            .map(|i| 0.5 - 0.5 * (2.0 * std::f64::consts::PI * as_f64(i) / as_f64(N_FFT)).cos())
            .collect();
        let twiddles = (0..N_FFT)
            .map(|j| {
                let angle = -2.0 * std::f64::consts::PI * as_f64(j) / as_f64(N_FFT);
                (angle.cos(), angle.sin())
            })
            .collect();
        Self {
            window,
            filters: filters(),
            twiddles,
        }
    }

    /// The encoder's input for up to 30 s of audio: MELS rows of FRAMES,
    /// cut or padded with silence to Whisper's window.
    #[must_use]
    pub fn features(&self, audio: &[f32]) -> Vec<f32> {
        let mut samples: Vec<f64> = audio.iter().take(SAMPLES).map(|&s| f64::from(s)).collect();
        samples.resize(SAMPLES, 0.0);
        let padded = reflect(&samples, N_FFT / 2);

        let mut mel = vec![0.0; MELS * FRAMES];
        let mut frame = vec![(0.0, 0.0); N_FFT];
        let mut spectrum = vec![(0.0, 0.0); N_FFT];
        let mut power = [0.0; BINS];
        for t in 0..FRAMES {
            let start = t * HOP;
            for (slot, (sample, weight)) in frame
                .iter_mut()
                .zip(padded[start..start + N_FFT].iter().zip(&self.window))
            {
                *slot = (sample * weight, 0.0);
            }
            transform(&frame, 0, 1, N_FFT, &mut spectrum, &self.twiddles, 1);
            for (p, &(re, im)) in power.iter_mut().zip(&spectrum) {
                *p = re * re + im * im;
            }
            for m in 0..MELS {
                let row = &self.filters[m * BINS..(m + 1) * BINS];
                mel[m * FRAMES + t] = row.iter().zip(&power).map(|(w, p)| w * p).sum();
            }
        }

        let logs: Vec<f64> = mel.iter().map(|&m| m.max(1e-10).log10()).collect();
        let floor = logs.iter().copied().fold(f64::NEG_INFINITY, f64::max) - 8.0;
        #[allow(clippy::cast_possible_truncation)]
        logs.iter()
            .map(|&l| ((l.max(floor) + 4.0) / 4.0) as f32)
            .collect()
    }
}

fn as_f64(n: usize) -> f64 {
    f64::from(u32::try_from(n).expect("a count Whisper uses fits in 32 bits"))
}

/// The samples with `edge` mirrored onto each end, the end sample itself
/// left out, as a centred short-time transform pads them.
fn reflect(samples: &[f64], edge: usize) -> Vec<f64> {
    let n = samples.len();
    let mut out = Vec::with_capacity(n + 2 * edge);
    out.extend((1..=edge).rev().map(|i| samples[i]));
    out.extend_from_slice(samples);
    out.extend((0..edge).map(|i| samples[n - 2 - i]));
    out
}

// Slaney's mel scale: linear to 1 kHz, logarithmic above.
fn hz_to_mel(hz: f64) -> f64 {
    if hz >= 1000.0 {
        15.0 + (hz / 1000.0).ln() / (6.4_f64.ln() / 27.0)
    } else {
        3.0 * hz / 200.0
    }
}

fn mel_to_hz(mel: f64) -> f64 {
    if mel >= 15.0 {
        1000.0 * ((6.4_f64.ln() / 27.0) * (mel - 15.0)).exp()
    } else {
        200.0 * mel / 3.0
    }
}

/// MELS triangles over the BINS frequencies from 0 to 8 kHz, each scaled
/// to the width it spans, as Whisper's filters are.
fn filters() -> Vec<f64> {
    let top = hz_to_mel(as_f64(RATE) / 2.0);
    let edges: Vec<f64> = (0..MELS + 2)
        .map(|i| mel_to_hz(top * as_f64(i) / as_f64(MELS + 1)))
        .collect();
    let mut weights = vec![0.0; MELS * BINS];
    for m in 0..MELS {
        let scale = 2.0 / (edges[m + 2] - edges[m]);
        for k in 0..BINS {
            let hz = as_f64(k) * as_f64(RATE) / as_f64(N_FFT);
            let rising = (hz - edges[m]) / (edges[m + 1] - edges[m]);
            let falling = (edges[m + 2] - hz) / (edges[m + 2] - edges[m + 1]);
            weights[m * BINS + k] = rising.min(falling).max(0.0) * scale;
        }
    }
    weights
}

/// The discrete Fourier transform of `n` points of `input`, starting at
/// `start` and `stride` apart, into `out`, split by its smallest factor at
/// each step. `twiddles` holds the whole transform's roots, of which this
/// size takes every `step`th.
fn transform(
    input: &[Complex],
    start: usize,
    stride: usize,
    n: usize,
    out: &mut [Complex],
    twiddles: &[Complex],
    step: usize,
) {
    if n == 1 {
        out[0] = input[start];
        return;
    }
    let p = [2, 3, 5]
        .into_iter()
        .find(|&p| n.is_multiple_of(p))
        .unwrap_or(n);
    assert!(p <= 5, "the transform takes sizes of 2, 3 and 5 only");
    let m = n / p;
    for r in 0..p {
        transform(
            input,
            start + r * stride,
            stride * p,
            m,
            &mut out[r * m..(r + 1) * m],
            twiddles,
            step * p,
        );
    }
    let mut parts = [(0.0, 0.0); 5];
    for k in 0..m {
        for (r, part) in parts.iter_mut().enumerate().take(p) {
            *part = out[r * m + k];
        }
        for q in 0..p {
            let mut sum = (0.0, 0.0);
            for (r, &(re, im)) in parts.iter().enumerate().take(p) {
                let (wr, wi) = twiddles[(r * (k + q * m) % n) * step];
                sum.0 += re * wr - im * wi;
                sum.1 += re * wi + im * wr;
            }
            out[q * m + k] = sum;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dft(input: &[Complex]) -> Vec<Complex> {
        let n = input.len();
        (0..n)
            .map(|k| {
                input
                    .iter()
                    .enumerate()
                    .fold((0.0, 0.0), |(sr, si), (j, &(re, im))| {
                        let angle = -2.0 * std::f64::consts::PI * as_f64(j * k % n) / as_f64(n);
                        let (wr, wi) = (angle.cos(), angle.sin());
                        (sr + re * wr - im * wi, si + re * wi + im * wr)
                    })
            })
            .collect()
    }

    #[test]
    fn the_transform_matches_the_plain_sum() {
        let input: Vec<Complex> = (0..N_FFT)
            .map(|i| ((as_f64(i) * 0.37).sin(), (as_f64(i) * 0.11).cos()))
            .collect();
        let spectrogram = Spectrogram::new();
        let mut fast = vec![(0.0, 0.0); N_FFT];
        transform(&input, 0, 1, N_FFT, &mut fast, &spectrogram.twiddles, 1);
        for (a, b) in fast.iter().zip(dft(&input)) {
            assert!(
                (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9,
                "{a:?} against {b:?}"
            );
        }
    }

    #[test]
    fn reflecting_leaves_out_the_end_sample() {
        assert_eq!(
            reflect(&[1.0, 2.0, 3.0, 4.0], 2),
            [3.0, 2.0, 1.0, 2.0, 3.0, 4.0, 3.0, 2.0]
        );
    }

    #[test]
    fn the_scale_turns_at_a_kilohertz() {
        assert!((hz_to_mel(1000.0) - 15.0).abs() < 1e-12);
        for hz in [0.0, 440.0, 1000.0, 3000.0, 8000.0] {
            assert!((mel_to_hz(hz_to_mel(hz)) - hz).abs() < 1e-6);
        }
    }

    // Values from the numpy extractor whose features transcribed a clip
    // word for word, for a second of 440 Hz at half scale.
    #[test]
    fn a_tone_matches_the_reference() {
        let tone: Vec<f32> = (0..RATE)
            .map(|i| {
                #[allow(clippy::cast_possible_truncation)]
                let s = (0.5
                    * (2.0 * std::f64::consts::PI * 440.0 * as_f64(i) / as_f64(RATE)).sin())
                    as f32;
                s
            })
            .collect();
        let features = Spectrogram::new().features(&tone);
        assert_eq!(features.len(), MELS * FRAMES);
        for &(m, t, want) in REFERENCE {
            let got = features[m * FRAMES + t];
            assert!(
                (got - want).abs() < 1e-4,
                "band {m} frame {t}: {got} against {want}"
            );
        }
    }

    const REFERENCE: &[(usize, usize, f32)] = &[
        (0, 0, 0.983_278_8),
        (3, 0, 1.007_814_9),
        (5, 10, -0.561_796_2),
        (8, 50, -0.561_796_2),
        (9, 50, 1.159_391_2),
        (10, 99, 1.349_457_9),
        (8, 100, 1.061_444_4),
        (40, 120, -0.561_796_2),
        (79, 2999, -0.561_796_2),
    ];
}
