//! Whether a signal has come, from what the machine says of itself, kept
//! apart from reading it so each case is tested without the machine.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::todo::Signal;

/// One switch the follower made, from its journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Switch {
    pub at: i64,
    /// The core it built, when it says.
    pub core: Option<String>,
}

/// What the machine says of itself, each read where it lives.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts {
    /// When this boot began.
    pub boot: Option<i64>,
    /// When the system profile last moved, a switch by any means.
    pub profile: Option<i64>,
    /// The follower's switches, oldest first.
    pub switches: Vec<Switch>,
    /// The core the machine runs, from the follower's newest look.
    pub core: Option<String>,
    /// When each service was last signed in to.
    pub signins: HashMap<String, i64>,
    /// The names of the devices present.
    pub devices: Vec<String>,
    /// The files a to-do waits on that are there, with when each changed.
    pub files: HashMap<PathBuf, i64>,
}

/// What came: when, and the figure that showed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heard {
    pub at: Option<i64>,
    /// The signal's own figure, by its name: `boot_at`, `core`, `device`
    /// or `changed_at`.
    pub figure: (&'static str, String),
}

/// Whether `commit` is `core` or before it, as git's history says; none
/// while that history doesn't have one of them.
pub trait History {
    fn reaches(&self, core: &str, commit: &str) -> Option<bool>;
}

/// Whether `signal` has come for a to-do raised at `raised`, and if not,
/// the figure that says where it stands.
pub fn heard(
    signal: &Signal,
    raised: Option<i64>,
    facts: &Facts,
    history: &dyn History,
) -> Result<Heard, Option<(&'static str, String)>> {
    let after = |at: Option<i64>| at.is_some_and(|at| raised.is_none_or(|raised| at > raised));
    match signal {
        Signal::Boot => {
            let figure = facts.boot.map(|at| ("boot_at", at.to_string()));
            if after(facts.boot) {
                Ok(Heard {
                    at: facts.boot,
                    figure: figure.unwrap_or_default(),
                })
            } else {
                Err(figure)
            }
        }
        Signal::Switch(None) => {
            let last = facts
                .switches
                .iter()
                .map(|s| s.at)
                .chain(facts.profile)
                .max();
            let figure = facts.core.clone().map(|core| ("core", core));
            if after(last) {
                Ok(Heard {
                    at: last,
                    figure: figure.unwrap_or(("core", String::new())),
                })
            } else {
                Err(figure)
            }
        }
        Signal::Switch(Some(commit)) => {
            let Some(core) = facts.core.clone() else {
                return Err(None);
            };
            if history.reaches(&core, commit) == Some(true) {
                let at = facts
                    .switches
                    .iter()
                    .find(|s| s.core.as_deref() == Some(core.as_str()))
                    .map(|s| s.at);
                Ok(Heard {
                    at,
                    figure: ("core", core),
                })
            } else {
                Err(Some(("core", core)))
            }
        }
        Signal::Signin(service) => {
            let at = facts.signins.get(service).copied();
            if after(at) {
                Ok(Heard {
                    at,
                    figure: ("changed_at", at.unwrap_or_default().to_string()),
                })
            } else {
                // Nothing writes a service's sign-in yet but what a test
                // does, so one never made says so rather than nothing.
                Err(Some(at.map_or(("signed_in", "never".to_owned()), |at| {
                    ("changed_at", at.to_string())
                })))
            }
        }
        Signal::Device(name) => facts
            .devices
            .iter()
            .find(|d| plain(d).contains(&plain(name)))
            .map(|d| Heard {
                at: None,
                figure: ("device", d.clone()),
            })
            .ok_or(None),
        // A file counts once it's written as the to-do is raised or
        // after, within the second both are told in.
        Signal::File(path) => match facts.files.get(path) {
            Some(at) if raised.is_none_or(|raised| *at >= raised) => Ok(Heard {
                at: Some(*at),
                figure: ("changed_at", at.to_string()),
            }),
            Some(at) => Err(Some(("changed_at", at.to_string()))),
            None => Err(None),
        },
    }
}

/// A device's name as it's matched: its letters and digits alone, in
/// lower case, so `ergodox-ez` finds "ErgoDox EZ".
fn plain(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A history where each commit is past every one before it in `line`.
    struct Line(Vec<&'static str>);

    impl History for Line {
        fn reaches(&self, core: &str, commit: &str) -> Option<bool> {
            let at = |c: &str| {
                self.0
                    .iter()
                    .position(|x| c.starts_with(x) || x.starts_with(c))
            };
            Some(at(commit)? <= at(core)?)
        }
    }

    fn facts() -> Facts {
        Facts {
            boot: Some(100),
            profile: Some(150),
            switches: vec![
                Switch {
                    at: 120,
                    core: Some("aaa".into()),
                },
                Switch {
                    at: 200,
                    core: Some("bbb".into()),
                },
            ],
            core: Some("bbb".into()),
            ..Facts::default()
        }
    }

    #[test]
    fn a_boot_counts_only_after_the_to_do_was_raised() {
        let line = Line(vec![]);
        assert!(
            heard(&Signal::Boot, Some(90), &facts(), &line).is_ok(),
            "a boot after it was raised counts"
        );
        assert_eq!(
            heard(&Signal::Boot, Some(110), &facts(), &line),
            Err(Some(("boot_at", "100".into())))
        );
    }

    #[test]
    fn any_switch_after_it_counts_the_profiles_too() {
        let line = Line(vec![]);
        let found = heard(&Signal::Switch(None), Some(190), &facts(), &line).unwrap();
        assert_eq!(found.at, Some(200));
        let mut by_hand = facts();
        by_hand.switches.clear();
        assert!(
            heard(&Signal::Switch(None), Some(140), &by_hand, &line).is_ok(),
            "a profile made after it counts"
        );
        assert!(
            heard(&Signal::Switch(None), Some(201), &facts(), &line).is_err(),
            "no switch came after it"
        );
    }

    #[test]
    fn a_switch_to_a_commit_is_heard_at_it_or_past_it() {
        let line = Line(vec!["aaa", "abc", "bbb", "ccc"]);
        let at = heard(&Signal::Switch(Some("abc".into())), None, &facts(), &line).unwrap();
        assert_eq!(
            at,
            Heard {
                at: Some(200),
                figure: ("core", "bbb".into())
            }
        );
        assert_eq!(
            heard(&Signal::Switch(Some("ccc".into())), None, &facts(), &line),
            Err(Some(("core", "bbb".into())))
        );
        // A commit the history doesn't have isn't past yet.
        assert!(
            heard(&Signal::Switch(Some("zzz".into())), None, &facts(), &line).is_err(),
            "a commit not in the history isn't past"
        );
    }

    #[test]
    fn with_no_core_known_no_switch_to_a_commit_is() {
        let line = Line(vec!["aaa"]);
        let none = Facts::default();
        assert_eq!(
            heard(&Signal::Switch(Some("aaa".into())), None, &none, &line),
            Err(None)
        );
    }

    #[test]
    fn a_sign_in_a_device_and_a_file_are_each_heard() {
        let line = Line(vec![]);
        let mut found = facts();
        found.signins.insert("google".into(), 300);
        found.devices.push("Thunderbolt Dock TB4".into());
        found.files.insert("/run/x".into(), 400);
        assert!(
            heard(&Signal::Signin("google".into()), Some(250), &found, &line).is_ok(),
            "signed in after it was raised"
        );
        assert!(
            heard(&Signal::Signin("google".into()), Some(350), &found, &line).is_err(),
            "signed in before it was raised"
        );
        assert!(
            heard(&Signal::Signin("slack".into()), None, &found, &line).is_err(),
            "never signed in"
        );
        let dock = heard(&Signal::Device("dock".into()), None, &found, &line).unwrap();
        assert_eq!(dock.figure, ("device", "Thunderbolt Dock TB4".into()));
        assert!(
            heard(&Signal::Device("Keyboard".into()), None, &found, &line).is_err(),
            "no keyboard is plugged in"
        );
        let spelled = heard(&Signal::Device("dock-tb4".into()), None, &found, &line);
        assert!(
            spelled.is_ok(),
            "a device matches by its letters and digits"
        );
        assert_eq!(
            heard(&Signal::Signin("slack".into()), None, &found, &line),
            Err(Some(("signed_in", "never".into())))
        );
        assert!(
            heard(&Signal::File("/run/x".into()), None, &found, &line).is_ok(),
            "the file is there"
        );
        assert!(
            heard(&Signal::File("/run/x".into()), Some(400), &found, &line).is_ok(),
            "changed the second it was raised"
        );
        assert!(
            heard(&Signal::File("/run/x".into()), Some(401), &found, &line).is_err(),
            "changed before it was raised"
        );
        assert!(
            heard(&Signal::File("/run/y".into()), None, &found, &line).is_err(),
            "the file isn't there"
        );
    }
}
