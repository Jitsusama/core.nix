//! Asking the running shell what the launcher shows, and to do what it
//! offers, through Quickshell's `qs ipc call launcher show`, `names` and
//! `doing`, as pi-ask's tool asks it for the catalogue.

use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

/// How the launcher joins a path's steps (Launcher.qml's catalogueStep), a
/// letter no name holds.
pub const STEP: &str = "\u{203a}";

/// How long the shell has to answer before it's taken as silent: a walk
/// of a place takes milliseconds, and qs itself a hundred or so.
const PATIENCE: Duration = Duration::from_secs(5);

/// How often a call still running is looked at while it's waited for.
const LOOK_AGAIN: Duration = Duration::from_millis(10);

/// What the shell answered.
#[derive(Debug, PartialEq)]
pub enum Answer {
    /// What a place or a row shows, as launcher/Showing.js shapes it, or
    /// what came of doing it.
    Shown(Value),
    /// A step named nothing: the step, the names it might have meant, the
    /// names of the steps before it, and whether what it was looked among
    /// is still being read, as a joined network's Speed is made only once
    /// wifid has said its detail.
    Missing {
        step: String,
        near: Vec<String>,
        within: Vec<String>,
        reading: bool,
    },
}

impl Answer {
    /// Whether the shell said what it shows, or looked among, is still
    /// being read.
    fn reading(&self) -> bool {
        match self {
            Answer::Shown(shown) => still_reading(shown),
            Answer::Missing { reading, .. } => *reading,
        }
    }
}

/// How long what a place shows may still be reading once it's first asked:
/// a provider asks its daemon only when it's seen, as Machine does, so a
/// place asked cold is read only from the terminal's first ask on. It's
/// longer than a Wi-Fi card's look across every band takes, which was 7.3 s
/// in the desktop's VM, since Wi-Fi asked cold lists only what a look it
/// starts hears; what's read sooner is said sooner, and a provider says
/// it's reading only while an answer is coming.
const SETTLING: Duration = Duration::from_secs(10);

/// How long between asking again while it settles.
const ASK_AGAIN: Duration = Duration::from_millis(150);

/// Asks as `show` does, again while the shell says what it shows is still
/// being read (`reading`, a provider's daemon not yet answered), until it's
/// read or `SETTLING` has passed, when it says what it has. A fact that's
/// simply unknown never waits, since only the shell knows whether it's coming,
/// and a step missed among rows still being read is asked again the same way.
pub fn settled(qs: &str, steps: &[String], drawers: bool) -> Result<Answer, String> {
    let began = Instant::now();
    loop {
        let answer = show(qs, steps, drawers)?;
        if !answer.reading() || began.elapsed() >= SETTLING {
            return Ok(answer);
        }
        thread::sleep(ASK_AGAIN);
    }
}

/// Whether the shell said what it shows is still being read.
pub fn still_reading(shown: &Value) -> bool {
    shown.get("reading").and_then(Value::as_bool) == Some(true)
}

/// Asks the shell through `qs` what the steps show, with each row's drawer
/// where `drawers` asks, since the shell builds them only then. An error is
/// the shell not answering, with what qs said where it said anything.
pub fn show(qs: &str, steps: &[String], drawers: bool) -> Result<Answer, String> {
    call(qs, if drawers { "drawers" } else { "show" }, &path(steps))
}

/// Asks the shell what the steps hold for their names alone, as completion
/// wants them, so nothing is read for figures a Tab never shows.
pub fn names(qs: &str, steps: &[String]) -> Result<Answer, String> {
    call(qs, "names", &path(steps))
}

/// Asks the shell through `qs` to do what the steps name, with the words
/// an act asks for, and past its question once the terminal's sure. It
/// answers with what it asks first (`asks`, with the steady ids of the
/// rows it asked about, `path`), the words an act takes (`takes`), or that
/// it's done. Given a `path`, the shell walks those ids rather than the
/// names, so a y does the row it was asked about even where rows moved
/// since, or nothing.
pub fn doing(
    qs: &str,
    steps: &[String],
    path: &[String],
    words: &str,
    sure: bool,
) -> Result<Answer, String> {
    call(qs, "doing", &asked(steps, path, words, sure))
}

/// What `doing` is given, as JSON, which is never empty as qs needs.
pub fn asked(steps: &[String], path: &[String], words: &str, sure: bool) -> String {
    serde_json::json!({ "steps": steps, "path": path, "words": words, "sure": sure }).to_string()
}

fn call(qs: &str, function: &str, argument: &str) -> Result<Answer, String> {
    let mut child = Command::new(qs)
        // After `--`, since `show` is also qs ipc's own command, which
        // takes no argument, and qs reads the call as it otherwise.
        .args(["ipc", "call", "--", "launcher", function, argument])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let began = Instant::now();
    // The answer and qs's own words are each read as they come, so one
    // longer than a pipe holds never stops qs writing it.
    let stdout = child.stdout.take().ok_or_else(String::new)?;
    let stderr = child.stderr.take().ok_or_else(String::new)?;
    let reading = read_apart(stdout);
    let hearing = read_apart(stderr);
    loop {
        if child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_some()
        {
            break;
        }
        if began.elapsed() > PATIENCE {
            let _ = child.kill();
            let _ = child.wait();
            return Err(String::new());
        }
        thread::sleep(LOOK_AGAIN);
    }
    let printed = reading.join().unwrap_or_default();
    let said = hearing.join().unwrap_or_default();
    answer_of(&printed).ok_or_else(|| {
        plain(if said.trim().is_empty() {
            &printed
        } else {
            &said
        })
    })
}

/// Reads all a pipe says, on a thread of its own.
fn read_apart(mut pipe: impl Read + Send + 'static) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut said = String::new();
        let _ = pipe.read_to_string(&mut said);
        said
    })
}

/// The path the shell is asked, each step after the mark that joins them:
/// qs takes no empty argument, so home is the mark alone.
pub fn path(steps: &[String]) -> String {
    if steps.is_empty() {
        return STEP.to_string();
    }
    let mut path = String::new();
    for step in steps {
        path.push_str(STEP);
        path.push_str(step);
    }
    path
}

/// What the shell's printed answer says, or None where it isn't one:
/// Quickshell's qs prints a failed call's error on stdout, in colour, and
/// still exits 0, so anything that isn't a JSON object is the shell not
/// answering.
pub fn answer_of(printed: &str) -> Option<Answer> {
    let value: Value = serde_json::from_str(printed.trim()).ok()?;
    if !value.is_object() {
        return None;
    }
    if let Some(step) = value.get("missing").and_then(Value::as_str) {
        return Some(Answer::Missing {
            step: step.to_string(),
            near: strings(value.get("near")),
            within: strings(value.get("in")),
            reading: value.get("reading").and_then(Value::as_bool) == Some(true),
        });
    }
    Some(Answer::Shown(value))
}

/// A JSON list's strings, or none.
pub fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|all| {
            all.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// What qs said, its colours taken away and its lines joined.
fn plain(said: &str) -> String {
    let mut out = String::new();
    let mut escaped = false;
    for letter in said.chars() {
        if escaped {
            escaped = !letter.is_ascii_alphabetic();
        } else if letter == '\u{1b}' {
            escaped = true;
        } else {
            out.push(letter);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What a stand-in's process takes to start and answer, at most.
    const LEEWAY: Duration = Duration::from_millis(500);

    #[test]
    fn a_place_still_reading_is_asked_again_only_until_its_time_is_up() {
        let began = Instant::now();
        let answer = settled(&stand_in::qs("reading"), &[], false).expect("the stand-in answers");
        let took = began.elapsed();
        assert!(matches!(&answer, Answer::Shown(shown) if still_reading(shown)));
        assert!(took >= SETTLING, "it gave up after {took:?}");
        assert!(
            took < SETTLING + ASK_AGAIN + LEEWAY,
            "it asked for {took:?}"
        );
    }

    #[test]
    fn a_step_missed_among_rows_still_reading_is_asked_again_until_its_time_is_up() {
        let began = Instant::now();
        let answer =
            settled(&stand_in::qs("missed-reading"), &[], false).expect("the stand-in answers");
        let took = began.elapsed();
        assert!(matches!(answer, Answer::Missing { reading: true, .. }));
        assert!(took >= SETTLING, "it gave up after {took:?}");
        assert!(
            took < SETTLING + ASK_AGAIN + LEEWAY,
            "it asked for {took:?}"
        );
    }

    #[test]
    fn a_step_missed_among_rows_read_is_said_at_once() {
        let began = Instant::now();
        let answer = settled(&stand_in::qs("missed"), &[], false).expect("the stand-in answers");
        assert!(matches!(answer, Answer::Missing { reading: false, .. }));
        assert!(
            began.elapsed() < ASK_AGAIN + LEEWAY,
            "it waited {:?}",
            began.elapsed()
        );
    }

    #[test]
    fn a_place_read_is_said_from_its_first_answer() {
        let began = Instant::now();
        let answer = settled(&stand_in::qs("read"), &[], false).expect("the stand-in answers");
        assert!(matches!(&answer, Answer::Shown(shown) if !still_reading(shown)));
        assert!(
            began.elapsed() < ASK_AGAIN + LEEWAY,
            "it waited {:?}",
            began.elapsed()
        );
    }

    #[test]
    fn a_place_is_still_reading_only_where_the_shell_says_so() {
        let read = serde_json::json!({"reading": false,
            "rows": [{"name": "Disk", "side": null}]});
        assert!(!still_reading(&read));
        assert!(!still_reading(&serde_json::json!({"rows": []})));
        assert!(still_reading(
            &serde_json::json!({"reading": true, "rows": []})
        ));
    }

    #[test]
    fn home_is_asked_as_the_mark_alone_and_a_place_after_it() {
        assert_eq!(path(&[]), "\u{203a}");
        assert_eq!(
            path(&["machine".to_string(), "disk".to_string()]),
            "\u{203a}machine\u{203a}disk"
        );
    }

    #[test]
    fn what_to_do_is_asked_as_json() {
        let steps = ["machine".to_string(), "power".to_string()];
        assert_eq!(
            asked(&steps, &[], "", false),
            r#"{"path":[],"steps":["machine","power"],"sure":false,"words":""}"#
        );
        let path = [
            "category:computer".to_string(),
            "category:power".to_string(),
        ];
        assert_eq!(
            asked(&steps, &path, "", true),
            concat!(
                r#"{"path":["category:computer","category:power"],"#,
                r#""steps":["machine","power"],"sure":true,"words":""}"#
            )
        );
    }

    #[test]
    fn a_place_is_shown() {
        let answer = answer_of(r#"{"in":["Machine"],"path":["category:computer"],"rows":[]}"#);
        assert!(matches!(answer, Some(Answer::Shown(_))));
    }

    #[test]
    fn a_step_naming_nothing_says_what_it_might_have_meant() {
        let answer = answer_of(r#"{"missing":"mahcine","near":["Machine"],"in":[]}"#);
        assert_eq!(
            answer,
            Some(Answer::Missing {
                step: "mahcine".to_string(),
                near: vec!["Machine".to_string()],
                within: vec![],
                reading: false,
            })
        );
    }

    #[test]
    fn qs_saying_it_failed_is_no_answer() {
        assert_eq!(
            answer_of("\u{1b}[31mERROR\u{1b}[0m: no running instance"),
            None
        );
        assert_eq!(answer_of("[]"), None);
        assert_eq!(answer_of(""), None);
    }

    #[test]
    fn what_qs_said_is_said_plainly() {
        assert_eq!(
            plain("\u{1b}[31mERROR\u{1b}[0m:  no\n running"),
            "ERROR: no running"
        );
    }
}

/// Stand-ins for qs, each a script giving one answer whatever it's asked,
/// so what the program does with an answer is tested without a shell.
#[cfg(test)]
pub mod stand_in {
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;
    use std::{env, fs, process};

    /// Each stand-in's name and the answer it prints.
    const ANSWERS: [(&str, &str); 6] = [
        (
            "missed-reading",
            r#"{"missing":"speed","near":[],"in":["Network","Office"],"reading":true}"#,
        ),
        (
            "missed",
            r#"{"missing":"speed","near":[],"in":["Network","Office"],"reading":false}"#,
        ),
        (
            "reading",
            r#"{"reading":true,"in":[],"path":[],"row":null,"rows":[]}"#,
        ),
        (
            "read",
            r#"{"reading":false,"in":[],"path":[],"row":null,"rows":[]}"#,
        ),
        ("failed", r#"{"failed":true}"#),
        ("unopened", r#"{"unopened":true,"in":["Passwords"]}"#),
    ];

    /// The stand-in giving the answer `name` names, to run as qs.
    pub fn qs(name: &str) -> String {
        written().join(name).to_string_lossy().into_owned()
    }

    /// Where the stand-ins are, every one written and closed before the
    /// first runs: a script still open for writing in one test while
    /// another starts a process can't be run (ETXTBSY).
    fn written() -> &'static Path {
        static FOLDER: OnceLock<PathBuf> = OnceLock::new();
        FOLDER.get_or_init(|| {
            let folder = env::temp_dir().join(format!("launcher-qs-{}", process::id()));
            fs::create_dir_all(&folder).expect("a folder for the stand-ins");
            let shell = shell();
            for (name, answer) in ANSWERS {
                let script = folder.join(name);
                fs::write(&script, format!("#!{shell}\necho '{answer}'\n"))
                    .expect("a stand-in written");
                fs::set_permissions(&script, fs::Permissions::from_mode(0o755))
                    .expect("a stand-in made runnable");
            }
            folder
        })
    }

    /// The shell running the tests, as the build sandbox has no /bin/sh.
    fn shell() -> String {
        env::var("SHELL")
            .ok()
            .filter(|shell| Path::new(shell).is_file())
            .or_else(|| {
                env::split_paths(&env::var_os("PATH")?)
                    .map(|folder| folder.join("sh"))
                    .find(|sh| sh.is_file())
                    .map(|sh| sh.to_string_lossy().into_owned())
            })
            .expect("a shell to run the stand-ins")
    }
}
