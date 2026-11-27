//! What keeps updates from running until an asker lets them: a pause,
//! which refuses the follower's asks while a person's still update, and
//! holds on commits, which refuse any head of main or of a followed input
//! that holds one, whoever asks. Both are kept in root's
//! state, so neither a switch, which starts an account's stopped units
//! again, nor a reboot undoes them; only `resume` or `lift` does.
//!
//! On 10 November an account's follower was stopped at 14:12 to keep the
//! machine where it was. A switch at 14:15 reloaded the account's service
//! manager, which started it again, and at 18:38 it built a head holding
//! a change a person had said not to switch to, held back only because
//! someone was at the machine.

use crate::config::Config;
use crate::follows::{self, Followed};
use crate::git::{self, Repo};
use crate::hosted;
use crate::run::Failure;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// The asks that pause updates and let them run again.
pub const PAUSE: &str = "pause";
pub const RESUME: &str = "resume";

/// The asks that hold a commit back and let it through again, each
/// followed by the commit.
pub const HOLD: &str = "hold";
pub const LIFT: &str = "lift";

/// What holds updates back.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Held {
    /// When updates were paused and by whom, if they are.
    #[serde(default)]
    pub paused: Option<By>,
    /// The commits no update may switch to a head holding.
    #[serde(default)]
    pub commits: Vec<Hold>,
}

/// Who asked, by uid, and when, in seconds since 1970.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct By {
    pub uid: u32,
    pub at: u64,
}

/// A commit held back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hold {
    pub commit: String,
    pub by: By,
}

fn file(config: &Config) -> PathBuf {
    config.state.join("held.json")
}

/// What holds updates back now. A file that can't be read holds nothing
/// back, as when nothing was ever held, but one that can't be parsed
/// pauses, since it was written to hold something.
pub fn read(config: &Config) -> Held {
    parsed(fs::read_to_string(file(config)).ok().as_deref())
}

fn parsed(text: Option<&str>) -> Held {
    match text {
        Some(text) => serde_json::from_str(text).unwrap_or_else(|_| Held {
            paused: Some(By { uid: 0, at: 0 }),
            commits: Vec::new(),
        }),
        None => Held::default(),
    }
}

fn write(config: &Config, held: &Held) -> Result<(), String> {
    let path = file(config);
    let new = path.with_extension("new");
    let text = serde_json::to_string(held).map_err(|e| e.to_string())?;
    fs::write(&new, text)
        .and_then(|()| fs::rename(&new, &path))
        .map_err(|e| format!("{}: {e}", path.display()))
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// Whether `word` is a commit's full id. A short one won't do: the host
/// answers whether a head holds a commit by naming their merge base in
/// full, and a prefix two commits share would hold back the wrong one.
#[must_use]
pub fn is_commit(word: &str) -> bool {
    word.len() == 40 && word.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Changes what's held as `ask` says, for the asker `uid`, and says what
/// holds now; none when `ask` isn't one of these.
pub fn changed(held: &mut Held, ask: &str, uid: u32, at: u64) -> Option<Result<String, String>> {
    let words: Vec<&str> = ask.split_whitespace().collect();
    let said = match words.as_slice() {
        [PAUSE] => {
            held.paused.get_or_insert(By { uid, at });
            Ok(())
        }
        [RESUME] => {
            held.paused = None;
            Ok(())
        }
        [HOLD, commit] if is_commit(commit) => {
            let commit = commit.to_ascii_lowercase();
            if !held.commits.iter().any(|hold| hold.commit == commit) {
                held.commits.push(Hold {
                    commit,
                    by: By { uid, at },
                });
            }
            Ok(())
        }
        [LIFT, commit] => {
            let before = held.commits.len();
            held.commits
                .retain(|hold| !hold.commit.eq_ignore_ascii_case(commit));
            if held.commits.len() == before {
                Err(format!("{commit} wasn't held, so nothing was lifted"))
            } else {
                Ok(())
            }
        }
        [HOLD | LIFT, ..] => Err(format!("`{HOLD}` and `{LIFT}` take one commit's full id")),
        _ => return None,
    };
    Some(said.map(|()| said_of(held)))
}

/// Takes an ask that changes what's held, keeps the change, and says what
/// holds now; none when the ask isn't one. An asker whose uid couldn't be
/// read changes nothing, since what's held names who asked.
pub fn ask(config: &Config, ask: &str, uid: Option<u32>) -> Option<Result<String, String>> {
    let mut held = read(config);
    let said = changed(&mut held, ask, uid.unwrap_or(u32::MAX), now())?;
    if uid.is_none() {
        return Some(Err(
            "who asked couldn't be told, so nothing changed".to_owned()
        ));
    }
    Some(said.and_then(|said| write(config, &held).map(|()| said)))
}

/// How what holds now begins, after anything the ask stopped, so an
/// asker can tell an ask taken from one refused.
const PAUSED: &str = "updates are paused";
const RUN: &str = "updates run";

/// Whether an answer says what holds now, so the ask was taken.
#[must_use]
pub fn says(why: &str) -> bool {
    why.contains(PAUSED) || why.contains(RUN)
}

/// What holds updates back, in a line.
#[must_use]
pub fn said_of(held: &Held) -> String {
    let paused = match &held.paused {
        Some(by) => format!(
            "{PAUSED} for the follower, since uid {} asked at {}; a person's ask still \
             updates, and `update-to-main {RESUME}` lets the follower's run",
            by.uid, by.at
        ),
        None => RUN.to_owned(),
    };
    if held.commits.is_empty() {
        return format!("{paused}, and no commit is held");
    }
    let commits: Vec<String> = held
        .commits
        .iter()
        .map(|hold| git::short(&hold.commit).to_owned())
        .collect();
    format!(
        "{paused}, and no head holding {} switches",
        commits.join(", ")
    )
}

/// Why no update the follower alone asked for may run now, if updates
/// are paused.
#[must_use]
pub fn paused(config: &Config) -> Option<String> {
    let held = read(config);
    held.paused.is_some().then(|| said_of(&held))
}

/// Why main at `head`, with `followed`, mustn't be switched to: it, or an
/// input as the build reads it, holds a commit that is held. A hold
/// neither can be seen to hold or not stops it too, since a hold is only
/// ever asked for to keep a change off the machine.
#[must_use]
pub fn stops(config: &Config, head: &str, followed: &Followed) -> Option<String> {
    let holds = read(config).commits;
    let first = holds.first()?;
    // Each input is judged at the commit the build reads, its followed one
    // or its lock's pin, so one the lock dropped is neither asked nor
    // fetched: its repository may have gone with it.
    let built = match follows::built(config, head, followed) {
        Ok(built) => built,
        Err(failure) => {
            return Some(format!(
                "{} is held, and what main at {} builds couldn't be read: {}",
                git::short(&first.commit),
                git::short(head),
                failure.why
            ));
        }
    };
    holds
        .iter()
        .find_map(|hold| stopped_by(config, head, &built, hold))
}

/// Whether a hold names `commit` of the input `name`, or one it holds, as
/// its copy reads it. Fetches nothing: a copy has a commit it built.
#[must_use]
pub fn keeps_off(config: &Config, name: &str, commit: &str) -> bool {
    let Some(follow) = config.follows.get(name) else {
        return false;
    };
    let repo = Repo::followed(config, name, follow);
    read(config).commits.iter().any(|hold| {
        git::has(&repo, &hold.commit) && git::descends(&repo, &hold.commit, commit).unwrap_or(true)
    })
}

/// Why `hold` stops main at `head`, its inputs at `built`, if it does.
fn stopped_by(config: &Config, head: &str, built: &Followed, hold: &Hold) -> Option<String> {
    let main = Repo::main(config);
    // A commit is judged in main and in every input whose copy has it, and
    // held if any of them builds it: anyone who can push to one repository,
    // signed or not, can put another's commit in it, so no one copy having
    // it settles whose it is. Copies are fetched only where that can't be
    // told without: every input's, when no copy has the commit, since a
    // stale one would read it as nobody's, and one that has it but not
    // what the build reads of it. So a repository out of reach fails a
    // hold only when the commit may be its own.
    let inputs: Vec<(Repo, &String)> = built
        .iter()
        .filter_map(|(name, commit)| {
            let follow = config.follows.get(name)?;
            Some((Repo::followed(config, name, follow), commit))
        })
        .collect();
    let main_has = git::has(&main, &hold.commit);
    let nobody_has = !main_has && inputs.iter().all(|(repo, _)| !git::has(repo, &hold.commit));
    let mut unknown = None;
    let mut unreached = None;
    let mut theirs = Vec::new();
    for (repo, commit) in &inputs {
        let has = git::has(repo, &hold.commit);
        if (nobody_has || (has && !git::has(repo, commit)))
            && let Err(failure) = git::fetch(repo)
        {
            if has {
                unknown.get_or_insert(failure.why);
            } else {
                unreached.get_or_insert(failure.why);
            }
            continue;
        }
        if git::has(repo, &hold.commit) {
            theirs.push((repo, *commit));
        }
    }
    // The commit may be an input's out of reach only while no copy has it.
    if theirs.is_empty() && !main_has {
        unknown = unknown.or(unreached);
    }
    let mut judged = |seen: Result<bool, Failure>| match seen {
        Ok(true) => true,
        Ok(false) => false,
        Err(failure) => {
            unknown = Some(failure.why);
            false
        }
    };
    let in_followed = theirs
        .iter()
        .any(|(repo, commit)| judged(git::descends(repo, &hold.commit, commit)));
    // A copy of main reads a commit it lacks as not in main, so it is
    // always asked. Main's host is asked too, since a hosted copy may lack
    // main's latest merges; it knows no other repository's commits, so
    // its failing for one an input's copy has is that input's, not untold.
    let in_main = match &config.hosted {
        Some(hosted) => match hosted::holds(config, hosted, &hold.commit, head) {
            Err(_) if !theirs.is_empty() => false,
            seen => judged(seen),
        },
        None => judged(git::descends(&main, &hold.commit, head)),
    };
    if in_main || in_followed {
        return Some(format!(
            "{} is held, and main at {} holds it; `update-to-main {LIFT} {}` lets it through",
            git::short(&hold.commit),
            git::short(head),
            hold.commit
        ));
    }
    unknown.map(|why| {
        format!(
            "{} is held, and whether main at {} holds it couldn't be told: {why}",
            git::short(&hold.commit),
            git::short(head)
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pause_holds_until_resumed_and_keeps_who_asked_first() {
        let mut held = Held::default();
        let said = changed(&mut held, "pause", 1000, 10).expect("pause is an ask");
        assert!(
            said.expect("a pause is kept")
                .starts_with("updates are paused"),
            "a pause didn't say updates are paused"
        );
        changed(&mut held, "pause", 1001, 20);
        assert_eq!(
            held.paused,
            Some(By { uid: 1000, at: 10 }),
            "a second pause took the first one's place"
        );
        changed(&mut held, "resume", 1001, 30);
        assert_eq!(held.paused, None, "resume left updates paused");
    }

    #[test]
    fn a_hold_names_one_commit_and_lifts_only_that_one() {
        let mut held = Held::default();
        let a = "21ad4a50a1b2c3d4e5f60718293a4b5c6d7e8f90";
        let b = "ea720d0500112233445566778899aabbccddeeff";
        changed(&mut held, &format!("hold {a}"), 1000, 1);
        changed(&mut held, &format!("hold {b}"), 1000, 2);
        changed(&mut held, &format!("hold {}", a.to_uppercase()), 1000, 3);
        let commits: Vec<&str> = held
            .commits
            .iter()
            .map(|hold| hold.commit.as_str())
            .collect();
        assert_eq!(commits, [a, b], "holds weren't one per commit: {commits:?}");
        changed(&mut held, &format!("lift {b}"), 1000, 4);
        let commits: Vec<&str> = held
            .commits
            .iter()
            .map(|hold| hold.commit.as_str())
            .collect();
        assert_eq!(commits, [a], "lifting one hold lifted another: {commits:?}");
        assert!(
            changed(&mut held, &format!("lift {b}"), 1000, 5).is_some_and(|said| said.is_err()),
            "lifting a commit that wasn't held said nothing of it"
        );
    }

    #[test]
    fn a_hold_without_a_commit_is_refused_and_other_asks_are_not_its() {
        let mut held = Held::default();
        for ask in ["hold", "hold main", "hold ea720d05", "lift"] {
            assert!(
                changed(&mut held, ask, 1000, 1).is_some_and(|said| said.is_err()),
                "{ask:?} wasn't refused"
            );
        }
        assert!(held.commits.is_empty(), "a refused hold held something");
        for ask in ["update", "follow", "stop"] {
            assert!(
                changed(&mut held, ask, 1000, 1).is_none(),
                "{ask:?} was taken as a hold"
            );
        }
    }

    #[test]
    fn a_held_file_that_cant_be_read_as_holds_pauses() {
        assert!(
            parsed(Some("{not json")).paused.is_some(),
            "a broken held file let updates run"
        );
        assert_eq!(parsed(None), Held::default(), "no held file held something");
        let mut held = Held::default();
        changed(
            &mut held,
            "hold ea720d0500112233445566778899aabbccddeeff",
            1000,
            1,
        );
        let text = serde_json::to_string(&held).expect("holds serialize");
        assert_eq!(
            parsed(Some(&text)),
            held,
            "what was kept didn't read back the same"
        );
    }
}
