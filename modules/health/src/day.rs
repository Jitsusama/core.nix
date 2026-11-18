//! A day of what the machine said about itself, tallied by kind, thing and
//! signature, in the form written to `<date>.json` for `health` to show
//! and for `cadence day` to judge.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::agents::Agents;
use crate::asks::Asks;
use crate::busy::Busy;
use crate::defunct::Defunct;
use crate::disk::Disks;
use crate::feel::Pain;
use crate::journal::{Entry, Mark, Whose};
use crate::kernel;
use crate::launcher::{self, Launcher};
use crate::launches::Launches;
use crate::memory::Memory;
use crate::sessions::{Heard, Said};
use crate::sign;
use crate::typing::Sample;

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
    /// The agents' tools, as their sessions' logs say: calls that failed,
    /// were refused, or were made twice running.
    Tools,
    /// What Joel typed to a session that sounds like something hurt, by
    /// where the session ran.
    Words,
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
            Self::Tools => "tools",
            Self::Words => "words",
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
    /// Each boot that started or stopped in the day, by its id.
    #[serde(default)]
    pub boots: BTreeMap<String, Boot>,
    /// How using the machine felt: the day's most painful interactions,
    /// worst first, as the pain-points detector ranked them.
    #[serde(default)]
    pub feel: Vec<Pain>,
    /// How far each session's log has been read, in bytes, so a restart
    /// reads on rather than reading a day's logs again.
    #[serde(default)]
    pub read: BTreeMap<String, u64>,
    /// What each watched program held, by its name.
    #[serde(default)]
    pub memory: BTreeMap<String, Memory>,
    /// The agents that died.
    #[serde(default)]
    pub agents: Agents,
    /// The asks for a secret Joel answered, read again from the journal
    /// on each start, so never resumed.
    #[serde(default)]
    pub asks: Asks,
    /// Each filesystem's least free space and gravest band, as the disk
    /// guards said them.
    #[serde(default)]
    pub disks: Disks,
    /// How long the launcher took to show what Joel did, read again from
    /// the journal on each start, so never resumed.
    #[serde(default)]
    pub launcher: Launcher,
    /// How long programs took to open and end, read again likewise.
    #[serde(default)]
    pub launches: Launches,
    /// pi-keys' samples of typing into pi, read again each tick.
    #[serde(default)]
    pub typing: Vec<Sample>,
    /// Whose work kept the processors busy, by the hour.
    #[serde(default)]
    pub busy: Busy,
    /// The most defunct children one parent left at a look, and who.
    #[serde(default)]
    pub defunct: Defunct,
    /// When the kernel said its last line, in microseconds, so a report's
    /// later lines are known by following it closely. It isn't kept, since
    /// each start reads the day's kernel lines again.
    #[serde(skip)]
    pub kernel_said: i64,
}

/// How one boot started and stopped, as far as the day saw it, each in
/// microseconds: stages as systemd timed them, moments since the kernel
/// started, and the shutdown on the wall clock.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Boot {
    /// When systemd said it had started, on the wall clock.
    pub started: i64,
    /// The firmware, before the boot loader.
    pub firmware: i64,
    /// The boot loader.
    pub loader: i64,
    /// The kernel.
    pub kernel: i64,
    /// The initrd, the password's wait among it.
    pub initrd: i64,
    /// systemd starting the machine's services.
    pub userspace: i64,
    /// When a disk first asked for its password.
    pub asked: Option<i64>,
    /// When the last disk opened.
    pub opened: Option<i64>,
    /// When the launcher was first ready.
    pub ready: Option<i64>,
    /// When the shutdown was asked for, on the wall clock.
    pub stopping: Option<i64>,
    /// The boot's last entry, on the wall clock.
    pub ended: Option<i64>,
}

impl Boot {
    /// How long Joel's password took, from the disk asking to it opening.
    #[must_use]
    pub fn password(&self) -> i64 {
        match (self.asked, self.opened) {
            (Some(asked), Some(opened)) if opened > asked => opened - asked,
            _ => 0,
        }
    }

    /// From the power button to a ready launcher, the password's wait left
    /// out, once both the boot's stages and the launcher are known.
    #[must_use]
    pub fn to_ready(&self) -> Option<i64> {
        let ready = self.ready?;
        (self.started > 0).then(|| self.firmware + self.loader + ready - self.password())
    }

    /// From the shutdown asked for to the boot's last entry.
    #[must_use]
    pub fn shutdown(&self) -> Option<i64> {
        Some(self.ended? - self.stopping?).filter(|took| *took >= 0)
    }
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
    /// Those of its errors agents' commands said, which were never put to
    /// Joel; never a crash, nor security's.
    #[serde(default)]
    pub agents_errors: u64,
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
        if let Some(mark) = entry.mark {
            self.launches.served(entry);
            self.mark(entry, mark);
            return;
        }
        if let Some(ask) = &entry.ask {
            self.asks.took(ask);
            return;
        }
        if entry.priority > 4
            && ((entry.thing == launcher::IDENTIFIER && self.launcher.took(&entry.message))
                || self.launches.heard(entry))
        {
            return;
        }
        let event = entry.priority > 4 && !entry.restart;
        if event && !(entry.kernel && kernel::is_device_event(&entry.message)) {
            return;
        }
        if entry.kernel {
            let follows = entry.at - self.kernel_said <= kernel::REPORT_GAP;
            self.kernel_said = entry.at;
            if follows && kernel::continues(&entry.message) {
                return;
            }
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
        if entry.priority <= 3 && !sign::is_probe(&entry.message) {
            thing.errors += 1;
            // A crash is counted as a crash, and a refusal the machine's
            // security made counts whoever was refused.
            if entry.agents && entry.crashed.is_none() && thing.kind != Some(Kind::Security) {
                thing.agents_errors += 1;
            }
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

impl Day {
    /// Keep a boot's moment. Each is kept the first time but a disk
    /// opening, where the last counts, since a machine may open several.
    fn mark(&mut self, entry: &Entry, mark: Mark) {
        let boot = self.boots.entry(entry.boot.clone()).or_default();
        let at = entry.since_boot;
        match mark {
            Mark::Started {
                firmware,
                loader,
                kernel,
                initrd,
                userspace,
            } => {
                *boot = Boot {
                    started: entry.at,
                    firmware,
                    loader,
                    kernel,
                    initrd,
                    userspace,
                    ..boot.clone()
                };
            }
            Mark::Asked => boot.asked = boot.asked.or(Some(at)),
            Mark::Opened => boot.opened = boot.opened.max(Some(at)),
            Mark::Ready => boot.ready = boot.ready.or(Some(at)),
            Mark::Stopping => boot.stopping = boot.stopping.or(Some(entry.at)),
        }
    }

    /// Count what a session's log said, from the session that ran at
    /// `place`: a tool's failure or refusal is an error of `pi <tool>`, a
    /// call made twice running a warning of it, and Joel's words an error
    /// of the place he said them.
    pub fn heard(&mut self, place: &str, heard: &Heard) {
        self.to = self.to.max(heard.at);
        let (name, kind, message, priority) = match &heard.said {
            Said::Failed { tool, why } | Said::Refused { tool, why } => {
                (format!("pi {tool}"), Kind::Tools, why.as_str(), 3)
            }
            Said::Again { tool } => (format!("pi {tool}"), Kind::Tools, "the same call again", 4),
            Said::Words { text } => (place.to_owned(), Kind::Words, text.as_str(), 3),
        };
        let thing = self.things.entry(name).or_default();
        thing.kind = thing.kind.or(Some(kind));
        thing.lines += 1;
        if priority <= 3 {
            thing.errors += 1;
        }
        thing.said(message, priority, heard.at);
    }

    /// Take from a day kept before this run what the journals don't say
    /// again: the sessions' tallies, how far each log was read, the
    /// memory the watched programs held, the disks and the defunct.
    pub fn resume(&mut self, kept: Self) {
        if kept.date != self.date {
            return;
        }
        self.read = kept.read;
        self.memory = kept.memory;
        self.agents = kept.agents;
        self.disks = kept.disks;
        self.busy = kept.busy;
        self.defunct = kept.defunct;
        self.things.extend(
            kept.things
                .into_iter()
                .filter(|(_, thing)| matches!(thing.kind, Some(Kind::Tools | Kind::Words))),
        );
    }

    /// Count a fault health saw itself, which no journal said: a line at
    /// error, from `name`, of `kind`, saying `message` at `at`.
    pub fn fault(&mut self, name: &str, kind: Kind, message: &str, at: i64) {
        self.to = self.to.max(at);
        let thing = self.things.entry(name.to_owned()).or_default();
        thing.kind = thing.kind.or(Some(kind));
        thing.lines += 1;
        thing.errors += 1;
        thing.said(message, 3, at);
    }

    /// Say when a boot that stopped in the day ended, from the journal's
    /// list of boots; a boot the day didn't see stop is left out.
    pub fn ended(&mut self, boot: &str, at: i64) {
        if let Some(boot) = self
            .boots
            .get_mut(boot)
            .filter(|boot| boot.stopping.is_some())
        {
            boot.ended = Some(at);
        }
    }
}

impl Thing {
    /// Count what the entry said under its signature.
    fn say(&mut self, entry: &Entry) {
        self.said(&entry.message, entry.priority, entry.at);
    }

    /// Count `message`, said at `priority` at `at`, under its signature.
    fn said(&mut self, message: &str, priority: u8, at: i64) {
        let mut signed = sign::signature(message);
        if !self.signatures.contains_key(&signed) && self.signatures.len() >= MOST_SIGNATURES {
            THE_REST.clone_into(&mut signed);
        }
        let first = message.lines().next().unwrap_or_default();
        let signature = self.signatures.entry(signed).or_insert_with(|| Signature {
            priority,
            first: at,
            example: first.chars().take(400).collect(),
            ..Signature::default()
        });
        signature.count += 1;
        signature.priority = signature.priority.min(priority);
        signature.last = signature.last.max(at);
    }

    /// The errors its kind's figure counts: a crash is said at an error's
    /// priority but counted once, as a crash, and what agents' commands
    /// said was never put to Joel. On 16 November these were 93 of
    /// programs' 179: tec aborting 66 times in agents' jobs, and the
    /// merge-garden labs failing on purpose 27.
    #[must_use]
    pub const fn figured_errors(&self) -> u64 {
        self.errors
            .saturating_sub(self.crashes)
            .saturating_sub(self.agents_errors)
    }

    /// How bad it is, for putting the worst first.
    #[must_use]
    pub const fn badness(&self) -> Badness {
        Badness {
            his_crashes: self.his_crashes,
            crashes: self.crashes,
            traces: self.traces,
            errors: self.errors.saturating_sub(self.agents_errors),
            restarts: self.restarts,
            lines: self.lines.saturating_sub(self.agents_errors),
            events: self.events,
            agents_errors: self.agents_errors,
        }
    }
}

/// How bad a thing, or things together, were. Its fields are in order of
/// gravity, so one is worse than another by its first field that differs:
/// Joel's crashes, then any crash, traces, errors, restarts, warnings, and
/// the devices' events, which are often only a cable pulled, and last how
/// many of its errors agents' commands said.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Badness {
    /// The crashes that were Joel's.
    pub his_crashes: u64,
    /// Every crash.
    pub crashes: u64,
    /// Stack traces.
    pub traces: u64,
    /// Lines at error or graver, crashes among them and agents' not.
    pub errors: u64,
    /// Restarts systemd scheduled.
    pub restarts: u64,
    /// Lines at warning or graver, errors among them and agents' not.
    pub lines: u64,
    /// Devices' events.
    pub events: u64,
    /// The errors agents' commands said.
    pub agents_errors: u64,
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
            agents_errors: self.agents_errors + other.agents_errors,
        }
    }
}

/// The thing an entry is about, and its kind: a crashed program by its
/// name, a kernel line by the driver or device it's from (the audit's and
/// a Mac's refusals are security's), and anything else by what said it, a
/// program's when it ran in a scope or a Mac's app.
fn named(entry: &Entry) -> (String, Kind) {
    if let Some(program) = &entry.crashed {
        return (program.clone(), Kind::Programs);
    }
    if entry.kernel {
        return kernel::named(&entry.message);
    }
    let kind = if SECURITY.contains(&entry.thing.as_str()) {
        Kind::Security
    } else if entry.ran_in.as_deref().is_some_and(|unit| {
        Path::new(unit)
            .extension()
            .is_some_and(|kind| kind == "scope" || kind == "app")
    }) {
        Kind::Programs
    } else {
        Kind::Services
    };
    (entry.thing.clone(), kind)
}

#[cfg(test)]
pub(crate) mod tests;
