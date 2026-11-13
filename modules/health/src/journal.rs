//! One entry of the journal as `journalctl -o json` writes it, cut down to
//! what the day is tallied by: when, how grave, which thing said it, and
//! what it said; and the journalctl followers that read them.

use std::process::Command;

use serde_json::{Map, Value};

use crate::asks::{self, Ask};
use crate::{launcher, launches};

/// The fields each entry is read by; journalctl always adds its time.
const FIELDS: &str = "MESSAGE,PRIORITY,SYSLOG_IDENTIFIER,_COMM,USER_UNIT,UNIT,_SYSTEMD_UNIT,\
                      _SYSTEMD_USER_UNIT,COREDUMP_COMM,COREDUMP_CGROUP,MESSAGE_ID,_TRANSPORT,\
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

/// Whether `message` is the launcher come ready.
fn is_ready(message: &str) -> bool {
    message
        .split_once("launcher: home ")
        .is_some_and(|(_, rest)| rest.contains("kept as the shell started"))
}

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

/// Whose a program is, from where systemd ran it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Whose {
    /// Joel's: in his programs' or his session's slice.
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
    let priority = text("PRIORITY").and_then(|p| p.parse().ok()).unwrap_or(6);
    let message = text("MESSAGE").unwrap_or_default();
    let identifier = text("SYSLOG_IDENTIFIER").or_else(|| text("_COMM"));
    let unit = text("USER_UNIT").or_else(|| text("UNIT"));
    let ask = (identifier.as_deref() == Some(asks::IDENTIFIER))
        .then(|| asks::of(text))
        .flatten();
    let thing = match (identifier.as_deref(), unit) {
        (Some("systemd") | None, Some(unit)) => unit,
        (Some(identifier), _) => identifier.to_owned(),
        (None, None) => "unnamed".to_owned(),
    };
    let job = match text("MESSAGE_ID").as_deref() {
        _ if !thing.ends_with(".service") => None,
        Some(UNIT_STOPPING) => Some(Job::Stopping),
        Some(UNIT_STARTED) => Some(Job::Started),
        _ => None,
    };
    let crashed = text("COREDUMP_COMM");
    let whose = crashed.as_ref().map(|_| whose(&fields));
    let micros = |name: &str| text(name).and_then(|value| value.parse().ok());
    let mark = match text("MESSAGE_ID").as_deref() {
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
        _ if is_ready(&message) => Some(Mark::Ready),
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
        restart: text("MESSAGE_ID").as_deref() == Some(RESTART_SCHEDULED),
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

/// Whose a crashed program was, from the slice it ran in: his programs and
/// his session are his, everything else (jobs, the background) is agents'.
fn whose(fields: &Map<String, Value>) -> Whose {
    let cgroup = fields
        .get("COREDUMP_CGROUP")
        .and_then(said)
        .unwrap_or_default();
    if cgroup.contains("/app.slice") || cgroup.contains("/session.slice") {
        Whose::His
    } else {
        Whose::Agents
    }
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

/// One journalctl following from the day's start, as `asked`.
#[must_use]
pub fn following(asked: &[String]) -> Command {
    let mut journalctl = Command::new("journalctl");
    journalctl
        .args(["--follow", "--output=json", "--no-pager", "--since=today"])
        .arg(format!("--output-fields={FIELDS}"))
        .args(asked);
    journalctl
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_warning_is_its_programs() {
        let line = r#"{"__REALTIME_TIMESTAMP":"1794450621321755","PRIORITY":"4",
            "SYSLOG_IDENTIFIER":"pebble-lights","_SYSTEMD_USER_UNIT":"pebble-lights.service",
            "MESSAGE":"light 3 answered in 812 ms"}"#
            .replace('\n', "");
        let entry = parse(&line).expect("a journal line is an entry");
        assert_eq!(
            entry.thing, "pebble-lights",
            "the identifier names the thing"
        );
        assert_eq!(entry.priority, 4, "the priority is read");
        assert_eq!(entry.at, 1_794_450_621_321_755, "the time is read");
        assert_eq!(
            entry.ran_in.as_deref(),
            Some("pebble-lights.service"),
            "its unit"
        );
        assert!(
            !entry.restart && !entry.kernel,
            "it's neither a restart nor the kernel's"
        );
    }

    #[test]
    fn systemd_speaking_of_a_unit_is_the_units() {
        let line = r#"{"__REALTIME_TIMESTAMP":"1","PRIORITY":"6","SYSLOG_IDENTIFIER":"systemd",
            "USER_UNIT":"merge-net.service","MESSAGE_ID":"5eb03494b6584870a536b337290809b3",
            "MESSAGE":"merge-net.service: Scheduled restart job, restart counter is at 1."}"#
            .replace('\n', "");
        let entry = parse(&line).expect("a journal line is an entry");
        assert_eq!(
            entry.thing, "merge-net.service",
            "the unit spoken of is the thing"
        );
        assert!(entry.restart, "systemd's restart message is a restart");
    }

    #[test]
    fn a_crash_says_whose_program_it_was() {
        let his = r#"{"__REALTIME_TIMESTAMP":"1","PRIORITY":"2",
            "SYSLOG_IDENTIFIER":"systemd-coredump","COREDUMP_COMM":"1password",
            "COREDUMP_CGROUP":"/user.slice/user-1000.slice/app.slice/app-1password.scope",
            "MESSAGE":"Process 4169227 (1password) of user 1000 dumped core."}"#
            .replace('\n', "");
        let agents = his
            .replace("1password", "chrome")
            .replace("/app.slice/", "/jobs.slice/jobs-timed.slice/");
        let his = parse(&his).expect("a crash is an entry");
        let agents = parse(&agents).expect("a crash is an entry");
        assert_eq!(
            his.crashed.as_deref(),
            Some("1password"),
            "the program is named"
        );
        assert_eq!(his.whose, Some(Whose::His), "a program in his slice is his");
        assert_eq!(
            agents.whose,
            Some(Whose::Agents),
            "a job's program is agents'"
        );
    }

    #[test]
    fn a_message_in_bytes_is_read_as_far_as_it_goes() {
        let line = r#"{"__REALTIME_TIMESTAMP":"1","PRIORITY":"3","_TRANSPORT":"kernel",
            "SYSLOG_IDENTIFIER":"kernel","MESSAGE":[110,118,109,101,255]}"#
            .replace('\n', "");
        let entry = parse(&line).expect("a kernel line is an entry");
        assert!(
            entry.message.starts_with("nvme"),
            "the bytes are read: {}",
            entry.message
        );
        assert!(entry.kernel, "the kernel's transport is the kernel's");
    }

    #[test]
    fn a_service_stopping_or_started_is_followed_whatever_its_description_says() {
        // systemd says a service with a description by it, as in "Stopping
        // Quickshell, which draws the launcher...", so a follower that
        // greps the message for a unit's name hears no restart of one.
        let stopping = format!("MESSAGE_ID={UNIT_STOPPING}");
        let follower = followed()
            .into_iter()
            .find(|asked| asked.contains(&stopping))
            .expect("a service stopping is followed");
        assert_eq!(
            follower,
            [stopping, format!("MESSAGE_ID={UNIT_STARTED}")],
            "a service's stop and start are followed only where the message names its unit"
        );
    }

    #[test]
    fn a_service_stopping_or_started_is_its_job_and_a_scope_is_none() {
        let line = |id: &str, unit: &str| {
            format!(
                r#"{{"__REALTIME_TIMESTAMP":"1","PRIORITY":"6","SYSLOG_IDENTIFIER":"systemd",
                "MESSAGE_ID":"{id}","USER_UNIT":"{unit}","MESSAGE":"Started {unit}"}}"#
            )
            .replace('\n', "")
        };
        let job = |id, unit| parse(&line(id, unit)).expect("systemd's line is an entry");
        let stopping = job(UNIT_STOPPING, "google.service");
        assert_eq!(
            (stopping.thing.as_str(), stopping.job),
            ("google.service", Some(Job::Stopping)),
            "a service stopping wasn't its job"
        );
        assert_eq!(
            job(UNIT_STARTED, "google.service").job,
            Some(Job::Started),
            "a service started wasn't its job"
        );
        assert_eq!(
            job(UNIT_STARTED, "work-1-2.scope").job,
            None,
            "a scope started was taken for a service's"
        );
    }

    #[test]
    fn a_boot_says_how_long_each_stage_took() {
        let line = r#"{"__REALTIME_TIMESTAMP":"1","PRIORITY":"6","SYSLOG_IDENTIFIER":"systemd",
            "_BOOT_ID":"382e94","__MONOTONIC_TIMESTAMP":"55725000",
            "MESSAGE_ID":"b07a249cd024414a82dd00cd181378ff","KERNEL_USEC":"432199",
            "INITRD_USEC":"24362468","USERSPACE_USEC":"20671351",
            "MESSAGE":"Startup finished in 9.709s (firmware) + 549ms (loader) + 432ms (kernel) + 24.362s (initrd) + 20.671s (userspace) = 55.725s."}"#
            .replace('\n', "");
        let entry = parse(&line).expect("a boot's line is an entry");
        assert_eq!(
            entry.mark,
            Some(Mark::Started {
                firmware: 9_709_000,
                loader: 549_000,
                kernel: 432_199,
                initrd: 24_362_468,
                userspace: 20_671_351,
            }),
            "each stage is read"
        );
        assert_eq!(
            (entry.boot.as_str(), entry.since_boot),
            ("382e94", 55_725_000),
            "its boot and when in it"
        );
        let user = line.replace(r#""KERNEL_USEC":"432199","#, "");
        assert_eq!(
            parse(&user).expect("an entry").mark,
            None,
            "an account's manager starting isn't the machine's boot"
        );
    }

    #[test]
    fn the_password_the_launcher_and_a_shutdown_are_marked() {
        let unit = |name: &str| {
            format!(
                r#"{{"__REALTIME_TIMESTAMP":"1","MESSAGE_ID":"39f53479d3a045ac8e11786248231fbf","UNIT":"{name}","MESSAGE":"Finished."}}"#
            )
        };
        let mark = |line: &str| parse(line).expect("an entry").mark;
        assert_eq!(
            mark(&unit("systemd-ask-password-plymouth.service")),
            Some(Mark::Asked),
            "Plymouth asking is the password asked"
        );
        assert_eq!(
            mark(&unit("systemd-cryptsetup@system.service")),
            Some(Mark::Opened),
            "any disk opening"
        );
        assert_eq!(
            mark(&unit("nix-daemon.service")),
            None,
            "other units aren't marks"
        );
        let said = |message: &str, id: &str| {
            format!(r#"{{"__REALTIME_TIMESTAMP":"1","MESSAGE_ID":"{id}","MESSAGE":"{message}"}}"#)
        };
        assert_eq!(
            mark(&said(
                "launcher: home and 7 places kept as the shell started, 7054 rows made in 213 ms",
                ""
            )),
            Some(Mark::Ready),
            "the launcher's rows made is it ready"
        );
        assert_eq!(
            mark(&said(
                "launcher: home kept as the shell started, 7989 rows made in 35 ms",
                ""
            )),
            Some(Mark::Ready),
            "the launcher's rows made with no places kept is it ready"
        );
        assert_eq!(
            mark(&said(
                "System is rebooting.",
                "98268866d1d54a499c4e98921d93bc40"
            )),
            Some(Mark::Stopping),
            "logind's rebooting is the shutdown asked"
        );
    }

    #[test]
    fn a_line_that_isnt_an_entry_is_none() {
        assert_eq!(parse("-- No entries --"), None, "words aren't an entry");
        assert_eq!(
            parse(r#"{"MESSAGE":"no time"}"#),
            None,
            "an entry needs its time"
        );
    }
}
