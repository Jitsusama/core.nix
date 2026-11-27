//! The tests of loudness.rs, its parent: their sound, kept 50 ms at a
//! time, read back as loud as it was, and silence where no take was.

use super::*;

const SECOND: usize = RATE;

/// `samples` of a sine whose power is full scale's, as a word's level is
/// measured, so it reads 0 dB, each sample scaled by `shape` of its
/// share of the way through.
#[expect(clippy::cast_precision_loss, reason = "a test's few seconds of samples")]
fn sine(samples: usize, shape: impl Fn(f32) -> f32) -> Vec<f32> {
    (0..samples)
        .map(|i| {
            let through = i as f32 / samples as f32;
            shape(through) * std::f32::consts::SQRT_2 * (i as f32 * 0.07).sin()
        })
        .collect()
}

/// The lab's reading, from the raw `sound` of their whole side: the
/// loudest quarter second starting from a second before `at` to three
/// tenths after, in 50 ms steps (`loudest_before` in score.rs, on
/// branch jitsusama/lab-echo-sound-apart).
#[expect(clippy::cast_precision_loss, reason = "a quarter second's samples")]
fn from_samples(sound: &[f32], at: usize) -> f32 {
    let mut loudest = -120.0_f32;
    let mut from = at.saturating_sub(SECOND);
    while from <= at + SECOND * 3 / 10 {
        let span = sound
            .get(from..(from + SECOND / 4).min(sound.len()))
            .unwrap_or_default();
        let energy: f32 = span.iter().map(|s| s * s).sum();
        let power = energy / span.len().max(1) as f32;
        loudest = loudest.max(10.0 * (power + 1e-12).log10());
        from += SECOND / 20;
    }
    loudest
}

#[test]
fn a_take_reads_as_loud_as_it_was_just_after_it() {
    let mut track = Track::default();
    track.hear(SECOND, &sine(2 * SECOND, |_| 1.0));
    let heard = track.loudest_before(3 * SECOND + SECOND / 10);
    assert!(
        heard.abs() < 0.1,
        "a take at 0 dB read {heard} dB 100 ms after it ended"
    );
}

#[test]
fn two_seconds_after_a_take_is_silence() {
    let mut track = Track::default();
    track.hear(SECOND, &sine(2 * SECOND, |_| 1.0));
    let heard = track.loudest_before(5 * SECOND);
    assert!(
        (heard + 120.0).abs() < 0.01,
        "2 s after a take read {heard} dB, not silence's -120"
    );
}

#[test]
fn a_gap_between_two_takes_is_silence() {
    let mut track = Track::default();
    track.hear(0, &sine(SECOND, |_| 1.0));
    track.hear(4 * SECOND, &sine(SECOND, |_| 1.0));
    let heard = track.loudest_before(5 * SECOND / 2);
    assert!(
        (heard + 120.0).abs() < 0.01,
        "the gap between two takes read {heard} dB, not silence's -120"
    );
}

#[test]
fn the_track_reads_what_the_lab_read_from_the_sound_itself() {
    // A take rising from silence, starting between two steps, inside a
    // side long enough that no quarter second runs past its end.
    let start = 1_234;
    let take = sine(3 * SECOND, |through| through);
    let mut sound = vec![0.0; 7 * SECOND];
    sound[start..start + take.len()].copy_from_slice(&take);
    let mut track = Track::default();
    track.hear(start, &take);
    for tenths in [5, 20, 35, 42] {
        let at = tenths * SECOND / 10;
        let (read, lab) = (track.loudest_before(at), from_samples(&sound, at));
        assert!(
            (read - lab).abs() < 0.01,
            "at sample {at} the track read {read} dB where the sound itself reads {lab}"
        );
    }
}
