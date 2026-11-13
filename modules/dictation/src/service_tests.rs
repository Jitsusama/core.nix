use super::*;

#[test]
fn each_state_has_its_word() {
    assert_eq!(
        [State::Idle, State::Listening, State::Transcribing].map(State::word),
        ["idle", "listening", "transcribing"]
    );
}

#[test]
fn a_press_while_writing_says_so_and_waits() {
    let (events, _heard) = mpsc::channel();
    let (jobs, work) = mpsc::channel();
    let mut generation = 3;
    let (state, said) = press(
        State::Transcribing,
        &mut None,
        &mut generation,
        &events,
        &jobs,
    );
    assert_eq!(state, State::Transcribing);
    assert_eq!(said, Some(Said::Word("still-writing")));
    assert_eq!(generation, 3);
    assert!(work.try_recv().is_err());
}

#[test]
fn letting_go_of_a_held_chord_stops_what_it_started() {
    assert!(let_go(State::Listening, Some(4), 4, 400));
    assert!(let_go(State::Listening, Some(4), 4, 9_000));
}

#[test]
fn a_tap_s_release_leaves_it_listening() {
    assert!(!let_go(State::Listening, Some(4), 4, 399));
}

#[test]
fn it_says_it_listens_only_once_the_microphone_sounds() {
    assert_eq!(shown(State::Listening, false), State::Idle);
    assert_eq!(shown(State::Listening, true), State::Listening);
    assert_eq!(shown(State::Transcribing, false), State::Transcribing);
    assert_eq!(shown(State::Idle, true), State::Idle);
}

#[test]
fn a_release_stops_nothing_it_didnt_start() {
    // A toggle started it, or the press that started it was stopped
    // since, or it's no longer listening.
    assert!(!let_go(State::Listening, None, 4, 900));
    assert!(!let_go(State::Listening, Some(3), 4, 900));
    assert!(!let_go(State::Transcribing, Some(4), 4, 900));
    assert!(!let_go(State::Idle, Some(4), 4, 900));
}

#[test]
fn a_press_while_listening_stops_quietly() {
    let (events, _heard) = mpsc::channel();
    let (jobs, _work) = mpsc::channel();
    let mut generation = 3;
    let (state, said) = press(State::Listening, &mut None, &mut generation, &events, &jobs);
    assert_eq!(state, State::Idle);
    assert!(said.is_none());
    assert_eq!(generation, 4);
}

/// A recording that has ended, holding `samples` of one level.
fn recorded(level: i16, samples: usize, muted: bool) -> Recording {
    Recording {
        child: Command::new("true").spawn().expect("true runs"),
        reader: thread::spawn(|| {}),
        samples: Arc::new(Mutex::new(vec![level; samples])),
        muted,
    }
}

#[test]
fn each_reason_for_nothing_has_its_word() {
    assert_eq!(
        [
            Nothing::Muted,
            Nothing::Short,
            Nothing::Silent,
            Nothing::Unclear
        ]
        .map(Nothing::word),
        [
            "heard-nothing muted",
            "heard-nothing short",
            "heard-nothing silent",
            "heard-nothing unclear"
        ]
    );
}

#[test]
fn silence_isnt_transcribed_and_says_so() {
    let (jobs, work) = mpsc::channel();
    let silent = recorded(12, RATE, false);
    assert_eq!(stop(Some(silent), &jobs), Err(Nothing::Silent));
    assert!(work.try_recv().is_err());
}

#[test]
fn speech_is_handed_to_whisper() {
    let (jobs, work) = mpsc::channel();
    let speech = recorded(8_000, RATE, false);
    assert_eq!(stop(Some(speech), &jobs), Ok(State::Transcribing));
    assert!(matches!(work.try_recv(), Ok(Job::Transcribe(audio)) if audio.len() == RATE));
}

#[test]
fn a_moment_isnt_transcribed_and_says_so() {
    let (jobs, _work) = mpsc::channel();
    let brief = recorded(8_000, RATE / 4, false);
    assert_eq!(stop(Some(brief), &jobs), Err(Nothing::Short));
}

#[test]
fn a_muted_microphone_is_named_over_silence_or_a_moment() {
    let (jobs, _work) = mpsc::channel();
    assert_eq!(
        stop(Some(recorded(0, RATE, true)), &jobs),
        Err(Nothing::Muted)
    );
    assert_eq!(
        stop(Some(recorded(0, RATE / 4, true)), &jobs),
        Err(Nothing::Muted)
    );
}

#[test]
fn speech_from_a_microphone_read_as_muted_is_still_typed() {
    assert_eq!(judge(&vec![0.25; RATE], true), None);
}

#[test]
fn a_stop_that_typed_nothing_says_why_before_idle() {
    let (mut ours, theirs) = UnixStream::pair().expect("a socket pair");
    let mut watchers = vec![theirs];
    assert_eq!(stopped(Err(Nothing::Short), &mut watchers), State::Idle);
    tell(State::Idle.word(), &mut watchers);
    drop(watchers);
    let mut heard = String::new();
    ours.read_to_string(&mut heard).expect("what was said");
    assert_eq!(heard, "heard-nothing short\nidle\n");
}

#[test]
fn a_tidy_in_time_is_typed() {
    let mut waiting = Some((2, "um hello".to_owned()));
    assert_eq!(
        settle(&mut waiting, 2, Some("Hello.".to_owned())).as_deref(),
        Some("Hello.")
    );
    assert!(waiting.is_none());
}

#[test]
fn a_take_with_no_tidy_is_typed_as_heard() {
    let mut waiting = Some((2, "um hello".to_owned()));
    assert_eq!(settle(&mut waiting, 2, None).as_deref(), Some("um hello"));
}

#[test]
fn a_tidy_after_its_time_is_dropped() {
    let mut waiting = Some((2, "um hello".to_owned()));
    // The time ran out, which typed the take as heard; its tidy then
    // comes, and is typed nowhere.
    assert_eq!(settle(&mut waiting, 2, None).as_deref(), Some("um hello"));
    assert_eq!(settle(&mut waiting, 2, Some("Hello.".to_owned())), None);
}

#[test]
fn an_older_takes_tidy_isnt_the_new_ones() {
    let mut waiting = Some((3, "the new take".to_owned()));
    assert_eq!(
        settle(&mut waiting, 2, Some("The old take.".to_owned())),
        None
    );
    assert_eq!(waiting, Some((3, "the new take".to_owned())));
}
