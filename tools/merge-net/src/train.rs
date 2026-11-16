//! Changes that passed, and that main then moved beside, ride one train to
//! main together rather than each running again alone. Since midnight on
//! 16 November, 167 merges landed, `ready` answered 115 times that a change
//! must run again, and 66 runs again went, a mean 17 minutes each: with 20
//! merges an hour, main moved beside a change again before its run ended,
//! and #4983 missed its time five times so.
//!
//! `ready`, where it would start a change's run again, boards it instead:
//! a file under the net's state folder, `train/riders/<head>.json`, says
//! its head, its branch and when it boarded, and `ready` starts the train
//! as a user unit of its own, `merge-net ride`, unless it already goes.
//! The train cuts a candidate at origin's main in a checkout of its own,
//! merges every rider still its pull request's head into it in the order
//! they boarded, as GitHub's merge button words a merge, and runs the gate
//! once there, `nix-offload there --lent-only --changed`, which builds what
//! any of them moved. When it passes, the candidate goes to main as a fast
//! forward, never forced, and GitHub marks each pull request merged. When
//! main moved meanwhile, the candidate is cut again on it, and goes when
//! its pass still speaks for it as one for a rebased change would
//! (`earlier.rs`); otherwise it runs again with whoever has boarded since.
//!
//! A rider that conflicts, or that rode a candidate that failed, falls:
//! `train/fell/<head>` says why, and `ready` runs that head again alone, as
//! before the train. A rider whose pull request moved on, or closed, steps
//! off. What landed is kept a line each in `train/landed.jsonl`, so a day's
//! rides can be counted against the runs again they saved.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde_json::{Value, json};

use crate::state::short;

/// A change waiting on the train.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rider {
    /// The head that passed, as `ready` was asked about it.
    pub head: String,
    /// The branch its pull request merges from.
    pub branch: String,
    /// When it boarded, in seconds since the epoch.
    pub at: u64,
}

/// What `ready` does with a change main moved beside.
#[derive(Debug, PartialEq, Eq)]
pub enum Seat {
    /// It boards, and the train starts unless it goes.
    Board,
    /// It already rides.
    Rides,
    /// It fell off at this head, for the reason given: it runs again alone.
    Fell(String),
}

/// The train's folder in the net's state folder `root`.
#[must_use]
pub fn folder(root: &Path) -> PathBuf {
    root.join("train")
}

fn riders_folder(root: &Path) -> PathBuf {
    folder(root).join("riders")
}

fn fell_folder(root: &Path) -> PathBuf {
    folder(root).join("fell")
}

/// Put `rider` on the train.
///
/// # Errors
///
/// When its file can't be written.
pub fn board(root: &Path, rider: &Rider) -> io::Result<()> {
    let riders = riders_folder(root);
    fs::create_dir_all(&riders)?;
    let said = json!({"head": rider.head, "branch": rider.branch, "at": rider.at});
    fs::write(
        riders.join(format!("{}.json", rider.head)),
        format!("{said}\n"),
    )
}

/// Every rider, in the order they boarded.
#[must_use]
pub fn riders(root: &Path) -> Vec<Rider> {
    let mut riders: Vec<Rider> = fs::read_dir(riders_folder(root))
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let said: Value = serde_json::from_str(&fs::read_to_string(entry.path()).ok()?).ok()?;
            Some(Rider {
                head: said["head"].as_str()?.to_owned(),
                branch: said["branch"].as_str()?.to_owned(),
                at: said["at"].as_u64()?,
            })
        })
        .collect();
    riders.sort_by(|one, other| (one.at, &one.head).cmp(&(other.at, &other.head)));
    riders
}

/// Why the rider at `head` fell off, if it did.
#[must_use]
pub fn fell(root: &Path, head: &str) -> Option<String> {
    fs::read_to_string(fell_folder(root).join(head))
        .ok()
        .map(|why| why.trim().to_owned())
}

/// Take the rider at `head` off the train.
fn alight(root: &Path, head: &str) {
    let _ = fs::remove_file(riders_folder(root).join(format!("{head}.json")));
}

/// Take the rider at `head` off the train for `why`, so `ready` runs it
/// again alone.
///
/// # Errors
///
/// When its reason can't be written.
pub fn fall(root: &Path, head: &str, why: &str) -> io::Result<()> {
    let fell = fell_folder(root);
    fs::create_dir_all(&fell)?;
    fs::write(fell.join(head), format!("{why}\n"))?;
    alight(root, head);
    Ok(())
}

/// What `ready` does with the change at `head`, by whether it rides and
/// why it fell, if it did.
#[must_use]
pub fn seat(rides: bool, fell: Option<String>) -> Seat {
    match (fell, rides) {
        (Some(why), _) => Seat::Fell(why),
        (None, true) => Seat::Rides,
        (None, false) => Seat::Board,
    }
}

/// The subject GitHub's merge button gives the merge of pull request
/// `number` from `owner`'s `branch`, which the net's searches read a pull
/// request's number from.
#[must_use]
pub fn subject(number: u64, owner: &str, branch: &str) -> String {
    format!("Merge pull request #{number} from {owner}/{branch}")
}

/// A rider as the candidate merges it: its head, the merge's subject and
/// its pull request's title.
pub struct Merging {
    pub head: String,
    pub subject: String,
    pub title: String,
}

/// What cutting a candidate did: the heads it merged and those that
/// conflicted, which it left out.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Cut {
    pub merged: Vec<String>,
    pub conflicted: Vec<String>,
}

/// Cut a candidate at `base` by merging each of `riders` in turn, through
/// `git` in the train's checkout, leaving out one that conflicts.
///
/// # Errors
///
/// When the checkout can't be put at `base`.
pub fn cut(
    git: &dyn Fn(&[&str]) -> Result<String, String>,
    base: &str,
    riders: &[Merging],
) -> Result<Cut, String> {
    git(&["checkout", "--quiet", "--force", "-B", "train", base])?;
    let mut done = Cut::default();
    for rider in riders {
        let merged = git(&[
            "merge",
            "--quiet",
            "--no-ff",
            "--no-edit",
            "-m",
            &rider.subject,
            "-m",
            &rider.title,
            &rider.head,
        ]);
        if merged.is_ok() {
            done.merged.push(rider.head.clone());
        } else {
            git(&["merge", "--abort"])?;
            done.conflicted.push(rider.head.clone());
        }
    }
    Ok(done)
}

/// The unit the train goes as: one, so a second boarding finds it.
pub const UNIT: &str = "merge-net-train.service";

/// What the unit's description says.
const DESCRIBED: &str = "merge-net's train merges the changes that passed together";

/// Board the change at `head`, asked from `branch`, and start the train
/// unless it goes, saying what was done; nothing when the train can't
/// take it and it runs again alone as before.
#[must_use]
pub fn offer(head: &str, branch: &str) -> Option<String> {
    let root = crate::state::State::from_env().root;
    let boarded = riders(&root).iter().any(|waiting| waiting.head == head);
    match seat(boarded, fell(&root, head)) {
        Seat::Fell(_) => return None,
        Seat::Rides => {}
        Seat::Board => {
            let rider = Rider {
                head: head.to_owned(),
                branch: branch.to_owned(),
                at: crate::state::now(),
            };
            board(&root, &rider).ok()?;
        }
    }
    if !going() && !start(&root) {
        alight(&root, head);
        return None;
    }
    let others = riders(&root).len().saturating_sub(1);
    Some(format!(
        "merge-net put {} on the merge train with {others} others, which merge to main \
         together once one run of changed-checks over all of them passes, its words going \
         to {}; merge-net started the train, so push nothing to this branch until it lands, \
         and once it has, the pull request reads merged",
        short(head),
        folder(&root).display()
    ))
}

/// Why the change at `head` fell off the train, as `ready` says it beside
/// running it again alone.
#[must_use]
pub fn fell_said(head: &str) -> Option<String> {
    let root = crate::state::State::from_env().root;
    fell(&root, head).map(|why| format!("it fell off the merge train: {why}"))
}

/// Whether the train's unit goes.
fn going() -> bool {
    let shown = systemctl(&["show", UNIT, "--property=ActiveState"]);
    let state = shown
        .trim()
        .strip_prefix("ActiveState=")
        .unwrap_or_default();
    state == "active" || state == "activating"
}

/// Start the train's unit, beside the asker in its slice: whether systemd
/// took it.
fn start(root: &Path) -> bool {
    let Ok(merge_net) = std::env::current_exe() else {
        return false;
    };
    let train = folder(root);
    if fs::create_dir_all(&train).is_err() {
        return false;
    }
    let cgroup = fs::read_to_string("/proc/self/cgroup").unwrap_or_default();
    let slice = crate::rerun::slice_of(&cgroup).unwrap_or("app.slice");
    let path = std::env::var("PATH").unwrap_or_default();
    Command::new("systemd-run")
        .args([
            "--user",
            "--quiet",
            "--collect",
            "--no-block",
            &format!("--unit={UNIT}"),
            &format!("--description={DESCRIBED}"),
            &format!("--slice={slice}"),
            &format!("--working-directory={}", train.display()),
            &format!("--setenv=PATH={path}"),
            "--property=UnsetEnvironment=SSH_AUTH_SOCK",
            &format!(
                "--property=StandardOutput=append:{}",
                train.join("train.log").display()
            ),
            "--property=StandardError=inherit",
        ])
        .arg(&merge_net)
        .arg("ride")
        .status()
        .is_ok_and(|status| status.success())
}

/// What `systemctl --user` says, asked `arguments`, or nothing.
fn systemctl(arguments: &[&str]) -> String {
    Command::new("systemctl")
        .arg("--user")
        .args(arguments)
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
        .unwrap_or_default()
}

/// How many times a passed candidate is cut again on a main that moved
/// before it goes back to run again.
const RECUTS: usize = 5;

/// Whom the train takes next, after a candidate it ran.
#[derive(Debug, PartialEq, Eq)]
enum Split {
    /// Everyone aboard, as after a candidate that passed.
    All,
    /// Only this many riders, the first to board, as after a red one.
    Front(usize),
    /// The one rider that rode a red candidate alone falls; then everyone.
    Falls,
}

/// Whom the train takes after a candidate of `riding` riders that `passed`
/// or not.
fn split(riding: usize, passed: bool) -> Split {
    let _ = (riding, passed);
    Split::Falls
}

/// Ride the train until no one waits on it: `merge-net ride`, as its unit
/// runs it.
///
/// # Errors
///
/// When its checkout can't be made.
pub fn ride(url: &str) -> Result<ExitCode, String> {
    let root = crate::state::State::from_env().root;
    let checkout = folder(&root).join("checkout");
    if !checkout.join(".git").exists() {
        run_git(
            &folder(&root),
            &["clone", "--quiet", url, &checkout.display().to_string()],
        )?;
    }
    let slug = slug_of(url);
    let owner = slug.split('/').next().unwrap_or_default().to_owned();
    let git = |arguments: &[&str]| run_git(&checkout, arguments);
    loop {
        let waiting = riders(&root);
        if waiting.is_empty() {
            println!("merge-net: no one waits on the train");
            return Ok(ExitCode::SUCCESS);
        }
        let mergings = aboard(&root, &git, &slug, &owner, &waiting);
        if mergings.is_empty() {
            continue;
        }
        git(&["fetch", "--quiet", "origin", "main"])?;
        let base = git(&["rev-parse", "refs/remotes/origin/main"])?;
        let done = cut(&git, &base, &mergings)?;
        for head in &done.conflicted {
            let _ = fall(
                &root,
                head,
                &format!("it conflicts with main at {}", short(&base)),
            );
        }
        if done.merged.is_empty() {
            continue;
        }
        let candidate = git(&["rev-parse", "HEAD"])?;
        let log = folder(&root).join(format!("{candidate}.log"));
        let began = crate::state::now();
        println!(
            "merge-net: the train runs changed-checks at {} over {}",
            short(&candidate),
            done.merged
                .iter()
                .map(|head| short(head))
                .collect::<Vec<_>>()
                .join(", ")
        );
        if !gate(&checkout, &log) {
            for head in &done.merged {
                let why = format!(
                    "the candidate it rode, {}, failed changed-checks; its words are in {}",
                    short(&candidate),
                    log.display()
                );
                let _ = fall(&root, head, &why);
            }
            continue;
        }
        match land(&checkout, &git, &candidate, &mergings, &done.merged) {
            Landing::Landed(landed) => {
                for head in &done.merged {
                    alight(&root, head);
                }
                keep_landed(&root, &landed, &done.merged, crate::state::now() - began);
                println!("merge-net: the train landed {} on main", short(&landed));
            }
            Landing::Moved => {}
            Landing::Refused(why) => {
                for head in &done.merged {
                    let _ = fall(
                        &root,
                        head,
                        &format!("the train couldn't reach main: {why}"),
                    );
                }
            }
        }
    }
}

/// The riders still their pull request's head, as the candidate merges
/// them, fetched into the checkout; one whose pull request moved on or
/// closed steps off.
fn aboard(
    root: &Path,
    git: &dyn Fn(&[&str]) -> Result<String, String>,
    slug: &str,
    owner: &str,
    waiting: &[Rider],
) -> Vec<Merging> {
    let mut mergings = Vec::new();
    for rider in waiting {
        let open = pull_of(slug, &rider.branch);
        let still = open
            .as_ref()
            .is_some_and(|(_, _, head)| *head == rider.head);
        let fetched = still && git(&["fetch", "--quiet", "origin", &rider.head]).is_ok();
        match open {
            Some((number, title, _)) if fetched => mergings.push(Merging {
                head: rider.head.clone(),
                subject: subject(number, owner, &rider.branch),
                title,
            }),
            _ => {
                println!(
                    "merge-net: {} steps off the train: its pull request moved on or closed",
                    short(&rider.head)
                );
                alight(root, &rider.head);
            }
        }
    }
    mergings
}

/// The open pull request from `branch`: its number, title and head.
fn pull_of(slug: &str, branch: &str) -> Option<(u64, String, String)> {
    let out = Command::new("gh")
        .args([
            "pr",
            "list",
            "--repo",
            slug,
            "--head",
            branch,
            "--state",
            "open",
            "--json",
            "number,title,headRefOid",
        ])
        .output()
        .ok()?;
    let pulls: Value = serde_json::from_slice(&out.stdout).ok()?;
    let pull = pulls.as_array()?.first()?;
    Some((
        pull["number"].as_u64()?,
        pull["title"].as_str()?.to_owned(),
        pull["headRefOid"].as_str()?.to_owned(),
    ))
}

/// Run the gate in `checkout`, its words going to `log`: whether it passed.
fn gate(checkout: &Path, log: &Path) -> bool {
    let Ok(words) = fs::File::create(log) else {
        return false;
    };
    let Ok(errors) = words.try_clone() else {
        return false;
    };
    Command::new("nix-offload")
        .args(["there", "--lent-only", "--changed"])
        .current_dir(checkout)
        .env_remove("SSH_AUTH_SOCK")
        .stdout(words)
        .stderr(errors)
        .status()
        .is_ok_and(|status| status.success())
}

/// How a passed candidate's push to main went.
#[derive(Debug, PartialEq, Eq)]
enum Landing {
    /// It landed as this commit.
    Landed(String),
    /// Main moved beside it, so its riders ride the next run.
    Moved,
    /// Main refused it for this reason, though main stood still.
    Refused(String),
}

/// Push the passed `candidate` to main as a fast forward, never forced,
/// cutting it again on a main that moved while its pass still speaks for
/// it, as many as `RECUTS` times.
fn land(
    checkout: &Path,
    git: &dyn Fn(&[&str]) -> Result<String, String>,
    candidate: &str,
    mergings: &[Merging],
    merged: &[String],
) -> Landing {
    let mut going = candidate.to_owned();
    for _ in 0..RECUTS {
        let before = git(&["rev-parse", "refs/remotes/origin/main"]).unwrap_or_default();
        let pushed = git(&[
            "push",
            "--quiet",
            "origin",
            &format!("{going}:refs/heads/main"),
        ]);
        let Err(why) = pushed else {
            return Landing::Landed(going);
        };
        if git(&["fetch", "--quiet", "origin", "main"]).is_err() {
            return Landing::Refused(why);
        }
        let Ok(main) = git(&["rev-parse", "refs/remotes/origin/main"]) else {
            return Landing::Refused(why);
        };
        if main == before {
            return Landing::Refused(why);
        }
        let riding: Vec<Merging> = mergings
            .iter()
            .filter(|merging| merged.contains(&merging.head))
            .map(|merging| Merging {
                head: merging.head.clone(),
                subject: merging.subject.clone(),
                title: merging.title.clone(),
            })
            .collect();
        let whole = cut(git, &main, &riding).is_ok_and(|again| again.merged.len() == merged.len());
        let Ok(head) = git(&["rev-parse", "HEAD"]) else {
            return Landing::Moved;
        };
        going = head;
        if !whole || !still_passes(checkout, &going, &main) {
            println!(
                "merge-net: main moved beside the train at {}, so it runs again",
                short(&main)
            );
            return Landing::Moved;
        }
    }
    Landing::Moved
}

/// Whether the pass of an earlier candidate of the same changes speaks for
/// `candidate` on `main`, as `ready` takes one for a rebased change.
fn still_passes(checkout: &Path, candidate: &str, main: &str) -> bool {
    let Ok(here) = std::env::current_dir() else {
        return false;
    };
    if std::env::set_current_dir(checkout).is_err() {
        return false;
    }
    let on_main = |base: &str| {
        Command::new("git")
            .args(["merge-base", "--is-ancestor", base, main])
            .status()
            .is_ok_and(|status| status.success())
    };
    let judged = crate::earlier::judged(candidate, main, &on_main);
    let _ = std::env::set_current_dir(here);
    judged.is_ok_and(|judged| {
        matches!(
            judged.before,
            crate::ready::Before::Clean(_) | crate::ready::Before::Twice(..)
        )
    })
}

/// Keep what landed at `landed`, the heads it carried and how long since
/// its run began, a line in `train/landed.jsonl`.
fn keep_landed(root: &Path, landed: &str, heads: &[String], seconds: u64) {
    let line = json!({
        "at": crate::state::now(),
        "landed": landed,
        "heads": heads,
        "seconds": seconds,
    });
    let kept = OpenOptions::new()
        .create(true)
        .append(true)
        .open(folder(root).join("landed.jsonl"))
        .and_then(|mut file| file.write_all(format!("{line}\n").as_bytes()));
    if let Err(error) = kept {
        eprintln!("merge-net: the train's landing at {landed} wasn't kept: {error}");
    }
}

/// `owner/name` of the repository at `url`.
fn slug_of(url: &str) -> String {
    url.trim_end_matches(".git")
        .rsplit('/')
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("/")
}

/// What git says in `folder`, or why it failed.
fn run_git(folder: &Path, arguments: &[&str]) -> Result<String, String> {
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
            "git {}: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&ran.stderr).trim()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A state folder of the test's own.
    fn root(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("merge-net-train-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        root
    }

    fn rider(head: &str, at: u64) -> Rider {
        Rider {
            head: head.to_owned(),
            branch: format!("jitsusama/{head}"),
            at,
        }
    }

    #[test]
    fn riders_are_read_back_in_the_order_they_boarded() {
        let root = root("order");
        board(&root, &rider("b2", 20)).unwrap();
        board(&root, &rider("a1", 10)).unwrap();
        assert_eq!(
            riders(&root),
            vec![rider("a1", 10), rider("b2", 20)],
            "the riders weren't read back first boarded first"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_rider_that_fell_rides_no_more_and_runs_again_alone() {
        let root = root("fell");
        board(&root, &rider("a1", 10)).unwrap();
        fall(&root, "a1", "it conflicts with main").unwrap();
        assert!(riders(&root).is_empty(), "a rider that fell still rode");
        assert_eq!(
            seat(false, fell(&root, "a1")),
            Seat::Fell("it conflicts with main".to_owned()),
            "a head that fell boarded again"
        );
        assert_eq!(seat(true, None), Seat::Rides, "a rider boarded twice");
        assert_eq!(seat(false, None), Seat::Board, "a new head didn't board");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_red_train_splits_until_the_rider_that_breaks_it_falls_alone() {
        let breaks = 2;
        let mut aboard: Vec<usize> = (0..5).collect();
        let (mut landed, mut fell, mut runs) = (Vec::new(), Vec::new(), 0);
        let mut take = aboard.len();
        while !aboard.is_empty() && runs < 20 {
            let riding: Vec<usize> = aboard.iter().copied().take(take).collect();
            runs += 1;
            match split(riding.len(), !riding.contains(&breaks)) {
                Split::All => landed.extend(riding.iter().copied()),
                Split::Falls => fell.extend(riding.iter().copied()),
                Split::Front(front) => {
                    take = front;
                    continue;
                }
            }
            aboard.retain(|rider| !riding.contains(rider));
            take = aboard.len();
        }
        assert_eq!(fell, vec![breaks], "not only the rider that broke the train fell");
        assert_eq!(landed, vec![0, 1, 3, 4], "the riders that broke nothing didn't land");
        assert!(runs <= 6, "the split took {runs} runs to find one rider in five");
    }

    #[test]
    fn a_merge_is_worded_as_githubs_merge_button_words_it() {
        assert_eq!(
            subject(5066, "Jitsusama", "jitsusama/quarantine-stands-apart"),
            "Merge pull request #5066 from Jitsusama/jitsusama/quarantine-stands-apart",
            "the net's searches read a merge's pull request from these words"
        );
    }

    /// Git in `repo` with no user's or system's config, as the other
    /// modules' tests run it, giving what it printed or why it failed.
    fn git_in(repo: &Path, arguments: &[&str]) -> Result<String, String> {
        let ran = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(arguments)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .output()
            .map_err(|error| error.to_string())?;
        if ran.status.success() {
            Ok(String::from_utf8_lossy(&ran.stdout).trim().to_owned())
        } else {
            Err(String::from_utf8_lossy(&ran.stderr).into_owned())
        }
    }

    /// A repository at `repo` holding one commit on main, which it gives.
    fn based(repo: &Path) -> String {
        fs::create_dir_all(repo).unwrap();
        git_in(repo, &["init", "--quiet", "-b", "main"]).unwrap();
        fs::write(repo.join("shared"), "main\n").unwrap();
        git_in(repo, &["add", "shared"]).unwrap();
        git_in(repo, &["commit", "--quiet", "-m", "base"]).unwrap();
        git_in(repo, &["rev-parse", "HEAD"]).unwrap()
    }

    /// Commit `words` as `file` in `repo` on `from`, giving the commit.
    fn change(repo: &Path, from: &str, file: &str, words: &str) -> String {
        git_in(repo, &["checkout", "--quiet", "--detach", from]).unwrap();
        fs::write(repo.join(file), words).unwrap();
        git_in(repo, &["add", file]).unwrap();
        git_in(repo, &["commit", "--quiet", "-m", file]).unwrap();
        git_in(repo, &["rev-parse", "HEAD"]).unwrap()
    }

    fn merging(head: &str, number: u64) -> Merging {
        Merging {
            head: head.to_owned(),
            subject: subject(number, "o", head),
            title: format!("change {number}"),
        }
    }

    #[test]
    fn a_passed_candidate_lands_on_main_only_as_a_fast_forward() {
        let place = root("land");
        let (origin, checkout) = (place.join("origin.git"), place.join("checkout"));
        let base = based(&checkout);
        let url = origin.display().to_string();
        git_in(&place, &["init", "--quiet", "--bare", "-b", "main", &url]).unwrap();
        let git = |arguments: &[&str]| git_in(&checkout, arguments);
        git(&["remote", "add", "origin", &url]).unwrap();
        git(&["push", "--quiet", "origin", "main"]).unwrap();
        git(&["fetch", "--quiet", "origin", "main"]).unwrap();
        let head = change(&checkout, &base, "apart", "one\n");
        let riding = [merging(&head, 1)];
        let done = cut(&git, &base, &riding).unwrap();
        let candidate = git(&["rev-parse", "HEAD"]).unwrap();
        assert_eq!(
            land(&checkout, &git, &candidate, &riding, &done.merged),
            Landing::Landed(candidate.clone()),
            "a passed candidate on a main that stood still didn't land"
        );
        assert_eq!(
            git_in(&origin, &["rev-parse", "main"]).unwrap(),
            candidate,
            "origin's main isn't the candidate that landed"
        );
        git(&["remote", "set-url", "origin", "/nowhere"]).unwrap();
        let next = change(&checkout, &candidate, "next", "two\n");
        let more = [merging(&next, 2)];
        let again = cut(&git, &candidate, &more).unwrap();
        let refused = git(&["rev-parse", "HEAD"]).unwrap();
        assert!(
            matches!(
                land(&checkout, &git, &refused, &more, &again.merged),
                Landing::Refused(_)
            ),
            "a push refused while main stood still didn't fall back"
        );
        fs::remove_dir_all(&place).unwrap();
    }

    #[test]
    fn a_candidate_merges_each_rider_and_leaves_out_one_that_conflicts() {
        let repo = root("cut");
        let base = based(&repo);
        let git = |arguments: &[&str]| git_in(&repo, arguments);
        let apart = change(&repo, &base, "apart", "one\n");
        let clashes = change(&repo, &base, "shared", "theirs\n");
        let main = change(&repo, &base, "shared", "mains\n");
        let done = cut(&git, &main, &[merging(&apart, 1), merging(&clashes, 2)]).unwrap();
        assert_eq!(
            done,
            Cut {
                merged: vec![apart.clone()],
                conflicted: vec![clashes],
            },
            "the candidate didn't merge the rider apart and leave out the one that clashed"
        );
        assert_eq!(
            git(&["log", "-1", "--format=%P"]).unwrap(),
            format!("{main} {apart}"),
            "the candidate's merge doesn't sit on main with the rider beside it"
        );
        assert!(
            git(&["status", "--porcelain"]).unwrap().is_empty(),
            "the conflicted merge was left half done"
        );
        fs::remove_dir_all(&repo).unwrap();
    }
}
