use std::f32::consts::TAU;

use rtrb::RingBuffer;

use super::*;
use crate::spot::Where;

/// A microphone that's recorded from, with the test's ends of its rings:
/// where it hears, and where its cleaned sound comes out.
fn recorded() -> (Microphone, Producer<f32>, Consumer<f32>) {
    named("mv7")
}

/// A microphone whose node is `name`, recorded from, with the test's ends.
fn named(name: &str) -> (Microphone, Producer<f32>, Consumer<f32>) {
    let (microphone_side, heard) = RingBuffer::new(16 * FRAME);
    let (cleaned, out) = RingBuffer::new(16 * FRAME);
    let shared = Arc::new(Shared::default());
    shared.wanted.store(true, Ordering::Release);
    let microphone = Microphone {
        ends: Ends {
            heard,
            cleaned,
            shared,
            microphone: name.to_owned(),
            description: name.to_owned(),
        },
        cleaner: None,
        denoiser: None,
        began: None,
        room: Room::default(),
        speech: Speech::default(),
    };
    (microphone, microphone_side, out)
}

/// A frame of a voice-like tone.
fn voice() -> [f32; FRAME] {
    std::array::from_fn(|at| {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a frame's sample index is far below f32's exact range"
        )]
        let at = at as f32;
        0.3 * (TAU * 220.0 * at / 48_000.0).sin()
    })
}

/// Everything that's come out so far.
fn drained(out: &mut Consumer<f32>) -> Vec<f32> {
    let mut samples = Vec::new();
    while let Ok(sample) = out.pop() {
        samples.push(sample);
    }
    samples
}

/// A model that takes a while to load, as `DeepFilterNet`'s does.
fn slow() -> Result<Model, String> {
    thread::sleep(Duration::from_millis(300));
    Model::load(Path::new("/nowhere"), Cleaner::RNNoise)
}

/// Waits for the model to arrive, no longer than a few seconds.
fn awaited(mut model: Loaded) -> Loaded {
    let deadline = Instant::now() + Duration::from_secs(5);
    while matches!(model, Loaded::Loading(_)) && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
        model = arrived(model);
    }
    model
}

#[test]
fn loading_a_model_leaves_the_thread_free() {
    let started = Instant::now();
    let model = loading(Cleaner::RNNoise, slow);
    assert!(
        started.elapsed() < Duration::from_millis(100),
        "loading held the thread {} ms",
        started.elapsed().as_millis()
    );
    assert!(
        matches!(model, Loaded::Loading(_)),
        "the model should still be loading"
    );
}

#[test]
fn a_voice_passes_as_heard_while_its_model_loads() {
    let (mut microphone, mut hears, mut out) = recorded();
    let model = loading(Cleaner::RNNoise, slow);
    let frame = voice();
    hears.push_entire_slice(&frame).expect("the ring has room");
    hears.push_entire_slice(&frame).expect("the ring has room");
    clean(&mut microphone, Some(&arrived(model)));
    let passed = drained(&mut out);
    assert_eq!(passed, [frame, frame].concat());
}

#[test]
fn the_model_cleans_once_it_arrives() {
    let (mut microphone, mut hears, mut out) = recorded();
    let model = awaited(loading(Cleaner::RNNoise, slow));
    assert!(
        matches!(model, Loaded::Yes(_)),
        "the model should have arrived"
    );
    let frame = voice();
    hears.push_entire_slice(&frame).expect("the ring has room");
    clean(&mut microphone, Some(&model));
    let cleaned = drained(&mut out);
    assert_eq!(cleaned.len(), FRAME, "one frame should come out");
    assert_ne!(
        cleaned,
        frame.to_vec(),
        "the frame should have been cleaned"
    );
}

#[test]
fn a_model_that_wont_load_is_refused() {
    let model = awaited(loading(Cleaner::DeepFilterNet, || {
        Err("no graph".to_owned())
    }));
    assert!(
        matches!(model, Loaded::Refused(_)),
        "a model that won't load should be refused"
    );
}

/// Settles what each microphone cleans with, as the worker does, with no
/// offer kept anywhere.
fn settle(microphones: &mut HashMap<u32, Microphone>, chosen: &Chosen) -> Vec<Cleaner> {
    settled(
        microphones,
        chosen,
        &mut Offers::new(chosen, Vec::new()),
        &mut Vec::new(),
    )
}

/// Picks kept in a file of the test's own, as the trial writes them, on a
/// machine that knows nothing of where it is, so its spot is
/// `network=none`.
fn picked(name: &str, said: &str) -> (Chosen, PathBuf) {
    let file = std::env::temp_dir().join(format!("worker-{name}-{}", std::process::id()));
    std::fs::write(&file, said).expect("the picks are written");
    let nowhere = file.with_extension("nowhere");
    (
        Chosen::new(Some(file.clone()), Cleaner::DeepFilterNet).within(Where::at(&nowhere)),
        file,
    )
}

#[test]
fn a_microphone_cleans_with_its_pick_at_the_spot_its_at() {
    let (chosen, file) = picked(
        "spot",
        "deepfilternet-gentle mv7 network=none\nrnnoise mv7 home=6366013\n",
    );
    let mut microphones = HashMap::from([(1, named("mv7").0)]);
    settle(&mut microphones, &chosen);
    assert_eq!(
        microphones[&1].cleaner,
        Some(Cleaner::DeepFilterNetGentle),
        "the MV7 should clean with its pick here, not its newest at home"
    );
    let _ = std::fs::remove_file(&file);
}

#[test]
fn each_microphone_cleans_with_its_own_pick() {
    let (chosen, file) = picked("own", "rnnoise mv7\ndeepfilternet-gentle laptop\n");
    let mut microphones = HashMap::from([(1, named("mv7").0), (2, named("laptop").0)]);
    let wanted = settle(&mut microphones, &chosen);
    assert_eq!(microphones[&1].cleaner, Some(Cleaner::RNNoise));
    assert_eq!(
        microphones[&2].cleaner,
        Some(Cleaner::DeepFilterNetGentle),
        "the laptop's microphone took another's pick"
    );
    assert_eq!(wanted.len(), 2, "both cleaners should load: {wanted:?}");
    let _ = std::fs::remove_file(&file);
}

#[test]
fn a_pick_cleans_from_the_next_recording() {
    let (chosen, file) = picked("next", "rnnoise mv7\n");
    let mut microphones = HashMap::from([(1, named("mv7").0)]);
    settle(&mut microphones, &chosen);
    chosen
        .choose("mv7", &chosen.spot(), Cleaner::DeepFilterNetGentle)
        .expect("picked again");
    settle(&mut microphones, &chosen);
    assert_eq!(
        microphones[&1].cleaner,
        Some(Cleaner::RNNoise),
        "a pick changed a call under way"
    );
    let shared = Arc::clone(&microphones[&1].ends.shared);
    shared.wanted.store(false, Ordering::Release);
    assert!(
        settle(&mut microphones, &chosen).is_empty(),
        "nothing records"
    );
    shared.wanted.store(true, Ordering::Release);
    settle(&mut microphones, &chosen);
    assert_eq!(microphones[&1].cleaner, Some(Cleaner::DeepFilterNetGentle));
    let _ = std::fs::remove_file(&file);
}

#[test]
fn a_call_that_ends_offers_choose_by_ear() {
    let folder = std::env::temp_dir().join(format!("worker-offered-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let chosen = Chosen::new(Some(folder.join("cleaner")), Cleaner::DeepFilterNet)
        .within(Where::at(&folder.join("nowhere")));
    let offer = chosen.beside("offer").expect("picks are kept");
    let mut offers = Offers::new(&chosen, Vec::new());
    let mut microphones = HashMap::from([(1, named("mv7").0)]);
    settled(&mut microphones, &chosen, &mut offers, &mut Vec::new());
    let call = Instant::now().checked_sub(Duration::from_mins(2));
    let microphone = microphones.get_mut(&1).expect("the MV7 is there");
    let began = microphone.began.as_mut().expect("its recording began");
    began.0 = call.expect("the clock goes back 2 minutes");
    microphone
        .ends
        .shared
        .wanted
        .store(false, Ordering::Release);
    settled(&mut microphones, &chosen, &mut offers, &mut Vec::new());
    offers.stand(&chosen);
    let said = std::fs::read_to_string(&offer).unwrap_or_default();
    assert!(
        said.contains("\"node\":\"mv7\""),
        "the MV7 should be offered: {said:?}"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

/// A frame of a steady hum at `amplitude`, a room's noise.
fn hum(amplitude: f32) -> [f32; FRAME] {
    std::array::from_fn(|at| if at % 2 == 0 { amplitude } else { -amplitude })
}

#[test]
fn what_a_microphone_hears_raw_is_its_room() {
    let (mut microphone, mut hears, mut out) = recorded();
    for _ in 0..12 {
        for _ in 0..10 {
            hears
                .push_entire_slice(&hum(0.01))
                .expect("the ring has room");
        }
        clean(&mut microphone, None);
        drained(&mut out);
    }
    assert_eq!(microphone.room.floor(), Some(-40), "the hum is the room");
}

#[test]
fn a_call_in_a_room_far_from_its_picks_offers_choose_by_ear_again() {
    let folder = std::env::temp_dir().join(format!("worker-room-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let chosen = Chosen::new(Some(folder.join("cleaner")), Cleaner::DeepFilterNet)
        .within(Where::at(&folder.join("nowhere")));
    let here = chosen.spot();
    chosen
        .choose("mv7", &here, Cleaner::RNNoise)
        .expect("the pick is kept");
    crate::room::kept(&chosen, "mv7", &here.key, -70).expect("its room is kept");
    let offer = chosen.beside("offer").expect("picks are kept");
    let mut offers = Offers::new(&chosen, Vec::new());
    let mut microphones = HashMap::from([(1, named("mv7").0)]);
    settled(&mut microphones, &chosen, &mut offers, &mut Vec::new());
    let call = Instant::now().checked_sub(Duration::from_mins(2));
    let microphone = microphones.get_mut(&1).expect("the MV7 is there");
    let began = microphone.began.as_mut().expect("its recording began");
    began.0 = call.expect("the clock goes back 2 minutes");
    for _ in 0..200 {
        microphone.room.heard(&hum(0.01));
    }
    microphone
        .ends
        .shared
        .wanted
        .store(false, Ordering::Release);
    settled(&mut microphones, &chosen, &mut offers, &mut Vec::new());
    offers.stand(&chosen);
    let said = std::fs::read_to_string(&offer).unwrap_or_default();
    assert!(
        said.contains("\"new\":\"noise\"") && said.contains("\"by\":30"),
        "a room 30 dB louder should be offered again: {said:?}"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

/// The MV7 recorded from for `lasted`, speaking `frames` of speech over a
/// quiet room after its warm-up, then let go: the calls it leaves to judge.
fn spoke(lasted: Duration, frames: usize) -> Vec<Call> {
    spoken(lasted, frames, false)
}

/// As [`spoke`], with the microphone `listened` to by a call too.
fn spoken(lasted: Duration, frames: usize, listened: bool) -> Vec<Call> {
    let chosen = Chosen::new(None, Cleaner::DeepFilterNet).within(Where::at(
        &std::env::temp_dir().join("worker-spoke-nowhere"),
    ));
    let mut offers = Offers::new(&chosen, Vec::new());
    let mut microphones = HashMap::from([(1, named("mv7").0)]);
    settle(&mut microphones, &chosen);
    let microphone = microphones.get_mut(&1).expect("the MV7 is there");
    microphone
        .ends
        .shared
        .listened
        .store(listened, Ordering::Release);
    let began = microphone.began.as_mut().expect("its recording began");
    began.0 = Instant::now()
        .checked_sub(lasted)
        .expect("the clock goes back");
    for _ in 0..200 {
        microphone.room.heard(&hum(0.001));
        microphone
            .speech
            .heard(&hum(0.001), microphone.room.floor());
    }
    for _ in 0..frames {
        microphone.speech.heard(&hum(0.1), microphone.room.floor());
    }
    microphone
        .ends
        .shared
        .wanted
        .store(false, Ordering::Release);
    let mut calls = Vec::new();
    settled(&mut microphones, &chosen, &mut offers, &mut calls);
    calls
}

#[test]
fn a_call_that_spoke_enough_is_judged_once_it_ends() {
    let calls = spoke(Duration::from_mins(2), 400);
    assert_eq!(calls.len(), 1, "the call wasn't left to judge");
    let call = &calls[0];
    assert_eq!(call.node, "mv7");
    assert_eq!(
        call.running,
        Way::Cleaned(Cleaner::DeepFilterNet),
        "it ran with the default"
    );
    assert_eq!(
        call.speech.len(),
        400 * FRAME,
        "the room was judged as speech"
    );
}

#[test]
fn a_twins_call_is_judged_as_the_twins_while_a_call_hears_its_microphone() {
    let calls = spoken(Duration::from_mins(2), 400, true);
    assert_eq!(
        calls.first().map(|call| call.running),
        Some(Way::Cleaned(Cleaner::DeepFilterNet)),
        "the twin's call wasn't judged as cleaned"
    );
}

#[test]
fn a_call_that_skips_noise_cancelling_is_heard_and_judged_raw() {
    let folder = std::env::temp_dir().join(format!("worker-plain-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let chosen = Chosen::new(Some(folder.join("cleaner")), Cleaner::DeepFilterNet)
        .within(Where::at(&folder.join("nowhere")));
    let mut offers = Offers::new(&chosen, Vec::new());
    let (microphone, mut hears, mut out) = named("mv7");
    let shared = Arc::clone(&microphone.ends.shared);
    shared.wanted.store(false, Ordering::Release);
    shared.listened.store(true, Ordering::Release);
    let mut microphones = HashMap::from([(1, microphone)]);
    let wanted = settled(&mut microphones, &chosen, &mut offers, &mut Vec::new());
    assert!(wanted.is_empty(), "a cleaner was loaded for {wanted:?}");
    assert!(hearing(&microphones), "the call wasn't being heard");
    let microphone = microphones.get_mut(&1).expect("the MV7 is there");
    let began = microphone.began.as_mut().expect("hearing the call began");
    began.0 = Instant::now()
        .checked_sub(Duration::from_mins(2))
        .expect("the clock goes back");
    for at in 0..50 {
        let amplitude = if at < 12 { 0.001 } else { 0.1 };
        for _ in 0..10 {
            hears
                .push_entire_slice(&hum(amplitude))
                .expect("the ring has room");
        }
        clean(microphone, None);
        assert!(
            drained(&mut out).is_empty(),
            "what a call on the microphone itself said was given to its twin"
        );
    }
    shared.listened.store(false, Ordering::Release);
    let mut calls = Vec::new();
    settled(&mut microphones, &chosen, &mut offers, &mut calls);
    assert!(!hearing(&microphones), "the call was heard after it ended");
    assert_eq!(
        calls
            .iter()
            .map(|call| (call.running, call.speech.len()))
            .collect::<Vec<_>>(),
        vec![(Way::Raw, 380 * FRAME)],
        "the call wasn't left to judge as raw, with its speech"
    );
    offers.stand(&chosen);
    let offer = chosen.beside("offer").expect("picks are kept");
    let said = std::fs::read_to_string(&offer).unwrap_or_default();
    assert!(
        said.contains("\"node\":\"mv7\""),
        "the call didn't offer Choose by Ear for the MV7: {said:?}"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_moment_or_a_call_too_quiet_is_not_judged() {
    assert!(
        spoke(Duration::from_secs(20), 400).is_empty(),
        "20 seconds was judged as a call"
    );
    assert!(
        spoke(Duration::from_mins(2), 100).is_empty(),
        "a second of speech was judged"
    );
}

#[test]
fn what_a_microphone_hears_raw_over_its_room_is_its_speech() {
    let (mut microphone, mut hears, mut out) = recorded();
    for at in 0..24 {
        let amplitude = if at < 12 { 0.001 } else { 0.1 };
        for _ in 0..10 {
            hears
                .push_entire_slice(&hum(amplitude))
                .expect("the ring has room");
        }
        clean(&mut microphone, None);
        drained(&mut out);
    }
    assert_eq!(
        microphone.speech.taken().len(),
        120 * FRAME,
        "what it heard over its room wasn't held"
    );
}
