//! How long mycelium's merges wait outside core's lock: none reaches a
//! machine until core's `flake.lock` carries it, and from there the card's
//! merge to on follows it. On 15 November mycelium #196 sat hours outside
//! the lock and three of the Mac's cells went unattached, while every
//! figure the hour read counted only core, the harness and the machine's
//! repository.

use std::fmt::Write as _;
use std::path::Path;

use serde_json::Value;

use super::{Class, Measure, count};
use crate::command;

/// The most minutes a mycelium merge may wait for core's lock: a bump's
/// changed-checks and its VM tests take some 30 minutes, and merge to on's
/// own 18 follow it.
const AT_MOST_MIN: f64 = 60.0;

/// The mycelium revision core's `flake.lock` carries.
fn locked(lock: &str) -> Option<String> {
    let lock: Value = serde_json::from_str(lock).ok()?;
    let input = lock.pointer("/nodes/root/inputs/mycelium")?.as_str()?;
    lock.pointer(&format!("/nodes/{input}/locked/rev"))?
        .as_str()
        .map(str::to_owned)
}

/// A merge as the card names it: `mycelium #211` from GitHub's subject,
/// else its subject, for a commit made on main itself.
fn named(subject: &str) -> String {
    subject
        .strip_prefix("Merge pull request #")
        .and_then(|rest| rest.split_whitespace().next())
        .filter(|number| number.bytes().all(|b| b.is_ascii_digit()))
        .map_or_else(
            || format!("mycelium's \"{subject}\""),
            |number| format!("mycelium #{number}"),
        )
}

/// The most minutes since mycelium's checkout last fetched for its reflog
/// to date what waits: code-health fetches it each hour on the hour and the
/// card reads it ten minutes on, so past this a fetch was missed.
const FRESH_MIN: i64 = 90;

/// `measure` as judged from a checkout that last fetched main at `fetched`,
/// at `now`: one fetched too long ago, or never, can't date what waits,
/// since its reflog dates the fetch, and it lists none it hasn't fetched.
fn dated(mut measure: Measure, fetched: Option<i64>, now: i64) -> Measure {
    let why = match fetched {
        None => "its checkout never fetched main".to_owned(),
        Some(at) if (now - at) / 60 > FRESH_MIN => {
            format!("its checkout last fetched {} min ago", (now - at) / 60)
        }
        Some(_) => return measure,
    };
    measure.target = None;
    let _ = write!(measure.says, " (unjudged: {why})");
    measure
}

/// When each of `merges`, newest first, first reached main, from `seen`,
/// main's reflog as `%gd %H` lines read with `--date=unix`: the oldest
/// entry at or after it on the first-parent line. A merge train commits
/// with the date its batch was cut, which can be hours before main took
/// it, so the commit's own date would age it from before it existed there.
fn arrived(merges: &[&str], seen: &str) -> Vec<Option<i64>> {
    let mut came: Vec<Option<i64>> = vec![None; merges.len()];
    for line in seen.lines() {
        let Some((at, hash)) = line
            .rsplit_once("@{")
            .and_then(|(_, rest)| rest.split_once("} "))
        else {
            continue;
        };
        let (Ok(at), Some(place)) = (
            at.parse::<i64>(),
            merges.iter().position(|merge| *merge == hash.trim()),
        ) else {
            continue;
        };
        // An entry at a merge holds every merge older than it as well.
        for held in &mut came[place..] {
            *held = Some(held.map_or(at, |was| was.min(at)));
        }
    }
    came
}

/// The measure from `waiting`, mycelium main's first-parent merges that
/// core's lock doesn't carry, as `%H %ct %s` lines, and `seen`, main's
/// reflog, at `now`: how long the oldest has waited since it reached
/// main, with each one behind it, oldest first. A merge no reflog entry
/// holds is aged from its commit's date.
#[must_use]
pub fn unlocked(waiting: &str, seen: &str, now: i64, owner: &str) -> Measure {
    let merges: Vec<(&str, i64, &str)> = waiting
        .lines()
        .filter_map(|line| {
            let (hash, rest) = line.split_once(' ')?;
            let (at, subject) = rest.split_once(' ')?;
            Some((hash, at.parse::<i64>().ok()?, subject))
        })
        .collect();
    let hashes: Vec<&str> = merges.iter().map(|(hash, _, _)| *hash).collect();
    // git log gives the newest first; the card names the longest wait first.
    let waits: Vec<(i64, &str)> = merges
        .iter()
        .zip(arrived(&hashes, seen))
        // Main can't have taken a merge before it was made.
        .map(|(&(_, at, subject), came)| {
            (
                (now - came.map_or(at, |came| came.max(at))).max(0) / 60,
                subject,
            )
        })
        .rev()
        .collect();
    let longest = waits.iter().map(|(minutes, _)| *minutes).max().unwrap_or(0);
    Measure {
        key: "mycelium-to-lock".to_owned(),
        class: Class::Ship,
        says: "minutes the longest-waiting mycelium merge has waited outside core's lock"
            .to_owned(),
        now: count(usize::try_from(longest).unwrap_or(0)),
        target: Some(AT_MOST_MIN),
        owner: owner.to_owned(),
        behind: waits
            .iter()
            .map(|(minutes, subject)| format!("{} waiting {minutes} min", named(subject)))
            .collect(),
    }
}

/// The measure, read from core's mirror at `core` and the `mycelium=PATH`
/// among `checkouts`, from its `origin/main` where it has one, else its
/// `main`; none when the machine names no mycelium checkout, or either
/// can't be read, which is said.
pub fn read(core: &Path, checkouts: &[String], now: i64, owner: &str) -> Option<Measure> {
    let path = checkouts
        .iter()
        .find_map(|named| named.strip_prefix("mycelium="))?;
    let lock = command(
        "git",
        &["-C", &core.to_string_lossy(), "show", "main:flake.lock"],
    )
    .ok()?;
    let Some(rev) = locked(&lock) else {
        eprintln!("cadence: core's flake.lock names no mycelium revision");
        return None;
    };
    let main = if command(
        "git",
        &["-C", path, "rev-parse", "-q", "--verify", "origin/main"],
    )
    .is_ok()
    {
        "origin/main"
    } else {
        "main"
    };
    match command(
        "git",
        &[
            "-C",
            path,
            "log",
            "--first-parent",
            "--format=%H %ct %s",
            &format!("{rev}..{main}"),
        ],
    ) {
        // A checkout with no reflog ages every merge from its commit.
        Ok(waiting) => {
            let seen = match command(
                "git",
                &[
                    "-C",
                    path,
                    "reflog",
                    "show",
                    "--date=unix",
                    "--format=%gd %H",
                    main,
                ],
            ) {
                Ok(seen) => seen,
                Err(error) => {
                    eprintln!(
                        "cadence: couldn't read the reflog of mycelium's {main} at {path}, \
                         so its waits are dated by their commits: {error}"
                    );
                    String::new()
                }
            };
            let measure = unlocked(&waiting, &seen, now, owner);
            // A main of its own, with no remote, is as fresh as it is.
            Some(if main == "origin/main" {
                dated(measure, fetched(path), now)
            } else {
                measure
            })
        }
        Err(error) => {
            eprintln!("cadence: couldn't read mycelium's {main} at {path}: {error}");
            None
        }
    }
}

/// When the checkout at `path` last fetched, by its `FETCH_HEAD`'s time.
fn fetched(path: &str) -> Option<i64> {
    let head = command(
        "git",
        &["-C", path, "rev-parse", "--git-path", "FETCH_HEAD"],
    )
    .ok()?;
    let modified = std::fs::metadata(Path::new(path).join(head.trim()))
        .ok()?
        .modified()
        .ok()?;
    let since = modified.duration_since(std::time::UNIX_EPOCH).ok()?;
    i64::try_from(since.as_secs()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_794_823_500;

    /// On 17 November train 7's merges carried 18:21 the day before as
    /// their commit date and reached main at 08:06, so the 08:10 card read
    /// 828 minutes where they had waited about 6.
    #[test]
    fn a_merge_waits_from_when_it_reached_main_not_from_its_commit() {
        let waiting = format!(
            "ccc {old} Merge 894834b18e83 on the merge train\n\
             bbb {old} Merge 90697d10fe98 on the merge train\n\
             aaa {old} Merge 6eec7c185f99 on the merge train\n",
            old = NOW - 830 * 60,
        );
        let seen = format!(
            "origin/main@{{{}}} ccc\norigin/main@{{{}}} aaa\norigin/main@{{{}}} lll\n",
            NOW - 6 * 60,
            NOW - 100 * 60,
            NOW - 900 * 60,
        );
        let m = unlocked(&waiting, &seen, NOW, "0FX11G");
        assert_eq!(
            m.behind,
            [
                "mycelium's \"Merge 6eec7c185f99 on the merge train\" waiting 100 min",
                "mycelium's \"Merge 90697d10fe98 on the merge train\" waiting 6 min",
                "mycelium's \"Merge 894834b18e83 on the merge train\" waiting 6 min",
            ],
            "a merge was aged from its commit, not from the fetch that first held it"
        );
        assert_eq!(m.now, 100.0, "the oldest reached main 100 minutes ago");
    }

    /// A reflog entry no later than the commit, as an index selector would
    /// parse, can't date a merge before it existed; and the figure is the
    /// longest wait, which a merge dated by its commit can be however old.
    #[test]
    fn a_wait_starts_no_earlier_than_its_commit_and_the_longest_counts() {
        let waiting = format!(
            "bbb {} Take the new lint\naaa {} Take the old lint\n",
            NOW - 50 * 60,
            NOW - 60 * 60,
        );
        let seen = format!(
            "origin/main@{{{}}} aaa\norigin/main@{{0}} lll\n",
            NOW - 10 * 60
        );
        let m = unlocked(&waiting, &seen, NOW, "0FX11G");
        assert_eq!(m.now, 50.0, "the longest wait wasn't the figure: {m:?}");
        let m = unlocked("aaa 1794820000 Take it\n", "main@{0} aaa\n", NOW, "0FX11G");
        assert_eq!(
            m.now, 58.0,
            "an entry before its commit dated the wait: {m:?}"
        );
    }

    /// A checkout that missed its fetch lists none of what it hasn't
    /// fetched and dates the rest late, so its waits go unjudged.
    #[test]
    fn a_checkout_that_missed_its_fetch_leaves_the_wait_unjudged() {
        let fresh = || unlocked("", "", NOW, "0FX11G");
        assert!(
            dated(fresh(), Some(NOW - 10 * 60), NOW).target.is_some(),
            "a checkout fetched ten minutes ago went unjudged"
        );
        let stale = dated(fresh(), Some(NOW - 100 * 60), NOW);
        assert!(
            stale.target.is_none() && stale.says.contains("last fetched 100 min ago"),
            "a checkout fetched 100 minutes ago was judged: {stale:?}"
        );
        let never = dated(fresh(), None, NOW);
        assert!(
            never.target.is_none() && never.says.contains("never fetched"),
            "a checkout never fetched was judged: {never:?}"
        );
    }

    #[test]
    fn the_oldest_merge_outside_the_lock_is_how_long_it_waits() {
        let waiting = format!(
            "bbb {} Merge pull request #211 from Jitsusama/jitsusama/run-anywhere\n\
             aaa {} Merge pull request #208 from Jitsusama/jitsusama/shared-trees\n",
            NOW - 10 * 60,
            NOW - 170 * 60,
        );
        let m = unlocked(&waiting, "", NOW, "0FX11G");
        assert_eq!(m.now, 170.0, "the oldest waited 170 minutes");
        assert_eq!(
            m.behind,
            [
                "mycelium #208 waiting 170 min",
                "mycelium #211 waiting 10 min"
            ],
            "each merge behind it, oldest first"
        );
        assert!(m.past().is_some(), "170 minutes is past a bar of 60");
    }

    #[test]
    fn nothing_outside_the_lock_waits_nought() {
        let m = unlocked("", "", NOW, "0FX11G");
        assert_eq!(m.now, 0.0, "an empty wait read as more than nought");
        assert!(m.behind.is_empty(), "nothing waits, so nothing is behind");
    }

    #[test]
    fn a_merge_subject_without_a_number_is_named_by_its_subject() {
        let waiting = format!("aaa {} Take the new lint\n", NOW - 5 * 60);
        assert_eq!(
            unlocked(&waiting, "", NOW, "0FX11G").behind,
            ["mycelium's \"Take the new lint\" waiting 5 min"],
            "a direct commit on main went unnamed"
        );
    }

    #[test]
    fn the_lock_says_which_mycelium_it_carries() {
        let lock = r#"{"nodes":{"mycelium":{"locked":{"rev":"dc1a1444ed7a"}},
            "root":{"inputs":{"mycelium":"mycelium"}}}}"#;
        assert_eq!(
            locked(lock).as_deref(),
            Some("dc1a1444ed7a"),
            "the lock's mycelium revision went unread"
        );
        assert_eq!(
            locked(r#"{"nodes":{}}"#),
            None,
            "a lock without mycelium named a revision"
        );
    }
}
