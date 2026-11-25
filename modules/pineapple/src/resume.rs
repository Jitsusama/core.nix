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
//! Each of Joel's resumes is also kept as a line in resumes.jsonl beside
//! the stops' folder, whether or not it ended a stop, so a stop a search
//! indexes only after he resumed is kept as resumed when it was said
//! before a resume that names it: what he ended stays ended.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::slack::micros;

/// Microseconds in a millisecond.
const MICROS_A_MILLI: u128 = 1000;

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
/// ids it resumed; none when the scope names none standing.
///
/// # Errors
///
/// Why a record couldn't be rewritten; those before it stay resumed.
pub fn resume(folder: &Path, scope: &str, at: u64, by: &str) -> io::Result<Vec<String>> {
    note(folder, scope, at, by)?;
    resume_only(folder, scope, at, by, |_| true)
}

/// Keeps one of Joel's resumes, of `scope` at `at` in milliseconds, in
/// the resumes beside `folder`, before any stop is marked, so none ends
/// without it.
fn note(folder: &Path, scope: &str, at: u64, by: &str) -> io::Result<()> {
    crate::record::append(
        &resumes(folder),
        &json!({ "at": at, "by": by, "scope": scope }),
    )
}

/// How a stop about to be kept is marked when Joel already resumed it:
/// the latest of his resumes kept beside it that names it and came at or
/// after it was said, as a search can index a stop only after he resumed
/// what he'd said. None for a stop no resume since names, or whose record
/// says only when it was heard, which is now.
#[must_use]
pub fn resumed_since(folder: &Path, stop: &Value) -> Option<Value> {
    let said = stop["said_at"].as_str().and_then(micros)?;
    let id = stop.get("id").and_then(Value::as_str).unwrap_or_default();
    let kept = Standing {
        id: id.to_owned(),
        file: PathBuf::new(),
        record: stop.clone(),
    };
    fs::read_to_string(resumes(folder))
        .unwrap_or_default()
        .lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|resume| {
            resume["at"]
                .as_u64()
                .is_some_and(|at| u128::from(at) * MICROS_A_MILLI >= said)
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
    if !only_shadow {
        // Kept as when it was said, which is what a stop said is
        // weighed against.
        let when = u64::try_from(said / MICROS_A_MILLI).unwrap_or(u64::MAX);
        note(folder, scope, when, "joel")?;
    }
    resume_only(folder, scope, at, "joel", |record| {
        (!only_shadow || record["shadow"] != false) && said_by(record).is_some_and(|by| by <= said)
    })
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

    #[test]
    fn a_stop_said_before_a_resume_naming_it_and_heard_after_is_kept_resumed() {
        let dir = folder("late", &[]);
        let said = |id: &str, reaches: &[&str], at: &str| {
            let mut one = stop(id, reaches);
            one["said_at"] = json!(at);
            one
        };
        // At 1,700,000,050,000 ms, a resume of one pull request's stops,
        // though none stood.
        let at = 1_700_000_050_000;
        assert!(
            resume(&dir, "pr:example/shop#12", at, "joel")
                .expect("resumes")
                .is_empty(),
            "nothing stood to end"
        );
        let before = said("before", &["pr:example/shop#12"], "1700000049.000000");
        assert_eq!(
            resumed_since(&dir, &before),
            Some(json!({ "at": at, "by": "joel" })),
            "said before the resume naming it, it's kept as that resume left it"
        );
        let after = said("after", &["pr:example/shop#12"], "1700000051.000000");
        assert_eq!(resumed_since(&dir, &after), None, "said after, it stands");
        let elsewhere = said("elsewhere", &["quest:QEST-A"], "1700000049.000000");
        assert_eq!(
            resumed_since(&dir, &elsewhere),
            None,
            "one the resume doesn't name stands"
        );
        let mut heard = stop("heard", &["pr:example/shop#12"]);
        heard["heard_at"] = json!(1_700_000_049_000_u64);
        assert_eq!(
            resumed_since(&dir, &heard),
            None,
            "one saying only when it was heard stands"
        );
        resume(&dir, "all", at + 5_000, "launcher").expect("resumes");
        assert_eq!(
            resumed_since(&dir, &after),
            Some(json!({ "at": at + 5_000, "by": "launcher" })),
            "a later resume of everything names it too"
        );
        assert!(
            standing(&dir).is_empty(),
            "what's kept of the resumes is no stop"
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
        let mut before = stop("before", &["everything"]);
        before["said_at"] = json!("1700000049.000000");
        let mut after = stop("after", &["everything"]);
        after["said_at"] = json!("1700000060.000000");
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
