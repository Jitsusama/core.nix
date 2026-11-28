//! Heads a review round read without naming them. Two kinds:
//!
//! - A change below the tip of a stack that one stack round read. The
//!   round records only the tip, which holds every commit of the changes
//!   below it, so each lower change asked a round of its own. On 27
//!   November that undid folding rounds into stacks of about five, and the
//!   start line held 61 asks.
//! - A head pushed after a round read an earlier commit of its change,
//!   where what came after touches only docs and tests, or merges main in
//!   cleanly. No reviewer's question turns on those, and the gate's checks
//!   at the head still have to pass.
//!
//! Both are asked only once no round read the head itself, so they're on
//! the refusal path, and each narrows the rounds by git before a base is
//! put to `on_main`: asking after every round's base once held `ready`
//! past 15 minutes (`same.rs`).

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;

use super::seen;
use crate::{git_at, short};

/// The most commits of a change walked for what came after its read one,
/// so a head on a branch that never shared main's history isn't walked
/// through whole.
const MOST_WALKED: usize = 500;

/// What the rounds say of a head none of them named.
#[derive(Debug, PartialEq, Eq)]
pub enum Folded {
    /// A round read it all the same, and why.
    Read(String),
    /// A round might have, but doesn't speak for it, and why not: the
    /// first commit and path that stopped it, or what git couldn't say.
    Refused(String),
    /// No round comes near it, so there's nothing to add to its refusal.
    Silent,
}

impl Folded {
    /// This, or `other` where this reads nothing; two refusals are both
    /// said, since each names a different round.
    fn or(self, other: Self) -> Self {
        match (self, other) {
            (read @ Self::Read(_), _) | (_, read @ Self::Read(_)) => read,
            (Self::Refused(one), Self::Refused(two)) => Self::Refused(format!("{one}; {two}")),
            (refused @ Self::Refused(_), Self::Silent) | (Self::Silent, refused) => refused,
        }
    }
}

/// Why `commit`, which no round named, was read all the same, in the repo
/// at `repo`: within a stack whose tip a stack round of `rounds` read, or
/// after a commit a round read with only docs, tests and clean merges of
/// `main` since. `on_main` says whether a range round's base is on main,
/// as for any round, so only rounds that read their commit in full count;
/// it's asked only of the rounds git has already found near `commit`.
pub fn read(
    repo: &Path,
    commit: &str,
    main: &str,
    rounds: &HashSet<seen::Read>,
    mut on_main: impl FnMut(&str) -> bool,
) -> Folded {
    let stack = within_a_stack(repo, commit, main, rounds, &mut on_main);
    if matches!(stack, Folded::Read(_)) {
        return stack;
    }
    stack.or(after_a_read(repo, commit, main, rounds, &mut on_main))
}

/// Whether some round of `rounds` of `commit` read it in full: a change's
/// round, or a range's from a base `on_main` finds on main.
fn read_in_full(
    rounds: &HashSet<seen::Read>,
    commit: &str,
    stack: bool,
    on_main: &mut impl FnMut(&str) -> bool,
) -> bool {
    rounds.iter().any(|read| {
        read.commit == commit
            && (read.stack || !stack)
            && read.from.as_deref().is_none_or(&mut *on_main)
    })
}

/// Whether a stack round of `rounds` read a tip that holds `commit`, and
/// read of what `commit`'s change touched just what `commit` holds.
///
/// A tip holds every commit below it, but a reviewer reads each file as
/// the tip has it, so a lower change pointed back at an earlier commit,
/// one with a bug a later commit fixed, isn't what was read. The credit
/// stands only when nothing from `commit` to the tip changes a path that
/// `commit`'s change touched.
///
/// The harness shows each change of a stack its own diff from its base on
/// the host (`gh pr diff`, in its `review/providers/github/changes.ts`),
/// and the bottom's base is the stack's trunk, which is main but needn't
/// be. A round records each member's head and no base, so a commit below
/// a bottom based elsewhere can't be told from the bottom's own, and is
/// credited here as the bottom's witness is by `same.rs`. A stack round
/// recording the base it read from would close both.
fn within_a_stack(
    repo: &Path,
    commit: &str,
    main: &str,
    rounds: &HashSet<seen::Read>,
    on_main: &mut impl FnMut(&str) -> bool,
) -> Folded {
    let mut tips: Vec<&str> = rounds
        .iter()
        .filter(|read| read.stack && read.commit != commit)
        .map(|read| read.commit.as_str())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    if tips.is_empty() {
        return Folded::Silent;
    }
    tips.sort_unstable();
    let holding = match tips_holding(repo, commit, &tips) {
        Ok(holding) => holding,
        Err(error) => {
            return Folded::Refused(format!(
                "which stack round's tip holds {} can't be told ({error})",
                short(commit)
            ));
        }
    };
    let mut said = Folded::Silent;
    for tip in holding {
        if !read_in_full(rounds, tip, true, on_main) {
            continue;
        }
        match unchanged_above(repo, commit, main, tip) {
            Ok(()) => {
                return Folded::Read(format!(
                    "{} is read: a stack round read {}, which holds it, and nothing above it \
                     changes what its change touched",
                    short(commit),
                    short(tip)
                ));
            }
            Err(why) => said = said.or(Folded::Refused(why)),
        }
    }
    said
}

/// Those of `tips` whose history holds `commit`, in their order, by one
/// git however many there are: a tip git doesn't have holds nothing.
fn tips_holding<'a>(repo: &Path, commit: &str, tips: &[&'a str]) -> Result<Vec<&'a str>, String> {
    let below = format!("^{commit}");
    let mut asked = vec![
        "rev-list",
        "--ancestry-path",
        "--ignore-missing",
        "--end-of-options",
        below.as_str(),
    ];
    asked.extend_from_slice(tips);
    let above = git_at(repo, &asked)?;
    let above: HashSet<&str> = above.lines().collect();
    Ok(tips
        .iter()
        .copied()
        .filter(|tip| above.contains(tip))
        .collect())
}

/// Whether no path that `commit`'s change touched, from its base on
/// `main`, is changed from `commit` to `tip`, or the first that is, and
/// by which commit when git can say.
fn unchanged_above(repo: &Path, commit: &str, main: &str, tip: &str) -> Result<(), String> {
    let refused = |why: String| format!("a stack round read {}, but {why}", short(tip));
    let touched = changed(repo, &format!("{main}...{commit}"), None).map_err(refused)?;
    let touched: HashSet<&str> = touched.iter().map(String::as_str).collect();
    let above = changed(repo, commit, Some(tip)).map_err(refused)?;
    let Some(path) = above.iter().find(|path| touched.contains(path.as_str())) else {
        return Ok(());
    };
    let by = git_at(
        repo,
        &[
            "rev-list",
            "--reverse",
            "--full-history",
            &format!("{commit}..{tip}"),
            "--",
            path,
        ],
    )
    .ok()
    .and_then(|by| by.lines().next().map(|by| format!(" ({})", short(by))));
    Err(refused(format!(
        "{path}, which {}'s change touched, changes above it{}",
        short(commit),
        by.unwrap_or_default()
    )))
}

/// The paths git diffs between `from` and `to`, or of the range `from`
/// alone, each side of a rename named.
fn changed(repo: &Path, from: &str, to: Option<&str>) -> Result<Vec<String>, String> {
    let mut asked = vec!["diff", "--name-only", "--no-renames", "-z", from];
    asked.extend(to);
    let said = git_at(repo, &asked)?;
    Ok(said
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect())
}

/// Whether a commit of `commit`'s change that a round of `rounds` read in
/// full is followed only by commits touching docs and tests, and merges
/// of `main` that git made cleanly. The newest read commit is the one
/// asked after, since what follows it is the least. Only the rounds of
/// the change's own commits have a base put to `on_main`.
fn after_a_read(
    repo: &Path,
    commit: &str,
    main: &str,
    rounds: &HashSet<seen::Read>,
    on_main: &mut impl FnMut(&str) -> bool,
) -> Folded {
    let not_main = format!("^{main}");
    // One past the most, so a change longer than that is told from one
    // just as long, and git stops walking there.
    let most = format!("--max-count={}", MOST_WALKED + 1);
    let change = match git_at(
        repo,
        &["rev-list", "--topo-order", &most, commit, &not_main],
    ) {
        Ok(change) => change,
        Err(error) => {
            return Folded::Refused(format!(
                "{}'s change can't be listed ({error})",
                short(commit)
            ));
        }
    };
    let change: Vec<&str> = change.lines().collect();
    if change.len() > MOST_WALKED {
        return Folded::Refused(format!(
            "{}'s change holds more than {MOST_WALKED} commits off main, so it isn't walked for \
             one a round read",
            short(commit)
        ));
    }
    let named: HashSet<&str> = rounds.iter().map(|read| read.commit.as_str()).collect();
    let mut of_the_change = change
        .iter()
        .copied()
        .filter(|one| *one != commit && named.contains(one))
        .peekable();
    let Some(&newest) = of_the_change.peek() else {
        return Folded::Silent;
    };
    let Some(reviewed) = of_the_change.find(|one| read_in_full(rounds, one, false, on_main)) else {
        return Folded::Refused(format!(
            "a round read {} of its change, but only from a base off main",
            short(newest)
        ));
    };
    match only_docs_tests_and_merges(repo, commit, main, reviewed) {
        Ok(()) => Folded::Read(format!(
            "{} is read: a round read {}, and what came after touches only docs and tests, or \
             merged main in cleanly",
            short(commit),
            short(reviewed)
        )),
        Err(why) => Folded::Refused(format!("a round read {}, but {why}", short(reviewed))),
    }
}

/// Whether every commit from `reviewed` to `commit` off `main` touches
/// only docs and tests or merges main cleanly, or the first, oldest
/// first, that doesn't, and why.
fn only_docs_tests_and_merges(
    repo: &Path,
    commit: &str,
    main: &str,
    reviewed: &str,
) -> Result<(), String> {
    let after = git_at(
        repo,
        &[
            "rev-list",
            "--topo-order",
            "--reverse",
            "--parents",
            commit,
            &format!("^{reviewed}"),
            &format!("^{main}"),
        ],
    )
    .map_err(|error| format!("what came after it can't be listed ({error})"))?;
    let mut inputs = Inputs::at(repo, commit);
    for line in after.lines() {
        let mut words = line.split(' ');
        let one = words.next().unwrap_or_default();
        let parents: Vec<&str> = words.collect();
        let judged = match parents.as_slice() {
            [_] => only_docs_and_tests(repo, one, &mut inputs),
            [first, second] => merged_main_cleanly(repo, one, [first, second], main),
            _ => Err(format!("has {} parents", parents.len())),
        };
        judged.map_err(|why| format!("{} after it {why}", short(one)))?;
    }
    Ok(())
}

/// Whether the commit `one`, of one parent, changes only docs and tests,
/// or the first file that isn't one: a file under `docs/` at the top or
/// any `.md` that nothing reads as an input (`Inputs`), or a `.test.ts`.
/// The top-level `tests/` isn't one: it holds the flake's public test
/// outputs and the gate's own rule checks.
fn only_docs_and_tests(repo: &Path, one: &str, inputs: &mut Inputs) -> Result<(), String> {
    let files = git_at(
        repo,
        &[
            "diff-tree",
            "--no-commit-id",
            "--name-only",
            "--no-renames",
            "-z",
            "-r",
            one,
        ],
    )
    .map_err(|error| format!("can't be read ({error})"))?;
    for file in files.split('\0').filter(|file| !file.is_empty()) {
        if file.ends_with(".test.ts") {
            continue;
        }
        if !is_doc(file) {
            return Err(format!("touches {file}, which is neither a doc nor a test"));
        }
        if read_by_agents(file) {
            return Err(format!(
                "touches {file}, which agents read as a skill or a guide"
            ));
        }
        if let Some(by) = inputs.naming(file)? {
            return Err(format!("touches {file}, which {by} reads"));
        }
    }
    Ok(())
}

/// Whether `file` is a doc by its place or name: under `docs/` at the top,
/// or a `.md` in any case.
fn is_doc(file: &str) -> bool {
    file.starts_with("docs/")
        || Path::new(file)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
}

/// Whether agents read `file` as instructions: a `SKILL.md`, installed as
/// a skill, an `AGENTS.md`, pasted into every review prompt, or anything
/// in a `skill` or `skills` folder.
fn read_by_agents(file: &str) -> bool {
    let mut parts = file.rsplit('/');
    let name = parts.next().unwrap_or_default();
    ["SKILL.md", "AGENTS.md"]
        .iter()
        .any(|read| name.eq_ignore_ascii_case(read))
        || parts.any(|folder| ["skill", "skills"].contains(&folder))
}

/// The files at a head that name others by a path, read once, and only
/// once a doc is asked after.
///
/// A doc named by `./` or `../` from a file that isn't one, as
/// `include_str!("../../good.md")` or a `./SKILL.md` a module installs, is
/// an input, not a doc. A path built while a program runs, a folder named
/// whole, or a file named from the repo's root without `./` isn't seen, so
/// such a doc still counts as one: the gate's checks at the head are what
/// stand behind it.
struct Inputs<'a> {
    repo: &'a Path,
    head: &'a str,
    named: Option<Result<HashMap<String, String>, String>>,
}

impl<'a> Inputs<'a> {
    fn at(repo: &'a Path, head: &'a str) -> Self {
        Self {
            repo,
            head,
            named: None,
        }
    }

    /// The file at the head that names `file`, if any does.
    fn naming(&mut self, file: &str) -> Result<Option<String>, String> {
        let (repo, head) = (self.repo, self.head);
        let named = self
            .named
            .get_or_insert_with(|| named_at(repo, head))
            .as_ref()
            .map_err(|error| format!("can't be told from an input ({error})"))?;
        Ok(named.get(file).cloned())
    }
}

/// Every path named by `./` or `../` from a file at `head` that isn't a
/// `.md`, resolved from that file's own folder, each to the first file
/// naming it, by one git.
fn named_at(repo: &Path, head: &str) -> Result<HashMap<String, String>, String> {
    let ran = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "grep",
            "-I",
            "-z",
            "-o",
            "-E",
            r"\.\.?/[A-Za-z0-9_./-]+",
            head,
        ])
        .args(["--", ":!*.md"])
        .output()
        .map_err(|error| format!("running git: {error}"))?;
    // git grep finding nothing says so by its code alone.
    let found_nothing = ran.status.code() == Some(1) && ran.stderr.is_empty();
    if !(ran.status.success() || found_nothing) {
        return Err(format!(
            "git grep failed: {}",
            String::from_utf8_lossy(&ran.stderr).trim()
        ));
    }
    let said = String::from_utf8_lossy(&ran.stdout);
    let at = format!("{head}:");
    let mut named = HashMap::new();
    for line in said.lines() {
        let Some((file, path)) = line
            .strip_prefix(&at)
            .and_then(|line| line.split_once('\0'))
        else {
            continue;
        };
        if let Some(path) = resolved(file, path) {
            named.entry(path).or_insert_with(|| file.to_owned());
        }
    }
    Ok(named)
}

/// `path` as named from `file`, from the repo's root, or nothing when it
/// climbs out of the repo.
fn resolved(file: &str, path: &str) -> Option<String> {
    let mut parts: Vec<&str> = file.split('/').collect();
    parts.pop();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            part => parts.push(part),
        }
    }
    Some(parts.join("/"))
}

/// Whether the merge `one` of `parents` took one of them from `main` and
/// holds just what git merges of the two without a conflict, so no line
/// of it was written by hand, or why not.
fn merged_main_cleanly(
    repo: &Path,
    one: &str,
    parents: [&str; 2],
    main: &str,
) -> Result<(), String> {
    let from_main = parents
        .iter()
        .any(|parent| git_at(repo, &["merge-base", "--is-ancestor", parent, main]).is_ok());
    if !from_main {
        return Err("merges a branch that isn't main".to_owned());
    }
    let merged = git_at(
        repo,
        &["merge-tree", "--write-tree", parents[0], parents[1]],
    )
    .map_err(|error| format!("merges main with a conflict ({error})"))?;
    let tree = git_at(repo, &["rev-parse", &format!("{one}^{{tree}}")])
        .map_err(|error| format!("can't be read ({error})"))?;
    if merged.lines().next() == Some(tree.as_str()) {
        Ok(())
    } else {
        Err("merges main with lines git's own merge doesn't make".to_owned())
    }
}

#[cfg(test)]
mod tests;
