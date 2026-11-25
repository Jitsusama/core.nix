//! The sign-in's tests, beside it: the keyring's program and the file,
//! the rest after a run-over or a lock, and the sign-in that stands.

use std::cell::Cell;

use super::*;

/// A sign-in for `token`, as the harness keeps one.
fn signed_in(token: &str) -> String {
    format!(r#"{{"token": {{"accessToken": "{token}", "cookie": "d", "teamId": "T1"}}}}"#)
}

/// A folder of the test's own, holding the file with `file` and a
/// keyring reader that runs `reader`.
fn kept_in(test: &str, file: &str, reader: &str) -> (PathBuf, PathBuf) {
    let folder = std::env::temp_dir().join(format!("slack-account-{test}"));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("the test's folder is made");
    let path = folder.join("credentials.json");
    std::fs::write(&path, file).expect("the file is written");
    let keyring = folder.join("keyring");
    // Written by a shell of its own, never by this process: a test
    // forking beside this one would hold the reader open for writing
    // until its child ran, and running the reader meanwhile fails as a
    // text file busy, so the file is read over it (seen once in
    // google's like test on a builder, 2026-11-14).
    let written = std::process::Command::new("/bin/sh")
        .args(["-c", r#"printf '%s\n' "$2" > "$1" && chmod 755 "$1""#, "sh"])
        .arg(&keyring)
        .arg(format!("#!/bin/sh\n{reader}"))
        .status()
        .expect("the reader's shell runs");
    assert!(written.success(), "the reader wasn't written");
    (keyring, path)
}

#[test]
fn the_keyrings_sign_in_is_read_over_the_file() {
    let reader = format!("cat <<'END'\n{}\nEND", signed_in("xoxc-keyring"));
    let (keyring, path) = kept_in("keyring", &signed_in("xoxc-file"), &reader);
    let account = Account::kept(Some(&keyring), &path).expect("the keyring's is read");
    assert_eq!(
        account.token, "xoxc-keyring",
        "the file was read over the keyring"
    );
}

#[test]
fn the_file_is_read_while_the_keyring_holds_no_sign_in() {
    for (test, reader) in [("refused", "exit 1"), ("empty", "exit 0")] {
        let (keyring, path) = kept_in(test, &signed_in("xoxc-file"), reader);
        let account = Account::kept(Some(&keyring), &path).expect("the file's is read");
        assert_eq!(account.token, "xoxc-file", "{test}: the file wasn't read");
    }
    let (_, path) = kept_in("unreadable", &signed_in("xoxc-file"), "");
    let nowhere = Path::new("/nonexistent/keyring");
    assert_eq!(
        Account::kept(Some(nowhere), &path).map(|account| account.token),
        Some("xoxc-file".to_owned()),
        "a keyring that can't be read hid the file"
    );
    let (keyring, _) = kept_in("neither", "", "exit 1");
    let missing = Path::new("/nonexistent/credentials.json");
    assert!(
        Account::kept(Some(&keyring), missing).is_none(),
        "a sign-in from nowhere"
    );
}

#[test]
fn a_keyring_that_never_answers_runs_over_and_isnt_read_past() {
    let patience = Duration::from_millis(300);
    let (keyring, path) = kept_in("hangs", &signed_in("xoxc-file"), "exec sleep 60");
    let (said, heard) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        // The case may have given up, and then nobody listens.
        let _ = said.send(Account::kept_within(Some(&keyring), &path, patience));
    });
    let signed = heard
        .recv_timeout(patience + Duration::from_secs(5))
        .expect("a keyring that never answers is never waited on for good");
    assert!(
        matches!(signed, Signed::RanOver),
        "a keyring that never answers ran over, not {signed:?}"
    );
}

#[test]
fn a_sign_in_that_ran_over_isnt_asked_for_again_until_its_rest_is_up() {
    let at = std::sync::Mutex::new(None);
    let asked = Cell::new(0);
    let ask = |answer: Signed| {
        asked.set(asked.get() + 1);
        answer
    };
    let hour = Duration::from_secs(3600);
    let first = rested(&at, hour, || ask(Signed::RanOver));
    assert!(
        matches!(first, Signed::RanOver),
        "it ran over, not {first:?}"
    );
    let resting = rested(&at, hour, || ask(Signed::Out));
    assert!(
        matches!(resting, Signed::Resting),
        "it rests, not {resting:?}"
    );
    assert_eq!(asked.get(), 1, "the keyring was asked during its rest");
    let after = rested(&at, Duration::ZERO, || ask(Signed::Out));
    assert!(
        matches!(after, Signed::Out),
        "asked once rested, not {after:?}"
    );
    let again = rested(&at, hour, || ask(Signed::Out));
    assert!(
        matches!(again, Signed::Out),
        "an answer ends the rest, not {again:?}"
    );
    assert_eq!(asked.get(), 3, "asked once its rest was up, and after");
}

#[test]
fn a_keyring_running_over_is_asked_once_however_many_want_it() {
    static AT: std::sync::Mutex<Option<Instant>> = std::sync::Mutex::new(None);
    static OTHERS_ASKED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let (asking, asked) = std::sync::mpsc::channel();
    let (release, released) = std::sync::mpsc::channel::<()>();
    let first = std::thread::spawn(move || {
        rested(&AT, REST, || {
            // The case may have given up, and then nobody listens.
            let _ = asking.send(());
            // Held until the other caller has come, or a minute.
            let _ = released.recv_timeout(Duration::from_mins(1));
            Signed::RanOver
        })
    });
    asked
        .recv_timeout(Duration::from_secs(5))
        .expect("the first caller started asking");
    let (said, heard) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let signed = rested(&AT, REST, || {
            OTHERS_ASKED.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Signed::Out
        });
        // The case may have given up, and then nobody listens.
        let _ = said.send(signed);
    });
    // Long enough for the other caller to ask, were it let.
    std::thread::sleep(Duration::from_millis(300));
    let _ = release.send(());
    let _ = first.join();
    let other = heard.recv_timeout(Duration::from_secs(5));
    assert_eq!(
        OTHERS_ASKED.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "a second caller asked the keyring while the first ran over: {other:?}"
    );
    assert!(
        matches!(other, Ok(Signed::Resting)),
        "the second caller rests once the first ran over, not {other:?}"
    );
}

/// A sign-in to the stand-in workspace, its token `token`.
fn account(token: &str) -> Account {
    Account {
        token: token.to_owned(),
        cookie: "d".to_owned(),
        team: "T1".to_owned(),
        user: String::new(),
    }
}

#[test]
fn a_sign_in_read_stands_while_the_keyring_runs_over_or_rests() {
    let token = |kept: Option<Account>| kept.map(|account| account.token);
    let read = Instant::now();
    let mut last = LastSignIn::new();
    let none = token(last.heard(Signed::RanOver, read));
    assert_eq!(none, None, "none read yet");
    let first = token(last.heard(Signed::In(account("xoxc-keyring")), read));
    assert_eq!(first.as_deref(), Some("xoxc-keyring"), "what's read");
    let over = token(last.heard(Signed::RanOver, read + Duration::from_secs(30)));
    assert_eq!(
        over.as_deref(),
        Some("xoxc-keyring"),
        "it stands as it runs over"
    );
    let resting = token(last.heard(Signed::Resting, read + Duration::from_mins(3)));
    assert_eq!(resting.as_deref(), Some("xoxc-keyring"), "and as it rests");
    let out = token(last.heard(Signed::Out, read + Duration::from_mins(4)));
    assert_eq!(out, None, "signed out is");
    let after = token(last.heard(Signed::Resting, read + Duration::from_mins(4)));
    assert_eq!(after, None, "and nothing stands once Joel signed out");
}

#[test]
fn a_sign_in_read_stands_no_longer_than_its_bound() {
    let read = Instant::now();
    let mut last = LastSignIn::new();
    assert!(!last.lapsed(read), "nothing read hasn't lapsed");
    let _ = last.heard(Signed::In(account("xoxc-keyring")), read);
    let late = read + STANDS;
    assert!(
        !last.lapsed(read + STANDS / 2) && last.lapsed(late),
        "a sign-in lapses at its bound, not before"
    );
    assert!(
        !last.stands(late),
        "a sign-in read {STANDS:?} ago stands no longer"
    );
    let kept = last
        .heard(Signed::Resting, late)
        .map(|account| account.token);
    assert_eq!(
        kept, None,
        "a locked keyring ends Slack once its bound is up"
    );
    assert!(
        last.heard(Signed::Resting, read).is_none(),
        "and what was let go isn't brought back"
    );
    assert!(!last.lapsed(late), "and is said to lapse once");
    assert!(
        no_longer_stands().contains("read 15 minutes ago no longer stands"),
        "the lapse says the bound: {}",
        no_longer_stands()
    );
}

#[test]
fn a_run_over_says_whether_a_sign_in_stands_meanwhile() {
    let stands = ran_over(true);
    assert!(
        stands.ends_with("and the sign-in last read stands meanwhile"),
        "one read stands: {stands}"
    );
    let none = ran_over(false);
    assert!(
        none.ends_with("with no sign-in standing, Slack reads as signed out till then")
            && !none.contains("stands"),
        "none read, so none is said to stand: {none}"
    );
}

#[test]
fn a_locked_keyring_isnt_read_past_and_rests() {
    let (keyring, path) = kept_in("locked", &signed_in("xoxc-file"), "exit 75");
    let signed = Account::kept_within(Some(&keyring), &path, Duration::from_secs(5));
    assert!(
        matches!(signed, Signed::Locked),
        "a locked keyring is said, not read past to the file: {signed:?}"
    );
    let at = std::sync::Mutex::new(None);
    let _ = rested(&at, REST, || Signed::Locked);
    let again = rested(&at, REST, || Signed::Out);
    assert!(
        matches!(again, Signed::Resting),
        "a locked keyring rests, not {again:?}"
    );
    let said = locked(true, REST);
    assert!(
        said.contains("is locked")
            && said.contains("isn't asked again for 3 minutes")
            && said.ends_with("the sign-in last read stands meanwhile"),
        "it says why, the rest and what stands: {said}"
    );
}

#[test]
fn a_sign_in_read_stands_while_the_keyring_is_locked() {
    let read = Instant::now();
    let mut last = LastSignIn::new();
    assert!(last.heard(Signed::Locked, read).is_none(), "none read yet");
    let _ = last.heard(Signed::In(account("xoxc-keyring")), read);
    let kept = last.heard(Signed::Locked, read + Duration::from_secs(30));
    assert_eq!(
        kept.map(|account| account.token).as_deref(),
        Some("xoxc-keyring"),
        "a dismissed unlock prompt doesn't end the sign-in"
    );
    assert!(
        last.heard(Signed::Locked, read + STANDS).is_none(),
        "nor stands past its bound"
    );
}

/// A keyring reader in `test`'s folder that prints a sign-in the first
/// time it's run and runs `after` every time since.
fn answers_once_then(test: &str, after: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("slack-account-{test}"));
    let asked = folder.join("asked");
    let reader = format!(
        "if [ -e '{asked}' ]; then {after}; fi\n: > '{asked}'\ncat <<'END'\n{}\nEND",
        signed_in("xoxc-keyring"),
        asked = asked.display(),
    );
    kept_in(test, "", &reader).0
}

/// Long past any stand-in keyring's answer on a loaded builder, and the
/// whole wait of one that hangs.
const READ_PATIENCE: Duration = Duration::from_secs(3);

/// `read_from` with a rest and a last sign-in of its own.
struct Reading {
    keyring: PathBuf,
    at: std::sync::Mutex<Option<Instant>>,
    last: std::sync::Mutex<LastSignIn>,
}

impl Reading {
    fn of(keyring: PathBuf) -> Reading {
        Reading {
            keyring,
            at: std::sync::Mutex::new(None),
            last: std::sync::Mutex::new(LastSignIn::new()),
        }
    }

    /// The token read, and the line said.
    fn read(&self) -> (Option<String>, Option<String>) {
        let missing = Path::new("/nonexistent/credentials.json");
        let (account, said) = Account::read_from(
            Some(&self.keyring),
            missing,
            &self.at,
            &self.last,
            READ_PATIENCE,
        );
        (account.map(|account| account.token), said)
    }
}

#[test]
fn read_keeps_its_last_sign_in_and_says_so_while_the_keyring_runs_over_or_is_locked() {
    for (test, after, line) in [
        ("read-runs-over", "exec sleep 60", "ran past"),
        ("read-locked", "exit 75", "is locked"),
    ] {
        let reading = Reading::of(answers_once_then(test, after));
        let (first, said) = reading.read();
        assert_eq!(first.as_deref(), Some("xoxc-keyring"), "{test}");
        assert_eq!(said, None, "{test}: an answer says nothing");
        let (second, said) = reading.read();
        assert_eq!(
            second.as_deref(),
            Some("xoxc-keyring"),
            "{test}: the sign-in last read stands"
        );
        let said = said.unwrap_or_default();
        assert!(
            said.contains(line) && said.ends_with("the sign-in last read stands meanwhile"),
            "{test}: the journal says how it answered and what stands: {said}"
        );
        let (third, said) = reading.read();
        assert_eq!(
            third.as_deref(),
            Some("xoxc-keyring"),
            "{test}: and stands as it rests"
        );
        assert_eq!(said, None, "{test}: a rest is said once, as it begins");
    }
}

#[test]
fn read_lets_a_sign_in_go_at_once_when_the_keyring_no_longer_holds_one() {
    let reading = Reading::of(answers_once_then("read-signed-out", "exit 1"));
    let (first, _) = reading.read();
    assert_eq!(first.as_deref(), Some("xoxc-keyring"), "what's read");
    let (out, said) = reading.read();
    assert_eq!(out, None, "signing out is taken at once");
    assert_eq!(said, None, "and isn't blamed on a locked keyring");
    let resting = rested(&reading.at, REST, || Signed::Out);
    assert!(
        matches!(resting, Signed::Out),
        "signing out doesn't rest the keyring, so a sign-in is read next round: {resting:?}"
    );
}

#[test]
fn read_says_once_as_a_kept_sign_in_lapses() {
    let reading = Reading::of(kept_in("read-lapses", "", "exit 75").0);
    let Some(then) = Instant::now().checked_sub(STANDS) else {
        // A clock younger than STANDS can't hold a read that old.
        return;
    };
    reading.last.lock().expect("the case's own lock").last = Some((account("xoxc-keyring"), then));
    let (lapsed, said) = reading.read();
    assert_eq!(lapsed, None, "a sign-in past its bound is let go");
    assert_eq!(said, Some(no_longer_stands()), "and that is said");
    let (_, again) = reading.read();
    assert_eq!(again, None, "once");
}
