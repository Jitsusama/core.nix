//! Ending a stop, which only Joel does: which stops stand, which of them a
//! scope he names reaches, and marking those resumed.
//!
//! A stop's record is never removed. Resuming it writes when and by whom
//! into its `resumed`, so the record says afterwards what stood and for
//! how long, and everything acting on stops (the harness's write guard,
//! parking and holds) takes a record with `resumed` set for one that no
//! longer stands. Resuming also clears its thread's reply, so a stop said
//! again there after Joel resumed is answered as a new one.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

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
    let mut resumed = Vec::new();
    for stop in standing(folder)
        .into_iter()
        .filter(|stop| names(stop, scope))
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

    fn folder(name: &str, records: &[Value]) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("pineapple-resume-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
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
