//! The net's world as it is: its own checkout of the repository, git and
//! changed-checks there, `nix-offload there` for each VM test, which sends
//! it to the VM builder, and gh for the PRs it lands. git, nix and gh are
//! the person's own, with their settings, so a commit is signed as theirs
//! are, and they're found on the path rather than pinned.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::holds;
use crate::state::{now, short};
use crate::watch::{Bad, Changed, Merged, World};

/// Where the quarantine list is in the repository.
pub const QUARANTINE: &str = "tools/merge-net/quarantine.json";

/// The longest a VM test may run before it's stopped and counts as failed:
/// the slowest pass in well under an hour on a builder of its own.
const LONGEST: Duration = Duration::from_secs(2 * 60 * 60);

/// The world as the net finds it.
pub struct Real {
    /// The net's state folder, where its checkout is.
    pub state: PathBuf,
    /// The repository's URL on GitHub, as origin.
    pub url: String,
    /// The checks' system.
    pub system: String,
    /// When the machine booted, in milliseconds, for the holds since.
    pub booted: u64,
    /// The branch it follows and lands its changes on: main, or a
    /// throwaway branch to prove the net on.
    pub branch: String,
}

impl Real {
    /// The branch it follows, as the checkout tracks it.
    fn followed(&self) -> String {
        format!("refs/remotes/origin/{}", self.branch)
    }

    fn checkout(&self) -> PathBuf {
        self.state.join("core")
    }

    /// The repository's `owner/name` on GitHub.
    fn slug(&self) -> String {
        let path = self
            .url
            .trim_start_matches("https://github.com/")
            .trim_start_matches("git@github.com:")
            .trim_end_matches('/');
        path.strip_suffix(".git").unwrap_or(path).to_owned()
    }

    /// The flake a builder fetches `commit` from by itself.
    fn flake(&self, commit: &str) -> String {
        format!("github:{}/{commit}", self.slug())
    }

    fn git(&self, arguments: &[&str]) -> Result<String, String> {
        run(Command::new("git")
            .arg("-C")
            .arg(self.checkout())
            .args(arguments))
    }

    /// Make the checkout when there's none yet.
    fn cloned(&self) -> Result<(), String> {
        if self.checkout().join(".git").exists() {
            return Ok(());
        }
        run(Command::new("git")
            .args(["clone", "--quiet", &self.url])
            .arg(self.checkout()))
        .map(|_| ())
    }

    /// Land a change on main through a PR merged on its own checks: a
    /// branch from main, `change` made in the checkout and committed as
    /// `message`, pushed, changed-checks passing on it, then a PR merged
    /// only at the commit checked.
    fn land(
        &self,
        branch: &str,
        message: &str,
        title: &str,
        body: &str,
        change: impl FnOnce() -> Result<(), String>,
    ) -> Result<Merged, String> {
        let followed = self.followed();
        self.git(&["fetch", "--quiet", "origin", &self.branch])?;
        self.git(&["checkout", "--quiet", "--force", "-B", branch, &followed])?;
        change()?;
        let (subject, rest) = message.split_once("\n\n").unwrap_or((message, ""));
        let mut commit = vec!["commit", "--quiet", "-m", subject];
        if !rest.is_empty() {
            commit.extend(["-m", rest]);
        }
        self.git(&commit)?;
        let sha = self.git(&["rev-parse", "HEAD"])?;
        self.git(&["push", "--quiet", "--force", "-u", "origin", branch])?;
        let log = self
            .state
            .join("logs")
            .join(format!("land-{}.log", branch.replace('/', "-")));
        let checked = Command::new("changed-checks")
            .args(["--base", &followed])
            .current_dir(self.checkout())
            .stdout(file(&log)?)
            .stderr(file(&log)?)
            .status()
            .map_err(|error| format!("running changed-checks: {error}"))?;
        if !checked.success() {
            return Err(format!("its checks failed, {}", log.display()));
        }
        let slug = self.slug();
        let made = run(Command::new("gh").args([
            "pr",
            "create",
            "--repo",
            &slug,
            "--base",
            &self.branch,
            "--head",
            branch,
            "--title",
            title,
            "--body",
            body,
        ]))?;
        let pr: u64 = made
            .rsplit('/')
            .next()
            .and_then(|number| number.trim().parse().ok())
            .ok_or_else(|| format!("gh made a PR it didn't name: {made}"))?;
        run(Command::new("gh").args([
            "pr",
            "merge",
            &pr.to_string(),
            "--repo",
            &slug,
            "--merge",
            "--match-head-commit",
            &sha,
        ]))?;
        // GitHub may name the merge's commit a moment after it merges; the
        // hold clears only once it's known and on main.
        let mut merged = None;
        for _ in 0..10 {
            merged = run(Command::new("gh").args([
                "pr",
                "view",
                &pr.to_string(),
                "--repo",
                &slug,
                "--json",
                "mergeCommit",
                "--jq",
                ".mergeCommit.oid",
            ]))
            .ok()
            .filter(|oid| !oid.is_empty() && oid != "null");
            if merged.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_secs(6));
        }
        Ok(Merged { pr, commit: merged })
    }
}

impl World for Real {
    fn held(&self) -> Option<String> {
        if self.state.join("pause").exists() {
            return Some(format!("{} is there", self.state.join("pause").display()));
        }
        holds::standing(&holds::messages(), self.booted)
    }

    fn head(&self) -> Result<String, String> {
        self.cloned()?;
        self.git(&["fetch", "--quiet", "origin", &self.branch])?;
        self.git(&["rev-parse", &self.followed()])
    }

    fn merges(&self, base: &str, head: &str) -> Result<Vec<String>, String> {
        let listed = self.git(&[
            "rev-list",
            "--first-parent",
            "--reverse",
            &format!("{base}..{head}"),
        ])?;
        Ok(listed.lines().map(str::to_owned).collect())
    }

    fn subject(&self, commit: &str) -> String {
        self.git(&["log", "-1", "--format=%s", commit])
            .unwrap_or_default()
    }

    fn changed(&self, base: &str, head: &str) -> Result<Changed, String> {
        // What two commits' checks differ in never changes, so it's kept.
        let kept = self
            .state
            .join("changed")
            .join(format!("{}-{}.txt", short(base), short(head)));
        if let Ok(said) = fs::read_to_string(&kept) {
            return Ok(Changed::parse(&said));
        }
        self.git(&["checkout", "--quiet", "--force", "--detach", head])?;
        let log = self
            .state
            .join("logs")
            .join(format!("changed-{}.log", short(head)));
        let ran = Command::new("changed-checks")
            .args(["--base", base, "--name-only", "--system", &self.system])
            .current_dir(self.checkout())
            .stderr(file(&log)?)
            .output()
            .map_err(|error| format!("running changed-checks: {error}"))?;
        let said = String::from_utf8_lossy(&ran.stdout).into_owned();
        let changed = Changed::parse(&said);
        // It fails when a check no longer evaluates, and names it; failing
        // otherwise, it couldn't compare, and the round is tried again.
        if !ran.status.success() && changed.broken.is_empty() {
            return Err(format!(
                "changed-checks couldn't compare, {}",
                log.display()
            ));
        }
        fs::create_dir_all(self.state.join("changed"))
            .and_then(|()| fs::write(&kept, &said))
            .map_err(|error| format!("keeping what changed: {error}"))?;
        Ok(changed)
    }

    fn has_check(&self, commit: &str, check: &str) -> Result<bool, String> {
        let flake = format!(
            "git+file://{}?rev={commit}#checks.{}",
            self.checkout().display(),
            self.system
        );
        let asked = format!("checks: builtins.hasAttr \"{check}\" checks");
        let said = run(Command::new("nix").args(["eval", "--json", &flake, "--apply", &asked]))?;
        Ok(said == "true")
    }

    fn build(&self, commit: &str, check: &str, log: &Path) -> Option<i32> {
        let attribute = format!("checks.{}.{check}", self.system);
        let output = file(log).ok()?;
        let mut child = Command::new("nix-offload")
            .args(["there", "-L", &self.flake(commit), &attribute])
            .stdin(Stdio::null())
            .stdout(output.try_clone().ok()?)
            .stderr(output)
            .spawn()
            .ok()?;
        let began = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => return status.code(),
                Ok(None) if began.elapsed() > LONGEST => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                Ok(None) => std::thread::sleep(Duration::from_secs(5)),
                Err(_) => return None,
            }
        }
    }

    fn quarantined(&self, head: &str) -> Vec<String> {
        self.git(&["show", &format!("{head}:{QUARANTINE}")])
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .map(|list| tests_in(&list))
            .unwrap_or_default()
    }

    fn revert(&self, bad: &Bad) -> Result<Merged, String> {
        let what = bad
            .pr
            .map_or_else(|| short(&bad.commit).to_owned(), |pr| format!("#{pr}"));
        let message = format!(
            "revert: undo {what}, which breaks a VM test\n\n{} failed twice at {}, on the same\ninput, so it comes off main until it's fixed forward.\nReverts \"{}\".",
            bad.test, bad.commit, bad.subject
        );
        let parents = self.git(&["rev-list", "--parents", "-n", "1", &bad.commit])?;
        let merge = parents.split_whitespace().count() > 2;
        let body = revert_body(bad, &unnamed(&tail(Path::new(&bad.log), 15), &hostname()));
        self.land(
            &format!("merge-net/revert-{}", short(&bad.commit)),
            &message,
            &format!("Revert {what}, Which Breaks {}", bad.test),
            &body,
            || {
                let mut reverting = vec!["revert", "--no-commit"];
                if merge {
                    reverting.extend(["-m", "1"]);
                }
                reverting.push(&bad.commit);
                self.git(&reverting).map(|_| ()).inspect_err(|_| {
                    let _ = self.git(&["revert", "--abort"]);
                })
            },
        )
    }

    fn quarantine(
        &self,
        test: &str,
        owner: Option<u64>,
        commit: &str,
        reason: &str,
    ) -> Result<Merged, String> {
        let path = self.checkout().join(QUARANTINE);
        let body = quarantine_body(test, owner, commit, reason);
        self.land(
            &format!("merge-net/quarantine-{test}"),
            &format!("test(merge-net): quarantine a flaky VM test\n\n{test} {reason}, so the net reverts nothing for it\nuntil its owner makes it steady."),
            &format!("Quarantine {test}, Which Failed Then Passed"),
            &body,
            || {
                let mut list = fs::read_to_string(&path)
                    .ok()
                    .and_then(|text| serde_json::from_str::<Value>(&text).ok())
                    .unwrap_or_else(|| json!([]));
                let entries = list.as_array_mut().ok_or("the quarantine list isn't a list")?;
                if !tests_in(&json!(entries.clone())).iter().any(|held| held == test) {
                    entries.push(json!({
                        "test": test, "reason": reason, "owner_pr": owner, "commit": commit, "at": now(),
                    }));
                }
                let text = serde_json::to_string_pretty(&list).map_err(|error| error.to_string())?;
                fs::write(&path, text + "\n").map_err(|error| format!("writing {}: {error}", path.display()))?;
                self.git(&["add", QUARANTINE]).map(|_| ())
            },
        )
    }

    fn on_main(&self, commit: &str) -> bool {
        self.git(&["merge-base", "--is-ancestor", commit, &self.followed()])
            .is_ok()
    }

    fn wait(&self, seconds: u64) {
        std::thread::sleep(Duration::from_secs(seconds));
    }
}

/// The tests a quarantine list names.
pub fn tests_in(list: &Value) -> Vec<String> {
    list.as_array()
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry["test"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// A revert's description, in the four sections every PR has.
pub fn revert_body(bad: &Bad, log: &str) -> String {
    let what = bad
        .pr
        .map_or_else(|| short(&bad.commit).to_owned(), |pr| format!("#{pr}"));
    let mut body = String::new();
    let _ = write!(
        body,
        "### 💡 TL;DR\n\n{what} comes off main, since {test} fails at its merge. The net ran the test twice \
         at that commit, on the same input, and both runs failed.\n\n\
         ### 🧩 Problem\n\n{what} merged as {commit}, \"{subject}\", and {test} fails there twice on the same \
         input, so main and any machine switched to it carry a broken test. The failing run's log ends:\n\n\
         ```text\n{log}\n```\n\n\
         ### 🔧 Solution\n\nThis reverts {commit} against its first parent, so the files it changed are as they \
         were before it. Its owner fixes it forward on a new branch.\n\n\
         ### 🔬 Validation\n\n\
         - Checked: {test} failed twice at {short} on the VM builder, through nix-offload there.\n\
         - Checked: changed-checks passed on this branch, for the checks it names other than VM tests.\n\
         - Watch: the net runs {test} at the next head; a failure there means this didn't undo it.\n",
        test = bad.test,
        commit = bad.commit,
        subject = plain(&bad.subject),
        short = short(&bad.commit),
    );
    body
}

/// A quarantine's description, in the four sections every PR has.
pub fn quarantine_body(test: &str, owner: Option<u64>, commit: &str, reason: &str) -> String {
    let owner = owner.map_or_else(
        || "its owner".to_owned(),
        |pr| format!("the owner of #{pr}"),
    );
    format!(
        "### 💡 TL;DR\n\n{test} goes on the net's quarantine list, since it {reason}. The net still runs it, \
         but reverts nothing when it fails.\n\n\
         ### 🧩 Problem\n\n{test} {reason} on the same input, so a failure of it says nothing about the merge \
         it ran for, and reverting for it would undo good work.\n\n\
         ### 🔧 Solution\n\nThis adds {test} to the quarantine list, with its reason and owner, {owner}, who \
         makes it steady and takes it off.\n\n\
         ### 🔬 Validation\n\n\
         - Checked: {test} failed and then passed at {commit} on the VM builder.\n\
         - Checked: changed-checks passed on this branch.\n"
    )
}

/// A log's last lines, as plain text a PR can hold: no colour codes, and
/// only ASCII, so no character the prose rules refuse comes in with it.
pub fn tail(log: &Path, lines: usize) -> String {
    let text = fs::read_to_string(log).unwrap_or_default();
    let kept: Vec<String> = text
        .lines()
        .map(plain)
        .filter(|line| !line.trim().is_empty())
        .collect();
    kept[kept.len().saturating_sub(lines)..].join("\n")
}

/// This machine's name, which a log may hold and a PR mustn't, since core
/// names nothing that builds on it.
fn hostname() -> String {
    fs::read_to_string("/proc/sys/kernel/hostname")
        .map(|name| name.trim().to_owned())
        .unwrap_or_default()
}

/// `text` with `name`, in any case, said as `this machine`.
pub fn unnamed(text: &str, name: &str) -> String {
    if name.is_empty() {
        return text.to_owned();
    }
    let (lower, sought) = (text.to_ascii_lowercase(), name.to_ascii_lowercase());
    let mut out = String::new();
    let mut from = 0;
    while let Some(at) = lower[from..].find(&sought) {
        out.push_str(&text[from..from + at]);
        out.push_str("this machine");
        from += at + sought.len();
    }
    out.push_str(&text[from..]);
    out
}

/// A line without its terminal escapes, and with `?` for what isn't ASCII.
pub fn plain(line: &str) -> String {
    let mut out = String::new();
    let mut chars = line.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(next) = chars.next() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(if character.is_ascii() && !character.is_ascii_control() {
            character
        } else {
            '?'
        });
    }
    out
}

/// Open a log for writing, new each time, with its folder made.
fn file(path: &Path) -> Result<fs::File, String> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)
            .map_err(|error| format!("making {}: {error}", folder.display()))?;
    }
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("opening {}: {error}", path.display()))
}

/// Run a command, giving what it printed, trimmed, or what it said wrong.
fn run(command: &mut Command) -> Result<String, String> {
    let ran = command
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("running {:?}: {error}", command.get_program()))?;
    if !ran.status.success() {
        return Err(format!(
            "{:?} failed: {}",
            command.get_program(),
            String::from_utf8_lossy(&ran.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&ran.stdout).trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_log_line_loses_its_colours_and_what_isnt_ascii() {
        assert_eq!(
            plain("\u{1b}[31mred\u{1b}[0m and \u{2014} dash"),
            "red and ? dash"
        );
    }

    #[test]
    fn a_revert_says_its_test_commit_and_pr_in_four_sections() {
        let bad = Bad {
            commit: "0123456789abcdef".to_owned(),
            subject: "Merge pull request #7 from someone/branch".to_owned(),
            pr: Some(7),
            test: "vm-a".to_owned(),
            log: String::new(),
        };
        let body = revert_body(&bad, "line one\nline two");
        let sections: Vec<&str> = body
            .lines()
            .filter(|line| line.starts_with("### "))
            .collect();
        assert_eq!(
            sections,
            [
                "### 💡 TL;DR",
                "### 🧩 Problem",
                "### 🔧 Solution",
                "### 🔬 Validation"
            ]
        );
        for named in ["#7", "vm-a", "0123456789abcdef", "line two"] {
            assert!(body.contains(named), "{named}");
        }
        assert!(!body.contains('\u{2014}'));
    }

    #[test]
    fn a_log_never_names_the_machine_it_ran_from() {
        assert_eq!(
            unnamed("built on Here, then here", "here"),
            "built on this machine, then this machine"
        );
        assert_eq!(unnamed("nothing", ""), "nothing");
    }

    #[test]
    fn a_quarantine_list_names_its_tests() {
        let list = json!([{ "test": "vm-a", "reason": "r" }, { "test": "vm-b" }]);
        assert_eq!(tests_in(&list), ["vm-a", "vm-b"]);
        assert!(tests_in(&json!({})).is_empty());
    }

    #[test]
    fn a_log_gives_its_last_lines_that_say_something() {
        let log = std::env::temp_dir().join(format!("merge-net-tail-{}.log", std::process::id()));
        fs::write(&log, "one\n\ntwo\nthree\n\n").unwrap();
        assert_eq!(tail(&log, 2), "two\nthree");
        fs::remove_file(&log).unwrap();
    }
}
