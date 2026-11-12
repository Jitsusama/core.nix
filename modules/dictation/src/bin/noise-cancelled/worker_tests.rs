use std::f32::consts::TAU;

use rtrb::RingBuffer;

use super::*;

/// A microphone that's recorded from, with the test's ends of its rings:
/// where it hears, and where its cleaned sound comes out.
fn recorded() -> (Microphone, Producer<f32>, Consumer<f32>) {
    let (microphone_side, heard) = RingBuffer::new(16 * FRAME);
    let (cleaned, out) = RingBuffer::new(16 * FRAME);
    let shared = Arc::new(Shared::default());
    shared.wanted.store(true, Ordering::Release);
    let microphone = Microphone {
        ends: Ends {
            heard,
            cleaned,
            shared,
        },
        denoiser: None,
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
    clean(&mut microphone, &arrived(model));
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
    clean(&mut microphone, &model);
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
