//! What a round left unread, said to Joel in For You beside the Slack
//! reader's rows (pineapple's `missed`), so he can look there himself for
//! a stop said then, rather than find it only in the journal: what a
//! search cut short or failing left out of his own words (own.rs), and a
//! thread failing so long it's let go (owed.rs), with its link.

use std::path::{Path, PathBuf};

use pineapple::github::{self, Thread};
use pineapple::missed;
use serde_json::Value;

use super::millis as now;

/// Where a site's rows go and what names them.
pub(super) struct Left {
    /// The site as For You is shown it, such as GitHub.
    pub(super) name: String,
    /// What each of its rows' names opens with: `pineapple-<site>-missed-`.
    prefix: String,
    /// Live, beside For You's row of what waits on Joel, which For You
    /// reads and nothing takes away; in shadow, the reader's own folder,
    /// which For You never reads.
    file: PathBuf,
    /// Where the site's own web pages are, for a thread's link.
    web: String,
}

impl Left {
    /// The rows of `site`, shown as `name`, whose API is at `origin`:
    /// under `state_home`, in For You's folder unless in `shadow`.
    pub(super) fn new(
        site: &str,
        name: &str,
        origin: &str,
        shadow: bool,
        state_home: &Path,
    ) -> Self {
        let file = if shadow {
            state_home.join(format!("pineapple/missed-{site}.json"))
        } else {
            state_home.join(format!("mission/waiting/pineapple-{site}-missed.json"))
        };
        Self {
            name: name.to_owned(),
            prefix: format!("pineapple-{site}-missed-"),
            file,
            web: web(origin),
        }
    }

    /// Tells Joel his own words in what `login` takes part in, changed from
    /// `from` to `to`, in seconds, went unsearched, as a search cut short
    /// or failing left them, so a stop of his there wasn't heard, and
    /// whether it's written: the search holds one that isn't (own.rs), as
    /// nothing searches that span again. The journal says it couldn't be
    /// written only when `aloud`.
    pub(super) fn own(&self, login: &str, from: u64, to: u64, aloud: bool) -> bool {
        let at = now();
        let (from, to) = (from.saturating_mul(1000), to.saturating_mul(1000));
        let say =
            |from: Option<u64>, to: u64, whole: bool| self.own_row(login, from, to, at, whole);
        self.keep(say(Some(from), to, true), at, say, aloud)
    }

    /// Tells Joel what was said on `thread` from `from`, in seconds, until
    /// it was let go, went unread, with its link, so he reads it himself,
    /// and whether it's written: one that isn't is owed still, so nothing
    /// is let go that he wasn't told of. The journal says it couldn't be
    /// written only when `aloud`.
    pub(super) fn thread(&self, thread: &Thread, from: u64, aloud: bool) -> bool {
        let at = now();
        let say =
            |from: Option<u64>, to: u64, whole: bool| self.thread_row(thread, from, to, at, whole);
        self.keep(
            say(Some(from.saturating_mul(1000)), at, true),
            at,
            say,
            aloud,
        )
    }

    /// The row saying what `login`'s search never read, from `from` to
    /// `to`, in milliseconds, raised `at`, with the search that finds it.
    fn own_row(&self, login: &str, from: Option<u64>, to: u64, at: u64, whole: bool) -> Value {
        let what = if whole { "what" } else { "parts of what" };
        let stamp = |millis: u64| github::stamp(millis / 1000);
        missed::row(
            &missed::named(&self.prefix, "own", from, to),
            &format!("The Safe Word Missed Part of {}", self.name),
            &format!(
                "The safe word never read {what} was said in pull requests and issues you take \
                 part in that changed {}, as its search was cut short or failed, so a stop said \
                 there then wasn't heard",
                missed::when(from, Some(to), at)
            ),
            &format!(
                "Search {}'s issues and pull requests for pineapple involves:{login} \
                 updated:{}..{}, for a stop said then",
                self.name,
                stamp(from.unwrap_or(0)),
                stamp(to)
            ),
            at,
            whole,
        )
    }

    /// The row saying what was said on `thread` from `from` to `to`, in
    /// milliseconds, raised `at`, was never read, with its link. Named
    /// for the thread, so it meets only its own rows.
    fn thread_row(
        &self,
        thread: &Thread,
        from: Option<u64>,
        to: u64,
        at: u64,
        whole: bool,
    ) -> Value {
        let what = if whole { "what" } else { "parts of what" };
        let kind = if thread.pull { "pull" } else { "issues" };
        missed::row(
            &missed::named(
                &self.prefix,
                &format!("thread-{:016x}", fnv(&thread.key())),
                from,
                to,
            ),
            &format!("The Safe Word Missed a Thread on {}", self.name),
            &format!(
                "The safe word never read {what} was said on {}#{} {}, as {} failed to give it \
                 however often it was asked, so a stop said there then wasn't heard",
                thread.repo,
                thread.number,
                missed::when(from, Some(to), at),
                self.name
            ),
            &format!(
                "Read {}/{}/{kind}/{} for a stop said then",
                self.web, thread.repo, thread.number
            ),
            at,
            whole,
        )
    }

    /// Keeps `row`, raised `at`, said with the rows it meets as `say` makes
    /// them, says in the journal what Joel reads, and whether it's written.
    /// Why it couldn't be is said only when `aloud`, as a row is tried
    /// again each round while it can't be: a thread's is said on the tries
    /// its wait chose (owed.rs), and a span's on the first try of the
    /// stretch it's held in (own.rs), so a broken rows file says so once a
    /// stretch, not once a round.
    fn keep(
        &self,
        row: Value,
        at: u64,
        say: impl FnOnce(Option<u64>, u64, bool) -> Value,
        aloud: bool,
    ) -> bool {
        let why = row["why"].as_str().unwrap_or_default().to_owned();
        match missed::keep(&self.file, &self.prefix, row, at, say) {
            Ok(written) => {
                eprintln!("<5>wrote Joel's row in {}: {written}", self.file.display());
                true
            }
            Err(_) if !aloud => false,
            Err(error) => {
                eprintln!(
                    "<3>couldn't write Joel's row in {}: {why}: {error}",
                    self.file.display()
                );
                false
            }
        }
    }
}

/// Where a site's web pages are, from where its API answers: GitHub's
/// own at github.com, another's without its `/api/v3`.
fn web(origin: &str) -> String {
    let origin = origin.trim_end_matches('/');
    origin.strip_suffix("/api/v3").map_or_else(
        || origin.replacen("://api.github.com", "://github.com", 1),
        str::to_owned,
    )
}

/// A name for `key` that stays the same across builds and holds no `-`
/// or `/`, as a row's name reads its span back from its dashes: 64-bit
/// FNV-1a, which the standard library's hasher doesn't promise to be.
fn fnv(key: &str) -> u64 {
    key.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// GitHub's rows in shadow, in a folder of `case`'s own in this test run,
/// removed once they're dropped, as cases run at once. A file written
/// where the rows' folder would be keeps any row from being written.
#[cfg(test)]
pub(super) struct Rows {
    pub(super) home: PathBuf,
    pub(super) left: Left,
}

#[cfg(test)]
impl Rows {
    /// Its folder is made here, and one already there fails the case, as
    /// its name can be guessed: a link someone left there would have the
    /// case write through it.
    pub(super) fn new(case: &str) -> Self {
        let home =
            std::env::temp_dir().join(format!("pineapple-left-{}-{case}", std::process::id()));
        std::fs::create_dir(&home).unwrap_or_else(|error| {
            panic!("couldn't make {} the case's own: {error}", home.display())
        });
        let left = Left::new("github", "GitHub", "https://api.github.com", true, &home);
        Self { home, left }
    }

    /// Keeps any row from being written, by a file where its folder would be.
    pub(super) fn break_home(&self) {
        let folder = self.home.join("pineapple");
        std::fs::write(folder, "").expect("a file where the rows' folder would be");
    }

    /// Lets rows be written again.
    pub(super) fn free_home(&self) {
        std::fs::remove_file(self.home.join("pineapple")).expect("the rows' folder freed");
    }
}

#[cfg(test)]
impl Drop for Rows {
    fn drop(&mut self) {
        drop(std::fs::remove_dir_all(&self.home));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sites_web_pages_are_where_its_api_answers_without_its_api() {
        assert_eq!(
            web("https://api.github.com"),
            "https://github.com",
            "GitHub's own"
        );
        assert_eq!(
            web("https://code.example.com/api/v3/"),
            "https://code.example.com",
            "another's, without its API's path"
        );
        assert_eq!(
            web("http://127.0.0.1:8080"),
            "http://127.0.0.1:8080",
            "a stand-in, as it is"
        );
    }

    #[test]
    fn a_threads_name_is_the_same_each_time_and_its_own() {
        assert_eq!(
            fnv(""),
            0xcbf2_9ce4_8422_2325,
            "FNV-1a's offset for nothing"
        );
        assert_eq!(
            fnv("a"),
            0xaf63_dc4c_8601_ec8c,
            "FNV-1a's published value for a"
        );
        assert_ne!(
            fnv("github:example/shop#12"),
            fnv("github:example/shop#13"),
            "two threads, two names"
        );
    }
}
