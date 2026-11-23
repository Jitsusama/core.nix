//! Whether a head only pins newer commits of repos whose own changes our
//! rounds already review, which then needs no round of its own. Such a
//! head is a flake.lock that moves inputs and nothing else: what it brings
//! in was read by a council of ours where it merged, so a round here would
//! read only hashes. On 22 November a mycelium pin on the first VM test's
//! path waited 20 minutes for one, and the velocity cell ruled that it
//! needs none (1OGSE7, 2c8f8e4a): it asks no one else to review anything.
//!
//! Only a node's commit may move: its lock's `rev`, `narHash` and
//! `lastModified`, every other field as it was, so a lock can't send the
//! fetch to another host, folder or ref under our repo's name. The new
//! commit must descend from the old one and be the very commit a pull
//! request into the repo's trunk merged as, as GitHub says. Being on the
//! trunk isn't enough: the repo's train merges with merge commits, which
//! put every commit a pull request carried on the trunk, though a round
//! read only its head (on 22 November 1,150 of mycelium's 1,543). Every
//! commit the pin brings in must have come in through a pull request
//! merged into that trunk as one of those commits, since the trunk takes
//! pushes too. A pull request whose merge is its own head counts as none:
//! GitHub marks an open one merged when its head is pushed to its base,
//! and only the train's merge commits, squashes and rebases make a commit
//! of their own. Its `narHash`
//! and `lastModified` must be what Nix makes of that commit fetched
//! afresh, since Nix takes a source whose hash it already holds without
//! fetching it. A branch, a pull request's head, a commit from a fork, a
//! commit pushed straight to the trunk or a pin back to an older commit
//! needs a round. A node added, removed or renamed, an input that follows
//! another, or a source of anyone else's needs one too. Anything that
//! can't be read or asked needs a round, as before, and a lock-only head
//! is told why.
//!
//! What this leans on: that a pull request merged into the repo's trunk
//! was read by a round of ours, which the repo's own merge gate keeps,
//! not this one.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::{git_at, short};

/// The repos whose every change merges through a round of ours, by their
/// owner and name on GitHub, each with its trunk. The harness is reviewed
/// the same way but isn't an input of this flake, so it joins only with
/// the change that makes it one.
const OURS: [(&str, &str); 1] = [("Jitsusama/mycelium", "main")];

/// The lock file a pin moves.
const LOCK: &str = "flake.lock";

/// The fields of a node's lock a pin may move: those naming its commit.
const MOVES: [&str; 3] = ["rev", "narHash", "lastModified"];

/// The most commits a pin may bring in, each asked after on its own: one
/// page of GitHub's compare.
const BROUGHT_AT_MOST: usize = 100;

/// How long a pin's asks may take together before it needs a round: a
/// GitHub slow to answer each of a hundred would otherwise hold `ready`
/// for hours.
const ASKED_WITHIN: Duration = Duration::from_mins(5);

/// A node of ours whose lock moved from one commit to another.
#[derive(Debug, PartialEq, Eq)]
struct Moved {
    repo: String,
    trunk: String,
    old: String,
    new: String,
    /// The node's lock as the head has it.
    locked: Value,
}

/// What a head that moves only flake.lock is told.
#[derive(Debug, PartialEq, Eq)]
pub enum Pinned {
    /// It needs no round, and why.
    Alone(String),
    /// It needs a round after all, and why, said so the person waiting on
    /// it can tell an outage worth asking again from a pin worth fixing.
    Declined(String),
}

/// What's asked about a pin's commits, of GitHub and of Nix, each giving
/// what was said as it was said, for the functions below to judge.
pub trait Asks {
    /// GitHub's compare of `old` to `new` in `repo`: its status, its count
    /// of commits and the commits it lists.
    fn compared(&self, repo: &str, old: &str, new: &str) -> Result<Value, String>;
    /// The pull requests GitHub ties to `rev` in `repo`.
    fn pulls(&self, repo: &str, rev: &str) -> Result<Value, String>;
    /// Nix's prefetch of `rev` of `repo`, fetched afresh.
    fn prefetched(&self, repo: &str, rev: &str) -> Result<Value, String>;
}

/// GitHub's API through gh, on github.com, the host a lock's `github`
/// type fetches from, and Nix's own prefetch.
pub struct Live;

impl Asks for Live {
    fn compared(&self, repo: &str, old: &str, new: &str) -> Result<Value, String> {
        // Only what's judged comes back: a compare's files and patches
        // can run to megabytes.
        let path = format!("repos/{repo}/compare/{old}...{new}?per_page={BROUGHT_AT_MOST}");
        let wanted = "{status, total_commits, commits: [.commits[].sha]}";
        json(&answer(gh(&[&path, "--jq", wanted]))?)
    }

    fn pulls(&self, repo: &str, rev: &str) -> Result<Value, String> {
        let path = format!("repos/{repo}/commits/{rev}/pulls");
        let wanted = "[.[] | {merge_commit_sha, merged_at, head: .head.sha, \
                      base: {ref: .base.ref, repo: .base.repo.full_name}}]";
        json(&answer(gh(&[&path, "--jq", wanted]))?)
    }

    fn prefetched(&self, repo: &str, rev: &str) -> Result<Value, String> {
        let mut nix = Command::new("timeout");
        nix.args([LIMIT, "nix", "flake", "prefetch", "--json"])
            .arg(format!("github:{repo}/{rev}"));
        json(&answer(nix)?)
    }
}

/// How long, in seconds, each ask is given.
const LIMIT: &str = "120";

/// gh asking github.com's API at `args`, given [`LIMIT`].
fn gh(args: &[&str]) -> Command {
    let mut gh = Command::new("timeout");
    gh.args([LIMIT, "gh", "api", "--hostname", "github.com"])
        .args(args);
    gh
}

/// `said` read as JSON, or why it can't be.
fn json(said: &str) -> Result<Value, String> {
    serde_json::from_str(said).map_err(|why| format!("it said what can't be read: {why}"))
}

/// What `command`, a program run under `timeout`, says, or what went
/// wrong, its own last words included.
fn answer(mut command: Command) -> Result<String, String> {
    let mut args = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned());
    let limit = args.next().unwrap_or_default();
    let program = args.next().unwrap_or_default();
    let ran = command.output().map_err(|why| {
        format!(
            "{} couldn't be run for {program}: {why}",
            command.get_program().to_string_lossy()
        )
    })?;
    if ran.status.success() {
        return Ok(String::from_utf8_lossy(&ran.stdout).trim().to_owned());
    }
    let said = String::from_utf8_lossy(&ran.stderr);
    let said = said.lines().last().unwrap_or("nothing").trim();
    Err(match ran.status.code() {
        Some(124) => format!("{program} didn't answer within {limit} s"),
        _ => format!("{program} failed, saying {said}"),
    })
}

/// Whether GitHub's `compared` says its head descends from its base and
/// isn't it.
fn ahead(compared: &Value) -> bool {
    compared["status"].as_str() == Some("ahead")
}

/// The commits `compared` says its head brings in, when it lists them
/// all and they're no more than [`BROUGHT_AT_MOST`]; why not otherwise.
fn brought(compared: &Value) -> Result<Vec<String>, String> {
    let unread = || "GitHub's compare of them can't be read".to_owned();
    let listed: Vec<String> = compared["commits"]
        .as_array()
        .ok_or_else(unread)?
        .iter()
        .map(|sha| sha.as_str().map(str::to_owned))
        .collect::<Option<_>>()
        .ok_or_else(unread)?;
    let total = compared["total_commits"]
        .as_u64()
        .and_then(|total| usize::try_from(total).ok())
        .ok_or_else(unread)?;
    if total > BROUGHT_AT_MOST {
        return Err(format!(
            "it brings in {total} commits, more than the {BROUGHT_AT_MOST} each asked after \
             alone"
        ));
    }
    if total != listed.len() {
        return Err(format!(
            "GitHub's compare lists {} of the {total} commits it brings in",
            listed.len()
        ));
    }
    Ok(listed)
}

/// The commit `pull` merged into `trunk` of `repo` as, when it's a pull
/// request merged there that made a commit of its own: one whose merge is
/// its own head may be only a push of its head to the trunk.
fn merged_into<'a>(pull: &'a Value, repo: &str, trunk: &str) -> Option<&'a str> {
    let merge = pull["merge_commit_sha"].as_str()?;
    (!pull["merged_at"].is_null()
        && pull["base"]["ref"].as_str() == Some(trunk)
        && pull["base"]["repo"].as_str() == Some(repo)
        && pull["head"].as_str().is_some_and(|head| head != merge))
    .then_some(merge)
}

/// Whether `pulls` holds a pull request merged into `trunk` of `repo` as
/// one of `within`, which brought the commit they're tied to in with the
/// pin: one merged elsewhere, as a squash leaves its own commits, didn't.
fn came_in_merged(pulls: &Value, repo: &str, trunk: &str, within: &[String]) -> bool {
    pulls.as_array().is_some_and(|pulls| {
        pulls.iter().any(|pull| {
            merged_into(pull, repo, trunk)
                .is_some_and(|merge| within.iter().any(|in_| in_ == merge))
        })
    })
}

/// Whether `pulls` holds a pull request merged into `trunk` of `repo` as
/// `rev`: a merge commit, a squash or a rebase's last commit.
fn merged_as(pulls: &Value, repo: &str, rev: &str, trunk: &str) -> bool {
    pulls.as_array().is_some_and(|pulls| {
        pulls
            .iter()
            .any(|pull| merged_into(pull, repo, trunk) == Some(rev))
    })
}

/// What `commit`, in the repo at `repo`, is told when its change from its
/// base on `main` only moves flake.lock; nothing for any other head.
pub fn ours_alone(repo: &Path, commit: &str, main: &str, asks: &impl Asks) -> Option<Pinned> {
    let base = git_at(repo, &["merge-base", main, commit]).ok()?;
    let files = git_at(repo, &["diff", "--name-only", &base, commit]).ok()?;
    if files != LOCK {
        return None;
    }
    let lock = |at: &str| {
        git_at(repo, &["show", &format!("{at}:{LOCK}")])
            .map_err(|why| format!("its {LOCK} can't be read at {}: {why}", short(at)))
    };
    let moved = match lock(&base).and_then(|before| moved_ours(&before, &lock(commit)?)) {
        Ok(moved) => moved,
        Err(why) => return Some(Pinned::Declined(why)),
    };
    for pin in &moved {
        if let Err(why) = vouched(pin, asks, ASKED_WITHIN) {
            return Some(Pinned::Declined(why));
        }
    }
    let repos: Vec<&str> = moved.iter().map(|pin| pin.repo.as_str()).collect();
    Some(Pinned::Alone(format!(
        "{} moves only {LOCK}, pinning {} to newer commits that pull requests into its trunk \
         merged as, whose own changes merge through rounds of ours, so it needs no round of \
         its own",
        short(commit),
        repos.join(" and ")
    )))
}

/// Whether `pin`'s new commit is newer, brings in only what pull requests
/// merged with it, is one's merge itself and is locked as Nix makes it,
/// asking each only once the one before holds, all `within` that long.
fn vouched(pin: &Moved, asks: &impl Asks, within: Duration) -> Result<(), String> {
    let (repo, trunk) = (pin.repo.as_str(), pin.trunk.as_str());
    let (old, new) = (short(&pin.old), short(&pin.new));
    let began = Instant::now();
    let in_time = || {
        (began.elapsed() <= within).then_some(()).ok_or_else(|| {
            format!(
                "asking after {repo} from {old} to {new} took longer than {} s",
                within.as_secs()
            )
        })
    };
    let compared = asks
        .compared(repo, &pin.old, &pin.new)
        .map_err(|why| format!("asking GitHub whether {repo} at {new} is newer: {why}"))?;
    if !ahead(&compared) {
        return Err(format!("{repo} at {new} isn't newer than at {old}"));
    }
    let commits =
        brought(&compared).map_err(|why| format!("from {old} to {new} of {repo}, {why}"))?;
    // The new commit is asked after by name, whatever the compare lists.
    if commits.last() != Some(&pin.new) {
        return Err(format!(
            "GitHub's compare of {repo} from {old} to {new} doesn't end at {new}"
        ));
    }
    for brought in commits.iter().rev() {
        in_time()?;
        let pulls = asks.pulls(repo, brought).map_err(|why| {
            format!(
                "asking GitHub which pull request brought {} into {repo}: {why}",
                short(brought)
            )
        })?;
        if *brought == pin.new && !merged_as(&pulls, repo, brought, trunk) {
            return Err(format!(
                "no pull request into {repo}'s {trunk} merged as {new}, so no round need have \
                 read it"
            ));
        }
        if !came_in_merged(&pulls, repo, trunk, &commits) {
            return Err(format!(
                "{} came onto {repo}'s {trunk} through no pull request merged there with \
                 this pin",
                short(brought)
            ));
        }
    }
    in_time()?;
    let prefetched = asks
        .prefetched(repo, &pin.new)
        .map_err(|why| format!("asking Nix for {repo} at {new}: {why}"))?;
    let fetched = &prefetched["locked"];
    for field in ["narHash", "lastModified"] {
        if fetched.get(field).is_none() {
            return Err(format!(
                "Nix's prefetch of {repo} at {new} gave no {field} to compare with"
            ));
        }
        if pin.locked.get(field) != fetched.get(field) {
            return Err(format!(
                "its {field} for {repo} at {new} isn't what Nix makes of that commit"
            ));
        }
    }
    Ok(())
}

/// The nodes of ours whose commits moved from `before` to `after`, two
/// flake.lock files, when nothing else in them did; why not otherwise.
fn moved_ours(before: &str, after: &str) -> Result<Vec<Moved>, String> {
    let parsed = |lock: &str| {
        serde_json::from_str::<Value>(lock).map_err(|why| format!("its {LOCK} isn't JSON: {why}"))
    };
    let (before, after) = (parsed(before)?, parsed(after)?);
    let nodes = |lock: &Value| {
        lock["nodes"]
            .as_object()
            .cloned()
            .ok_or_else(|| format!("its {LOCK} holds no nodes"))
    };
    let (was, now) = (nodes(&before)?, nodes(&after)?);
    if but(&before, &["nodes"]) != but(&after, &["nodes"]) {
        return Err(format!("it changes the {LOCK}'s version or root"));
    }
    if !was.keys().eq(now.keys()) {
        return Err("it adds, removes or renames a node".to_owned());
    }
    let mut moved = Vec::new();
    for (name, node) in &now {
        let old = &was[name];
        if old == node {
            continue;
        }
        let Some((repo, trunk)) = ours(old) else {
            return Err(format!("it moves {name}, which isn't a repo of ours"));
        };
        let (locked, unlocked) = (&old["locked"], &node["locked"]);
        if but(old, &["locked"]) != but(node, &["locked"]) {
            return Err(format!(
                "it changes {name}'s inputs or what flake.nix asks of it"
            ));
        }
        if but(locked, &MOVES) != but(unlocked, &MOVES) {
            return Err(format!("it changes {name}'s lock beyond its commit"));
        }
        let commit = |locked: &Value| {
            commit_of(locked).ok_or_else(|| format!("{name}'s lock names no full commit"))
        };
        let (from, to) = (commit(locked)?, commit(unlocked)?);
        if from == to {
            return Err(format!("it changes {name}'s hash but not its commit"));
        }
        moved.push(Moved {
            repo: repo.to_owned(),
            trunk: trunk.to_owned(),
            old: from,
            new: to,
            locked: unlocked.clone(),
        });
    }
    if moved.is_empty() {
        return Err("it moves no node".to_owned());
    }
    Ok(moved)
}

/// The fields of the object `value` but those in `left`, or nothing if it
/// isn't one.
fn but<'a>(value: &'a Value, left: &[&str]) -> Option<Vec<(&'a String, &'a Value)>> {
    value.as_object().map(|fields| {
        fields
            .iter()
            .filter(|(key, _)| !left.contains(&key.as_str()))
            .collect()
    })
}

/// The repo of ours `node` is locked to, with its trunk, if it is one.
fn ours(node: &Value) -> Option<(&'static str, &'static str)> {
    let locked = &node["locked"];
    if locked["type"].as_str() != Some("github") {
        return None;
    }
    let repo = format!("{}/{}", locked["owner"].as_str()?, locked["repo"].as_str()?);
    OURS.iter().copied().find(|(ours, _)| *ours == repo)
}

/// The commit a lock names, when it names one in full, as Nix writes it
/// and GitHub lists it: anything shorter could be a branch or a tag,
/// which GitHub would read as of today, and capitals would never match
/// a commit GitHub lists.
fn commit_of(locked: &Value) -> Option<String> {
    let rev = locked["rev"].as_str()?;
    (rev.len() == 40
        && rev
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')))
    .then(|| rev.to_owned())
}

#[cfg(test)]
pub(crate) mod tests;
