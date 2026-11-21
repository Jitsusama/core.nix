//! Tending a repository from its row (`code_rows.rs`), as Code's page shows
//! it wants: Forget Gone Trees, where git still lists trees whose folders
//! are gone, has git forget them, and Gather Its Store, where git's own
//! upkeep would gather its store now, runs that upkeep. Both act only on a
//! checkout's own folder, the one holding `.git` as a folder, as Pull does,
//! never on a tree cut from it or a bare checkout, which git is told of
//! another way. Neither touches a file to work in: forgetting a tree takes
//! only git's own record of it, its branch stays, and a locked tree is
//! kept, as git keeps it. Git's record of a gone tree, its HEAD and its
//! own refs and reflogs, can be the one place some commits are kept, as a tree
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

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use repos::model::Checkout;
use serde_json::{Value, json};

pub use crate::git::READ;
use crate::git::{fed, ran};

/// The longest gathering a store may take before it's stopped. A store of
/// millions of objects takes minutes, and one stopped is left as it was,
/// since git writes a pack whole before it removes what it gathered.
pub const GATHER: Duration = Duration::from_mins(30);

/// How old an upkeep's lock is before it's called left behind: git's own
/// half a day, past which its gc takes a `gc.pid` for a dead one's, as
/// Joel's own gc or a scheduled upkeep may hold a large store far longer
/// than a gathering Code starts.
const LEFT: Duration = Duration::from_hours(12);

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
    /// Git's upkeep left its lock on the store behind, longer ago than any
    /// upkeep takes, as a killed one does, and git keeps no rule to call it
    /// stale.
    Left,
    /// The checkout's config names a program git's gc runs, which a
    /// `.git` from elsewhere could name, and which no `-c` takes back.
    Hooked,
    /// What git's records of its gone trees hold couldn't all be read, as
    /// a reftable keeps them where no file says, so none is taken on trust.
    Records,
}

impl Refused {
    fn code(self) -> &'static str {
        match self {
            Refused::Tree => "tree",
            Refused::Busy => "busy",
            Refused::Unread => "unread",
            Refused::Failed => "failed",
            Refused::Stranded => "stranded",
            Refused::Left => "left",
            Refused::Hooked => "hooked",
            Refused::Records => "records",
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

/// What git's records of a checkout's trees hold, apart by whether
/// `worktree prune` would take the record: gone are those unlocked whose
/// folder is gone, or never said, and kept the rest; and the folders the
/// gone records name, for git to forget each by.
struct Held {
    gone: Vec<String>,
    kept: Vec<String>,
    folders: Vec<PathBuf>,
}

/// What the records of the trees of the checkout whose git folder is
/// `common` hold, each its HEAD, the refs it keeps of its own, and its
/// reflogs' commits written since `since`, as git's gc keeps them.
///
/// # Errors
///
/// [`Refused::Records`] for a gone record that can't be read for what it
/// holds: one in a reftable, whose HEAD file says only
/// `refs/heads/.invalid`, or one whose HEAD says neither a commit nor a
/// ref, which is refused rather than taken to hold nothing.
fn held_by_trees(common: &Path, since: u64) -> Result<Held, Refused> {
    let mut held = Held {
        gone: Vec::new(),
        kept: Vec::new(),
        folders: Vec::new(),
    };
    let Ok(records) = std::fs::read_dir(common.join("worktrees")) else {
        return Ok(held);
    };
    for record in records.flatten().map(|record| record.path()) {
        let there = std::fs::read_to_string(record.join("gitdir")).ok();
        let tree = there.map(|there| record.join(there.trim()));
        let gone = !tree.as_ref().is_some_and(|tree| tree.exists());
        if record.join("locked").exists() || !gone {
            pointed(&record, since, &mut held.kept);
            continue;
        }
        let tip = std::fs::read_to_string(record.join("HEAD")).unwrap_or_default();
        let tip = tip.trim();
        let readable = (named(tip) || tip.starts_with("ref: refs/"))
            && tip != "ref: refs/heads/.invalid"
            && !record.join("reftable").exists();
        if !readable {
            return Err(Refused::Records);
        }
        pointed(&record, since, &mut held.gone);
        held.folders
            .extend(tree.and_then(|tree| tree.parent().map(Path::to_path_buf)));
    }
    for side in [&mut held.gone, &mut held.kept] {
        side.retain(|commit| named(commit));
        side.sort();
        side.dedup();
    }
    Ok(held)
}

/// Adds to `held` what the git folder `git_dir` points at: its HEAD, the
/// refs it keeps as files, and its reflogs' commits, unchecked.
fn pointed(git_dir: &Path, since: u64, held: &mut Vec<String>) {
    let tip = std::fs::read_to_string(git_dir.join("HEAD")).unwrap_or_default();
    held.push(tip.trim().to_owned());
    for file in files(&git_dir.join("refs")) {
        let tip = std::fs::read_to_string(file).unwrap_or_default();
        held.push(tip.trim().to_owned());
    }
    logged(git_dir, since, held);
}

/// Adds to `held` both commits of each entry in the reflogs the git
/// folder `git_dir` keeps, its HEAD's and its refs', written since
/// `since`, unchecked.
fn logged(git_dir: &Path, since: u64, held: &mut Vec<String>) {
    for log in files(&git_dir.join("logs")) {
        let log = std::fs::read_to_string(log).unwrap_or_default();
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
}

/// Every file under `folder`, however deep.
fn files(folder: &Path) -> Vec<PathBuf> {
    let (mut found, mut folders) = (Vec::new(), vec![folder.to_path_buf()]);
    while let Some(folder) = folders.pop() {
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => folders.push(entry.path()),
                Ok(kind) if kind.is_file() => found.push(entry.path()),
                _ => {}
            }
        }
    }
    found
}

/// Whether a gone tree of the checkout at `folder` is all that keeps
/// some commits: a commit git's record of it holds that no ref reaches,
/// as a tree worked in at no branch, or rebased, leaves. That record,
/// which `worktree prune` takes, holds its HEAD and its reflog, roots
/// git's gc walks from, so once it's taken, gc drops what only it held.
/// Nothing a ref, the checkout's HEAD, a kept tree or a reflog entry git
/// keeps reaches is lost, as an amended branch's old tip is not, and an
/// entry git would expire anyway counts on neither side, so a tree is held
/// no longer than git would hold it, and lets go of nothing git would
/// drop. What's held goes to git on its input, never its command line,
/// whose length a checkout of thousands of trees would pass.
fn stranding(git: &str, folder: &Path, held: &Held, since: u64) -> Result<bool, Refused> {
    if held.gone.is_empty() {
        return Ok(false);
    }
    let mut reached = held.kept.clone();
    logged(&folder.join(".git"), since, &mut reached);
    reached.retain(|commit| named(commit));
    let mut input = held.gone.clone();
    // Run from the checkout's own folder, a single worktree's refs are
    // every ref and its own HEAD, but no tree's record.
    input.extend(["--not".to_owned(), "--all".to_owned()]);
    input.extend(reached);
    let args = [
        "rev-list",
        "--ignore-missing",
        "--max-count=1",
        "--single-worktree",
        "--stdin",
    ];
    let left = fed(git, folder, &args, input.join("\n") + "\n", Some(READ))
        .map_err(|_| Refused::Failed)?;
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
    let since = kept_since(git, folder);
    let held = held_by_trees(&folder.join(".git"), since)?;
    if stranding(git, folder, &held, since)? {
        return Err(Refused::Stranded);
    }
    // Each by the folder its record names, those read alone, never a
    // prune of whatever's gone by then, so a tree gone since they were
    // read waits for the next press.
    let mut refused = false;
    for tree in &held.folders {
        let tree = tree.to_string_lossy();
        let remove = ["worktree", "remove", tree.as_ref()];
        refused |= ran(git, folder, &remove, Some(READ)).is_err();
    }
    let kept = gone(&own(folder)?);
    let forgot = before.saturating_sub(kept);
    if refused && forgot == 0 {
        return Err(Refused::Failed);
    }
    Ok(Tended::Forgot { forgot, kept })
}

/// Gathers the store of the checkout at `folder` as git's own upkeep
/// does, with the git at `git`, where it's due: git's `gc` task, at the
/// checkout's config, waited on rather than left in the background. A
/// store read the same after as before was left untouched, whatever git's
/// exit said, since git's upkeep is quiet when it passes one by. Its
/// upkeep forgets no tree, as its worktree-prune task would forget those
/// gone past three months, or never checked out, so only Forget Gone
/// Trees forgets one. A gc task a checkout's config turns on still
/// forgets a gone tree git can't date, so it's refused, as Forget is,
/// while a gone tree alone holds some commits.
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
    // holds this lock, or a killed one left it.
    if let Ok(lock) = std::fs::metadata(folder.join(".git/objects/maintenance.lock")) {
        let age = lock.modified().ok().and_then(|at| at.elapsed().ok());
        return Err(if age.is_some_and(|age| age > LEFT) {
            Refused::Left
        } else {
            Refused::Busy
        });
    }
    let hooks = ["config", "--get-all", "gc.recentObjectsHook"];
    match ran(git, folder, &hooks, Some(READ)) {
        Ok(said) if !said.trim().is_empty() => return Err(Refused::Hooked),
        Ok(_) | Err(Some(1)) => {}
        Err(_) => return Err(Refused::Failed),
    }
    let since = kept_since(git, folder);
    let held = held_by_trees(&folder.join(".git"), since)?;
    if stranding(git, folder, &held, since)? {
        return Err(Refused::Stranded);
    }
    let args = [
        "-c",
        "maintenance.worktree-prune.enabled=false",
        "-c",
        "gc.worktreePruneExpire=never",
        "maintenance",
        "run",
        "--auto",
        "--no-detach",
        "--quiet",
    ];
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
