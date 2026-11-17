//! The engine on a clock the test sets, with a stand-in releaser, so a
//! yes can land in the moment between an ask's ten minutes and the tidy
//! that would mark it ended, which the daemon, waking at the end itself,
//! never leaves open for a test to reach.

use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};

use secret_asks::ask::{Ask, WAITS_MS};
use secret_asks::engine::{self, Engine, Releaser, Secret, Setting};
use secret_asks::words::Words;
use serde_json::{Value, json};

/// The item each ask names.
const ITEM: &str = "op://Private/Release Test/password";
/// The launcher's vocabulary, which the engine says its rows in.
const VOCABULARY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../quickshell/launcher/words.json"
);
/// What the stand-in releaser would read, were it ever asked.
const VALUE: &str = "this-is-not-a-secret";

/// A releaser that serves every ask and counts each value it reads.
struct Counting(Arc<AtomicUsize>);

impl Releaser for Counting {
    fn refused(&self, _: &Ask) -> Option<String> {
        None
    }

    fn locked(&self) -> bool {
        false
    }

    fn read_value(&self, _: &Ask) -> Result<Secret, String> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(Secret::new(VALUE.to_owned()))
    }

    fn command(&self, _: &Ask, value: &Secret) -> Result<Command, String> {
        let mut command = Command::new("sh");
        command.args(["-c", "printf %s \"$1\"", "sh", value.reveal()]);
        Ok(command)
    }
}

/// A folder of the test's own, emptied first.
fn folder(name: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("secret-asks-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(folder.join("tree")).expect("the folder is made");
    folder
}

/// Puts up an ask for the item, to run in the folder's tree, and answers
/// its id.
fn put_up(folder: &Path) -> String {
    let ran = Command::new(env!("CARGO_BIN_EXE_secret-asks"))
        .args(["ask", "--item", ITEM, "--why", "To try the line.", "--asks"])
        .arg(folder.join("asks"))
        .arg("--tree")
        .arg(folder.join("tree"))
        .output()
        .expect("the ask is put up");
    assert!(ran.status.success(), "the ask wasn't put up: {ran:?}");
    String::from_utf8_lossy(&ran.stdout).trim().to_owned()
}

/// Every line the journal stand-in was told.
fn told(journal: &UnixDatagram) -> String {
    let mut said = String::new();
    let mut line = [0_u8; 4096];
    while let Ok(length) = journal.recv(&mut line) {
        said.push_str(&String::from_utf8_lossy(&line[..length]));
    }
    said
}

#[test]
fn a_yes_a_moment_past_an_ask_s_ten_minutes_is_refused_before_any_tidy() {
    let folder = folder("clock");
    let journal =
        UnixDatagram::bind(folder.join("journal.sock")).expect("the journal stand-in binds");
    journal
        .set_nonblocking(true)
        .expect("the journal stand-in reads without waiting");
    let text = std::fs::read_to_string(VOCABULARY).expect("the vocabulary reads");
    let read = Arc::new(AtomicUsize::new(0));
    let now = Arc::new(AtomicI64::new(engine::now_ms()));
    let (changed, _heard) = mpsc::channel();
    let clock = Arc::clone(&now);
    let engine = Arc::new(
        Engine::new(
            Setting {
                asks: folder.join("asks"),
                git: PathBuf::from("git"),
                home: folder.display().to_string(),
                words: Words::from_vocabulary(&text).expect("the vocabulary is JSON"),
                journal: folder.join("journal.sock"),
                least_shown_ms: None,
                says_commits: false,
                forges: Vec::new(),
                gh: None,
            },
            Box::new(Counting(Arc::clone(&read))),
            changed,
        )
        .with_clock(move || clock.load(Ordering::SeqCst)),
    );
    let id = put_up(&folder);
    // Now on the clock is after the ask was put up, as it is for a daemon.
    now.store(engine::now_ms(), Ordering::SeqCst);
    engine.look();
    let (rows, _) = engine.rows();
    let drawn = rows
        .iter()
        .find(|row| row["thing"] == format!("ask:{id}"))
        .unwrap_or_else(|| panic!("the ask isn't drawn, among {rows:?}"));
    assert_eq!(drawn["state"], "waiting", "the ask doesn't wait: {drawn}");
    let mut yes = drawn["acts"][0]["ask"].clone();
    assert_eq!(
        yes["ask"], "release",
        "the row's first act isn't a yes: {drawn}"
    );
    yes["id"] = json!(7);

    let file = folder.join("asks").join(format!("{id}.ask.json"));
    let said: Value = serde_json::from_str(&std::fs::read_to_string(file).expect("the ask reads"))
        .expect("the ask is JSON");
    let asked = said["asked_ms"].as_i64().expect("the ask says when");
    now.store(asked + WAITS_MS + 1, Ordering::SeqCst);
    let answer = engine.release(&yes);

    assert_eq!(
        answer["error"]["code"], "refused",
        "a yes a moment past ten minutes: {answer}"
    );
    assert_eq!(read.load(Ordering::SeqCst), 0, "the value was read");
    let answered = std::fs::read_to_string(folder.join("asks").join(format!("{id}.answer.json")))
        .unwrap_or_default();
    assert!(
        !answered.contains(VALUE),
        "the value reached the answer: {answered}"
    );
    let told = told(&journal);
    assert!(
        !told.contains(VALUE),
        "the journal was told the value: {told}"
    );
}
