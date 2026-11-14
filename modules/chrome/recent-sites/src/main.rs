//! The sites Chrome visited lately, for Chrome's Recent inside the launcher:
//! one a site, the newest titled page of each, newest first, at most nine.
//!
//! `chrome-recent-sites sites [--profile PATH]`, or the command bare,
//! reads the History of that profile, Chrome's Default profile when none
//! is given, and prints a line a site: the page's
//! address, its title and when it was last visited in seconds since 1970,
//! a tab between each, the last left empty where Chrome kept no time, so
//! the launcher can place it among Recent's other sources by when (as
//! recents.nix's `dated` says). Chrome writes its history as it browses, so
//! this reads a copy of it and its journal, which SQLite rolls back to the
//! last whole write, and never the file Chrome holds. A profile with no
//! history prints nothing. Google's own windows (Mail, Calendar, Meet,
//! Drive and the documents) are left out, since their programs keep their
//! own, as are sign-in pages and this machine's own addresses, by the rule
//! what Chrome has open is held to too (../session, which the programs
//! daemon reads Chrome's open pages with).
//!
//! `chrome-recent-sites downloads [--profile PATH]` prints the files Chrome
//! downloaded that are still where it saved them, newest first, at most
//! nine (downloads.rs): each one's path, its size in bytes, when it
//! finished in seconds since 1970, or nothing where Chrome kept no time,
//! and the address of the page it came from, tab-separated.
//!
//! It takes the shape every command of ours does (docs/commands.md),
//! through the terminal kit. The launcher reads those lines through a
//! pipe, so there they stay as they are; into a terminal they're a table
//! fitting its width (said.rs), and with `--json` they're the records of
//! a `chrome-recent-sites.sites/1` or `chrome-recent-sites.downloads/1`
//! object.

mod downloads;
mod said;

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chrome_session::{host, on_the_web, one_line};
use rusqlite::Connection;
use terminal_kit::{Exit, Failure, Help};

/// As many as 1 to 9 can pick.
const MOST: usize = 9;

/// How far back it looks for nine sites.
const LOOKED_AT: u32 = 5000;

/// The seconds from 1601, where Chrome counts its time from, to 1970.
const FROM_1601: i64 = 11_644_473_600;

#[derive(Debug, PartialEq, Eq)]
struct Site {
    address: String,
    title: String,
    /// When it was last visited, in seconds since 1970, where Chrome says.
    visited: Option<i64>,
}

/// A time as Chrome keeps it, microseconds since 1601, in seconds since
/// 1970, or none where it kept none.
fn since_1970(chrome: i64) -> Option<i64> {
    (chrome > 0).then(|| chrome / 1_000_000 - FROM_1601)
}

/// The newest titled page of each site in a history, newest first.
fn sites(history: &Connection) -> rusqlite::Result<Vec<Site>> {
    let mut asked = history.prepare(
        "SELECT url, title, last_visit_time FROM urls
         WHERE hidden = 0 AND title <> '' AND (url LIKE 'https://%' OR url LIKE 'http://%')
         ORDER BY last_visit_time DESC LIMIT ?1",
    )?;
    let mut seen = HashSet::new();
    let mut found = Vec::new();
    let mut rows = asked.query([LOOKED_AT])?;
    while let Some(row) = rows.next()? {
        let address: String = row.get(0)?;
        let title: String = row.get(1)?;
        let visited = since_1970(row.get(2)?);
        let Some(host) = host(&address).map(str::to_ascii_lowercase) else {
            continue;
        };
        if !on_the_web(&address) || !seen.insert(host) {
            continue;
        }
        found.push(Site {
            address,
            title: one_line(&title),
            visited,
        });
        if found.len() == MOST {
            break;
        }
    }
    Ok(found)
}

/// Reads a profile's sites from a copy of its history, made in `scratch`
/// and gone once read.
fn read(profile: &Path, scratch: &Path) -> Result<Vec<Site>, String> {
    from_history(profile, scratch, sites)
}

/// What `reading` finds in a copy of a profile's history, made in
/// `scratch` and gone once read; nothing where Chrome kept no history.
fn from_history<T>(
    profile: &Path,
    scratch: &Path,
    reading: impl FnOnce(&Connection) -> rusqlite::Result<Vec<T>>,
) -> Result<Vec<T>, String> {
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
            fs::copy(&journal, folder.join("History-journal"))
                .map_err(|error| format!("{}: {error}", journal.display()))?;
        }
        let connection = Connection::open(&copy).map_err(|error| error.to_string())?;
        reading(&connection).map_err(|error| error.to_string())
    })();
    let _ = fs::remove_dir_all(&folder);
    read
}

fn help() -> Help {
    Help::new("chrome-recent-sites", "the sites Chrome visited lately")
        .example(
            "chrome-recent-sites",
            "what Chrome's Default profile visited",
        )
        .example("chrome-recent-sites downloads", "what it downloaded")
        .subcommand(
            reads("sites", "a site a line, newest first, nine at most")
                .alias("site")
                .example("chrome-recent-sites sites", "what Chrome visited lately")
                .example(
                    "chrome-recent-sites sites --profile P",
                    "what profile P visited",
                ),
        )
        .subcommand(
            reads("downloads", "the files it downloaded that are still there")
                .alias("download")
                .example("chrome-recent-sites downloads", "what it downloaded"),
        )
        .option("--profile PATH", "a profile's folder; Default's by default")
        .exit(Exit::Failed, "its history couldn't be read, or said")
        .exit(
            Exit::Config,
            "no --profile given, and no HOME to find one in",
        )
}

/// A subcommand that reads a profile's history.
fn reads(name: &str, about: &str) -> Help {
    Help::new(name, about)
        .option("--profile PATH", "a profile's folder; Default's by default")
        .exit(Exit::Failed, "its history couldn't be read, or said")
        .exit(
            Exit::Config,
            "no --profile given, and no HOME to find one in",
        )
}

/// What the command line asks: downloads or sites, and of which profile.
fn asked(args: &[String]) -> Result<(bool, Option<PathBuf>), Failure> {
    let mut words = Vec::new();
    let mut profile = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.split_once('=') {
            Some(("--profile", path)) => profile = Some(PathBuf::from(path)),
            _ if arg == "--profile" => {
                let path = args.next().ok_or_else(|| {
                    Failure::new(Exit::Usage, "--profile needs a folder")
                        .try_this("chrome-recent-sites --profile ~/.config/google-chrome/Default")
                })?;
                profile = Some(PathBuf::from(path));
            }
            _ => words.push(arg.as_str()),
        }
    }
    match words.as_slice() {
        [] | ["sites" | "site"] => Ok((false, profile)),
        ["downloads" | "download"] => Ok((true, profile)),
        _ => Err(
            Failure::new(Exit::Usage, "a profile is named with --profile")
                .because("sites and downloads take no other words")
                .try_this("chrome-recent-sites --help"),
        ),
    }
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let (downloaded, profile) = asked(&context.args)?;
        let profile = match profile {
            Some(profile) => profile,
            None => default_profile()?,
        };
        let scratch = env::var_os("XDG_RUNTIME_DIR").map_or_else(env::temp_dir, PathBuf::from);
        let unread = |error: String| {
            Failure::new(Exit::Failed, "Chrome's history couldn't be read").because(error)
        };
        let unsaid = |error: std::io::Error| {
            Failure::new(
                Exit::Failed,
                "what Chrome's history holds couldn't be written",
            )
            .because(error.to_string())
        };
        if downloaded {
            let found = from_history(&profile, &scratch, |history| {
                downloads::downloads(history, downloads::on_disk)
            })
            .map_err(unread)?;
            said::downloads(context, &found).map_err(unsaid)
        } else {
            let found = read(&profile, &scratch).map_err(unread)?;
            said::sites(context, &found).map_err(unsaid)
        }
    })
}

/// Chrome's Default profile, under HOME.
fn default_profile() -> Result<PathBuf, Failure> {
    let home = env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .ok_or_else(|| {
            Failure::new(Exit::Config, "there's no HOME to find Chrome's profile in")
                .try_this("chrome-recent-sites --profile ~/.config/google-chrome/Default")
        })?;
    Ok(PathBuf::from(home).join(".config/google-chrome/Default"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let folder = env::temp_dir().join(format!(
            "chrome-recent-sites-test-{}-{name}",
            std::process::id()
        ));
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
        assert_eq!(
            titles(&read(&profile, &profile).unwrap()),
            ["New page", "News", "Plain"]
        );
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
                (
                    "https://www.google.com/search?q=x",
                    "x - Google Search",
                    20,
                    0,
                ),
            ],
        );
        assert_eq!(
            titles(&read(&profile, &profile).unwrap()),
            ["x - Google Search"]
        );
    }

    #[test]
    fn nine_at_most() {
        let profile = folder("nine");
        let addresses: Vec<(String, i64)> = (0..12_i64)
            .map(|n| (format!("https://site{n}.example.com/"), n))
            .collect();
        let pages: Vec<(&str, &str, i64, i64)> = addresses
            .iter()
            .map(|(address, n)| (address.as_str(), "A page", *n, 0))
            .collect();
        history(&profile.join("History"), &pages);
        let found = read(&profile, &profile).unwrap();
        assert_eq!(found.len(), 9);
        assert_eq!(found[0].address, "https://site11.example.com/");
    }

    #[test]
    fn each_says_when_it_was_visited_in_seconds_since_1970() {
        let profile = folder("visited");
        history(
            &profile.join("History"),
            &[
                ("https://example.org/", "Dated", 13_444_473_600_500_000, 0),
                ("https://undated.example.org/", "Undated", 0, 0),
            ],
        );
        let found = read(&profile, &profile).unwrap();
        let visited: Vec<_> = found.iter().map(|site| site.visited).collect();
        assert_eq!(visited, [Some(1_800_000_000), None]);
    }

    #[test]
    fn a_title_is_one_line() {
        let profile = folder("one-line");
        history(
            &profile.join("History"),
            &[("https://example.org/", "Two\tparts\nhere", 1, 0)],
        );
        assert_eq!(
            titles(&read(&profile, &profile).unwrap()),
            ["Two parts here"]
        );
    }

    #[test]
    fn reading_leaves_chromes_history_as_it_was_and_no_copy_behind() {
        let profile = folder("untouched");
        let scratch = folder("scratch");
        history(
            &profile.join("History"),
            &[("https://example.org/", "A page", 1, 0)],
        );
        let before = fs::read(profile.join("History")).unwrap();
        read(&profile, &scratch).unwrap();
        assert_eq!(fs::read(profile.join("History")).unwrap(), before);
        assert_eq!(
            fs::read_dir(&scratch).unwrap().count(),
            0,
            "the copy is gone"
        );
    }

    #[test]
    fn a_profile_chrome_never_ran_has_none() {
        assert_eq!(read(&folder("none"), &folder("none-scratch")).unwrap(), []);
    }

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn sites_are_asked_for_bare_or_by_name() {
        for line in ["", "sites", "site"] {
            assert_eq!(
                asked(&words(line)).ok(),
                Some((false, None)),
                "{line:?} should ask for the sites"
            );
        }
    }

    #[test]
    fn a_profile_is_named_with_its_option_either_way() {
        for line in ["--profile p", "sites --profile p", "--profile=p sites"] {
            assert_eq!(
                asked(&words(line)).ok(),
                Some((false, Some(PathBuf::from("p")))),
                "{line:?} should read profile p's sites"
            );
        }
        assert_eq!(
            asked(&words("downloads --profile p")).ok(),
            Some((true, Some(PathBuf::from("p")))),
            "downloads --profile p should read profile p's downloads"
        );
    }

    #[test]
    fn a_stray_word_or_a_missing_folder_is_asked_wrongly() {
        for line in ["sites p", "downloads p", "--profile"] {
            assert_eq!(
                asked(&words(line)).err().map(|failure| failure.exit),
                Some(Exit::Usage),
                "{line:?} should be refused as asked wrongly"
            );
        }
    }
}
