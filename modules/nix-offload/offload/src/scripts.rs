//! The startup scripts root has named for lent builders, each with when
//! it was first named, kept where only `lines` reads so they outlast the
//! switch that changes them.
//!
//! A switch that names a new script restarts `lines`, which then saw every
//! builder standing run another script than the machine's, and refused
//! them all, cutting every run on them. Kept here, the script before is
//! still trusted on a builder made before the change, for as long as such
//! a builder can live, so a change drains as builders end rather than
//! cutting them. The record starts with the first script named after this
//! file came to be, so the first change after it ships still cuts.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde_json::{Value, json};

use crate::vet::Earlier;

/// How long after a change a builder made before it may run on: as long
/// as the lenders' longest lend, 12 hours, and a margin for its boot.
pub const DRAINS_FOR: i64 = 13 * 3600;

/// A script root named, and when it first named it, in seconds since 1970.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Named {
    pub script: String,
    pub since: i64,
}

/// What follows from `kept`, oldest first, once `script` is named at
/// `now`: what to keep, when it differs from `kept`, and the scripts before
/// it that a builder may still run. Each earlier script is trusted on a
/// builder made before the one after it was named, for [`DRAINS_FOR`]
/// after that; one past it is dropped.
pub fn named(kept: &[Named], script: &str, now: i64) -> (Option<Vec<Named>>, Vec<Earlier>) {
    let mut all = kept.to_vec();
    if all.last().is_none_or(|last| last.script != script) {
        all.retain(|one| one.script != script);
        all.push(Named {
            script: script.to_owned(),
            since: now,
        });
    }
    let afters: Vec<i64> = all.iter().skip(1).map(|one| one.since).collect();
    let mut keep = Vec::new();
    let mut earlier = Vec::new();
    for (one, after) in all.iter().zip(afters.iter().map(Some).chain([None])) {
        match after {
            Some(&after) if now >= after + DRAINS_FOR => {}
            Some(&after) => {
                keep.push(one.clone());
                earlier.push(Earlier {
                    script: one.script.clone(),
                    made_before: after,
                });
            }
            None => keep.push(one.clone()),
        }
    }
    let changed = (keep != kept).then_some(keep);
    (changed, earlier)
}

/// The scripts kept in `file`, none when it isn't there yet.
fn read(file: &Path) -> Result<Vec<Named>, String> {
    let text = match fs::read_to_string(file) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("{}: {error}", file.display())),
    };
    let value: Value = serde_json::from_str(&text)
        .map_err(|error| format!("{} isn't JSON: {error}", file.display()))?;
    value
        .as_array()
        .ok_or(format!("{} holds no list", file.display()))?
        .iter()
        .map(|one| {
            Some(Named {
                script: one["script"].as_str()?.to_owned(),
                since: one["since"].as_i64()?,
            })
        })
        .collect::<Option<_>>()
        .ok_or(format!(
            "{} holds an entry without its script or time",
            file.display()
        ))
}

/// Writes `named` to `file` whole, through a draft beside it, so a reader
/// never sees half of it.
fn write(file: &Path, named: &[Named]) -> Result<(), String> {
    let list: Vec<Value> = named
        .iter()
        .map(|one| json!({"script": one.script, "since": one.since}))
        .collect();
    let draft = file.with_extension("draft");
    fs::write(&draft, Value::Array(list).to_string())
        .and_then(|()| fs::rename(&draft, file))
        .map_err(|error| format!("{}: {error}", file.display()))
}

/// The scripts before `script` that a builder may still run, as `file`
/// keeps them at `now`, recording `script` there once it's new.
pub fn earlier(file: &Path, script: &str, now: i64) -> Result<Vec<Earlier>, String> {
    let kept = read(file)?;
    let (changed, earlier) = named(&kept, script, now);
    if let Some(keep) = changed {
        write(file, &keep)?;
    }
    Ok(earlier)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(script: &str, since: i64) -> Named {
        Named {
            script: script.to_owned(),
            since,
        }
    }

    #[test]
    fn the_first_script_is_kept_with_nothing_before_it() {
        assert_eq!(
            named(&[], "a", 100),
            (Some(vec![one("a", 100)]), Vec::new()),
            "the first script wasn't kept alone"
        );
        assert_eq!(
            named(&[one("a", 100)], "a", 500),
            (None, Vec::new()),
            "naming the same script again changed the record"
        );
    }

    #[test]
    fn a_changed_script_trusts_the_one_before_on_builders_made_before() {
        let (changed, earlier) = named(&[one("a", 100)], "b", 500);
        assert_eq!(
            changed,
            Some(vec![one("a", 100), one("b", 500)]),
            "the new script wasn't kept after the old"
        );
        assert_eq!(
            earlier,
            vec![Earlier {
                script: "a".to_owned(),
                made_before: 500
            }],
            "the old script wasn't trusted on builders made before the change"
        );
        let kept = changed.unwrap_or_default();
        assert_eq!(
            named(&kept, "b", 500 + DRAINS_FOR - 1).1.len(),
            1,
            "the old script went before its builders could have ended"
        );
        assert_eq!(
            named(&kept, "b", 500 + DRAINS_FOR),
            (Some(vec![one("b", 500)]), Vec::new()),
            "the old script outlived every builder that could run it"
        );
    }

    #[test]
    fn a_script_named_again_is_named_anew() {
        let (changed, earlier) = named(&[one("a", 100), one("b", 200)], "a", 300);
        assert_eq!(
            changed,
            Some(vec![one("b", 200), one("a", 300)]),
            "a script named again kept its first time"
        );
        assert_eq!(
            earlier,
            vec![Earlier {
                script: "b".to_owned(),
                made_before: 300
            }],
            "the script between wasn't the one trusted before"
        );
    }

    #[test]
    fn the_record_outlasts_a_restart() {
        let folder = tempfile::tempdir().expect("no folder for the record");
        let file = folder.path().join("scripts.json");
        assert_eq!(
            earlier(&file, "a", 100),
            Ok(Vec::new()),
            "the first script read as having one before it"
        );
        assert_eq!(
            earlier(&file, "b", 500),
            Ok(vec![Earlier {
                script: "a".to_owned(),
                made_before: 500
            }]),
            "the script before the change wasn't read back"
        );
        assert_eq!(
            read(&file),
            Ok(vec![one("a", 100), one("b", 500)]),
            "the record didn't hold both scripts"
        );
        fs::write(&file, "[{\"script\": 1}]").expect("couldn't spoil the record");
        assert!(
            earlier(&file, "b", 600).is_err(),
            "a spoiled record read as one"
        );
    }
}
