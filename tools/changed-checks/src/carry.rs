//! Whether a keep of a head of main carries each check's derivation from
//! the newest record before it rather than evaluating the head whole, and
//! how far carrying may go.
//!
//! On 13 November each keep evaluated its head whole: about 800 ordinary
//! checks in 88 to 198 s, beside the VM tests traced together in 149 to
//! 495 s. A keep took a median of about 560 s, so records trailed main by
//! 50 to 160 files and gates evaluated 400 to 650 checks. Gates already
//! carry a base's derivations from the newest record (`past` in
//! `alone.rs`), so a keep does the same: each check main reached nothing of
//! since keeps the record's derivation, and only the rest are evaluated.
//! When main reached nothing the VM tests read together, what they read is
//! carried too. When it reached every one of them, the keep evaluates
//! whole, tracing them together as before, since nothing else records what
//! they read together.
//!
//! A carried derivation is never evaluated again, so a wrong one would
//! stand. The record says how many keeps in a row carried to it, and a keep
//! evaluates whole once 6 have, or once the record it would carry from is
//! over 2 hours old, gate or not. A whole keep past a record names each
//! check a carry would have given another derivation, so a carry that went
//! wrong shows. Derivations a gate kept first may have been carried, so
//! they count as a carry; past the bound they're set aside and the keep
//! evaluates whole all the same, naming each check a gate kept otherwise.
//! On 14 November two keeps past the bound found their bases kept by gates
//! and evaluated nothing, counting a seventh and an eighth carry.

use std::fs;
use std::time::Duration;

use serde_json::Value;

use crate::alone;
use crate::checks::{self, Derivations};
use crate::reads::{self, Reads};
use crate::runs::Run;

/// How many keeps in a row may carry before one evaluates whole.
const IN_A_ROW: u64 = 6;

/// How old a record may be and still be carried from.
const CARRIED_FOR: Duration = Duration::from_hours(2);

/// The record a keep would carry from: its head, its derivations, how many
/// keeps in a row carried to them, and why a keep may not carry from it,
/// when it may not.
struct Record {
    head: String,
    known: Derivations,
    carried: u64,
    refused: Option<String>,
}

/// Keep `base`'s derivations, carried from the newest record when they may
/// be, or else as `whole` keeps them, and give how many keeps in a row have
/// carried to them.
pub fn keep(
    root: &str,
    base: &str,
    system: &str,
    noted: &mut Run,
    whole: impl FnOnce(&mut Run) -> Result<(), String>,
) -> Result<u64, String> {
    let record = newest(root, base, system);
    if let Some(record) = &record {
        match &record.refused {
            Some(why) => eprintln!(
                "changed-checks: the keep of {} evaluates whole, since {why}",
                short(base)
            ),
            None => {
                if let Some(carried) = carried(root, base, system, record, noted)? {
                    return Ok(carried);
                }
            }
        }
    }
    let path = checks::keeping(&crate::cache(), base, system);
    let aside = path.with_extension("gated");
    let bounded = record
        .as_ref()
        .is_some_and(|record| record.refused.is_some());
    let moved = |from: &std::path::Path, to: &std::path::Path| {
        fs::rename(from, to).map_err(|error| format!("{}: {error}", from.display()))
    };
    if bounded && path.exists() {
        moved(&path, &aside)?;
    }
    let fresh = !path.exists();
    if let Err(why) = whole(noted) {
        // Put back what a gate kept, so later gates on the base needn't
        // evaluate it again.
        if aside.exists() && !path.exists() {
            moved(&aside, &path)?;
        }
        return Err(why);
    }
    if !fresh {
        return Ok(record.map_or(0, |record| record.carried + 1));
    }
    let Ok(Some(evaluated)) = checks::read_kept(&path) else {
        return Ok(0);
    };
    if let Ok(Some(gated)) = checks::read_kept(&aside) {
        drifted(base, system, &evaluated, &gated, "a gate kept");
    }
    // Gone or not, it's no longer needed.
    let _ = fs::remove_file(&aside);
    if let Some(record) = &record
        && let Some(carry) = alone::carry(root, base, system, (&record.head, &record.known))
    {
        let than = format!("carrying from {} gave", short(&record.head));
        drifted(base, system, &evaluated, &carry.known, &than);
    }
    Ok(0)
}

/// The newest record of a head of main `base` came after, and whether a
/// keep of `base` may carry from it.
fn newest(root: &str, base: &str, system: &str) -> Option<Record> {
    let (head, known) = alone::recorded(root, base, system).filter(|(head, _)| head != base)?;
    let at = alone::keeping(&crate::cache(), &head, system);
    let carried = fs::read_to_string(&at)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|record| record.get("carried")?.as_u64())
        .unwrap_or(0);
    let old = fs::metadata(&at)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_none_or(|age| age > CARRIED_FOR);
    let refused = if carried >= IN_A_ROW {
        Some(format!(
            "{carried} keeps in a row carried up to {}",
            short(&head)
        ))
    } else if old {
        Some(format!(
            "the record at {} is over {} hours old",
            short(&head),
            CARRIED_FOR.as_secs() / 3600
        ))
    } else {
        None
    };
    Some(Record {
        head,
        known,
        carried,
        refused,
    })
}

/// Keep `base`'s derivations carried from `record`, and give how many keeps
/// in a row have carried; nothing when main's change from it can't be
/// judged, or reached every VM test, so the keep evaluates whole.
fn carried(
    root: &str,
    base: &str,
    system: &str,
    record: &Record,
    noted: &mut Run,
) -> Result<Option<u64>, String> {
    let Some(carry) = alone::carry(root, base, system, (&record.head, &record.known)) else {
        return Ok(None);
    };
    if carry.vm > 0 && carry.vm_left.len() == carry.vm {
        eprintln!(
            "changed-checks: the keep of {} evaluates whole, since main from {} reached each \
             of {system}'s VM tests",
            short(base),
            short(&record.head)
        );
        return Ok(None);
    }
    let flake = format!("git+file://{root}?rev={base}");
    let path = checks::keeping(&crate::cache(), base, system);
    checks::kept(&path, || carry.evaluated(&flake, system, noted))?;
    if carry.vm_kept
        && let Err(why) = vm_read(root, base, system, &record.head)
    {
        eprintln!(
            "changed-checks: couldn't carry what {system}'s VM tests read at {}: {why}",
            short(&record.head)
        );
    }
    let count = record.carried + 1;
    eprintln!(
        "changed-checks: the keep of {} carries from {}, the {} keep in a row to carry, the \
         derivations of {} of {system}'s {} checks main reached nothing of since",
        short(base),
        short(&record.head),
        ordinal(count),
        carry.known.len(),
        carry.ordinary + carry.vm
    );
    Ok(Some(count))
}

/// Keep as `base`'s what the VM tests read together at `head`, which main
/// reached none of since, unless it's kept already.
fn vm_read(root: &str, base: &str, system: &str, head: &str) -> Result<(), String> {
    let at = reads::keeping(&crate::cache(), base, system);
    if at.exists() {
        return Ok(());
    }
    let from = reads::keeping(&crate::cache(), head, system);
    let text = fs::read_to_string(&from).map_err(|error| format!("{}: {error}", from.display()))?;
    let read = Reads::from_kept(&text, &reads::tree(root, head)?)
        .ok_or("its record was made from another tree")?;
    reads::write(&at, &read, &reads::tree(root, base)?)
}

/// Say which checks `than` gave other derivations, `known`, than `base`'s
/// keep evaluated whole, `whole`.
fn drifted(base: &str, system: &str, whole: &Derivations, known: &Derivations, than: &str) {
    let differ: Vec<&str> = known
        .iter()
        .filter(|(name, derivation)| whole.get(*name) != Some(*derivation))
        .map(|(name, _)| name.as_str())
        .collect();
    if !differ.is_empty() {
        eprintln!(
            "changed-checks: evaluated whole at {}, {system}'s {} came to other derivations than \
             {than}",
            short(base),
            differ.join(", ")
        );
    }
}

/// `count` as a word that places it: first, second and so on.
fn ordinal(count: u64) -> String {
    const PLACES: [&str; 6] = ["first", "second", "third", "fourth", "fifth", "sixth"];
    usize::try_from(count)
        .ok()
        .and_then(|count| PLACES.get(count.checked_sub(1)?))
        .map_or_else(|| format!("{count}th"), |place| (*place).to_owned())
}

/// A commit by its first 12 characters, as changed-checks names one.
fn short(commit: &str) -> &str {
    commit.get(..12).unwrap_or(commit)
}
