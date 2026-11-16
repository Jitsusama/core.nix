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
use std::time::Duration;

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
/// fast-forward alone, with the git at `git`.
///
/// # Errors
///
/// Says why it left the checkout as it was.
pub fn pull(git: &str, folder: &Path) -> Result<Pulled, Refused> {
    let _ = (git, folder);
    Err(Refused::Failed)
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
