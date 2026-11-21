use super::*;

/// `seconds` of a sine at `level`, stepping `step` radians a sample.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "a test's few seconds of samples"
)]
fn tone(seconds: f32, level: f32, step: f32) -> Vec<f32> {
    let count = (seconds * RATE as f32) as usize;
    (0..count)
        .map(|i| level * (i as f32 * step).sin())
        .collect()
}

/// Speech stood in for by syllables of a tone at about -20 dBFS, each
/// 200 ms, with 50 ms near the room's quiet between them, as a stop
/// or a breath falls.
fn spoken(seconds: f32) -> Vec<f32> {
    let syllable = [tone(0.2, 0.14, 0.07), quiet(0.05)].concat();
    syllable
        .iter()
        .copied()
        .cycle()
        .take(quiet(seconds).len())
        .collect()
}

/// `seconds` of noise at about `level`, white or, `brown`, each sample
/// a step from the last, as a breath or a rumble is.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "a test's few seconds of samples"
)]
fn noise(seconds: f32, level: f32, brown: bool) -> Vec<f32> {
    let count = (seconds * RATE as f32) as usize;
    let mut seed: u32 = 1;
    let mut walk = 0.0;
    let raw: Vec<f32> = (0..count)
        .map(|_| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let white = (seed >> 8) as f32 / (1 << 24) as f32 * 2.0 - 1.0;
            walk += white;
            if brown { walk } else { white }
        })
        .collect();
    let rms = (raw.iter().map(|s| s * s).sum::<f32>() / count as f32).sqrt();
    raw.iter().map(|s| s * level / rms).collect()
}

/// `seconds` of a breath at about `level`: white noise softened as a
/// breath's is, its high hiss falling away.
fn breath(seconds: f32, level: f32) -> Vec<f32> {
    let mut soft = 0.0;
    let raw: Vec<f32> = noise(seconds, 1.0, false)
        .into_iter()
        .map(|white| {
            soft = 0.98 * soft + white;
            soft
        })
        .collect();
    let loudest = raw.iter().fold(0.0_f32, |most, s| most.max(s.abs()));
    raw.iter().map(|s| s * level / loudest).collect()
}

/// `seconds` of a breath through the throat at about `level`: white
/// noise rung through two resonances, at 600 and 1200 Hz and each
/// 200 Hz wide, as a vowel's first two shape the breath before it.
/// Rung that narrowly, noise is alike to itself a few periods on.
#[expect(
    clippy::cast_precision_loss,
    reason = "a test's few seconds of samples"
)]
fn throat(seconds: f32, level: f32) -> Vec<f32> {
    let ring = |sound: &[f32], pitch: f32| -> Vec<f32> {
        let rate = RATE as f32;
        let fade = (-std::f32::consts::PI * 200.0 / rate).exp();
        let turn = 2.0 * fade * (std::f32::consts::TAU * pitch / rate).cos();
        let (mut last, mut before) = (0.0_f32, 0.0_f32);
        sound
            .iter()
            .map(|white| {
                let next = white + turn * last - fade * fade * before;
                (before, last) = (last, next);
                next
            })
            .collect()
    };
    let white = noise(seconds, 1.0, false);
    let raw: Vec<f32> = ring(&white, 600.0)
        .iter()
        .zip(ring(&white, 1200.0))
        .map(|(low, high)| low + high)
        .collect();
    let loudest = raw.iter().fold(0.0_f32, |most, s| most.max(s.abs()));
    raw.iter().map(|s| s * level / loudest).collect()
}

/// `seconds` of a voice at about -27 dBFS: a pitch and its first twelve
/// harmonics, each fainter, the pitch rising a tenth as a question's
/// does.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "a test's few seconds of samples"
)]
fn voice(seconds: f32, pitch: f32) -> Vec<f32> {
    let count = (seconds * RATE as f32) as usize;
    let mut phase = 0.0_f32;
    (0..count)
        .map(|i| {
            let rising = 1.0 + 0.1 * i as f32 / count as f32;
            phase += std::f32::consts::TAU * pitch * rising / RATE as f32;
            (1..=12)
                .map(|k| (k as f32 * phase).sin() / k as f32)
                .sum::<f32>()
                * 0.05
        })
        .collect()
}

/// `sound` with every other frame's level taken down by `gain`.
fn stepping(sound: &[f32], gain: f32) -> Vec<f32> {
    sound
        .chunks(FRAME)
        .enumerate()
        .flat_map(|(i, frame)| {
            let gain = if i % 2 == 0 { 1.0 } else { gain };
            frame.iter().map(move |sample| sample * gain)
        })
        .collect()
}

/// `seconds` of a room's rumble at `level`: white noise through a filter
/// that passes little above 50 Hz, so its loudness swells and fades by
/// several decibels from one fifth of a second to the next.
#[expect(
    clippy::cast_precision_loss,
    reason = "a test's few seconds of samples"
)]
fn rumble(seconds: f32, level: f32) -> Vec<f32> {
    let mut low = 0.0;
    let raw: Vec<f32> = noise(seconds, 1.0, false)
        .into_iter()
        .map(|white| {
            low = 0.98 * low + white;
            low
        })
        .collect();
    let rms = (raw.iter().map(|s| s * s).sum::<f32>() / raw.len() as f32).sqrt();
    raw.iter().map(|s| s * level / rms).collect()
}

/// A room's quiet, about -70 dBFS.
fn quiet(seconds: f32) -> Vec<f32> {
    tone(seconds, 0.0004, 1.3)
}

/// `seconds` of a 60 Hz mains hum far under any speech, at -83 dBFS.
#[expect(clippy::cast_precision_loss, reason = "the sample rate is small")]
fn hum(seconds: f32) -> Vec<f32> {
    tone(seconds, 0.0001, std::f32::consts::TAU * 60.0 / RATE as f32)
}

fn cut(sound: &[f32]) -> Vec<Take> {
    let mut cutter = Cutter::new();
    let mut takes = Vec::new();
    for piece in sound.chunks(1000) {
        takes.extend(cutter.feed(piece));
    }
    takes.extend(cutter.finish());
    takes
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a test's sample counts stay small"
)]
fn seconds(samples: usize) -> f32 {
    samples as f32 / RATE as f32
}

#[test]
fn a_pause_between_sentences_makes_two_takes_each_starting_just_before_its_speech() {
    let sound = [quiet(2.0), spoken(3.0), quiet(1.5), spoken(2.0), quiet(2.0)].concat();
    let takes = cut(&sound);
    assert_eq!(
        takes.len(),
        2,
        "{:?}",
        takes.iter().map(|t| t.start).collect::<Vec<_>>()
    );
    assert!(
        (seconds(takes[0].start) - 1.8).abs() < 0.05,
        "{}",
        seconds(takes[0].start)
    );
    assert!(
        (seconds(takes[1].start) - 6.3).abs() < 0.05,
        "{}",
        seconds(takes[1].start)
    );
}

#[test]
fn a_breath_between_words_stays_in_one_take() {
    let sound = [quiet(1.0), spoken(2.0), quiet(0.2), spoken(2.0), quiet(1.0)].concat();
    assert_eq!(cut(&sound).len(), 1);
}

#[test]
fn talk_with_no_pause_is_cut_inside_one_window_and_nothing_is_lost() {
    let sound = [quiet(1.0), spoken(70.0)].concat();
    let takes = cut(&sound);
    assert!(takes.len() >= 5, "{}", takes.len());
    let longest = takes.iter().map(|take| take.samples.len()).max();
    assert!(
        longest.is_some_and(|samples| samples < WINDOW && seconds(samples) <= 14.0),
        "the longest take, {longest:?} samples, must be 14 s at most"
    );
    let heard: usize = takes.iter().map(|take| take.samples.len()).sum();
    assert!(seconds(heard) >= 70.0, "{}", seconds(heard));
    for pair in takes.windows(2) {
        assert_eq!(pair[0].start + pair[0].samples.len(), pair[1].start);
    }
}

#[test]
fn loud_noise_with_no_voice_in_it_makes_no_take() {
    let sounds = [
        ("white", noise(1.0, 0.1, false)),
        ("brown", noise(1.0, 0.1, true)),
        ("a breath", breath(1.0, 0.1)),
        ("a breath through the throat", throat(1.0, 0.1)),
        // Its frames are voiced now and then, so a long one must not
        // be heard just by lasting.
        ("a long breath through the throat", throat(3.0, 0.1)),
    ];
    for (name, loud) in sounds {
        let sound = [quiet(2.0), loud, quiet(2.0)].concat();
        assert_eq!(cut(&sound), Vec::new(), "{name}");
    }
}

#[test]
fn a_low_voice_and_a_high_one_are_each_a_take() {
    for pitch in [65.0, 380.0] {
        let sound = [quiet(2.0), voice(0.4, pitch), quiet(2.0)].concat();
        assert_eq!(cut(&sound).len(), 1, "pitch: {pitch}");
    }
}

#[test]
fn a_low_voice_on_a_hissy_line_is_a_take() {
    // The hiss is 10 dB under the voice, as a far end's noisy
    // microphone or a laptop's fan puts it.
    let hissy: Vec<f32> = voice(0.4, 85.0)
        .iter()
        .zip(noise(0.4, 0.014, false))
        .map(|(voice, hiss)| voice + hiss)
        .collect();
    let sound = [quiet(2.0), hissy, quiet(2.0)].concat();
    assert_eq!(cut(&sound).len(), 1);
}

#[test]
fn a_voice_riding_on_a_drift_is_a_take() {
    // The drift is three times the voice, as a breath into the
    // microphone or a draught carries it: read as it comes, it is
    // alike to itself at every lag and hides the voice's repeat.
    let drifting: Vec<f32> = voice(0.4, 85.0)
        .iter()
        .zip(noise(0.4, 0.13, true))
        .map(|(voice, drift)| voice + drift)
        .collect();
    let sound = [quiet(2.0), drifting, quiet(2.0)].concat();
    assert_eq!(cut(&sound).len(), 1);
}

#[test]
fn a_rumble_below_a_voice_makes_no_take() {
    let step = std::f32::consts::TAU * 45.0 / 16_000.0;
    let sound = [quiet(2.0), tone(1.0, 0.1, step), quiet(2.0)].concat();
    assert_eq!(cut(&sound), Vec::new());
}

#[test]
fn noise_takes_three_frames_voiced_with_the_one_before_to_be_heard() {
    // Each frame is read with the one before it, so 60 ms of voice
    // after noise holds two voiced frames and 80 ms holds three. The
    // take is short enough that one voiced frame in twenty loud ones
    // asks for only two, so the three are what's tested.
    for (heard, seconds) in [(false, 0.06), (true, 0.08)] {
        let sound = [
            quiet(2.0),
            noise(0.2, 0.05, false),
            voice(seconds, 150.0),
            noise(0.2, 0.05, false),
            quiet(2.0),
        ]
        .concat();
        assert_eq!(
            cut(&sound).len(),
            usize::from(heard),
            "{seconds} s of voice"
        );
    }
}

#[test]
fn a_short_answer_in_a_long_loud_take_is_a_take() {
    // 0.3 s of voice in 4 s of loud noise is one loud frame in
    // fourteen.
    let sound = [
        quiet(2.0),
        noise(2.0, 0.05, false),
        voice(0.3, 150.0),
        noise(2.0, 0.05, false),
        quiet(2.0),
    ]
    .concat();
    assert_eq!(cut(&sound).len(), 1, "the answer in the noise");
}

#[test]
fn a_short_answer_after_a_line_sent_nothing_is_a_take() {
    // A call sends nothing while its far end is muted, so the line's
    // hiss comes back only with the answer, and the answer must not
    // be lost in it: whether the call was quieter before the mute, or
    // the nothing ends a few samples into a frame, so that frame
    // holds only a little of the hiss, or the nothing is a faint
    // dither, about -120 dBFS, rather than exact zeros, or the mute is
    // the half second the recorder fills a lost link with, starting
    // half a frame in, so that only 24 of its frames are whole.
    for (before, mute, into_a_frame, faint) in [
        (0.0, RATE * 3, 0, 0.0),
        (5.0, RATE * 3, 0, 0.0),
        (0.0, RATE * 3, FRAME - 10, 0.0),
        (0.0, RATE * 3, FRAME - 10, 1e-6),
        (5.01, RATE / 2, 0, 0.0),
    ] {
        let nothing: Vec<f32> = (0..mute + into_a_frame)
            .map(|i| if i % 2 == 0 { faint } else { -faint })
            .collect();
        let sound = [
            quiet(before),
            nothing,
            noise(3.5, 0.005, false),
            voice(0.3, 150.0),
            noise(3.5, 0.005, false),
            quiet(2.0),
        ]
        .concat();
        let takes = cut(&sound);
        assert_eq!(
            takes.len(),
            1,
            "the answer in the hiss, after {before} s of quiet, \
             {into_a_frame} samples of a frame and nothing of {faint}"
        );
        let answer = quiet(before).len() + mute + into_a_frame + RATE * 7 / 2;
        let take = &takes[0];
        assert!(
            take.start <= answer
                && take.start + take.samples.len() >= answer + voice(0.3, 150.0).len(),
            "the whole answer, after {before} s of quiet"
        );
    }
}

#[test]
fn speech_across_dropped_packets_is_heard_whole() {
    // A call drops a few packets now and then, and sends nothing in
    // their place: the line hasn't changed, so its floor still holds,
    // and the words after the gap mustn't set a floor of their own.
    let sound = [
        quiet(2.0),
        voice(1.5, 150.0),
        vec![0.0; RATE / 5],
        voice(2.5, 150.0),
        quiet(2.0),
    ]
    .concat();
    let takes = cut(&sound);
    assert_eq!(takes.len(), 1, "the words across the gap");
    assert!(
        seconds(takes[0].samples.len()) >= 4.2,
        "{}",
        seconds(takes[0].samples.len())
    );
}

#[test]
fn a_short_answer_after_a_dropped_part_of_a_frame_is_a_take() {
    // Packets dropped part of the way into a frame leave it mostly
    // nothing, but not quite: it mustn't set a floor under the line's
    // hiss, or the hiss reads as loud and the answer is lost in it. The
    // drop may end a few samples into the frame or start a few in, or
    // sit inside it with a little sound on each side, as a resampler's
    // ringing leaves it.
    for into in [0, 5, 10] {
        let sound = [
            noise(3.0, 0.005, false),
            noise(1.0, 0.005, false)[..into].to_vec(),
            vec![0.0; FRAME - 10],
            noise(3.5, 0.005, false),
            voice(0.3, 150.0),
            noise(3.5, 0.005, false),
            quiet(2.0),
        ]
        .concat();
        assert_eq!(
            cut(&sound).len(),
            1,
            "the answer in the hiss, {into} samples in"
        );
    }
}

#[test]
fn a_lost_packet_adds_no_hiss_to_a_take() {
    // The line's floor holds across the packets lost after a word, up
    // to just short of a mute, so its hiss after them is no more
    // speech than before: the take is the word and its edges, where a
    // cleared floor would add a fifth of a second of hiss at least.
    // The hiss wanders, as a line's does, by 1.4 dB a frame. The
    // losses are counted out, not drawn from the mute's length, so a
    // shorter mute fails them.
    let hiss = stepping(&noise(3.0, 0.005, false), 0.85);
    for lost in [2, 10, 19] {
        let sound = [
            hiss.clone(),
            voice(0.4, 150.0),
            vec![0.0; FRAME * lost],
            hiss.clone(),
            quiet(1.0),
        ]
        .concat();
        let takes = cut(&sound);
        assert_eq!(takes.len(), 1, "the voice, across {lost} frames lost");
        assert!(
            seconds(takes[0].samples.len()) < 0.9,
            "{}, across {lost} frames lost",
            seconds(takes[0].samples.len())
        );
    }
}

#[test]
fn a_soft_answer_on_a_line_idling_on_a_faint_hum_is_a_take() {
    // A line idling on a hum far under any speech, at -83 dB, dips
    // under nothing's size for a millisecond at each crossing, yet its
    // frames are its floor, not nothing cut into: a soft answer after
    // its talk, 20 dB under it, is speech.
    let soft: Vec<f32> = voice(0.3, 150.0)
        .iter()
        .map(|sample| sample * 0.1)
        .collect();
    let sound = [hum(2.0), voice(2.0, 150.0), hum(1.0), soft, hum(1.0)].concat();
    assert_eq!(cut(&sound).len(), 2, "the talk and the soft answer");
}

#[test]
fn words_with_nothing_between_are_a_take() {
    // A far end that gates its line sends nothing between words, so
    // the words alone mustn't set the floor they're heard against, nor
    // a lone answer, whether or not the call was quiet before.
    for before in [0.0, 2.0] {
        let mut sound = [quiet(before), vec![0.0; RATE * 2]].concat();
        for _ in 0..6 {
            sound.extend(voice(0.25, 150.0));
            sound.extend(vec![0.0; RATE / 10]);
        }
        sound.extend(vec![0.0; RATE * 2]);
        assert_eq!(cut(&sound).len(), 1, "the words, after {before} s of quiet");
        let answer = [
            quiet(before),
            vec![0.0; RATE * 2],
            voice(0.3, 150.0),
            vec![0.0; RATE * 2],
        ]
        .concat();
        assert_eq!(
            cut(&answer).len(),
            1,
            "the answer, after {before} s of quiet"
        );
    }
}

#[test]
fn a_long_word_on_a_gated_line_is_heard_whole() {
    // A gated line's words may run past a second with no nothing
    // between, steady enough that none of them is 12 dB under the
    // rest: they mustn't become the floor the words after them are
    // judged against, nor may a word's unvoiced sound, a tenth of a
    // second of loud hiss, be taken for the line's own.
    for hiss in [0.0, 0.1] {
        let sound = [
            vec![0.0; RATE * 2],
            voice(0.7, 150.0),
            noise(hiss, 0.05, false),
            voice(0.8, 150.0),
            vec![0.0; RATE / 10],
            voice(1.0, 150.0),
            vec![0.0; RATE * 2],
        ]
        .concat();
        let takes = cut(&sound);
        assert_eq!(takes.len(), 1, "the words, with {hiss} s of hiss");
        assert!(
            seconds(takes[0].samples.len()) >= 2.6 + hiss,
            "{}, with {hiss} s of hiss",
            seconds(takes[0].samples.len())
        );
    }
}

#[test]
fn a_line_that_starts_gating_is_heard_once_its_hiss_is_gone() {
    // A far end's noise suppression may start gating its line with
    // no mute, sending nothing between words: once the hiss heard
    // before has left the floor's ten seconds, the line's words are
    // all its floor holds, and they mustn't be judged against
    // themselves.
    let mut sound = noise(3.0, 0.005, false);
    for _ in 0..50 {
        sound.extend(voice(0.25, 150.0));
        sound.extend(vec![0.0; RATE / 10]);
    }
    sound.extend(vec![0.0; RATE * 2]);
    let heard: usize = cut(&sound).iter().map(|take| take.samples.len()).sum();
    assert!(seconds(heard) >= 17.0, "{} s of 17.5", seconds(heard));
}

#[test]
fn a_laugh_or_a_held_s_on_a_gated_line_keeps_it_gated() {
    // A laugh, a cough or a held "s" between a gated line's words is
    // loud and has no voice, as hiss hasn't, but a laugh or a cough
    // rises and falls as hiss doesn't, and an "s" ends sooner: none may
    // be taken for the line's own sound and set a floor the words after
    // it are lost under. The laugh lasts 0.8 s, longer than the line's
    // own sound must, and its level steps 6 dB from one frame to the
    // next; the "s" is steady, as loud as the words, for 0.4 s.
    for (what, sound) in [
        ("laugh", stepping(&noise(0.8, 0.045, false), 0.5)),
        ("held s", noise(0.4, 0.045, false)),
    ] {
        let long = 1.5 + seconds(sound.len());
        let sound = [
            vec![0.0; RATE * 2],
            voice(0.7, 150.0),
            sound,
            voice(0.8, 150.0),
            vec![0.0; RATE * 2],
        ]
        .concat();
        let takes = cut(&sound);
        assert_eq!(takes.len(), 1, "the words around the {what}");
        assert!(
            seconds(takes[0].samples.len()) >= long,
            "{} s of {long}, around the {what}",
            seconds(takes[0].samples.len())
        );
    }
}

#[test]
fn a_short_answer_on_a_rumbling_line_after_a_mute_is_a_take() {
    // A room's rumble swells and fades by several decibels in a fifth
    // of a second, as hiss doesn't, so after a mute it must still be
    // known for the line's own sound, or the line stays gated, its
    // rumble all speech, and an answer in it is one long take, lost
    // once it holds too few voiced frames for its length. The answer
    // comes 0.7 s after the mute, so the line's own sound must be known
    // inside that.
    let sound = [
        rumble(3.0, 0.01),
        vec![0.0; RATE],
        rumble(0.7, 0.01),
        voice(0.3, 150.0),
        rumble(4.0, 0.01),
        quiet(1.0),
    ]
    .concat();
    let answer = RATE * 47 / 10;
    let takes = cut(&sound);
    let take = takes
        .iter()
        .find(|take| {
            take.start <= answer && take.start + take.samples.len() >= answer + RATE * 3 / 10
        })
        .expect("the answer in the rumble");
    assert!(
        seconds(take.samples.len()) < 2.0,
        "{} s of take for a 0.3 s answer",
        seconds(take.samples.len())
    );
}

#[test]
fn quiet_alone_and_a_click_make_no_take() {
    let sound = [quiet(5.0), spoken(0.1), quiet(5.0)].concat();
    assert_eq!(cut(&sound), Vec::new());
}
