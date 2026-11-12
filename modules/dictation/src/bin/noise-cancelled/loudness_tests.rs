//! The loudness tests: measured as the standard's own figures say, and
//! every clip of a trial matched to one level.

use super::*;
use crate::denoise::RATE;

/// `seconds` of a tone at `hertz`, its peak at `amplitude`.
fn tone(hertz: f32, amplitude: f32, seconds: f32) -> Vec<f32> {
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a test's few seconds of samples"
    )]
    let count = (seconds * RATE as f32) as usize;
    (0..count)
        .map(|index| {
            #[expect(clippy::cast_precision_loss, reason = "a test's few seconds of samples")]
            let time = index as f32 / RATE as f32;
            amplitude * (std::f32::consts::TAU * hertz * time).sin()
        })
        .collect()
}

fn loudest(samples: &[f32]) -> f32 {
    samples.iter().fold(0.0, |most, sample| most.max(sample.abs()))
}

#[test]
fn a_full_scale_tone_measures_as_the_standard_says() {
    // ITU-R BS.1770-4: a 0 dBFS sine at 1 kHz in one channel reads -3.01.
    let measured = integrated(&tone(1000.0, 1.0, 5.0)).expect("a loudness");
    assert!((measured + 3.01).abs() < 0.1, "measured {measured} LUFS, not -3.01");
}

#[test]
fn a_quiet_clip_and_a_loud_one_are_played_equally_loud() {
    let mut quiet = tone(440.0, 0.01, 6.0);
    let mut loud = tone(1000.0, 0.3, 6.0);
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
    let mut steady = tone(1000.0, 0.01, 6.0);
    let mut spiky = tone(1000.0, 0.01, 6.0);
    spiky[RATE as usize] = 0.5;
    matched(&mut [&mut steady, &mut spiky]);
    assert!(loudest(&spiky) <= PEAK, "a clip reached {}", loudest(&spiky));
    let (steady, spiky) = (integrated(&steady).unwrap(), integrated(&spiky).unwrap());
    assert!(
        (steady - spiky).abs() < 0.1,
        "one clip played at {steady} LUFS and the other at {spiky}"
    );
    assert!(steady < TARGET, "the clips were raised to {steady} past the spike's headroom");
}

#[test]
fn silence_is_left_as_it_is() {
    let mut silence = vec![0.0; RATE as usize * 6];
    let mut voice = tone(1000.0, 0.05, 6.0);
    matched(&mut [&mut silence, &mut voice]);
    assert_eq!(integrated(&silence), None);
    assert!(silence.iter().all(|sample| *sample == 0.0), "silence was raised");
    let measured = integrated(&voice).expect("a loudness");
    assert!((measured - TARGET).abs() < 0.1, "the voice played at {measured} LUFS");
}
