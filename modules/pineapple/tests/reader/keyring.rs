//! pineapple-slack with a keyring that hangs, as one waiting on its
//! unlock does: stops are held back only the sign-in's patience and rest,
//! never for good, and a keyring that hangs or is locked after it answered
//! leaves Slack read with the sign-in last read.

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;

use super::{JOEL, PACE, PATIENCE, decisions, each, home, stand_in, until_decided};

/// The reader started in `home` against `origin`, its keyring a stand-in
/// secret-tool running `script`, which `{sign_in}` in prints the sign-in
/// of, and its journal written to home/journal.
pub(super) fn reading_with_keyring(home: &Path, origin: &str, script: &str) -> Child {
    let sign_in = json!({"token": {
        "accessToken": "xoxc-stand-in", "cookie": "stand-in", "teamId": "T0TEAM0001",
        "userId": JOEL,
    }});
    let printed = format!("printf '%s\\n' '{sign_in}'");
    let keyring = home.join("keyring");
    // Written by a shell of its own, never by this process: a case forking
    // beside this one would hold it open for writing until its child ran,
    // and running it meanwhile fails as a text file busy.
    let written = Command::new("/bin/sh")
        .args(["-c", r#"printf '%s\n' "$2" > "$1" && chmod 755 "$1""#, "sh"])
        .arg(&keyring)
        .arg(format!(
            "#!/bin/sh\n{}",
            script.replace("{sign_in}", &printed)
        ))
        .status()
        .expect("the keyring's shell runs");
    assert!(written.success(), "the keyring wasn't written");
    let journal = std::fs::File::create(home.join("journal")).expect("the journal");
    Command::new(env!("CARGO_BIN_EXE_pineapple-slack"))
        .args(["run", "--origin", origin, "--every", PACE])
        .env_remove("SAFE_WORD_WAITING")
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_STATE_HOME", home.join("state"))
        .env("SIGN_IN_KEYRING", &keyring)
        .stdout(Stdio::null())
        .stderr(journal)
        .spawn()
        .expect("the reader starts")
}

#[test]
fn a_keyring_that_never_answers_holds_the_stops_back_only_its_patience_and_rest() {
    let (origin, _) = stand_in();
    let home = home();
    // Asked first, it never answers, as one waiting on a locked keyring's
    // unlock doesn't; asked again, it prints the sign-in.
    let asked = home.join("keyring-asked");
    let script = format!(
        "if [ -e '{asked}' ]; then {{sign_in}}; else : > '{asked}'; exec sleep 600; fi",
        asked = asked.display()
    );
    let mut reader = reading_with_keyring(&home, &origin, &script);
    let began = Instant::now();
    until_decided(&home, 4);
    let took = began.elapsed();
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    assert!(asked.exists(), "the keyring was asked");
    // At a tenth of a second a round, twenty rounds' patience and rest.
    assert!(
        took >= Duration::from_secs(4),
        "the stops waited out the keyring's patience and rest: {took:?}"
    );
    assert_eq!(
        each(&decisions(&home), "verdict"),
        ["stop", "stop", "stop-asked", "stop"],
        "every stop said while the keyring hung is heard"
    );
    let said = std::fs::read_to_string(home.join("journal")).expect("the journal");
    // Twenty rounds' patience; the rest, at this pace, under a minute.
    assert!(
        said.contains("Slack's sign-in ran past 2 seconds and was stopped")
            && said.contains("restart pineapple-slack to ask again sooner"),
        "the journal says the keyring ran over, with the patience used: {said}"
    );
}

#[test]
fn a_keyring_locked_after_answering_is_asked_once_a_rest_while_slack_is_read() {
    let (origin, requests) = stand_in();
    let home = home();
    // Asked first, it prints the sign-in; asked after, it ends as the
    // keyring program does while the sign-in is held locked. Each ask
    // leaves a line.
    let asks = home.join("keyring-asks");
    let script = format!(
        "if [ -e '{asks}' ]; then echo >> '{asks}'; exit 75; else echo > '{asks}'; {{sign_in}}; fi",
        asks = asks.display()
    );
    let mut reader = reading_with_keyring(&home, &origin, &script);
    let searches = || {
        let asked = requests.lock().expect("what was asked");
        asked
            .iter()
            .filter(|line| line.contains("search.messages"))
            .count()
    };
    let said = || std::fs::read_to_string(home.join("journal")).unwrap_or_default();
    let began = Instant::now();
    while !said().contains("is locked") && began.elapsed() < PATIENCE {
        thread::sleep(Duration::from_millis(20));
    }
    let at_locked = searches();
    // Half the two seconds' rest at this pace, ten rounds.
    thread::sleep(Duration::from_secs(1));
    let during_rest = searches() - at_locked;
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    let asked = std::fs::read_to_string(&asks)
        .unwrap_or_default()
        .lines()
        .count();
    assert!(
        said().contains(
            "the keyring holding the Slack sign-in is locked, its unlock prompt dismissed or not \
             shown, so it isn't asked again for 1 minutes, and the sign-in last read stands"
        ),
        "the journal says it's locked and the last sign-in stands: {}",
        said()
    );
    assert_eq!(
        asked, 2,
        "the keyring is asked once, then rests, not prompted every round"
    );
    assert!(
        during_rest >= 3,
        "Slack is still searched while the keyring rests, {during_rest} times in a second"
    );
}

#[test]
fn a_keyring_that_hangs_after_answering_leaves_slack_read_with_the_last_sign_in() {
    let (origin, requests) = stand_in();
    let home = home();
    // Asked first, it prints the sign-in; asked second, it never answers;
    // asked after, it prints it again.
    let (once, hung) = (home.join("keyring-once"), home.join("keyring-hung"));
    let script = format!(
        "if [ -e '{hung}' ] || [ ! -e '{once}' ]; then : > '{once}'; {{sign_in}}; \
         else : > '{hung}'; exec sleep 600; fi",
        once = once.display(),
        hung = hung.display()
    );
    let mut reader = reading_with_keyring(&home, &origin, &script);
    let searches = || {
        let asked = requests.lock().expect("what was asked");
        asked
            .iter()
            .filter(|line| line.contains("search.messages"))
            .count()
    };
    let said = || std::fs::read_to_string(home.join("journal")).unwrap_or_default();
    let began = Instant::now();
    while !said().contains("so the one last read is used") && began.elapsed() < PATIENCE {
        thread::sleep(Duration::from_millis(20));
    }
    let at_run_over = searches();
    // Half the two seconds' rest at this pace: searches only with the
    // sign-in last read.
    thread::sleep(Duration::from_secs(1));
    let during_rest = searches() - at_run_over;
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    assert!(hung.exists(), "the keyring hung once");
    assert!(
        said().contains("Slack's sign-in ran past 2 seconds and was stopped, so the one last read"),
        "the journal says the last sign-in is used: {}",
        said()
    );
    assert!(
        during_rest >= 3,
        "Slack is still searched while the keyring rests, {during_rest} times in a second"
    );
}
