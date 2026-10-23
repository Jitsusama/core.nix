//! Names the devices that have stopped working: any whose runtime power
//! management failed, which the kernel then holds failed and won't wake until
//! the next boot, any the machine expects that isn't there, and any disk that
//! reported a fault to smartd this boot, which device-check-disk keeps. And the
//! kernel itself, when it oopsed, locked up, heard of a hardware fault or had a
//! task stuck in it, since it can misbehave until the next boot and only says
//! so in its log.
//!
//!   device-check    names what has gone wrong since it last ran, and what
//!                   works again, in the journal and on the desktop
//!
//! Each is named once, so running it often costs nothing to read. It exits 0
//! whatever it finds, and non-zero only when it couldn't look.
//!
//! `DEVICE_CHECK_ROOT` puts a directory in front of /sys, /proc and /run,
//! `DEVICE_CHECK_EXPECTED` names the expected devices' file, `DEVICE_CHECK_STATE`
//! where it keeps what it found, and `DEVICE_CHECK_NOTIFY` the program that
//! notifies, for tests.

use std::collections::BTreeSet;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{self, Command, ExitCode, Stdio};

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

/// The line to say for a finding, or None when nobody needs telling: only
/// a new boot clears the kernel's trouble.
fn line(root: &OsStr, change: Change, finding: &str) -> Option<String> {
    let (kind, subject) = finding.split_once('\t')?;
    Some(match (kind, change) {
        ("power", Change::Broke) => {
            format!("{subject} failed a power change and won't wake until the next boot")
        }
        ("power", Change::Fixed) => format!("{subject} wakes again"),
        ("missing", Change::Broke) => format!("{subject} is missing"),
        ("missing", Change::Fixed) => format!("{subject} is back"),
        ("disk", Change::Broke) => format!("The disk {subject}"),
        ("disk", Change::Fixed) => format!(
            "The disk {}",
            subject.replacen(" reports ", " no longer reports ", 1)
        ),
        ("kernel", Change::Broke) => trouble(root, subject)?,
        _ => return None,
    })
}

/// Says each finding, warnings for what broke and notices for what's fixed,
/// whose priority journald reads from the prefix.
fn tell<'a>(
    root: &OsStr,
    notify: &OsStr,
    change: Change,
    findings: impl Iterator<Item = &'a String>,
) {
    let (priority, urgency) = match change {
        Change::Broke => (4, "critical"),
        Change::Fixed => (5, "normal"),
    };
    for finding in findings {
        let Some(line) = line(root, change, finding) else {
            continue;
        };
        eprintln!("<{priority}>{line}");
        let _ = Command::new(notify)
            .args(["-u", urgency, "-a", "Devices"])
            .arg(&line)
            .stdin(Stdio::null())
            .status();
    }
}

fn look() -> io::Result<()> {
    let root = env::var_os("DEVICE_CHECK_ROOT").unwrap_or_default();
    let expected = env::var_os("DEVICE_CHECK_EXPECTED")
        .map_or_else(|| PathBuf::from("/etc/device-check"), PathBuf::from);
    let state = match env::var_os("DEVICE_CHECK_STATE") {
        Some(state) => PathBuf::from(state),
        None => match env::var_os("XDG_RUNTIME_DIR").filter(|dir| !dir.is_empty()) {
            Some(runtime) => Path::new(&runtime).join("device-check"),
            None => return Err(io::Error::other("XDG_RUNTIME_DIR isn't set")),
        },
    };
    let notify = env::var_os("DEVICE_CHECK_NOTIFY").unwrap_or_else(|| OsString::from(NOTIFY_SEND));

    fs::create_dir_all(&state)?;
    let kept = state.join("found");
    let found: BTreeSet<String> = match fs::read_to_string(&kept) {
        Ok(text) => text.lines().map(str::to_owned).collect(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => BTreeSet::new(),
        Err(error) => return Err(error),
    };
    let now: BTreeSet<String> = failed_power(&root)
        .into_iter()
        .chain(missing(&root, &expected))
        .chain(disk_faults(&root))
        .chain(kernel_trouble(&root))
        .collect();

    tell(&root, &notify, Change::Broke, now.difference(&found));
    tell(&root, &notify, Change::Fixed, found.difference(&now));

    let next = state.join(format!("now.{}", process::id()));
    let text = now.iter().fold(String::new(), |mut text, finding| {
        // Writing to a String can't fail.
        _ = writeln!(text, "{finding}");
        text
    });
    fs::write(&next, text)?;
    fs::rename(&next, &kept)
}

fn main() -> ExitCode {
    match look() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("device-check: couldn't look: {error}");
            ExitCode::FAILURE
        }
    }
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
            line(root, Change::Fixed, "disk\t/dev/nvme0 reports a reports b").as_deref(),
            Some("The disk /dev/nvme0 no longer reports a reports b")
        );
        assert_eq!(line(root, Change::Fixed, "kernel\toops"), None);
    }

    #[test]
    fn reads_trouble_only_from_its_own_taints() {
        let dir = env::temp_dir().join(format!("device-check-test-{}", process::id()));
        let kernel = dir.join("proc/sys/kernel");
        fs::create_dir_all(&kernel).unwrap();
        fs::write(kernel.join("tainted"), format!("{}\n", 512 + 4096 + 1024)).unwrap();
        fs::write(kernel.join("hung_task_detect_count"), "0\n").unwrap();
        assert!(kernel_trouble(dir.as_os_str()).is_empty());
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
