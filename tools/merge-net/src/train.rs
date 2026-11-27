//! Changes that passed, and that main then moved beside, ride one train to
//! main together rather than each running again alone. Since midnight on
//! 16 November, 167 merges landed, `ready` answered 115 times that a change
//! must run again, and 66 runs again went, a mean 17 minutes each: with 20
//! merges an hour, main moved beside a change again before its run ended,
//! and #4983 missed its time five times so.
//!
//! `ready`, where it would start a change's run again, or send a head
//! behind main to rebase over files main moved, boards it instead:
//! a file under the net's state folder, `train/riders/<head>.json`, says
//! its head, its branch and when it boarded, and `ready` starts the train
//! as a user unit of its own, `merge-net ride`, unless it already goes.
//! The train goes as the build the watch follows, which the watch names in
//! the state folder's `installed` as it starts, never as the build that
//! asked, which may be a tree's or a profile's from before the last
//! switch; and between candidates a ride a switch replaced goes on as the
//! new build, in the same unit.
//! The train cuts a candidate at origin's main in a checkout of its own,
//! merges every rider still its pull request's head into it in the order
//! they boarded, as GitHub's merge button words a merge, and runs the gate
//! once there, `nix-offload there --lent-only --changed`, which builds what
//! any of them moved. When it passes, GitHub merges each rider's pull
//! request in the same order through its API, at the head that passed,
//! and signs each merge with the key machines trust main by, which no
//! agent holds; the train pushes nothing to main. Main's tree after those
//! merges must be the candidate's, or the train reverts them on GitHub,
//! the newest first. When GitHub refuses one, those it merged before it
//! stay, as main then holds the candidate's tree just before that merge;
//! the refused one falls to run again alone, and the rest stay aboard,
//! unless GitHub refused the first rider twice in a row, when they fall
//! too (`landing.rs`). A draft never boards. When main moved before the merges, the candidate is
//! cut again on it, and goes when its pass still speaks for it as one for
//! a rebased change would (`earlier/`); otherwise it runs again with
//! whoever has boarded since, holding the right of way meanwhile, so
//! changes that would merge alone board it rather than move main again
//! (`right_of_way.rs`).
//!
//! A candidate that fails splits: the train takes the first half of its
//! riders to board, and halves again while they fail, so the riders that
//! broke nothing land together and the one that breaks it is found alone,
//! five riders in five runs at most (`Splitting`). A train red for more
//! than its riders gives up, and every rider runs again alone. A candidate
//! no builder or evaluator took says nothing of its riders, unless its
//! words name a failed check: the train cuts again a minute on, keeping
//! its riders and its search, and only once its candidates have gone
//! untaken for half an hour do the last one's riders fall.
//! A rider that conflicts, or that rode a failed candidate alone, falls:
//! `train/fell/<head>` says why, and `ready` runs that head again alone, as
//! before the train. A rider whose pull request moved on, or closed, steps
//! off. What landed, and each rider that left unlanded with when it
//! boarded, is kept a line each in `train/landed.jsonl`, so a day's rides
//! can be counted against the runs again they saved.
//!
//! Long because one ride, from boarding through each run to landing, is
//! read here in order; its parts that stand alone are modules below.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Duration;

use serde_json::{Value, json};

use crate::state::short;
use right_of_way::Ended;
pub use right_of_way::gives_way;

mod gated;
mod hub;
mod installed;
mod landed;
mod landing;
mod pulls;
mod splitting;
use gated::{Gated, gated, words_of};
use hub::GitHub;
pub use installed::install;
use installed::{build, hand_over, handed, newer};
use landed::{Left, boarded_at, keep_landed, keep_left};
use landing::{Landing, land};
use pulls::aboard;
pub use pulls::drafted;
use splitting::{After, Splitting, UNTAKEN_FOR};

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

/// The rider at `head`, if it rides.
#[must_use]
pub fn rider_at(root: &Path, head: &str) -> Option<Rider> {
    riders(root).into_iter().find(|rider| rider.head == head)
}

/// Why the rider at `head` fell off, if it did.
#[must_use]
pub fn fell(root: &Path, head: &str) -> Option<String> {
    fs::read_to_string(fell_folder(root).join(head))
        .ok()
        .map(|why| why.trim().to_owned())
}

/// Take the rider at `head` off the train, saying whether it was on it.
fn alight(root: &Path, head: &str) -> bool {
    fs::remove_file(riders_folder(root).join(format!("{head}.json"))).is_ok()
}

/// Take the rider at `head` off the train without landing, as it left
/// `how`, keeping when it boarded. Kept only once it is off, so a rider
/// not aboard keeps nothing, and one whose file stays keeps no line until
/// it truly leaves, which would count it twice.
fn leave(root: &Path, head: &str, how: Left) {
    let boarded = boarded_at(root, &[head.to_owned()]);
    if alight(root, head) {
        keep_left(root, head, &boarded, how);
    }
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
    leave(root, head, Left::Fell);
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

/// A rider as the candidate merges it: its pull request's number, its
/// head, the merge's subject and its pull request's title.
#[derive(Clone)]
pub struct Merging {
    pub number: u64,
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
    let boarded = rider_at(&root, head).is_some();
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
    if !goes_or_starts(&root) {
        // One that boarded only now waited on no train, so only a rider
        // already aboard keeps its wait.
        if boarded {
            leave(&root, head, Left::Off);
        } else {
            alight(&root, head);
        }
        return None;
    }
    let others = riders(&root).len().saturating_sub(1);
    Some(format!(
        "merge-net put {} on the merge train with {others} others, which merge to main \
         together once one run of changed-checks over all of them passes, its words going \
         to {}; merge-net started the train, so push nothing to this branch until it lands; \
         a pull request the train merged and then reverted reads merged too, so its fell \
         folder says so, by head, when one does",
        short(head),
        folder(&root).display()
    ))
}

/// What `ready` says of the change at `head` while it rides, the train
/// started again if it stopped; nothing when it doesn't ride. Judged as if
/// it didn't, a rider behind a main that moved its files would be told to
/// rebase, and the push would take it off the train and to its back.
#[must_use]
pub fn riding(head: &str) -> Option<String> {
    let root = crate::state::State::from_env().root;
    rider_at(&root, head)?;
    if !goes_or_starts(&root) {
        leave(&root, head, Left::Off);
        return None;
    }
    Some(rides_said(&root))
}

/// Take the rider at `head` off the train, as one whose merge over main
/// passed merges without it.
pub fn step_off(head: &str) {
    let root = crate::state::State::from_env().root;
    leave(&root, head, Left::Off);
}

/// What `ready` says of a rider asked of again, among the riders under
/// `root`: that it waits, not that it was just put on the train.
fn rides_said(root: &Path) -> String {
    format!(
        "it waits on the train, {} riders in all, for one run of changed-checks over them to \
         pass, its words going to {}; push nothing to this branch until it lands; a pull \
         request the train merged and then reverted reads merged too, so its fell folder \
         says so, by head, when one does",
        riders(root).len(),
        folder(root).display()
    )
}

/// Why the change at `head` fell off the train, as `ready` says it beside
/// running it again alone.
#[must_use]
pub fn fell_said(head: &str) -> Option<String> {
    let root = crate::state::State::from_env().root;
    fell(&root, head).map(|why| format!("it fell off the merge train: {why}"))
}

/// Do to the right of way of the train under `root` what a run that
/// `ended` so does, saying so in the train's log.
fn way(root: &Path, ended: &Ended) {
    if let Some(said) = right_of_way::follow(root, ended, crate::state::now()) {
        println!("merge-net: {said}");
    }
}

/// Whether the train under `root` goes, started if it didn't. Two readies
/// at once may both find it stopped, and the one whose start loses the
/// race to the other's finds it going after all, rather than being sent
/// off the train.
fn goes_or_starts(root: &Path) -> bool {
    going() || start(root) || going()
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
    let Some(merge_net) = build(root, std::env::current_exe()) else {
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

/// How long the train waits after a candidate no builder took before it
/// cuts the next.
const UNTAKEN_PAUSE: Duration = Duration::from_mins(1);

/// Ride the train until no one waits on it: `merge-net ride`, as its unit
/// runs it.
///
/// # Errors
///
/// When its checkout can't be made.
pub fn ride(url: &str) -> Result<ExitCode, String> {
    let root = crate::state::State::from_env().root;
    begin(&root, handed().is_some());
    let ridden = ride_from(&root, url);
    right_of_way::run_ends(&root);
    way(&root, &Ended::Otherwise);
    ridden
}

/// Begin a ride under `root`. A train that was killed left its right of
/// way behind, so one starting afresh holds none, and however it stops,
/// changes merge alone again; one a ride was `handed_over` from holds
/// what that ride held. A run a killed train marked isn't running either.
fn begin(root: &Path, handed_over: bool) {
    right_of_way::run_ends(root);
    if !handed_over {
        way(root, &Ended::Otherwise);
    }
}

/// Ride the train whose state is under `root` from the repository at
/// `url`, until no one waits on it.
fn ride_from(root: &Path, url: &str) -> Result<ExitCode, String> {
    let root = root.to_path_buf();
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
    let mut splitting = Splitting::default();
    loop {
        let waiting = riders(&root);
        if waiting.is_empty() {
            println!("merge-net: no one waits on the train");
            return Ok(ExitCode::SUCCESS);
        }
        // A ride started before a switch went on as the build from before
        // it: on 18 November one pushed #5426, #5460 and #5321 to main
        // itself, under Joel's key, after the switch that made the train
        // merge through GitHub. Between candidates it goes on as the build
        // installed instead; when that build won't run, it goes on as it
        // is rather than leave its riders waiting on an asker.
        if let Some(newer) = newer(&root, std::env::current_exe(), handed().as_deref()) {
            println!(
                "merge-net: another merge-net is installed, so the train goes on as {}",
                newer.display()
            );
            println!("merge-net: {}, so it goes on as it is", hand_over(&newer));
        }
        let mut mergings = aboard(&root, &git, &slug, &owner, &waiting);
        mergings.truncate(splitting.take());
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
        // Only a clear run takes changes that would merge alone, so none
        // boards a red's search or an outage only to fall with its riders.
        // One that boards rides the next candidate, cut once this one ends.
        if splitting.clear() {
            right_of_way::run_begins(&root, began);
            println!(
                "merge-net: the train's run is clear, so a change that would merge alone \
                 boards it until the run ends"
            );
        }
        println!(
            "merge-net: the train runs changed-checks at {} over {}",
            short(&candidate),
            done.merged
                .iter()
                .map(|head| short(head))
                .collect::<Vec<_>>()
                .join(", ")
        );
        match gate(&checkout, &log) {
            Gated::Passed => splitting.taken(),
            Gated::Red => {
                right_of_way::run_ends(&root);
                splitting.taken();
                way(&root, &Ended::Otherwise);
                red(&root, &candidate, &log, &done.merged, &mut splitting);
                continue;
            }
            Gated::Unsent => {
                right_of_way::run_ends(&root);
                // The train stays in its ride rather than waiting for a
                // ready, since a rider is told it needn't ask again.
                let now = crate::state::now();
                if unsent(&root, &candidate, &log, &done.merged, &mut splitting, now) {
                    std::thread::sleep(UNTAKEN_PAUSE);
                }
                continue;
            }
        }
        // Said before the first merge, so an outside merge can wait it out:
        // one between the train's merges left main untested on 25 November,
        // reverting all three of a run's. Each line until the one saying it
        // ends is the landing's, and a train stopped between leaves it
        // unended, so an outside merge waits on.
        println!("merge-net: the train lands {} on main", short(&candidate));
        let landing = land(
            &checkout,
            &git,
            &GitHub { slug: slug.clone() },
            &base,
            &candidate,
            &mergings,
            &done.merged,
        );
        // Held through the landing, whose recuts a change merged alone
        // would send back to run again.
        right_of_way::run_ends(&root);
        after_landing(&root, landing, &done.merged, began, &mut splitting);
        println!("merge-net: the landing of {} ends", short(&candidate));
    }
}

/// After `landing` the `merged` riders of a run begun at `began`: those
/// that landed alight and are kept, the rest fall saying why, and the
/// right of way and `splitting` go on from there.
fn after_landing(
    root: &Path,
    landing: Landing,
    merged: &[String],
    began: u64,
    splitting: &mut Splitting,
) {
    let falls = |why: &str| {
        for head in merged {
            let _ = fall(root, head, why);
        }
    };
    match landing {
        Landing::Landed(landed) => {
            splitting.landed(merged.len());
            // Read before alighting, which takes each rider's file.
            let boarded = boarded_at(root, merged);
            for head in merged {
                alight(root, head);
            }
            keep_landed(root, &landed, merged, &boarded, crate::state::now() - began);
            println!("merge-net: the train landed {} on main", short(&landed));
            let left = !riders(root).is_empty();
            way(root, &Ended::Landed { left });
            return;
        }
        Landing::Partly {
            landed,
            kept,
            refused,
            why,
        } => {
            // The riders landed alight, the refused one falls to run again
            // alone, and those after it stay aboard for the next run.
            // The refused rider leaves the front with those landed, so a
            // search for a red candidate looks past it too.
            splitting.landed(kept.len() + 1);
            let boarded = boarded_at(root, &kept);
            for head in &kept {
                alight(root, head);
            }
            keep_landed(root, &landed, &kept, &boarded, crate::state::now() - began);
            let (before, stay) = match kept.len() {
                1 => ("the rider before it".to_owned(), "stays"),
                riders => (format!("the {riders} riders before it"), "stay"),
            };
            let said = format!(
                "{why}; {before} landed as {} and {stay}, and it leaves the train",
                short(&landed)
            );
            println!("merge-net: {said}");
            let _ = fall(root, &refused, &said);
            let left = !riders(root).is_empty();
            way(root, &Ended::Landed { left });
            return;
        }
        Landing::Turned { refused, why } => {
            if splitting.turned() {
                falls(&format!(
                    "{why}, as it refused the train's first rider before, so every rider \
                     leaves the train"
                ));
                way(root, &Ended::Otherwise);
                *splitting = Splitting::default();
                return;
            }
            // Nothing merged, so only the refused rider falls, and those
            // after it stay aboard for the next run with the train's way
            // and search kept, as after a landing that leaves riders.
            let said = format!("{why}; the train merged nothing, and it leaves the train");
            println!("merge-net: {said}");
            let _ = fall(root, &refused, &said);
            let left = !riders(root).is_empty();
            way(root, &Ended::Landed { left });
            return;
        }
        Landing::Moved => {
            way(root, &Ended::Moved);
            return;
        }
        Landing::Reverted {
            reverted,
            kept,
            why,
        } => {
            for head in merged {
                let said = if let Some((_, revert)) = reverted.iter().find(|(at, _)| at == head) {
                    format!(
                        "the train reverted its merge, as {why}; its pull request reads merged, \
                         so it lands again only through a revert of #{revert}, GitHub's revert"
                    )
                } else if kept.contains(head) {
                    format!(
                        "its merge may stand on main though no gate ran that tree, as {why}; \
                         its pull request says whether it merged"
                    )
                } else {
                    format!("the train stopped before merging it, as {why}")
                };
                let _ = fall(root, head, &said);
            }
        }
        Landing::Refused(why) => falls(&format!("the train couldn't reach main: {why}")),
    }
    way(root, &Ended::Otherwise);
    *splitting = Splitting::default();
}

/// After no builder took the candidate `candidate` of `merged` riders,
/// its words in `log`, `now`: whether the train tries again, its riders
/// aboard, or the riders of that candidate fall back to running again
/// alone, as `splitting` says. Riders that boarded since rode none of the
/// untaken candidates, so they stay aboard and start afresh.
fn unsent(
    root: &Path,
    candidate: &str,
    log: &Path,
    merged: &[String],
    splitting: &mut Splitting,
    now: u64,
) -> bool {
    if splitting.untaken(now) {
        println!(
            "merge-net: no builder took the candidate {}, its words in {}, so the train tries \
             again with its riders aboard",
            short(candidate),
            log.display()
        );
        return true;
    }
    let why = format!(
        "no builder or evaluator took the train's candidates for {} minutes, the last {}; \
         its words are in {}",
        UNTAKEN_FOR / 60,
        short(candidate),
        log.display()
    );
    println!("merge-net: {why}, so the riders of that candidate fall");
    for head in merged {
        let _ = fall(root, head, &why);
    }
    false
}

/// After the candidate `candidate` of `merged` riders failed, its words in
/// `log`: the next takes fewer riders, or the rider that rode it alone
/// falls, or every rider does, as `splitting` says.
fn red(root: &Path, candidate: &str, log: &Path, merged: &[String], splitting: &mut Splitting) {
    let (falling, why) = match splitting.red(merged.len()) {
        After::Falls => (merged.to_vec(), "the candidate it rode alone"),
        After::AllFall => (
            riders(root).into_iter().map(|rider| rider.head).collect(),
            "the train gave up, as the second rider in a row to ride a candidate alone \
             failed too, so its red is not one rider's; the last",
        ),
        After::Narrows => {
            println!(
                "merge-net: the candidate {} failed changed-checks, so the train takes its \
                 first {} of {} riders next",
                short(candidate),
                splitting.take(),
                merged.len()
            );
            return;
        }
    };
    println!(
        "merge-net: the candidate {} failed changed-checks, so {} of the train's riders fall \
         back to running again alone",
        short(candidate),
        falling.len()
    );
    for head in &falling {
        let said = format!(
            "{why}, {}, failed changed-checks; its words are in {}",
            short(candidate),
            log.display()
        );
        let _ = fall(root, head, &said);
    }
}

/// Run the gate in `checkout`, its words going to `log`: how it ended.
fn gate(checkout: &Path, log: &Path) -> Gated {
    let Ok(words) = fs::File::create(log) else {
        return Gated::Red;
    };
    let Ok(errors) = words.try_clone() else {
        return Gated::Red;
    };
    let ran = Command::new("nix-offload")
        .args(["there", "--lent-only", "--changed"])
        .current_dir(checkout)
        .env_remove("SSH_AUTH_SOCK")
        .stdout(words)
        .stderr(errors)
        .status();
    ran.map_or(Gated::Red, |status| gated(status.code(), &words_of(log)))
}

/// `owner/name` of the repository at `url`.
fn slug_of(url: &str) -> String {
    // An ssh origin names its host before a colon, git@github.com:owner/repo.
    let path = url.rsplit_once(':').map_or(url, |(_, path)| path);
    path.trim_end_matches('/')
        .trim_end_matches(".git")
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

mod right_of_way;

#[cfg(test)]
mod tests;
