//! The line a fetch of a hosted main leaves in the log, since a machine's
//! log otherwise gives no time and a fetch's size is only known once it's
//! done: where it fetched, when it began, how long it took, what main
//! moved from and to, and how much the checkout's objects grew.

use crate::git;
use crate::hosted::is_commit;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// What git is asked for a ref's commit, the ref's name after: each ref
/// the name matches, which may be ones below it, with its name.
pub const REF_READ: [&str; 2] = ["for-each-ref", "--format=%(objectname) %(refname)"];

/// What git is asked for the size of the checkout's objects.
pub const COUNT: [&str; 2] = ["count-objects", "-v"];

/// The commit `REF_READ` said for the ref `name` alone: none when git
/// failed, and empty when no ref has that name.
pub fn ref_named<'a>(said: Option<&'a str>, name: &str) -> Option<&'a str> {
    let said = said?;
    Some(
        said.lines()
            .find_map(|line| line.split_once(' ').filter(|(_, at)| *at == name))
            .map_or("", |(commit, _)| commit),
    )
}

/// The line a fetch leaves in the log, whether it `worked` or not: where,
/// when it began, how long it took over how many tries, main's move from and to, and how much
/// the objects grew while it ran, by any writer, in KiB as `git
/// count-objects -v` says them, left out when that isn't known.
pub fn line(
    checkout: &Path,
    worked: bool,
    began: SystemTime,
    (seconds, tries): (u64, u32),
    (from, to): (Option<&str>, Option<&str>),
    stored: Option<(u64, u64)>,
) -> String {
    let grew = stored.map_or_else(String::new, |(before, after)| {
        let (kib, way) = if after < before {
            (before - after, "smaller")
        } else {
            (after - before, "larger")
        };
        if kib < 1024 {
            format!(", its objects {kib} KiB {way}")
        } else {
            format!(", its objects {} MiB {way}", kib.div_ceil(1024))
        }
    });
    format!(
        "update-to-main: {} main into {}, starting at {} and taking {seconds} s over \
         {tries} {}, from {} to {}{grew}",
        if worked { "fetched" } else { "failed to fetch" },
        checkout.display(),
        utc(began),
        if tries == 1 { "try" } else { "tries" },
        seen(from),
        seen(to),
    )
}

/// The KiB `git count-objects -v` says are stored, its loose objects' size
/// and its packs' added.
pub fn kib_stored(counted: &str) -> Option<u64> {
    let field = |name: &str| {
        counted
            .lines()
            .find_map(|line| line.strip_prefix(name)?.trim().parse::<u64>().ok())
    };
    field("size:")?.checked_add(field("size-pack:")?)
}

/// A commit as the log says it: shortened, `nothing` where git said no
/// ref is there, and `unknown` where git failed or said something other
/// than a commit, as the account's git may.
fn seen(commit: Option<&str>) -> String {
    match commit {
        Some("") => "nothing".to_owned(),
        Some(commit) if is_commit(commit) => git::short(commit).to_owned(),
        _ => "unknown".to_owned(),
    }
}

/// `at` as a UTC time, `2026-11-17T20:20:05Z`, from the civil calendar's
/// days since 1970 (Howard Hinnant's `civil_from_days`). A clock before
/// 1970 reads as its first second.
fn utc(at: SystemTime) -> String {
    let seconds = at
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let (days, of_day) = (seconds / 86_400, seconds % 86_400);
    let shifted = days + 719_468;
    let era = shifted / 146_097;
    let of_era = shifted % 146_097;
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * of_year + 2) / 153;
    let day = of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 {
        month_from_march + 3
    } else {
        month_from_march - 9
    };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        of_day / 3600,
        of_day % 3600 / 60,
        of_day % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn a_time_is_said_in_utc() {
        let at = |seconds| UNIX_EPOCH + Duration::from_secs(seconds);
        assert_eq!(utc(at(0)), "1970-01-01T00:00:00Z", "the epoch");
        assert_eq!(
            utc(at(1_794_946_805)),
            "2026-11-17T20:20:05Z",
            "an evening in November"
        );
        assert_eq!(
            utc(at(951_782_400)),
            "2000-02-29T00:00:00Z",
            "a leap day of a century's leap year"
        );
        assert_eq!(
            utc(at(4_107_542_400)),
            "2100-03-01T00:00:00Z",
            "a century's common year, with no leap day after February's 28th"
        );
    }

    #[test]
    fn the_stored_size_adds_loose_and_packed() {
        let counted = "count: 4\nsize: 16\nin-pack: 900\npacks: 3\nsize-pack: 2048\n\
                       prune-packable: 0\ngarbage: 0\nsize-garbage: 0\n";
        assert_eq!(kib_stored(counted), Some(2064), "16 loose and 2048 packed");
        assert_eq!(
            kib_stored("count: 4\n"),
            None,
            "a count without sizes says none"
        );
        assert_eq!(
            kib_stored(&format!("size: {}\nsize-pack: 1\n", u64::MAX)),
            None,
            "sizes too large to add say none"
        );
    }

    /// The reads the line is drawn from, as the fetch asks them, run
    /// against real git: a fresh checkout's count has both sizes, its
    /// main's ref reads as no ref rather than a failure, even with a ref
    /// below that name, and as its commit once it's there.
    #[test]
    fn the_lines_reads_are_what_git_says() {
        let checkout = tempfile::tempdir().unwrap();
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git")
                .args(args)
                .current_dir(checkout.path())
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_AUTHOR_NAME", "a")
                .env("GIT_AUTHOR_EMAIL", "a@example.com")
                .env("GIT_COMMITTER_NAME", "a")
                .env("GIT_COMMITTER_EMAIL", "a@example.com")
                .output()
                .unwrap();
            assert!(out.status.success(), "git {args:?} failed: {out:?}");
            String::from_utf8(out.stdout).unwrap().trim().to_owned()
        };
        let main = "refs/update-to-main/main";
        let read = || git(&[REF_READ[0], REF_READ[1], main]);
        git(&["init", "--quiet"]);
        assert_eq!(
            kib_stored(&git(&COUNT)),
            Some(0),
            "a fresh checkout stores nothing"
        );
        git(&["commit", "--quiet", "--allow-empty", "-m", "one"]);
        let commit = git(&["rev-parse", "HEAD"]);
        git(&["update-ref", &format!("{main}/beside"), &commit]);
        let below = read();
        assert_eq!(
            seen(ref_named(Some(&below), main)),
            "nothing",
            "a ref below main's name isn't main's: {below:?}"
        );
        git(&["update-ref", "-d", &format!("{main}/beside")]);
        git(&["update-ref", main, &commit]);
        assert_eq!(
            ref_named(Some(&read()), main),
            Some(commit.as_str()),
            "main's ref should read as its commit"
        );
        assert_eq!(ref_named(None, main), None, "a failed read says none");
    }

    #[test]
    fn a_fetch_says_where_when_what_moved_how_long_and_how_much() {
        let began = UNIX_EPOCH + Duration::from_secs(1_794_946_805);
        let (a, b) = ("a".repeat(40), "b".repeat(40));
        let (a, b) = (a.as_str(), b.as_str());
        let (short_a, short_b) = (git::short(a), git::short(b));
        let said =
            |worked, moved, stored| line(Path::new("/c"), worked, began, (42, 1), moved, stored);
        let fetched = "update-to-main: fetched main into /c, \
                       starting at 2026-11-17T20:20:05Z and taking 42 s over 1 try";
        assert_eq!(
            said(true, (Some(a), Some(b)), Some((1024, 1024 * 19 + 1))),
            format!("{fetched}, from {short_a} to {short_b}, its objects 19 MiB larger"),
            "a fetch should say the move, the time and the growth, rounded up"
        );
        assert_eq!(
            line(
                Path::new("/c"),
                false,
                began,
                (42, 3),
                (Some(a), Some(a)),
                Some((2048, 2048))
            ),
            format!(
                "update-to-main: failed to fetch main into /c, \
                 starting at 2026-11-17T20:20:05Z and taking 42 s over 3 tries, \
                 from {short_a} to {short_a}, its objects 0 KiB larger"
            ),
            "a failed fetch should say so, and what it left"
        );
        assert_eq!(
            said(true, (Some(""), Some(b)), Some((0, 300))),
            format!("{fetched}, from nothing to {short_b}, its objects 300 KiB larger"),
            "a first fetch should say main came from nothing, and a small growth in KiB"
        );
        assert_eq!(
            said(true, (None, Some("main\n")), Some((4096, 1024))),
            format!("{fetched}, from unknown to unknown, its objects 3 MiB smaller"),
            "a failed read or one that isn't a commit should read as unknown, and a shrink as one"
        );
        assert_eq!(
            said(true, (Some(a), Some(b)), Some((0, 1023))),
            format!("{fetched}, from {short_a} to {short_b}, its objects 1023 KiB larger"),
            "under a MiB should be said in KiB"
        );
        assert_eq!(
            said(true, (Some(a), Some(b)), Some((0, 1024))),
            format!("{fetched}, from {short_a} to {short_b}, its objects 1 MiB larger"),
            "a MiB should be said as one"
        );
        assert_eq!(
            said(true, (Some(a), Some(b)), None),
            format!("{fetched}, from {short_a} to {short_b}"),
            "an unknown size should be left out"
        );
    }
}
