//! The sites Chrome visited lately, for Chrome's Recent inside the launcher:
//! one a site, the newest titled page of each, newest first, at most nine.
//!
//! `chrome-recent-sites [PROFILE]` reads the History of PROFILE, Chrome's
//! Default profile when none is given, and prints a line a site: the page's
//! address, a tab and its title. Chrome writes its history as it browses, so
//! this reads a copy of it and its journal, which SQLite rolls back to the
//! last whole write, and never the file Chrome holds. A profile with no
//! history prints nothing. Google's own windows (Mail, Calendar, Meet,
//! Drive and the documents) are left out, since their programs keep their
//! own, as are sign-in pages and this machine's own addresses.

use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rusqlite::Connection;

/// As many as 1 to 9 can pick.
const MOST: usize = 9;

/// How far back it looks for nine sites.
const LOOKED_AT: u32 = 5000;

/// Sites left out: Google's own windows, which are programs of their own,
/// signing in, and this machine.
const LEFT_OUT: [&str; 9] = [
    "mail.google.com",
    "calendar.google.com",
    "meet.google.com",
    "drive.google.com",
    "docs.google.com",
    "accounts.google.com",
    "localhost",
    "127.0.0.1",
    "::1",
];

#[derive(Debug, PartialEq, Eq)]
struct Site {
    address: String,
    title: String,
}

/// The host a web address names, or none when it isn't one.
fn host(address: &str) -> Option<&str> {
    let rest = address.strip_prefix("https://").or_else(|| address.strip_prefix("http://"))?;
    let authority = &rest[..rest.find(['/', '?', '#']).unwrap_or(rest.len())];
    let authority = authority.rsplit('@').next().unwrap_or(authority);
    let host = match authority.strip_prefix('[') {
        Some(bracketed) => &bracketed[..bracketed.find(']')?],
        None => authority.split(':').next().unwrap_or(authority),
    };
    (!host.is_empty()).then_some(host)
}

/// The newest titled page of each site in a history, newest first.
fn sites(history: &Connection) -> rusqlite::Result<Vec<Site>> {
    let mut asked = history.prepare(
        "SELECT url, title FROM urls
         WHERE hidden = 0 AND title <> '' AND (url LIKE 'https://%' OR url LIKE 'http://%')
         ORDER BY last_visit_time DESC LIMIT ?1",
    )?;
    let mut seen = HashSet::new();
    let mut found = Vec::new();
    let mut rows = asked.query([LOOKED_AT])?;
    while let Some(row) = rows.next()? {
        let address: String = row.get(0)?;
        let title: String = row.get(1)?;
        let Some(host) = host(&address).map(str::to_ascii_lowercase) else {
            continue;
        };
        if LEFT_OUT.contains(&host.as_str()) || !seen.insert(host) {
            continue;
        }
        // A line a site, so nothing in a title may break one.
        let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
        found.push(Site { address, title });
        if found.len() == MOST {
            break;
        }
    }
    Ok(found)
}

/// Reads a profile's sites from a copy of its history, made in `scratch`
/// and gone once read.
fn read(profile: &Path, scratch: &Path) -> Result<Vec<Site>, String> {
    let history = profile.join("History");
    if !history.exists() {
        return Ok(Vec::new());
    }
    let folder = scratch.join(format!("chrome-recent-sites-{}", std::process::id()));
    fs::create_dir_all(&folder).map_err(|error| format!("{}: {error}", folder.display()))?;
    let read = (|| {
        let copy = folder.join("History");
        fs::copy(&history, &copy).map_err(|error| format!("{}: {error}", history.display()))?;
        let journal = profile.join("History-journal");
        if journal.exists() {
            fs::copy(&journal, folder.join("History-journal")).map_err(|error| format!("{}: {error}", journal.display()))?;
        }
        let connection = Connection::open(&copy).map_err(|error| error.to_string())?;
        sites(&connection).map_err(|error| error.to_string())
    })();
    let _ = fs::remove_dir_all(&folder);
    read
}

fn main() -> ExitCode {
    let profile = match env::args_os().nth(1) {
        Some(profile) => PathBuf::from(profile),
        None => PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".config/google-chrome/Default"),
    };
    let scratch = env::var_os("XDG_RUNTIME_DIR").map_or_else(env::temp_dir, PathBuf::from);
    let printed = read(&profile, &scratch).and_then(|found| {
        let mut out = io::stdout().lock();
        found
            .iter()
            .try_for_each(|site| writeln!(out, "{}\t{}", site.address, site.title))
            .map_err(|error| error.to_string())
    });
    match printed {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("chrome-recent-sites: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let folder = env::temp_dir().join(format!("chrome-recent-sites-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    /// A history with Chrome's urls table, each page (address, title,
    /// visited, hidden) in it.
    fn history(at: &Path, pages: &[(&str, &str, i64, i64)]) {
        let connection = Connection::open(at).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE urls(id INTEGER PRIMARY KEY, url LONGVARCHAR, title LONGVARCHAR,
                 visit_count INTEGER DEFAULT 0, typed_count INTEGER DEFAULT 0,
                 last_visit_time INTEGER NOT NULL, hidden INTEGER DEFAULT 0)",
            )
            .unwrap();
        for (address, title, visited, hidden) in pages {
            connection
                .execute(
                    "INSERT INTO urls(url, title, last_visit_time, hidden) VALUES (?1, ?2, ?3, ?4)",
                    (address, title, visited, hidden),
                )
                .unwrap();
        }
    }

    fn titles(found: &[Site]) -> Vec<&str> {
        found.iter().map(|site| site.title.as_str()).collect()
    }

    #[test]
    fn each_site_once_by_its_newest_page_newest_first() {
        let profile = folder("once");
        history(
            &profile.join("History"),
            &[
                ("https://example.org/old", "Old page", 10, 0),
                ("https://news.example.com/", "News", 30, 0),
                ("https://example.org/new", "New page", 40, 0),
                ("http://plain.example.net:8080/a?b", "Plain", 20, 0),
            ],
        );
        assert_eq!(titles(&read(&profile, &profile).unwrap()), ["New page", "News", "Plain"]);
    }

    #[test]
    fn what_has_its_own_program_or_no_title_or_isnt_the_web_is_left_out() {
        let profile = folder("left-out");
        history(
            &profile.join("History"),
            &[
                ("https://mail.google.com/mail/u/0/", "Inbox", 90, 0),
                ("https://docs.google.com/document/d/x", "A document", 80, 0),
                ("https://accounts.google.com/signin", "Sign in", 70, 0),
                ("http://localhost:3000/", "Local", 60, 0),
                ("https://hidden.example.com/", "Hidden", 50, 1),
                ("https://untitled.example.com/", "", 40, 0),
                ("chrome://settings/", "Settings", 30, 0),
                ("https://www.google.com/search?q=x", "x - Google Search", 20, 0),
            ],
        );
        assert_eq!(titles(&read(&profile, &profile).unwrap()), ["x - Google Search"]);
    }

    #[test]
    fn nine_at_most() {
        let profile = folder("nine");
        let addresses: Vec<String> = (0..12).map(|n| format!("https://site{n}.example.com/")).collect();
        let pages: Vec<(&str, &str, i64, i64)> =
            addresses.iter().enumerate().map(|(n, address)| (address.as_str(), "A page", n as i64, 0)).collect();
        history(&profile.join("History"), &pages);
        let found = read(&profile, &profile).unwrap();
        assert_eq!(found.len(), 9);
        assert_eq!(found[0].address, "https://site11.example.com/");
    }

    #[test]
    fn a_title_is_one_line() {
        let profile = folder("one-line");
        history(&profile.join("History"), &[("https://example.org/", "Two\tparts\nhere", 1, 0)]);
        assert_eq!(titles(&read(&profile, &profile).unwrap()), ["Two parts here"]);
    }

    #[test]
    fn reading_leaves_chromes_history_as_it_was_and_no_copy_behind() {
        let profile = folder("untouched");
        let scratch = folder("scratch");
        history(&profile.join("History"), &[("https://example.org/", "A page", 1, 0)]);
        let before = fs::read(profile.join("History")).unwrap();
        read(&profile, &scratch).unwrap();
        assert_eq!(fs::read(profile.join("History")).unwrap(), before);
        assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0, "the copy is gone");
    }

    #[test]
    fn a_profile_chrome_never_ran_has_none() {
        assert_eq!(read(&folder("none"), &folder("none-scratch")).unwrap(), []);
    }

    #[test]
    fn a_host_is_found_in_any_web_address() {
        assert_eq!(host("https://user@Example.org:443/a"), Some("Example.org"));
        assert_eq!(host("http://[::1]:8000/"), Some("::1"));
        assert_eq!(host("https://example.org?q"), Some("example.org"));
        assert_eq!(host("file:///etc/hosts"), None);
        assert_eq!(host("https:///nothing"), None);
    }
}
