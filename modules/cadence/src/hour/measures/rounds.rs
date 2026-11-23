//! How many council rounds each change merged in the last day took, read
//! from the harness's own record of every round run on each change rather
//! than from the count a landing note types, which is free text and which
//! no note before 15:20 on 19 November carried. WCJQSG ruled then that past
//! round 5 a round raises only what would mislead Joel or break something,
//! and J7XHQN asked for the measure. At 18:48 WCJQSG set its target, a
//! median of at most 3, and asked that each merge past 5 be named with
//! what its rounds after the fifth caught, and filed as drag to the merge's
//! owner when they caught nothing. What a round caught is read from the
//! harness's record of the fixes for its findings, committed or queued, a
//! judge's finding counting for the council it consolidated. A queued fix
//! counts when what the change does changed after its round read it, as
//! many owners decide a finding is real and fix it without recording the
//! commit, which left real catches on #1860's and #5614's rounds past
//! five read as nothing. A queued fix on a change that didn't can't have
//! been made before the merge: it is a follow-up, which
//! WCJQSG's ruling makes of what a late round needn't raise, so it catches
//! nothing and is named apart.
//!
//! A round counts once it was collected and one of its reviewers answered,
//! as merge-net counts one, so a round a reload killed before anyone
//! answered, which stays open, isn't a round. Rounds run on another
//! machine count from its mirror, but their fixes are kept there, so a
//! late one's catch can't be read here and its merge isn't filed.
//!
//! Long because its main test needs six merges' records, one for each way
//! a late round's fix can be read, and splitting it would hide how they
//! differ.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::{Class, Measure, count, median, seconds, short};

/// The measure's key, from one hour to the next.
pub const KEY: &str = "rounds-per-merge";

/// The round past which WCJQSG's ruling narrows what a round may raise.
pub const PAST: usize = 5;

/// The median rounds per merge WCJQSG set as the target.
pub const TARGET: f64 = 3.0;

/// The key of a merge's own line when its rounds past five caught
/// nothing, before the change it names.
pub const FRUITLESS: &str = "rounds-past-five-fixed-nothing:";

/// What the median says when it read no record of rounds, or none of
/// fixes, so it can't tell which merges' rounds past five caught nothing.
const NO_RUNS: &str = "no review record read";
const NO_FIXES: &str = "no fixes record read";

/// Whether `median`, the rounds measure, read the records that say which
/// merges' rounds past five caught nothing.
#[must_use]
pub fn read_whole(median: &Measure) -> bool {
    median.key == KEY && !median.says.contains(NO_RUNS) && !median.says.contains(NO_FIXES)
}

/// How long before its merge a round still counts toward it. A branch's
/// name can be cut again for a later change, so a round older than this
/// is taken as another change's.
pub const BEFORE: i64 = 3 * super::DAY;

/// One repository's main: its key as the review record names it, such as
/// `github:Jitsusama/core.nix`, `git log --first-parent --format='%H
/// %ct %s'` of it, and whether the change a merge commit merged gained
/// commits of its own after a round read the head it names, `None` where
/// git can't say.
pub struct Main<'a> {
    pub repo: &'a str,
    pub log: &'a str,
    pub moved: &'a dyn Fn(&str, &str) -> Option<bool>,
}

/// A change merged to a main: its repository, number and branch, when,
/// and its merge commit, with how to read whether it moved after a round.
struct Merged<'a> {
    repo: &'a str,
    number: &'a str,
    branch: &'a str,
    at: i64,
    merge: &'a str,
    moved: &'a dyn Fn(&str, &str) -> Option<bool>,
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
            let (merge, at, subject) = (words.next()?, words.next()?, words.next()?);
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
                merge,
                moved: main.moved,
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
    /// Each judge's or critique's id with the round it read, its basis.
    bases: Vec<(String, String)>,
    /// Each round's id with the head it read, its witness.
    witnesses: Vec<(String, String)>,
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
                if !id.starts_with("council-") || !answered(run) {
                    return None;
                }
                Some((id.to_owned(), seconds(&run["startedAt"])?))
            })
            .collect();
        let field = |name: &str| -> Option<Vec<(String, String)>> {
            Some(
                record["runs"]
                    .as_array()?
                    .iter()
                    .filter_map(|run| {
                        Some((
                            run["id"].as_str()?.to_owned(),
                            run[name].as_str()?.to_owned(),
                        ))
                    })
                    .collect(),
            )
        };
        let bases = field("basis")?;
        let witnesses = field("witness")?;
        Some(Self {
            repo: change["repo"]["key"].as_str()?.to_owned(),
            named: named.to_owned(),
            hosted,
            tree: tree.to_owned(),
            councils,
            bases,
            witnesses,
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

/// Whether `run` was collected, neither open nor closed, and one of its
/// reviewers answered, as merge-net reads a round before counting it.
fn answered(run: &Value) -> bool {
    let flag = |name: &str| run[name].as_bool().unwrap_or(false);
    !flag("open")
        && !flag("closed")
        && run["outcomes"]
            .as_array()
            .is_some_and(|outcomes| outcomes.iter().any(|o| o.get("failure").is_none()))
}

/// The councils `there`, another machine's records, holds that `here`
/// doesn't: rounds run elsewhere, whose fixes are kept elsewhere too.
fn away<'a>(here: &[Reviewed], there: &'a [Reviewed]) -> BTreeSet<&'a str> {
    let held: BTreeSet<&str> = here
        .iter()
        .flat_map(|r| &r.councils)
        .map(|(id, _)| id.as_str())
        .collect();
    there
        .iter()
        .flat_map(|r| &r.councils)
        .map(|(id, _)| id.as_str())
        .filter(|id| !held.contains(id))
        .collect()
}

/// A finding whose fix was committed, or is still queued.
struct Fix {
    subject: String,
    queued: bool,
    /// The commit a committed fix names, where it names one.
    commit: Option<String>,
}

/// What each council's findings led to, by the council's id: each finding
/// whose fix was committed or is still queued, from `fixes`, each the text
/// of one file of the harness's review fixes. A fix answered or skipped
/// instead, its owner finding nothing to change, caught nothing. A finding
/// a judge consolidated names the judge's run, so it is taken back through
/// `bases`, each run's basis, to the council it came from.
fn caught(fixes: &[String], bases: &BTreeMap<&str, &str>) -> BTreeMap<String, Vec<Fix>> {
    let mut caught: BTreeMap<String, Vec<Fix>> = BTreeMap::new();
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
            // A critique reads a judge, which reads a council; a few steps
            // reach it, and a loop in the record ends there.
            let mut run = run;
            for _ in 0..3 {
                match bases.get(run) {
                    Some(earlier) => run = earlier,
                    None => break,
                }
            }
            // Only a fix with no outcome yet is queued. One whose outcome
            // can't be read caught nothing, so a change in the record's
            // shape files drag rather than hiding it.
            let queued = fix.get("outcome").is_none_or(Value::is_null);
            if queued || fix["outcome"]["kind"].as_str() == Some("committed") {
                caught.entry(run.to_owned()).or_default().push(Fix {
                    subject: subject.to_owned(),
                    queued,
                    commit: fix["outcome"]["commit"].as_str().map(str::to_owned),
                });
            }
        }
    }
    caught
}

/// What one merge's rounds past five led to: what they caught, how many
/// fixes they queued as follow-ups, and how many queued fixes couldn't be
/// told either way.
struct Late {
    rounds: usize,
    change: String,
    caught: Vec<String>,
    follow_ups: usize,
    /// Queued fixes whose rounds' heads couldn't be read, so they are
    /// neither caught nor follow-ups.
    unread: usize,
    /// Rounds past five run on another machine, whose fixes are kept
    /// there, so what they caught can't be read here.
    away: usize,
}

impl Late {
    /// Splits what `councils` past the fifth raised on `merged`, from
    /// `fixes` by council, each round's head read from `witnesses`. A
    /// committed fix was caught. A queued one was caught only when what the
    /// change does changed after its round read it, so its fix could be in
    /// what merged; a rebase, or a merge from main, alone doesn't count.
    /// One on a change that didn't is a follow-up, so the drag of a late
    /// round that queued only those shows. One whose round read no head,
    /// or whose commits git can't read, is unread, as unread fixes are, so
    /// it files nothing. A later round's commit changes the change as a fix
    /// made for this round's finding would, so the two aren't told apart,
    /// as WCJQSG was told on 20 November. A fix the round committed itself
    /// changes it too, so a queued one is caught only if the change moved
    /// after each of the round's own commits as well, and is a follow-up if
    /// it didn't after one; a commit not named leaves it unread. Git is asked
    /// once a round and commit, and only for a round with a queued fix. A
    /// round with no fix here that ran on another machine, as `away` holds,
    /// is counted away, since its fixes are kept there.
    fn split(
        merged: &Merged,
        councils: &BTreeSet<(i64, &str)>,
        fixes: &BTreeMap<String, Vec<Fix>>,
        witnesses: &BTreeMap<&str, &str>,
        away: &BTreeSet<&str>,
    ) -> (Vec<String>, usize, usize, usize) {
        let (mut caught, mut follow_ups, mut unread, mut elsewhere) = (Vec::new(), 0, 0, 0);
        for (_, id) in councils.iter().skip(PAST) {
            let Some(each) = fixes.get(*id) else {
                elsewhere += usize::from(away.contains(id));
                continue;
            };
            let moved = each
                .iter()
                .any(|fix| fix.queued)
                .then(|| {
                    let reads = std::iter::once(witnesses.get(id).copied()).chain(
                        each.iter()
                            .filter(|fix| !fix.queued)
                            .map(|fix| fix.commit.as_deref()),
                    );
                    let said: Vec<Option<bool>> = reads
                        .map(|read| read.and_then(|read| (merged.moved)(merged.merge, read)))
                        .collect();
                    if said.contains(&Some(false)) {
                        Some(false)
                    } else {
                        said.iter().all(|s| *s == Some(true)).then_some(true)
                    }
                })
                .flatten();
            for fix in each {
                match (fix.queued, moved) {
                    (false, _) | (true, Some(true)) => caught.push(fix.subject.clone()),
                    (true, Some(false)) => follow_ups += 1,
                    (true, None) => unread += 1,
                }
            }
        }
        (caught, follow_ups, unread, elsewhere)
    }

    /// The merge's line under the median.
    fn line(&self, fixes_read: bool) -> String {
        let Self {
            rounds: n,
            change,
            caught,
            follow_ups,
            unread,
            away,
        } = self;
        let queued = match follow_ups {
            0 => String::new(),
            1 => ", 1 queued as a follow-up".to_owned(),
            many => format!(", {many} queued as follow-ups"),
        } + &match unread {
            0 => String::new(),
            1 => ", 1 queued fix unread".to_owned(),
            many => format!(", {many} queued fixes unread"),
        } + &match away {
            0 => String::new(),
            1 => ", 1 round's fixes kept on another machine".to_owned(),
            many => format!(", {many} rounds' fixes kept on another machine"),
        };
        // What a round elsewhere caught is kept there, so nothing caught is
        // known only of the rounds run here.
        let here = if *away > 0 { " here" } else { "" };
        match caught.as_slice() {
            _ if !fixes_read => format!("{change}: {n} rounds, what they caught unread"),
            [] => format!("{change}: {n} rounds, those past five caught nothing{here}{queued}"),
            [one] => format!(
                "{change}: {n} rounds, past five caught {}{queued}",
                short(one)
            ),
            [first, rest @ ..] => format!(
                "{change}: {n} rounds, past five caught {} and {} more{queued}",
                short(first),
                rest.len()
            ),
        }
    }
}

/// The median council rounds per change merged to `mains` in the day
/// before `now`, from `records`, each the text of one file of the
/// harness's review runs, and `mirrored`, each one of another machine's
/// kept here by the harness, then a line of its own for each merge past five
/// whose rounds after the fifth caught nothing in `fixes`, owned by whoever
/// `landed` it, else `owner`; `landed` is asked only when there is one.
/// With no fixes record read, what the rounds caught can't be told, so
/// the line says so and none is filed. A fix queued as a follow-up, as
/// `Late::split` tells, is named but caught nothing; one it can't tell is
/// named unread, and its merge isn't filed. A council counts toward a merge
/// when it started in the `BEFORE` up to it, and once by its id however
/// many records hold it. A change with a record but no council counts
/// nought; one with no record at all may be one the matching can't see,
/// so it is counted apart rather than read as nought.
#[must_use]
pub fn rounds(
    mains: &[Main],
    records: &[String],
    mirrored: &[String],
    fixes: &[String],
    landed: impl FnOnce() -> BTreeMap<String, String>,
    now: i64,
    owner: &str,
) -> Vec<Measure> {
    let here: Vec<Reviewed> = records.iter().filter_map(|r| Reviewed::read(r)).collect();
    let there: Vec<Reviewed> = mirrored.iter().filter_map(|r| Reviewed::read(r)).collect();
    let away = away(&here, &there);
    let reviewed: Vec<&Reviewed> = here.iter().chain(&there).collect();
    let bases: BTreeMap<&str, &str> = reviewed
        .iter()
        .flat_map(|r| &r.bases)
        .map(|(id, basis)| (id.as_str(), basis.as_str()))
        .collect();
    let witnesses: BTreeMap<&str, &str> = reviewed
        .iter()
        .flat_map(|r| &r.witnesses)
        .map(|(id, head)| (id.as_str(), head.as_str()))
        .collect();
    let caught = caught(fixes, &bases);
    let fixes_read = !fixes.is_empty();
    let mut unrecorded = 0;
    let mut each: Vec<Late> = mains
        .iter()
        .flat_map(|main| merged(main, now))
        .filter_map(|m| {
            let about: Vec<&Reviewed> = reviewed.iter().copied().filter(|r| r.about(&m)).collect();
            if about.is_empty() {
                unrecorded += 1;
                return None;
            }
            let by_id: BTreeMap<&str, i64> = about
                .iter()
                .flat_map(|r| &r.councils)
                .map(|(id, at)| (id.as_str(), *at))
                .collect();
            let councils: BTreeSet<(i64, &str)> = by_id
                .into_iter()
                .filter(|(_, at)| *at <= m.at && *at >= m.at - BEFORE)
                .map(|(id, at)| (at, id))
                .collect();
            let (after_fifth, follow_ups, unread, away) =
                Late::split(&m, &councils, &caught, &witnesses, &away);
            let label = m.repo.rsplit('/').next().unwrap_or(m.repo);
            Some(Late {
                rounds: councils.len(),
                change: format!("{label}#{}", m.number),
                caught: after_fifth,
                follow_ups,
                unread,
                away,
            })
        })
        .collect();
    let mut figures: Vec<f64> = each.iter().map(|late| count(late.rounds)).collect();
    figures.sort_by(f64::total_cmp);
    each.sort_by(|a, b| {
        b.rounds
            .cmp(&a.rounds)
            .then_with(|| a.change.cmp(&b.change))
    });
    each.retain(|late| late.rounds > PAST);
    let read = if reviewed.is_empty() {
        format!(", {NO_RUNS}")
    } else if !fixes_read {
        format!(" with a review record, {unrecorded} with none, {NO_FIXES}")
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
        behind: each.iter().map(|late| late.line(fixes_read)).collect(),
    };
    each.retain(|late| fixes_read && late.caught.is_empty() && late.unread == 0 && late.away == 0);
    let landed = if each.is_empty() {
        BTreeMap::new()
    } else {
        landed()
    };
    let fruitless = each.into_iter().map(
        |Late {
             rounds: n, change, ..
         }| Measure {
            key: format!("{FRUITLESS}{change}"),
            class: Class::Ship,
            says: format!("council rounds on {change}, whose rounds past five caught nothing"),
            now: count(n),
            target: Some(count(PAST)),
            owner: landed.get(&change).map_or(owner, String::as_str).to_owned(),
            behind: Vec::new(),
        },
    );
    std::iter::once(median).chain(fruitless).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The moment the tests read at.
    const NOW: i64 = 1_800_000_000;

    const CORE: &str = "github:Jitsusama/core.nix";

    /// A round started `ago` seconds before `NOW`, collected with one
    /// reviewer's answer, as the record keeps it.
    fn started(id: &str, ago: i64) -> String {
        let at = jiff::Timestamp::from_second(NOW - ago).expect("a moment the tests read");
        format!(
            r#"{{"id": "{id}", "round": "council", "startedAt": "{at}",
                "outcomes": [{{"participantId": "one"}}]}}"#
        )
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
            // once though this record has it start a second apart, and one
            // only this record holds.
            record(
                "local-range",
                CORE,
                "origin/main..here/eleven",
                "elsewhere",
                &[started("council-b", 601), started("council-c", 300)],
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
                moved: &|_, _| None,
            }],
            &records,
            &[],
            &[],
            || panic!("asked who landed a merge though none is filed"),
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
             record, 1 with none, no fixes record read"
        );
        // 0, 3 and 6.
        assert_eq!(m.now, 3.0, "rounds of #11 to #13");
        // No fixes record was read, so what #12's rounds caught is unknown
        // and nothing is filed on it.
        assert_eq!(m.behind, ["core.nix#12: 6 rounds, what they caught unread"]);
        assert_eq!(all.len(), 1, "#12 filed though its fixes went unread");
    }

    /// The harness's record of the fixes for findings, each by the council
    /// that raised it and how it ended, `None` while it is still queued. An
    /// ending of two words is its kind and the commit it names.
    fn fixes(each: &[(&str, &str, Option<&str>)]) -> String {
        let fixes: Vec<serde_json::Value> = each
            .iter()
            .map(|(run, subject, outcome)| {
                let mut fix = serde_json::json!({"subject": {"finding": {"subject": subject,
                    "origin": {"runId": run}}}});
                if let Some(ending) = outcome {
                    fix["outcome"] = match ending.split_once(' ') {
                        Some((kind, commit)) => {
                            serde_json::json!({ "kind": kind, "commit": commit })
                        }
                        None => serde_json::json!({ "kind": ending }),
                    };
                }
                fix
            })
            .collect();
        serde_json::json!({ "fixes": fixes }).to_string()
    }

    /// Whether each change changed after a round read the head it names:
    /// #23 did after its sixth and the fix it made, #24 did after its sixth
    /// but not after the fix it made, #26 merged what its seventh read, and
    /// git can't read #25. A round that only committed fixes is never asked.
    fn moved(merge: &str, witness: &str) -> Option<bool> {
        match (merge, witness) {
            ("cccc", "c-read" | "c-made") | ("dddd", "d-read") => Some(true),
            ("dddd", "d-made") | ("ffff", "f-merged") => Some(false),
            (_, "f-fixed") => panic!("asked git of a round that queued nothing"),
            _ => None,
        }
    }

    /// The head each round past five read, where its record names one;
    /// #23's seventh names none, as records from before witnesses don't.
    fn witness(id: &str) -> Option<&'static str> {
        match id {
            "council-c-1" => Some("c-read"),
            "council-d-1" => Some("d-read"),
            "council-e-1" => Some("e-read"),
            "council-f-0" => Some("f-merged"),
            "council-f-1" => Some("f-fixed"),
            _ => None,
        }
    }

    /// Seven rounds on `change`, a minute apart, the sixth and seventh past
    /// five. Their ids count down as time goes on, so a round is taken as
    /// past five by when it started, not by its id: council-x-1 and
    /// council-x-0 are the sixth and seventh.
    fn seven(change: &str) -> Vec<String> {
        (0..7)
            .map(|n| {
                let id = format!("council-{change}-{}", 6 - n);
                let run = started(&id, 3600 - 60 * n);
                match witness(&id) {
                    Some(read) => {
                        run.replace(r#""round""#, &format!(r#""witness": "{read}", "round""#))
                    }
                    None => run,
                }
            })
            .collect()
    }

    #[test]
    fn a_merge_past_five_says_what_its_later_rounds_caught_or_goes_to_its_owner() {
        let log = format!(
            "aaaa {} Merge pull request #21 from Jitsusama/here/a\n\
             bbbb {} Merge pull request #22 from Jitsusama/here/b\n\
             cccc {} Merge pull request #23 from Jitsusama/here/c\n\
             dddd {} Merge pull request #24 from Jitsusama/here/d\n\
             eeee {} Merge pull request #25 from Jitsusama/here/e\n\
             ffff {} Merge pull request #26 from Jitsusama/here/f\n",
            NOW - 60,
            NOW - 120,
            NOW - 180,
            NOW - 240,
            NOW - 300,
            NOW - 360
        );
        let records: Vec<String> = ["21", "22", "23", "24", "25", "26"]
            .iter()
            .zip(["a", "b", "c", "d", "e", "f"])
            .map(|(number, change)| {
                let mut runs = seven(change);
                // A judge that consolidated #23's seventh.
                if change == "c" {
                    runs.push(started("judge-c", 30).replace(
                        r#""round": "council""#,
                        r#""round": "judge", "basis": "council-c-0""#,
                    ));
                }
                record("github", CORE, number, "elsewhere", &runs)
            })
            .collect();
        // #21's fifth round fixed one, which isn't past five, and its sixth's
        // fix ended in a way the record doesn't name; #23's seventh caught
        // two, one through its judge, and its sixth made one and queued one,
        // made before the merge since the change moved on after both, and
        // its seventh one whose round names no head, unread; #22's sixth was
        // only answered and its seventh's fix skipped; #24's sixth made one
        // and queued one the change didn't move on from, a follow-up, and its
        // seventh one unread; #25's sixth queued two whose merge git can't
        // read, unread, so it files nothing; #26's sixth fixed one and its
        // seventh queued two follow-ups.
        let fixed = [fixes(&[
            ("council-a-2", "in the fifth", Some("committed")),
            ("judge-c", "a test that never failed", Some("committed")),
            ("council-c-0", "a second", Some("committed")),
            ("council-c-1", "queued", None),
            ("council-c-1", "made", Some("committed c-made")),
            ("council-b-1", "answered", Some("answered")),
            ("council-b-0", "skipped", Some("skipped")),
            ("council-a-1", "unnamed", Some("dropped")),
            ("council-d-1", "queued alone", None),
            ("council-d-1", "made first", Some("committed d-made")),
            ("council-d-0", "unwitnessed too", None),
            ("council-e-1", "one queued", None),
            ("council-e-1", "two queued", None),
            ("council-c-0", "unwitnessed", None),
            ("council-f-1", "fixed", Some("committed")),
            ("council-f-0", "a follow-up", None),
            ("council-f-0", "a second follow-up", None),
        ])];
        let landed = BTreeMap::from([("core.nix#22".to_owned(), "B8RBNW".to_owned())]);
        // Each ask of git may fetch, so #25's sixth, queuing two, asks once.
        let asked = std::cell::Cell::new(0);
        let counted = |merge: &str, witness: &str| {
            asked.set(asked.get() + usize::from(witness == "e-read"));
            moved(merge, witness)
        };
        let all = rounds(
            &[Main {
                repo: CORE,
                log: &log,
                moved: &counted,
            }],
            &records,
            &[],
            &fixed,
            || landed,
            NOW,
            "ORCH",
        );
        assert_eq!(
            all[0].behind,
            [
                "core.nix#21: 7 rounds, those past five caught nothing",
                "core.nix#22: 7 rounds, those past five caught nothing",
                "core.nix#23: 7 rounds, past five caught queued and 3 more, 1 queued fix unread",
                "core.nix#24: 7 rounds, past five caught made first, 1 queued as a follow-up, \
                 1 queued fix unread",
                "core.nix#25: 7 rounds, those past five caught nothing, 2 queued fixes unread",
                "core.nix#26: 7 rounds, past five caught fixed, 2 queued as follow-ups",
            ],
            "a fix queued before the merge wasn't counted as caught, or a follow-up, an \
             unread fix or an outcome the record doesn't name was"
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
            "a merge whose rounds past five caught nothing goes to who landed it"
        );
        assert!(
            all[1..].iter().all(|m| m.past().is_some()),
            "each is past its target of five, so it's filed"
        );
        assert_eq!(
            asked.get(),
            1,
            "git was asked for each queued fix, not once a round"
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
                moved: &|_, _| None,
            }],
            &[],
            &[],
            &[],
            BTreeMap::new,
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

    #[test]
    fn a_round_counts_once_answered_and_one_run_elsewhere_files_nothing() {
        let log = format!(
            "aaaa {} Merge pull request #31 from Jitsusama/here/a\n\
             bbbb {} Merge pull request #32 from Jitsusama/here/b\n",
            NOW - 60,
            NOW - 120
        );
        // Of #31's five, only the first was collected with an answer: a
        // reload killed the second open with every reviewer failed, the
        // third ended with every reviewer failed, the fourth was closed,
        // and the fifth has no outcomes at all.
        let ended = |id, ago, outcomes| {
            started(id, ago).replace(r#"[{"participantId": "one"}]"#, outcomes)
        };
        let a = [
            started("council-a", 600),
            ended("council-a-open", 500, r#"[{"failure": "stopped"}], "open": true"#),
            ended("council-a-failed", 400, r#"[{"failure": "wall"}]"#),
            ended("council-a-closed", 300, r#"[{}], "closed": true"#),
            ended("council-a-bare", 200, "[]"),
        ];
        let ours = [
            record("github", CORE, "31", "elsewhere", &a),
            record(
                "github",
                CORE,
                "32",
                "elsewhere",
                &(0..5)
                    .map(|n| started(&format!("council-b-{n}"), 3600 - 60 * n))
                    .collect::<Vec<_>>(),
            ),
        ];
        // #32's sixth and seventh ran on the Mac; its fifth is in both
        // machines' records and counts once, as one run here.
        let theirs = [record(
            "github",
            CORE,
            "32",
            "",
            &[
                started("council-b-4", 3360),
                started("council-b-5", 3300),
                started("council-b-6", 3240),
            ],
        )];
        let fixed = [fixes(&[("council-b-0", "in the first", Some("committed"))])];
        let all = rounds(
            &[Main {
                repo: CORE,
                log: &log,
                moved: &|_, _| None,
            }],
            &ours,
            &theirs,
            &fixed,
            || panic!("asked who landed a merge though none is filed"),
            NOW,
            "ORCH",
        );
        assert_eq!(all[0].now, 4.0, "the median of #31's 1 and #32's 7");
        assert_eq!(
            all[0].behind,
            [concat!(
                "core.nix#32: 7 rounds, those past five caught nothing here, ",
                "2 rounds' fixes kept on another machine"
            )],
            "a round no reviewer answered counted, or one run elsewhere didn't"
        );
        assert_eq!(
            all.len(),
            1,
            "a merge whose late rounds ran elsewhere was filed as catching nothing"
        );
    }
}
