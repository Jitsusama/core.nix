//! A hold on swaps: while one stands, no update moves a `current`, however
//! it runs (the timer, a switch's activation, or by hand with `--now`).
//!
//! Stopping the timer doesn't stop a switch, whose activation starts the
//! service on its own: on 11 November a switch swapped in a change while
//! the timer was stopped to keep it out. So the hold is a file under the
//! root every package lives in, read by every update. A package with no
//! version yet is still installed under it, as under the hour's spacing,
//! since pi can't load it at all until it is. A hold says why, and may
//! end by itself; one that can't be read holds, so a garbled file never
//! lets a swap through.

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::config::Config;

/// A hold that stands: why, and when it ends by itself, if it does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hold {
    pub why: String,
    pub until: Option<SystemTime>,
}

/// The word in the file for a hold only a release ends.
const NEVER: &str = "never";

/// Where the hold is kept, beside the time of the last swap.
#[must_use]
pub fn file(config: &Config) -> PathBuf {
    config.root.join(".hold")
}

/// The hold that stands at `now`, if one does.
#[must_use]
pub fn standing(config: &Config, now: SystemTime) -> Option<Hold> {
    let text = match fs::read_to_string(file(config)) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => {
            return Some(Hold {
                why: format!("its hold can't be read: {error}"),
                until: None,
            });
        }
    };
    let hold = parse(&text).unwrap_or_else(|| Hold {
        why: format!("its hold file says what isn't a hold: {}", text.trim()),
        until: None,
    });
    match hold.until {
        Some(until) if until <= now => None,
        _ => Some(hold),
    }
}

/// Put a hold in place, replacing any that stood.
///
/// # Errors
///
/// Says why the file couldn't be written, which leaves any earlier hold.
pub fn put(config: &Config, hold: &Hold) -> Result<(), String> {
    let path = file(config);
    let written = path.with_extension("new");
    fs::create_dir_all(&config.root)
        .map_err(|error| format!("{}: {error}", config.root.display()))?;
    fs::write(&written, render(hold))
        .and_then(|()| fs::rename(&written, &path))
        .map_err(|error| format!("{}: {error}", path.display()))
}

/// End the hold; whether one stood to end.
///
/// # Errors
///
/// Says why the file couldn't be removed.
pub fn release(config: &Config) -> Result<bool, String> {
    let path = file(config);
    match fs::remove_file(&path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

/// How long a hold asked for lasts: whole minutes, `90m`, or hours, `2h`.
#[must_use]
pub fn lasting(words: &str) -> Option<Duration> {
    let (count, minutes) = match words.strip_suffix('h') {
        Some(hours) => (hours, 60),
        None => (words.strip_suffix('m').unwrap_or(words), 1),
    };
    let count: u64 = count.parse().ok().filter(|count| *count > 0)?;
    Some(Duration::from_mins(count.checked_mul(minutes)?))
}

/// The file's text: when it ends, as seconds since the epoch or `never`,
/// then why.
fn render(hold: &Hold) -> String {
    let until = hold
        .until
        .and_then(|until| until.duration_since(UNIX_EPOCH).ok())
        .map_or_else(|| NEVER.to_owned(), |since| since.as_secs().to_string());
    format!("until {until}\n{}\n", hold.why.trim())
}

fn parse(text: &str) -> Option<Hold> {
    let (first, why) = text.split_once('\n')?;
    let until = first.strip_prefix("until ")?.trim();
    let until = if until == NEVER {
        None
    } else {
        Some(UNIX_EPOCH + Duration::from_secs(until.parse().ok()?))
    };
    Some(Hold {
        why: why.trim().to_owned(),
        until,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hold_reads_back_as_it_was_put() {
        let until = UNIX_EPOCH + Duration::from_secs(1_794_400_000);
        for hold in [
            Hold {
                why: "a switch is under way".to_owned(),
                until: Some(until),
            },
            Hold {
                why: "until the fix lands".to_owned(),
                until: None,
            },
        ] {
            assert_eq!(parse(&render(&hold)), Some(hold.clone()), "{hold:?}");
        }
    }

    #[test]
    fn a_length_is_minutes_or_hours() {
        assert_eq!(lasting("90"), Some(Duration::from_mins(90)), "bare minutes");
        assert_eq!(lasting("45m"), Some(Duration::from_mins(45)), "minutes");
        assert_eq!(lasting("2h"), Some(Duration::from_hours(2)), "hours");
        for wrong in ["", "0", "h", "-5m", "2d", "soon"] {
            assert_eq!(lasting(wrong), None, "{wrong:?} was taken as a length");
        }
    }

    #[test]
    fn what_isnt_a_hold_reads_as_none() {
        for wrong in ["", "until\n", "until later\nwhy\n", "why\n"] {
            assert_eq!(parse(wrong), None, "{wrong:?} read as a hold");
        }
    }
}
