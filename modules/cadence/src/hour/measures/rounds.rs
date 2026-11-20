//! How many council rounds each change merged in the last day took, read
//! from the harness's own record of every round run on each change rather
//! than from the count a landing note types, which is free text and which
//! no note before 15:20 on 19 November carried. WCJQSG ruled then that past
//! round 5 a round raises only what would mislead Joel or break something,
//! and J7XHQN asked for the measure. At 18:48 WCJQSG set its target, a
//! median of at most 3, and asked that each merge past 5 be named with
//! what its rounds after the fifth caught, and filed as drag to the merge's
//! owner when they caught nothing. What a round caught is read from the
//! harness's record of the fixes committed for its findings.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::{Class, Measure, count, median, seconds, short};

/// The measure's key, from one hour to the next.
pub const KEY: &str = "rounds-per-merge";

/// The round past which WCJQSG's ruling narrows what a round may raise.
pub const PAST: usize = 5;

/// The median rounds per merge WCJQSG set as the target.
pub const TARGET: f64 = 3.0;

/// The key of a merge's own line when its rounds past five fixed
/// nothing, before the change it names.
pub const FRUITLESS: &str = "rounds-past-five-fixed-nothing:";

/// How long before its merge a round still counts toward it. A branch's
/// name can be cut again for a later change, so a round older than this
/// is taken as another change's.
pub const BEFORE: i64 = 3 * super::DAY;

/// One repository's main: its key as the review record names it, such as
/// `github:Jitsusama/core.nix`, and `git log --first-parent --format='%H
/// %ct %s'` of it.
pub struct Main<'a> {
    pub repo: &'a str,
    pub log: &'a str,
}

/// A change merged to a main: its repository, number and branch, and when.
struct Merged<'a> {
    repo: &'a str,
    number: &'a str,
    branch: &'a str,
    at: i64,
}

/// The changes `main` merged in the day before `now`, each from its merge
/// commit's subject, "Merge pull request #N from Owner/branch". A revert,
/// GitHub's `revert-N-...` or the net's `merge-net/revert-...`, undoes a
/// change rather than making one, so no round is owed it.
fn merged<'a>(main: &Main<'a>, now: i64) -> Vec<Merged<'a>> {
    main.log
        .lines()
        .filter_map(|line| {
            let mut words = line.splitn(3, ' ');
            let (_, at, subject) = (words.next()?, words.next()?, words.next()?);
            let at = at.parse::<i64>().ok()?;
            let (number, from) = subject
                .strip_prefix("Merge pull request #")?
                .split_once(" from ")?;
            let (_, branch) = from.split_once('/')?;
            Some(Merged {
                repo: main.repo,
                number,
                branch,
                at,
            })
        })
        .filter(|m| m.at > now - super::DAY && m.at <= now)
        .filter(|m| !m.branch.starts_with("revert-") && !m.branch.contains("/revert-"))
        .collect()
}

/// What one record of the harness's review runs is about, and its
/// councils, each by its id and when it started.
struct Reviewed {
    repo: String,
    /// A hosted change's number, or a local range's head, a branch or a
    /// commit.
    named: String,
    hosted: bool,
    /// The last part of the checkout the range was read in, which names
    /// the branch when the tree was cut for it.
    tree: String,
    councils: Vec<(String, i64)>,
}

impl Reviewed {
    fn read(text: &str) -> Option<Self> {
        let record: Value = serde_json::from_str(text).ok()?;
        let change = &record["change"];
        let id = change["id"].as_str()?;
        let hosted = change["provider"].as_str() == Some("github");
        let named = if hosted {
            id
        } else {
            id.rsplit_once("..").map_or(id, |(_, head)| head)
        };
        let tree = change["repo"]["localPath"]
            .as_str()
            .and_then(|path| path.rsplit('/').next())
            .unwrap_or_default();
        let councils = record["runs"]
            .as_array()?
            .iter()
            .filter_map(|run| {
                let id = run["id"].as_str()?;
                // A judge or a critique reads a council's findings; only a
                // council is a round of review.
                if !id.starts_with("council-") {
                    return None;
                }
                Some((id.to_owned(), seconds(&run["startedAt"])?))
            })
            .collect();
        Some(Self {
            repo: change["repo"]["key"].as_str()?.to_owned(),
            named: named.to_owned(),
            hosted,
            tree: tree.to_owned(),
            councils,
        })
    }

    /// Whether this record is about `merged`: its number when hosted, else
    /// its branch, or the tree the work tool cuts for that branch, whose
    /// name is the repository's key and the branch with each separator a
    /// hyphen. A range read at a commit in a checkout that names no branch
    /// can't be told apart, so it counts for none.
    fn about(&self, merged: &Merged) -> bool {
        if self.repo != merged.repo {
            return false;
        }
        if self.hosted {
            return self.named == merged.number;
        }
        let tree = format!(
            "worktree-{}-{}",
            merged.repo.replace([':', '/'], "-"),
            merged.branch.replace('/', "-")
        );
        self.named == merged.branch || self.tree == tree
    }
}

/// What each council's findings led to, by the council's id: the subject
/// of each finding whose fix was committed, from `fixes`, each the text of
/// one file of the harness's review fixes.
fn caught(fixes: &[String]) -> BTreeMap<String, Vec<String>> {
    let mut caught: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for text in fixes {
        let Ok(record) = serde_json::from_str::<Value>(text) else {
            continue;
        };
        for fix in record["fixes"].as_array().into_iter().flatten() {
            let finding = &fix["subject"]["finding"];
            let (Some(run), Some(subject)) = (
                finding["origin"]["runId"].as_str(),
                finding["subject"].as_str(),
            ) else {
                continue;
            };
            if fix["outcome"]["kind"].as_str() == Some("committed") {
                caught
                    .entry(run.to_owned())
                    .or_default()
                    .push(subject.to_owned());
            }
        }
    }
    caught
}

/// The median council rounds per change merged to `mains` in the day
/// before `now`, from `records`, each the text of one file of the
/// harness's review runs, then a line of its own for each merge past five
/// whose rounds after the fifth led to no fix in `fixes`, owned by whoever
/// `landed` it, else `owner`. A council counts toward a merge when it
/// started in the `BEFORE` up to it, and once however many records hold
/// it. A change with a record but no council counts nought; one with no
/// record at all may be one the matching can't see, so it is counted
/// apart rather than read as nought.
#[must_use]
pub fn rounds(
    mains: &[Main],
    records: &[String],
    fixes: &[String],
    landed: &BTreeMap<String, String>,
    now: i64,
    owner: &str,
) -> Vec<Measure> {
    let reviewed: Vec<Reviewed> = records.iter().filter_map(|r| Reviewed::read(r)).collect();
    let caught = caught(fixes);
    let mut unrecorded = 0;
    let mut each: Vec<(usize, String, Vec<String>)> = mains
        .iter()
        .flat_map(|main| merged(main, now))
        .filter_map(|m| {
            let about: Vec<&Reviewed> = reviewed.iter().filter(|r| r.about(&m)).collect();
            if about.is_empty() {
                unrecorded += 1;
                return None;
            }
            let councils: BTreeSet<(i64, &str)> = about
                .iter()
                .flat_map(|r| &r.councils)
                .filter(|(_, at)| *at <= m.at && *at >= m.at - BEFORE)
                .map(|(id, at)| (*at, id.as_str()))
                .collect();
            let after_fifth: Vec<String> = councils
                .iter()
                .skip(PAST)
                .flat_map(|(_, id)| caught.get(*id).into_iter().flatten().cloned())
                .collect();
            let label = m.repo.rsplit('/').next().unwrap_or(m.repo);
            Some((councils.len(), format!("{label}#{}", m.number), after_fifth))
        })
        .collect();
    let mut figures: Vec<f64> = each.iter().map(|(n, _, _)| count(*n)).collect();
    figures.sort_by(f64::total_cmp);
    each.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    each.retain(|(n, _, _)| *n > PAST);
    let read = if reviewed.is_empty() {
        ", no review record read".to_owned()
    } else {
        format!(" with a review record, {unrecorded} with none")
    };
    let median = Measure {
        key: KEY.to_owned(),
        class: Class::Ship,
        says: format!(
            "median council rounds per change merged in the last day, of {}{read}",
            figures.len()
        ),
        now: median(&figures),
        target: Some(TARGET),
        owner: owner.to_owned(),
        behind: each
            .iter()
            .map(|(n, change, fixed)| match fixed.as_slice() {
                [] => format!("{change}: {n} rounds, those past five fixed nothing"),
                [one] => format!("{change}: {n} rounds, past five fixed {}", short(one)),
                [first, rest @ ..] => format!(
                    "{change}: {n} rounds, past five fixed {} and {} more",
                    short(first),
                    rest.len()
                ),
            })
            .collect(),
    };
    let fruitless = each
        .into_iter()
        .filter(|(_, _, fixed)| fixed.is_empty())
        .map(|(n, change, _)| Measure {
            key: format!("{FRUITLESS}{change}"),
            class: Class::Ship,
            says: format!("council rounds on {change}, whose rounds past five fixed nothing"),
            now: count(n),
            target: Some(count(PAST)),
            owner: landed.get(&change).map_or(owner, String::as_str).to_owned(),
            behind: Vec::new(),
        });
    std::iter::once(median).chain(fruitless).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The moment the tests read at.
    const NOW: i64 = 1_800_000_000;

    const CORE: &str = "github:Jitsusama/core.nix";

    /// A round started `ago` seconds before `NOW`, as the record keeps it.
    fn started(id: &str, ago: i64) -> String {
        let at = jiff::Timestamp::from_second(NOW - ago).expect("a moment the tests read");
        format!(r#"{{"id": "{id}", "round": "council", "startedAt": "{at}"}}"#)
    }

    /// One record of `runs` on a change, as the harness writes it.
    fn record(provider: &str, repo: &str, id: &str, tree: &str, runs: &[String]) -> String {
        format!(
            r#"{{"change": {{"provider": "{provider}", "id": "{id}",
                "repo": {{"key": "{repo}", "localPath": "/trees/{tree}"}}}},
              "runs": [{}]}}"#,
            runs.join(",")
        )
    }

    #[test]
    fn each_merge_counts_the_councils_on_its_number_branch_or_tree() {
        let log = format!(
            "{}\n{}\n{}\n{}\n{}\n",
            format_args!(
                "aaaa {} Merge pull request #11 from Jitsusama/here/eleven",
                NOW - 60
            ),
            format_args!(
                "bbbb {} Merge pull request #12 from Jitsusama/here/twelve",
                NOW - 120
            ),
            format_args!(
                "cccc {} Merge pull request #13 from Jitsusama/here/thirteen",
                NOW - 180
            ),
            format_args!(
                "dddd {} Merge pull request #14 from Jitsusama/here/fourteen",
                NOW - 240
            ),
            // Merged more than a day ago, so not this measure's.
            format_args!(
                "eeee {} Merge pull request #10 from Jitsusama/here/ten",
                NOW - 90_000
            ),
        ) + &format!(
            "ffff {} Merge pull request #15 from Jitsusama/revert-11-here/eleven\n\
             9999 {} Merge pull request #16 from Jitsusama/merge-net/revert-aaaa\n",
            NOW - 30,
            NOW - 20
        );
        let six: Vec<String> = (0..6)
            .map(|n| started(&format!("council-six-{n}"), 600))
            .collect();
        let records = vec![
            // By its number: two councils and a judge, which isn't a round.
            record(
                "github",
                CORE,
                "11",
                "elsewhere",
                &[
                    started("council-a", 900),
                    started("council-b", 600),
                    started("judge-a", 500),
                ],
            ),
            // By its branch, the same council again in another record,
            // once, and one only this record holds.
            record(
                "local-range",
                CORE,
                "origin/main..here/eleven",
                "elsewhere",
                &[started("council-b", 600), started("council-c", 300)],
            ),
            // By its tree, at a commit: six rounds, past the ruling's five.
            record(
                "local-range",
                CORE,
                "origin/main..0123456789abcdef0123456789abcdef01234567",
                "worktree-github-Jitsusama-core.nix-here-twelve",
                &six,
            ),
            // A round after its merge, one long before, and another
            // repository's change by the same number count for none.
            record(
                "github",
                CORE,
                "13",
                "elsewhere",
                &[
                    started("council-late", 60),
                    started("council-old", BEFORE + 600),
                ],
            ),
            record(
                "github",
                "github:Jitsusama/yonder",
                "14",
                "elsewhere",
                &[started("council-yonder", 600)],
            ),
        ];
        let all = rounds(
            &[Main {
                repo: CORE,
                log: &log,
            }],
            &records,
            &[],
            &BTreeMap::new(),
            NOW,
            "ORCH",
        );
        let m = &all[0];
        assert_eq!(m.key, KEY);
        assert_eq!(m.target, Some(3.0), "WCJQSG's target");
        // #14's only record is another repository's, so it has none; the
        // two reverts are no change of their own.
        assert_eq!(
            m.says,
            "median council rounds per change merged in the last day, of 3 with a review \
             record, 1 with none"
        );
        // 0, 3 and 6.
        assert_eq!(m.now, 3.0, "rounds of #11 to #13");
        assert_eq!(
            m.behind,
            ["core.nix#12: 6 rounds, those past five fixed nothing"]
        );
        assert_eq!(all.len(), 2, "#12's rounds past five fixed nothing");
    }

    /// The harness's record of the fixes committed for findings, each by
    /// the council that raised it, and one answered rather than fixed.
    fn fixes(committed: &[(&str, &str)]) -> String {
        let fixes: Vec<serde_json::Value> = committed
            .iter()
            .map(|(run, subject)| {
                serde_json::json!({"subject": {"finding": {"subject": subject,
                    "origin": {"runId": run}}}, "outcome": {"kind": "committed"}})
            })
            .chain(std::iter::once(serde_json::json!({"subject": {"finding": {
                "subject": "answered", "origin": {"runId": "council-b-6"}}},
                "outcome": {"kind": "answered"}})))
            .collect();
        serde_json::json!({ "fixes": fixes }).to_string()
    }

    #[test]
    fn a_merge_past_five_says_what_its_later_rounds_fixed_or_goes_to_its_owner() {
        let log = format!(
            "aaaa {} Merge pull request #21 from Jitsusama/here/a\n\
             bbbb {} Merge pull request #22 from Jitsusama/here/b\n\
             cccc {} Merge pull request #23 from Jitsusama/here/c\n",
            NOW - 60,
            NOW - 120,
            NOW - 180
        );
        // Seven rounds each, a minute apart, the sixth and seventh past five.
        let seven = |change: &str| -> Vec<String> {
            (0..7)
                .map(|n| started(&format!("council-{change}-{n}"), 3600 - 60 * n))
                .collect()
        };
        let records: Vec<String> = ["21", "22", "23"]
            .iter()
            .zip(["a", "b", "c"])
            .map(|(number, change)| record("github", CORE, number, "elsewhere", &seven(change)))
            .collect();
        // #21's fifth round fixed one, which isn't past five; #23's seventh
        // fixed two; #22's sixth was only answered.
        let fixed = [fixes(&[
            ("council-a-4", "in the fifth"),
            ("council-c-6", "a test that never failed"),
            ("council-c-6", "a second"),
        ])];
        let landed = BTreeMap::from([("core.nix#22".to_owned(), "B8RBNW".to_owned())]);
        let all = rounds(
            &[Main {
                repo: CORE,
                log: &log,
            }],
            &records,
            &fixed,
            &landed,
            NOW,
            "ORCH",
        );
        assert_eq!(
            all[0].behind,
            [
                "core.nix#21: 7 rounds, those past five fixed nothing",
                "core.nix#22: 7 rounds, those past five fixed nothing",
                "core.nix#23: 7 rounds, past five fixed a test that never failed and 1 more",
            ]
        );
        let fruitless: Vec<(&str, &str, f64)> = all[1..]
            .iter()
            .map(|m| (m.key.as_str(), m.owner.as_str(), m.now))
            .collect();
        assert_eq!(
            fruitless,
            [
                ("rounds-past-five-fixed-nothing:core.nix#21", "ORCH", 7.0),
                ("rounds-past-five-fixed-nothing:core.nix#22", "B8RBNW", 7.0),
            ],
            "a merge whose rounds past five fixed nothing goes to who landed it"
        );
        assert!(
            all[1..].iter().all(|m| m.past().is_some()),
            "each is past its target of five, so it's filed"
        );
    }

    #[test]
    fn with_no_review_record_read_the_line_says_so() {
        let log = format!(
            "aaaa {} Merge pull request #11 from Jitsusama/here\n",
            NOW - 60
        );
        let all = rounds(
            &[Main {
                repo: CORE,
                log: &log,
            }],
            &[],
            &[],
            &BTreeMap::new(),
            NOW,
            "ORCH",
        );
        let m = &all[0];
        assert!(
            m.says.ends_with("of 0, no review record read"),
            "{}",
            m.says
        );
    }
}
