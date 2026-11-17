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

/// The most minutes since mycelium's checkout last took main for it to
/// judge what waits: the hour fetches main itself before it reads, and when
/// that fails, a fetch of main within the bar means no merge it hasn't seen
/// can have waited past it, `AT_MOST_MIN`.
const FRESH_MIN: i64 = 60;

/// How many of the hour's own fetches of main it keeps, about a day's.
const FETCHES_KEPT: usize = 25;

/// `measure` as judged from a checkout that last fetched main at `fetched`,
/// at `now`: one fetched too long ago, or never, can't date what waits,
/// since its reflog dates the fetch, and it lists none it hasn't fetched.
fn dated(mut measure: Measure, fetched: Option<i64>, now: i64) -> Measure {
    let why = match fetched {
        None => "no fetch of its checkout's main could be dated".to_owned(),
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
/// entry at or after it on the first-parent line.
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

/// Whether `subject` is a merge train's, which commits with the date its
/// batch was cut, hours before main took it at worst.
fn on_the_train(subject: &str) -> bool {
    subject.ends_with("on the merge train")
}

/// The measure from `waiting`, mycelium main's first-parent merges that
/// core's lock doesn't carry, as `%H %ct %s` lines, and `seen`, main's
/// reflog, and `fetches`, when the hour itself took main, at `now`: the
/// longest any has waited since it reached main, with each one behind it,
/// oldest first. GitHub dates an ordinary merge as it makes it, so its
/// commit says when; a train's merge waits from the hour's last fetch
/// before the reflog's first entry holding it, which saw main without it,
/// or from that entry where no such fetch is known, or its commit where
/// none holds it, since its commit's date would age it from before it
/// existed there.
#[must_use]
pub fn unlocked(waiting: &str, seen: &str, fetches: &[i64], now: i64, owner: &str) -> Measure {
    let merges: Vec<(&str, i64, &str)> = waiting
        .lines()
        .filter_map(|line| {
            let (hash, rest) = line.split_once(' ')?;
            let (at, subject) = rest.split_once(' ')?;
            Some((hash, at.parse::<i64>().ok()?, subject))
        })
        .collect();
    let hashes: Vec<&str> = merges.iter().map(|(hash, _, _)| *hash).collect();
    // git log gives the newest first; the card names the oldest first.
    let waits: Vec<(i64, &str)> = merges
        .iter()
        .zip(arrived(&hashes, seen))
        // Main can't have taken a merge before it was made.
        .map(|(&(_, at, subject), came)| {
            let came = came.filter(|_| on_the_train(subject)).map_or(at, |came| {
                fetches
                    .iter()
                    .copied()
                    .filter(|fetch| *fetch < came)
                    .max()
                    .unwrap_or(came)
            });
            ((now - came.max(at)).max(0) / 60, subject)
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
/// can't be read, which is said. `took_at` holds when the hour last took
/// main itself, oldest first, and gains `now` when this one does.
pub fn read(
    core: &Path,
    checkouts: &[String],
    now: i64,
    owner: &str,
    took_at: &mut Vec<i64>,
) -> Option<Measure> {
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
    let remote = command(
        "git",
        &["-C", path, "rev-parse", "-q", "--verify", "origin/main"],
    )
    .is_ok();
    let main = if remote { "origin/main" } else { "main" };
    // Nothing else fetches the checkout on a clock, so the hour takes main
    // itself, leaving FETCH_HEAD to whoever works there. A link that
    // stalls gives up in half a minute rather than holding the card, and
    // git's own upkeep, which a fetch can start, is left to Joel's checkout.
    let took = remote
        && command(
            "git",
            &[
                "-C",
                path,
                "-c",
                "http.lowSpeedLimit=1000",
                "-c",
                "http.lowSpeedTime=30",
                "-c",
                "maintenance.auto=false",
                "-c",
                "gc.auto=0",
                "fetch",
                "--quiet",
                "--no-write-fetch-head",
                "origin",
                "+refs/heads/main:refs/remotes/origin/main",
            ],
        )
        .map_err(|error| eprintln!("cadence: couldn't fetch mycelium's main at {path}: {error}"))
        .is_ok();
    let mine = took_at.clone();
    if took {
        // Stamped once it's done, never before the reflog entry it wrote,
        // or the next hour would date a merge from the fetch that took it.
        took_at.push(crate::now_ms() / 1000);
        let cut = took_at.len().saturating_sub(FETCHES_KEPT);
        took_at.drain(..cut);
    }
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
            let measure = unlocked(&waiting, &seen, &mine, now, owner);
            // A main of its own, with no remote, is as fresh as it is.
            Some(if remote {
                dated(measure, if took { Some(now) } else { fetched(path) }, now)
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

/// Whether `fetch_head`, a `FETCH_HEAD`'s text, says its fetch took main:
/// any fetch writes one, of whichever branches it took.
fn took_main(fetch_head: &str) -> bool {
    fetch_head.lines().any(|line| {
        line.split('\t')
            .nth(2)
            .is_some_and(|took| took.starts_with("branch 'main' of "))
    })
}

/// When the checkout at `path` last fetched main, by its `FETCH_HEAD`'s
/// time where that fetch took main.
fn fetched(path: &str) -> Option<i64> {
    let head = command(
        "git",
        &["-C", path, "rev-parse", "--git-path", "FETCH_HEAD"],
    )
    .ok()?;
    let head = Path::new(path).join(head.trim());
    if !took_main(&std::fs::read_to_string(&head).ok()?) {
        return None;
    }
    let modified = std::fs::metadata(head).ok()?.modified().ok()?;
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
        let m = unlocked(&waiting, &seen, &[], NOW, "0FX11G");
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

    /// The hour is main's only fetcher on a clock, so a train's merge first
    /// shows in the reflog at the hour's own fetch, up to an hour after it
    /// arrived: it waits from the hour's last fetch that saw main without
    /// it, which reads at most one fetch long and never short.
    #[test]
    fn a_train_merge_waits_from_the_last_fetch_that_saw_main_without_it() {
        let waiting = format!("aaa {} Merge 1111 on the merge train\n", NOW - 300 * 60);
        let seen = format!("origin/main@{{{}}} aaa\n", NOW - 10 * 60);
        let fetches = [NOW - 130 * 60, NOW - 70 * 60, NOW - 10 * 60];
        let m = unlocked(&waiting, &seen, &fetches, NOW, "0FX11G");
        assert_eq!(
            m.now, 70.0,
            "a train merge was dated by the fetch that first held it: {m:?}"
        );
        let m = unlocked(&waiting, &seen, &[NOW - 10 * 60], NOW, "0FX11G");
        assert_eq!(
            m.now, 10.0,
            "with no fetch before it, the first holding entry didn't date it: {m:?}"
        );
        let late = format!("aaa {} Merge 1111 on the merge train\n", NOW - 40 * 60);
        let m = unlocked(&late, &seen, &fetches, NOW, "0FX11G");
        assert_eq!(
            m.now, 40.0,
            "a fetch before its commit dated the wait: {m:?}"
        );
    }

    /// A reflog entry no later than the commit, as an index selector would
    /// parse, can't date a merge before it existed; and the figure is the
    /// longest wait, which a merge dated by its commit can be however old.
    #[test]
    fn a_wait_starts_no_earlier_than_its_commit_and_the_longest_counts() {
        let waiting = format!(
            "bbb {} Merge 2222 on the merge train\naaa {} Merge 1111 on the merge train\n",
            NOW - 50 * 60,
            NOW - 60 * 60,
        );
        let seen = format!(
            "origin/main@{{{}}} aaa\norigin/main@{{0}} lll\n",
            NOW - 10 * 60
        );
        let m = unlocked(&waiting, &seen, &[], NOW, "0FX11G");
        assert_eq!(m.now, 50.0, "the longest wait wasn't the figure: {m:?}");
        let m = unlocked(
            "aaa 1794820000 Merge 1111 on the merge train\n",
            "main@{0} aaa\n",
            &[],
            NOW,
            "0FX11G",
        );
        assert_eq!(
            m.now, 58.0,
            "an entry before its commit dated the wait: {m:?}"
        );
    }

    /// GitHub dates an ordinary merge as it makes it, so a fetch that first
    /// held it an hour on would date it an hour short.
    #[test]
    fn an_ordinary_merge_waits_from_its_commit_however_late_it_was_fetched() {
        let waiting = format!(
            "aaa {} Merge pull request #208 from Jitsusama/jitsusama/shared-trees\n",
            NOW - 70 * 60
        );
        let seen = format!("origin/main@{{{}}} aaa\n", NOW - 15 * 60);
        // The hour fetched between its commit and the fetch that held it,
        // which would date a train's merge from 20 minutes.
        let m = unlocked(&waiting, &seen, &[NOW - 20 * 60], NOW, "0FX11G");
        assert_eq!(
            m.now, 70.0,
            "an ordinary merge was dated by the fetch that held it: {m:?}"
        );
    }

    /// Any fetch writes `FETCH_HEAD`, so only one that took main says main
    /// is fresh.
    #[test]
    fn only_a_fetch_that_took_main_dates_the_checkout() {
        let took = "9111802\t\tbranch 'main' of https://github.com/Jitsusama/mycelium\n";
        let other =
            "74b5cff\tnot-for-merge\tbranch 'trees' of https://github.com/Jitsusama/mycelium\n";
        assert!(took_main(took), "a fetch of main wasn't read as one");
        assert!(
            took_main(&format!(
                "{other}9111802\tnot-for-merge\tbranch 'main' of x\n"
            )),
            "a fetch of every branch wasn't read as taking main"
        );
        assert!(!took_main(other), "a fetch of another branch dated main");
    }

    /// A checkout that missed its fetch lists none of what it hasn't
    /// fetched and dates the rest late, so its waits go unjudged.
    #[test]
    fn a_checkout_that_missed_its_fetch_leaves_the_wait_unjudged() {
        let fresh = || unlocked("", "", &[], NOW, "0FX11G");
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
            never.target.is_none() && never.says.contains("no fetch of its checkout's main"),
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
        let m = unlocked(&waiting, "", &[], NOW, "0FX11G");
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
        let m = unlocked("", "", &[], NOW, "0FX11G");
        assert_eq!(m.now, 0.0, "an empty wait read as more than nought");
        assert!(m.behind.is_empty(), "nothing waits, so nothing is behind");
    }

    #[test]
    fn a_merge_subject_without_a_number_is_named_by_its_subject() {
        let waiting = format!("aaa {} Take the new lint\n", NOW - 5 * 60);
        assert_eq!(
            unlocked(&waiting, "", &[], NOW, "0FX11G").behind,
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
