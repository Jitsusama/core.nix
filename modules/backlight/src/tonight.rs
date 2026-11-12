//! Tonight's evening: the one set on the command line, its start moved to
//! when rhythm says Joel's wind-down begins tonight, two and a half hours
//! before he usually stops (modules/rhythm), read from its profile once a
//! minute.
//!
//! Why: Joel keeps no fixed bedtime, so the evening the light research
//! (RSCH-20261108-HGB8JZ) keeps dim is the one before his own stop, which
//! rhythm learns from when he's at the keys. Until it knows, its profile
//! says its guess, a midnight stop, so the evening starts at 21:30 as set
//! and the journal says it's a guess. With no profile, or one whose start
//! would stretch the evening past half a day, the start stays as set.

use std::fs;
use std::path::PathBuf;

use crate::evening::{Evening, clock, minute};

/// The longest evening a profile's start may make, in minutes: a stop
/// rhythm saw at dawn mustn't dim the whole day.
pub const LONGEST: f64 = 12.0 * 60.0;

/// Where rhythm's wind-down comes from, and what it last followed.
pub struct Tonight {
    set: Evening,
    profile: PathBuf,
    followed: Follows,
}

/// What tonight's evening follows.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Follows {
    /// Nothing yet, before the first look.
    Nothing,
    /// The evening as set.
    Set,
    /// Rhythm's wind-down, from `start`, and whether it's learned rather
    /// than a guess.
    WindDown { start: f64, learned: bool },
}

impl Tonight {
    /// Tonight's evening from `set`, moved by the profile at `profile`.
    #[must_use]
    pub fn new(set: Evening, profile: PathBuf) -> Self {
        Tonight {
            set,
            profile,
            followed: Follows::Nothing,
        }
    }

    /// The evening to dim by now, and a line for the journal when what it
    /// follows has changed since the last look.
    pub fn look(&mut self) -> (Evening, Option<String>) {
        let read = fs::read_to_string(&self.profile)
            .ok()
            .and_then(|text| wind_down(&text))
            .filter(|(start, _)| self.set.starting(*start).length() <= LONGEST);
        let follows = read.map_or(Follows::Set, |(start, learned)| Follows::WindDown {
            start,
            learned,
        });
        let evening = read.map_or(self.set, |(start, _)| self.set.starting(start));
        let said = (self.followed != follows).then(|| match follows {
            Follows::WindDown {
                start,
                learned: true,
            } => format!(
                "the evening dims from {}, two and a half hours before Joel usually stops",
                clock(start)
            ),
            Follows::WindDown { start, .. } => format!(
                "the evening dims from {}, rhythm's guess until it learns when Joel stops",
                clock(start)
            ),
            Follows::Set | Follows::Nothing => {
                format!("the evening dims from {}, as set", clock(self.set.start()))
            }
        });
        self.followed = follows;
        (evening, said)
    }
}

/// The text of `key`'s value in a profile rhythm wrote, which says each as
/// `"key": "value"`.
fn field<'a>(profile: &'a str, key: &str) -> Option<&'a str> {
    let after = &profile[profile.find(&format!("\"{key}\""))? + key.len() + 2..];
    let value = after
        .trim_start()
        .strip_prefix(':')?
        .trim_start()
        .strip_prefix('"')?;
    Some(&value[..value.find('"')?])
}

/// The wind-down's start in a profile rhythm wrote, in minutes past
/// midnight, and whether it's learned rather than a guess.
#[must_use]
pub fn wind_down(profile: &str) -> Option<(f64, bool)> {
    let start = minute(field(profile, "windDown")?)?;
    Some((start, field(profile, "basis") == Some("learned")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evening::SHARE;

    fn profile(basis: &str, wind_down: &str) -> String {
        format!(
            "{{\"v\": 1, \"basis\": \"{basis}\", \"nights\": 9, \"usual\": \"00:40\", \
             \"weekday\": \"Thursday\", \"tonight\": \"00:40\", \"windDown\": \"{wind_down}\", \
             \"sunrise\": null, \"sunset\": null, \"weekdays\": {{}}}}"
        )
    }

    /// A profile file of its own for `test`, or none when `said` is.
    fn tonight(test: &str, said: Option<&str>) -> (Tonight, PathBuf) {
        let root = std::env::temp_dir().join(format!("tonight-{}-{test}", std::process::id()));
        fs::create_dir_all(&root).expect("a folder for the profile");
        let path = root.join("profile.json");
        let _ = fs::remove_file(&path);
        if let Some(said) = said {
            fs::write(&path, said).expect("the profile written");
        }
        let set = Evening::read("21:30-05:00").expect("a well-formed evening");
        (Tonight::new(set, path.clone()), path)
    }

    fn share_at(evening: Evening, hour: f64, minute: f64) -> f64 {
        evening.share(hour * 60.0 + minute)
    }

    #[test]
    fn rhythm_s_wind_down_is_read_with_whether_it_s_learned() {
        assert_eq!(
            wind_down(&profile("learned", "22:10")),
            Some((22.0 * 60.0 + 10.0, true))
        );
        assert_eq!(
            wind_down(&profile("guess", "21:30")),
            Some((21.0 * 60.0 + 30.0, false))
        );
        for said in ["", "{}", "{\"windDown\": \"25:00\"}", "not a profile"] {
            assert_eq!(wind_down(said), None, "{said:?} read as a wind-down");
        }
    }

    #[test]
    fn a_learned_wind_down_starts_the_evening() {
        let (mut tonight, _) = tonight("learned", Some(&profile("learned", "22:10")));
        let (evening, _) = tonight.look();
        let before = share_at(evening, 22.0, 0.0);
        assert!(
            (before - 1.0).abs() < 1e-9,
            "dimmed to {before} before Joel's wind-down"
        );
        let after = share_at(evening, 22.0, 40.0);
        assert!(
            (after - SHARE).abs() < 1e-9,
            "half an hour into it, only {after}"
        );
    }

    #[test]
    fn with_no_profile_the_evening_starts_as_set() {
        let (mut tonight, _) = tonight("none", None);
        let (evening, said) = tonight.look();
        let share = share_at(evening, 22.0, 0.0);
        assert!((share - SHARE).abs() < 1e-9, "at 22:00 it left {share}");
        let said = said.expect("the first look says what it follows");
        assert!(said.contains("21:30"), "said {said:?}");
    }

    #[test]
    fn a_wind_down_that_would_dim_past_half_a_day_is_left() {
        // A night rhythm saw end at eight in the morning starts its
        // wind-down at 05:30, which would dim the screen until the next
        // dawn.
        let (mut tonight, _) = tonight("dawn", Some(&profile("learned", "05:30")));
        let (evening, _) = tonight.look();
        let noon = share_at(evening, 12.0, 0.0);
        assert!((noon - 1.0).abs() < 1e-9, "dimmed at noon to {noon}");
        let late = share_at(evening, 22.0, 0.0);
        assert!((late - SHARE).abs() < 1e-9, "at 22:00 it left {late}");
    }

    #[test]
    fn what_it_follows_is_said_once_and_again_when_it_changes() {
        let (mut tonight, path) = tonight("said", Some(&profile("guess", "21:30")));
        let first = tonight.look().1.expect("the first look says");
        assert!(first.contains("guess"), "said {first:?}");
        assert_eq!(tonight.look().1, None, "said again unchanged");
        fs::write(&path, profile("learned", "23:00")).expect("the profile rewritten");
        let moved = tonight.look().1.expect("a moved wind-down is said");
        assert!(moved.contains("23:00"), "said {moved:?}");
        fs::remove_file(&path).expect("the profile gone");
        let gone = tonight.look().1.expect("a profile gone is said");
        assert!(gone.contains("as set"), "said {gone:?}");
    }
}
