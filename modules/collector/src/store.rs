//! Where spans are kept: one SQLite file for each hour they began in.
//!
//! An hour is dropped by deleting its file, so keeping within the bound
//! costs nothing and never vacuums. The oldest hours go first, by age once
//! past the hours kept and by size once the files pass the bytes kept.

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, params};
use serde_json::Value;

use crate::otlp::Span;

const HOUR: i64 = 3_600 * 1_000_000_000;

/// How much the store keeps.
#[derive(Debug, Clone, Copy)]
pub struct Keep {
    pub hours: i64,
    pub bytes: u64,
}

impl Keep {
    /// Seven days of spans, in at most 2 GiB.
    pub const USUAL: Keep = Keep {
        hours: 7 * 24,
        bytes: 2 << 30,
    };
}

pub struct Store {
    dir: PathBuf,
    keep: Keep,
    /// The hour last written to, and its file held open.
    open: Option<(i64, Connection)>,
}

impl Store {
    /// The store in `dir`, made if it isn't there.
    ///
    /// # Errors
    ///
    /// When the folder can't be made.
    pub fn new(dir: &Path, keep: Keep) -> Result<Self, String> {
        fs::create_dir_all(dir).map_err(|e| format!("making {}: {e}", dir.display()))?;
        Ok(Store {
            dir: dir.to_owned(),
            keep,
            open: None,
        })
    }

    /// Keeps the spans, each in the file of the hour it began, then drops
    /// what's past what's kept, counting from `now`.
    ///
    /// # Errors
    ///
    /// When an hour's file can't be written.
    pub fn add(&mut self, spans: &[Span], now: i64) -> Result<(), String> {
        let mut hours: Vec<i64> = spans.iter().map(|s| s.start.div_euclid(HOUR)).collect();
        hours.sort_unstable();
        hours.dedup();
        for hour in hours {
            let connection = self.writing(hour)?;
            let transaction = connection.unchecked_transaction().map_err(sql)?;
            for span in spans.iter().filter(|s| s.start.div_euclid(HOUR) == hour) {
                transaction
                    .execute(
                        "INSERT OR REPLACE INTO spans \
                         (trace, span, parent, name, service, start, end, status, attributes) \
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                        params![
                            span.trace,
                            span.id,
                            span.parent,
                            span.name,
                            span.service,
                            span.start,
                            span.end,
                            span.status,
                            Value::Object(span.attributes.clone()).to_string(),
                        ],
                    )
                    .map_err(sql)?;
            }
            transaction.commit().map_err(sql)?;
        }
        self.expire(now);
        Ok(())
    }

    fn writing(&mut self, hour: i64) -> Result<&Connection, String> {
        if self.open.as_ref().is_none_or(|(open, _)| *open != hour) {
            let connection = Connection::open(self.file(hour)).map_err(sql)?;
            connection
                .execute_batch(
                    "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; \
                     CREATE TABLE IF NOT EXISTS spans (trace TEXT NOT NULL, span TEXT NOT NULL, \
                       parent TEXT, name TEXT NOT NULL, service TEXT NOT NULL, \
                       start INTEGER NOT NULL, end INTEGER NOT NULL, status TEXT, \
                       attributes TEXT NOT NULL, PRIMARY KEY (trace, span)); \
                     CREATE INDEX IF NOT EXISTS spans_by_start ON spans (start);",
                )
                .map_err(sql)?;
            self.open = Some((hour, connection));
        }
        Ok(&self.open.as_ref().expect("just opened").1)
    }

    fn file(&self, hour: i64) -> PathBuf {
        self.dir.join(format!("spans-{hour}.sqlite"))
    }

    /// The hours there are files for, oldest first.
    fn hours(&self) -> Vec<i64> {
        let mut hours: Vec<i64> = fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().into_string().ok()?;
                name.strip_prefix("spans-")?
                    .strip_suffix(".sqlite")?
                    .parse()
                    .ok()
            })
            .collect();
        hours.sort_unstable();
        hours
    }

    /// An hour's bytes, its write-ahead log and index beside it.
    fn size(&self, hour: i64) -> u64 {
        let file = self.file(hour);
        ["", "-wal", "-shm"]
            .iter()
            .filter_map(|end| fs::metadata(format!("{}{end}", file.display())).ok())
            .map(|meta| meta.len())
            .sum()
    }

    fn drop_hour(&mut self, hour: i64) {
        if self.open.as_ref().is_some_and(|(open, _)| *open == hour) {
            self.open = None;
        }
        let file = self.file(hour);
        for end in ["", "-wal", "-shm"] {
            let _ = fs::remove_file(format!("{}{end}", file.display()));
        }
    }

    /// Drops the hours older than those kept, then the oldest while the
    /// files pass the bytes kept, never the newest.
    pub fn expire(&mut self, now: i64) {
        let oldest = now.div_euclid(HOUR) - self.keep.hours;
        let mut hours = self.hours();
        for hour in hours.iter().copied().filter(|h| *h < oldest) {
            self.drop_hour(hour);
        }
        hours.retain(|h| *h >= oldest);
        let mut total: u64 = hours.iter().map(|h| self.size(*h)).sum();
        while total > self.keep.bytes && hours.len() > 1 {
            let hour = hours.remove(0);
            total -= self.size(hour);
            self.drop_hour(hour);
        }
    }

    /// Every span open at any time from `from` to `to`, by when it began.
    ///
    /// A span is looked for an hour before the one `from` is in, so one that
    /// began earlier and was still going is found.
    ///
    /// # Errors
    ///
    /// When a file can't be read.
    pub fn during(&self, from: i64, to: i64) -> Result<Vec<Span>, String> {
        let first = from.div_euclid(HOUR) - 1;
        let last = to.div_euclid(HOUR);
        let mut found = Vec::new();
        for hour in self
            .hours()
            .into_iter()
            .filter(|h| (first..=last).contains(h))
        {
            found.extend(self.read(hour, "WHERE start <= ?1 AND end >= ?2", params![to, from])?);
        }
        found.sort_by_key(|s| (s.start, s.end));
        Ok(found)
    }

    /// Every span of one trace, by when it began.
    ///
    /// # Errors
    ///
    /// When a file can't be read.
    pub fn trace(&self, trace: &str) -> Result<Vec<Span>, String> {
        let trace = trace.to_ascii_lowercase();
        let mut found = Vec::new();
        for hour in self.hours() {
            found.extend(self.read(hour, "WHERE trace = ?1", params![trace])?);
        }
        found.sort_by_key(|s| (s.start, s.end));
        Ok(found)
    }

    fn read(
        &self,
        hour: i64,
        filter: &str,
        values: impl rusqlite::Params,
    ) -> Result<Vec<Span>, String> {
        let connection =
            Connection::open_with_flags(self.file(hour), OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(sql)?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT trace, span, parent, name, service, start, end, status, attributes \
                 FROM spans {filter}"
            ))
            .map_err(sql)?;
        let rows = statement
            .query_map(values, |row| {
                let attributes: String = row.get(8)?;
                Ok(Span {
                    trace: row.get(0)?,
                    id: row.get(1)?,
                    parent: row.get(2)?,
                    name: row.get(3)?,
                    service: row.get(4)?,
                    start: row.get(5)?,
                    end: row.get(6)?,
                    status: row.get(7)?,
                    attributes: serde_json::from_str(&attributes).unwrap_or_default(),
                })
            })
            .map_err(sql)?;
        rows.collect::<Result<_, _>>().map_err(sql)
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "map_err hands the error over"
)]
fn sql(error: rusqlite::Error) -> String {
    format!("the store: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(id: &str, start: i64, end: i64) -> Span {
        Span {
            trace: "4bf92f3577b34da6a3ce929d0e0e4736".to_owned(),
            id: id.repeat(16)[..16].to_owned(),
            parent: None,
            name: format!("step {id}"),
            service: "launcher".to_owned(),
            start,
            end,
            status: None,
            attributes: serde_json::Map::new(),
        }
    }

    #[test]
    fn a_moment_finds_every_span_open_then_even_one_begun_the_hour_before() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::new(dir.path(), Keep::USUAL).unwrap();
        let noon = 500_000 * HOUR;
        store
            .add(
                &[
                    span("a", noon - 10 * 60 * 1_000_000_000, noon + 5),
                    span("b", noon + 1, noon + 2),
                    span("c", noon + 100, noon + 200),
                ],
                noon,
            )
            .unwrap();
        let names: Vec<_> = store
            .during(noon + 1, noon + 3)
            .unwrap()
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert_eq!(names, ["step a", "step b"]);
        assert_eq!(
            store
                .trace("4BF92F3577B34DA6A3CE929D0E0E4736")
                .unwrap()
                .len(),
            3
        );
    }

    #[test]
    fn hours_past_those_kept_are_dropped_whole() {
        let dir = tempfile::tempdir().unwrap();
        let keep = Keep {
            hours: 2,
            bytes: u64::MAX,
        };
        let mut store = Store::new(dir.path(), keep).unwrap();
        let now = 500_000 * HOUR;
        store
            .add(
                &[span("a", now - 3 * HOUR, now - 3 * HOUR + 1)],
                now - 3 * HOUR,
            )
            .unwrap();
        store
            .add(&[span("b", now - HOUR, now - HOUR + 1)], now)
            .unwrap();
        assert_eq!(store.hours(), [500_000 - 1]);
    }

    #[test]
    fn past_the_bytes_kept_the_oldest_hours_go_but_never_the_newest() {
        let dir = tempfile::tempdir().unwrap();
        let keep = Keep {
            hours: 1_000,
            bytes: 1,
        };
        let mut store = Store::new(dir.path(), keep).unwrap();
        let now = 500_000 * HOUR;
        for back in (0..3).rev() {
            let start = now - back * HOUR;
            store.add(&[span("a", start, start + 1)], now).unwrap();
        }
        assert_eq!(store.hours(), [500_000]);
        assert_eq!(store.during(now, now + 1).unwrap().len(), 1);
    }
}
