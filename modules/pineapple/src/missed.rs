//! Rows saying what a reader read on past and never read, so Joel can
//! look there himself for a stop said then: each in For You, beside the
//! row saying whether the reader hears, until he says he's looked.
//!
//! A row is named for its reader, its kind and its span, so a span
//! written again is the same row, and a kind's spans that come within
//! [`MEET`] of each other are said as one row, whole or in parts, as
//! Joel searches one stretch at once. Each reader words its own rows
//! (deaf.rs for Slack, the GitHub reader's left.rs) and keeps them here.

use std::path::Path;

use serde_json::{Value, json};

use crate::deaf::{Change, make};

/// How long a row is kept after the span it names ends, in milliseconds:
/// a week, long past when Joel would look for a stop said then.
pub const MISSED_FOR: u64 = 7 * 24 * 60 * 60 * 1000;

/// How near one kind's spans come, in milliseconds, to be said in one
/// row: ten minutes, as long as a reader's own look back, as one stretch
/// Joel searches at once. Spans that keep coming that near chain into one
/// row however long the stretch, said as parts once any of it was read;
/// farther apart they're rows of their own.
pub const MEET: u64 = 10 * 60 * 1000;

/// A row's name for what it missed from `from` to `to`, in milliseconds
/// since 1970: `prefix`, naming its reader, then its `kind`, then its
/// start, an unknown one as 0, and its end. A kind may hold `-`, as the
/// span is read back from the end.
#[must_use]
pub fn named(prefix: &str, kind: &str, from: Option<u64>, to: u64) -> String {
    format!("{prefix}{kind}-{}-{to}", from.unwrap_or(0))
}

/// When a span from `from` to `to`, each in milliseconds since 1970 or
/// unknown, was, as Joel reads it in a row raised `at`: as minutes before
/// then, so no zone is named, a start part way into a minute said from
/// the whole one.
#[must_use]
pub fn when(from: Option<u64>, to: Option<u64>, at: u64) -> String {
    let ago = |then: u64| at.saturating_sub(then);
    match (
        from.map(|from| ago(from).div_ceil(60_000)),
        to.map(|to| ago(to) / 60_000),
    ) {
        (Some(from), Some(to)) if to > 0 && to < from => {
            format!("from {from} to {} before this was raised", minutes(to))
        }
        (Some(from), _) => format!("in the {} before this was raised", minutes(from)),
        (None, _) => "before this was raised, since when it last read can't be told".to_owned(),
    }
}

fn minutes(count: u64) -> String {
    if count == 1 {
        "1 minute".to_owned()
    } else {
        format!("{count} minutes")
    }
}

/// A row named `id` saying `why`, titled `title`, with the one `step`
/// that finds what was missed, raised `at` in milliseconds and `whole`
/// where none of its span was read. No round reads that span again, so
/// only Joel's word marks it done.
#[must_use]
pub fn row(id: &str, title: &str, why: &str, step: &str, at: u64, whole: bool) -> Value {
    json!({
        "id": id,
        "title": title,
        "why": why,
        "whole": whole,
        "raisedAt": slack_people::messages::iso(at),
        "minutes": 2,
        "ready": true,
        "doneWhen": {"by": "person"},
        "steps": [step],
    })
}

/// Adds `row`, raised `at` in milliseconds, to the rows in `file` named
/// for `prefix`, keeping those whose span ended in the [`MISSED_FOR`]
/// before, and any not named for it, as another's to age. Rows of its
/// kind whose spans come within [`MEET`] of its own are said with it as
/// one row, which `say` makes from the span they reach together (its
/// start, if known, and end) and whether that's whole: where together
/// they leave no read stretch, whatever order they're met in. Rows that
/// meet only what it spans once it has taken others are taken too, as a
/// file written before rows met holds. That row is named anew, so one
/// Joel said he'd looked at comes back holding what's new; one not named
/// by [`named`] meets nothing, and one whose start can't be told meets
/// only its own name, so no dated row is lost into it.
///
/// `file` is a file of its own, which a reader's stop doesn't take away
/// as it does the row saying it can't hear, since what it says stays true
/// once the reader hears again or restarts. One not there holds no row;
/// one that can't be read, or holds no rows, is left as it is, so no row
/// it holds is lost. What the row written says is given back, so the
/// journal says what Joel reads.
///
/// # Errors
///
/// Why it couldn't be read or written.
pub fn keep(
    file: &Path,
    prefix: &str,
    row: Value,
    at: u64,
    say: impl FnOnce(Option<u64>, u64, bool) -> Value,
) -> std::io::Result<String> {
    let unread = |why: String| std::io::Error::new(std::io::ErrorKind::InvalidData, why);
    let kept = match std::fs::read(file) {
        Ok(read) => {
            let kept: Value = serde_json::from_slice(&read)
                .map_err(|error| unread(format!("it holds no rows it can read: {error}")))?;
            if !kept["items"].is_array() {
                return Err(unread("it holds no rows".to_owned()));
            }
            Some(kept)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    let recent = |row: &Value| {
        let Some(named) = row["id"].as_str().and_then(|id| id.strip_prefix(prefix)) else {
            return true;
        };
        named
            .rsplit('-')
            .next()
            .and_then(|ended| ended.parse::<u64>().ok())
            .is_some_and(|ended| ended.saturating_add(MISSED_FOR) >= at)
    };
    let mut rows: Vec<Value> = kept
        .as_ref()
        .and_then(|kept| kept["items"].as_array())
        .into_iter()
        .flatten()
        .filter(|kept| recent(kept))
        .cloned()
        .collect();
    let mut own = span_of(&row, prefix);
    let mut taken: Vec<Span> = span_of(&row, prefix).into_iter().collect();
    if let Some(span) = own.as_mut() {
        loop {
            let before = rows.len();
            rows.retain(|earlier| match span_of(earlier, prefix) {
                Some(theirs) if span.meets(&theirs) => {
                    span.reach(&theirs);
                    taken.push(theirs);
                    false
                }
                _ => true,
            });
            if rows.len() == before {
                break;
            }
        }
    }
    let row = match own {
        Some(span) if taken.len() > 1 => say(span.from, span.to, whole(taken)),
        _ => row,
    };
    let why = row["why"].as_str().unwrap_or_default().to_owned();
    rows.push(row);
    let file_of = json!({
        "v": 2,
        "updated": slack_people::messages::iso(at),
        "quest": "pineapple",
        "items": rows,
    });
    make(file, &Change::Tell(file_of)).map(|()| why)
}

/// Whether `spans` met in one row are all never read and leave no read
/// stretch between them, taken from the earliest start on, so the order
/// they were met in says nothing.
fn whole(mut spans: Vec<Span>) -> bool {
    spans.sort_by_key(|span| span.from.unwrap_or(0));
    let mut reached: Option<u64> = None;
    spans.iter().all(|span| {
        let joins = reached.is_none_or(|to| span.from.unwrap_or(0) <= to);
        reached = Some(reached.map_or(span.to, |to| to.max(span.to)));
        span.whole && joins
    })
}

/// The span a row names, read back from its name and fields: its kind,
/// its start, if known, and end, in milliseconds, and whether all of it
/// was never read.
struct Span {
    kind: String,
    from: Option<u64>,
    to: u64,
    whole: bool,
}

impl Span {
    /// Whether `theirs` is the same kind's and comes within [`MEET`];
    /// where either start can't be told, only the same span meets.
    fn meets(&self, theirs: &Span) -> bool {
        self.kind == theirs.kind
            && match (self.from, theirs.from) {
                (Some(own), Some(other)) => {
                    own <= theirs.to.saturating_add(MEET) && other <= self.to.saturating_add(MEET)
                }
                (None, None) => self.to == theirs.to,
                _ => false,
            }
    }

    /// Spans `theirs` too; whether the two are whole is [`whole`]'s.
    fn reach(&mut self, theirs: &Span) {
        self.from = self
            .from
            .zip(theirs.from)
            .map(|(own, other)| own.min(other));
        self.to = self.to.max(theirs.to);
    }
}

/// The span of a row named for `prefix`, read from the end of its name,
/// so a kind holding `-` reads whole; none for a row named otherwise.
fn span_of(row: &Value, prefix: &str) -> Option<Span> {
    let mut parts = row["id"].as_str()?.strip_prefix(prefix)?.rsplitn(3, '-');
    let to = parts.next()?.parse().ok()?;
    let from: u64 = parts.next()?.parse().ok()?;
    let kind = parts.next().filter(|kind| !kind.is_empty())?.to_owned();
    Some(Span {
        kind,
        from: (from != 0).then_some(from),
        to,
        whole: row["whole"].as_bool().unwrap_or(true),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const AT: u64 = 1_795_000_000_000;

    /// A row of `kind`'s from `from` to `to`, under the prefix `p-`.
    fn of(kind: &str, from: u64, to: u64) -> Value {
        let id = named("p-", kind, Some(from), to);
        row(&id, "Missed", kind, "Look", AT, true)
    }

    #[test]
    fn a_kind_holding_dashes_reads_back_whole() {
        let span = span_of(&of("thread-0a", 5, 9), "p-").expect("a span");
        assert_eq!(
            (span.kind.as_str(), span.from, span.to),
            ("thread-0a", Some(5), 9),
            "read from the end of its name"
        );
        assert!(
            span_of(&of("own", 5, 9), "q-").is_none(),
            "one named for another reader has no span here"
        );
    }

    #[test]
    fn rows_of_two_kinds_never_meet_and_one_kinds_do() {
        let home = std::env::temp_dir().join(format!("pineapple-missed-{}", std::process::id()));
        let file = home.join("missed.json");
        let keep_of = |kind: &str, from: u64, to: u64| {
            let say = |from: Option<u64>, to: u64, _: bool| of(kind, from.unwrap_or(0), to);
            keep(&file, "p-", of(kind, from, to), AT, say).expect("written")
        };
        keep_of("thread-0a", AT - 9_000, AT - 8_000);
        keep_of("thread-0b", AT - 9_000, AT - 8_000);
        keep_of("thread-0a", AT - 7_000, AT - 6_000);
        let kept: Value =
            serde_json::from_slice(&std::fs::read(&file).expect("read")).expect("rows");
        std::fs::remove_dir_all(&home).expect("the case's folder goes");
        let ids: Vec<&str> = kept["items"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| row["id"].as_str())
            .collect();
        let (a, b) = (
            format!("p-thread-0a-{}-{}", AT - 9_000, AT - 6_000),
            format!("p-thread-0b-{}-{}", AT - 9_000, AT - 8_000),
        );
        assert_eq!(
            ids,
            [b.as_str(), a.as_str()],
            "one thread's spans are one row, and another's stays its own: {kept}"
        );
    }

    #[test]
    fn a_file_it_cant_read_is_left_and_a_week_old_row_goes_but_anothers_stays() {
        let home =
            std::env::temp_dir().join(format!("pineapple-missed-kept-{}", std::process::id()));
        std::fs::create_dir_all(&home).expect("the case's folder");
        let file = home.join("missed.json");
        let say = |from: Option<u64>, to: u64, _: bool| of("own", from.unwrap_or(0), to);
        let new = of("own", AT - 2_000, AT - 1_000);
        for (held, why) in [
            ("not rows", "rows it can read"),
            (r#"{"v": 2}"#, "holds no rows"),
        ] {
            std::fs::write(&file, held).expect("a file it can't take rows from");
            let refused =
                keep(&file, "p-", new.clone(), AT, say).map_err(|error| error.to_string());
            let left = std::fs::read_to_string(&file).expect("still there");
            assert!(
                refused.as_ref().is_err_and(|error| error.contains(why)) && left == held,
                "one holding {held} is refused and left as it is: {refused:?}, {left}"
            );
        }
        let ended = AT - MISSED_FOR - 1;
        let theirs = row(
            &named("q-", "own", Some(ended - 9_000), ended),
            "Missed",
            "Theirs",
            "Look",
            AT,
            true,
        );
        let held = [
            of("own", ended - 9_000, ended),
            theirs,
            of("thread-0b", AT - 9_000, AT),
        ];
        std::fs::write(&file, json!({ "items": held }).to_string()).expect("rows written");
        keep(&file, "p-", new, AT, say).expect("written");
        let kept: Value =
            serde_json::from_slice(&std::fs::read(&file).expect("read")).expect("rows");
        std::fs::remove_dir_all(&home).expect("the case's folder goes");
        let ids: Vec<&str> = kept["items"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| row["id"].as_str())
            .collect();
        assert_eq!(
            ids,
            [
                named("q-", "own", Some(ended - 9_000), ended),
                named("p-", "thread-0b", Some(AT - 9_000), AT),
                named("p-", "own", Some(AT - 2_000), AT - 1_000),
            ],
            "its own a week past ended go, another reader's are its own to age: {kept}"
        );
    }
}
