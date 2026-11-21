//! Tending a repository from its row (`code_rows.rs`), as Code's page shows
//! it wants: Forget Gone Trees, where git still lists trees whose folders
//! are gone, has git forget them, and Gather Its Store, where git's own
//! upkeep would gather its store now, runs that upkeep. Both act only on a
//! checkout's own folder, the one holding `.git` as a folder, as Pull does,
//! never on a tree cut from it or a bare checkout, which git is told of
//! another way. Neither touches a file to work in: forgetting a tree takes
//! only git's own record of it, its branch stays, and a locked tree is
//! kept, as git keeps it. Git's record of a gone tree, its HEAD and its
//! own reflog, can be the one place some commits are kept, as a tree
//! worked in at no branch, or rebased, leaves them, so forgetting is
//! refused while one is rather than leave them to git's next gc, which
//! Gather Its Store runs. The checkout is read before and after
//! from git's own files, as `repos` reads it, so the answer says what
//! changed rather than what was asked.
//!
//! Git runs as `git.rs` runs it, with Joel's config and asking at no
//! terminal. Forgetting is stopped past [`READ`], gathering past
//! [`GATHER`], with all it started, and git lets go of its locks as it's
//! asked to stop.

use std::path::Path;
use std::time::{Duration, SystemTime};

use repos::model::Checkout;
use serde_json::{Value, json};

pub use crate::git::READ;
use crate::git::ran;

/// The longest gathering a store may take before it's stopped. A store of
/// millions of objects takes minutes, and one stopped is left as it was,
/// since git writes a pack whole before it removes what it gathered.
pub const GATHER: Duration = Duration::from_mins(30);

/// How a tending went when it changed nothing it shouldn't.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tended {
    /// Git forgot this many gone trees, and kept this many it lists still,
    /// gone but locked.
    Forgot { forgot: u64, kept: u64 },
    /// Its store was gathered, and git still calls it due or not.
    Gathered { still: bool },
    /// Git ran its upkeep and left the store as it was, as it does when
    /// what it holds loose is unreached and too young to drop, when
    /// another upkeep took the store as it began, or when Joel's own
    /// config says it isn't due where the checkout's alone, which repos
    /// reads, says it is. Which, git doesn't say, so neither does Code.
    Untouched,
    /// There was nothing to do, as a reading since the row's said.
    Nothing,
}

/// Why a tending left the checkout as it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// The folder isn't a checkout's own, as a tree cut from one isn't.
    Tree,
    /// A Pull, a switch or a tending of the same checkout is under way,
    /// or git's own upkeep holds its store.
    Busy,
    /// Its git folder couldn't be read.
    Unread,
    /// Git failed, or ran past its wait.
    Failed,
    /// A gone tree holds commits no branch or other ref reaches.
    Stranded,
}

impl Refused {
    fn code(self) -> &'static str {
        match self {
            Refused::Tree => "tree",
            Refused::Busy => "busy",
            Refused::Unread => "unread",
            Refused::Failed => "failed",
            Refused::Stranded => "stranded",
        }
    }
}

/// The two acts, by what each answers in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Forget,
    Gather,
}

impl Act {
    fn id(self) -> &'static str {
        match self {
            Act::Forget => "verb:code:forget",
            Act::Gather => "verb:code:gather",
        }
    }
}

/// What a tending's answer says, from each act's own words in the
/// vocabulary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Words {
    forget: Value,
    gather: Value,
}

impl Words {
    /// Each act's words from the vocabulary, each it hasn't said by its name.
    #[must_use]
    pub fn read(vocabulary: &Value) -> Words {
        let said = |act: Act| {
            vocabulary["things"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|thing| thing["id"] == act.id())
                .map_or(Value::Null, |thing| thing["said"].clone())
        };
        Words {
            forget: said(Act::Forget),
            gather: said(Act::Gather),
        }
    }

    fn say(&self, act: Act, word: &str) -> String {
        let said = match act {
            Act::Forget => &self.forget,
            Act::Gather => &self.gather,
        };
        said[word].as_str().unwrap_or(word).to_owned()
    }

    /// The answer to a tending's ask: what it did, or why it didn't,
    /// refused as the interface codes it, with the words the row's foot
    /// says.
    #[must_use]
    pub fn answer(&self, id: &Value, act: Act, tended: Result<Tended, Refused>) -> Value {
        let counted = |one: &str, many: &str, count: u64| {
            let word = if count == 1 { one } else { many };
            self.say(act, word).replace("{count}", &count.to_string())
        };
        match tended {
            Ok(Tended::Forgot { forgot, kept }) => {
                let mut said = if forgot == 0 {
                    self.say(act, "nothing")
                } else {
                    counted("forgot", "forgotMany", forgot)
                };
                if kept > 0 {
                    said.push_str(&counted("kept", "keptMany", kept));
                }
                json!({"id": id, "said": said})
            }
            Ok(Tended::Gathered { still }) => {
                let word = if still { "gatheredStill" } else { "gathered" };
                json!({"id": id, "said": self.say(act, word)})
            }
            Ok(Tended::Untouched) => json!({"id": id, "said": self.say(act, "untouched")}),
            Ok(Tended::Nothing) => json!({"id": id, "said": self.say(act, "nothing")}),
            Err(why) => {
                let said = self.say(act, why.code());
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

/// The checkout at `folder` read from git's own files, where it's a
/// checkout's own folder: a tree cut from one holds `.git` as a file
/// naming it, and a bare one holds no `.git` at all.
fn own(folder: &Path) -> Result<Checkout, Refused> {
    let common = folder.join(".git");
    if !common.is_dir() || !common.join("HEAD").is_file() {
        return Err(Refused::Tree);
    }
    Ok(repos::read::checkout(&common, None).checkout().clone())
}

/// How many trees cut from it are gone: listed by git, with no folder.
fn gone(checkout: &Checkout) -> u64 {
    let gone = checkout.trees.iter().filter(|tree| !tree.present).count();
    u64::try_from(gone).unwrap_or(u64::MAX)
}

/// How long git keeps a reflog's entry that no ref reaches, before gc
/// expires it, as the time before which none is kept: the checkout's
/// `gc.reflogExpireUnreachable`, which git reads as such a time, or git's
/// own 30 days where it says none.
fn kept_since(git: &str, folder: &Path) -> u64 {
    const DEFAULT: Duration = Duration::from_hours(30 * 24);
    let args = [
        "config",
        "--type=expiry-date",
        "--get",
        "gc.reflogExpireUnreachable",
    ];
    let said = ran(git, folder, &args, Some(READ)).ok();
    said.and_then(|said| said.trim().parse().ok())
        .unwrap_or_else(|| {
            let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);
            now.unwrap_or_default().saturating_sub(DEFAULT).as_secs()
        })
}

/// Whether `said` is a whole commit's name, as git writes one, and so
/// nothing git could read as anything else.
fn named(said: &str) -> bool {
    matches!(said.len(), 40 | 64)
        && said.bytes().all(|byte| byte.is_ascii_hexdigit())
        && said.bytes().any(|byte| byte != b'0')
}

/// The commits git's records of the gone trees of the checkout whose git
/// folder is `common` hold, as `worktree prune` would take them: each
/// unlocked record whose folder is gone, or never said. Each holds its
/// HEAD, where it's at a commit, and its own reflog's commits, those
/// written since `since`, as git's gc keeps them.
fn held_by_gone(common: &Path, since: u64) -> Vec<String> {
    let Ok(records) = std::fs::read_dir(common.join("worktrees")) else {
        return Vec::new();
    };
    let mut held = Vec::new();
    for record in records.flatten().map(|record| record.path()) {
        let there = std::fs::read_to_string(record.join("gitdir"));
        let gone = !there.is_ok_and(|there| record.join(there.trim()).exists());
        if record.join("locked").exists() || !gone {
            continue;
        }
        let tip = std::fs::read_to_string(record.join("HEAD")).unwrap_or_default();
        held.push(tip.trim().to_owned());
        let log = std::fs::read_to_string(record.join("logs/HEAD")).unwrap_or_default();
        for entry in log.lines() {
            let said = entry.split('\t').next().unwrap_or_default();
            let words: Vec<&str> = said.split_whitespace().collect();
            let at = words
                .len()
                .checked_sub(2)
                .and_then(|at| words[at].parse().ok());
            if at.is_some_and(|at: u64| at >= since) {
                held.extend(words.iter().take(2).map(|commit| (*commit).to_owned()));
            }
        }
    }
    held.retain(|commit| named(commit));
    held.sort();
    held.dedup();
    held
}

/// Whether a gone tree of the checkout at `folder` is all that keeps
/// some commits: a commit git's record of it holds that no ref reaches,
/// as a tree worked in at no branch, or rebased, leaves. That record,
/// which `worktree prune` takes, holds its HEAD and its reflog, roots
/// git's gc walks from, so once it's taken, gc drops what only it held.
/// Nothing a ref reaches is lost, and an entry git would expire anyway
/// is left out, so a tree is held no longer than git would hold it.
fn stranding(git: &str, folder: &Path) -> Result<bool, Refused> {
    let held = held_by_gone(&folder.join(".git"), kept_since(git, folder));
    if held.is_empty() {
        return Ok(false);
    }
    let mut args = vec!["rev-list", "--ignore-missing", "--max-count=1"];
    args.extend(held.iter().map(String::as_str));
    args.extend(["--not", "--glob=*"]);
    let left = ran(git, folder, &args, Some(READ)).map_err(|_| Refused::Failed)?;
    Ok(!left.trim().is_empty())
}

/// Has git forget the trees cut from the checkout at `folder` whose
/// folders are gone, with the git at `git`, keeping those locked, and
/// none at all while one gone holds commits nothing else does.
///
/// # Errors
///
/// Says why it left the checkout as it was.
pub fn forget(git: &str, folder: &Path) -> Result<Tended, Refused> {
    let checkout = own(folder)?;
    let before = gone(&checkout);
    if before == 0 {
        return Ok(Tended::Nothing);
    }
    if stranding(git, folder)? {
        return Err(Refused::Stranded);
    }
    ran(git, folder, &["worktree", "prune"], Some(READ)).map_err(|_| Refused::Failed)?;
    let kept = gone(&own(folder)?);
    Ok(Tended::Forgot {
        forgot: before.saturating_sub(kept),
        kept,
    })
}

/// Gathers the store of the checkout at `folder` as git's own upkeep
/// does, with the git at `git`, where it's due: git's `gc` task, at the
/// checkout's config, waited on rather than left in the background. A
/// store read the same after as before was left untouched, whatever git's
/// exit said, since git's upkeep is quiet when it passes one by.
///
/// # Errors
///
/// Says why it left the checkout as it was.
pub fn gather(git: &str, folder: &Path, wait: Duration) -> Result<Tended, Refused> {
    let before = own(folder)?.weight.ok_or(Refused::Unread)?;
    if !before.due {
        return Ok(Tended::Nothing);
    }
    // Git's upkeep passes a store by, and says nothing, while another
    // holds this lock.
    if folder.join(".git/objects/maintenance.lock").exists() {
        return Err(Refused::Busy);
    }
    let args = ["maintenance", "run", "--auto", "--no-detach", "--quiet"];
    ran(git, folder, &args, Some(wait)).map_err(|_| Refused::Failed)?;
    let after = own(folder)?.weight.ok_or(Refused::Unread)?;
    if after == before {
        return Ok(Tended::Untouched);
    }
    Ok(Tended::Gathered { still: after.due })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn words() -> Words {
        Words::read(&json!({"things": [
            {"id": "verb:code:forget", "said": {
                "nothing": "no gone trees to forget",
                "forgot": "forgot 1 gone tree",
                "forgotMany": "forgot {count} gone trees",
                "kept": ", kept 1 locked",
                "keptMany": ", kept {count} locked",
                "tree": "not forgotten: it isn't a checkout's own folder",
                "stranded": "not forgotten: a gone tree holds commits no branch has",
            }},
            {"id": "verb:code:gather", "said": {
                "gathered": "gathered its store",
                "gatheredStill": "gathered its store, and git still calls it due",
                "untouched": "git left its store as it was",
                "nothing": "nothing was due",
            }},
        ]}))
    }

    #[test]
    fn forgetting_says_how_many_trees_it_forgot_and_kept() {
        let said = |tended| words().answer(&json!(3), Act::Forget, Ok(tended))["said"].clone();
        assert_eq!(
            said(Tended::Forgot { forgot: 1, kept: 0 }),
            "forgot 1 gone tree"
        );
        assert_eq!(
            said(Tended::Forgot { forgot: 4, kept: 2 }),
            "forgot 4 gone trees, kept 2 locked"
        );
        assert_eq!(
            said(Tended::Forgot { forgot: 0, kept: 1 }),
            "no gone trees to forget, kept 1 locked"
        );
        assert_eq!(said(Tended::Nothing), "no gone trees to forget");
    }

    #[test]
    fn gathering_says_whether_there_was_anything_to_gather() {
        let said = |tended| words().answer(&json!(3), Act::Gather, Ok(tended))["said"].clone();
        assert_eq!(
            said(Tended::Gathered { still: false }),
            "gathered its store"
        );
        assert_eq!(
            said(Tended::Gathered { still: true }),
            "gathered its store, and git still calls it due"
        );
        assert_eq!(said(Tended::Untouched), "git left its store as it was");
        assert_eq!(said(Tended::Nothing), "nothing was due");
    }

    #[test]
    fn forgetting_refused_for_a_stranded_tree_says_so() {
        let stranded = words().answer(&json!(3), Act::Forget, Err(Refused::Stranded));
        assert_eq!(
            stranded["error"]["said"], "not forgotten: a gone tree holds commits no branch has",
            "{stranded}"
        );
    }

    #[test]
    fn a_refused_tending_says_why_in_its_own_acts_words() {
        assert_eq!(
            words().answer(&json!(3), Act::Forget, Err(Refused::Tree)),
            json!({"id": 3, "error": {
                "code": "refused",
                "refused": "tree",
                "message": "not forgotten: it isn't a checkout's own folder",
                "said": "not forgotten: it isn't a checkout's own folder",
            }})
        );
        // A word its act hasn't is said by its name, never another act's.
        assert_eq!(
            words().answer(&json!(3), Act::Gather, Err(Refused::Tree))["error"]["said"],
            "tree"
        );
    }

    #[test]
    fn only_a_checkouts_own_folder_is_tended() {
        let folder = std::env::temp_dir().join(format!("tend-own-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        // A tree cut from a checkout holds `.git` as a file.
        std::fs::write(folder.join(".git"), "gitdir: /elsewhere\n").unwrap();
        assert_eq!(forget("git", &folder), Err(Refused::Tree));
        assert_eq!(gather("git", &folder, GATHER), Err(Refused::Tree));
        let _ = std::fs::remove_dir_all(&folder);
    }
}
