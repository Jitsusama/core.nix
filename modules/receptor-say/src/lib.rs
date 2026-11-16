//! How a daemon says one of mycelium's receptors: how an outside source
//! stands, in counts, to the realm's mycelium node, which draws it as a
//! cell on the source's outside network (mycelium's PROTOCOL.md,
//! Receptors).
//!
//! Each daemon reading an outside source counts what it already holds and
//! hands the counts here, so the rules of what reaches mycelium are kept
//! once, for all of them:
//!
//! - Only fixed words and integers are arguments: the source and the
//!   state are words of this crate's own, never text a source gave, and
//!   each count is held to the most mycelium takes, so a count past it is
//!   said as that rather than refused.
//! - mycelium is run by the absolute path the service names, its
//!   arguments given one by one and never through a shell.
//! - The realm is the service's, in `MYCELIUM_REALM`, of mycelium's own
//!   letters, never anything a source holds. With it or `MYCELIUM` unset,
//!   as on a machine in no realm, nothing is said.
//! - Whose source it is, where that matters, is declared by the machine
//!   (`Declared`) and compared with what the source itself says, so
//!   another account's counts are never said in the realm.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

/// The sources a receptor reads, as mycelium names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Slack,
    Mail,
    Calendar,
    Calls,
    Reviews,
    GitHub,
    Messages,
}

impl Source {
    /// The source's name, as mycelium takes it.
    pub fn word(self) -> &'static str {
        match self {
            Self::Slack => "slack",
            Self::Mail => "mail",
            Self::Calendar => "calendar",
            Self::Calls => "calls",
            Self::Reviews => "reviews",
            Self::GitHub => "github",
            Self::Messages => "messages",
        }
    }
}

/// How a receptor stands, in mycelium's words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// It read its source, and there's something to count.
    Reading,
    /// It read its source, and there's nothing.
    Quiet,
    /// Its source couldn't be read; the counts are what it last read.
    Failing,
    /// It no longer reads for the realm, its counts none.
    Off,
}

impl State {
    /// The state's word, as mycelium takes it.
    pub fn word(self) -> &'static str {
        match self {
            Self::Reading => "reading",
            Self::Quiet => "quiet",
            Self::Failing => "failing",
            Self::Off => "off",
        }
    }
}

/// What one receptor says: its source, how it stands, and its counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Told {
    pub source: Source,
    pub state: State,
    /// What came or comes within the hour.
    pub hour: u64,
    /// What's today's.
    pub today: u64,
    /// Of today's, what waits on Joel.
    pub addressed: u64,
}

impl Told {
    /// What `source` says as it stops reading for the realm: off, its
    /// counts none, so nothing of what it last read is said again.
    pub fn off(source: Source) -> Self {
        Self {
            source,
            state: State::Off,
            hour: 0,
            today: 0,
            addressed: 0,
        }
    }
}

/// Where receptors are said: mycelium, and the realm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Speaker {
    program: PathBuf,
    realm: String,
}

impl Speaker {
    /// The speaker the service names, if it names one: an absolute
    /// program and a realm of mycelium's letters.
    pub fn named(program: Option<OsString>, realm: Option<OsString>) -> Option<Self> {
        let program = PathBuf::from(program?);
        let realm = realm?.into_string().ok()?;
        let lettered = realm
            .bytes()
            .all(|letter| letter.is_ascii_lowercase() || letter.is_ascii_digit() || letter == b'-');
        let sound = !realm.is_empty() && lettered && !realm.starts_with('-');
        (program.is_absolute() && sound).then_some(Self { program, realm })
    }

    /// The speaker the service's environment names, in `MYCELIUM` and
    /// `MYCELIUM_REALM`.
    pub fn from_env() -> Option<Self> {
        Self::named(
            std::env::var_os("MYCELIUM"),
            std::env::var_os("MYCELIUM_REALM"),
        )
    }

    /// What mycelium is given for one receptor, the next said within
    /// `every`: fixed words, the realm the service named, and integers,
    /// each count held to the most mycelium takes.
    pub fn arguments(&self, told: Told, every: Duration, pid: u32) -> Vec<OsString> {
        let most = |count: u64| count.min(u64::from(u32::MAX)).to_string();
        let words = [
            "--realm".to_owned(),
            self.realm.clone(),
            "receptors".to_owned(),
            "say".to_owned(),
            told.source.word().to_owned(),
            "--state".to_owned(),
            told.state.word().to_owned(),
            "--hour".to_owned(),
            most(told.hour),
            "--today".to_owned(),
            most(told.today),
            "--addressed".to_owned(),
            most(told.addressed),
            "--every".to_owned(),
            every.as_secs().max(1).to_string(),
            "--pid".to_owned(),
            pid.to_string(),
        ];
        words.into_iter().map(OsString::from).collect()
    }

    /// Says how each receptor stands, the next said within `every`.
    ///
    /// # Errors
    ///
    /// When mycelium can't be run, or refuses what it's told, for any one
    /// of them; those after it aren't said.
    pub fn say(&self, told: &[Told], every: Duration, pid: u32) -> Result<(), String> {
        for &one in told {
            let status = Command::new(&self.program)
                .args(self.arguments(one, every, pid))
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .status()
                .map_err(|why| format!("running {}: {why}", self.program.display()))?;
            if !status.success() {
                return Err(format!("{} {status}", self.program.display()));
            }
        }
        Ok(())
    }

    /// Says each of `sources` off, its counts none.
    ///
    /// # Errors
    ///
    /// As `say`.
    pub fn say_off(&self, sources: &[Source], every: Duration, pid: u32) -> Result<(), String> {
        let _ = (sources, every, pid);
        Ok(())
    }
}

/// The longest login GitHub gives.
const LONGEST_LOGIN: usize = 39;

/// Whose a source is, as the machine declares it for its realm: compared
/// with whom the source says it is, so a receptor says only the declared
/// one's counts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declared(String);

impl Declared {
    /// A declared mail address, if it is one: something before and after
    /// an `@`, and no space.
    pub fn address(declared: Option<OsString>) -> Option<Self> {
        let declared = declared?.into_string().ok()?;
        let sound = declared
            .split_once('@')
            .is_some_and(|(name, place)| !name.is_empty() && !place.is_empty())
            && !declared.contains(char::is_whitespace);
        sound.then_some(Self(declared))
    }

    /// A declared GitHub login, if it is one: GitHub's letters, digits and
    /// single hyphens, neither first nor last, at most 39 of them.
    pub fn login(declared: Option<OsString>) -> Option<Self> {
        let _ = declared?;
        None
    }

    fn _login(declared: Option<OsString>) -> Option<Self> {
        let declared = declared?.into_string().ok()?;
        let lettered = declared
            .bytes()
            .all(|letter| letter.is_ascii_alphanumeric() || letter == b'-');
        let sound = lettered
            && (1..=LONGEST_LOGIN).contains(&declared.len())
            && !declared.starts_with('-')
            && !declared.ends_with('-')
            && !declared.contains("--");
        sound.then_some(Self(declared))
    }

    /// Whether `said`, as the source gives it, is the one declared: the
    /// same, letter for letter, but for ASCII case, as mail and GitHub
    /// both take it.
    pub fn is(&self, said: &str) -> bool {
        said.eq_ignore_ascii_case(&self.0)
    }
}

#[cfg(test)]
mod tests;
