//! Names the devices that have stopped working: any whose runtime power
//! management failed, which the kernel then holds failed and won't wake until
//! the next boot, any the machine expects that isn't there, and any disk that
//! reported a fault to smartd this boot, which device-check-disk keeps. And the
//! kernel itself, when it oopsed, locked up, heard of a hardware fault or had a
//! task stuck in it, since it can misbehave until the next boot and only says
//! so in its log. And sleep: one that failed, and one the hardware spent
//! mostly awake, which drains a battery as fast as being on.
//!
//!   device-check    answers what's wrong now, and tells the desktop what
//!                   has gone wrong since it last ran and what works again,
//!                   and the journal too when it's run under one
//!
//! Each change is told once, so running it often costs nothing to read. Its
//! answer takes the form `--format` asks for, as every command of ours does
//! (docs/commands.md). It exits 0 whatever it finds, and non-zero only when
//! it couldn't look.
//!
//! `DEVICE_CHECK_ROOT` puts a directory in front of /sys, /proc and /run,
//! `DEVICE_CHECK_EXPECTED` names the expected devices' file, `DEVICE_CHECK_STATE`
//! where it keeps what it found, `DEVICE_CHECK_NOTIFY` the program that
//! notifies, and `DEVICE_CHECK_SUSPENDED` the seconds spent asleep since
//! boot in place of the clocks', for tests.

use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};
use std::process::{self, Command, ExitCode, Stdio};
use terminal_kit::{Exit, Failure, Help, Pen, Room, layout};

mod sleep;
use sleep::{Sleep, Slept, sleep_findings, sleep_now};

/// notify-send, as the package builds it in.
const NOTIFY_SEND: &str = match option_env!("DEVICE_CHECK_NOTIFY_SEND") {
    Some(path) => path,
    None => "notify-send",
};

/// The kernel's taints that mean trouble, by bit, with what each means for
/// the person at the machine. It never clears one, and its other taints, a
/// warning among them, are no sign of trouble.
const TAINTS: [(u32, &str); 4] = [
    (7, "oops"),
    (14, "lockup"),
    (4, "machine check"),
    (5, "bad page"),
];

/// A path under the root the tests stand in for the machine's.
fn under(root: &OsStr, path: &str) -> PathBuf {
    let mut joined = root.to_os_string();
    joined.push(path);
    PathBuf::from(joined)
}

/// A file's first line, without its line ending.
fn first_line(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    Some(text.lines().next().unwrap_or("").to_owned())
}

/// A file's first word as a number, or zero.
fn number(path: &Path) -> u64 {
    first_line(path)
        .and_then(|line| line.split_whitespace().next()?.parse().ok())
        .unwrap_or(0)
}

/// Each device whose power change failed, as "power", a tab, and the device
/// with its driver. It walks sysfs's devices without following links.
fn failed_power(root: &OsStr) -> Vec<String> {
    let mut found = Vec::new();
    walk(&under(root, "/sys/devices"), &mut found);
    found
}

fn walk(dir: &Path, found: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if kind.is_dir() {
            walk(&path, found);
        } else if entry.file_name() == "runtime_status"
            && dir.file_name() == Some(OsStr::new("power"))
        {
            if first_line(&path).as_deref().map(str::trim) != Some("error") {
                continue;
            }
            let Some(device) = dir.parent() else {
                continue;
            };
            let name = device.file_name().unwrap_or_default().to_string_lossy();
            let driver = fs::read_link(device.join("driver"))
                .ok()
                .and_then(|driver| {
                    driver
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                })
                .unwrap_or_else(|| "none".to_owned());
            found.push(format!("power\t{name} ({driver})"));
        }
    }
}

/// Each expected device that isn't there, as "missing", a tab, and the name
/// the machine gave it. The expected file holds a name, an equals sign and a
/// path a line, and the path's last part may be a glob.
fn missing(root: &OsStr, expected: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(expected) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let (name, path) = line.split_once('=').unwrap_or((line, ""));
            if name.is_empty() || present(root, path) {
                None
            } else {
                Some(format!("missing\t{name}"))
            }
        })
        .collect()
}

/// Whether anything in the path's directory matches its last part.
fn present(root: &OsStr, path: &str) -> bool {
    let (dir, pattern) = path.rsplit_once('/').unwrap_or((path, path));
    if pattern.is_empty() {
        return false;
    }
    let Ok(entries) = fs::read_dir(under(root, dir)) else {
        return false;
    };
    entries
        .flatten()
        .any(|entry| glob(pattern.as_bytes(), entry.file_name().as_encoded_bytes()))
}

/// Whether a name matches a shell pattern: `*`, `?`, `[...]` with `!` or `^`
/// to negate and `-` for a range, and `\` to take the next byte as it is.
fn glob(pattern: &[u8], name: &[u8]) -> bool {
    match pattern.split_first() {
        None => name.is_empty(),
        Some((b'*', rest)) => (0..=name.len()).any(|skip| glob(rest, &name[skip..])),
        Some((b'?', rest)) => !name.is_empty() && glob(rest, &name[1..]),
        Some((b'[', rest)) => match (class(rest, name.first().copied()), name.split_first()) {
            (Some((true, after)), Some((_, name))) => glob(after, name),
            // An unclosed bracket is itself.
            (None, Some((b'[', name))) => glob(rest, name),
            (Some(_) | None, _) => false,
        },
        Some((b'\\', [escaped, rest @ ..])) => {
            name.first() == Some(escaped) && glob(rest, &name[1..])
        }
        Some((byte, rest)) => name.first() == Some(byte) && glob(rest, &name[1..]),
    }
}

/// Whether a byte is in the bracket class that starts the pattern, and the
/// pattern after the class, or None when the class never closes.
fn class(pattern: &[u8], byte: Option<u8>) -> Option<(bool, &[u8])> {
    let (negated, mut rest) = match pattern.split_first() {
        Some((b'!' | b'^', rest)) => (true, rest),
        _ => (false, pattern),
    };
    let mut matched = false;
    let mut first = true;
    loop {
        match rest {
            [] => return None,
            [b']', after @ ..] if !first => {
                return Some((matched != negated && byte.is_some(), after));
            }
            [low, b'-', high, after @ ..] if *high != b']' => {
                matched |= byte.is_some_and(|byte| (*low..=*high).contains(&byte));
                rest = after;
            }
            [single, after @ ..] => {
                matched |= byte == Some(*single);
                rest = after;
            }
        }
        first = false;
    }
}

/// Each disk that reported a fault, as "disk", a tab, and the disk with what
/// it reported.
fn disk_faults(root: &OsStr) -> Vec<String> {
    let Ok(entries) = fs::read_dir(under(root, "/run/device-check/disks")) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .filter(|entry| !entry.file_name().as_encoded_bytes().starts_with(b"."))
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    files.sort();
    files
        .iter()
        .filter_map(|file| {
            let text = fs::read_to_string(file).ok()?;
            let line = text.lines().next()?.trim_matches('\t');
            let (device, message) = line.split_once('\t').unwrap_or((line, ""));
            Some(format!(
                "disk\t{device} reports {}",
                message.trim_matches('\t')
            ))
        })
        .collect()
}

/// Each kind of trouble the kernel has had this boot, as "kernel", a tab,
/// and the kind.
fn kernel_trouble(root: &OsStr) -> Vec<String> {
    let tainted = number(&under(root, "/proc/sys/kernel/tainted"));
    let stuck = number(&under(root, "/proc/sys/kernel/hung_task_detect_count"));
    let mut found: Vec<String> = TAINTS
        .iter()
        .filter(|(bit, _)| tainted & (1u64 << bit) != 0)
        .map(|(_, kind)| format!("kernel\t{kind}"))
        .collect();
    if stuck > 0 {
        found.push("kernel\tstuck task".to_owned());
    }
    found
}

/// What the kernel's trouble of each kind means for the person at the machine.
fn trouble(root: &OsStr, kind: &str) -> Option<String> {
    Some(match kind {
        "oops" => "The kernel hit a fault and may misbehave until the next boot".to_owned(),
        "lockup" => {
            "The kernel locked up a processor and may misbehave until the next boot".to_owned()
        }
        "machine check" => "The processor reported a hardware error".to_owned(),
        "bad page" => "The kernel found a damaged memory page".to_owned(),
        "stuck task" => {
            let waited = first_line(&under(root, "/proc/sys/kernel/hung_task_timeout_secs"))
                .map(|line| line.trim().to_owned())
                .filter(|line| !line.is_empty())
                .unwrap_or_else(|| "120".to_owned());
            format!("A task was stuck in the kernel for over {waited} seconds")
        }
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Change {
    Broke,
    Fixed,
}

/// Whether a device's name is plural, so its line says "are": its last word
/// ends in s, as "The speakers" and "The USB-C ports" do, unless the s is a
/// possessive's or ends "ss" or "us". Every name the hardware modules give
/// reads right by it.
fn plural(subject: &str) -> bool {
    let last = subject.rsplit(' ').next().unwrap_or(subject);
    last.ends_with('s') && !last.ends_with("'s") && !last.ends_with("ss") && !last.ends_with("us")
}

/// The line to say for a finding, or None when nobody needs telling: only
/// a new boot clears the kernel's trouble.
fn line(root: &OsStr, change: Change, finding: &str) -> Option<String> {
    let (kind, subject) = finding.split_once('\t')?;
    Some(match (kind, change) {
        ("power", Change::Broke) => {
            format!("{subject} failed a power change and won't wake until the next boot")
        }
        ("power", Change::Fixed) => format!("{subject} wakes again"),
        ("missing", change) => {
            let verb = if plural(subject) { "are" } else { "is" };
            let state = if change == Change::Broke {
                "missing"
            } else {
                "back"
            };
            format!("{subject} {verb} {state}")
        }
        ("disk", Change::Broke) => format!("The disk {subject}"),
        ("disk", Change::Fixed) => format!(
            "The disk {}",
            subject.replacen(" reports ", " no longer reports ", 1)
        ),
        ("kernel", Change::Broke) => trouble(root, subject)?,
        _ => return None,
    })
}

/// Where it looks, keeps what it found, and notifies.
struct Places {
    root: OsString,
    expected: PathBuf,
    state: PathBuf,
    notify: OsString,
}

impl Places {
    fn here() -> Result<Self, Failure> {
        let state = match env::var_os("DEVICE_CHECK_STATE") {
            Some(state) => PathBuf::from(state),
            None => match env::var_os("XDG_RUNTIME_DIR").filter(|dir| !dir.is_empty()) {
                Some(runtime) => Path::new(&runtime).join("device-check"),
                None => {
                    return Err(Failure::new(
                        Exit::Config,
                        "there's nowhere to keep what it finds",
                    )
                    .because("XDG_RUNTIME_DIR isn't set, as it is in a signed-in session")
                    .try_this("run it signed in, or set DEVICE_CHECK_STATE to a folder"));
                }
            },
        };
        Ok(Self {
            root: env::var_os("DEVICE_CHECK_ROOT").unwrap_or_default(),
            expected: env::var_os("DEVICE_CHECK_EXPECTED")
                .map_or_else(|| PathBuf::from("/etc/device-check"), PathBuf::from),
            state,
            notify: env::var_os("DEVICE_CHECK_NOTIFY")
                .unwrap_or_else(|| OsString::from(NOTIFY_SEND)),
        })
    }
}

/// What a look found: what's wrong now, what broke and what was fixed since
/// the last, and what went wrong with the sleeps between them.
struct Look {
    now: BTreeSet<String>,
    broke: Vec<String>,
    fixed: Vec<String>,
    sleeps: Vec<Slept>,
    /// The kernel's sleep counts now, kept for the next look, or None on a
    /// kernel that keeps none.
    sleep: Option<Sleep>,
}

/// Looks, comparing with what the last look kept. The first look after boot
/// judges no sleep, having no counts to judge it from.
fn look(places: &Places) -> io::Result<Look> {
    let root = &places.root;
    let found: BTreeSet<String> = match fs::read_to_string(places.state.join("found")) {
        Ok(text) => text.lines().map(str::to_owned).collect(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => BTreeSet::new(),
        Err(error) => return Err(error),
    };
    let now: BTreeSet<String> = failed_power(root)
        .into_iter()
        .chain(missing(root, &places.expected))
        .chain(disk_faults(root))
        .chain(kernel_trouble(root))
        .collect();
    let said = |change, findings: Vec<&String>| -> Vec<String> {
        findings
            .into_iter()
            .filter(|finding| line(root, change, finding).is_some())
            .cloned()
            .collect()
    };
    let broke = said(Change::Broke, now.difference(&found).collect());
    let fixed = said(Change::Fixed, found.difference(&now).collect());
    let sleep = sleep_now(root);
    let sleeps = sleep.map_or_else(Vec::new, |now| {
        fs::read_to_string(places.state.join("sleep"))
            .ok()
            .and_then(|text| Sleep::parse(&text))
            .filter(|before| before.same_boot(&now))
            .map_or_else(Vec::new, |before| sleep_findings(root, &before, &now))
    });
    Ok(Look {
        now,
        broke,
        fixed,
        sleeps,
        sleep,
    })
}

/// Whether stderr is the journal's, as systemd says by naming its stream:
/// a command a person runs from a terminal a service started inherits the
/// variable, but not the stream.
fn into_the_journal() -> bool {
    let Some(named) = env::var_os("JOURNAL_STREAM") else {
        return false;
    };
    let Ok(stderr) = fs::metadata("/proc/self/fd/2") else {
        return false;
    };
    named.to_str() == Some(&format!("{}:{}", stderr.dev(), stderr.ino()))
}

/// Tells what changed since the last look, as a notification, and in the
/// journal at a priority journald reads from the prefix when it's run under
/// one: warnings for what broke and a sleep that went wrong, notices for
/// what's fixed.
fn tell(places: &Places, look: &Look) {
    let root = &places.root;
    let journal = into_the_journal();
    let say = |priority: u8, urgency: &str, line: &str| {
        if journal {
            eprintln!("<{priority}>{line}");
        }
        let _ = Command::new(&places.notify)
            .args(["-u", urgency, "-a", "Devices"])
            .arg(line)
            .stdin(Stdio::null())
            .status();
    };
    for (change, findings, priority, urgency) in [
        (Change::Broke, &look.broke, 4, "critical"),
        (Change::Fixed, &look.fixed, 5, "normal"),
    ] {
        for line in findings
            .iter()
            .filter_map(|finding| line(root, change, finding))
        {
            say(priority, urgency, &line);
        }
    }
    for slept in &look.sleeps {
        say(4, "normal", &slept.line());
    }
}

/// Keeps what's wrong now, and the kernel's sleep counts, for the next look.
fn keep(places: &Places, look: &Look) -> io::Result<()> {
    let replace = |name: &str, text: &str| {
        let next = places.state.join(format!("{name}.{}", process::id()));
        fs::write(&next, text)?;
        fs::rename(&next, places.state.join(name))
    };
    if let Some(sleep) = look.sleep {
        replace("sleep", &sleep.kept())?;
    }
    let text = look.now.iter().fold(String::new(), |mut text, finding| {
        // Writing to a String can't fail.
        _ = writeln!(text, "{finding}");
        text
    });
    replace("found", &text)
}

/// A finding as facts: its kind, what it's of, and for a disk what it
/// reported, null for the rest.
fn finding_json(finding: &str) -> Value {
    let (kind, subject) = finding.split_once('\t').unwrap_or(("", finding));
    let (subject, report) = match subject.split_once(" reports ") {
        Some((disk, report)) if kind == "disk" => (disk, Some(report)),
        _ => (subject, None),
    };
    json!({"kind": kind, "subject": subject, "report": report})
}

/// The answer for a script: what's wrong now, and what changed since the
/// last look.
fn answer_json(look: &Look) -> Value {
    let findings = |found: &mut dyn Iterator<Item = &String>| -> Vec<Value> {
        found.map(|finding| finding_json(finding)).collect()
    };
    json!({
        "schema": "device-check.look/1",
        "wrong": findings(&mut look.now.iter()),
        "broke": findings(&mut look.broke.iter()),
        "fixed": findings(&mut look.fixed.iter()),
        "sleeps": look.sleeps.iter().map(Slept::json).collect::<Vec<_>>(),
    })
}

/// The answer for a person: what's wrong now, a line each, or that
/// nothing is.
fn answer_text(look: &Look, root: &OsStr, pen: &Pen, width: Room) -> String {
    let wrong: Vec<String> = look
        .now
        .iter()
        .filter_map(|finding| line(root, Change::Broke, finding))
        .collect();
    if wrong.is_empty() {
        return format!(
            "{}{}\n",
            pen.marked("done", "success"),
            pen.role("text", "Nothing has stopped working")
        );
    }
    wrong.iter().fold(String::new(), |mut text, line| {
        // Writing to a String can't fail.
        _ = writeln!(
            text,
            "{}{}",
            pen.marked("caution", "warning"),
            pen.role("text", &layout::hung(line, layout::MARK, width))
        );
        text
    })
}

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "device-check",
        "names the devices that have stopped working, and what works again",
    )
    .example(
        "device-check",
        "what's wrong now, telling the desktop what changed since the last look",
    )
    .example("device-check --json", "what's wrong now, for a script")
    .exit(Exit::Failed, "it couldn't read or keep what it found")
    .exit(Exit::Config, "there's nowhere to keep what it finds")
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        if let Some(word) = context
            .args
            .iter()
            .find(|arg| !matches!(arg.as_str(), "--verbose" | "-v"))
        {
            return Err(
                Failure::new(Exit::Usage, format!("{word} isn't taken here"))
                    .because("device-check takes nothing but the flags every command takes")
                    .try_this("device-check --help"),
            );
        }
        let places = Places::here()?;
        let couldnt = |what: &'static str| {
            let state = places.state.display().to_string();
            move |error: io::Error| {
                Failure::new(Exit::Failed, what)
                    .because(error.to_string())
                    .try_this(format!("check that {state} can be written"))
            }
        };
        fs::create_dir_all(&places.state).map_err(couldnt("it couldn't make its folder"))?;
        let look = look(&places).map_err(couldnt("it couldn't read what it last found"))?;
        tell(&places, &look);
        keep(&places, &look).map_err(couldnt("it couldn't keep what it found"))?;
        let said = if context.format().is_json() {
            context.answer(&answer_json(&look))
        } else {
            context.say(&answer_text(
                &look,
                &places.root,
                &context.out,
                context.room,
            ))
        };
        // Nothing is left to say if stdout has closed.
        let _ = said;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs_as_find_names_them() {
        assert!(glob(b"phy*", b"phy0"));
        assert!(glob(b"phy*", b"phy"));
        assert!(!glob(b"phy*", b"wlan0"));
        assert!(glob(b"hci?", b"hci0"));
        assert!(!glob(b"hci?", b"hci10"));
        assert!(glob(b"video[0-9]", b"video3"));
        assert!(!glob(b"video[!0-9]", b"video3"));
        assert!(glob(b"card[]x]", b"card]"));
        assert!(glob(b"a\\*", b"a*"));
        assert!(!glob(b"a\\*", b"ab"));
        assert!(glob(b"x[y", b"x[y"));
        assert!(glob(b"hci0", b"hci0"));
        assert!(!glob(b"hci0", b"hci01"));
    }

    #[test]
    fn names_agree_with_their_verb() {
        let root = OsStr::new("/nonexistent");
        for (name, verb) in [
            ("The USB-C ports", "are"),
            ("The thermal sensors", "are"),
            ("The firmware's settings", "are"),
            ("Bluetooth's adapter", "is"),
            ("The camera's image processor", "is"),
            ("The bus", "is"),
            ("The glass", "is"),
            ("The TPM", "is"),
        ] {
            assert_eq!(
                line(root, Change::Broke, &format!("missing\t{name}")),
                Some(format!("{name} {verb} missing")),
                "{name}"
            );
        }
    }

    #[test]
    fn says_each_finding_as_it_broke() {
        let root = OsStr::new("/nonexistent");
        assert_eq!(
            line(root, Change::Broke, "power\t0000:00:05.0 (intel-ipu7)").as_deref(),
            Some(
                "0000:00:05.0 (intel-ipu7) failed a power change and won't wake until the next boot"
            )
        );
        assert_eq!(
            line(root, Change::Broke, "missing\tBluetooth").as_deref(),
            Some("Bluetooth is missing")
        );
        assert_eq!(
            line(root, Change::Broke, "disk\t/dev/nvme0 reports Low spare").as_deref(),
            Some("The disk /dev/nvme0 reports Low spare")
        );
        assert_eq!(
            line(root, Change::Broke, "kernel\tstuck task").as_deref(),
            Some("A task was stuck in the kernel for over 120 seconds")
        );
    }

    #[test]
    fn says_each_finding_as_it_was_fixed() {
        let root = OsStr::new("/nonexistent");
        assert_eq!(
            line(root, Change::Fixed, "power\t0000:00:05.0 (intel-ipu7)").as_deref(),
            Some("0000:00:05.0 (intel-ipu7) wakes again")
        );
        assert_eq!(
            line(root, Change::Fixed, "missing\tBluetooth").as_deref(),
            Some("Bluetooth is back")
        );
        assert_eq!(
            line(root, Change::Fixed, "missing\tThe speakers").as_deref(),
            Some("The speakers are back")
        );
        assert_eq!(
            line(root, Change::Fixed, "disk\t/dev/nvme0 reports a reports b").as_deref(),
            Some("The disk /dev/nvme0 no longer reports a reports b")
        );
        assert_eq!(line(root, Change::Fixed, "kernel\toops"), None);
    }

    #[test]
    fn a_finding_is_told_in_facts() {
        assert_eq!(
            finding_json("disk\t/dev/nvme0 reports Low spare"),
            json!({"kind": "disk", "subject": "/dev/nvme0", "report": "Low spare"}),
            "a disk's finding parts the disk from its report"
        );
        assert_eq!(
            finding_json("missing\tThe speakers reports nothing"),
            json!({"kind": "missing", "subject": "The speakers reports nothing",
                   "report": null}),
            "only a disk's finding has a report"
        );
    }

    #[test]
    fn a_person_reads_whats_wrong_now() {
        let root = OsStr::new("/nonexistent");
        let look = |now: &[&str]| Look {
            now: now.iter().map(|finding| (*finding).to_owned()).collect(),
            broke: Vec::new(),
            fixed: Vec::new(),
            sleeps: Vec::new(),
            sleep: None,
        };
        let width = Room::of_stdout(&terminal_kit::Environment::of([false; 3], &[]));
        let pen = Pen::plain();
        let done = pen.marked("done", "success");
        let caution = pen.marked("caution", "warning");
        assert_eq!(
            answer_text(&look(&[]), root, &pen, width),
            format!("{done}Nothing has stopped working\n"),
            "with nothing wrong it says so"
        );
        assert_eq!(
            answer_text(
                &look(&["missing\tBluetooth", "kernel\toops"]),
                root,
                &pen,
                width
            ),
            format!(
                "{caution}The kernel hit a fault and may misbehave until the next boot\n\
                 {caution}Bluetooth is missing\n"
            ),
            "each thing wrong is a line, marked"
        );
    }

    #[test]
    fn reads_trouble_only_from_its_own_taints() {
        let dir = env::temp_dir().join(format!("device-check-test-{}", process::id()));
        let kernel = dir.join("proc/sys/kernel");
        fs::create_dir_all(&kernel).unwrap();
        fs::write(kernel.join("tainted"), format!("{}\n", 512 + 4096 + 1024)).unwrap();
        fs::write(kernel.join("hung_task_detect_count"), "0\n").unwrap();
        assert_eq!(
            kernel_trouble(dir.as_os_str()),
            [] as [std::string::String; 0]
        );
        fs::write(
            kernel.join("tainted"),
            format!("{}\n", 128 + 16384 + 16 + 32),
        )
        .unwrap();
        fs::write(kernel.join("hung_task_detect_count"), "3\n").unwrap();
        assert_eq!(
            kernel_trouble(dir.as_os_str()),
            [
                "kernel\toops",
                "kernel\tlockup",
                "kernel\tmachine check",
                "kernel\tbad page",
                "kernel\tstuck task"
            ]
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
