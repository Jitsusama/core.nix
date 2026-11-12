use super::*;

/// A tone of `hertz` at 48 kHz, `seconds` long, at `level` of full scale.
fn tone(hertz: f32, seconds: f32, level: f32) -> Vec<f32> {
    let rate = 48_000.0_f32;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a few seconds of samples, always positive"
    )]
    let count = (seconds * rate) as usize;
    (0..count)
        .map(|index| {
            #[expect(clippy::cast_precision_loss, reason = "far under 2^24 samples")]
            let at = index as f32 / rate;
            level * (std::f32::consts::TAU * hertz * at).sin()
        })
        .collect()
}

/// The root mean square of `samples`, past their first and last 100.
fn level(samples: &[f32]) -> f64 {
    let inside = &samples[100..samples.len() - 100];
    let sum: f64 = inside.iter().map(|sample| f64::from(*sample).powi(2)).sum();
    #[expect(clippy::cast_precision_loss, reason = "far under 2^52 samples")]
    let count = inside.len() as f64;
    (sum / count).sqrt()
}

#[test]
fn a_voice_band_tone_comes_down_to_16_khz_as_loud() {
    let heard = tone(1_000.0, 1.0, 0.5);
    let down = decimated(&heard);
    assert_eq!(down.len(), heard.len() / 3, "every third sample is kept");
    let ratio = level(&down) / level(&heard);
    assert!(
        (ratio - 1.0).abs() < 0.01,
        "a 1 kHz tone came out at {ratio} of its level"
    );
}

#[test]
fn what_16_khz_cant_hold_is_taken_out_first() {
    let heard = tone(12_000.0, 1.0, 0.5);
    let down = decimated(&heard);
    let ratio = level(&down) / level(&heard);
    assert!(
        ratio < 0.01,
        "a 12 kHz tone came back folded at {ratio} of its level"
    );
}

#[test]
fn a_six_second_clip_is_repeated_and_heard_three_times() {
    let clip: Vec<f32> = (0..6 * HEARD)
        .map(|index| f32::from(u16::try_from(index % 1000).unwrap_or(0)))
        .collect();
    let heard = windows(&clip);
    assert_eq!(
        heard.len(),
        3,
        "six seconds, repeated to twelve, are three windows"
    );
    assert!(
        heard.iter().all(|window| window.len() == WINDOW),
        "every window is 9.01 s"
    );
    assert_eq!(
        heard[1][..10],
        clip[HEARD..HEARD + 10],
        "each starts a second on"
    );
    assert_eq!(
        heard[0][6 * HEARD..6 * HEARD + 10],
        clip[..10],
        "the clip goes on with itself"
    );
}

#[test]
fn a_long_clip_is_heard_a_second_apart_without_repeating() {
    let clip = vec![0.25; 20 * HEARD];
    assert_eq!(
        windows(&clip).len(),
        11,
        "twenty seconds are eleven windows"
    );
    let exactly = vec![0.25; WINDOW];
    assert_eq!(windows(&exactly).len(), 1, "9.01 s is one window");
    assert!(windows(&[]).is_empty(), "nothing is no window");
}

#[test]
fn marks_are_fitted_to_what_people_say() {
    let marks = fitted([3.0, 3.0, 3.0]);
    let near = |got: f64, wanted: f64| (got - wanted).abs() < 1e-5;
    assert!(near(marks.voice, 2.912_007), "voice: {}", marks.voice);
    assert!(
        near(marks.background, 3.246_400),
        "background: {}",
        marks.background
    );
    assert!(near(marks.overall, 2.783_454), "overall: {}", marks.overall);
}
