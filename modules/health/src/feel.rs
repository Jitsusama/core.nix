//! How using the machine felt: the day's most painful interactions, as the
//! pain-points detector ranks them in `pain-points/<date>.jsonl` beside
//! health's own folder, worst first. Health keeps them in its day, so
//! cadence reads one file, and `health` opens with the worst three.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// One interaction that hurt, as the detector writes it.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Pain {
    /// When it started, in seconds since the epoch.
    pub at: i64,
    /// Where: the launcher, or pi.
    pub place: String,
    /// What Joel was after, as `program:slack`, `on:look` or `sound`.
    pub trying: String,
    /// How long it took him.
    pub seconds: f64,
    /// How long it usually takes.
    pub usual_seconds: f64,
    /// The seconds the detector says were lost.
    pub lost_seconds: f64,
    /// Its rank in the day, 1 the worst.
    pub rank: u32,
    /// What slowed him: waiting, hunting, gave up or tried again.
    pub slowed: String,
    /// Which part was at fault: opening, search and drawing are the
    /// shell's speed, finding the launcher's ranking and names.
    pub part: String,
    /// How many times he opened the place for it; in pi, the messages
    /// he sent in the go.
    pub openings: u32,
    /// What his keys did meanwhile, in the launcher; pi's signs are its
    /// own, and left out.
    pub signs: Signs,
}

/// What Joel's keys did during one interaction.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Signs {
    /// Letters typed.
    pub letters: u32,
    /// Letters erased.
    pub erased: u32,
}

/// The day's pains from `folder`, worst first; a line that isn't one is
/// left out, and a day with no file has none.
#[must_use]
pub fn read(folder: &Path, date: &str) -> Vec<Pain> {
    let said = fs::read_to_string(folder.join(format!("{date}.jsonl"))).unwrap_or_default();
    let mut pains: Vec<Pain> = said
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    pains.sort_by_key(|pain| pain.rank);
    pains
}

/// One pain in a line Joel reads without knowing the detector, after its
/// time: "finding Slack took 89 s over 2 openings, 67 s lost to trying
/// again (18 letters typed, 13 erased)".
#[must_use]
pub fn said(pain: &Pain) -> String {
    let in_pi = pain.place == "pi";
    let doing = match pain.part.as_str() {
        _ if in_pi => "a pi session".to_owned(),
        "opening" => format!("opening the launcher for {}", named(&pain.trying)),
        "search" => format!("searching for {}", named(&pain.trying)),
        "drawing" => format!("waiting on the launcher to draw {}", named(&pain.trying)),
        _ => format!("finding {}", named(&pain.trying)),
    };
    let mut line = format!("{doing} took {} s", whole(pain.seconds));
    if pain.openings > 1 {
        let over = if in_pi { "messages" } else { "openings" };
        let _ = write!(line, " over {} {over}", pain.openings);
    }
    // The detector counts two minutes at most a go, so past that it
    // can't say how much.
    if pain.lost_seconds >= LOST_AT_MOST {
        let _ = write!(line, ", 2 min or more lost to {}", slowed(&pain.slowed));
    } else if pain.lost_seconds > 0.0 {
        let _ = write!(
            line,
            ", {} s lost to {}",
            whole(pain.lost_seconds),
            slowed(&pain.slowed)
        );
    }
    if pain.signs.letters > 0 {
        let _ = write!(
            line,
            " ({} letters typed, {} erased)",
            pain.signs.letters, pain.signs.erased
        );
    }
    line
}

/// What he was after, in his words: `program:slack` is Slack, `on:look`
/// is look, and `session:awake` the session awake.
fn named(trying: &str) -> String {
    let (what, name) = trying.split_once(':').unwrap_or(("", trying));
    let mut letters = name.chars();
    let capital = letters
        .next()
        .map(|first| first.to_uppercase().chain(letters).collect::<String>())
        .unwrap_or_default();
    match what {
        "session" => format!("the session {name}"),
        "" if name == "looked for something" => "something".to_owned(),
        _ => capital,
    }
}

/// What slowed him, said as a cause.
fn slowed(slowed: &str) -> &str {
    match slowed {
        "tried again" => "trying again",
        "gave up" => "giving up",
        "stopped it" => "stopping it",
        other => other,
    }
}

/// The most the detector counts lost in one go, its give-up mark.
const LOST_AT_MOST: f64 = 120.0;

/// Seconds to the nearest whole one, as the detector's are tenths.
fn whole(seconds: f64) -> String {
    format!("{seconds:.0}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_days_pains_are_read_worst_first() {
        let folder = tempfile::tempdir().expect("a folder");
        fs::write(
            folder.path().join("2026-11-12.jsonl"),
            "{\"at\":1,\"trying\":\"sound\",\"rank\":2}\nnot a pain\n\
             {\"at\":2,\"trying\":\"program:slack\",\"rank\":1,\"seconds\":89.0,\
             \"signs\":{\"letters\":18,\"erased\":13,\"arrows\":2}}\n",
        )
        .expect("the day's file");
        let pains = read(folder.path(), "2026-11-12");
        let trying: Vec<&str> = pains.iter().map(|pain| pain.trying.as_str()).collect();
        assert_eq!(
            trying,
            ["program:slack", "sound"],
            "worst first, the rest left out"
        );
        assert!(
            read(folder.path(), "2026-11-13").is_empty(),
            "a day without a file"
        );
    }

    #[test]
    fn a_pain_is_said_in_plain_words() {
        let pain = Pain {
            trying: "program:slack".into(),
            seconds: 89.4,
            lost_seconds: 67.0,
            slowed: "tried again".into(),
            part: "finding".into(),
            openings: 2,
            signs: Signs {
                letters: 18,
                erased: 13,
            },
            ..Pain::default()
        };
        assert_eq!(
            said(&pain),
            "finding Slack took 89 s over 2 openings, 67 s lost to trying again \
             (18 letters typed, 13 erased)"
        );
        let waited = Pain {
            trying: "session:awake".into(),
            seconds: 4.2,
            part: "opening".into(),
            openings: 1,
            ..Pain::default()
        };
        assert_eq!(
            said(&waited),
            "opening the launcher for the session awake took 4 s"
        );
    }

    #[test]
    fn a_pi_go_is_said_as_pi_and_its_cap_as_a_floor() {
        let line = r#"{"at":3,"place":"pi","trying":"a session","seconds":300.0,
            "lost_seconds":120.0,"slowed":"waiting","part":"reply","openings":3,
            "signs":{"waited_ms":200000,"errors":0,"status":"ok"}}"#;
        let pain: Pain = serde_json::from_str(line).expect("a pi line isn't read");
        assert_eq!(
            said(&pain),
            "a pi session took 300 s over 3 messages, 2 min or more lost to waiting",
            "a pi go reads as the launcher's, or its two-minute cap as exact"
        );
    }
}
