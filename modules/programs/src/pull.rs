//! Pull on a repository's row (`code_rows.rs`): its checkout's branch
//! brought up to the branch it follows, fetched first, by fast-forward
//! alone. It never merges or rebases: a checkout with changes of its own
//! to its tracked files, or whose branch has commits the one it follows
//! hasn't, is left as it is, and the answer says why. It acts only on a
//! checkout's own folder, the one holding `.git` as a folder, never on a
//! tree cut from it, which is where an agent works.
//!
//! Git runs as Joel's own would, with his config, since a fetch needs his
//! keys; nothing asks him anything, and a fetch past [`FETCH`] is stopped.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// The longest a fetch may run before it's stopped, which leaves the
/// checkout as it was.
pub const FETCH: Duration = Duration::from_secs(120);

/// How a pull went when it changed nothing it shouldn't.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pulled {
    /// Its branch was at the one it follows already, or ahead of it alone.
    Even,
    /// Its branch was brought forward by this many commits.
    Took(u64),
}

/// Why a pull left the checkout as it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// The folder isn't a checkout's own, as a tree cut from one isn't.
    Tree,
    /// Its head is no branch, or its branch follows none.
    Unfollowed,
    /// Its tracked files have changes of its own.
    Changed,
    /// The fetch failed or ran past [`FETCH`].
    Unfetched,
    /// Its branch and the one it follows have each moved, so only a merge
    /// or a rebase would bring them together.
    Diverged,
    /// Git refused the fast-forward.
    Failed,
}

impl Refused {
    fn code(self) -> &'static str {
        match self {
            Refused::Tree => "tree",
            Refused::Unfollowed => "unfollowed",
            Refused::Changed => "changed",
            Refused::Unfetched => "unfetched",
            Refused::Diverged => "diverged",
            Refused::Failed => "failed",
        }
    }
}

/// What a pull's answer says, from Pull's own words in the vocabulary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Words {
    said: Value,
}

impl Words {
    /// Pull's words from the vocabulary, each it hasn't said by its name.
    #[must_use]
    pub fn read(vocabulary: &Value) -> Words {
        let said = vocabulary["things"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|thing| thing["id"] == "verb:code:pull")
            .map_or(Value::Null, |thing| thing["said"].clone());
        Words { said }
    }

    fn say(&self, word: &str) -> String {
        self.said[word].as_str().unwrap_or(word).to_owned()
    }

    /// The answer to a pull's ask: what it did, or why it didn't, refused
    /// as the interface codes it, with the words the row's foot says.
    #[must_use]
    pub fn answer(&self, id: &Value, pulled: Result<Pulled, Refused>) -> Value {
        match pulled {
            Ok(Pulled::Even) => json!({"id": id, "said": self.say("even")}),
            Ok(Pulled::Took(1)) => json!({"id": id, "said": self.say("took")}),
            Ok(Pulled::Took(count)) => json!({
                "id": id,
                "said": self.say("tookMany").replace("{count}", &count.to_string()),
            }),
            Err(why) => {
                let said = self.say(why.code());
                json!({"id": id, "error": {
                    "code": "refused",
                    "refused": why.code(),
                    "message": said,
                    "said": said,
                }})
            }
        }
    }
}

/// Brings the checkout at `folder` up to the branch it follows, by
/// fast-forward alone, with the git at `git`: it says why before
/// fetching where it can, so a refusal waits on no network.
///
/// # Errors
///
/// Says why it left the checkout as it was.
pub fn pull(git: &str, folder: &Path) -> Result<Pulled, Refused> {
    // A tree cut from a checkout holds `.git` as a file naming it, where
    // an agent works; only a checkout's own folder holds it as a folder.
    if !folder.join(".git").is_dir() {
        return Err(Refused::Tree);
    }
    let run = |args: &[&str]| ran(git, folder, args, None);
    run(&["symbolic-ref", "--quiet", "HEAD"]).map_err(|()| Refused::Unfollowed)?;
    run(&["rev-parse", "--verify", "--quiet", "@{upstream}"]).map_err(|()| Refused::Unfollowed)?;
    let changed =
        run(&["status", "--porcelain", "--untracked-files=no"]).map_err(|()| Refused::Failed)?;
    if !changed.trim().is_empty() {
        return Err(Refused::Changed);
    }
    ran(git, folder, &["fetch", "--quiet"], Some(FETCH)).map_err(|()| Refused::Unfetched)?;
    let counted = run(&["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
        .map_err(|()| Refused::Failed)?;
    match counts(&counted).ok_or(Refused::Failed)? {
        (_, 0) => Ok(Pulled::Even),
        (0, behind) => {
            run(&["merge", "--ff-only", "--quiet", "@{upstream}"]).map_err(|()| Refused::Failed)?;
            Ok(Pulled::Took(behind))
        }
        _ => Err(Refused::Diverged),
    }
}

/// The commits only the head has and only the branch it follows has, as
/// `rev-list --left-right --count` prints them.
fn counts(printed: &str) -> Option<(u64, u64)> {
    let mut counts = printed.split_whitespace().map(str::parse::<u64>);
    Some((counts.next()?.ok()?, counts.next()?.ok()?))
}

/// Runs git in `folder` with nothing to answer it, so it never waits on
/// a prompt, answering what it printed; past `wait`, where one is given,
/// it's stopped. Only a fetch is given one: a fast-forward stopped part
/// way would leave the checkout half written.
fn ran(git: &str, folder: &Path, args: &[&str], wait: Option<Duration>) -> Result<String, ()> {
    let mut child = Command::new(git)
        .arg("-C")
        .arg(folder)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| ())?;
    let started = Instant::now();
    if let Some(wait) = wait {
        while child.try_wait().map_err(|_| ())?.is_none() {
            if started.elapsed() > wait {
                let _ = child.kill();
                let _ = child.wait();
                return Err(());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    let done = child.wait_with_output().map_err(|_| ())?;
    if !done.status.success() {
        return Err(());
    }
    String::from_utf8(done.stdout).map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn words() -> Words {
        Words::read(&json!({"things": [{"id": "verb:code:pull", "said": {
            "even": "up to date",
            "took": "took 1 commit",
            "tookMany": "took {count} commits",
            "changed": "not pulled: it has changes of its own",
        }}]}))
    }

    #[test]
    fn a_pull_says_how_many_commits_it_took_or_that_there_were_none() {
        let said = |pulled| words().answer(&json!(3), Ok(pulled))["said"].clone();
        assert_eq!(said(Pulled::Even), "up to date");
        assert_eq!(said(Pulled::Took(1)), "took 1 commit");
        assert_eq!(said(Pulled::Took(12)), "took 12 commits");
    }

    #[test]
    fn the_commits_each_side_has_alone_are_read_as_git_counts_them() {
        assert_eq!(counts("2\t5\n"), Some((2, 5)));
        assert_eq!(counts("2\n"), None);
    }

    #[test]
    fn a_refused_pull_says_why_in_its_own_words_and_by_its_code() {
        assert_eq!(
            words().answer(&json!(3), Err(Refused::Changed)),
            json!({"id": 3, "error": {
                "code": "refused",
                "refused": "changed",
                "message": "not pulled: it has changes of its own",
                "said": "not pulled: it has changes of its own",
            }})
        );
    }
}
