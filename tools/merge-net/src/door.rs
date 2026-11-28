//! `merge-net door PR HEAD`, the merge door: how a cloud room gets a pull
//! request onto the merge train when it can't reach the train itself. The
//! room asks it through `mycelium run` on the machine the net runs on, and
//! the door runs `ready HEAD` from a clean clone at the pull request's
//! branch, so `ready` judges it, and boards the train, exactly as it would
//! for a session in its own tree.
//!
//! The door never merges. It takes only an open pull request into main
//! from one of the repository's own branches, whose head GitHub gives as
//! the HEAD the room asked about, so a room can't send it to ready a
//! commit nobody proposed, or one its pull request has since moved past.

use std::env;
use std::fmt;
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde_json::Value;
use terminal_kit::Exit;

use crate::state::{State, short};
use crate::train::slug_of;

/// What the door asks GitHub of the pull request, which [`judge`] reads.
const FIELDS: &str = "state,headRefOid,baseRefName,isDraft,headRefName,isCrossRepository";

/// A pull request as GitHub gives it to the door.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pull {
    pub state: String,
    pub head: String,
    pub base: String,
    pub draft: bool,
    pub branch: String,
    pub cross_repository: bool,
}

/// A pull request's branch as GitHub names it, which the door clones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch(pub String);

/// Why the door won't ready a pull request, with what GitHub or git said
/// of it, so the room can tell which condition failed without asking
/// again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// It isn't open, so there's nothing to merge.
    NotOpen { state: String },
    /// Its head isn't the one asked about: it moved, or the room asked of
    /// another commit.
    OtherHead { asked: String, github: String },
    /// It merges into another branch than main, which the train never
    /// lands on.
    NotIntoMain { base: String },
    /// It's a draft, its owner's word that it isn't to land yet.
    Draft,
    /// It comes from a fork, whose branch the net's clone can't fetch and
    /// whose owner the net doesn't vouch for.
    CrossRepository { branch: String },
    /// Its branch's name isn't one git takes as a branch, so it's never
    /// put on a command line.
    BadBranch { branch: String, said: String },
    /// The branch the clone took is at another commit than the head asked
    /// about: it moved between GitHub's answer and the clone.
    Moved { asked: String, cloned: String },
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotOpen { state } => write!(f, "it isn't open: GitHub says it's {state}"),
            Self::OtherHead { asked, github } => write!(
                f,
                "its head isn't {}: GitHub says it's {}",
                short(asked),
                short(github)
            ),
            Self::NotIntoMain { base } => {
                write!(
                    f,
                    "it doesn't merge into main: GitHub says it merges into {base}"
                )
            }
            Self::Draft => write!(f, "it's a draft: GitHub says isDraft is true"),
            Self::CrossRepository { branch } => write!(
                f,
                "its branch {branch} is a fork's: GitHub says isCrossRepository is true"
            ),
            Self::BadBranch { branch, said } => write!(
                f,
                "its branch {branch:?}, as GitHub names it, isn't a branch name git takes: {said}"
            ),
            Self::Moved { asked, cloned } => write!(
                f,
                "its branch moved: asked to ready {}, the clone took {}",
                short(asked),
                short(cloned)
            ),
        }
    }
}

/// Run `ready HEAD` for pull request `pr` from a clone at its branch, and
/// end with ready's own code, or refuse it, saying why.
pub fn door(pr: u64, head: &str) -> Result<ExitCode, String> {
    let url = env::var("MERGE_NET_REPOSITORY").unwrap_or_else(|_| crate::REPOSITORY.to_owned());
    let root = State::from_env().root;
    let doors = root.join("door");
    fs::create_dir_all(&doors).map_err(|error| format!("making {}: {error}", doors.display()))?;
    // Held for the whole run, so two asks of one pull request never clone
    // into, or clear, each other's folder.
    let _held = held(&doors.join(format!("{pr}.lock")), pr)?;
    let said = asked_github(pr, &slug_of(&url))?;
    let pull = pull_of(&said).map_err(|why| format!("GitHub's answer for #{pr}: {why}"))?;
    let branch = match judge(&pull, head).and_then(well_formed) {
        Ok(branch) => branch,
        Err(refusal) => return Ok(refused(pr, &refusal)),
    };
    let folder = doors.join(folder_name(pr, head));
    clear_other_heads(&doors, pr, head);
    if let Err(refusal) = cloned(&root, &url, &branch, &folder, head)? {
        return Ok(refused(pr, &refusal));
    }
    let exe = env::current_exe().map_err(|error| format!("finding merge-net itself: {error}"))?;
    let ran = Command::new(exe)
        .args(["ready", head])
        .current_dir(&folder)
        .status()
        .map_err(|error| format!("running ready in {}: {error}", folder.display()))?;
    // A ready ended by a signal reads as a shell would say it, 128 and the
    // signal, though it ends the door as a failure, since that's no code
    // ready chose.
    let rc = ran
        .code()
        .or_else(|| ran.signal().map(|signal| 128 + signal))
        .unwrap_or(1);
    eprintln!(
        "merge-net: the door's ready at {} ended rc {rc}",
        short(head)
    );
    // The clone stays, since ready's rerun starts a unit in the tree that
    // asked, which is this one.
    Ok(ran
        .code()
        .and_then(|code| u8::try_from(code).ok())
        .and_then(Exit::of)
        .map_or(ExitCode::FAILURE, Into::into))
}

/// Say why the door refused `pr`, and end as refused.
fn refused(pr: u64, refusal: &Refusal) -> ExitCode {
    eprintln!("merge-net: the door refuses #{pr}: {refusal}");
    Exit::Refused.into()
}

/// The lock at `path`, waiting for another ask of the same pull request
/// to end, and saying so while it waits. It goes when the door ends, or
/// when its process dies.
fn held(path: &Path, pr: u64) -> Result<fs::File, String> {
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
        .map_err(|error| format!("opening {}: {error}", path.display()))?;
    match lock.try_lock() {
        Ok(()) => return Ok(lock),
        Err(fs::TryLockError::WouldBlock) => {
            eprintln!("merge-net: waiting for another ask of the door for #{pr} to end");
        }
        Err(fs::TryLockError::Error(error)) => {
            return Err(format!("locking {}: {error}", path.display()));
        }
    }
    lock.lock()
        .map_err(|error| format!("locking {}: {error}", path.display()))?;
    Ok(lock)
}

/// What GitHub says of pull request `pr` of `slug`, as JSON, given two
/// minutes, as the train gives it, since a room waits on the answer.
fn asked_github(pr: u64, slug: &str) -> Result<String, String> {
    let ran = Command::new("timeout")
        .arg("120")
        .args([
            "gh",
            "pr",
            "view",
            &pr.to_string(),
            "-R",
            slug,
            "--json",
            FIELDS,
        ])
        .output()
        .map_err(|error| format!("running gh: {error}"))?;
    if ran.status.success() {
        Ok(String::from_utf8_lossy(&ran.stdout).into_owned())
    } else {
        Err(format!(
            "couldn't ask GitHub of #{pr} in {slug}: {}",
            String::from_utf8_lossy(&ran.stderr).trim()
        ))
    }
}

/// The pull request in GitHub's answer `said`, the seam the door's tests
/// read fixtures through, as no test asks GitHub.
pub fn pull_of(said: &str) -> Result<Pull, String> {
    let value: Value =
        serde_json::from_str(said).map_err(|error| format!("it isn't JSON: {error}"))?;
    let text = |key: &str| {
        value[key]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| format!("it gave no {key}"))
    };
    let flag = |key: &str| {
        value[key]
            .as_bool()
            .ok_or_else(|| format!("it gave no {key}"))
    };
    Ok(Pull {
        state: text("state")?,
        head: text("headRefOid")?,
        base: text("baseRefName")?,
        draft: flag("isDraft")?,
        branch: text("headRefName")?,
        cross_repository: flag("isCrossRepository")?,
    })
}

/// The branch to clone for `pull`, asked of at `head`, or why the door
/// won't ready it. It's pure, so each refusal is tested on what GitHub
/// might say; whether git takes the branch's name is [`well_formed`]'s.
pub fn judge(pull: &Pull, head: &str) -> Result<Branch, Refusal> {
    if pull.state != "OPEN" {
        return Err(Refusal::NotOpen {
            state: pull.state.clone(),
        });
    }
    if pull.head != head {
        return Err(Refusal::OtherHead {
            asked: head.to_owned(),
            github: pull.head.clone(),
        });
    }
    if pull.base != "main" {
        return Err(Refusal::NotIntoMain {
            base: pull.base.clone(),
        });
    }
    if pull.draft {
        return Err(Refusal::Draft);
    }
    if pull.cross_repository {
        return Err(Refusal::CrossRepository {
            branch: pull.branch.clone(),
        });
    }
    Ok(Branch(pull.branch.clone()))
}

/// `branch`, when git takes its name as a branch's as it stands. A name
/// git would read as something else, as `@{-1}`, which it expands to the
/// branch checked out before, is refused too, since the door would then
/// clone another branch than the one GitHub named.
pub fn well_formed(branch: Branch) -> Result<Branch, Refusal> {
    let refuse = |said: String| Refusal::BadBranch {
        branch: branch.0.clone(),
        said,
    };
    let ran = Command::new("git")
        .args(["check-ref-format", "--branch", &branch.0])
        .current_dir(env::temp_dir())
        .output()
        .map_err(|error| refuse(format!("running git: {error}")))?;
    let taken = String::from_utf8_lossy(&ran.stdout).trim().to_owned();
    if !ran.status.success() {
        let said = String::from_utf8_lossy(&ran.stderr).trim().to_owned();
        return Err(refuse(if said.is_empty() {
            "git check-ref-format refused it".to_owned()
        } else {
            said
        }));
    }
    if taken != branch.0 {
        return Err(refuse(format!("git reads it as {taken:?}")));
    }
    Ok(branch)
}

/// The folder a door at `head` of `pr` clones into, named for both, so a
/// pull request's next head never reuses a clone of its last.
pub fn folder_name(pr: u64, head: &str) -> String {
    format!("{pr}-{}", short(head))
}

/// Which of `names`, the door's folders, are clones of `pr` at another head
/// than `head`, which a door at `head` clears.
pub fn other_heads<'a>(names: &[&'a str], pr: u64, head: &str) -> Vec<&'a str> {
    let ours = folder_name(pr, head);
    let prefix = format!("{pr}-");
    names
        .iter()
        .copied()
        .filter(|name| name.starts_with(&prefix) && *name != ours)
        .collect()
}

/// Remove the clones in `doors` of `pr` at other heads. One that can't be
/// removed is said and left, since it only costs room, and the door's own
/// clone doesn't need it gone.
fn clear_other_heads(doors: &Path, pr: u64, head: &str) {
    let Ok(entries) = fs::read_dir(doors) else {
        return;
    };
    let folders: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    let names: Vec<&str> = folders.iter().map(String::as_str).collect();
    for name in other_heads(&names, pr, head) {
        let folder = doors.join(name);
        if let Err(error) = fs::remove_dir_all(&folder) {
            eprintln!(
                "merge-net: the door's clone {} wasn't removed: {error}",
                folder.display()
            );
        }
    }
}

/// Make `folder` a clone of `url` at `branch`, reusing one already there
/// at `head` on that branch, and check it took `head`. Its error is one
/// the door couldn't get past; its refusal, that the branch moved.
fn cloned(
    root: &Path,
    url: &str,
    branch: &Branch,
    folder: &Path,
    head: &str,
) -> Result<Result<(), Refusal>, String> {
    if folder.exists() {
        let at = git_in(folder, &["rev-parse", "HEAD"]);
        let on = git_in(folder, &["symbolic-ref", "--quiet", "--short", "HEAD"]);
        if at.as_deref() == Ok(head) && on.as_deref() == Ok(branch.0.as_str()) {
            eprintln!(
                "merge-net: the door reuses its clone of {} at {}",
                branch.0,
                short(head)
            );
            return Ok(Ok(()));
        }
        // A clone cut short, or left at another commit, is made again
        // rather than trusted.
        fs::remove_dir_all(folder)
            .map_err(|error| format!("removing {}: {error}", folder.display()))?;
    }
    eprintln!(
        "merge-net: the door clones {} at {} into {}",
        branch.0,
        short(head),
        folder.display()
    );
    let reference: PathBuf = root.join("core");
    let ran = Command::new("git")
        .args(["clone", "--quiet", "--no-tags", "--reference-if-able"])
        .arg(&reference)
        .args(["-b", &branch.0, url])
        .arg(folder)
        .output()
        .map_err(|error| format!("running git clone: {error}"))?;
    if !ran.status.success() {
        let _ = fs::remove_dir_all(folder);
        return Err(format!(
            "couldn't clone {} of {url}: {}",
            branch.0,
            String::from_utf8_lossy(&ran.stderr).trim()
        ));
    }
    let cloned = git_in(folder, &["rev-parse", "HEAD"])?;
    if cloned != head {
        // A clone at another head would be judged as this one, so it goes,
        // and the next ask clones afresh.
        let _ = fs::remove_dir_all(folder);
        return Ok(Err(Refusal::Moved {
            asked: head.to_owned(),
            cloned,
        }));
    }
    Ok(Ok(()))
}

/// What git says in `folder`, or why it failed.
fn git_in(folder: &Path, arguments: &[&str]) -> Result<String, String> {
    let ran = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args(arguments)
        .output()
        .map_err(|error| format!("running git: {error}"))?;
    if ran.status.success() {
        Ok(String::from_utf8_lossy(&ran.stdout).trim().to_owned())
    } else {
        Err(format!(
            "git {} in {} failed: {}",
            arguments.join(" "),
            folder.display(),
            String::from_utf8_lossy(&ran.stderr).trim()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &str = "08c885690815f5b468316df71fe33c79bf174603";
    const OTHER: &str = "2c3d81b93aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    /// GitHub's answer for an open pull request into main from one of the
    /// repository's own branches at [`HEAD`], with `changed` set over it.
    fn said(changed: &[(&str, Value)]) -> String {
        let mut value = serde_json::json!({
            "state": "OPEN",
            "headRefOid": HEAD,
            "baseRefName": "main",
            "isDraft": false,
            "headRefName": "jitsusama/way-holds-detached-asks",
            "isCrossRepository": false,
        });
        for (key, changed) in changed {
            value[*key] = changed.clone();
        }
        value.to_string()
    }

    fn judged(changed: &[(&str, Value)]) -> Result<Branch, Refusal> {
        judge(
            &pull_of(&said(changed)).expect("GitHub's answer read"),
            HEAD,
        )
    }

    #[test]
    fn an_open_pull_into_main_at_the_head_asked_gives_its_branch() {
        assert_eq!(
            judged(&[]),
            Ok(Branch("jitsusama/way-holds-detached-asks".to_owned()))
        );
    }

    #[test]
    fn a_pull_that_isnt_open_is_refused_saying_its_state() {
        let refused = judged(&[("state", "MERGED".into())]);
        assert_eq!(
            refused,
            Err(Refusal::NotOpen {
                state: "MERGED".to_owned()
            })
        );
        assert!(
            refused.unwrap_err().to_string().contains("MERGED"),
            "the refusal never said what GitHub said"
        );
    }

    #[test]
    fn a_pull_at_another_head_is_refused_saying_both() {
        let refused = judged(&[("headRefOid", OTHER.into())]);
        assert_eq!(
            refused,
            Err(Refusal::OtherHead {
                asked: HEAD.to_owned(),
                github: OTHER.to_owned()
            })
        );
        let said = refused.unwrap_err().to_string();
        assert!(
            said.contains(short(HEAD)) && said.contains(short(OTHER)),
            "the refusal didn't name both heads: {said}"
        );
    }

    #[test]
    fn a_pull_into_another_base_is_refused_saying_it() {
        let refused = judged(&[("baseRefName", "next".into())]);
        assert_eq!(
            refused,
            Err(Refusal::NotIntoMain {
                base: "next".to_owned()
            })
        );
        let said = refused.unwrap_err().to_string();
        assert!(
            said.contains("next"),
            "the refusal didn't name the base it was into: {said}"
        );
    }

    #[test]
    fn a_draft_is_refused() {
        let refused = judged(&[("isDraft", true.into())]);
        assert_eq!(refused, Err(Refusal::Draft));
        let said = refused.unwrap_err().to_string();
        assert!(
            said.contains("isDraft"),
            "the refusal didn't say what GitHub said made it a draft: {said}"
        );
    }

    #[test]
    fn a_forks_pull_is_refused() {
        let refused = judged(&[("isCrossRepository", true.into())]);
        assert_eq!(
            refused,
            Err(Refusal::CrossRepository {
                branch: "jitsusama/way-holds-detached-asks".to_owned()
            })
        );
        let said = refused.unwrap_err().to_string();
        assert!(
            said.contains("isCrossRepository"),
            "the refusal didn't say what GitHub said made it a fork's: {said}"
        );
    }

    #[test]
    fn an_answer_short_of_a_field_is_no_pull() {
        let mut value: Value = serde_json::from_str(&said(&[])).unwrap();
        value.as_object_mut().unwrap().remove("isCrossRepository");
        let read = pull_of(&value.to_string());
        assert!(
            read.as_ref()
                .is_err_and(|why| why.contains("isCrossRepository")),
            "an answer with no isCrossRepository was read as {read:?}"
        );
        let read = pull_of("not json");
        assert!(read.is_err(), "an answer not JSON was read as {read:?}");
    }

    #[test]
    fn a_branch_git_wont_take_as_it_stands_is_refused() {
        assert_eq!(
            well_formed(Branch("jitsusama/merge-net-door".to_owned())),
            Ok(Branch("jitsusama/merge-net-door".to_owned()))
        );
        for name in ["-b", "bad..name", "a b", "ends.lock", "@{-1}", ""] {
            assert!(
                matches!(
                    well_formed(Branch(name.to_owned())),
                    Err(Refusal::BadBranch { branch, .. }) if branch == name
                ),
                "{name:?} was taken as a branch"
            );
        }
    }

    #[test]
    fn a_clone_is_named_for_its_pull_and_head() {
        assert_eq!(folder_name(7361, HEAD), "7361-08c885690815");
    }

    #[test]
    fn only_the_same_pulls_clones_at_other_heads_are_cleared() {
        let names = [
            "7361-08c885690815",
            "7361-2c3d81b93aaa",
            "73610-2c3d81b93aaa",
            "736-2c3d81b93aaa",
            "8000-08c885690815",
            "7361.lock",
        ];
        assert_eq!(other_heads(&names, 7361, HEAD), ["7361-2c3d81b93aaa"]);
    }
}
