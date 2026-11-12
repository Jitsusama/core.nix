//! Spend judged by what it bought. Joel's words of 11 November at 22:27
//! put productivity at quality before cost: "we need to walk the line
//! between cost efficiency and productivity". So the bill is read per
//! change that reached his laptop and stayed, beside how many reached it
//! and how many were undone, and a cheaper day that brought fewer or worse
//! changes reads as out rather than as a saving.
//!
//! A change is a pull request merged to core's main. It reached the laptop
//! when the first update that built a core holding it ended, and it stayed
//! unless a later merge reverted it. The bill is the one paid, from
//! spend-watch's settled days only, which run a day or two behind.
//!
//! The harness's changes are counted the same way and apart: a pull
//! request merged to its main reached him when pi-packages first moved
//! his install to a commit holding it. The bill stays per core change,
//! the figure its owner agreed, since the bill pays for both.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use serde_json::Value;

use super::readings::count;
use super::{Bar, Reading, shown};
use crate::records::{self, Look};
use crate::{DAY, command};

/// How many days back the changes and the bill are read: the newest
/// settled day and the seven before it, with a day to spare for the bill's
/// lag.
const DAYS: i64 = 10;

/// Everything these readings are read from, as text.
#[derive(Debug, Default)]
pub struct Sources {
    /// `git log --first-parent --format='%H %ct %s' main`, newest first.
    pub main: String,
    /// The bodies of main's commits that say "This reverts commit".
    pub reverts: String,
    /// Every update's look at main, from update-to-main's journal.
    pub looks: Vec<Look>,
    /// spend-watch's `collection.json`, as its daemon last wrote it.
    pub bills: String,
}

/// One day's changes that reached the laptop.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Reached {
    /// Those still in main now.
    pub kept: usize,
    /// Those a later merge reverted.
    pub reverted: usize,
}

impl Reached {
    /// The share of the day's changes that were undone, in percent.
    fn undone(self) -> Option<f64> {
        let all = self.kept + self.reverted;
        (all > 0).then(|| 100.0 * count(self.reverted) / count(all))
    }
}

/// Reads the sources from the machine: core's mirror as cadence's tick
/// last fetched it, update-to-main's journal, and spend-watch's file, so
/// reading costs no fetch and no report.
pub fn gather(folder: &Path, runtime: &Path, now: i64) -> Sources {
    let core = folder.join("core.git");
    let core = core.to_string_lossy();
    let since = format!("--since=@{}", now - DAYS * DAY);
    let git = |args: &[&str]| {
        let mut all = vec!["-C", &core];
        all.extend_from_slice(args);
        command("git", &all).unwrap_or_default()
    };
    Sources {
        main: git(&[
            "log",
            "--first-parent",
            &since,
            "--format=%H %ct %s",
            "main",
        ]),
        reverts: git(&[
            "log",
            &since,
            "--grep=This reverts commit",
            "--format=%b",
            "main",
        ]),
        looks: records::parse_looks(
            &command(
                "journalctl",
                &[
                    "-u",
                    "update-to-main.service",
                    &since,
                    "-o",
                    "short-unix",
                    "--no-pager",
                    "--grep=\"step\":\"(done|unchanged)\"",
                ],
            )
            .unwrap_or_default(),
        ),
        bills: fs::read_to_string(runtime.join("spend-watch/collection.json")).unwrap_or_default(),
    }
}

/// The harness's changes, read the way core's are: its mirror as
/// pi-packages last fetched it, and each move of his install from
/// pi-packages's journal, as a look that built the commit it moved to.
pub fn gather_harness(now: i64) -> Sources {
    let data = std::env::var_os("XDG_DATA_HOME").map_or_else(
        || {
            std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                .join(".local/share")
        },
        std::path::PathBuf::from,
    );
    let mirror = data.join("pi-packages/github.com/Jitsusama/agentic-harness.pi/mirror.git");
    let mirror = mirror.to_string_lossy();
    let since = format!("--since=@{}", now - DAYS * DAY);
    let git = |args: &[&str]| {
        let mut all = vec!["-C", &mirror];
        all.extend_from_slice(args);
        command("git", &all).unwrap_or_default()
    };
    Sources {
        main: git(&[
            "log",
            "--first-parent",
            &since,
            "--format=%H %ct %s",
            "main",
        ]),
        reverts: git(&[
            "log",
            &since,
            "--grep=This reverts commit",
            "--format=%b",
            "main",
        ]),
        looks: moves(
            &command(
                "journalctl",
                &[
                    "--user",
                    "-t",
                    "pi-packages",
                    &since,
                    "-o",
                    "short-unix",
                    "--no-pager",
                    "--grep=agentic-harness.pi: moved from",
                ],
            )
            .unwrap_or_default(),
        ),
        bills: String::new(),
    }
}

/// pi-packages's moves of the harness install, "<at> ... moved from <sha>
/// to <sha>", each as a look at the commit it moved to.
fn moves(journal: &str) -> Vec<Look> {
    journal
        .lines()
        .filter(|line| line.contains("agentic-harness.pi: moved from "))
        .filter_map(|line| {
            let at: f64 = line.split(' ').next()?.parse().ok()?;
            let core = line.rsplit(" to ").next()?.trim().to_owned();
            #[expect(clippy::cast_possible_truncation, reason = "a time in seconds")]
            let at = at as i64;
            Some(Look {
                at,
                core,
                switched: true,
            })
        })
        .collect()
}

/// Each day's changes that reached the laptop, by the local day the first
/// look that built them ended. A change merged before the first look is
/// left out, since it may have been on before the looks begin.
#[must_use]
pub fn reached(s: &Sources, tz: &TimeZone) -> BTreeMap<Date, Reached> {
    let mut merges: Vec<(&str, i64, &str)> = s
        .main
        .lines()
        .filter_map(|line| {
            let mut words = line.splitn(3, ' ');
            let sha = words.next()?;
            let at = words.next()?.parse().ok()?;
            Some((sha, at, words.next().unwrap_or_default()))
        })
        .collect();
    merges.reverse();
    let position: BTreeMap<&str, usize> = merges
        .iter()
        .enumerate()
        .map(|(i, (sha, _, _))| (*sha, i))
        .collect();
    let mut looks: Vec<&Look> = s.looks.iter().collect();
    looks.sort_by_key(|look| look.at);
    let Some(first) = looks.first().map(|look| look.at) else {
        return BTreeMap::new();
    };
    let (reverted, undoes) = reverts(s, &merges);
    let mut days: BTreeMap<Date, Reached> = BTreeMap::new();
    let mut next = 0;
    for look in looks {
        // A core's looks name its full sha, the harness's moves a short one.
        let Some(upto) = position.get(look.core.as_str()).copied().or_else(|| {
            (look.core.len() >= 7)
                .then(|| {
                    merges
                        .iter()
                        .position(|(sha, _, _)| sha.starts_with(&look.core))
                })
                .flatten()
        }) else {
            continue;
        };
        let Ok(day) = Timestamp::from_second(look.at).map(|t| t.to_zoned(tz.clone()).date()) else {
            continue;
        };
        for (i, (sha, at, _)) in merges.iter().enumerate().take(upto + 1).skip(next) {
            if *at < first || undoes.contains(&i) {
                continue;
            }
            let reached = days.entry(day).or_default();
            if reverted.contains(*sha) {
                reached.reverted += 1;
            } else {
                reached.kept += 1;
            }
        }
        next = next.max(upto + 1);
    }
    days
}

/// The merges a later one reverted, by sha, and the merges that are
/// reverts themselves, by position, which bring nothing new. merge-net
/// names the merge it reverts in its branch, `merge-net/revert-<sha12>`;
/// a revert by hand says "This reverts commit <sha>" in its commit, and
/// its branch, by convention, starts `revert-`.
fn reverts<'a>(
    s: &Sources,
    merges: &[(&'a str, i64, &str)],
) -> (BTreeSet<&'a str>, BTreeSet<usize>) {
    let named: BTreeSet<&str> = s
        .reverts
        .split("This reverts commit ")
        .skip(1)
        .filter_map(|rest| rest.get(..40))
        .collect();
    let mut prefixes = Vec::new();
    let mut undoes = BTreeSet::new();
    for (i, (_, _, subject)) in merges.iter().enumerate() {
        let branch = subject.rsplit(' ').next().unwrap_or_default();
        if let Some(prefix) = branch.split_once("merge-net/revert-").map(|(_, p)| p) {
            prefixes.push(prefix);
            undoes.insert(i);
        } else if branch
            .rsplit('/')
            .next()
            .is_some_and(|b| b.starts_with("revert-"))
        {
            undoes.insert(i);
        }
    }
    let reverted = merges
        .iter()
        .map(|(sha, _, _)| *sha)
        .filter(|sha| {
            named.contains(sha) || prefixes.iter().any(|p| p.len() >= 7 && sha.starts_with(p))
        })
        .collect();
    (reverted, undoes)
}

/// Each settled day's bill, from spend-watch's this week and last week by
/// day. A row it shows as "about" is estimated from pi's logs rather than
/// billed, and is left out. Its rows name a day without its year, which is
/// `today`'s, or the year before when that would be later than today.
#[must_use]
pub fn bills(collection: &str, today: Date) -> BTreeMap<Date, f64> {
    let Ok(all) = serde_json::from_str::<Value>(collection) else {
        return BTreeMap::new();
    };
    ["last-week/day", "this-week/day"]
        .iter()
        .filter_map(|report| all["reports"][*report]["rows"].as_array())
        .flatten()
        .filter_map(|row| {
            let side = row["side"].as_str()?.strip_prefix('$')?;
            let dollars: f64 = side.split(' ').next()?.replace(',', "").parse().ok()?;
            Some((day(row["name"].as_str()?, today)?, dollars))
        })
        .collect()
}

/// "Mon 9 Nov" as a date, in `today`'s year or the one before.
fn day(name: &str, today: Date) -> Option<Date> {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let mut words = name.split(' ').skip(1);
    let number: i8 = words.next()?.parse().ok()?;
    let named = words.next()?;
    let month = MONTHS.iter().position(|m| *m == named)?;
    let month = i8::try_from(month + 1).ok()?;
    let date = Date::new(today.year(), month, number).ok()?;
    if date > today {
        Date::new(today.year() - 1, month, number).ok()
    } else {
        Some(date)
    }
}

/// The readings: the bill per change that stayed on the newest settled
/// day, and the changes that stayed yesterday, each against the seven days
/// before; and the share of the last seven days' changes that were undone.
#[must_use]
pub fn readings(s: &Sources, now: i64, tz: &TimeZone) -> (Reading, Reading, Reading) {
    let today = Timestamp::from_second(now).map_or(Date::MIN, |t| t.to_zoned(tz.clone()).date());
    let days = reached(s, tz);
    let bills = bills(&s.bills, today);
    // The day the looks begin, perhaps late in it, is no whole day, so
    // counts start the day after.
    let from = s
        .looks
        .iter()
        .map(|look| look.at)
        .min()
        .and_then(|at| Timestamp::from_second(at).ok())
        .and_then(|t| t.to_zoned(tz.clone()).date().tomorrow().ok());
    let whole = |d: Date| from.is_some_and(|f| f <= d);
    let kept = |d: Date| days.get(&d).map_or(0, |r| r.kept);
    let each = |d: Date| {
        let kept = kept(d);
        bills
            .get(&d)
            .filter(|_| kept > 0 && whole(d))
            .map(|bill| bill / count(kept))
    };
    let week_before = |d: Date, of: &dyn Fn(Date) -> Option<f64>| {
        let days: Vec<f64> = (1..=7)
            .filter_map(|back| d.checked_sub(jiff::Span::new().days(back)).ok())
            .filter_map(of)
            .collect();
        (days.len() >= 3).then(|| days.iter().sum::<f64>() / count(days.len()))
    };
    let settled = bills
        .keys()
        .rev()
        .copied()
        .find(|d| *d < today && each(*d).is_some());
    let mut bill = Reading::new(
        "the bill per core change that reached you and stayed {}",
        settled.and_then(each),
        "$",
        "QCXZEB",
    )
    .from("spend-watch's settled bill, core's main and update-to-main's journal");
    if let Some(d) = settled {
        let before = week_before(d, &each);
        bill = bill.detail(Some(format!(
            "{}: {} for {} changes{}; the bill pays for all the AI work, core's and the rest",
            d.strftime("%a %-d %b"),
            shown(bills[&d], "$"),
            kept(d),
            before.map_or_else(String::new, |b| format!(
                ", the week before {} each",
                shown(b, "$")
            ))
        )));
        if let Some(before) = before {
            bill = bill.bar(Bar::AtMost(before));
        }
    }
    let changes = stayed(
        s,
        &days,
        today,
        tz,
        ("{} core changes reached you and stayed yesterday", "80ECO2"),
        "core's main and update-to-main's journal",
    );
    let week = (0..7)
        .filter_map(|back| today.checked_sub(jiff::Span::new().days(back)).ok())
        .filter_map(|d| days.get(&d).copied())
        .fold(Reached::default(), |all, r| Reached {
            kept: all.kept + r.kept,
            reverted: all.reverted + r.reverted,
        });
    let undone = Reading::new(
        "{} of the week's core changes undone after reaching you",
        week.undone(),
        "%",
        "D5QEKR",
    )
    .bar(Bar::AtMost(2.0))
    .detail(Some(format!(
        "{} of {}",
        week.reverted,
        week.kept + week.reverted
    )))
    .from("core's main, merge-net's reverts and those by hand");
    (bill, changes, undone)
}

/// The harness's changes that reached him and stayed yesterday, against
/// the seven days before.
#[must_use]
pub fn harness(s: &Sources, now: i64, tz: &TimeZone) -> Reading {
    let today = Timestamp::from_second(now).map_or(Date::MIN, |t| t.to_zoned(tz.clone()).date());
    stayed(
        s,
        &reached(s, tz),
        today,
        tz,
        (
            "{} harness changes reached you and stayed yesterday",
            "85PKB2",
        ),
        "the harness's main and pi-packages's journal",
    )
}

/// The changes that reached him and stayed yesterday, held to at least the
/// mean of the seven days before. The day the looks begin, perhaps late in
/// it, is no whole day, so counting starts the day after.
fn stayed(
    s: &Sources,
    days: &BTreeMap<Date, Reached>,
    today: Date,
    tz: &TimeZone,
    (says, owner): (&'static str, &'static str),
    from: &'static str,
) -> Reading {
    let first = s
        .looks
        .iter()
        .map(|look| look.at)
        .min()
        .and_then(|at| Timestamp::from_second(at).ok())
        .and_then(|t| t.to_zoned(tz.clone()).date().tomorrow().ok());
    let whole = |d: Date| first.is_some_and(|f| f <= d);
    let kept = |d: Date| count(days.get(&d).map_or(0, |r| r.kept));
    let yesterday = today.yesterday().ok().filter(|d| whole(*d));
    let mut changes = Reading::new(says, yesterday.map(kept), "", owner).from(from);
    let before = yesterday.and_then(|d| {
        let week: Vec<f64> = (1..=7)
            .filter_map(|back| d.checked_sub(jiff::Span::new().days(back)).ok())
            .filter(|day| whole(*day))
            .map(kept)
            .collect();
        (week.len() >= 3).then(|| week.iter().sum::<f64>() / count(week.len()))
    });
    if let Some(before) = before {
        changes = changes.bar(Bar::AtLeast(before));
    }
    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tz() -> TimeZone {
        TimeZone::fixed(jiff::tz::offset(-5))
    }

    /// Noon on a November 2026 day, local.
    fn noon(day: i8) -> i64 {
        Date::new(2026, 11, day)
            .unwrap()
            .to_zoned(tz())
            .unwrap()
            .timestamp()
            .as_second()
            + 12 * 3600
    }

    /// A sha of one hex digit forty times, so no two share a prefix.
    fn sha(n: u8) -> String {
        format!("{n:x}").repeat(40)
    }

    fn look(at: i64, core: u8) -> Look {
        Look {
            at,
            core: sha(core),
            switched: true,
        }
    }

    /// Merges numbered from 1, oldest first, each an hour apart from the
    /// morning of 9 November, as main's log gives them, newest first.
    fn main(subjects: &[&str]) -> String {
        let mut lines: Vec<String> = subjects
            .iter()
            .zip(1u8..)
            .map(|(s, n)| {
                format!(
                    "{} {} {s}",
                    sha(n),
                    noon(9) - 6 * 3600 + i64::from(n) * 3600
                )
            })
            .collect();
        lines.reverse();
        lines.join("\n")
    }

    const PR: &str = "Merge pull request #1 from Jitsusama/jitsusama/a-thing";

    #[test]
    fn a_change_reaches_on_the_day_of_the_first_look_that_built_it() {
        let s = Sources {
            main: main(&[PR, PR, PR]),
            looks: vec![look(noon(9) - 5 * 3600, 1), look(noon(10), 3)],
            ..Sources::default()
        };
        let days = reached(&s, &tz());
        let day = |d| Date::new(2026, 11, d).unwrap();
        assert_eq!(
            days[&day(10)].kept,
            2,
            "the second and third reach on the 10th"
        );
        assert_eq!(
            days.get(&day(9)).map(|r| r.kept),
            Some(1),
            "the first reaches on the 9th"
        );
    }

    #[test]
    fn a_change_merged_before_the_first_look_is_left_out() {
        let s = Sources {
            main: main(&[PR, PR]),
            looks: vec![look(noon(10), 2)],
            ..Sources::default()
        };
        let days = reached(&s, &tz());
        assert_eq!(
            days.values().map(|r| r.kept).sum::<usize>(),
            0,
            "both merged before any look, so either may have been on already"
        );
    }

    #[test]
    fn reverts_count_against_the_change_and_are_no_change_themselves() {
        let net = format!(
            "Merge pull request #3 from Jitsusama/merge-net/revert-{}",
            &sha(1)[..12]
        );
        let by_hand = "Merge pull request #4 from Jitsusama/jitsusama/revert-the-thing";
        let s = Sources {
            main: main(&[
                PR,
                PR,
                &net,
                by_hand,
                "Merge pull request #5 from J/jitsusama/net-reverts-again",
            ]),
            reverts: format!("Revert it\n\nThis reverts commit {}, reversing\n", sha(2)),
            looks: vec![look(noon(9) - 6 * 3600, 0), look(noon(10), 5)],
            ..Sources::default()
        };
        let days = reached(&s, &tz());
        let r = days[&Date::new(2026, 11, 10).unwrap()];
        assert_eq!(
            r,
            Reached {
                kept: 1,
                reverted: 2
            },
            "two reverted, two reverts, one kept"
        );
    }

    const COLLECTION: &str = r#"{"reports": {
        "last-week/day": {"rows": [{"name": "Sun 8 Nov", "side": "$6,606 · 22%"}]},
        "this-week/day": {"rows": [
            {"name": "Mon 9 Nov", "side": "$6,416 · 34%"},
            {"name": "Tue 10 Nov", "side": "about $6,254 · 33%"}
        ]}
    }}"#;

    #[test]
    fn only_settled_days_are_billed() {
        let bills = bills(COLLECTION, Date::new(2026, 11, 11).unwrap());
        let got: Vec<(String, f64)> = bills.iter().map(|(d, b)| (d.to_string(), *b)).collect();
        assert_eq!(
            got,
            vec![
                ("2026-11-08".to_owned(), 6606.0),
                ("2026-11-09".to_owned(), 6416.0)
            ]
        );
    }

    #[test]
    fn a_day_named_after_today_is_last_year() {
        let jan = Date::new(2027, 1, 2).unwrap();
        assert_eq!(day("Thu 31 Dec", jan), Date::new(2026, 12, 31).ok());
    }

    #[test]
    fn the_bill_is_read_per_change_that_stayed() {
        let s = Sources {
            main: main(&[PR; 6]),
            looks: vec![look(noon(8), 0), look(noon(9), 4), look(noon(10), 6)],
            bills: r#"{"reports": {"this-week/day": {"rows": [
                {"name": "Mon 9 Nov", "side": "$400 · 50%"},
                {"name": "Tue 10 Nov", "side": "$400 · 50%"}
            ]}}}"#
                .to_owned(),
            ..Sources::default()
        };
        let (bill, changes, undone) = readings(&s, noon(11), &tz());
        assert_eq!(
            bill.now,
            Some(200.0),
            "$400 for the two changes of the 10th"
        );
        assert_eq!(changes.now, Some(2.0), "two reached yesterday");
        assert_eq!(undone.now, Some(0.0), "none undone");
    }

    #[test]
    fn a_move_of_the_install_is_a_look_at_the_commit_it_moved_to() {
        let journal = "1794478400.5 here pi-packages[7]: github.com/Jitsusama/agentic-harness.pi: \
            moved from 6f867474 to 32301441\n\
            1794478401.0 here pi-packages[7]: github.com/Jitsusama/neovim.pi: at d5b46091\n";
        assert_eq!(
            moves(journal),
            vec![Look {
                at: 1_794_478_400,
                core: "32301441".to_owned(),
                switched: true
            }]
        );
    }

    #[test]
    fn a_short_sha_finds_the_merge_it_names() {
        let s = Sources {
            main: main(&[PR; 3]),
            looks: vec![
                look(noon(8), 0),
                Look {
                    at: noon(10),
                    core: sha(3)[..8].to_owned(),
                    switched: true,
                },
            ],
            ..Sources::default()
        };
        assert_eq!(
            harness(&s, noon(11), &tz()).now,
            Some(3.0),
            "all three reached on the 10th through the short sha"
        );
    }
}
