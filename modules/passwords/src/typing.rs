//! Whether a secret may be typed into the window the launcher opened over,
//! as Type the Password does while copying is off (ISXEG8's terms for
//! R-type-password, 2026-11-24). A typed password reaches whatever reads
//! that window's keys, so the daemon types only into the window Joel
//! chose, once the launcher has let go of the keys, and never into one an
//! agent reads: a model would see the password and send it on to its
//! provider. A window is an agent's when an agent's process is behind it,
//! among its forebears or below it (ISXEG8, 2026-11-24): pi, pi-spawner,
//! which runs its commands, or mycelium, which holds and attaches pi
//! sessions; the hosted runtime, which runs sessions as node; or anything
//! in the agents' or jobs' slices, where a hosted session and every
//! command it runs are kept. Below a window, a session reached by ssh and
//! a multiplexer, whose sessions run out of sight, are refused too. A
//! kitty, the one from the package the daemon's kitten comes from and
//! outside the jobs' slice, is asked what runs in its focused window
//! instead, since every kitty window shares the one kitty process, which
//! lives in the agents' slice itself: refused when what it names isn't
//! its own, when an agent runs in it or below it, when a multiplexer
//! stands in front, past which kitty can't see, and when kitty can't say.
//! An X11 window is refused, since niri knows only xwayland-satellite
//! behind it, and any X program can read its keys.
//!
//! This is no full guard, which is why the place stays off until agents
//! are fenced (ISXEG8's R-type-password-spawned, 2026-11-25). An agent
//! set on hiding can start a window through niri, or into a slice or
//! scope of its choosing, that nothing here ties to it; it can write a
//! type straight to the daemon through the shell's own pipes; and it can
//! drive the launcher through its IPC and a virtual keyboard, pressing
//! Return itself. The fence that keeps the shell's pipes, its IPC socket,
//! Wayland and niri's socket from agents closes all three.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

/// The layer the launcher draws on, as it names itself to niri, which
/// holds the keys while it's open.
const LAUNCHER: &str = "launcher";

/// How long the launcher has to let go of the keys before a type is
/// refused rather than typed into it, as long as a paste waits
/// (modules/quickshell/clipboard): on a loaded builder the launcher took
/// 0.9 s to close.
pub const LETTING_GO: Duration = Duration::from_secs(3);

/// How long kitty has to say what runs in a window.
const KITTY_ANSWERS: Duration = Duration::from_secs(2);

/// The programs that are a session, or hold one: pi itself, and mycelium,
/// which holds pi sessions and attaches a window to one running elsewhere.
const SESSIONS: [&str; 2] = ["pi", "mycelium"];

/// The programs that are an agent's: the sessions, and pi-spawner, which
/// runs a session's commands.
const AGENTS: [&str; 3] = ["pi", "pi-spawner", "mycelium"];

/// The hosted runtime, in a program's path or arguments: it runs pi
/// sessions as node, a name that says nothing.
const RUNTIME: &str = "pi-host-runtime";

/// The slices agents run in: hosted sessions and the sessions' kitty in
/// the agents', every command a session runs in the jobs'.
const AGENTS_SLICE: &str = "/agents.slice/";
const JOBS_SLICE: &str = "/jobs.slice/";

/// The programs that hold a terminal's sessions out of kitty's sight.
const MULTIPLEXERS: [&str; 3] = ["tmux", "zellij", "screen"];

/// kitty's process, by the name Linux gives it: Nix's wrapper renames it.
const KITTIES: [&str; 2] = ["kitty", ".kitty-wrapped"];

/// The programs behind X11's windows, by the names Linux gives them
/// (xwayland-satellite cut to 15 letters): niri knows only the one
/// speaking for every X program, and any X program can read the keys
/// typed into another's window.
const X11: [&str; 3] = ["xwayland-satell", "xwayland-satellite", "Xwayland"];

/// Why a window is refused for what runs below it.
const AGENT_RUNS: &str = "an agent runs in that window";
const UNSEEN: &str = "a multiplexer runs in that window, past which nothing can see";
const UNREAD: &str = "what runs in that window can't be read";

/// Why a type is refused while the keys are elsewhere: in no window, as
/// while 1Password's own prompt for Joel's leave closes, or another one.
pub const NOWHERE: &str = "no window has the keys";
pub const MOVED: &str = "the keys went to another window";

/// Whether a refusal is the keys being elsewhere, which may pass: once
/// 1Password's prompt goes, niri gives them back to the window before it.
#[must_use]
pub fn elsewhere(why: &str) -> bool {
    why == NOWHERE || why == MOVED
}

/// The window the keys are in, as niri says it.
#[derive(Clone, Debug, PartialEq)]
pub struct Window {
    pub id: u64,
    pub pid: Option<u32>,
    pub app_id: String,
}

/// Asks niri one question, `ask` as its socket takes it, and answers its
/// reply, or none where niri can't be asked.
fn asked_niri(socket: &Path, ask: &[u8]) -> Option<String> {
    let mut stream = UnixStream::connect(socket).ok()?;
    stream.set_read_timeout(Some(LETTING_GO)).ok()?;
    stream.write_all(ask).ok()?;
    let mut answer = String::new();
    BufReader::new(stream).read_line(&mut answer).ok()?;
    Some(answer)
}

/// Whether niri's answer to `Layers` says the launcher holds no keys:
/// none when it isn't an answer to that.
#[must_use]
pub fn let_go(answer: &str) -> Option<bool> {
    let answer: Value = serde_json::from_str(answer).ok()?;
    let layers = answer.get("Ok")?.get("Layers")?.as_array()?;
    Some(!layers.iter().any(|layer| {
        layer.get("namespace").and_then(Value::as_str) == Some(LAUNCHER)
            && layer.get("keyboard_interactivity").and_then(Value::as_str) != Some("None")
    }))
}

/// The window niri's answer to `FocusedWindow` says has the keys: an
/// inner none where none has, an outer none when it isn't an answer.
#[must_use]
pub fn focused(answer: &str) -> Option<Option<Window>> {
    let answer: Value = serde_json::from_str(answer).ok()?;
    let window = answer.get("Ok")?.get("FocusedWindow")?;
    if window.is_null() {
        return Some(None);
    }
    Some(Some(Window {
        id: window.get("id")?.as_u64()?,
        pid: window
            .get("pid")
            .and_then(Value::as_u64)
            .and_then(|pid| u32::try_from(pid).ok()),
        app_id: window
            .get("app_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned(),
    }))
}

/// Waits until niri says the launcher has let go of the keys, asking
/// afresh each few milliseconds, as niri answers one question a
/// connection. Answers whether it did within `LETTING_GO`.
#[must_use]
pub fn keys_let_go(niri: &Path) -> bool {
    let deadline = Instant::now() + LETTING_GO;
    while Instant::now() < deadline {
        if asked_niri(niri, b"\"Layers\"\n")
            .as_deref()
            .and_then(let_go)
            == Some(true)
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    false
}

/// The window niri says has the keys now, or why that can't be told.
///
/// # Errors
///
/// Where niri can't be asked or gave no answer it could read.
pub fn focused_now(niri: &Path) -> Result<Option<Window>, String> {
    asked_niri(niri, b"\"FocusedWindow\"\n")
        .as_deref()
        .and_then(focused)
        .ok_or_else(|| "niri didn't say which window has the keys".to_owned())
}

/// A process's name as Linux gives it, or "" for one that's gone.
fn program(pid: u32) -> String {
    std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .map(|comm| comm.trim_end().to_owned())
        .unwrap_or_default()
}

/// A process's parent, or none for the first or one that's gone. Its
/// name, in brackets, may hold anything, so it's read past the last.
fn parent(pid: u32) -> Option<u32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after = &stat[stat.rfind(')')? + 1..];
    after.split_whitespace().nth(1)?.parse().ok()
}

/// A process's arguments, its program first, or none for one that's gone.
fn arguments(pid: u32) -> Vec<String> {
    std::fs::read(format!("/proc/{pid}/cmdline"))
        .map(|line| {
            line.split(|byte| *byte == 0)
                .filter(|word| !word.is_empty())
                .map(|word| String::from_utf8_lossy(word).into_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// A process's control group as Linux gives it, or "" for one that's gone.
fn cgroup(pid: u32) -> String {
    std::fs::read_to_string(format!("/proc/{pid}/cgroup")).unwrap_or_default()
}

/// Whether a control group, as /proc gives it, is an agent's: in the jobs'
/// slice, or the agents' slice but for a scope kitty makes for one of its
/// windows, which holds whatever Joel starts there.
#[must_use]
pub fn agents_group(group: &str) -> bool {
    group.lines().any(|line| {
        let path = line.splitn(3, ':').nth(2).unwrap_or("");
        let unit = path.rsplit('/').next().unwrap_or("");
        path.contains(JOBS_SLICE) || (path.contains(AGENTS_SLICE) && !kittys_window(unit))
    })
}

/// Whether a unit is a scope kitty makes for a window: `kitty-<pid>-<n>.scope`.
fn kittys_window(unit: &str) -> bool {
    unit.strip_prefix("kitty-")
        .and_then(|rest| rest.strip_suffix(".scope"))
        .and_then(|rest| rest.split_once('-'))
        .is_some_and(|(pid, n)| {
            [pid, n]
                .iter()
                .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
        })
}

/// Whether a process named so, with these arguments, in this control
/// group, is an agent's. kitty's own group is passed as "", since the
/// sessions' kitty lives in the agents' slice and is asked what it runs.
#[must_use]
pub fn agents(name: &str, words: &[String], group: &str) -> bool {
    AGENTS.contains(&name) || words.iter().any(|word| word.contains(RUNTIME)) || agents_group(group)
}

/// Whether a control group, as /proc gives it, is in the jobs' slice,
/// where only an agent's commands run.
fn in_jobs(group: &str) -> bool {
    group.lines().any(|line| {
        line.splitn(3, ':')
            .nth(2)
            .unwrap_or("")
            .contains(JOBS_SLICE)
    })
}

/// Whether the process at `pid` is an agent's, as /proc says. kitty's
/// own is judged by the jobs' slice alone, since the sessions' kitty
/// lives in the agents' slice.
fn an_agent(pid: u32) -> bool {
    let name = program(pid);
    let group = cgroup(pid);
    let group = if KITTIES.contains(&name.as_str()) && !in_jobs(&group) {
        String::new()
    } else {
        group
    };
    let mut words = arguments(pid);
    if let Ok(path) = std::fs::read_link(format!("/proc/{pid}/exe")) {
        words.push(path.to_string_lossy().into_owned());
    }
    agents(&name, &words, &group)
}

/// The process at `pid` or among its forebears that is an agent's, up to
/// the first, or none.
#[must_use]
pub fn session_above(pid: u32) -> Option<u32> {
    let mut at = Some(pid);
    // A loop in what /proc says can't be, but is never followed forever.
    for _ in 0..4096 {
        let here = at?;
        if an_agent(here) {
            return Some(here);
        }
        at = parent(here).filter(|up| *up != 0 && *up != here);
    }
    None
}

/// Whether `pid` descends from `forebear`, as kitty's own window's
/// processes do.
fn descends(pid: u32, forebear: u32) -> bool {
    let mut at = parent(pid);
    for _ in 0..4096 {
        match at {
            Some(up) if up == forebear => return true,
            Some(up) if up != 0 => at = parent(up),
            _ => return false,
        }
    }
    false
}

/// Why nothing may be typed for what runs anywhere below `pid`, or none:
/// an agent, or a session held or reached by any word naming one, as ssh
/// running mycelium on another machine does, or a multiplexer, whose
/// sessions run beside the terminal rather than below it. A terminal
/// other than kitty can't be asked what it runs, and what kitty says runs
/// in its window may have more below it, so both are read this way.
fn below(pid: u32) -> Option<&'static str> {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Some(UNREAD);
    };
    let parents: Vec<(u32, u32)> = entries
        .filter_map(|entry| entry.ok()?.file_name().to_str()?.parse().ok())
        .filter_map(|child| Some((child, parent(child)?)))
        .collect();
    let mut below = vec![pid];
    let mut seen = 0;
    while seen < below.len() {
        let at = below[seen];
        seen += 1;
        for (child, _) in parents
            .iter()
            .filter(|(child, up)| *up == at && *child != at)
        {
            let (name, words) = (program(*child), arguments(*child));
            if an_agent(*child) || a_sessions(&name, &words) {
                return Some(AGENT_RUNS);
            }
            if multiplexes(&name, &words) {
                return Some(UNSEEN);
            }
            if !below.contains(child) {
                below.push(*child);
            }
        }
    }
    None
}

/// Whether a process named so is one of X11's, behind every X window.
#[must_use]
pub fn x11s(name: &str) -> bool {
    X11.contains(&name)
}

/// Whether a program named so, with these arguments, is a multiplexer,
/// whose sessions kitty can't see past: tmux names its client "tmux: client".
#[must_use]
pub fn multiplexes(name: &str, words: &[String]) -> bool {
    let one = |word: &str| {
        let base = word.rsplit('/').next().unwrap_or(word);
        MULTIPLEXERS
            .iter()
            .any(|multiplexer| base == *multiplexer || base.starts_with(&format!("{multiplexer}:")))
    };
    one(name) || words.first().is_some_and(|first| one(first))
}

/// Whether a program named so, with these arguments, is a session or
/// holds one: pi or mycelium by its name, or by any word naming one, as
/// ssh running mycelium on another machine does.
#[must_use]
pub fn a_sessions(name: &str, words: &[String]) -> bool {
    let named = |word: &str| {
        let base = word.rsplit('/').next().unwrap_or(word);
        SESSIONS.contains(&base)
    };
    named(name) || words.iter().any(|word| named(word))
}

/// Whether a process named so, run from `exe`, in this control group, is
/// a kitty to ask what it runs: named as kitty is, run from the package
/// the daemon's kitten comes from, `from`, and outside the jobs' slice,
/// where only an agent's commands run. Anything else named kitty, as a
/// program renamed to answer for it, is refused outright.
#[must_use]
pub fn kitty_believed(name: &str, exe: &str, group: &str, from: Option<&Path>) -> bool {
    KITTIES.contains(&name)
        && from.is_some_and(|from| Path::new(exe).starts_with(from))
        && !in_jobs(group)
}

/// The package a kitten comes from, two folders above it, which kitty's
/// own program is in too.
#[must_use]
pub fn kittys_package(kitten: &str) -> Option<&Path> {
    Path::new(kitten)
        .parent()?
        .parent()
        .filter(|from| from.components().count() > 1)
}

/// Whether the process at `pid` is a kitty to ask, as `kitty_believed` says.
fn kitty(pid: u32, from: Option<&Path>) -> bool {
    let exe = std::fs::read_link(format!("/proc/{pid}/exe"))
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    kitty_believed(&program(pid), &exe, &cgroup(pid), from)
}

/// Where kitty at `pid` listens for remote control: what it was started
/// with, else the address kitty.conf's `listen_on` names by its pid.
#[must_use]
pub fn listens_on(words: &[String], pid: u32) -> String {
    let mut given = words.iter();
    while let Some(word) = given.next() {
        if let Some(address) = word.strip_prefix("--listen-on=") {
            return address.to_owned();
        }
        if word == "--listen-on"
            && let Some(address) = given.next()
        {
            return address.clone();
        }
    }
    format!("unix:@kitty-{pid}")
}

/// What kitty's `ls` says runs in the window that has its keys: each
/// foreground process's id and arguments, from its focused OS window's
/// active tab's active window. None where it names no such window.
#[must_use]
pub fn foreground(listed: &str) -> Option<Vec<(Option<u32>, Vec<String>)>> {
    let listed: Value = serde_json::from_str(listed).ok()?;
    let flag = |of: &Value, name: &str| of.get(name).and_then(Value::as_bool) == Some(true);
    let os = listed
        .as_array()?
        .iter()
        .find(|os| flag(os, "is_focused"))?;
    let tab = os
        .get("tabs")?
        .as_array()?
        .iter()
        .find(|tab| flag(tab, "is_focused") || flag(tab, "is_active"))?;
    let window = tab
        .get("windows")?
        .as_array()?
        .iter()
        .find(|window| flag(window, "is_focused") || flag(window, "is_active"))?;
    let processes = window.get("foreground_processes")?.as_array()?;
    Some(
        processes
            .iter()
            .map(|process| {
                let pid = process
                    .get("pid")
                    .and_then(Value::as_u64)
                    .and_then(|pid| u32::try_from(pid).ok());
                let words = process
                    .get("cmdline")
                    .and_then(Value::as_array)
                    .map(|words| {
                        words
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_owned)
                            .collect()
                    })
                    .unwrap_or_default();
                (pid, words)
            })
            .collect(),
    )
}

/// What kitty at `pid` says runs in its focused window, asked through
/// `kitty`, the program, within `KITTY_ANSWERS`.
fn kitty_says(kitty: &str, pid: u32) -> Result<String, String> {
    let address = listens_on(&arguments(pid), pid);
    let mut asked = Command::new(kitty)
        .args(["@", "--to", &address, "ls"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|why| format!("kitty can't be asked what runs in it: {why}"))?;
    let mut out = asked.stdout.take();
    let reading = std::thread::spawn(move || {
        let mut said = String::new();
        if let Some(out) = out.as_mut() {
            let _ = out.read_to_string(&mut said);
        }
        said
    });
    let deadline = Instant::now() + KITTY_ANSWERS;
    loop {
        match asked.try_wait() {
            Ok(Some(ended)) if ended.success() => break,
            Ok(Some(_)) => return Err("kitty didn't say what runs in it".to_owned()),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            _ => {
                let _ = asked.kill();
                let _ = asked.wait();
                return Err("kitty didn't say what runs in it in time".to_owned());
            }
        }
    }
    reading
        .join()
        .map_err(|_| "kitty didn't say what runs in it".to_owned())
}

/// Whether a secret may be typed into `window`, the one the launcher
/// opened over, as niri says it has the keys now: answers its app id, or
/// why not with the app id as far as it's known. `kitty_program` is the
/// program to ask a terminal with, none where none is named, which
/// refuses any terminal.
///
/// # Errors
///
/// Where another window has the keys, niri says not whose it is, an agent
/// reads it, or it's a terminal that runs an agent, a multiplexer, or
/// can't say what it runs.
pub fn fit(
    niri: &Path,
    window: u64,
    kitty_program: Option<&str>,
) -> Result<String, (String, String)> {
    let unknown = |why: String| (String::new(), why);
    let Some(focused) = focused_now(niri).map_err(unknown)? else {
        return Err(unknown(NOWHERE.to_owned()));
    };
    let app = focused.app_id;
    let refused = |why: &str| Err((app.clone(), why.to_owned()));
    if focused.id != window {
        return refused(MOVED);
    }
    let Some(pid) = focused.pid else {
        return refused("niri didn't say whose window it is");
    };
    let name = program(pid);
    if x11s(&name) {
        return refused("that window is X11's, where any X program can read its keys");
    }
    if session_above(pid).is_some() {
        return refused("an agent reads that window");
    }
    let Some(kitty_program) = kitty_program else {
        if KITTIES.contains(&name.as_str()) {
            return refused("kitty can't be asked what runs in it");
        }
        return below(pid).map_or(Ok(app.clone()), refused);
    };
    if !kitty(pid, kittys_package(kitty_program)) {
        if KITTIES.contains(&name.as_str()) {
            return refused("that kitty isn't the one kitten asks");
        }
        return below(pid).map_or(Ok(app.clone()), refused);
    }
    let said = match kitty_says(kitty_program, pid) {
        Ok(said) => said,
        Err(why) => return refused(&why),
    };
    let Some(running) = foreground(&said) else {
        return refused("kitty didn't say what runs in its window");
    };
    // What kitty says runs there must be kitty's own, each by its id, or
    // whatever answered as kitty could name anything.
    if running.is_empty()
        || running
            .iter()
            .any(|(process, _)| process.is_none_or(|process| !descends(process, pid)))
    {
        return refused("kitty named nothing of its own running in its window");
    }
    let named = |pid: &Option<u32>| pid.map(program).unwrap_or_default();
    if running
        .iter()
        .any(|(pid, words)| multiplexes(&named(pid), words))
    {
        return refused("kitty can't see past the multiplexer in that terminal");
    }
    let session = running.iter().any(|(pid, words)| {
        a_sessions(&named(pid), words) || pid.and_then(session_above).is_some()
    });
    if session {
        return refused("an agent runs in that terminal");
    }
    match running
        .iter()
        .find_map(|(process, _)| process.and_then(below))
    {
        Some(UNSEEN) => refused("kitty can't see past the multiplexer in that terminal"),
        Some(UNREAD) => refused(UNREAD),
        Some(_) => refused("an agent runs in that terminal"),
        None => Ok(app),
    }
}

/// One journal line for one type, as journald's native socket takes it:
/// the entry's title, the window's app id and how it ended, never the
/// secret. Each value is made one line, since a newline would start
/// another field.
#[must_use]
pub fn journal_line(title: &str, app: &str, outcome: &str, why: &str) -> String {
    let plain = |value: &str| value.replace(|c: char| c.is_control(), " ");
    let (title, app, why) = (plain(title), plain(app), plain(why));
    // A type refused before any window was read has no app to name.
    let into = if app.is_empty() {
        String::new()
    } else {
        format!(" into {app}")
    };
    let message = if why.is_empty() {
        format!("{outcome} {title:?}{into}")
    } else {
        format!("{outcome} {title:?}{into}: {why}")
    };
    [
        ("SYSLOG_IDENTIFIER", "passwords".to_owned()),
        (
            "PRIORITY",
            if why.is_empty() { "6" } else { "5" }.to_owned(),
        ),
        ("MESSAGE", message),
        ("PASSWORDS_TYPED_TITLE", title),
        ("PASSWORDS_TYPED_APP_ID", app),
        ("PASSWORDS_TYPED", outcome.to_owned()),
    ]
    .iter()
    .fold(String::new(), |mut said, (field, value)| {
        use std::fmt::Write as _;
        let _ = writeln!(said, "{field}={value}");
        said
    })
}

#[cfg(test)]
#[path = "typing_tests.rs"]
mod tests;
