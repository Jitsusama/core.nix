//! The files Chrome downloaded, for its Recent in the launcher: each one
//! still where Chrome saved it, newest first, at most nine, read from the
//! downloads table of its history, which keeps a row for every download
//! until its history is cleared, whether or not the file is still there.
//!
//! Only a finished download is a file to open: one Chrome is still
//! writing, or that was cancelled or cut off, is left out, as is one whose
//! file has since been moved or removed. A file downloaded twice to one
//! path is listed once, as its newest. Its size is the file's as it is
//! now, and when is when Chrome finished it, or else when it started.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::{LOOKED_AT, MOST, since_1970};

/// A download Chrome finished, as its history's `state` says it.
const COMPLETE: i64 = 1;

#[derive(Debug, PartialEq, Eq)]
pub struct Download {
    pub path: PathBuf,
    /// Its size in bytes, as the file is now.
    pub size: u64,
    /// When it finished, in seconds since 1970, where Chrome says.
    pub when: Option<i64>,
    /// The address of the page it was downloaded from, or of whatever sent
    /// it there, or nothing where Chrome kept neither.
    pub from: String,
}

impl Download {
    pub fn line(&self) -> String {
        let when = self.when.map(|at| at.to_string()).unwrap_or_default();
        format!(
            "{}\t{}\t{when}\t{}",
            self.path.display(),
            self.size,
            self.from
        )
    }
}

/// A regular file's size, or none where nothing is there to open.
pub fn on_disk(path: &Path) -> Option<u64> {
    fs::metadata(path)
        .ok()
        .filter(fs::Metadata::is_file)
        .map(|file| file.len())
}

/// The files a history says Chrome downloaded, newest first, each one
/// `size` finds still there, at most nine.
pub fn downloads(
    history: &Connection,
    size: impl Fn(&Path) -> Option<u64>,
) -> rusqlite::Result<Vec<Download>> {
    let mut asked = history.prepare(
        "SELECT target_path, end_time, start_time, tab_url, referrer FROM downloads
         WHERE state = ?1 AND target_path <> ''
         ORDER BY CASE WHEN end_time > 0 THEN end_time ELSE start_time END DESC LIMIT ?2",
    )?;
    let mut seen = HashSet::new();
    let mut found = Vec::new();
    let mut rows = asked.query((COMPLETE, LOOKED_AT))?;
    while let Some(row) = rows.next()? {
        let path = PathBuf::from(row.get::<_, String>(0)?);
        let (ended, started): (i64, i64) = (row.get(1)?, row.get(2)?);
        let (page, referrer): (Option<String>, Option<String>) = (row.get(3)?, row.get(4)?);
        if !seen.insert(path.clone()) {
            continue;
        }
        let Some(size) = size(&path) else {
            continue;
        };
        let from = [page, referrer]
            .into_iter()
            .flatten()
            .find(|address| !address.is_empty())
            .unwrap_or_default();
        found.push(Download {
            path,
            size,
            when: since_1970(ended).or_else(|| since_1970(started)),
            from,
        });
        if found.len() == MOST {
            break;
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::from_history;
    use std::env;

    fn folder(name: &str) -> PathBuf {
        let folder = env::temp_dir().join(format!(
            "chrome-recent-sites-test-{}-downloads-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    /// Chrome's microseconds since 1601 for a moment in seconds since 1970.
    fn chrome_time(seconds: i64) -> i64 {
        (seconds + 11_644_473_600) * 1_000_000
    }

    /// One download, as its history row holds it.
    struct Row<'a> {
        path: &'a str,
        state: i64,
        started: i64,
        ended: i64,
        page: &'a str,
        referrer: &'a str,
    }

    fn finished<'a>(path: &'a str, ended: i64, page: &'a str) -> Row<'a> {
        Row {
            path,
            state: COMPLETE,
            started: ended - 5,
            ended,
            page,
            referrer: "",
        }
    }

    /// A history with the downloads table as Chrome writes it, its columns
    /// as the current schema has them, holding these rows.
    fn history(at: &Path, rows: &[Row]) {
        let connection = Connection::open(at).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE downloads (id INTEGER PRIMARY KEY, guid VARCHAR NOT NULL,
                 current_path LONGVARCHAR NOT NULL, target_path LONGVARCHAR NOT NULL,
                 start_time INTEGER NOT NULL, received_bytes INTEGER NOT NULL,
                 total_bytes INTEGER NOT NULL, state INTEGER NOT NULL,
                 danger_type INTEGER NOT NULL, interrupt_reason INTEGER NOT NULL,
                 hash BLOB NOT NULL, end_time INTEGER NOT NULL, opened INTEGER NOT NULL,
                 last_access_time INTEGER NOT NULL, transient INTEGER NOT NULL,
                 referrer VARCHAR NOT NULL, site_url VARCHAR NOT NULL,
                 embedder_download_data VARCHAR NOT NULL, tab_url VARCHAR NOT NULL,
                 tab_referrer_url VARCHAR NOT NULL, http_method VARCHAR NOT NULL,
                 by_ext_id VARCHAR NOT NULL, by_ext_name VARCHAR NOT NULL,
                 by_web_app_id VARCHAR NOT NULL, etag VARCHAR NOT NULL,
                 last_modified VARCHAR NOT NULL, mime_type VARCHAR(255) NOT NULL,
                 original_mime_type VARCHAR(255) NOT NULL)",
            )
            .unwrap();
        for (n, row) in rows.iter().enumerate() {
            let ended = if row.ended == 0 {
                0
            } else {
                chrome_time(row.ended)
            };
            connection
                .execute(
                    "INSERT INTO downloads VALUES (NULL, ?1, ?2, ?2, ?3, 0, 0, ?4, 0, 0, x'', ?5,
                     0, 0, 0, ?6, '', '', ?7, '', 'GET', '', '', '', '', '', '', '')",
                    (
                        format!("guid-{n}"),
                        row.path,
                        chrome_time(row.started),
                        row.state,
                        ended,
                        row.referrer,
                        row.page,
                    ),
                )
                .unwrap();
        }
    }

    /// A file of `bytes` bytes at `path`.
    fn file(path: &Path, bytes: usize) {
        fs::write(path, vec![b'x'; bytes]).unwrap();
    }

    fn named(found: &[Download]) -> Vec<String> {
        found
            .iter()
            .map(|one| one.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn each_file_still_there_newest_first_with_its_size_when_and_where_from() {
        let profile = folder("there");
        let at = |name: &str| profile.join(name).display().to_string();
        let (report, older, gone) = (at("report.pdf"), at("older.zip"), at("gone.iso"));
        file(&profile.join("report.pdf"), 2048);
        file(&profile.join("older.zip"), 10);
        history(
            &profile.join("History"),
            &[
                finished(&older, 1_800_000_000, "https://files.example.org/"),
                finished(&gone, 1_800_000_200, "https://gone.example.org/"),
                finished(&report, 1_800_000_100, "https://docs.example.com/q3"),
            ],
        );
        let found =
            from_history(&profile, &profile, |history| downloads(history, on_disk)).unwrap();
        assert_eq!(
            found.iter().map(Download::line).collect::<Vec<_>>(),
            [
                format!("{report}\t2048\t1800000100\thttps://docs.example.com/q3"),
                format!("{older}\t10\t1800000000\thttps://files.example.org/"),
            ]
        );
    }

    #[test]
    fn only_a_finished_download_is_listed_and_a_path_once_as_its_newest() {
        let profile = folder("finished");
        let at = |name: &str| profile.join(name).display().to_string();
        let (writing, cancelled, cut, twice) =
            (at("writing"), at("cancelled"), at("cut"), at("twice"));
        for name in ["writing", "cancelled", "cut", "twice"] {
            file(&profile.join(name), 1);
        }
        let unfinished = |path, state| Row {
            path,
            state,
            started: 1_800_000_300,
            ended: 0,
            page: "",
            referrer: "",
        };
        history(
            &profile.join("History"),
            &[
                unfinished(&writing, 0),
                unfinished(&cancelled, 2),
                unfinished(&cut, 4),
                finished(&twice, 1_800_000_010, "https://old.example.org/"),
                finished(&twice, 1_800_000_020, "https://new.example.org/"),
            ],
        );
        let found =
            from_history(&profile, &profile, |history| downloads(history, on_disk)).unwrap();
        assert_eq!(named(&found), ["twice"]);
        assert_eq!(found[0].from, "https://new.example.org/");
    }

    #[test]
    fn with_no_finish_kept_its_start_says_when_and_a_referrer_where_from() {
        let profile = folder("started");
        let path = profile.join("started.txt").display().to_string();
        file(&profile.join("started.txt"), 3);
        history(
            &profile.join("History"),
            &[Row {
                path: &path,
                state: COMPLETE,
                started: 1_800_000_000,
                ended: 0,
                page: "",
                referrer: "https://sent.example.org/",
            }],
        );
        let found =
            from_history(&profile, &profile, |history| downloads(history, on_disk)).unwrap();
        assert_eq!(found[0].when, Some(1_800_000_000));
        assert_eq!(found[0].from, "https://sent.example.org/");
    }

    #[test]
    fn nine_at_most_and_a_folder_where_a_file_was_is_no_file() {
        let profile = folder("nine");
        let paths: Vec<String> = (0..12)
            .map(|n| profile.join(format!("file{n}")).display().to_string())
            .collect();
        for (n, path) in paths.iter().enumerate() {
            if n == 11 {
                fs::create_dir_all(path).unwrap();
            } else {
                file(Path::new(path), n);
            }
        }
        let rows: Vec<Row> = paths
            .iter()
            .zip(0..)
            .map(|(path, n)| finished(path, 1_800_000_000 + n, ""))
            .collect();
        history(&profile.join("History"), &rows);
        let found =
            from_history(&profile, &profile, |history| downloads(history, on_disk)).unwrap();
        assert_eq!(found.len(), 9);
        assert_eq!(named(&found)[0], "file10");
        assert_eq!(found[0].size, 10);
        assert_eq!(found[0].from, "");
    }

    #[test]
    fn a_profile_chrome_never_ran_has_none() {
        let profile = folder("none");
        assert_eq!(
            from_history(&profile, &profile, |history| downloads(history, on_disk)).unwrap(),
            []
        );
    }
}
