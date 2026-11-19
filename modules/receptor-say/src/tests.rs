use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

const MYCELIUM: &str = "/nix/store/x-mycelium/bin/mycelium";

/// How many times a stand-in another test's fork holds busy is run again
/// before the test fails.
const BUSY_TRIES: usize = 1000;

fn speaker() -> Speaker {
    Speaker::named(Some(MYCELIUM.into()), Some("work".into())).expect("a sound speaker")
}

fn words(said: &[OsString]) -> Vec<&str> {
    said.iter()
        .map(|word| word.to_str().expect("every argument is text"))
        .collect()
}

#[test]
fn only_an_absolute_program_and_a_realm_of_mycelium_s_letters_are_taken() {
    let program = || Some(OsString::from(MYCELIUM));
    let taken = [
        Speaker::named(program(), Some("work".into())),
        Speaker::named(program(), Some("personal-2".into())),
        Speaker::named(Some("mycelium".into()), Some("work".into())),
        Speaker::named(program(), Some("work; rm".into())),
        Speaker::named(program(), Some("--realm".into())),
        Speaker::named(program(), Some("Work".into())),
        Speaker::named(program(), Some(String::new().into())),
        Speaker::named(program(), None),
        Speaker::named(None, Some("work".into())),
    ]
    .map(|named| named.is_some());
    assert_eq!(
        taken,
        [true, true, false, false, false, false, false, false, false],
        "only an absolute program and a realm of mycelium's letters are taken"
    );
}

#[test]
fn mycelium_is_given_fixed_words_the_realm_and_integers() {
    let told = Told {
        source: Source::GitHub,
        state: State::Reading,
        hour: 1,
        today: 2,
        addressed: 3,
    };
    let said = speaker().arguments(told, Duration::from_secs(60), 4242);
    assert_eq!(
        words(&said),
        [
            "--realm",
            "work",
            "receptors",
            "say",
            "github",
            "--state",
            "reading",
            "--hour",
            "1",
            "--today",
            "2",
            "--addressed",
            "3",
            "--every",
            "60",
            "--pid",
            "4242",
        ]
    );
}

#[test]
fn a_count_past_what_mycelium_takes_is_said_as_the_most_it_takes() {
    let told = Told {
        source: Source::Calls,
        state: State::Reading,
        hour: u64::from(u32::MAX) + 1,
        today: u64::MAX,
        addressed: u64::MAX,
    };
    let said = speaker().arguments(told, Duration::from_secs(60), 1);
    let counts: Vec<&str> = [8, 10, 12].map(|at| words(&said)[at]).to_vec();
    assert_eq!(
        counts, ["4294967295"; 3],
        "a count past u32, which mycelium refuses, wasn't held to it"
    );
}

#[test]
fn a_round_shorter_than_a_second_is_said_as_one() {
    let said = speaker().arguments(Told::off(Source::Mail), Duration::from_millis(10), 1);
    assert_eq!(words(&said)[14], "1", "mycelium takes no round of 0 s");
}

#[test]
fn each_source_and_state_is_said_in_mycelium_s_own_word() {
    let sources = [
        Source::Slack,
        Source::Mail,
        Source::Calendar,
        Source::Calls,
        Source::Reviews,
        Source::GitHub,
        Source::Messages,
    ]
    .map(Source::word);
    assert_eq!(
        sources,
        [
            "slack", "mail", "calendar", "calls", "reviews", "github", "messages"
        ]
    );
    let states = [State::Reading, State::Quiet, State::Failing, State::Off].map(State::word);
    assert_eq!(states, ["reading", "quiet", "failing", "off"]);
}

#[test]
fn a_source_said_off_counts_nothing() {
    assert_eq!(
        Told::off(Source::Calendar),
        Told {
            source: Source::Calendar,
            state: State::Off,
            hour: 0,
            today: 0,
            addressed: 0,
        }
    );
}

#[test]
fn only_an_address_is_declared_as_one() {
    let declared = [
        "joel@example.com",
        "joel",
        "@example.com",
        "joel@",
        "joel @example.com",
        "joel@example.com\n",
    ]
    .map(|address| Declared::address(Some(address.into())).is_some());
    assert_eq!(
        declared,
        [true, false, false, false, false, false],
        "only an address with no space is declared"
    );
    assert_eq!(Declared::address(None), None, "nothing declared is none");
}

#[test]
fn only_a_login_github_gives_is_declared_as_one() {
    let declared = [
        "jsama",
        "j-sama-2",
        "-jsama",
        "jsama-",
        "j--sama",
        "jsama@example.com",
        "j sama",
        "",
        "a123456789b123456789c123456789d12345678",
        "a123456789b123456789c123456789d123456789",
    ]
    .map(|login| Declared::login(Some(login.into())).is_some());
    assert_eq!(
        declared,
        [
            true, true, false, false, false, false, false, false, true, false
        ],
        "only GitHub's letters, single inner hyphens and at most 39 are declared"
    );
}

#[test]
fn the_declared_one_is_matched_exactly_but_for_ascii_case() {
    let declared = Declared::login(Some("JSama".into())).expect("a sound login");
    let matched = [
        "jsama", "JSAMA", "JSama", "jsama2", "jsam", " jsama", "jſama",
    ]
    .map(|said| declared.is(said));
    assert_eq!(
        matched,
        [true, true, true, false, false, false, false],
        "only the same login, ASCII case aside, is the declared one"
    );
}

/// A folder of the test's own under the temporary folder, made empty.
fn folder(name: &str) -> PathBuf {
    static MADE: AtomicUsize = AtomicUsize::new(0);
    let made = MADE.fetch_add(1, Ordering::Relaxed);
    let folder =
        std::env::temp_dir().join(format!("receptor-say-{}-{made}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("the test's folder is made");
    folder
}

/// A stand-in for mycelium that adds each argument it's given to `given`,
/// a line each, then a blank line, and fails when told to by a `fail`
/// file beside it.
fn stand_in(folder: &Path) -> (PathBuf, PathBuf) {
    let program = folder.join("mycelium");
    let given = folder.join("given");
    let fail = folder.join("fail");
    std::fs::write(
        &program,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" '' >> {}\n[ ! -e {} ]\n",
            given.display(),
            fail.display()
        ),
    )
    .expect("the stand-in is written");
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700))
        .expect("the stand-in runs");
    (program, given)
}

/// Says `told` through `speaker`, running the stand-in again while another
/// test's child, forked while the stand-in was still open for writing,
/// holds it busy, which lasts only until that child execs.
fn said_through(speaker: &Speaker, told: &[Told]) -> Result<(), String> {
    let mut said = speaker.say(told, Duration::from_secs(60), 4242);
    for _ in 0..BUSY_TRIES {
        match &said {
            Err(why) if why.contains("os error 26") => {
                std::thread::yield_now();
                said = speaker.say(told, Duration::from_secs(60), 4242);
            }
            _ => break,
        }
    }
    said
}

#[test]
fn each_receptor_is_said_by_running_mycelium_with_its_arguments_alone() {
    let folder = folder("said");
    let (program, given) = stand_in(&folder);
    let speaker = Speaker::named(Some(program.into()), Some("work".into())).expect("sound");
    let told = [Told::off(Source::Mail), Told::off(Source::Calendar)];
    said_through(&speaker, &told).expect("the stand-in takes it");
    let expected: String = told
        .iter()
        .map(|&one| {
            let mut lines: Vec<String> =
                words(&speaker.arguments(one, Duration::from_secs(60), 4242))
                    .into_iter()
                    .map(str::to_owned)
                    .collect();
            lines.push(String::new());
            lines.join("\n") + "\n"
        })
        .collect();
    assert_eq!(
        std::fs::read_to_string(&given).expect("the stand-in wrote what it was given"),
        expected,
        "mycelium wasn't given each receptor's arguments alone"
    );
    let _ = std::fs::remove_dir_all(folder);
}

#[test]
fn a_receptor_said_with_more_words_is_given_them_after_its_own() {
    let folder = folder("with");
    let (program, given) = stand_in(&folder);
    let speaker = Speaker::named(Some(program.into()), Some("work".into())).expect("sound");
    let told = Told::off(Source::GitHub);
    let more = || vec![OsString::from("--owner"), OsString::from("kestrel-co")];
    let mut said = speaker.say_with(told, Duration::from_secs(60), 4242, more());
    for _ in 0..BUSY_TRIES {
        match &said {
            Err(why) if why.contains("os error 26") => {
                std::thread::yield_now();
                said = speaker.say_with(told, Duration::from_secs(60), 4242, more());
            }
            _ => break,
        }
    }
    said.expect("the stand-in takes it");
    let built = speaker.arguments_with(told, Duration::from_secs(60), 4242, more());
    assert_eq!(
        words(&built)[built.len() - 2..],
        ["--owner", "kestrel-co"],
        "the more words weren't put after the receptor's own"
    );
    let mut expected: Vec<&str> = words(&built);
    expected.push("");
    assert_eq!(
        std::fs::read_to_string(&given).expect("the stand-in wrote what it was given"),
        expected.join("\n") + "\n",
        "mycelium wasn't given what arguments_with builds"
    );
    let _ = std::fs::remove_dir_all(folder);
}

#[test]
fn saying_off_says_each_source_off_with_no_counts() {
    let folder = folder("off");
    let (program, given) = stand_in(&folder);
    let speaker = Speaker::named(Some(program.into()), Some("work".into())).expect("sound");
    let mut said = speaker.say_off(
        &[Source::Mail, Source::Calendar],
        Duration::from_secs(60),
        1,
    );
    for _ in 0..BUSY_TRIES {
        match &said {
            Err(why) if why.contains("os error 26") => {
                std::thread::yield_now();
                said = speaker.say_off(
                    &[Source::Mail, Source::Calendar],
                    Duration::from_secs(60),
                    1,
                );
            }
            _ => break,
        }
    }
    said.expect("the stand-in takes it");
    let given = std::fs::read_to_string(&given).expect("the stand-in wrote what it was given");
    let receptors: Vec<&str> = given.split("\n\n").filter(|one| !one.is_empty()).collect();
    assert_eq!(receptors.len(), 2, "both sources weren't said: {given}");
    for (one, source) in receptors.iter().zip(["mail", "calendar"]) {
        let lines: Vec<&str> = one.lines().collect();
        assert_eq!(
            [lines[4], lines[6], lines[8], lines[10], lines[12]],
            [source, "off", "0", "0", "0"],
            "{source} wasn't said off with no counts"
        );
    }
    let _ = std::fs::remove_dir_all(folder);
}

#[test]
fn a_mycelium_that_refuses_is_said_as_failing_and_stops_there() {
    let folder = folder("refused");
    let (program, given) = stand_in(&folder);
    std::fs::write(folder.join("fail"), "").expect("the stand-in is told to fail");
    let speaker = Speaker::named(Some(program.into()), Some("work".into())).expect("sound");
    let refused = said_through(
        &speaker,
        &[Told::off(Source::Mail), Told::off(Source::Calendar)],
    );
    assert!(refused.is_err(), "a refusal wasn't said: {refused:?}");
    let given = std::fs::read_to_string(&given).expect("the stand-in wrote what it was given");
    assert!(
        !given.contains("calendar"),
        "the receptor after a refusal was said anyway: {given}"
    );
    let _ = std::fs::remove_dir_all(folder);
}
