//! Joel's visits in Chrome, from each profile's History: when each visit
//! began and how it came about (a reload, back or forward, anything
//! else), never where. A run of reloads a minute apart says a page
//! wouldn't do what he wanted, and going back within seconds says a page
//! wasn't the one he meant, so both rank beside the launcher's moments.
//!
//! Chrome holds its history under a lock while it writes it, so the file's
//! bytes are read into memory and opened there, read-only, and never
//! copied to disk. A read that fails or finds the file torn is passed over
//! until the next run. The one question asked of it, `ASKED`, names two
//! columns of its visits and nothing else: no URL, title, search term or
//! other table is ever read, so none can be kept or sent.

use crate::pain::{ChromeSigns, Painful, Signs, float, tenths};
use rusqlite::{Connection, MAIN_DB};
use std::fs::File;
use std::path::Path;

/// The only question put to a history.
pub const ASKED: &str =
    "SELECT visit_time, transition FROM visits WHERE visit_time >= ?1 ORDER BY visit_time";

/// Chrome counts time in microseconds from 1601; this is 1970 on it.
const FROM_1601_MS: i64 = 11_644_473_600_000;
/// A transition's low byte says how a visit came about; 8 is a reload.
const CORE: i64 = 0xFF;
const RELOAD: i64 = 8;
/// The qualifier Chrome sets on a visit made by back or forward.
const FORWARD_BACK: i64 = 0x0100_0000;
/// A reload within this of the last is the same try again, as with a
/// search he gives up on in the launcher.
pub const AGAIN_MS: i64 = 60_000;
/// Back within this of reaching a page says it wasn't the one.
pub const BACK_MS: i64 = 10_000;
/// A go lost him this at most, as in the launcher and pi.
const MOST_MS: i64 = 120_000;

/// One visit: when it began, in milliseconds since the epoch, and how.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visit {
    pub at: i64,
    pub transition: i64,
}

impl Visit {
    fn reload(self) -> bool {
        self.transition & CORE == RELOAD
    }

    fn back(self) -> bool {
        self.transition & FORWARD_BACK != 0
    }
}

/// The visits a profile's History holds from `since` on, oldest first.
pub fn visits(history: &Path, since: i64) -> Result<Vec<Visit>, String> {
    let fault = |e: &dyn std::fmt::Display| format!("{}: {e}", history.display());
    let mut file = File::open(history).map_err(|e| fault(&e))?;
    let size = file.metadata().map_err(|e| fault(&e))?.len();
    let size = usize::try_from(size).map_err(|e| fault(&e))?;
    let mut db = Connection::open_in_memory().map_err(|e| fault(&e))?;
    db.deserialize_read_exact(MAIN_DB, &mut file, size, true)
        .map_err(|e| fault(&e))?;
    let mut asked = db.prepare(ASKED).map_err(|e| fault(&e))?;
    let rows = asked
        .query_map([(since + FROM_1601_MS) * 1000], |row| {
            Ok(Visit {
                at: row.get::<_, i64>(0)? / 1000 - FROM_1601_MS,
                transition: row.get(1)?,
            })
        })
        .map_err(|e| fault(&e))?;
    rows.collect::<Result<_, _>>().map_err(|e| fault(&e))
}

/// The day's goes in Chrome that lost him at least a second: each run of
/// reloads, and each visit he went back from within seconds. `profiles`
/// are each profile's visits, oldest first; only goes begun in [since,
/// until) count.
pub fn painful(profiles: &[Vec<Visit>], since: i64, until: i64) -> Vec<Painful> {
    let _ = (profiles, since, until, AGAIN_MS, BACK_MS, MOST_MS);
    let _ = (float(0), tenths(0.0), ChromeSigns::default());
    let _: Option<Signs> = None;
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-11-11 16:00 UTC.
    const AT: i64 = 1_794_412_800_000;
    const LINK: i64 = 0x3000_0000;
    const RELOADED: i64 = 0x3000_0008;
    const BACK: i64 = 0x3100_0000;

    fn visit(at: i64, transition: i64) -> Visit {
        Visit { at, transition }
    }

    /// A history as Chrome lays it out, as far as these columns go, with a
    /// URL and a title beside each visit that must never be read.
    pub(crate) fn history(at: &Path, visits: &[Visit]) {
        let db = Connection::open(at).expect("a history");
        db.execute_batch(
            "CREATE TABLE urls (id INTEGER PRIMARY KEY, url LONGVARCHAR, title LONGVARCHAR);
             CREATE TABLE visits (id INTEGER PRIMARY KEY, url INTEGER NOT NULL,
               visit_time INTEGER NOT NULL, from_visit INTEGER, transition INTEGER NOT NULL,
               segment_id INTEGER, visit_duration INTEGER DEFAULT 0 NOT NULL,
               opener_visit INTEGER);
             INSERT INTO urls VALUES (1, 'https://FAKE-7QEY1S.example/secret', 'FAKE title');",
        )
        .expect("the tables");
        for v in visits {
            db.execute(
                "INSERT INTO visits (url, visit_time, transition) VALUES (1, ?1, ?2)",
                [(v.at + FROM_1601_MS) * 1000, v.transition],
            )
            .expect("a visit");
        }
    }

    #[test]
    fn the_history_is_asked_for_two_columns_of_its_visits_alone() {
        let words: Vec<&str> = ASKED
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|w| !w.is_empty())
            .collect();
        let allowed = [
            "SELECT", "visit_time", "transition", "FROM", "visits", "WHERE", "1", "ORDER", "BY",
        ];
        for word in &words {
            assert!(allowed.contains(word), "the history is asked for {word}");
        }
    }

    #[test]
    fn visits_read_from_a_history_in_memory_say_when_and_how() {
        let folder = std::env::temp_dir().join(format!("pain-points-chrome-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("a scratch folder");
        let at = folder.join("History");
        history(&at, &[visit(AT - 5000, LINK), visit(AT, LINK), visit(AT + 9000, RELOADED)]);
        let read = visits(&at, AT);
        std::fs::remove_dir_all(&folder).expect("the scratch folder goes");
        assert_eq!(
            read,
            Ok(vec![visit(AT, LINK), visit(AT + 9000, RELOADED)]),
            "visits from the day on, in Chrome's own microseconds"
        );
        assert!(
            visits(Path::new("/no/such/History"), AT).is_err(),
            "a history that isn't there is passed over"
        );
    }

    #[test]
    fn reloads_a_minute_apart_are_one_go_that_loses_its_time() {
        let day = [
            visit(AT - 2000, LINK),
            visit(AT, RELOADED),
            visit(AT + 20_000, RELOADED),
            visit(AT + 50_000, RELOADED),
            visit(AT + 200_000, RELOADED),
        ];
        let named = painful(&[day.to_vec()], AT - 10_000, AT + 1);
        assert_eq!(named.len(), 1, "one go, and a lone reload isn't one");
        let p = &named[0];
        assert_eq!((p.place, p.trying.as_str()), ("chrome", "a page"));
        assert_eq!((p.at, p.lost_seconds, p.seconds), (AT / 1000, 50.0, 50.0));
        assert_eq!((p.slowed, p.part, p.openings), ("reloading", "reload", 3));
        assert_eq!(
            p.signs,
            Signs::Chrome(ChromeSigns {
                profile: 0,
                reloads: 3,
                backs: 0
            })
        );
    }

    #[test]
    fn a_visit_between_reloads_ends_the_go() {
        let day = [
            visit(AT, RELOADED),
            visit(AT + 10_000, LINK),
            visit(AT + 20_000, RELOADED),
        ];
        assert!(
            painful(&[day.to_vec()], AT - 1, AT + 1).is_empty(),
            "he went somewhere between, so each reload stands alone"
        );
    }

    #[test]
    fn going_back_within_seconds_loses_the_time_on_the_page() {
        let day = [
            visit(AT, LINK),
            visit(AT + 4000, BACK),
            visit(AT + 30_000, LINK),
            visit(AT + 50_000, BACK),
            visit(AT + 60_000, LINK),
            visit(AT + 60_500, BACK),
        ];
        let named = painful(&[Vec::new(), day.to_vec()], AT - 1, AT + 100_000);
        assert_eq!(
            named.len(),
            1,
            "one back within ten seconds; 20 s on a page was reading it, half a second a redirect"
        );
        let p = &named[0];
        assert_eq!((p.at, p.lost_seconds), (AT / 1000, 4.0));
        assert_eq!((p.slowed, p.part, p.openings), ("going back", "back", 1));
        assert_eq!(
            p.signs,
            Signs::Chrome(ChromeSigns {
                profile: 1,
                reloads: 0,
                backs: 1
            }),
            "the profile by its place among them"
        );
    }

    #[test]
    fn a_go_loses_two_minutes_at_most_and_counts_on_the_day_it_began() {
        let day: Vec<Visit> = (0..6).map(|n| visit(AT + n * 50_000, RELOADED)).collect();
        assert_eq!(
            painful(&[day.clone()], AT - 1, AT + 1)[0].lost_seconds,
            120.0
        );
        assert!(painful(&[day], AT + 1, AT + 2).is_empty(), "begun before the day");
    }
}
