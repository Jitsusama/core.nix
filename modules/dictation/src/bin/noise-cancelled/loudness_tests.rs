//! The loudness tests: measured as the standard's own figures say, and
//! every clip of a trial matched to one level.

use super::*;
use crate::denoise::RATE;

/// A second's samples.
fn second() -> usize {
    usize::try_from(RATE).expect("a rate within usize")
}

/// `seconds` of a tone at `hertz`, its peak at `amplitude`.
#[expect(clippy::cast_possible_truncation, reason = "a sample is within an f32")]
fn tone(hertz: f64, amplitude: f64, seconds: usize) -> Vec<f32> {
    (0..seconds * second())
        .map(|index| {
            let time =
                f64::from(u32::try_from(index).expect("a test's few seconds")) / f64::from(RATE);
            (amplitude * (std::f64::consts::TAU * hertz * time).sin()) as f32
        })
        .collect()
}

fn loudest(samples: &[f32]) -> f32 {
    samples
        .iter()
        .fold(0.0, |most, sample| most.max(sample.abs()))
}

#[test]
fn a_full_scale_tone_measures_as_the_standard_says() {
    // ITU-R BS.1770-4: a 0 dBFS sine at 1 kHz in one channel reads -3.01.
    let measured = integrated(&tone(1000.0, 1.0, 5)).expect("a loudness");
    assert!(
        (measured + 3.01).abs() < 0.1,
        "measured {measured} LUFS, not -3.01"
    );
}

#[test]
fn a_quiet_clip_and_a_loud_one_are_played_equally_loud() {
    let mut quiet = tone(440.0, 0.01, 6);
    let mut loud = tone(1000.0, 0.3, 6);
    matched(&mut [&mut quiet, &mut loud]);
    for clip in [&quiet, &loud] {
        let measured = integrated(clip).expect("a loudness");
        assert!(
            (measured - TARGET).abs() < 0.1,
            "a clip played at {measured} LUFS, not {TARGET}"
        );
    }
}

#[test]
fn a_clip_is_never_raised_past_its_peak() {
    let mut steady = tone(1000.0, 0.01, 6);
    let mut spiky = tone(1000.0, 0.01, 6);
    spiky[second()] = 0.5;
    matched(&mut [&mut steady, &mut spiky]);
    assert!(
        loudest(&spiky) <= PEAK,
        "a clip reached {}",
        loudest(&spiky)
    );
    let (steady, spiky) = (integrated(&steady).unwrap(), integrated(&spiky).unwrap());
    assert!(
        (steady - spiky).abs() < 0.1,
        "one clip played at {steady} LUFS and the other at {spiky}"
    );
    assert!(
        steady < TARGET,
        "the clips were raised to {steady} past the spike's headroom"
    );
}

#[test]
fn silence_is_left_as_it_is() {
    let mut silence = vec![0.0; second() * 6];
    let mut voice = tone(1000.0, 0.05, 6);
    matched(&mut [&mut silence, &mut voice]);
    assert!(
        integrated(&silence).is_none(),
        "silence was measured as a voice"
    );
    assert!(
        silence.iter().all(|sample| sample.to_bits() == 0),
        "silence was raised"
    );
    let measured = integrated(&voice).expect("a loudness");
    assert!(
        (measured - TARGET).abs() < 0.1,
        "the voice played at {measured} LUFS"
    );
}
