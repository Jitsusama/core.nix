//! One entry of the journal as `journalctl -o json` writes it, cut down to
//! what the day is tallied by: when, how grave, which thing said it, and
//! what it said; and the journalctl followers that read them.

use std::process::Command;

use serde_json::{Map, Value};

use crate::asks::{self, Ask};
use crate::{launcher, launches};

/// The fields each entry is read by; journalctl always adds its time.
const FIELDS: &str = "MESSAGE,PRIORITY,SYSLOG_IDENTIFIER,_COMM,USER_UNIT,UNIT,_SYSTEMD_UNIT,\
                      _SYSTEMD_USER_UNIT,_SYSTEMD_CGROUP,COREDUMP_COMM,COREDUMP_CGROUP,MESSAGE_ID,\
                      _TRANSPORT,\
                      KERNEL_USEC,INITRD_USEC,USERSPACE_USEC,ASK_OUTCOME,ASK_SHOWN_MS,\
                      ASK_ITEM_HASH,ASK_ARGV_HASH,ASK_AVOIDABLE";

/// systemd's message for a unit whose restart is scheduled, the one line a
/// restart always leaves, at a priority the warning reader doesn't see.
pub const RESTART_SCHEDULED: &str = "5eb03494b6584870a536b337290809b3";

/// systemd's message once the machine has started, saying how long each
/// stage took.
pub const STARTUP_FINISHED: &str = "b07a249cd024414a82dd00cd181378ff";

/// logind's message as the machine starts to power down or reboot.
pub const SHUTTING_DOWN: &str = "98268866d1d54a499c4e98921d93bc40";

/// systemd's message for a unit whose start finished.
pub const UNIT_STARTED: &str = "39f53479d3a045ac8e11786248231fbf";

/// systemd's message for a unit it starts to stop.
pub const UNIT_STOPPING: &str = "de5b426a63be47a7b6ac3eaac82e2f6f";

/// What the launcher says once its rows are made after the shell starts,
/// the moment it's ready to use: "launcher: home and 7 places kept as the
/// shell started", or with no places, as `journalctl --grep` finds it.
pub const LAUNCHER_READY: &str = "launcher: home (and [0-9]+ places )?kept as the shell started";

/// His shell's unit, as modules/quickshell/home.nix names it, in his
/// session's slice.
pub const SHELL: &str = "quickshell.service";

/// Whether `message` is the launcher come ready.
fn is_ready(message: &str) -> bool {
    message
        .split_once("launcher: home ")
        .is_some_and(|(_, rest)| rest.contains("kept as the shell started"))
}

/// Where logind runs, the only teller of the machine going down.
pub(crate) const LOGIND: &str = "/system.slice/systemd-logind.service";

/// Where systemd-coredump runs, a unit for each crash it tells.
const COREDUMP_SLICE: &str = "/system.slice/system-systemd\\x2dcoredump.slice/";

/// The unit Plymouth runs once a disk asks for its password.
const PASSWORD_ASKED: &str = "systemd-ask-password-plymouth.service";

/// The units that open an encrypted disk.
const DISK_OPENER: &str = "systemd-cryptsetup@";

/// A journal entry, as the day takes it.
#[derive(Debug, PartialEq, Eq)]
pub struct Entry {
    /// When it was written, in microseconds since the epoch.
    pub at: i64,
    /// syslog's priority: 0 is an emergency, 3 an error, 4 a warning.
    pub priority: u8,
    /// Which thing said it: a program's identifier, or the unit systemd
    /// spoke of.
    pub thing: String,
    /// What it said.
    pub message: String,
    /// The unit that wrote it, a service or a program's scope; on a Mac,
    /// the app it ran from.
    pub ran_in: Option<String>,
    /// The program a core dump was of, when it's one.
    pub crashed: Option<String>,
    /// Whose a core dump's program was, when it's one.
    pub whose: Option<Whose>,
    /// Whether an agent or one of its commands wrote it, from the slice
    /// it ran in: what it says was never put to Joel.
    pub agents: bool,
    /// Where it was written, as far as what it may tell goes.
    pub origin: Origin,
    /// Whether it's systemd scheduling a unit's restart.
    pub restart: bool,
    /// Whether the kernel said it.
    pub kernel: bool,
    /// The boot it was said in.
    pub boot: String,
    /// When, in microseconds since the kernel started.
    pub since_boot: i64,
    /// The moment in a boot or a shutdown it marks, when it's one.
    pub mark: Option<Mark>,
    /// The ask for a secret it answered, when it's secret-asks's line.
    pub ask: Option<Ask>,
    /// What systemd did to the service it's about, when it's one.
    pub job: Option<Job>,
}

/// What systemd did to a service: stopping one, then starting it again, is
/// how a switch restarts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Job {
    /// It started to stop the service.
    Stopping,
    /// The service's start finished.
    Started,
}

/// A moment in starting or stopping the machine, each said once a boot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// The machine has started; how long each stage took, in microseconds.
    Started {
        /// The firmware, before the boot loader.
        firmware: i64,
        /// The boot loader.
        loader: i64,
        /// The kernel, before the initrd.
        kernel: i64,
        /// The initrd, opening the disk among it.
        initrd: i64,
        /// systemd starting the machine's services.
        userspace: i64,
    },
    /// A disk asked for its password.
    Asked,
    /// A disk opened.
    Opened,
    /// The launcher is ready.
    Ready,
    /// The machine started to power down or reboot.
    Stopping,
}

/// Where a line was written, which says what it may tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// A manager of systemd's, in its own init.scope: the only teller of a
    /// unit's restart, stop or start, a boot's stages and a reload.
    Systemd,
    /// His shell's own unit in his session's slice: the only teller of the
    /// launcher's timings and its being ready. A unit a job starts can't
    /// take that name while the shell's unit file is there.
    Shell,
    /// The jobs slice, where every command is run that agents start, that
    /// he waits on or that the timed work runs, and the builds' cache: none
    /// of them is his launcher or a program he opened.
    Jobs,
    /// Anywhere else with a cgroup: his programs, the agents themselves,
    /// the system's services.
    Elsewhere,
    /// No cgroup at all: the kernel's own lines, and those whose writer was
    /// gone before journald looked, which anything could have said.
    Unknown,
}

/// Whose a program is, from where systemd ran it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Whose {
    /// Joel's: in his programs', his session's or his awaited work's slice.
    His,
    /// Agents' and their tools', test browsers' included.
    Agents,
}

/// The entry one line of `journalctl -o json` holds, or none when the line
/// isn't one.
#[must_use]
pub fn parse(line: &str) -> Option<Entry> {
    let Ok(Value::Object(fields)) = serde_json::from_str::<Value>(line) else {
        return None;
    };
    let text = |name: &str| fields.get(name).and_then(said);
    let at = text("__REALTIME_TIMESTAMP")?.parse().ok()?;
    let cgroup = text("_SYSTEMD_CGROUP").unwrap_or_default();
    let agents = agents_work(&cgroup);
    let priority = text("PRIORITY").and_then(|p| p.parse().ok()).unwrap_or(6);
    let message = text("MESSAGE").unwrap_or_default();
    let identifier = text("SYSLOG_IDENTIFIER").or_else(|| text("_COMM"));
    let unit = text("USER_UNIT").or_else(|| text("UNIT"));
    let asked_joel = identifier.as_deref() == Some(asks::IDENTIFIER) && !agents_ran(&cgroup);
    let ask = asked_joel.then(|| asks::of(text)).flatten();
    // What systemd and logind tell (a unit's restart, stop or start, a
    // boot's stages, the machine going down) is taken only from where they
    // write it. Any command can write those fields, and a job can name the
    // slice a unit it starts runs in, but no unit it starts runs in a
    // manager's own init.scope or as logind. A process of his account can
    // still write itself into that init.scope's cgroup, and nothing here
    // stops that: this keeps out what commands say from where they run.
    let systemd = told_by_systemd(&cgroup);
    let origin = match in_account(&cgroup).map(|mut below| (below.next(), below.next())) {
        _ if systemd => Origin::Systemd,
        _ if cgroup.is_empty() => Origin::Unknown,
        Some((Some("session.slice"), Some(SHELL))) => Origin::Shell,
        Some((Some("jobs.slice"), _)) => Origin::Jobs,
        _ => Origin::Elsewhere,
    };
    let told = text("MESSAGE_ID").filter(|id| systemd || (id == SHUTTING_DOWN && cgroup == LOGIND));
    let thing = match (identifier.as_deref(), unit) {
        (Some("systemd") | None, Some(unit)) if systemd => unit,
        (Some(identifier), _) => identifier.to_owned(),
        (None, _) => "unnamed".to_owned(),
    };
    let job = match told.as_deref() {
        _ if !thing.ends_with(".service") => None,
        Some(UNIT_STOPPING) => Some(Job::Stopping),
        Some(UNIT_STARTED) => Some(Job::Started),
        _ => None,
    };
    // A real crash is told by systemd-coredump, a unit of the system's that
    // only root starts, so a crash said from anywhere else is a forgery and
    // only a line. A report can come with no cgroup at all, when it's gone
    // before journald looks (62 of 177 from 15 to 17 November), so that's
    // kept, though a command that ends at once can forge one the same way.
    // Should journald write the root cgroup, as the kernel's helper's for a
    // crash of the manager or journald itself, that's kept too: no process
    // of an account's can enter it.
    let crashed = text("COREDUMP_COMM")
        .filter(|_| matches!(cgroup.as_str(), "" | "/") || cgroup.starts_with(COREDUMP_SLICE));
    let whose = crashed.as_ref().map(|_| whose(&fields));
    let micros = |name: &str| text(name).and_then(|value| value.parse().ok());
    let mark = match told.as_deref() {
        Some(STARTUP_FINISHED) => micros("KERNEL_USEC").map(|kernel| Mark::Started {
            firmware: stage(&message, "(firmware)").unwrap_or_default(),
            loader: stage(&message, "(loader)").unwrap_or_default(),
            kernel,
            initrd: micros("INITRD_USEC").unwrap_or_default(),
            userspace: micros("USERSPACE_USEC").unwrap_or_default(),
        }),
        Some(SHUTTING_DOWN) => Some(Mark::Stopping),
        Some(UNIT_STARTED) => match text("UNIT") {
            Some(unit) if unit == PASSWORD_ASKED => Some(Mark::Asked),
            Some(unit) if unit.starts_with(DISK_OPENER) => Some(Mark::Opened),
            _ => None,
        },
        _ if origin == Origin::Shell && is_ready(&message) => Some(Mark::Ready),
        _ => None,
    };
    Some(Entry {
        at,
        priority,
        thing,
        message,
        ran_in: text("_SYSTEMD_USER_UNIT").or_else(|| text("_SYSTEMD_UNIT")),
        crashed,
        whose,
        agents,
        origin,
        restart: told.as_deref() == Some(RESTART_SCHEDULED),
        kernel: text("_TRANSPORT").as_deref() == Some("kernel"),
        boot: text("_BOOT_ID").unwrap_or_default(),
        since_boot: micros("__MONOTONIC_TIMESTAMP").unwrap_or_default(),
        mark,
        ask,
        job,
    })
}

/// How long a stage took, in microseconds, from systemd's "9.709s
/// (firmware) + 549ms (loader)": the duration said just before its name.
fn stage(message: &str, name: &str) -> Option<i64> {
    let before = &message[..message.find(name)?];
    let said = before.split_whitespace().last()?;
    let (number, per_unit) = if let Some(number) = said.strip_suffix("ms") {
        (number, 1_000)
    } else if let Some(number) = said.strip_suffix("us") {
        (number, 1)
    } else {
        (said.strip_suffix('s')?, 1_000_000)
    };
    let (whole, fraction) = number.split_once('.').unwrap_or((number, ""));
    let mut micros = whole.parse::<i64>().ok()? * per_unit;
    let mut place = per_unit / 10;
    for digit in fraction.chars() {
        micros += i64::from(digit.to_digit(10)?) * place;
        place /= 10;
    }
    Some(micros)
}

/// A field's words: journald writes a message that isn't UTF-8 as its
/// bytes, which are read as far as they go.
fn said(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(bytes) => {
            let bytes: Vec<u8> = bytes
                .iter()
                .filter_map(Value::as_u64)
                .filter_map(|byte| u8::try_from(byte).ok())
                .collect();
            Some(String::from_utf8_lossy(&bytes).into_owned())
        }
        _ => None,
    }
}

/// Whether a line came from a manager of systemd's: the machine's, in
/// `/init.scope`, or an account's, in its `user@N.service/init.scope`.
fn told_by_systemd(cgroup: &str) -> bool {
    cgroup == "/init.scope"
        || in_account(cgroup)
            .is_some_and(|mut below| below.next() == Some("init.scope") && below.next().is_none())
}

/// The cgroups below an account's manager that `cgroup` sits in, outermost
/// first, or none when it isn't below one. A delegated unit can make
/// cgroups of any name below itself, so a slice counts only where its
/// manager puts it.
fn in_account(cgroup: &str) -> Option<std::str::Split<'_, char>> {
    let (uid, rest) = cgroup
        .strip_prefix("/user.slice/user-")?
        .split_once(".slice/user@")?;
    let below = rest.strip_prefix(uid)?.strip_prefix(".service/")?;
    Some(below.split('/'))
}

/// Whose a crashed program was, from the slice it ran in: his programs,
/// his session and what he waits on at a terminal are his; everything else
/// (agents' work, the timed work, the background) is agents'.
fn whose(fields: &Map<String, Value>) -> Whose {
    let cgroup = fields
        .get("COREDUMP_CGROUP")
        .and_then(said)
        .unwrap_or_default();
    let mut below = in_account(&cgroup);
    let mut next = || below.as_mut().and_then(Iterator::next);
    match (next(), next()) {
        (Some("app.slice" | "session.slice"), _)
        | (Some("jobs.slice"), Some("jobs-awaited.slice")) => Whose::His,
        _ => Whose::Agents,
    }
}

/// Whether the cgroup a line came from is an agent's or its commands':
/// a lab run of a daemon writes under the installed one's identifier, and
/// what it says was never put to Joel.
fn agents_ran(cgroup: &str) -> bool {
    cgroup.contains("/agents.slice") || cgroup.contains("/jobs.slice")
}

/// Whether a line came from agents' work: their shells' holder
/// (`jobs.service`) or a command in `jobs-work.slice`, the two places the
/// jobs broker calls agents'. Everything else in the jobs slice is his or
/// the machine's: what he waits on at a terminal, the timed work, and the
/// compile cache every build reads. An agent itself, and the terminal Joel
/// reads its sessions in, run in agents.slice, and what they say is the
/// machine's to answer for, so it counts.
fn agents_work(cgroup: &str) -> bool {
    in_account(cgroup).is_some_and(|mut below| {
        below.next() == Some("jobs.slice")
            && below
                .next()
                .is_some_and(|kind| matches!(kind, "jobs.service" | "jobs-work.slice"))
    })
}

/// What each journal follower asks journalctl for.
#[must_use]
pub fn followed() -> Vec<Vec<String>> {
    let asked = |words: &[&str]| words.iter().map(|word| (*word).to_owned()).collect();
    vec![
        // Warnings and graver.
        asked(&["-p", "warning"]),
        // What systemd and logind say only as information: a restart
        // scheduled, and the machine started or stopping.
        vec![
            format!("MESSAGE_ID={RESTART_SCHEDULED}"),
            format!("MESSAGE_ID={STARTUP_FINISHED}"),
            format!("MESSAGE_ID={SHUTTING_DOWN}"),
        ],
        // Who asked systemd to reload, said as a notice with no unit,
        // about two hundred a day: a restart after one a switch didn't ask
        // for is no switch's (`reloads.rs`).
        asked(&["CODE_FUNC=manager_log_caller"]),
        // A service stopping and started, by the machine's systemd or
        // Joel's: the two a restart is timed between, and the start of the
        // disk's password and its opening. Every unit's are followed, since
        // systemd names a service by its description where it has one;
        // the scopes an agent's commands start in, about two thousand an
        // hour, come as information and leave the day as they come.
        vec![
            format!("MESSAGE_ID={UNIT_STOPPING}"),
            format!("MESSAGE_ID={UNIT_STARTED}"),
        ],
        // The launcher ready, after each start of the shell, and how long
        // it took to show each thing Joel did, which it says as information.
        vec![
            format!("SYSLOG_IDENTIFIER={}", launcher::IDENTIFIER),
            format!("--grep={LAUNCHER_READY}|{}", launcher::FOLLOWED),
        ],
        // How long each program took to open and to end, which the
        // windows daemon says as information.
        vec![
            format!("SYSLOG_IDENTIFIER={}", launches::IDENTIFIER),
            format!("--grep={}", launches::FOLLOWED),
        ],
        // Any program saying it's ready and how long it took, as
        // information; warnings come through the first.
        vec![
            "-p".to_owned(),
            "notice..info".to_owned(),
            format!("--grep={}", launches::READY_FOLLOWED),
        ],
        // Each ask for a secret Joel answered, said only as information.
        vec![format!("SYSLOG_IDENTIFIER={}", asks::IDENTIFIER)],
        // The kernel's notices and information, which is where it says a
        // USB device dropped or a PCIe link corrected itself; the day keeps
        // only those and leaves the rest, about a thousand lines an hour.
        asked(&["-k", "-p", "notice..info"]),
    ]
}

/// One journalctl reading the day, as `asked`, from `since`, the day's
/// start, until `until`, in seconds since 1970, and ending: the histories
/// are merged in the order their lines were written (`replay.rs`) before
/// each follows on. Every history is given the same two, rather than
/// each working out "today" and "now" as it starts, so all read one day
/// to one moment however long the others take.
#[must_use]
pub fn reading(asked: &[String], since: i64, until: i64) -> Command {
    journalctl(
        &[&format!("--since=@{since}"), &format!("--until=@{until}")],
        asked,
    )
}

/// One journalctl following, as `asked`, from just after `after`, the
/// cursor its history ended at, or from `since`, the day's start its
/// history read from, when its history had no lines.
#[must_use]
pub fn following(asked: &[String], after: Option<&str>, since: i64) -> Command {
    let from = after.map_or_else(
        || format!("--since=@{since}"),
        |cursor| format!("--after-cursor={cursor}"),
    );
    journalctl(&["--follow", &from], asked)
}

/// One journalctl, from where `from` says, as `asked`, through every boot
/// of the day: journalctl follows only this boot unless told otherwise, so
/// after a reboot the day would be counted from the reboot alone. `-k`
/// chooses this boot too: systemd 262's journalctl reads every boot with
/// `--boot=all` either side of it (checked on 17 November), and asking
/// last keeps it so where the last choice wins.
fn journalctl(from: &[&str], asked: &[String]) -> Command {
    let mut journalctl = Command::new("journalctl");
    journalctl
        .args(["--output=json", "--no-pager"])
        .args(from)
        .arg(format!("--output-fields={FIELDS}"))
        .args(asked)
        .arg("--boot=all");
    journalctl
}

#[cfg(test)]
mod tests;
