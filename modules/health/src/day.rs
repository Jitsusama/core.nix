//! A day of what the machine said about itself, tallied by kind, thing and
//! signature, in the form written to `<date>.json` for `health` to show
//! and for `cadence day` to judge.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::journal::{Entry, Whose};
use crate::sign;

/// The most signatures kept for one thing; past it, the rest are counted
/// together, so a thing saying something new each line can't fill memory.
const MOST_SIGNATURES: usize = 100;

/// What the rest are counted under once a thing has its most.
pub const THE_REST: &str = "(the rest)";

/// The programs whose lines are about who may do what on the machine.
const SECURITY: [&str; 12] = [
    "sudo",
    "su",
    "run0",
    "pkexec",
    "polkitd",
    "polkit-agent-helper-1",
    "sshd",
    "sshd-session",
    "audit",
    "unix_chkpwd",
    "login",
    "greetd",
];

/// What the kernel says only as information that's still a device going
/// wrong, or going away: a USB device dropped or reset, a PCI Express link's
/// corrected errors, a processor throttled, firmware that wouldn't load.
/// Each is said in full in the kernel's own words, and matched as written.
const DEVICE_EVENTS: [&[&str]; 6] = [
    &["USB disconnect"],
    &["reset ", " USB device number "],
    &["AER: ", " error"],
    &["throttled"],
    &["Direct firmware load"],
    &["failed to load firmware"],
];

/// Which kind of thing a line is about, as `health` groups them.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// The machine's parts, as the kernel speaks of them.
    Devices,
    /// Programs run in their own scope, and every program that crashed.
    Programs,
    /// Daemons, systemd's services and what they say.
    Services,
    /// Who may do what: sudo, polkit, logins, the audit log.
    Security,
}

impl Kind {
    /// Its name, as `health` shows it and is typed.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Devices => "devices",
            Self::Programs => "programs",
            Self::Services => "services",
            Self::Security => "security",
        }
    }
}

/// One day's tally.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Day {
    /// The day, as `YYYY-MM-DD` in the machine's time zone.
    pub date: String,
    /// The last entry taken, in microseconds since the epoch.
    pub to: i64,
    /// Each thing that said anything, by its name.
    pub things: BTreeMap<String, Thing>,
}

/// What one thing said in the day.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Thing {
    /// Which kind it is.
    pub kind: Option<Kind>,
    /// Its lines at warning or graver.
    pub lines: u64,
    /// Those at error or graver.
    pub errors: u64,
    /// Its stack traces.
    pub traces: u64,
    /// Its restarts, as systemd scheduled them.
    pub restarts: u64,
    /// A device's events the kernel said only as information.
    #[serde(default)]
    pub events: u64,
    /// The times it crashed, and of those, the ones in Joel's own slices.
    pub crashes: u64,
    /// The crashes of it that were his.
    pub his_crashes: u64,
    /// What it said, by signature.
    pub signatures: BTreeMap<String, Signature>,
}

/// One thing it kept saying.
#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    /// How many times.
    pub count: u64,
    /// The gravest priority it was said at.
    pub priority: u8,
    /// When it was first said, in microseconds since the epoch.
    pub first: i64,
    /// When it was last said.
    pub last: i64,
    /// Its first line, the first time, as it was said.
    pub example: String,
}

impl Day {
    /// A day with nothing in it yet.
    #[must_use]
    pub fn new(date: &str) -> Self {
        Self {
            date: date.to_owned(),
            ..Self::default()
        }
    }

    /// Take one entry into the day.
    pub fn take(&mut self, entry: &Entry) {
        self.to = self.to.max(entry.at);
        let event = entry.priority > 4 && !entry.restart;
        if event && !(entry.kernel && is_device_event(&entry.message)) {
            return;
        }
        let (name, kind) = named(entry);
        let thing = self.things.entry(name).or_default();
        thing.kind = thing.kind.or(Some(kind));
        if entry.restart {
            thing.restarts += 1;
            return;
        }
        if event {
            thing.events += 1;
            thing.say(entry);
            return;
        }
        thing.lines += 1;
        if entry.priority <= 3 {
            thing.errors += 1;
        }
        if entry.crashed.is_some() {
            thing.crashes += 1;
            if entry.whose == Some(Whose::His) {
                thing.his_crashes += 1;
            }
        } else if sign::is_trace(&entry.message) {
            thing.traces += 1;
        }
        thing.say(entry);
    }
}

impl Thing {
    /// Count what the entry said under its signature.
    fn say(&mut self, entry: &Entry) {
        let mut signed = sign::signature(&entry.message);
        if !self.signatures.contains_key(&signed) && self.signatures.len() >= MOST_SIGNATURES {
            THE_REST.clone_into(&mut signed);
        }
        let first = entry.message.lines().next().unwrap_or_default();
        let signature = self.signatures.entry(signed).or_insert_with(|| Signature {
            priority: entry.priority,
            first: entry.at,
            example: first.chars().take(400).collect(),
            ..Signature::default()
        });
        signature.count += 1;
        signature.priority = signature.priority.min(entry.priority);
        signature.last = signature.last.max(entry.at);
    }

    /// How bad it is, for putting the worst first.
    #[must_use]
    pub const fn badness(&self) -> Badness {
        Badness {
            his_crashes: self.his_crashes,
            crashes: self.crashes,
            traces: self.traces,
            errors: self.errors,
            restarts: self.restarts,
            lines: self.lines,
            events: self.events,
        }
    }
}

/// How bad a thing, or things together, were. Its fields are in order of
/// gravity, so one is worse than another by its first field that differs:
/// Joel's crashes, then any crash, traces, errors, restarts, warnings, and
/// last the devices' events, which are often only a cable pulled.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Badness {
    /// The crashes that were Joel's.
    pub his_crashes: u64,
    /// Every crash.
    pub crashes: u64,
    /// Stack traces.
    pub traces: u64,
    /// Lines at error or graver, crashes among them.
    pub errors: u64,
    /// Restarts systemd scheduled.
    pub restarts: u64,
    /// Lines at warning or graver, errors among them.
    pub lines: u64,
    /// Devices' events.
    pub events: u64,
}

impl std::ops::Add for Badness {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            his_crashes: self.his_crashes + other.his_crashes,
            crashes: self.crashes + other.crashes,
            traces: self.traces + other.traces,
            errors: self.errors + other.errors,
            restarts: self.restarts + other.restarts,
            lines: self.lines + other.lines,
            events: self.events + other.events,
        }
    }
}

/// Whether a kernel line said as information is a device's event.
fn is_device_event(message: &str) -> bool {
    DEVICE_EVENTS
        .iter()
        .any(|words| words.iter().all(|word| message.contains(word)))
}

/// The thing an entry is about, and its kind: a crashed program by its
/// name, a kernel line by the driver or device it opens with, and anything
/// else by what said it.
fn named(entry: &Entry) -> (String, Kind) {
    if let Some(program) = &entry.crashed {
        return (program.clone(), Kind::Programs);
    }
    if entry.kernel {
        let opening = entry.message.split_whitespace().next().unwrap_or("kernel");
        let device = sign::signature(opening.trim_end_matches(':'));
        let kind = if device == "audit" {
            Kind::Security
        } else {
            Kind::Devices
        };
        return (device, kind);
    }
    let kind = if SECURITY.contains(&entry.thing.as_str()) {
        Kind::Security
    } else if entry.ran_in.as_deref().is_some_and(|unit| {
        Path::new(unit)
            .extension()
            .is_some_and(|kind| kind == "scope")
    }) {
        Kind::Programs
    } else {
        Kind::Services
    };
    (entry.thing.clone(), kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(thing: &str, priority: u8, message: &str) -> Entry {
        Entry {
            at: 1,
            priority,
            thing: thing.to_owned(),
            message: message.to_owned(),
            ran_in: Some(format!("{thing}.service")),
            crashed: None,
            whose: None,
            restart: false,
            kernel: false,
        }
    }

    #[test]
    fn repeats_are_counted_under_one_signature() {
        let mut day = Day::new("2026-11-12");
        day.take(&entry("pebble-lights", 4, "light 3 answered in 812 ms"));
        day.take(&Entry {
            at: 9,
            ..entry("pebble-lights", 3, "light 4 answered in 90 ms")
        });
        let thing = &day.things["pebble-lights"];
        assert_eq!(
            thing.kind,
            Some(Kind::Services),
            "a service's lines are a service's"
        );
        assert_eq!(
            (thing.lines, thing.errors),
            (2, 1),
            "both lines, one an error"
        );
        let said = &thing.signatures["light # answered in # ms"];
        assert_eq!(said.count, 2, "one signature for both");
        assert_eq!(
            (said.priority, said.first, said.last),
            (3, 1, 9),
            "gravest, first, last"
        );
        assert_eq!(
            said.example, "light 3 answered in 812 ms",
            "the first said, as it was"
        );
        assert_eq!(day.to, 9, "the day runs to its last entry");
    }

    #[test]
    fn a_crash_is_its_programs_and_says_whose() {
        let mut day = Day::new("2026-11-12");
        let crash = |whose| Entry {
            crashed: Some("chrome".to_owned()),
            whose: Some(whose),
            ..entry(
                "systemd-coredump",
                2,
                "Process 1 (chrome) of user 1000 dumped core.",
            )
        };
        day.take(&crash(Whose::Agents));
        day.take(&crash(Whose::His));
        let chrome = &day.things["chrome"];
        assert_eq!(chrome.kind, Some(Kind::Programs), "a crash is a program's");
        assert_eq!(
            (chrome.crashes, chrome.his_crashes),
            (2, 1),
            "two crashes, one his"
        );
        assert_eq!(chrome.traces, 0, "a crash isn't counted again as a trace");
    }

    #[test]
    fn a_trace_and_a_restart_are_counted() {
        let mut day = Day::new("2026-11-12");
        day.take(&entry(
            "quickshell",
            4,
            "qrc:/Now.qml:3: TypeError: x is null",
        ));
        day.take(&Entry {
            restart: true,
            ..entry(
                "quickshell.service",
                6,
                "quickshell.service: Scheduled restart job",
            )
        });
        assert_eq!(day.things["quickshell"].traces, 1, "the trace is counted");
        let service = &day.things["quickshell.service"];
        assert_eq!(
            (service.restarts, service.lines),
            (1, 0),
            "a restart isn't a warning"
        );
    }

    #[test]
    fn the_kernel_speaks_of_devices_and_the_audit_log() {
        let mut day = Day::new("2026-11-12");
        let kernel = |message| Entry {
            kernel: true,
            ..entry("kernel", 3, message)
        };
        day.take(&kernel("iwlwifi 0000:00:14.3: Microcode SW error detected"));
        day.take(&kernel("nvme0n1: I/O error"));
        day.take(&kernel("audit: type=1400 apparmor=\"DENIED\""));
        assert_eq!(
            day.things["iwlwifi"].kind,
            Some(Kind::Devices),
            "a driver is a device"
        );
        assert_eq!(
            day.things["nvme#n#"].kind,
            Some(Kind::Devices),
            "a disk is a device"
        );
        assert_eq!(
            day.things["audit"].kind,
            Some(Kind::Security),
            "the audit log is security"
        );
    }

    #[test]
    fn a_device_event_is_counted_and_other_information_left_out() {
        let mut day = Day::new("2026-11-12");
        let kernel = |message| Entry {
            kernel: true,
            ..entry("kernel", 6, message)
        };
        day.take(&kernel("usb 3-2.1.3: USB disconnect, device number 22"));
        day.take(&kernel(
            "usb 2-1: reset SuperSpeed USB device number 2 using xhci_hcd",
        ));
        day.take(&kernel(
            "pcieport 0000:00:1c.0: AER: Corrected error message received from 0000:01:00.0",
        ));
        day.take(&kernel("pcieport 0000:00:1c.0: AER: enabled with IRQ 122"));
        day.take(&kernel("Bluetooth: hci0: Firmware loaded in 1534 usecs"));
        day.take(&entry("pebble-lights", 6, "USB disconnect seen"));
        let usb = &day.things["usb"];
        assert_eq!(usb.kind, Some(Kind::Devices), "a USB event is a device's");
        assert_eq!(
            (usb.events, usb.lines),
            (2, 0),
            "a disconnect and a reset are events, not warnings"
        );
        assert_eq!(
            day.things["pcieport"].events, 1,
            "a corrected error is an event; turning AER on isn't"
        );
        assert!(
            !day.things.contains_key("Bluetooth") && !day.things.contains_key("pebble-lights"),
            "other information is left out: {:?}",
            day.things.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn security_and_programs_are_told_apart_from_services() {
        let mut day = Day::new("2026-11-12");
        day.take(&entry(
            "sudo",
            4,
            "pam_unix(sudo:auth): conversation failed",
        ));
        day.take(&Entry {
            ran_in: Some("app-niri-kitty-1.scope".to_owned()),
            ..entry("kitty", 4, "[0.1] Failed to set locale")
        });
        assert_eq!(
            day.things["sudo"].kind,
            Some(Kind::Security),
            "sudo is security"
        );
        assert_eq!(
            day.things["kitty"].kind,
            Some(Kind::Programs),
            "a scope's is a program's"
        );
    }

    #[test]
    fn a_thing_saying_something_new_each_line_is_kept_to_its_most() {
        let mut day = Day::new("2026-11-12");
        for n in 0..=MOST_SIGNATURES {
            let word: String = std::iter::repeat_n('a', n + 1).collect();
            day.take(&entry("chatty", 4, &word));
        }
        let chatty = &day.things["chatty"];
        assert_eq!(
            chatty.signatures.len(),
            MOST_SIGNATURES + 1,
            "its most, and the rest"
        );
        assert_eq!(
            chatty.signatures[THE_REST].count, 1,
            "the one past its most"
        );
    }
}
