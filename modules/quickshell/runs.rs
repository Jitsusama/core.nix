//! The launcher's runs (launcher/Runs.qml): pi's fleets and review rounds
//! of the last day, and any still running, as one JSON array on standard
//! output. A run is a folder of participants, each keeping its request,
//! its lease, its progress while it works and its result once it ends;
//! fleets keep theirs in subagent-workflow/runs, and councils, judges and
//! the other review rounds in review/transcripts/runs, named for the round.
//! A round is named for the change it read, from the review file that
//! lists it (review/runs, one per change, named for it).
//!
//! The harness's state folder is the only argument. A file that can't be
//! read or parsed says nothing, so a participant just begun, or one whose
//! files are half written, is shown with what's there. Given `answer` and
//! a participant's result file instead, it prints the answer the
//! participant gave, for a pager.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, SystemTime};

/// A JSON value, as the harness writes them.
#[derive(Debug, Clone, PartialEq)]
enum Value {
    Null,
    Bool(bool),
    Number(f64),
    Text(String),
    List(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(fields) => fields
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    fn text(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            Value::Text(text) => Some(text),
            _ => None,
        }
    }

    fn number(&self, key: &str) -> Option<f64> {
        match self.get(key)? {
            Value::Number(number) => Some(*number),
            _ => None,
        }
    }
}

/// Reads JSON text, or nothing where it isn't JSON.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn parse(text: &str) -> Option<Value> {
        let mut reader = Reader {
            bytes: text.as_bytes(),
            at: 0,
        };
        let value = reader.value()?;
        reader.space();
        (reader.at == reader.bytes.len()).then_some(value)
    }

    fn space(&mut self) {
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| b.is_ascii_whitespace())
        {
            self.at += 1;
        }
    }

    fn eat(&mut self, word: &str) -> bool {
        if self.bytes[self.at..].starts_with(word.as_bytes()) {
            self.at += word.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self) -> Option<Value> {
        self.space();
        match *self.bytes.get(self.at)? {
            b'{' => {
                self.at += 1;
                let mut fields = Vec::new();
                self.space();
                if self.eat("}") {
                    return Some(Value::Object(fields));
                }
                loop {
                    self.space();
                    let name = self.string()?;
                    self.space();
                    if !self.eat(":") {
                        return None;
                    }
                    fields.push((name, self.value()?));
                    self.space();
                    if self.eat("}") {
                        return Some(Value::Object(fields));
                    }
                    if !self.eat(",") {
                        return None;
                    }
                }
            }
            b'[' => {
                self.at += 1;
                let mut items = Vec::new();
                self.space();
                if self.eat("]") {
                    return Some(Value::List(items));
                }
                loop {
                    items.push(self.value()?);
                    self.space();
                    if self.eat("]") {
                        return Some(Value::List(items));
                    }
                    if !self.eat(",") {
                        return None;
                    }
                }
            }
            b'"' => self.string().map(Value::Text),
            b't' => self.eat("true").then_some(Value::Bool(true)),
            b'f' => self.eat("false").then_some(Value::Bool(false)),
            b'n' => self.eat("null").then_some(Value::Null),
            _ => {
                let start = self.at;
                while self
                    .bytes
                    .get(self.at)
                    .is_some_and(|b| b.is_ascii_digit() || b"+-.eE".contains(b))
                {
                    self.at += 1;
                }
                let number = std::str::from_utf8(&self.bytes[start..self.at]).ok()?;
                number.parse().ok().map(Value::Number)
            }
        }
    }

    fn hex(&mut self) -> Option<u32> {
        let digits = std::str::from_utf8(self.bytes.get(self.at..self.at + 4)?).ok()?;
        self.at += 4;
        u32::from_str_radix(digits, 16).ok()
    }

    fn string(&mut self) -> Option<String> {
        if !self.eat("\"") {
            return None;
        }
        let mut out = Vec::new();
        loop {
            let byte = *self.bytes.get(self.at)?;
            self.at += 1;
            match byte {
                b'"' => return String::from_utf8(out).ok(),
                b'\\' => {
                    let escaped = *self.bytes.get(self.at)?;
                    self.at += 1;
                    let c = match escaped {
                        b'n' => '\n',
                        b't' => '\t',
                        b'r' => '\r',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'u' => {
                            let high = self.hex()?;
                            let code = if (0xd800..0xdc00).contains(&high) && self.eat("\\u") {
                                let low = self.hex()?;
                                0x10000 + ((high - 0xd800) << 10) + (low.checked_sub(0xdc00)?)
                            } else {
                                high
                            };
                            char::from_u32(code).unwrap_or('\u{fffd}')
                        }
                        other => other as char,
                    };
                    let mut buffer = [0; 4];
                    out.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
                }
                other => out.push(other),
            }
        }
    }
}

fn read(path: &Path) -> Option<Value> {
    Reader::parse(&fs::read_to_string(path).ok()?)
}

/// A string as JSON writes it.
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn number(value: Option<f64>) -> String {
    match value {
        Some(number) if number.is_finite() => format!("{number}"),
        _ => "null".to_string(),
    }
}

/// The word after a flag in a participant's command line.
fn flag<'a>(args: Option<&'a Value>, name: &str) -> &'a str {
    let Some(Value::List(args)) = args else {
        return "";
    };
    let mut words = args.iter();
    while let Some(word) = words.next() {
        if *word == Value::Text(name.to_string()) {
            if let Some(Value::Text(next)) = words.next() {
                return next;
            }
        }
    }
    ""
}

/// What one participant keeps, read from its folder.
#[derive(Debug, Default, PartialEq)]
struct Participant {
    id: String,
    state: String,
    activity: String,
    model: String,
    thinking: String,
    started: String,
    ended: String,
    elapsed: Option<f64>,
    cost: Option<f64>,
    answer: String,
    pid: Option<f64>,
    cwd: String,
}

fn participant(folder: &Path) -> Participant {
    let none = Value::Null;
    let request = read(&folder.join("request.json"));
    let request = request.as_ref().unwrap_or(&none);
    let lease = read(&folder.join("lease.json"));
    let lease = lease.as_ref().unwrap_or(&none);
    let progress = read(&folder.join("progress.json"));
    let progress = progress.as_ref().unwrap_or(&none);
    let result = read(&folder.join("result.json"));
    let result = result.as_ref().unwrap_or(&none);
    let state = said_state(progress, lease, result, folder);
    let said = result.text("finalAssistantText").unwrap_or("");
    Participant {
        id: folder
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        state,
        activity: progress.text("activity").unwrap_or("").to_string(),
        model: flag(request.get("args"), "--model").to_string(),
        thinking: flag(request.get("args"), "--thinking").to_string(),
        started: result.text("startedAt").unwrap_or("").to_string(),
        ended: result.text("completedAt").unwrap_or("").to_string(),
        elapsed: progress.number("elapsedMs"),
        cost: result
            .get("usage")
            .and_then(|usage| usage.get("cost"))
            .and_then(|cost| cost.number("total")),
        answer: if said.trim().is_empty() {
            String::new()
        } else {
            folder.join("result.json").to_string_lossy().into_owned()
        },
        pid: lease.number("parentPid"),
        cwd: request.text("cwd").unwrap_or("").to_string(),
    }
}

fn is_running(state: &str) -> bool {
    matches!(state, "starting" | "running" | "idle")
}

/// How a participant stands: as its files say, unless they say it runs
/// and its process is gone, when it's "gone", since nothing will finish
/// it. One with no state yet is starting while its request is new.
fn said_state(progress: &Value, lease: &Value, result: &Value, folder: &Path) -> String {
    let said = [progress, result, lease]
        .iter()
        .find_map(|file| file.text("state"));
    let alive =
        |pid: Option<f64>| pid.is_some_and(|pid| Path::new(&format!("/proc/{pid}")).exists());
    match said {
        Some(state) if is_running(state) && !alive(lease.number("childPid")) => "gone".to_string(),
        Some(state) => state.to_string(),
        None => {
            let asked = fs::metadata(folder.join("request.json")).and_then(|m| m.modified());
            let new = asked.is_ok_and(|asked| {
                asked
                    .elapsed()
                    .is_ok_and(|age| age < Duration::from_secs(60))
            });
            if new { "starting" } else { "gone" }.to_string()
        }
    }
}

/// Whether a participant runs, from its progress and lease alone.
fn runs_now(folder: &Path) -> bool {
    let none = Value::Null;
    let progress = read(&folder.join("progress.json")).unwrap_or(Value::Null);
    let lease = read(&folder.join("lease.json")).unwrap_or(Value::Null);
    is_running(&said_state(&progress, &lease, &none, folder))
}

/// A change as a review file is named for it: provider, host, owner,
/// repository and number, joined by underscores, said as "repository#number".
fn change(file: &str) -> Option<String> {
    let stem = file.strip_suffix(".json")?;
    let (rest, number) = stem.rsplit_once('_')?;
    number.parse::<u64>().ok()?;
    let mut parts = rest.splitn(4, '_');
    let (_provider, _host, _owner, repository) =
        (parts.next()?, parts.next()?, parts.next()?, parts.next()?);
    Some(format!("{repository}#{number}"))
}

/// A run: its id, kind, what it read, and its participants.
struct Run {
    id: String,
    kind: String,
    about: String,
    participants: Vec<Participant>,
}

fn json(run: &Run) -> String {
    let first = run.participants.first();
    let started = run
        .participants
        .iter()
        .map(|p| p.started.as_str())
        .filter(|started| !started.is_empty())
        .min()
        .unwrap_or("");
    let participants: Vec<String> = run
        .participants
        .iter()
        .map(|p| {
            format!(
                "{{\"id\":{},\"state\":{},\"activity\":{},\"model\":{},\"thinking\":{},\"started\":{},\
                 \"ended\":{},\"elapsed\":{},\"cost\":{},\"answer\":{}}}",
                quoted(&p.id),
                quoted(&p.state),
                quoted(&p.activity),
                quoted(&p.model),
                quoted(&p.thinking),
                quoted(&p.started),
                quoted(&p.ended),
                number(p.elapsed),
                number(p.cost),
                quoted(&p.answer)
            )
        })
        .collect();
    format!(
        "{{\"id\":{},\"kind\":{},\"about\":{},\"pid\":{},\"cwd\":{},\"started\":{},\"participants\":[{}]}}",
        quoted(&run.id),
        quoted(&run.kind),
        quoted(&run.about),
        number(first.and_then(|p| p.pid)),
        quoted(first.map(|p| p.cwd.as_str()).unwrap_or("")),
        quoted(started),
        participants.join(",")
    )
}

/// The runs under one root whose participants changed since `since`, or
/// still run.
fn runs(root: &Path, reviews: bool, since: SystemTime) -> Vec<Run> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return found;
    };
    for entry in entries.flatten() {
        let folder = entry.path();
        let Ok(members) = fs::read_dir(folder.join("reviewers")) else {
            continue;
        };
        let mut recent = false;
        let mut folders: Vec<PathBuf> = Vec::new();
        for member in members.flatten() {
            let path = member.path();
            let changed = ["progress.json", "lease.json", "result.json"]
                .iter()
                .filter_map(|name| {
                    fs::metadata(path.join(name))
                        .and_then(|m| m.modified())
                        .ok()
                })
                .max();
            recent |= changed.is_some_and(|changed| changed >= since);
            folders.push(path);
        }
        if folders.is_empty() {
            continue;
        }
        folders.sort();
        if !recent && !folders.iter().any(|path| runs_now(path)) {
            continue;
        }
        let participants: Vec<Participant> = folders.iter().map(|path| participant(path)).collect();
        let id = entry.file_name().to_string_lossy().into_owned();
        let kind = if reviews {
            id.split('-').next().unwrap_or("review").to_string()
        } else {
            "fleet".to_string()
        };
        found.push(Run {
            id,
            kind,
            about: String::new(),
            participants,
        });
    }
    found
}

/// Names each review round for the change whose review file lists it.
fn name_rounds(found: &mut [Run], files: &Path, since: SystemTime) {
    let Ok(entries) = fs::read_dir(files) else {
        return;
    };
    for entry in entries.flatten() {
        let recent = entry
            .metadata()
            .and_then(|m| m.modified())
            .is_ok_and(|m| m >= since);
        if !recent {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let (Some(about), Ok(text)) = (change(&name), fs::read_to_string(entry.path())) else {
            continue;
        };
        for run in found
            .iter_mut()
            .filter(|run| run.kind != "fleet" && run.about.is_empty())
        {
            if text.contains(&quoted(&run.id)) {
                run.about = about.clone();
            }
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if let [mode, file] = args.as_slice() {
        if mode == "answer" {
            let Some(text) = read(Path::new(file))
                .and_then(|result| result.text("finalAssistantText").map(String::from))
            else {
                eprintln!("pi-runs: {file} holds no answer");
                return ExitCode::FAILURE;
            };
            return match io::stdout()
                .lock()
                .write_all(format!("{}\n", text.trim_end()).as_bytes())
            {
                Ok(()) => ExitCode::SUCCESS,
                Err(_) => ExitCode::FAILURE,
            };
        }
    }
    let [state] = args.as_slice() else {
        eprintln!("usage: pi-runs <the harness's state folder> | pi-runs answer <a result file>");
        return ExitCode::from(2);
    };
    let state = PathBuf::from(state);
    let since = SystemTime::now() - Duration::from_secs(24 * 3600);
    let mut found = runs(&state.join("subagent-workflow/runs"), false, since);
    let mut rounds = runs(&state.join("review/transcripts/runs"), true, since);
    // A round named for its change may be listed by a file a day old, so
    // those are read for a week.
    name_rounds(
        &mut rounds,
        &state.join("review/runs"),
        since - Duration::from_secs(6 * 24 * 3600),
    );
    found.append(&mut rounds);
    let out: Vec<String> = found.iter().map(json).collect();
    match io::stdout()
        .lock()
        .write_all(format!("[{}]\n", out.join(",")).as_bytes())
    {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_is_read_whole() {
        let value = Reader::parse(
            r#" {"a": [1, -2.5e1, true, false, null], "b": {"c": "x\"\\\n\u00e9\ud83d\ude00"}} "#,
        )
        .unwrap();
        assert_eq!(
            value.get("a"),
            Some(&Value::List(vec![
                Value::Number(1.0),
                Value::Number(-25.0),
                Value::Bool(true),
                Value::Bool(false),
                Value::Null
            ]))
        );
        assert_eq!(
            value.get("b").unwrap().text("c"),
            Some("x\"\\\n\u{e9}\u{1f600}")
        );
    }

    #[test]
    fn half_a_file_is_nothing() {
        assert_eq!(Reader::parse(r#"{"a": [1, 2"#), None);
        assert_eq!(Reader::parse(r#"{"a": 1} x"#), None);
        assert_eq!(Reader::parse(""), None);
    }

    #[test]
    fn a_flag_says_the_word_after_it() {
        let args =
            Reader::parse(r#"["cli.js", "--model", "anthropic/opus", "--thinking", "high"]"#);
        assert_eq!(flag(args.as_ref(), "--model"), "anthropic/opus");
        assert_eq!(flag(args.as_ref(), "--thinking"), "high");
        assert_eq!(flag(args.as_ref(), "--tools"), "");
        assert_eq!(flag(None, "--model"), "");
    }

    #[test]
    fn a_review_file_names_its_change() {
        assert_eq!(
            change("github_github_Jitsusama_core.nix_380.json").as_deref(),
            Some("core.nix#380")
        );
        assert_eq!(
            change("host_host_owner_the_repository_2264446.json").as_deref(),
            Some("the_repository#2264446")
        );
        assert_eq!(change("local_range.json"), None);
    }

    #[test]
    fn a_participant_runs_only_while_its_process_does() {
        let folder = env::temp_dir().join(format!("pi-runs-test-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let running = Value::Object(vec![("state".into(), Value::Text("running".into()))]);
        let lease = |pid: f64| Value::Object(vec![("childPid".into(), Value::Number(pid))]);
        let me = std::process::id() as f64;
        assert_eq!(
            said_state(&running, &lease(me), &Value::Null, &folder),
            "running"
        );
        assert_eq!(
            said_state(&running, &lease(4_000_000_000.0), &Value::Null, &folder),
            "gone"
        );
        let done = Value::Object(vec![("state".into(), Value::Text("complete".into()))]);
        assert_eq!(
            said_state(&done, &Value::Null, &Value::Null, &folder),
            "complete"
        );
        // Nothing said yet: starting while its request is new, else gone.
        assert_eq!(
            said_state(&Value::Null, &Value::Null, &Value::Null, &folder),
            "gone"
        );
        fs::write(folder.join("request.json"), "{}").unwrap();
        assert_eq!(
            said_state(&Value::Null, &Value::Null, &Value::Null, &folder),
            "starting"
        );
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn numbers_are_written_plainly() {
        assert_eq!(number(Some(1571469.0)), "1571469");
        assert_eq!(number(Some(8.5)), "8.5");
        assert_eq!(number(Some(f64::NAN)), "null");
        assert_eq!(number(None), "null");
    }
}
