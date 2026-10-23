//! Tuple's people for the shell (Tuple.qml, People.qml): who they are and
//! how they stand, said only as it changes. Tuple's `contacts follow`
//! prints every contact on each change, one JSON array a line, thousands of
//! them and over a megabyte each time, though Tuple knows how only the
//! favourites stand and says "unknown" for everyone else. The shell would
//! read all of it on each change of anyone's presence, so this reads it
//! instead and passes on two short lines, each only when it differs from
//! the last one written:
//!
//! - `{"people":[[id,"name","email"],...]}`, everyone, in Tuple's order;
//! - `{"standing":[[id,"status",favourite,recent],...]}`, only those who
//!   are a favourite, are recent, or stand somewhere Tuple knows.
//!
//! When both change, people come first, so a standing never names someone
//! the shell hasn't heard of. The arguments are the tuple command and the
//! socket Tuple's daemon listens on. Any tuple command starts a daemon
//! when none is running, so Tuple is run only while one answers there,
//! and otherwise this ends with 69 at once, leaving a stopped Tuple
//! stopped. Tuple runs as `tuple --format json contacts follow`, and when
//! it ends this ends with its exit code. A line that isn't a list of contacts, an error
//! or half a line, says nothing. When the shell stops reading, Tuple's
//! follow is stopped and this ends quietly, and when the shell stops this
//! with a signal, Tuple's follow ends with it.

use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

/// What follows the tuple command.
const ARGS: [&str; 4] = ["--format", "json", "contacts", "follow"];

unsafe extern "C" {
    fn prctl(option: i32, arg2: u64, ...) -> i32;
    fn getppid() -> i32;
}

const PR_SET_PDEATHSIG: i32 = 1;
const SIGTERM: u64 = 15;
const ESRCH: i32 = 3;

/// Has the command's process told to end when the thread starting it
/// does. Tuple's follow writes only when someone's presence changes, so
/// it would only find out this had gone the next time it wrote, which can
/// be hours, and each restart of the shell would leave one behind.
fn dies_with_us(command: &mut Command) -> &mut Command {
    let parent = std::process::id();
    // SAFETY: between fork and exec, only two system calls, both safe in
    // a forked child, and an error made without allocating.
    unsafe {
        command.pre_exec(move || {
            if prctl(PR_SET_PDEATHSIG, SIGTERM) != 0 {
                return Err(io::Error::last_os_error());
            }
            // Gone already, before it could be told: nothing would end it.
            if u32::try_from(getppid()).ok() != Some(parent) {
                return Err(io::Error::from_raw_os_error(ESRCH));
            }
            Ok(())
        })
    }
}

/// A JSON value, as Tuple writes them. A number keeps the text it was
/// written as, so an id is passed on just as Tuple gave it, whatever its
/// size or form.
#[derive(Debug, Clone, PartialEq)]
enum Value {
    Null,
    Bool(bool),
    Number(String),
    Text(String),
    List(Vec<Value>),
    Object(Vec<(String, Value)>),
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

    fn digits(&mut self) -> usize {
        let start = self.at;
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_digit) {
            self.at += 1;
        }
        self.at - start
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
            _ => self.number().map(Value::Number),
        }
    }

    /// A number, held to JSON's form, since it's written back as it was.
    fn number(&mut self) -> Option<String> {
        let start = self.at;
        self.eat("-");
        if !self.eat("0") && self.digits() == 0 {
            return None;
        }
        if self.eat(".") && self.digits() == 0 {
            return None;
        }
        if self.eat("e") || self.eat("E") {
            if !self.eat("+") {
                self.eat("-");
            }
            if self.digits() == 0 {
                return None;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.at]).ok()?;
        Some(text.to_string())
    }

    fn hex(&mut self) -> Option<u32> {
        let digits = self.bytes.get(self.at..self.at + 4)?;
        if !digits.iter().all(u8::is_ascii_hexdigit) {
            return None;
        }
        self.at += 4;
        u32::from_str_radix(std::str::from_utf8(digits).ok()?, 16).ok()
    }

    fn string(&mut self) -> Option<String> {
        if !self.eat("\"") {
            return None;
        }
        let mut out = Vec::new();
        loop {
            // Most of a string is plain, so it's taken a run at a time.
            let start = self.at;
            while self
                .bytes
                .get(self.at)
                .is_some_and(|&b| b != b'"' && b != b'\\')
            {
                self.at += 1;
            }
            out.extend_from_slice(&self.bytes[start..self.at]);
            let byte = *self.bytes.get(self.at)?;
            self.at += 1;
            if byte == b'"' {
                return String::from_utf8(out).ok();
            }
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
                    let mut code = high;
                    // A surrogate pair is one character; half of one alone
                    // is none, and what follows it is read afresh.
                    if (0xd800..0xdc00).contains(&high) {
                        let pair = self.at;
                        if self.eat("\\u") {
                            match self.hex() {
                                Some(low) if (0xdc00..0xe000).contains(&low) => {
                                    code = 0x10000 + ((high - 0xd800) << 10) + (low - 0xdc00);
                                }
                                _ => self.at = pair,
                            }
                        }
                    }
                    char::from_u32(code).unwrap_or('\u{fffd}')
                }
                other => other as char,
            };
            let mut buffer = [0; 4];
            out.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
        }
    }
}

/// Writes a string as JSON does onto the end of `out`.
fn quote(out: &mut String, text: &str) {
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
}

/// Someone as the shell finds them: their id as JSON writes it, the name
/// they're shown by, and their email, "" when Tuple has none.
#[derive(Debug, PartialEq)]
struct Person {
    id: String,
    name: String,
    email: String,
}

/// How someone stands: their id as JSON writes it, their status as Tuple
/// says it, and whether they're a favourite and recent.
#[derive(Debug, PartialEq)]
struct Standing {
    id: String,
    status: String,
    favourite: bool,
    recent: bool,
}

/// Text that says something; "" and null say nothing.
fn said(value: Value) -> Option<String> {
    match value {
        Value::Text(text) if !text.is_empty() => Some(text),
        _ => None,
    }
}

/// One contact, taken from Tuple's object, its words moved rather than
/// copied, since there are thousands each line.
fn contact(fields: Vec<(String, Value)>) -> (Person, Option<Standing>) {
    let (mut id, mut full, mut short, mut email, mut status) = (None, None, None, None, None);
    let (mut favourite, mut recent) = (false, false);
    for (key, value) in fields {
        match key.as_str() {
            "id" => id = Some(value),
            "full_name" => full = said(value),
            "short_name" => short = said(value),
            "email" => email = said(value),
            "status" => status = said(value),
            "favorited" => favourite = value == Value::Bool(true),
            "recent" => recent = value == Value::Bool(true),
            _ => {}
        }
    }
    let (id, id_text) = match id {
        Some(Value::Number(number)) => (number.clone(), number),
        Some(Value::Text(text)) => {
            let mut id = String::new();
            quote(&mut id, &text);
            (id, text)
        }
        _ => ("null".to_string(), String::new()),
    };
    let status = status.unwrap_or_else(|| "unknown".to_string());
    let standing = (favourite || recent || status != "unknown").then(|| Standing {
        id: id.clone(),
        status,
        favourite,
        recent,
    });
    let name = full.or(short).or_else(|| email.clone()).unwrap_or(id_text);
    let email = email.unwrap_or_default();
    (Person { id, name, email }, standing)
}

fn people_line(people: &[Person]) -> String {
    let size: usize = people
        .iter()
        .map(|p| p.id.len() + p.name.len() + p.email.len() + 10)
        .sum();
    let mut out = String::with_capacity(size + 16);
    out.push_str("{\"people\":[");
    for (n, person) in people.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        out.push('[');
        out.push_str(&person.id);
        out.push(',');
        quote(&mut out, &person.name);
        out.push(',');
        quote(&mut out, &person.email);
        out.push(']');
    }
    out.push_str("]}");
    out
}

fn standing_line(standing: &[Standing]) -> String {
    let mut out = String::from("{\"standing\":[");
    for (n, one) in standing.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        out.push('[');
        out.push_str(&one.id);
        out.push(',');
        quote(&mut out, &one.status);
        out.push_str(if one.favourite { ",true" } else { ",false" });
        out.push_str(if one.recent { ",true]" } else { ",false]" });
    }
    out.push_str("]}");
    out
}

/// What was last said of Tuple's people, so only a change is said again.
#[derive(Default)]
struct Follower {
    people: Option<Vec<Person>>,
    standing: Option<Vec<Standing>>,
}

impl Follower {
    /// The lines to write for one of Tuple's, people before standing.
    fn heard(&mut self, line: &str) -> Vec<String> {
        let Some(Value::List(items)) = Reader::parse(line) else {
            return Vec::new();
        };
        let mut people = Vec::with_capacity(items.len());
        let mut standing = Vec::new();
        for item in items {
            let Value::Object(fields) = item else {
                return Vec::new();
            };
            let (person, stands) = contact(fields);
            people.push(person);
            standing.extend(stands);
        }
        let mut lines = Vec::new();
        if self.people.as_ref() != Some(&people) {
            lines.push(people_line(&people));
            self.people = Some(people);
        }
        if self.standing.as_ref() != Some(&standing) {
            lines.push(standing_line(&standing));
            self.standing = Some(standing);
        }
        lines
    }
}

/// Runs Tuple's follow and writes what changes to `out`, flushed a line
/// at a time. Says the follow's exit code once it ends, 1 when a signal
/// ended it or it couldn't start, and 0 when `out` closed first, since
/// then the shell has gone and there's no one to tell.
fn follow(command: &mut Command, mut out: impl Write) -> i32 {
    let mut child = match dies_with_us(command).stdout(Stdio::piped()).spawn() {
        Ok(child) => child,
        Err(error) => {
            eprintln!(
                "tuple-people: can't start {}: {error}",
                command.get_program().to_string_lossy()
            );
            return 1;
        }
    };
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return 1;
    };
    let mut input = BufReader::with_capacity(1 << 20, stdout);
    let mut follower = Follower::default();
    let mut line = Vec::new();
    loop {
        line.clear();
        match input.read_until(b'\n', &mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
        let Ok(text) = std::str::from_utf8(&line) else {
            continue;
        };
        for said in follower.heard(text) {
            let written = out
                .write_all(said.as_bytes())
                .and_then(|()| out.write_all(b"\n"))
                .and_then(|()| out.flush());
            if written.is_err() {
                let _ = child.kill();
                let _ = child.wait();
                return 0;
            }
        }
    }
    drop(input);
    match child.wait() {
        Ok(status) => status.code().unwrap_or(1),
        Err(_) => 1,
    }
}

/// How this ends when no daemon answers: sysexits' "unavailable".
const NOT_RUNNING: u8 = 69;

/// Whether Tuple's daemon answers on its socket. A stopped daemon leaves
/// the socket's file behind, so only a connection tells.
fn running(socket: &Path) -> bool {
    UnixStream::connect(socket).is_ok()
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let [tuple, socket] = args.as_slice() else {
        eprintln!("usage: tuple-people <the tuple command> <its daemon's socket>");
        return ExitCode::from(2);
    };
    if !running(Path::new(socket)) {
        return ExitCode::from(NOT_RUNNING);
    }
    let code = follow(Command::new(tuple).args(ARGS), io::stdout().lock());
    ExitCode::from(u8::try_from(code).unwrap_or(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A contact as Tuple writes one.
    fn contact(id: u32, name: &str, status: &str, favourite: bool) -> String {
        let mut full = String::new();
        quote(&mut full, name);
        format!(
            "{{\"email\":\"p{id}@example.com\",\"favorited\":{favourite},\"full_name\":{full},\
             \"id\":{id},\"kind\":\"user\",\"recent\":false,\"short_name\":null,\"status\":\"{status}\"}}"
        )
    }

    fn snapshot(contacts: &[String]) -> String {
        format!("[{}]\n", contacts.join(","))
    }

    fn usual() -> Vec<String> {
        vec![
            contact(1, "Ada Brook", "online", true),
            contact(2, "Ben Carrow", "unknown", false),
            contact(3, "Cal Dunmore", "offline", true),
        ]
    }

    const PEOPLE: &str = "{\"people\":[[1,\"Ada Brook\",\"p1@example.com\"],\
        [2,\"Ben Carrow\",\"p2@example.com\"],[3,\"Cal Dunmore\",\"p3@example.com\"]]}";
    const STANDING: &str = "{\"standing\":[[1,\"online\",true,false],[3,\"offline\",true,false]]}";

    #[test]
    fn json_is_read_whole() {
        let value = Reader::parse(
            r#" {"a": [1, -2.5e1, true, false, null], "b": "x\"\\\n\u00e9\ud83d\ude00"} "#,
        )
        .unwrap();
        assert_eq!(
            value,
            Value::Object(vec![
                (
                    "a".into(),
                    Value::List(vec![
                        Value::Number("1".into()),
                        Value::Number("-2.5e1".into()),
                        Value::Bool(true),
                        Value::Bool(false),
                        Value::Null
                    ])
                ),
                ("b".into(), Value::Text("x\"\\\n\u{e9}\u{1f600}".into()))
            ])
        );
    }

    #[test]
    fn half_a_surrogate_is_no_character() {
        assert_eq!(
            Reader::parse(r#""\ud83dA\ud83d\u0041""#),
            Some(Value::Text("\u{fffd}A\u{fffd}A".into()))
        );
    }

    #[test]
    fn only_numbers_as_json_writes_them_are_numbers() {
        for number in ["+1", ".5", "1.", "01", "1e", "-"] {
            assert_eq!(Reader::parse(number), None, "{number}");
        }
        assert_eq!(Reader::parse("0"), Some(Value::Number("0".into())));
    }

    #[test]
    fn the_first_snapshot_says_both() {
        let mut follower = Follower::default();
        assert_eq!(follower.heard(&snapshot(&usual())), [PEOPLE, STANDING]);
    }

    #[test]
    fn the_same_snapshot_again_says_nothing() {
        let mut follower = Follower::default();
        follower.heard(&snapshot(&usual()));
        assert!(follower.heard(&snapshot(&usual())).is_empty());
    }

    #[test]
    fn a_change_of_status_says_only_standing() {
        let mut follower = Follower::default();
        follower.heard(&snapshot(&usual()));
        let mut changed = usual();
        changed[0] = contact(1, "Ada Brook", "busy", true);
        assert_eq!(
            follower.heard(&snapshot(&changed)),
            ["{\"standing\":[[1,\"busy\",true,false],[3,\"offline\",true,false]]}"]
        );
    }

    #[test]
    fn a_change_of_name_says_only_people() {
        let mut follower = Follower::default();
        follower.heard(&snapshot(&usual()));
        let mut changed = usual();
        changed[1] = contact(2, "Ben Carrow-Hale", "unknown", false);
        let said = follower.heard(&snapshot(&changed));
        assert_eq!(said.len(), 1);
        assert!(said[0].starts_with("{\"people\":"));
        assert!(said[0].contains("[2,\"Ben Carrow-Hale\",\"p2@example.com\"]"));
    }

    #[test]
    fn a_favourite_made_says_standing() {
        let mut follower = Follower::default();
        follower.heard(&snapshot(&usual()));
        let mut changed = usual();
        changed[1] = contact(2, "Ben Carrow", "unknown", true);
        assert_eq!(
            follower.heard(&snapshot(&changed)),
            [
                "{\"standing\":[[1,\"online\",true,false],[2,\"unknown\",true,false],\
              [3,\"offline\",true,false]]}"
            ]
        );
    }

    #[test]
    fn strangers_of_unknown_standing_are_left_out() {
        let mut follower = Follower::default();
        let line = r#"[{"id":1,"status":"unknown","favorited":false,"recent":false},
            {"id":2,"status":"","favorited":false},
            {"id":3},
            {"id":4,"status":null,"recent":true},
            {"id":5,"status":"away","favorited":false,"recent":false}]"#;
        let said = follower.heard(line);
        assert_eq!(
            said[1],
            "{\"standing\":[[4,\"unknown\",false,true],[5,\"away\",false,false]]}"
        );
    }

    #[test]
    fn names_fall_back_to_the_short_name_the_email_then_the_id() {
        let mut follower = Follower::default();
        let line = r#"[{"id":1,"full_name":"Ada Brook","short_name":"Ada","email":"a@example.com"},
            {"id":2,"full_name":"","short_name":"Ben","email":"b@example.com"},
            {"id":3,"full_name":null,"email":"c@example.com"},
            {"id":4,"email":null},
            {"id":4.5},
            {"id":"six"}]"#;
        assert_eq!(
            follower.heard(line)[0],
            "{\"people\":[[1,\"Ada Brook\",\"a@example.com\"],[2,\"Ben\",\"b@example.com\"],\
             [3,\"c@example.com\",\"c@example.com\"],[4,\"4\",\"\"],[4.5,\"4.5\",\"\"],\
             [\"six\",\"six\",\"\"]]}"
        );
    }

    #[test]
    fn awkward_words_come_back_as_they_went() {
        let name = "Zoë \"Z\" O'Neil \\ the\nsecond\t\u{1}\u{1f600}";
        let mut follower = Follower::default();
        let said = follower.heard(&snapshot(&[contact(7, name, "online", true)]));
        let Some(Value::Object(fields)) = Reader::parse(&said[0]) else {
            panic!("{}", said[0]);
        };
        let Value::List(people) = &fields[0].1 else {
            panic!("{}", said[0]);
        };
        assert_eq!(
            people[0],
            Value::List(vec![
                Value::Number("7".into()),
                Value::Text(name.into()),
                Value::Text("p7@example.com".into())
            ])
        );
    }

    #[test]
    fn a_malformed_line_says_nothing_and_disturbs_nothing() {
        let mut follower = Follower::default();
        follower.heard(&snapshot(&usual()));
        let whole = snapshot(&usual());
        for line in [
            "Error: the daemon isn't running\n",
            &whole[..whole.len() / 2],
            "{\"contacts\":[]}\n",
            "[1,2,3]\n",
            "",
        ] {
            assert!(follower.heard(line).is_empty(), "{line}");
        }
        assert!(follower.heard(&whole).is_empty());
        let mut changed = usual();
        changed[2] = contact(3, "Cal Dunmore", "online", true);
        assert_eq!(follower.heard(&snapshot(&changed)).len(), 1);
    }

    /// A stand-in for the tuple command, in a file of its own.
    fn stand_in(name: &str, script: &str) -> std::path::PathBuf {
        let path = env::temp_dir().join(format!("tuple-people-test-{}-{name}", std::process::id()));
        fs::write(&path, script).unwrap();
        path
    }

    #[test]
    fn it_says_what_changed_and_ends_as_tuple_did() {
        let mut changed = usual();
        changed[0] = contact(1, "Ada Brook", "busy", true);
        let script = stand_in(
            "follow",
            &format!(
                "[ \"$*\" = \"--format json contacts follow\" ] || exit 9\n\
                 printf '%s' '{}'\necho 'Error: lost the daemon'\nprintf '%s' '{}'\nexit 3\n",
                snapshot(&usual()),
                snapshot(&changed)
            ),
        );
        let mut written = Vec::new();
        let code = follow(Command::new("sh").arg(&script).args(ARGS), &mut written);
        fs::remove_file(&script).unwrap();
        assert_eq!(code, 3);
        assert_eq!(
            String::from_utf8(written).unwrap(),
            format!(
                "{PEOPLE}\n{STANDING}\n\
                 {{\"standing\":[[1,\"busy\",true,false],[3,\"offline\",true,false]]}}\n"
            )
        );
    }

    /// Output whose reader has gone.
    struct Closed;

    impl Write for Closed {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
    }

    #[test]
    fn a_closed_output_ends_it_quietly() {
        let script = stand_in("forever", "while :; do echo '[]'; done\n");
        let code = follow(Command::new("sh").arg(&script), Closed);
        fs::remove_file(&script).unwrap();
        assert_eq!(code, 0);
    }

    /// The kernel ends the child as the thread that started it ends, so a
    /// thread of the tests' own stands in for the shell's helper.
    #[test]
    fn the_follow_ends_with_whoever_started_it() {
        use std::os::unix::process::ExitStatusExt;
        use std::time::{Duration, Instant};
        let mut child = std::thread::spawn(|| {
            dies_with_us(&mut Command::new("sleep").arg("60"))
                .spawn()
                .unwrap()
        })
        .join()
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break Some(status);
            }
            if Instant::now() > deadline {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        assert_eq!(status.and_then(|s| s.signal()), Some(15));
    }

    #[test]
    fn a_tuple_that_wont_start_ends_it_with_1() {
        let missing = env::temp_dir().join("tuple-people-test-nothing-here");
        assert_eq!(follow(&mut Command::new(missing), Vec::new()), 1);
    }

    /// A socket's place of its own for a test, empty to start with.
    fn socket(name: &str) -> std::path::PathBuf {
        let path =
            env::temp_dir().join(format!("tuple-people-socket-{}-{name}", std::process::id()));
        let _ = fs::remove_file(&path);
        path
    }

    #[test]
    fn a_daemon_listening_is_running() {
        let path = socket("listening");
        let _daemon = std::os::unix::net::UnixListener::bind(&path).unwrap();
        assert!(running(&path));
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn the_socket_a_stopped_daemon_left_is_not_running() {
        let path = socket("left");
        drop(std::os::unix::net::UnixListener::bind(&path).unwrap());
        assert!(path.exists());
        assert!(!running(&path));
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn no_socket_is_not_running() {
        assert!(!running(&socket("none")));
    }
}
