//! One opening of the launcher, as its Trials.js keeps it in
//! launcher-trials.jsonl: the keys pressed by kind, what was picked, how
//! long that took and how long the launcher was open, and how its slowest
//! keys and frames went.

use serde::Deserialize;

/// The keys of one opening, counted by kind.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Keys {
    pub letters: u32,
    pub erased: u32,
    pub arrows: u32,
    pub steps: u32,
    pub backs: u32,
    pub tabs: u32,
    pub returns: u32,
}

/// A key Joel could feel: how long its search took and how long until
/// the frame that drew it, in milliseconds.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Felt {
    pub searched: f64,
    pub drawn: f64,
}

/// The keys the launcher timed in one opening, of which the felt ones
/// are kept.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Timed {
    pub felt: Vec<Felt>,
}

/// The frames drawn while the opening moved: how many came late and the
/// longest gap before one, in milliseconds.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Frames {
    pub drawn: u32,
    pub late: u32,
    pub longest: f64,
}

/// One opening. `at` is when it opened, in milliseconds since the epoch;
/// `ms` how long until the pick, or -1 with nothing picked; `open` how
/// long it stayed open.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(default)]
pub struct Trial {
    pub at: i64,
    pub keys: Keys,
    pub picked: String,
    pub ms: i64,
    pub open: i64,
    pub missed: bool,
    pub timed: Option<Timed>,
    pub frames: Option<Frames>,
}

impl Default for Trial {
    fn default() -> Self {
        Trial {
            at: 0,
            keys: Keys::default(),
            picked: String::new(),
            ms: -1,
            open: -1,
            missed: false,
            timed: None,
            frames: None,
        }
    }
}

/// The kinds of pick whose second word is the launcher's own vocabulary,
/// a program's id or a verb's, rather than what Joel wrote or named: a
/// copy's second word is the text copied, an agent's its id, a file's its
/// path, so those keep their first word alone.
const WORDED: &[&str] = &[
    "bluetooth",
    "calendar",
    "camera",
    "capture",
    "collection",
    "file-chooser",
    "keyboard",
    "notice",
    "on",
    "program",
    "return",
    "session",
    "shortcut",
    "slack",
    "spend",
    "swayimg",
    "today",
    "transcript",
    "verb",
    "waits",
    "warp",
    "window",
];

impl Trial {
    /// Whether Joel set out to find something: he typed or stepped in.
    pub fn sought(&self) -> bool {
        self.keys.letters > 0 || self.keys.steps > 0
    }

    /// Whether something was picked.
    pub fn picked_one(&self) -> bool {
        !self.picked.is_empty()
    }

    /// When it closed, in milliseconds since the epoch.
    pub fn closed(&self) -> i64 {
        self.at + self.open.max(self.ms).max(0)
    }

    /// What Joel was doing, as a kind and never its content: the pick's
    /// first word, and its second where that is the launcher's own word,
    /// such as "program:slack" or "capture:window"; "looked for something"
    /// where he typed or stepped in and picked nothing, and "glanced"
    /// where he did neither.
    pub fn trying(&self) -> String {
        if !self.picked_one() {
            return if self.sought() {
                "looked for something"
            } else {
                "glanced"
            }
            .to_string();
        }
        let mut words = self.picked.split(':');
        let first = words.next().unwrap_or_default();
        match words.next() {
            Some(second) if WORDED.contains(&first) && vocabulary(second) => {
                format!("{first}:{second}")
            }
            _ => first.to_string(),
        }
    }
}

/// Whether a word reads as the launcher's own: an id in lower case, as
/// "google-chrome" or "keyboards-lights-1", or a few words of a label with
/// no digits, as "Power Off". Anything longer, or with digits in a label,
/// may be something Joel named.
fn vocabulary(word: &str) -> bool {
    let id = word.len() <= 40
        && word.starts_with(|c: char| c.is_ascii_lowercase())
        && word
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "-._".contains(c));
    let label = word.len() <= 24
        && word.split(' ').count() <= 3
        && word.starts_with(|c: char| c.is_ascii_alphabetic())
        && word.chars().all(|c| c.is_ascii_alphabetic() || c == ' ');
    id || label
}

/// The trials in the text of launcher-trials.jsonl, a line each; a line
/// that doesn't read as one, as the last one cut short by a write, is
/// passed over.
pub fn read(text: &str) -> Vec<Trial> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picking(picked: &str) -> Trial {
        Trial {
            picked: picked.to_string(),
            ms: 2000,
            open: 2000,
            ..Trial::default()
        }
    }

    #[test]
    fn a_program_s_pick_says_which_program() {
        assert_eq!(picking("program:slack").trying(), "program:slack");
        assert_eq!(picking("capture:window").trying(), "capture:window");
        assert_eq!(picking("session:Power Off").trying(), "session:Power Off");
    }

    #[test]
    fn a_copy_s_pick_never_says_what_was_copied() {
        assert_eq!(
            picking("copy:@Levi heads up from the review").trying(),
            "copy"
        );
        assert_eq!(picking("copy:short").trying(), "copy");
    }

    #[test]
    fn ids_names_and_paths_stay_out() {
        assert_eq!(picking("agent:85PKB2").trying(), "agent");
        assert_eq!(
            picking("file:/home/jitsusama/Documents/x.pdf").trying(),
            "file"
        );
        assert_eq!(picking("on:look:bad88285473c0b1a").trying(), "on:look");
        assert_eq!(picking("person:Some One").trying(), "person");
        assert_eq!(
            picking("slack:conversation:C0123ABCD").trying(),
            "slack:conversation"
        );
    }

    #[test]
    fn nothing_picked_says_whether_he_looked() {
        let looked = Trial {
            keys: Keys {
                letters: 3,
                ..Keys::default()
            },
            ..Trial::default()
        };
        assert_eq!(looked.trying(), "looked for something");
        let stepped = Trial {
            keys: Keys {
                steps: 1,
                ..Keys::default()
            },
            ..Trial::default()
        };
        assert_eq!(stepped.trying(), "looked for something");
        assert_eq!(Trial::default().trying(), "glanced");
    }

    #[test]
    fn a_trial_reads_from_the_launcher_s_line() {
        let line = r#"{"at":1794455364916,"home":"today","keys":{"letters":4,"erased":4,"arrows":9,"steps":0,"backs":0,"tabs":0,"returns":3},"picked":"on:look:bad8","kind":"","typed":"","depth":3,"ms":19521,"open":19546,"missed":false,"timed":{"count":2,"felt":[{"searched":14,"drawn":20,"parts":null}]},"frames":{"drawn":1733,"late":6,"longest":69}}"#;
        let trials = read(&format!("{line}\n{{\"at\":1,\n"));
        assert_eq!(trials.len(), 1, "the cut-short line is passed over");
        let trial = &trials[0];
        assert_eq!(trial.at, 1_794_455_364_916);
        assert_eq!(trial.keys.erased, 4);
        assert_eq!(trial.ms, 19521);
        assert_eq!(trial.closed(), 1_794_455_364_916 + 19_546);
        assert_eq!(trial.timed.as_ref().map(|t| t.felt.len()), Some(1));
        assert_eq!(trial.frames.as_ref().map(|f| f.longest), Some(69.0));
    }
}
