//! Ending a stop, which only Joel does: which stops stand, which of them a
//! scope he names reaches, and marking those resumed.
//!
//! A stop's record is never removed. Resuming it writes when and by whom
//! into its `resumed`, so the record says afterwards what stood and for
//! how long, and everything acting on stops (the harness's write guard,
//! parking and holds) takes a record with `resumed` set for one that no
//! longer stands. Resuming also clears its thread's reply, so a stop said
//! again there after Joel resumed is answered as a new one.
//!
//! Each of Joel's resumes is also kept in resumes.jsonl beside the stops'
//! folder, whether or not it ended a stop, so a stop of his own that a
//! search indexes only after he resumed is kept as resumed when he said it
//! shortly before a resume that names it: what he ended stays ended. A
//! stop anyone else said stands however late it's heard, as Joel may
//! never have seen it.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::slack::micros;

/// Microseconds in a millisecond.
const MICROS_A_MILLI: u128 = 1000;

/// Microseconds in a second.
const MICROS_A_SECOND: u128 = 1_000_000;

/// How long before a resume a stop of Joel's may have been said and still
/// be kept as that resume left it: past the eight and a half minutes a
/// search has been seen to index late, and well short of a reader away
/// for an afternoon.
const LATE_AT_MOST: u128 = 30 * 60 * MICROS_A_SECOND;

/// How long after a stop was said a resume must come to end it: a resume
/// is timed by this machine's clock and a stop by its platform's, which
/// writes whole seconds on GitHub, so one said in the same few seconds
/// stands, costing at most one more press.
const SKEW: u128 = 5 * MICROS_A_SECOND;

/// The file beside the stops' folder keeping each of Joel's resumes as a
/// line: when, by what, and the scope it named. It's kept outside the
/// folder, so nothing reading the folder's records meets it.
const RESUMES: &str = "resumes.jsonl";

/// Where the resumes are kept for the stops kept in `folder`.
fn resumes(folder: &Path) -> PathBuf {
    folder.with_file_name(RESUMES)
}

/// A stop standing, with the file it's kept in.
#[derive(Clone, Debug)]
pub struct Standing {
    /// Its id, as its record names it, or its file's when it names none.
    pub id: String,
    /// The file holding its record.
    pub file: PathBuf,
    /// The record as written.
    pub record: Value,
}

/// Every stop standing in `folder`, by file name; a record that can't be
/// read is left out, as it is wherever stops are acted on.
#[must_use]
pub fn standing(folder: &Path) -> Vec<Standing> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|kind| kind == "json"))
        .collect();
    files.sort();
    files
        .into_iter()
        .filter_map(|file| {
            let record: Value = serde_json::from_str(&fs::read_to_string(&file).ok()?).ok()?;
            if !record.get("resumed").is_none_or(Value::is_null) {
                return None;
            }
            let stem = file.file_stem()?.to_string_lossy().into_owned();
            let id = record
                .get("id")
                .and_then(Value::as_str)
                .map_or(stem, str::to_owned);
            Some(Standing { id, file, record })
        })
        .collect()
}

/// What a stop reaches, as its record says.
#[must_use]
pub fn reaches(record: &Value) -> Vec<&str> {
    record
        .get("reaches")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

/// Whether the scope Joel names reaches a stop: `all` reaches every one,
/// and otherwise its id, or one of its reaches as the record writes it,
/// such as `pr:owner/name#12` or `quest:ID`, letter case aside.
#[must_use]
pub fn names(stop: &Standing, scope: &str) -> bool {
    let scope = scope.trim();
    scope.eq_ignore_ascii_case("all")
        || stop.id == scope
        || reaches(&stop.record)
            .iter()
            .any(|reach| reach.eq_ignore_ascii_case(scope))
}

/// Marks each stop `scope` names resumed at `at` (milliseconds since the
/// epoch) by `by`, written whole and renamed into place, and answers the
/// ids it resumed; none when the scope names none standing. Once they're
/// all marked, the resume is kept beside them, with what each stop it
/// ended reached, so a resume of one stop also ends a stop of Joel's said
/// there before it and heard late.
///
/// # Errors
///
/// Why a record couldn't be rewritten; those before it stay resumed, and
/// the resume isn't kept, so a stop heard late stands.
pub fn resume(folder: &Path, scope: &str, at: u64, by: &str) -> io::Result<Vec<String>> {
    let named: Vec<Standing> = standing(folder)
        .into_iter()
        .filter(|stop| names(stop, scope))
        .collect();
    let resumed = resume_only(folder, scope, at, by, |_| true)?;
    let mut noted = vec![scope.to_owned()];
    for stop in named.iter().filter(|stop| resumed.contains(&stop.id)) {
        for reach in reaches(&stop.record) {
            if !noted.iter().any(|kept| kept.eq_ignore_ascii_case(reach)) {
                noted.push(reach.to_owned());
            }
        }
    }
    note(folder, &noted, at, by);
    Ok(resumed)
}

/// Keeps one of Joel's resumes, of each of `named` at `at` in
/// milliseconds, in the resumes beside `folder`. One that can't be kept
/// is said and let go, as it only keeps a stop heard late from standing:
/// his resume still ends what it named.
fn note(folder: &Path, named: &[String], at: u64, by: &str) {
    for one in named {
        let line = json!({ "at": at, "by": by, "scope": one });
        if let Err(error) = crate::record::append(&resumes(folder), &line) {
            eprintln!(
                "<3>couldn't keep a resume in {}, so a stop said before it and heard \
                 late stands: {error}",
                resumes(folder).display()
            );
        }
    }
}

/// Whether a resume at `at` ends a stop said at `said` and heard at
/// `heard`, all in microseconds: it came more than [`SKEW`] after the stop
/// was said, and at most [`LATE_AT_MOST`], and no later than the stop was
/// heard, so a resume dated ahead ends nothing still to be said.
fn ends(said: u128, at: u128, heard: u128) -> bool {
    at <= heard && at >= said.saturating_add(SKEW) && at - said <= LATE_AT_MOST
}

/// How a stop about to be kept is marked when Joel already resumed it:
/// the first of his resumes kept beside it that names it and [`ends`] it,
/// as a search can index a stop only after he resumed what he'd said.
/// None for a stop someone else said, which he may never have seen, or
/// one whose record says only when it was heard, which is now.
#[must_use]
pub fn resumed_since(folder: &Path, stop: &Value) -> Option<Value> {
    if stop["by_joel"] != true {
        return None;
    }
    let said = stop["said_at"].as_str().and_then(micros)?;
    let heard = u128::from(stop["heard_at"].as_u64()?) * MICROS_A_MILLI;
    let id = stop.get("id").and_then(Value::as_str).unwrap_or_default();
    let kept = Standing {
        id: id.to_owned(),
        file: PathBuf::new(),
        record: stop.clone(),
    };
    fs::read_to_string(resumes(folder))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|resume| {
            resume["at"]
                .as_u64()
                .is_some_and(|at| ends(said, u128::from(at) * MICROS_A_MILLI, heard))
                && resume["scope"]
                    .as_str()
                    .is_some_and(|scope| names(&kept, scope))
        })
        .map(|resume| json!({ "at": resume["at"], "by": resume["by"] }))
}

/// A stop's record `stop`, about to be kept in `folder`, as Joel left it:
/// marked resumed by [`resumed_since`] when he resumed it before it was
/// heard, and otherwise as it is.
#[must_use]
pub fn as_left(folder: &Path, mut stop: Value) -> Value {
    if let Some(resumed) = resumed_since(folder, &stop) {
        stop["resumed"] = resumed;
    }
    stop
}

/// Whether a stop's record says it's been resumed, so it no longer stands.
#[must_use]
pub fn ended(stop: &Value) -> bool {
    !stop.get("resumed").is_none_or(Value::is_null)
}

/// As [`resume`], but only the stops whose records `keep` picks, so a
/// reader in shadow ends only the stops it wrote in shadow, never one
/// that stands for real.
///
/// # Errors
///
/// Why a record couldn't be rewritten; those before it stay resumed.
pub fn resume_only(
    folder: &Path,
    scope: &str,
    at: u64,
    by: &str,
    keep: impl Fn(&Value) -> bool,
) -> io::Result<Vec<String>> {
    let mut resumed = Vec::new();
    for stop in standing(folder)
        .into_iter()
        .filter(|stop| names(stop, scope) && keep(&stop.record))
    {
        let mut record = stop.record;
        if let Some(fields) = record.as_object_mut() {
            fields.insert("resumed".to_owned(), json!({ "at": at, "by": by }));
        }
        let written = stop.file.with_extension("new");
        fs::write(&written, format!("{record:#}\n"))?;
        fs::rename(&written, &stop.file)?;
        resumed.push(stop.id);
    }
    Ok(resumed)
}

/// Ends the stops Joel's own resume, said where a reader heard it, names
/// in `scope`, as his press in the launcher does. Only stops said at or
/// before `said` (in microseconds), since a search can index late and a
/// round can judge a resume beside a stop said after it (KP0L6A); a stop
/// whose record says neither when it was said nor heard is left for his
/// press. With `only_shadow`, as a reader in shadow or one whose resumes
/// aren't let go live, it ends only stops written in shadow, never one
/// that stands for real.
///
/// # Errors
///
/// Why a record couldn't be rewritten; those before it stay resumed.
pub fn as_joel(
    folder: &Path,
    scope: &str,
    at: u64,
    said: u128,
    only_shadow: bool,
) -> io::Result<Vec<String>> {
    let resumed = resume_only(folder, scope, at, "joel", |record| {
        (!only_shadow || record["shadow"] != false) && said_by(record).is_some_and(|by| by <= said)
    })?;
    if !only_shadow {
        // Kept as when it was said, which is what a stop said is
        // weighed against.
        let when = u64::try_from(said / MICROS_A_MILLI).unwrap_or(u64::MAX);
        note(folder, &[scope.to_owned()], when, "joel");
    }
    Ok(resumed)
}

/// When a stop was said, in microseconds: as its record writes Slack's
/// timestamp, else when it was heard, in milliseconds.
fn said_by(record: &Value) -> Option<u128> {
    record["said_at"].as_str().and_then(micros).or_else(|| {
        record["heard_at"]
            .as_u64()
            .map(|at| u128::from(at) * MICROS_A_MILLI)
    })
}

/// The reply ledger without the what-stopped replies of stops since
/// resumed, so the gate answers a thread again once its stop is over; a
/// line naming no stop, or one never kept, is left as it is.
#[must_use]
pub fn cleared(ledger: &str, folder: &Path) -> String {
    let over: Vec<String> = fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let record: Value =
                serde_json::from_str(&fs::read_to_string(entry.path()).ok()?).ok()?;
            if record.get("resumed").is_none_or(Value::is_null) {
                return None;
            }
            record.get("id").and_then(Value::as_str).map(str::to_owned)
        })
        .collect();
    let kept: Vec<&str> = ledger
        .lines()
        .filter(|line| {
            let Ok(one) = serde_json::from_str::<Value>(line) else {
                return true;
            };
            let stopped = one.get("reply").and_then(Value::as_str) == Some("what-stopped");
            let key = one.get("key").and_then(Value::as_str).unwrap_or_default();
            !(stopped && over.iter().any(|id| id == key))
        })
        .collect();
    kept.iter().map(|line| [line, "\n"].concat()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record;

    /// A stops folder of its own, in a state folder of its own, as the
    /// resumes are kept beside it.
    fn folder(name: &str, records: &[Value]) -> PathBuf {
        let state =
            std::env::temp_dir().join(format!("pineapple-resume-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&state);
        let dir = state.join("stops");
        fs::create_dir_all(&dir).expect("a scratch folder is made");
        for (at, record) in records.iter().enumerate() {
            fs::write(dir.join(format!("{at}.json")), record.to_string())
                .expect("a record is written");
        }
        dir
    }

    fn stop(id: &str, reaches: &[&str]) -> Value {
        json!({ "id": id, "shadow": true, "reaches": reaches, "resumed": null })
    }

    #[test]
    fn a_scope_resumes_what_it_names_and_leaves_the_rest() {
        let dir = folder(
            "names",
            &[
                stop("slack:C1:1.0", &["pr:example/shop#12"]),
                stop("slack:C1:2.0", &["quest:QEST-A"]),
            ],
        );
        let resumed = resume(&dir, "PR:example/shop#12", 7, "joel").expect("resumes");
        assert_eq!(resumed, ["slack:C1:1.0"], "only the stop its reach names");
        let left: Vec<String> = standing(&dir).into_iter().map(|stop| stop.id).collect();
        assert_eq!(left, ["slack:C1:2.0"], "the other still stands");
        let record: Value =
            serde_json::from_str(&fs::read_to_string(dir.join("0.json")).expect("kept"))
                .expect("json");
        assert_eq!(
            record["resumed"],
            json!({ "at": 7, "by": "joel" }),
            "marked, not removed"
        );
    }

    #[test]
    fn a_resume_only_ends_the_stops_its_caller_picks() {
        let mut live = stop("live", &["pr:example/shop#12"]);
        live["shadow"] = json!(false);
        let dir = folder("only", &[stop("shadow", &["pr:example/shop#12"]), live]);
        let resumed = resume_only(&dir, "pr:example/shop#12", 1, "joel", |record| {
            record["shadow"] != false
        })
        .expect("resumes");
        assert_eq!(resumed, ["shadow"], "only the shadow stop is ended");
        let left: Vec<String> = standing(&dir).into_iter().map(|stop| stop.id).collect();
        assert_eq!(left, ["live"], "the live stop still stands");
    }

    #[test]
    fn joels_resume_ends_only_stops_said_before_it_and_known_to_be() {
        let said = |id: &str, at: &str| {
            let mut one = stop(id, &["everything"]);
            one["said_at"] = json!(at);
            one
        };
        let mut heard = stop("heard", &["everything"]);
        heard["heard_at"] = json!(1_700_000_000_000_u64);
        let mut live = said("live", "1700000000.000100");
        live["shadow"] = json!(false);
        let records = [
            said("before", "1700000000.000100"),
            heard,
            said("after", "1700000099.000100"),
            stop("unknown", &["everything"]),
            live,
        ];
        let dir = folder("as-joel", &records);
        let resume = 1_700_000_050_000_000;
        assert_eq!(
            as_joel(&dir, "all", 1, resume, true).expect("resumes"),
            ["before", "heard"],
            "said or heard before it, and in shadow"
        );
        assert_eq!(
            as_joel(&dir, "all", 1, resume, false).expect("resumes"),
            ["live"],
            "a live stop too, once resumes may end one"
        );
        let left: Vec<String> = standing(&dir).into_iter().map(|stop| stop.id).collect();
        assert_eq!(left, ["after", "unknown"], "later and unknown stops stand");
    }

    /// A stop of Joel's reaching `reaches`, said at `said` as Slack writes
    /// it, and heard at 1,700,000,100,000 ms.
    fn joels(id: &str, reaches: &[&str], said: &str) -> Value {
        let mut one = stop(id, reaches);
        one["by_joel"] = json!(true);
        one["said_at"] = json!(said);
        one["heard_at"] = json!(1_700_000_100_000_u64);
        one
    }

    #[test]
    fn a_stop_of_joels_said_before_a_resume_naming_it_and_heard_after_is_kept_resumed() {
        let dir = folder("late", &[]);
        // At 1,700,000,050,000 ms, a resume of one pull request's stops,
        // though none stood.
        let at = 1_700_000_050_000;
        assert!(
            resume(&dir, "pr:example/shop#12", at, "joel")
                .expect("resumes")
                .is_empty(),
            "nothing stood to end"
        );
        let pr = ["pr:example/shop#12"];
        let before = joels("before", &pr, "1700000040.000000");
        assert_eq!(
            resumed_since(&dir, &before),
            Some(json!({ "at": at, "by": "joel" })),
            "said before the resume naming it, it's kept as that resume left it"
        );
        let cases = [
            (
                joels("after", &pr, "1700000051.000000"),
                "said after, it stands",
            ),
            (
                joels("close", &pr, "1700000046.000000"),
                "said within the clocks' skew of it, it stands",
            ),
            (
                joels("long-before", &pr, "1699998190.000000"),
                "said more than half an hour before, it stands",
            ),
            (
                joels("elsewhere", &["quest:QEST-A"], "1700000040.000000"),
                "one the resume doesn't name stands",
            ),
        ];
        for (late, why) in &cases {
            assert_eq!(resumed_since(&dir, late), None, "{why}");
        }
        let mut others = before.clone();
        others["by_joel"] = json!(false);
        assert_eq!(
            resumed_since(&dir, &others),
            None,
            "someone else's, which he may never have seen, stands"
        );
        let mut early = before.clone();
        early["heard_at"] = json!(at - 1_000);
        assert_eq!(
            resumed_since(&dir, &early),
            None,
            "one heard before the resume's time, which is dated ahead, stands"
        );
        let mut heard = stop("heard", &pr);
        heard["by_joel"] = json!(true);
        heard["heard_at"] = json!(1_700_000_049_000_u64);
        assert_eq!(
            resumed_since(&dir, &heard),
            None,
            "one saying only when it was heard stands"
        );
        resume(&dir, "all", at + 20_000, "launcher").expect("resumes");
        assert_eq!(
            resumed_since(&dir, &cases[0].0),
            Some(json!({ "at": at + 20_000, "by": "launcher" })),
            "a later resume of everything names it too"
        );
        assert_eq!(
            resumed_since(&dir, &before),
            Some(json!({ "at": at, "by": "joel" })),
            "one both name is kept as the first left it"
        );
        assert!(
            resumes(&dir).exists() && !dir.join(RESUMES).exists(),
            "the resumes are kept beside the stops' folder, not in it"
        );
    }

    #[test]
    fn a_resume_of_one_stop_keeps_what_it_reached_for_a_stop_heard_late() {
        let dir = folder("reached", &[stop("one", &["pr:example/shop#12"])]);
        let at = 1_700_000_050_000;
        assert_eq!(
            resume(&dir, "one", at, "joel").expect("resumes"),
            ["one"],
            "the stop it names"
        );
        assert_eq!(
            resumed_since(
                &dir,
                &joels("there", &["pr:example/shop#12"], "1700000040.000000")
            ),
            Some(json!({ "at": at, "by": "joel" })),
            "one of Joel's said where it reached, before it, is kept resumed"
        );
        assert_eq!(
            resumed_since(
                &dir,
                &joels("elsewhere", &["quest:QEST-A"], "1700000040.000000")
            ),
            None,
            "one said somewhere else stands"
        );
    }

    #[test]
    fn joels_typed_resume_is_kept_as_said_and_one_in_shadow_not_at_all() {
        let dir = folder("typed", &[]);
        let said = 1_700_000_050_000_000;
        as_joel(&dir, "all", 1, said, true).expect("resumes");
        assert!(
            !resumes(&dir).exists(),
            "a resume in shadow ends no live stop, so none is kept"
        );
        as_joel(&dir, "all", 1_700_000_099_000, said, false).expect("resumes");
        let before = joels("before", &["everything"], "1700000040.000000");
        let after = joels("after", &["everything"], "1700000060.000000");
        assert_eq!(
            resumed_since(&dir, &before),
            Some(json!({ "at": 1_700_000_050_000_u64, "by": "joel" })),
            "kept as when he said it"
        );
        assert_eq!(
            resumed_since(&dir, &after),
            None,
            "a stop said after he said resume, though before it was heard, stands"
        );
    }

    #[test]
    fn all_resumes_every_stop_and_an_unknown_scope_none() {
        let dir = folder(
            "all",
            &[stop("a", &["everything"]), stop("b", &["realm:work"])],
        );
        assert_eq!(
            resume(&dir, "repo:example/x", 1, "joel").expect("resumes"),
            Vec::<String>::new(),
            "none named"
        );
        assert_eq!(
            resume(&dir, "all", 1, "joel").expect("resumes"),
            ["a", "b"],
            "every one"
        );
        assert!(standing(&dir).is_empty(), "nothing stands after");
    }

    #[test]
    fn a_resumed_stop_clears_its_threads_reply() {
        let dir = folder(
            "cleared",
            &[
                stop("over", &["everything"]),
                stop("still", &["everything"]),
            ],
        );
        resume(&dir, "over", 1, "joel").expect("resumes");
        let ledger = concat!(
            "{\"at\":1,\"thread\":\"t1\",\"reply\":\"what-stopped\",\"key\":\"over\"}\n",
            "{\"at\":2,\"thread\":\"t2\",\"reply\":\"what-stopped\",\"key\":\"still\"}\n",
            "{\"at\":3,\"thread\":\"t1\",\"reply\":\"joel-asked\",\"key\":\"over\"}\n",
        );
        let kept = cleared(ledger, &dir);
        let ats: Vec<u64> = record::ledger(&kept).iter().map(|one| one.at).collect();
        assert_eq!(
            ats,
            [2, 3],
            "the resumed stop's reply goes; the standing one's and the ask stay"
        );
    }
}
