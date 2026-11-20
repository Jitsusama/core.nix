//! The net's world as it is: its own checkout of the repository, git and
//! changed-checks there, `nix-offload there` for each check, which sends
//! it to a builder, and for each evaluation, and gh for the PRs it lands.
//! git, nix and gh are the person's own, with their settings, so a commit
//! is signed as theirs are, and they're found on the path rather than
//! pinned.

use std::collections::{BTreeSet, VecDeque};
use std::fmt::Write as _;
use std::fs;
use std::io::{BufRead as _, BufReader, Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::holds;
use crate::running;
use crate::state::{now, short};
use crate::subtests;
use crate::watch::{
    Bad, Changed, DROPPED, MOVED, Merged, UNCHECKED, UNCOMPARED, UNREACHED, UNSENT, World, untaken,
};

/// What the net's changed-checks runs ask, so a builder lets them ahead of
/// sessions' runs waiting for a turn: on 8 November the net's comparison
/// waited an hour behind sessions' gates and tested nothing meanwhile. Its
/// VM tests stay bulk, behind sessions', as `there_args` says.
const FIRST: &str = "--first";

/// Where the quarantine list is in the repository.
pub const QUARANTINE: &str = "tools/merge-net/quarantine.json";

/// The longest a VM test may run on a builder before it's stopped, a stall
/// rather than a failure: the slowest pass in well under an hour on a
/// builder of its own.
const LONGEST: Duration = Duration::from_hours(2);

/// How long a VM test's log may stay silent on a builder before it's
/// stopped as a stall. A run built with `-L` prints as it goes, and a build
/// that hung on the builder once sat silent for over an hour.
const SILENT: Duration = Duration::from_mins(30);

/// What nix-offload says once a builder has taken a run. Before it, the run
/// waits its turn in the builder's queue, which says nothing for as long as
/// the queue takes, so neither clock may start until it's said.
const TAKEN: &str = "nix-offload: building on ";

/// When a run sent to a builder has been there too long, or too quiet.
struct Clock {
    taken: Option<Instant>,
    /// Where this run's part of the log begins. A run the net's restart
    /// cut short records nothing, so the next one has its name and writes
    /// after it, and the builder that took the run cut short hasn't taken
    /// this one.
    from: usize,
    written: u64,
    heard: Instant,
}

impl Clock {
    /// A clock for a run about to write to `log`, after what it holds.
    fn new(log: &Path, now: Instant) -> Self {
        let written = fs::metadata(log).map_or(0, |meta| meta.len());
        Self {
            taken: None,
            from: usize::try_from(written).unwrap_or(usize::MAX),
            written,
            heard: now,
        }
    }

    /// This run's part of `log`, read from where it begins rather than
    /// from the runs before it; none when it can't be read.
    fn ours(&self, log: &Path) -> Option<Vec<u8>> {
        let mut ours = Vec::new();
        fs::File::open(log)
            .and_then(|mut file| {
                file.seek(SeekFrom::Start(
                    u64::try_from(self.from).unwrap_or(u64::MAX),
                ))?;
                file.read_to_end(&mut ours)
            })
            .ok()?;
        Some(ours)
    }

    /// Whether this run's part of `log` says a builder took it.
    fn said_taken(&self, log: &Path) -> bool {
        self.ours(log).is_some_and(|ours| taken_in(&ours))
    }

    /// Whether the run whose log is `log` should be stopped at `now`.
    fn overdue(&mut self, log: &Path, now: Instant) -> bool {
        let written = fs::metadata(log).map_or(0, |meta| meta.len());
        if written != self.written {
            self.written = written;
            self.heard = now;
            if self.taken.is_none() && self.said_taken(log) {
                self.taken = Some(now);
            }
        }
        self.taken.is_some_and(|taken| {
            now.duration_since(taken) > LONGEST || now.duration_since(self.heard) > SILENT
        })
    }
}

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
    /// Held by whatever moves the checkout's head, as a change landing
    /// does, so two can't take turns in it. At 06:01 on 11 November a
    /// comparison checked out main's head under a quarantine being cut.
    /// Its commit went on the detached head, the tree was left changed,
    /// and its checks refused to run. Comparisons now each take a
    /// checkout of their own ([`Comparing`]) and never hold this.
    pub moving: Mutex<()>,
    /// The runs nothing needs now, by commit and check, which are stopped.
    pub dropped: Mutex<BTreeSet<(String, String)>>,
    /// Where the merge-net installed is, which the watch follows: once
    /// that's another build, this one is superseded.
    pub follow: Option<PathBuf>,
}

/// Whether the merge-net installed is another build than the one running.
/// On 7 November three switches in 25 minutes each restarted the net with
/// a new build, and a round of about 20 minutes ran from 16:21 past 17:15.
/// A running build no longer on disk has been replaced; an installed one
/// that can't be read says nothing.
pub fn replaced(installed: std::io::Result<PathBuf>, running: std::io::Result<PathBuf>) -> bool {
    let Ok(installed) = installed else {
        return false;
    };
    !running
        .and_then(fs::canonicalize)
        .is_ok_and(|running| running == installed)
}

impl Real {
    /// The branch it follows, as the checkout tracks it.
    fn followed(&self) -> String {
        format!("refs/remotes/origin/{}", self.branch)
    }

    fn checkout(&self) -> PathBuf {
        self.state.join("core")
    }

    /// The checkout to itself, until what's given back is dropped.
    fn moving(&self) -> MutexGuard<'_, ()> {
        self.moving.lock().unwrap_or_else(PoisonError::into_inner)
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

    /// The quarantine list at `commit`, empty where it doesn't read.
    fn listed_at(&self, commit: &str) -> Value {
        self.git(&["show", &format!("{commit}:{QUARANTINE}")])
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .unwrap_or_else(|| json!([]))
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

    /// Cut `branch` from `base`, make `change` in the checkout, commit it
    /// as `message`, and give the commit.
    fn cut(
        &self,
        branch: &str,
        base: &str,
        message: &str,
        change: &dyn Fn() -> Result<(), String>,
    ) -> Result<String, String> {
        self.git(&["checkout", "--quiet", "--force", "-B", branch, base])?;
        change()?;
        let (subject, rest) = message.split_once("\n\n").unwrap_or((message, ""));
        let mut commit = vec!["commit", "--quiet", "-m", subject];
        if !rest.is_empty() {
            commit.extend(["-m", rest]);
        }
        self.git(&commit)?;
        self.git(&["rev-parse", "HEAD"])
    }

    /// Merge main's newest head into `branch`, at `sha` and cut from
    /// `cut_from`, and push it, giving main's newest head and the branch's
    /// commit. Main may move in the minutes a change's checks take, and a
    /// merge in them may have fixed what the change is for, or changed what
    /// it meets. A change to the quarantine list alone that meets another
    /// there is cut again from the newest head instead, since it's made
    /// from the list as it finds it: on 10 November the net's quarantine of
    /// vm-desktop-says-what-happens passed its checks at 22:01 and never
    /// landed, because #2996 had taken a test off the list meanwhile.
    fn onto_newest(
        &self,
        branch: &str,
        cut_from: &str,
        sha: String,
        message: &str,
        change: &dyn Fn() -> Result<(), String>,
    ) -> Result<(String, String), String> {
        self.git(&["fetch", "--quiet", "origin", &self.branch])?;
        let newest = self.git(&["rev-parse", &self.followed()])?;
        if newest == cut_from {
            return Ok((newest, sha));
        }
        let merging = format!("Merge {} into {branch}", short(&newest));
        if let Err(why) = self.git(&["merge", "--quiet", "--no-edit", "-m", &merging, &newest]) {
            let _ = self.git(&["merge", "--abort"]);
            let changed = self.git(&["diff", "--name-only", cut_from, &sha])?;
            if !changed.lines().all(|file| file == QUARANTINE) {
                return Err(why);
            }
            let again = self.cut(branch, &newest, message, change)?;
            self.git(&["push", "--quiet", "--force", "origin", branch])?;
            return Ok((newest, again));
        }
        self.git(&["push", "--quiet", "origin", branch])?;
        let merged = self.git(&["rev-parse", "HEAD"])?;
        Ok((newest, merged))
    }

    /// Land a change on main through a PR merged on its own checks: a
    /// branch from main, `change` made in the checkout and committed as
    /// `message`, pushed, changed-checks passing on it, then main's newest
    /// head merged into it and pushed, and a PR merged only at that commit,
    /// and only if `still`, asked with main's newest head and the change at
    /// it, says it's wanted. Main moves in the minutes the checks take, so
    /// `still` is asked of main as it lands on, not as it was cut from.
    fn land(
        &self,
        branch: &str,
        message: &str,
        title: &str,
        body: &str,
        still: &dyn Fn(&str, &str) -> Result<bool, String>,
        change: &dyn Fn() -> Result<(), String>,
    ) -> Result<Merged, String> {
        let moving = self.moving();
        let followed = self.followed();
        self.git(&["fetch", "--quiet", "origin", &self.branch])?;
        let cut_from = self.git(&["rev-parse", &followed])?;
        let sha = self.cut(branch, &cut_from, message, change)?;
        self.git(&["push", "--quiet", "--force", "-u", "origin", branch])?;
        let log = self
            .state
            .join("logs")
            .join(format!("land-{}.log", branch.replace('/', "-")));
        let checked = Command::new("changed-checks")
            .args(["--base", &cut_from, FIRST])
            .current_dir(self.checkout())
            .stdout(file(&log)?)
            .stderr(file(&log)?)
            .status()
            .map_err(|error| format!("running changed-checks: {error}"))?;
        if untaken(checked.code()) {
            return Err(format!("{UNCHECKED}, {}", log.display()));
        }
        if !checked.success() {
            let failed = unbuilt(lines_of(&log));
            let new: Vec<&String> = failed
                .iter()
                .filter(|check| !self.fails_at(&cut_from, check))
                .collect();
            if failed.is_empty() || !new.is_empty() {
                return Err(format!("its checks failed, {}", log.display()));
            }
            append_line(
                &log,
                &format!(
                    "merge-net: {} fail before this change too, so it lands",
                    failed.join(" ")
                ),
            );
        }
        let (newest, sha) = self.onto_newest(branch, &cut_from, sha, message, change)?;
        if !asked_free(moving, still, &newest, &sha)
            .map_err(|why| format!("{why}, {}", log.display()))?
        {
            return Err(format!("{DROPPED}, {}", log.display()));
        }
        self.pull(branch, title, body, &sha)
    }
}

/// What `still` says of `newest` and `sha`, asked once `moving` is given
/// back. Its runs take minutes and need nothing of the checkout, and what
/// it decides can land a change of its own, a flaky test's quarantine,
/// which takes the checkout in turn. On 12 November a revert's `still`
/// found vm-desktop-launcher-moves-windows flaky at main's newest head and
/// listed it while the revert held the checkout, so the net waited on
/// itself from 07:02 until it was restarted at 08:15.
fn asked_free(
    moving: MutexGuard<'_, ()>,
    still: &dyn Fn(&str, &str) -> Result<bool, String>,
    newest: &str,
    sha: &str,
) -> Result<bool, String> {
    drop(moving);
    still(newest, sha)
}

impl Real {
    /// Open a PR for `branch`, or take up the one already open for it, and
    /// merge it at `sha`. On 10 November GitHub refused the net's merge of
    /// a quarantine a moment after its push, saying the head had moved, and
    /// the PR it left open made every later land of that test fail at
    /// opening one.
    fn pull(&self, branch: &str, title: &str, body: &str, sha: &str) -> Result<Merged, String> {
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
        ]));
        let pr = match made {
            Ok(made) => {
                pr_named(&made).ok_or_else(|| format!("gh made a PR it didn't name: {made}"))?
            }
            Err(why) => {
                let pr = open_already(&why).ok_or(why)?;
                // What the open PR says is of an earlier try; this one's
                // body says what lands now, and failing to say it stops
                // nothing.
                let _ = run(Command::new("gh").args([
                    "pr",
                    "edit",
                    &pr.to_string(),
                    "--repo",
                    &slug,
                    "--title",
                    title,
                    "--body",
                    body,
                ]));
                pr
            }
        };
        let merge = || {
            run(Command::new("gh").args([
                "pr",
                "merge",
                &pr.to_string(),
                "--repo",
                &slug,
                "--merge",
                "--match-head-commit",
                sha,
            ]))
        };
        let mut merging = merge();
        for _ in 0..3 {
            match &merging {
                Err(why) if head_not_seen(why) => {
                    // GitHub takes a few seconds to see a push as the PR's
                    // head, and the merge names that head, so it's asked
                    // again after a pause rather than at once.
                    std::thread::sleep(Duration::from_secs(10));
                    merging = merge();
                }
                _ => break,
            }
        }
        merging.map_err(|why| refused(pr, &why))?;
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

impl Real {
    /// What `nix eval` says of `attribute`, asked `how`, on a lent builder.
    /// `nix-offload there --run` runs it from the net's checkout, the only
    /// folder it needs, and the builder fetches the flake itself.
    fn evaluated(&self, attribute: &str, how: &[&str]) -> Result<String, String> {
        run(Command::new("nix-offload")
            .args(eval_args(attribute, how))
            .current_dir(self.checkout()))
    }

    /// Whether `check` fails at `commit` as well, so a change isn't refused
    /// for a check that was failing before it: a revert waiting on main's
    /// other breaks would leave its own break in place.
    fn fails_at(&self, commit: &str, check: &str) -> bool {
        if !matches!(self.has_check(commit, check), Ok(true)) {
            return false;
        }
        let built = Command::new("nix")
            .args([
                "build",
                "--no-link",
                &format!(
                    "github:{}/{commit}#checks.{}.{check}",
                    self.slug(),
                    self.system
                ),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        built.is_ok_and(|status| !status.success())
    }
}

/// The PR a link gh printed names, as `.../pull/2941`.
fn pr_named(said: &str) -> Option<u64> {
    said.trim().rsplit('/').next()?.trim().parse().ok()
}

/// The PR gh says is already open for the branch, when that's why it
/// opened none.
fn open_already(why: &str) -> Option<u64> {
    if !why.contains("already exists") {
        return None;
    }
    why.split_whitespace().rev().find_map(pr_named)
}

/// How a land fails whose PR `pr` GitHub refused to merge for `why`:
/// as `MOVED` when main moved after its checks ran, which they must run
/// again on, and otherwise as the PR left open.
fn refused(pr: u64, why: &str) -> String {
    if why.contains("Base branch was modified") {
        return format!("{MOVED}: #{pr} was refused: {why}");
    }
    format!("#{pr} is open but didn't merge: {why}")
}

/// Whether GitHub refused a merge only because it hadn't yet seen the
/// pushed commit as the PR's head.
fn head_not_seen(why: &str) -> bool {
    why.contains("Head branch was modified")
}

/// Whether a build's log says its builder couldn't fetch the commit's own
/// source, as when GitHub answered 503 to every builder at 06:38 on
/// 8 November, and the net called a test it never ran flaky.
pub fn unfetched(log: impl IntoIterator<Item = impl AsRef<str>>, commit: &str) -> bool {
    log.into_iter().any(|line| {
        let line = line.as_ref();
        line.contains("while fetching the input") && line.contains(commit)
    })
}

/// A log's lines, read one at a time and ended as `str::lines` ends them,
/// what isn't UTF-8 in them replaced, and none for a log that can't be
/// opened. A VM test's log runs to tens
/// of megabytes. On 11 November the watcher, reading each whole, grew
/// from 591 to 981 MB in sixteen minutes, nearly all of it full 64 MB
/// allocator arenas, which each thread keeps at the most it ever held.
pub fn lines_of(log: &Path) -> impl Iterator<Item = String> {
    fs::File::open(log).into_iter().flat_map(|file| {
        BufReader::new(file)
            .split(b'\n')
            .map_while(Result::ok)
            .map(|mut line| {
                if line.last() == Some(&b'\r') {
                    line.pop();
                }
                String::from_utf8_lossy(&line).into_owned()
            })
    })
}

/// The checks a nix build's log says it couldn't build, by the names their
/// derivations go by.
pub fn unbuilt(log: impl IntoIterator<Item = impl AsRef<str>>) -> Vec<String> {
    let mut names: Vec<String> = log
        .into_iter()
        .filter_map(|line| {
            let rest = line
                .as_ref()
                .trim()
                .strip_prefix("error: Cannot build '/nix/store/")?
                .to_owned();
            let (path, _) = rest.split_once(".drv'")?;
            path.split_once('-').map(|(_, name)| name.to_owned())
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

/// What the net asks `nix-offload there` for one check: as bulk when the
/// binary's `--takes` lists it, so a session's VM tests waiting for a VM
/// builder go ahead of the net's dozens, and without it otherwise, since
/// an older binary refuses a word it doesn't know.
pub fn there_args(flake: &str, attribute: &str, takes: &str) -> Vec<String> {
    let mut args = vec!["there".to_owned()];
    if takes.lines().any(|word| word.trim() == "bulk") {
        args.push("--bulk".to_owned());
    }
    args.extend(["-L".to_owned(), flake.to_owned(), attribute.to_owned()]);
    args
}

/// The longest an evaluation runs on a builder: one takes about 16
/// seconds, and one past ten minutes is one that hangs.
const EVALUATES_AT_MOST: &str = "600";

/// What the net asks `nix-offload there` to evaluate `attribute` with,
/// `how` saying what `nix eval` prints: on a lent builder, never here.
/// Four at a time here held 3.4 GB of the person's machine on 11 November,
/// about 850 MB and most of a core each, for 82 derivations an hour while
/// a search ran; sent, each holds 13 MB here and takes 16 seconds against
/// 10.
pub fn eval_args(attribute: &str, how: &[&str]) -> Vec<String> {
    let sent = ["there", "--run", "--longest", EVALUATES_AT_MOST, "--"];
    let mut args: Vec<String> = sent
        .into_iter()
        .chain(["nix", "eval"])
        .map(str::to_owned)
        .collect();
    args.extend(how.iter().map(|&word| word.to_owned()));
    args.push(attribute.to_owned());
    args
}

/// Add a line to a log, as the net's own word among a command's.
fn append_line(log: &Path, line: &str) {
    if let Ok(mut file) = fs::OpenOptions::new().append(true).open(log) {
        let _ = writeln!(file, "{line}");
    }
}

impl Real {
    /// Build `check` at `commit` into `log`, or only the subtests `picked`
    /// names, as `run-subtests --there` builds them; none when it couldn't
    /// be started or ran past its time.
    fn built(
        &self,
        commit: &str,
        check: &str,
        picked: &[&str],
        log: &Path,
        dropped: &dyn Fn() -> bool,
    ) -> Option<i32> {
        let output = file(log).ok()?;
        let mut clock = Clock::new(log, Instant::now());
        let takes = run(Command::new("nix-offload").args(["there", "--takes"])).unwrap_or_default();
        let args = build_args(&self.flake(commit), &self.system, check, picked, &takes);
        let child = Command::new("nix-offload")
            .args(args)
            .stdin(Stdio::null())
            .stdout(output.try_clone().ok()?)
            .stderr(output)
            .spawn()
            .ok()?;
        ended(child, &mut clock, commit, log, dropped)
    }

    /// Compare a Mac's checks at `commit` against `base` into `log`, as a
    /// gate whose Mac was away would have, in a checkout of `commit`; none
    /// when a builder took it and it ran past its time or was stopped. One
    /// no Mac took ends `UNSENT`, as nix-offload ends 75 for a Mac part it
    /// can't send, and one that compared nothing ends `UNCOMPARED`: one
    /// that couldn't be started, or that `mac_ending` reads so.
    fn compared_on_a_mac(
        &self,
        commit: &str,
        base: &str,
        log: &Path,
        dropped: &dyn Fn() -> bool,
    ) -> Option<i32> {
        // One that couldn't be started compared nothing, and says nothing
        // of the merge, so it waits for the next round rather than being
        // read as a stall told to the merge's owner. Why is said in the
        // net's journal and in the run's log, which net.log points at,
        // since a cause that lasts, as a full disk, is the net's to mend
        // and not the merge's.
        let unstarted = |why: String| {
            let said = format!(
                "merge-net: a Mac's comparison at {} couldn't start: {why}",
                commit.get(..12).unwrap_or(commit)
            );
            eprintln!("{said}");
            let _ = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log)
                .and_then(|mut kept| writeln!(kept, "{said}"));
            Some(UNCOMPARED)
        };
        let apart = match Comparing::at(self, commit) {
            Ok(apart) => apart,
            Err(why) => return unstarted(why),
        };
        let output = match file(log) {
            Ok(output) => output,
            Err(why) => return unstarted(format!("{}: {why}", log.display())),
        };
        let mut clock = Clock::new(log, Instant::now());
        let child = output.try_clone().and_then(|out| {
            Command::new("nix-offload")
                .args(mac_args(base))
                .current_dir(&apart.folder)
                .stdin(Stdio::null())
                .stdout(out)
                .stderr(output)
                .spawn()
        });
        let child = match child {
            Ok(child) => child,
            Err(why) => return unstarted(format!("nix-offload: {why}")),
        };
        let code = ended(child, &mut clock, commit, log, dropped);
        mac_ended(code, &clock, log)
    }
}

/// How a Mac's comparison that ended with `code` is read, from its own
/// part of `log`, which the runs before it on the same merge share.
fn mac_ended(code: Option<i32>, clock: &Clock, log: &Path) -> Option<i32> {
    let read = mac_ending(code, clock.ours(log).as_deref());
    // net.log points at the run's log for why it compared nothing, and a
    // run ended before a builder took it printed nothing that says so.
    if read == Some(UNCOMPARED) {
        let why = if code.is_none() {
            "it was ended before any Mac took it"
        } else {
            "it failed only on reaching a cache, which says nothing of the change"
        };
        let _ = fs::OpenOptions::new()
            .append(true)
            .open(log)
            .and_then(|mut kept| writeln!(kept, "merge-net: this compared nothing: {why}"));
    }
    read
}

/// Whether a run's part of its log says a builder took it.
fn taken_in(ours: &[u8]) -> bool {
    ours.windows(TAKEN.len())
        .any(|seen| seen == TAKEN.as_bytes())
}

/// What a Mac's comparison that ended with `code` says of its riders,
/// read with `ours`, its run's part of the log: `UNCOMPARED` for one
/// that compared nothing, so it waits for the next round. One whose
/// process was ended by a signal before any builder took it ran nothing:
/// on 17 November a hold on the Mac's load stopped 135 that way, each
/// read as a stall, and 65 merges were let go unchecked. One that failed
/// on the network alone says nothing of the change either: on 19 November
/// two such failed #5650's comparison and told its owner so. A log that
/// can't be read leaves the ending as it was.
fn mac_ending(code: Option<i32>, ours: Option<&[u8]>) -> Option<i32> {
    let Some(ours) = ours else {
        return match code {
            Some(UNSENT | UNREACHED) => Some(UNSENT),
            code => code,
        };
    };
    match code {
        None if !taken_in(ours) => Some(UNCOMPARED),
        Some(UNSENT | UNREACHED) => Some(UNSENT),
        Some(code) if code != 0 && unresolved(&String::from_utf8_lossy(ours)) => Some(UNCOMPARED),
        code => code,
    }
}

/// What a download says when the network or the cache failed it, not
/// what it fetched: on 19 November the Mac couldn't name its caches'
/// hosts, and a resolver that times out, a link that won't connect or a
/// cache that's down, resets the connection or fails its TLS fails the
/// same way. A 404 isn't among them, since a fetch the change names can
/// be missing.
const NETWORK_WORDS: [&str; 12] = [
    "Could not resolve host",
    "Resolving timed out",
    "Timeout was reached",
    "Failed to connect",
    "Couldn't connect to server",
    "Recv failure",
    "SSL connect error",
    "SSL_ERROR_SYSCALL",
    "HTTP error 500",
    "HTTP error 502",
    "HTTP error 503",
    "HTTP error 504",
];

/// Whether a failed download, `why` after nix's "unable to download ", was
/// a cache's: a store path's narinfo or its nar. A fetch the change names
/// at evaluation fails the same way when its host is wrong, and that's
/// the change's.
fn a_caches(why: &str) -> bool {
    why.strip_prefix('\'')
        .and_then(|quoted| quoted.split('\'').next())
        .is_some_and(|url| url.ends_with(".narinfo") || url.contains("/nar/"))
}

/// The errors a build prints after a cache couldn't be reached, as nix
/// printed them on 19 November: each error line's start, and words it must
/// hold beside.
const NETWORK_ERRORS: [(&str, &str); 5] = [
    ("Cannot build ", ""),
    ("path ", "there is no substituter that can build it"),
    ("some references of path ", "could not be realised"),
    (
        "some substitutes for the outputs of derivation ",
        "usually happens due to networking issues",
    ),
    ("Build failed due to failed dependency", ""),
];

/// What changed-checks says after any build that failed, a cache's too,
/// when it builds a Mac's checks itself: what failed is in the errors
/// above it, so it says nothing more of the change.
const CHANGED_FAILED: &str = "\u{2715} a check the change touched failed";

/// Whether a failed run's log says it failed on the network alone: some
/// download failed with `NETWORK_WORDS`, every other error is one
/// `NETWORK_ERRORS` names, every reason is that dependencies failed, and
/// no other line says why, as one that a check no longer evaluates. An evaluation error, a builder that
/// failed (which a fetch failing on the network prints too, so that stays
/// a failure) or a check that failed beside a lookup error is a failure.
fn unresolved(words: &str) -> bool {
    let mut named = false;
    for line in words.lines().map(str::trim_start) {
        if let Some(error) = line.strip_prefix("error:") {
            let error = error.trim_start();
            let unreached = error
                .strip_prefix("unable to download ")
                .is_some_and(|why| {
                    a_caches(why) && NETWORK_WORDS.iter().any(|word| why.contains(word))
                });
            let known = unreached
                || NETWORK_ERRORS
                    .iter()
                    .any(|(start, beside)| error.starts_with(start) && error.contains(beside));
            if !known {
                return false;
            }
            named |= unreached;
        } else if let Some(reason) = line.strip_prefix("Reason:") {
            let reason = reason.trim();
            if !(reason.ends_with(" dependencies failed.")
                || reason.ends_with(" dependency failed."))
            {
                return false;
            }
        } else if line.starts_with('\u{2715}') && line != CHANGED_FAILED {
            return false;
        }
    }
    named
}

/// What the net asks `nix-offload there` to compare a Mac's checks with,
/// against `base`: the arguments a gate keeps for a comparison it skipped.
pub fn mac_args(base: &str) -> Vec<String> {
    ["there", "--changed", "--build", "--no-vm", "--base", base]
        .into_iter()
        .chain(["--system", crate::macs::SYSTEM])
        .map(str::to_owned)
        .collect()
}

/// How `child`, building at `commit` into `log`, ended; none when it was
/// dropped or ran past its time, which stops it.
fn ended(
    mut child: std::process::Child,
    clock: &mut Clock,
    commit: &str,
    log: &Path,
    dropped: &dyn Fn() -> bool,
) -> Option<i32> {
    loop {
        match child.try_wait() {
            // A run whose builder couldn't fetch the commit's source ran
            // nothing, so it's sent again like one no builder took.
            Ok(Some(status)) if !status.success() && unfetched(lines_of(log), commit) => {
                return Some(UNSENT);
            }
            Ok(Some(status)) => return status.code(),
            Ok(None) if dropped() || clock.overdue(log, Instant::now()) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_secs(5)),
            Err(_) => return None,
        }
    }
}

/// What the net asks `nix-offload there` to build `check` with at
/// `flake`: the check, or with `picked` only those of its subtests, built
/// from the flake's `subtests` as `run-subtests --there` asks for them.
/// Test and subtest names hold only letters, digits, `-` and `_`, so
/// each is a Nix string as JSON writes it.
pub fn build_args(
    flake: &str,
    system: &str,
    check: &str,
    picked: &[&str],
    takes: &str,
) -> Vec<String> {
    let test = subtests::whole(check);
    if picked.is_empty() {
        return there_args(flake, &format!("checks.{system}.{test}"), takes);
    }
    let words: Vec<String> = picked
        .iter()
        .map(|word| Value::from(*word).to_string())
        .collect();
    let apply = format!(
        "subtests: subtests {{ check = {}; words = [ {} ]; }}",
        Value::from(test),
        words.join(" ")
    );
    let mut args = there_args(flake, &format!("legacyPackages.{system}.subtests"), takes);
    let at = args.len() - 2;
    args.splice(at..at, ["--apply".to_owned(), apply]);
    args
}

/// The hold a pause file in the net's state folder `state` makes, which no
/// session's address holds, so cadence asks no one to end it.
fn paused(state: &Path) -> Option<holds::Held> {
    let file = state.join("pause");
    file.exists().then(|| holds::Held {
        why: format!("{} is there", file.display()),
        from: None,
    })
}

impl World for Real {
    fn held(&self) -> Option<holds::Held> {
        paused(&self.state).or_else(|| holds::standing(&holds::messages(), self.booted))
    }

    fn head(&self) -> Result<String, String> {
        self.cloned()?;
        self.git(&["fetch", "--quiet", "origin", &self.branch])?;
        self.git(&["rev-parse", &self.followed()])
    }

    fn fetched(&self) -> Option<String> {
        self.git(&["rev-parse", &self.followed()]).ok()
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

    fn parent(&self, commit: &str) -> Result<String, String> {
        self.git(&["rev-parse", &format!("{commit}^1")])
    }

    fn mac_skipped(&self, merge: &str) -> bool {
        self.git(&["rev-parse", &format!("{merge}^2")])
            .is_ok_and(|head| crate::macs::skipped_at(&self.state, &head))
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
        let apart = Comparing::at(self, head)?;
        let log = self
            .state
            .join("logs")
            .join(format!("changed-{}.log", short(head)));
        let ran = Command::new("changed-checks")
            .args([
                "--base",
                base,
                "--name-only",
                "--system",
                &self.system,
                FIRST,
            ])
            .current_dir(&apart.folder)
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
        if crate::macs::base_of(check).is_some() {
            return Ok(true);
        }
        let check = subtests::whole(check);
        let attribute = format!("{}#checks.{}", self.flake(commit), self.system);
        let asked = format!("checks: builtins.hasAttr \"{check}\" checks");
        let said = self.evaluated(&attribute, &["--json", "--apply", &asked])?;
        Ok(said == "true")
    }

    fn drv(&self, commit: &str, check: &str) -> Result<String, String> {
        // A Mac's comparison builds nothing of its own name, and runs
        // again at every other commit.
        if crate::macs::base_of(check).is_some() {
            return Ok(format!("{check}@{commit}"));
        }
        let check = subtests::whole(check);
        let attribute = format!(
            "{}#checks.{}.{check}.drvPath",
            self.flake(commit),
            self.system
        );
        kept_drv(&self.state.join("drv"), commit, check, || {
            self.evaluated(&attribute, &["--raw"])
        })
    }

    fn build(&self, commit: &str, check: &str, log: &Path) -> Option<i32> {
        let dropped = || self.dropped(commit, check);
        if let Some(base) = crate::macs::base_of(check) {
            return self.compared_on_a_mac(commit, base, log, &dropped);
        }
        let picked = subtests::subtests_of(check);
        let code = self.built(commit, check, &picked, log, &dropped)?;
        if picked.is_empty() || code == 0 {
            return Some(code);
        }
        if subtests::none_selected(lines_of(log)) {
            return Some(0);
        }
        if subtests::unbuildable(lines_of(log)) {
            return self.built(commit, subtests::whole(check), &[], log, &dropped);
        }
        Some(code)
    }

    fn quarantined(&self, head: &str) -> Vec<String> {
        crate::quarantine::whole(&self.listed_at(head))
    }

    fn listed_subtests(&self, head: &str, test: &str) -> BTreeSet<String> {
        crate::quarantine::subtests_of(&self.listed_at(head), test)
    }

    // Main's own changes to the list are walked newest first, as the first
    // parent's diff is a merge's own, back to the newest whose list still
    // held what fails; the change after it took that off. Asking only for
    // a merge whose list lacked the whole test missed an owner taking off
    // their own entry while another's stayed, or a subtest of their entry.
    fn unlisted_by(&self, test: &str, failing: &BTreeSet<String>, head: &str) -> Option<u64> {
        let moved = self
            .git(&[
                "log",
                "--first-parent",
                "--format=%H",
                head,
                "--",
                QUARANTINE,
            ])
            .ok()?;
        let mut after = None;
        for commit in moved.lines() {
            if crate::quarantine::covers(&self.listed_at(commit), test, failing) {
                return after.and_then(|took: &str| crate::watch::pr_of(&self.subject(took)));
            }
            after = Some(commit);
        }
        None
    }

    fn revert(
        &self,
        bad: &Bad,
        still: &dyn Fn(&str, &str) -> Result<bool, String>,
    ) -> Result<Merged, String> {
        let what = bad
            .pr
            .map_or_else(|| short(&bad.commit).to_owned(), |pr| format!("#{pr}"));
        let message = format!(
            "revert: undo {what}, which breaks a check\n\n{} failed twice at {}, on the same\ninput, so it comes off main until it's fixed forward.\nReverts \"{}\".",
            bad.test, bad.commit, bad.subject
        );
        let parents = self.git(&["rev-list", "--parents", "-n", "1", &bad.commit])?;
        let merge = parents.split_whitespace().count() > 2;
        let body = revert_body(
            bad,
            &unnamed(&failure(Path::new(&bad.log), 15), &hostname()),
        );
        self.land(
            &format!("merge-net/revert-{}", short(&bad.commit)),
            &message,
            &format!("Revert {what}, Which Breaks {}", bad.test),
            &body,
            still,
            &|| {
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
        subtests: &BTreeSet<String>,
        owner: u64,
        commit: &str,
        reason: &str,
    ) -> Result<Merged, String> {
        let path = self.checkout().join(QUARANTINE);
        // An entry naming subtests excuses only them (quarantine.rs), so
        // the description says which.
        let what = if subtests.is_empty() {
            test.to_owned()
        } else {
            let named: Vec<&str> = subtests.iter().map(String::as_str).collect();
            format!("{test}'s {}", named.join(", "))
        };
        let body = quarantine_body(&what, owner, commit, reason);
        self.land(
            &format!("merge-net/quarantine-{test}"),
            &format!("test(merge-net): quarantine a failing check\n\n{what} {reason}, so the net reverts nothing for it\nuntil its owner mends it."),
            &format!("Quarantine {test} Until Its Owner Mends It"),
            &body,
            &|_, _| Ok(true),
            &|| {
                let mut list = fs::read_to_string(&path)
                    .ok()
                    .and_then(|text| serde_json::from_str::<Value>(&text).ok())
                    .unwrap_or_else(|| json!([]));
                let entries = list.as_array_mut().ok_or("the quarantine list isn't a list")?;
                let fields = json!({"reason": reason, "owner_pr": owner, "commit": commit, "at": now()});
                crate::quarantine::list(entries, test, subtests, fields);
                let written = serde_json::to_string_pretty(&list).map_err(|error| error.to_string())?;
                fs::write(&path, written + "\n").map_err(|error| format!("writing {}: {error}", path.display()))?;
                self.git(&["add", QUARANTINE]).map(|_| ())
            },
        )
    }

    fn on_main(&self, commit: &str) -> bool {
        self.git(&["merge-base", "--is-ancestor", commit, &self.followed()])
            .is_ok()
    }

    /// Read in update-to-main's journal, which says each switch's core, a
    /// week of it: a hold is written or looked at once or twice a round, so
    /// its 40 to 50 seconds with the disk busy on 10 November cost little
    /// beside a round's hours.
    fn runs(&self, commit: &str) -> bool {
        let journal = run(Command::new("journalctl").args([
            "--unit=update-to-main.service",
            "--output=cat",
            "--no-pager",
            "--since=-7d",
            "--grep=\"step\":\"done\"",
        ]));
        journal
            .ok()
            .as_deref()
            .and_then(running::core)
            .is_some_and(|core| {
                self.git(&["merge-base", "--is-ancestor", commit, &core])
                    .is_ok()
            })
    }

    fn wait(&self, seconds: u64) {
        std::thread::sleep(Duration::from_secs(seconds));
    }

    fn drop_runs(&self, commit: &str, check: &str) {
        let run = (commit.to_owned(), check.to_owned());
        self.dropped
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(run);
    }

    fn keep_runs(&self, commit: &str, check: &str) {
        let run = (commit.to_owned(), check.to_owned());
        self.dropped
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&run);
    }

    fn dropped(&self, commit: &str, check: &str) -> bool {
        let run = (commit.to_owned(), check.to_owned());
        self.dropped
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(&run)
    }

    fn superseded(&self) -> bool {
        self.follow
            .as_ref()
            .is_some_and(|installed| replaced(fs::canonicalize(installed), std::env::current_exe()))
    }
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
         - Checked: {test} failed twice at {short} on a builder, through nix-offload there.\n\
         - Checked: changed-checks passed on this branch, for the checks it names other than VM tests \
         and those that fail before it too.\n\
         - Watch: the net runs {test} at the next head; a failure there means this didn't undo it.\n",
        test = bad.test,
        commit = bad.commit,
        subject = plain(&bad.subject),
        short = short(&bad.commit),
    );
    body
}

/// A quarantine's description, in the four sections every PR has.
pub fn quarantine_body(test: &str, owner: u64, commit: &str, reason: &str) -> String {
    let owner = format!("the owner of #{owner}");
    format!(
        "### 💡 TL;DR\n\n{test} goes on the net's quarantine list, since it {reason}. The net still runs it, \
         but reverts nothing when it fails.\n\n\
         ### 🧩 Problem\n\n{test} {reason}, so its failures say nothing about the merges the net checks, \
         and reverting for them would undo good work.\n\n\
         ### 🔧 Solution\n\nThis adds {test} to the quarantine list, with its reason and owner, {owner}, who \
         mends it and takes it off.\n\n\
         ### 🔬 Validation\n\n\
         - Checked: on the VM builder, {test} {reason}, as of {commit}.\n\
         - Checked: changed-checks passed on this branch, for the checks it names other than those \
         that fail before it too.\n"
    )
}

/// A log's last lines, as plain text a PR can hold: no colour codes, and
/// only ASCII, so no character the prose rules refuse comes in with it.
pub fn tail(log: &Path, lines: usize) -> String {
    let mut kept = VecDeque::with_capacity(lines + 1);
    for line in lines_of(log).map(|line| plain(&line)) {
        if line.trim().is_empty() {
            continue;
        }
        kept.push_back(line);
        if kept.len() > lines {
            kept.pop_front();
        }
    }
    Vec::from(kept).join("\n")
}

/// What says a test failed and why, read out of its log: the lines pytest
/// and the driver mark a failure with, each once, however often nix repeats
/// them. A log's last lines are its machines shutting down, which say
/// nothing; they're given only when no line is marked.
pub fn failure(log: &Path, lines: usize) -> String {
    const MARKS: [&str; 5] = [
        "FAILED ",
        "AssertionError",
        "Error: ",
        "failed with error",
        "Traceback",
    ];
    let mut said: Vec<String> = Vec::new();
    for line in lines_of(log).map(|line| plain(&line)) {
        let bare = line
            .split_once("> ")
            .map_or(line.as_str(), |(_, rest)| rest)
            .trim()
            .to_owned();
        if MARKS.iter().any(|mark| bare.contains(mark)) && !said.contains(&bare) {
            said.push(bare);
        }
    }
    if said.is_empty() {
        return tail(log, lines);
    }
    said[said.len().saturating_sub(lines)..].join("\n")
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
                for next in chars.by_ref() {
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
/// A checkout of its own for one comparison, at the head it compares, so
/// it never waits on the net's checkout or another comparison. Holding the
/// net's checkout for a whole changed-checks run, minutes each, kept each
/// round's comparison behind the quick look's on 14 November: 4 to 17
/// minutes before a round's first VM test began. It's given back when
/// dropped.
struct Comparing<'a> {
    net: &'a Real,
    folder: PathBuf,
}

/// How many comparisons this net has made, so each checkout is named apart.
static COMPARISONS: AtomicU64 = AtomicU64::new(0);

impl<'a> Comparing<'a> {
    fn at(net: &'a Real, head: &str) -> Result<Self, String> {
        let within = net.state.join("compare");
        clear_ended(net, &within);
        let nth = COMPARISONS.fetch_add(1, Ordering::Relaxed);
        let folder = within.join(format!("{}-{nth}-{}", std::process::id(), short(head)));
        let path = folder.display().to_string();
        net.git(&[
            "worktree", "add", "--quiet", "--force", "--detach", &path, head,
        ])?;
        Ok(Self { net, folder })
    }
}

impl Drop for Comparing<'_> {
    fn drop(&mut self) {
        let path = self.folder.display().to_string();
        if self
            .net
            .git(&["worktree", "remove", "--force", &path])
            .is_err()
        {
            let _ = fs::remove_dir_all(&self.folder);
            let _ = self.net.git(&["worktree", "prune"]);
        }
    }
}

/// Take back the comparisons' checkouts a net that ended left, as one
/// stopped mid-run does. Each is named by its net's process first, and
/// two nets overlap while one moves to a new build, so only those of a
/// process that's gone are cleared.
fn clear_ended(net: &Real, within: &Path) {
    let Ok(entries) = fs::read_dir(within) else {
        return;
    };
    let mut cleared = false;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let process = name.split('-').next().unwrap_or_default();
        if process.is_empty() || Path::new("/proc").join(process).exists() {
            continue;
        }
        let _ = fs::remove_dir_all(entry.path());
        cleared = true;
    }
    if cleared {
        let _ = net.git(&["worktree", "prune"]);
    }
}

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

/// The derivation a check builds at a commit, kept in `dir` once known.
/// A commit's derivation never changes, but a search asks it of the same
/// merge again and again, and each ask evaluated the flake afresh:
/// seen in the CPU alarms of 8 November, at 1.5 to 3 GB an evaluation.
/// Only a full commit hash is kept, since a name like a branch moves.
fn kept_drv(
    dir: &Path,
    commit: &str,
    check: &str,
    evaluate: impl FnOnce() -> Result<String, String>,
) -> Result<String, String> {
    let fixed = commit.len() == 40 && commit.bytes().all(|byte| byte.is_ascii_hexdigit());
    let kept = dir.join(format!("{commit}-{check}"));
    if fixed
        && let Ok(drv) = fs::read_to_string(&kept)
        && drv.starts_with("/nix/store/")
    {
        return Ok(drv);
    }
    let drv = evaluate()?;
    // A memo not written costs only the next evaluation, so it never fails
    // the ask.
    if fixed && drv.starts_with("/nix/store/") {
        let _ = fs::create_dir_all(dir).and_then(|()| fs::write(&kept, &drv));
    }
    Ok(drv)
}

/// Run a command, giving what it printed, trimmed, or what it said wrong.
fn run(command: &mut Command) -> Result<String, String> {
    let ran = command
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("running {}: {error}", command.get_program().display()))?;
    if !ran.status.success() {
        // git's merge, told to be quiet, says a conflict on its output
        // alone, and a failure that says nothing can't be told from another.
        let said = String::from_utf8_lossy(&ran.stderr).trim().to_owned();
        let said = if said.is_empty() {
            String::from_utf8_lossy(&ran.stdout).trim().to_owned()
        } else {
            said
        };
        return Err(format!(
            "{} failed: {said}",
            command.get_program().display()
        ));
    }
    Ok(String::from_utf8_lossy(&ran.stdout).trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::git_in;

    // cadence asks `from` to end a long stop, so a pause, which no session
    // holds, must name none: as "pause" it read as a session's address.
    #[test]
    fn a_pause_holds_the_net_in_no_sessions_name() {
        let dir = std::env::temp_dir().join(format!("merge-net-paused-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(paused(&dir), None, "no pause file held the net");
        fs::write(dir.join("pause"), "").unwrap();
        let held = paused(&dir);
        fs::remove_dir_all(&dir).unwrap();
        let held = held.expect("a pause file didn't hold the net");
        assert_eq!(held.from, None, "a pause named a session: {held:?}");
    }

    #[test]
    fn the_watch_ends_once_another_build_is_installed() {
        let dir = std::env::temp_dir().join(format!("merge-net-follow-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let (old, new) = (dir.join("old"), dir.join("new"));
        fs::write(&old, "").unwrap();
        fs::write(&new, "").unwrap();
        let installed = || fs::canonicalize(&new);
        assert!(
            !replaced(installed(), Ok(new.clone())),
            "the build installed runs"
        );
        assert!(
            replaced(installed(), Ok(old.clone())),
            "an older build runs"
        );
        assert!(
            replaced(installed(), Ok(dir.join("collected"))),
            "the running build is gone from disk"
        );
        assert!(
            !replaced(fs::canonicalize(dir.join("unread")), Ok(old)),
            "nothing can be read of what's installed"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_pr_left_open_for_the_branch_is_taken_up_rather_than_failing() {
        let why = "gh failed: a pull request for branch \"merge-net/quarantine-x\" into branch \"main\" \
                   already exists:\nhttps://github.com/someone/core.nix/pull/2941";
        assert_eq!(
            open_already(why),
            Some(2941),
            "the open PR wasn't found in {why}"
        );
        assert_eq!(
            open_already("gh failed: HTTP 502: Bad Gateway https://github.com/x/y/pull/7"),
            None,
            "a failure other than an open PR was taken for one"
        );
        assert_eq!(
            pr_named("https://github.com/someone/core.nix/pull/2952\n"),
            Some(2952),
            "the PR gh opened wasn't read from its link"
        );
    }

    #[test]
    fn a_merge_github_refused_before_seeing_the_push_is_asked_again() {
        assert!(
            head_not_seen(
                "gh failed: GraphQL: Head branch was modified. Review and try the merge again. \
                 (mergePullRequest)"
            ),
            "GitHub's lag after a push wasn't seen as one"
        );
        assert!(
            !head_not_seen("gh failed: GraphQL: Base branch was modified. (mergePullRequest)"),
            "a moved main, which the net lands onto afresh, was asked again"
        );
    }

    #[test]
    fn a_merge_refused_because_main_moved_says_so() {
        let base = "gh failed: GraphQL: Base branch was modified. Review and try the merge \
                    again. (mergePullRequest)";
        assert!(
            refused(4049, base).starts_with(MOVED),
            "a merge refused as main moved wasn't said to be: {}",
            refused(4049, base)
        );
        assert_eq!(
            refused(4049, "gh failed: the PR is closed"),
            "#4049 is open but didn't merge: gh failed: the PR is closed",
            "another refusal wasn't said as before"
        );
    }

    #[test]
    fn a_commits_check_is_evaluated_once_and_a_moving_name_every_time() {
        let dir = std::env::temp_dir().join(format!("merge-net-drv-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let commit = "df45383375c37fbbfb1cee0afa8943ff123e4ac6";
        let drv = "/nix/store/aaaa-vm-a.drv";
        let asked = std::cell::Cell::new(0);
        let evaluate = || {
            asked.set(asked.get() + 1);
            Ok(drv.to_owned())
        };
        assert_eq!(kept_drv(&dir, commit, "vm-a", evaluate), Ok(drv.to_owned()));
        assert_eq!(kept_drv(&dir, commit, "vm-a", evaluate), Ok(drv.to_owned()));
        assert_eq!(asked.get(), 1, "a commit's derivation is kept once known");
        kept_drv(&dir, "origin/main", "vm-a", evaluate).expect("evaluated");
        kept_drv(&dir, "origin/main", "vm-a", evaluate).expect("evaluated");
        assert_eq!(asked.get(), 3, "a name that moves is evaluated each time");
        let failed = kept_drv(&dir, commit, "vm-b", || Err("nix failed".to_owned()));
        assert!(failed.is_err());
        assert!(
            !dir.join(format!("{commit}-vm-b")).exists(),
            "a failure isn't kept"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_source_the_builder_couldnt_fetch_is_no_failure() {
        let commit = "df45383375c37fbbfb1cee0afa8943ff123e4ac6";
        let log = format!(
            "error:\n       \u{2026} while fetching the input 'github:someone/core/{commit}'\n\n\
             \x20      error: Failed to open archive (Source threw exception: error: unable \
             to download 'https://github.com/someone/core/archive/{commit}.tar.gz': \
             HTTP error 503\n"
        );
        assert!(unfetched(log.lines(), commit));
        assert!(
            !unfetched(log.lines(), "0123456789abcdef0123456789abcdef01234567"),
            "another input's fetch is the test's own to fail on"
        );
        assert!(!unfetched(
            "error: builder failed with exit code 1".lines(),
            commit
        ));
    }

    #[test]
    fn a_build_log_names_the_checks_it_couldnt_build() {
        let log = "building '/nix/store/aaa-stands-alone.drv'...\n\
            error: Cannot build '/nix/store/abc123-nothing-names-its-own-look.drv'.\n\
            \x20      Reason: builder failed with exit code 1.\n\
            error: Cannot build '/nix/store/def456-vm-clock-keeps-time.drv'.\n\
            error: Cannot build '/nix/store/abc123-nothing-names-its-own-look.drv'.\n";
        assert_eq!(
            unbuilt(log.lines()),
            vec![
                "nothing-names-its-own-look".to_owned(),
                "vm-clock-keeps-time".to_owned()
            ]
        );
    }

    #[test]
    fn a_run_is_bulk_only_where_nix_offload_takes_it() {
        let flake = "git+file:///net/core?rev=abc";
        let attribute = "checks.x86_64-linux.vm-a";
        assert_eq!(
            there_args(flake, attribute, "tree\napply\noutlive\nbulk\n"),
            ["there", "--bulk", "-L", flake, attribute]
        );
        assert_eq!(
            there_args(flake, attribute, "tree\napply\noutlive\n"),
            ["there", "-L", flake, attribute],
            "an older binary would refuse it"
        );
        assert_eq!(
            there_args(flake, attribute, ""),
            ["there", "-L", flake, attribute],
            "none could be read"
        );
    }

    #[test]
    fn a_search_builds_only_the_subtests_it_was_narrowed_to() {
        let flake = "github:o/core/abc";
        assert_eq!(
            build_args(flake, "x86_64-linux", "vm-a", &[], "bulk\n"),
            ["there", "--bulk", "-L", flake, "checks.x86_64-linux.vm-a"],
            "a whole test is its check"
        );
        assert_eq!(
            build_args(
                flake,
                "x86_64-linux",
                "vm-a::test_b::test_c",
                &["test_b", "test_c"],
                "bulk\n"
            ),
            [
                "there",
                "--bulk",
                "-L",
                "--apply",
                "subtests: subtests { check = \"vm-a\"; words = [ \"test_b\" \"test_c\" ]; }",
                flake,
                "legacyPackages.x86_64-linux.subtests"
            ],
            "a narrowed one is built from the flake's subtests, as run-subtests asks"
        );
    }

    #[test]
    fn a_derivation_is_evaluated_on_a_builder_never_here() {
        let attribute = "github:o/core/abc#checks.x86_64-linux.vm-a.drvPath";
        assert_eq!(
            eval_args(attribute, &["--raw"]),
            [
                "there",
                "--run",
                "--longest",
                "600",
                "--",
                "nix",
                "eval",
                "--raw",
                attribute
            ],
            "an evaluation here holds most of a GB of the person's machine"
        );
        let checks = "github:o/core/abc#checks.x86_64-linux";
        let asked = "checks: builtins.hasAttr \"vm-a\" checks";
        assert_eq!(
            eval_args(checks, &["--json", "--apply", asked])[5..],
            ["nix", "eval", "--json", "--apply", asked, checks],
            "what it's asked goes with it"
        );
    }

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
    fn a_run_waiting_its_turn_is_never_overdue() {
        let log = std::env::temp_dir().join(format!("merge-net-queue-{}.log", std::process::id()));
        let _ = fs::remove_file(&log);
        let start = Instant::now();
        let mut clock = Clock::new(&log, start);
        fs::write(
            &log,
            "nix-offload: the VM builder has 4 VM tests, so this waits for room\n",
        )
        .unwrap();
        assert!(!clock.overdue(&log, start));
        assert!(
            !clock.overdue(&log, start + LONGEST * 2),
            "queued, however long"
        );
        let taken = start + LONGEST * 2;
        fs::write(
            &log,
            "waits for room\nnix-offload: building on the lent builder b\n",
        )
        .unwrap();
        assert!(!clock.overdue(&log, taken));
        assert!(!clock.overdue(&log, taken + SILENT));
        assert!(
            clock.overdue(&log, taken + SILENT + Duration::from_secs(1)),
            "silent"
        );
        let _ = fs::remove_file(&log);
    }

    #[test]
    fn a_run_cut_short_before_it_leaves_this_one_waiting_its_turn() {
        let log = std::env::temp_dir().join(format!("merge-net-after-{}.log", std::process::id()));
        fs::write(
            &log,
            "nix-offload: building on the lent builder b\nhalfway\n",
        )
        .unwrap();
        let start = Instant::now();
        let mut clock = Clock::new(&log, start);
        let mut text = fs::read_to_string(&log).unwrap();
        text.push_str("nix-offload: 15 runs wait ahead of this, so this waits for room\n");
        fs::write(&log, &text).unwrap();
        assert!(!clock.overdue(&log, start));
        assert!(
            !clock.overdue(&log, start + SILENT * 2),
            "queued, though the run before it was taken"
        );
        let taken = start + SILENT * 2;
        text.push_str("nix-offload: building on the lent builder b\n");
        fs::write(&log, &text).unwrap();
        assert!(!clock.overdue(&log, taken));
        assert!(clock.overdue(&log, taken + SILENT + Duration::from_secs(1)));
        let _ = fs::remove_file(&log);
    }

    #[test]
    fn a_run_on_a_builder_too_long_is_overdue() {
        let log = std::env::temp_dir().join(format!("merge-net-long-{}.log", std::process::id()));
        let _ = fs::remove_file(&log);
        let start = Instant::now();
        let mut clock = Clock::new(&log, start);
        let mut text = String::from("nix-offload: building on b, flake\n");
        let mut now = start;
        while now <= start + LONGEST {
            text.push_str("still going\n");
            fs::write(&log, &text).unwrap();
            assert!(!clock.overdue(&log, now));
            now += Duration::from_mins(10);
        }
        text.push_str("still going\n");
        fs::write(&log, &text).unwrap();
        assert!(
            clock.overdue(&log, now),
            "past the longest, though never silent"
        );
        let _ = fs::remove_file(&log);
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
    fn a_quarantine_of_a_broken_test_says_it_broke_rather_than_that_it_passed() {
        let body = quarantine_body(
            "vm-a",
            2900,
            "e38290ad",
            "has been broken since #2900, whose revert won't apply",
        );
        assert!(
            body.contains("since it has been broken since #2900"),
            "the TL;DR doesn't give the reason: {body}"
        );
        assert!(
            !body.contains("passed at"),
            "a broken test's quarantine says it passed: {body}"
        );
    }

    /// The tests a quarantine list names, in its order.
    fn tests_in(list: &Value) -> Vec<String> {
        list.as_array()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry["test"].as_str().map(str::to_owned))
            .collect()
    }

    #[test]
    fn a_log_is_read_a_line_at_a_time_as_str_lines_reads_it() {
        let log = std::env::temp_dir().join(format!("merge-net-lines-{}.log", std::process::id()));
        fs::write(&log, b"one\r\ntw\xffo\n\nthree").unwrap();
        assert_eq!(
            lines_of(&log).collect::<Vec<_>>(),
            ["one", "tw\u{fffd}o", "", "three"],
            "a carriage return ends a line, and a byte that isn't UTF-8 is replaced"
        );
        fs::remove_file(&log).unwrap();
        assert_eq!(
            lines_of(&log).count(),
            0,
            "a log that isn't there has no lines"
        );
    }

    #[test]
    fn a_log_gives_its_last_lines_that_say_something() {
        let log = std::env::temp_dir().join(format!("merge-net-tail-{}.log", std::process::id()));
        fs::write(&log, "one\n\ntwo\nthree\n\n").unwrap();
        assert_eq!(tail(&log, 2), "two\nthree");
        fs::remove_file(&log).unwrap();
    }

    #[test]
    fn a_failure_is_read_from_the_lines_that_mark_it_not_the_shutdown() {
        let log =
            std::env::temp_dir().join(format!("merge-net-failure-{}.log", std::process::id()));
        fs::write(
            &log,
            "vm-test-run-clock> machine # RAS: Correctable Errors collector initialized.\n\
             vm-test-run-clock> E       AssertionError: assert 'planted' == 'caught'\n\
             vm-test-run-clock> FAILED clock-vm-test.py::test_planted - AssertionError\n\
             vm-test-run-clock> server # qemu-system-x86_64: terminating on signal 15\n\
             \x20      > FAILED clock-vm-test.py::test_planted - AssertionError\n\
             \x20      > (finished: cleanup, in 0.56 seconds)\n",
        )
        .unwrap();
        assert_eq!(
            failure(&log, 15),
            "E       AssertionError: assert 'planted' == 'caught'\n\
             FAILED clock-vm-test.py::test_planted - AssertionError"
        );
        fs::write(&log, "one\ntwo\n").unwrap();
        assert_eq!(
            failure(&log, 1),
            "two",
            "with nothing marked, its last lines"
        );
        fs::remove_file(&log).unwrap();
    }

    #[test]
    fn a_failure_that_says_nothing_on_its_errors_says_what_it_printed() {
        let why = run(Command::new("sh").args(["-c", "echo CONFLICT in a; exit 1"]));
        assert_eq!(
            why,
            Err("sh failed: CONFLICT in a".to_owned()),
            "a failure printed only on its output was said as nothing"
        );
    }

    /// Writes `list` as the quarantine list in `repo` and commits it there
    /// as someone, giving the commit.
    fn listed(repo: &Path, list: &Value) -> String {
        let path = repo.join(QUARANTINE);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, serde_json::to_string_pretty(list).unwrap() + "\n").unwrap();
        git_in(repo, &["add", QUARANTINE]);
        git_in(repo, &["commit", "-qm", "listed"]);
        git_in(repo, &["rev-parse", "HEAD"])
    }

    /// A net whose checkout is cloned from a repository of its own, with
    /// main's quarantine list holding `held`, and the folder it's all in.
    fn a_net(name: &str, held: &Value) -> (Real, PathBuf) {
        let root = std::env::temp_dir().join(format!("merge-net-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let origin = root.join("origin.git");
        let main = root.join("main");
        fs::create_dir_all(&origin).unwrap();
        git_in(&origin, &["init", "-q", "--bare", "-b", "main"]);
        git_in(&root, &["clone", "-q", "origin.git", "main"]);
        listed(&main, held);
        git_in(&main, &["push", "-q", "origin", "HEAD:main"]);
        let net = Real {
            state: root.join("net"),
            url: origin.display().to_string(),
            system: "x86_64-linux".to_owned(),
            booted: 0,
            branch: "main".to_owned(),
            moving: Mutex::new(()),
            dropped: Mutex::default(),
            follow: None,
        };
        fs::create_dir_all(&net.state).unwrap();
        net.cloned().unwrap();
        git_in(&net.checkout(), &["config", "user.name", "net"]);
        git_in(&net.checkout(), &["config", "user.email", "net@t"]);
        (net, root)
    }

    #[test]
    fn a_mac_comparison_that_cannot_start_compared_nothing() {
        // Read as a stall, it would be told to the merge's owner, though
        // the cause, here a commit the checkout lacks, is the net's.
        let (net, root) = a_net("unstarted-mac", &json!([]));
        let log = root.join("unstarted.log");
        let ended = net.compared_on_a_mac(&"0".repeat(40), "base", &log, &|| false);
        let said = fs::read_to_string(&log).unwrap_or_default();
        let _ = fs::remove_dir_all(&root);
        assert_eq!(
            ended,
            Some(UNCOMPARED),
            "a Mac's comparison that couldn't start wasn't read as comparing nothing"
        );
        assert!(
            said.contains("couldn't start: "),
            "why a Mac's comparison couldn't start wasn't kept in its log: {said}"
        );
    }

    #[test]
    fn a_landing_asks_whether_it_is_still_wanted_with_the_checkout_given_back() {
        let moving = Mutex::new(());
        let free = Mutex::new(None);
        let still = |_: &str, _: &str| {
            *free.lock().unwrap() = Some(moving.try_lock().is_ok());
            Ok(true)
        };
        let wanted = asked_free(moving.lock().unwrap(), &still, "newest", "sha");
        assert_eq!(wanted, Ok(true), "what still said was lost");
        assert_eq!(
            *free.lock().unwrap(),
            Some(true),
            "still was asked while the landing held the checkout, so a quarantine it lands \
             would wait on the landing forever"
        );
    }

    #[test]
    fn a_comparison_leaves_the_checkout_to_a_landing() {
        let (net, root) = a_net("apart", &json!([]));
        let main = root.join("main");
        let base = git_in(&main, &["rev-parse", "HEAD"]);
        fs::write(main.join("other"), "moved\n").unwrap();
        git_in(&main, &["add", "other"]);
        git_in(&main, &["commit", "-qm", "moved"]);
        git_in(&main, &["push", "-q", "origin", "HEAD:main"]);
        let head = git_in(&main, &["rev-parse", "HEAD"]);
        git_in(&net.checkout(), &["fetch", "-q", "origin", "main"]);
        let before = git_in(&net.checkout(), &["rev-parse", "HEAD"]);
        let (tell, told) = std::sync::mpsc::channel();
        std::thread::scope(|scope| {
            let landing = net.moving();
            scope.spawn(|| tell.send(net.changed(&base, &head).is_ok()));
            assert!(
                told.recv_timeout(Duration::from_secs(30)).is_ok(),
                "a comparison waited for a change landing in the checkout"
            );
            drop(landing);
        });
        assert_eq!(
            git_in(&net.checkout(), &["rev-parse", "HEAD"]),
            before,
            "a comparison moved the checkout a landing works in"
        );
        let left: Vec<_> = fs::read_dir(net.state.join("compare"))
            .map(|entries| entries.flatten().map(|entry| entry.path()).collect())
            .unwrap_or_default();
        assert!(
            left.is_empty(),
            "a comparison left its checkout behind: {left:?}"
        );
        assert!(
            !git_in(&net.checkout(), &["worktree", "list"]).contains("compare"),
            "git still lists a comparison's checkout"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_comparison_left_by_a_net_that_ended_is_cleared() {
        let (net, root) = a_net("stale", &json!([]));
        let head = git_in(&root.join("main"), &["rev-parse", "HEAD"]);
        // No process has this id: Linux's ids stop well below it.
        let stale = net.state.join("compare").join("99999999-0-abc");
        git_in(
            &net.checkout(),
            &[
                "worktree",
                "add",
                "-q",
                "--detach",
                &stale.display().to_string(),
                &head,
            ],
        );
        let _ = net.changed(&head, &head);
        assert!(!stale.exists(), "a dead net's comparison checkout was kept");
        assert!(
            !git_in(&net.checkout(), &["worktree", "list"]).contains("99999999"),
            "git still lists a dead net's comparison checkout"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_quarantine_meeting_another_on_main_is_cut_again_from_the_newest() {
        let (net, root) = a_net("recut", &json!([{"test": "kept"}, {"test": "mended"}]));
        let main = root.join("main");
        let path = net.checkout().join(QUARANTINE);
        let change = || {
            let mut list: Value =
                serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
            list.as_array_mut().unwrap().push(json!({"test": "broken"}));
            fs::write(&path, serde_json::to_string_pretty(&list).unwrap() + "\n").unwrap();
            net.git(&["add", QUARANTINE]).map(|_| ())
        };
        let cut_from = net.git(&["rev-parse", &net.followed()]).unwrap();
        let branch = "merge-net/quarantine-broken";
        let sha = net.cut(branch, &cut_from, "quarantine", &change).unwrap();
        let newest = listed(&main, &json!([{"test": "kept"}]));
        git_in(&main, &["push", "-q", "origin", "HEAD:main"]);
        let landed = net.onto_newest(branch, &cut_from, sha, "quarantine", &change);
        let (on, at) = landed.expect("a quarantine meeting another on main didn't land");
        assert_eq!(on, newest, "it wasn't put on main's newest head");
        assert_eq!(
            net.git(&["rev-parse", &format!("{at}^")]).unwrap(),
            newest,
            "it wasn't cut again from main's newest head"
        );
        let list = net.git(&["show", &format!("{at}:{QUARANTINE}")]).unwrap();
        assert_eq!(
            tests_in(&serde_json::from_str(&list).unwrap()),
            ["kept", "broken"],
            "the list cut again lost main's change or its own"
        );
        assert_eq!(
            git_in(&root.join("origin.git"), &["rev-parse", branch]),
            at,
            "what was cut again wasn't pushed"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_relisting_is_owned_by_the_merge_that_took_off_what_fails_now() {
        let (net, root) = a_net(
            "unlisted",
            &json!([
                {"test": "vm-a", "owner_pr": 1, "subtests": ["test_x"]},
                {"test": "vm-a", "owner_pr": 2, "subtests": ["test_y", "test_z"]},
            ]),
        );
        let main = root.join("main");
        let merged = |list: &Value, pr: u64| {
            listed(&main, list);
            let subject = format!("Merge pull request #{pr} from someone/fix");
            git_in(&main, &["commit", "--amend", "-qm", &subject]);
            git_in(&main, &["push", "-q", "origin", "HEAD:main"]);
            net.git(&["fetch", "-q", "origin"]).unwrap();
            git_in(&main, &["rev-parse", "HEAD"])
        };
        let second = json!({"test": "vm-a", "owner_pr": 2, "subtests": ["test_y", "test_z"]});
        merged(&json!([second]), 11);
        merged(&json!([second, {"test": "vm-b"}]), 12);
        let head = merged(
            &json!([
                {"test": "vm-a", "owner_pr": 2, "subtests": ["test_y"]},
                {"test": "vm-b"},
            ]),
            13,
        );
        let by = |failing: &str| {
            let failing: BTreeSet<String> = failing.split_whitespace().map(str::to_owned).collect();
            net.unlisted_by("vm-a", &failing, &head)
        };
        assert_eq!(
            by("test_x"),
            Some(11),
            "#1 taking off its own entry, while #2's stays, wasn't found"
        );
        assert_eq!(
            by("test_z"),
            Some(13),
            "a subtest taken off an entry that stays wasn't found"
        );
        assert_eq!(
            by("test_y"),
            None,
            "test_y is held still, so nobody took it off"
        );
        let head = merged(&json!([{"test": "vm-b"}]), 14);
        assert_eq!(
            net.unlisted_by("vm-a", &BTreeSet::new(), &head),
            Some(14),
            "a test failing whole wasn't owned by the merge that took it off"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_change_beyond_the_list_meeting_another_on_main_says_where() {
        let (net, root) = a_net("conflict", &json!([{"test": "kept"}]));
        let main = root.join("main");
        let path = net.checkout().join(QUARANTINE);
        let other = net.checkout().join("other");
        let change = || {
            fs::write(&path, "[]\n").unwrap();
            fs::write(&other, "o\n").unwrap();
            net.git(&["add", QUARANTINE, "other"]).map(|_| ())
        };
        let cut_from = net.git(&["rev-parse", &net.followed()]).unwrap();
        let branch = "merge-net/revert-x";
        let sha = net.cut(branch, &cut_from, "revert", &change).unwrap();
        listed(&main, &json!([{"test": "kept"}, {"test": "new"}]));
        git_in(&main, &["push", "-q", "origin", "HEAD:main"]);
        let landed = net.onto_newest(branch, &cut_from, sha.clone(), "revert", &change);
        assert!(
            landed.as_ref().is_err_and(|why| why.contains("CONFLICT")),
            "a change beyond the list was cut again, or its conflict went unsaid: {landed:?}"
        );
        assert_eq!(
            net.git(&["rev-parse", "HEAD"]).unwrap(),
            sha,
            "the merge that met a conflict wasn't undone"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    /// What #5650's comparison printed on 19 November when the Mac couldn't
    /// name its caches' hosts, line for line, trimmed to one of each.
    const LOOKUPS: &str = "\
nix-offload: building on a-mac, /tmp/compare
\
warning: unable to download 'https://cache.nixos.org/6cr488dcf8v0b1rvcj0zwc6vbj9slx3k.narinfo': \
Could not resolve hostname (6) Could not resolve host: cache.nixos.org; retrying in 277 ms \
(attempt 1/5)
error: unable to download \
'https://cache.nixos.org/nar/0ipa1af092lbrh7nnh508k9r87k29wq1x2agpfd8kpwghdx0z2bh.nar.zst': \
Could not resolve hostname (6) Could not resolve host: cache.nixos.org
error: path '/nix/store/zjp7c0q83l4ab8fvz6vvhq2kcinanf71-rustc-1.99.1' is required, but \
there is no substituter that can build it
error: some references of path \
'/nix/store/iy1y0jds2l1nn22rks4va7crg8r835bn-clippy-1.99.1' could not be realised
error: some substitutes for the outputs of derivation \
'/nix/store/js8vczs3xzm3gkx4iysycvmhzlrlb5hj-clippy-1.99.1.drv' failed (usually happens due \
to networking issues); try '--fallback' to build derivation from source
\u{2705} path:/Users/someone/.cache/nix-offload/there/1840030#checks.aarch64-darwin.stands-alone
\u{274c} path:/Users/someone/.cache/nix-offload/there/1840030#checks.aarch64-darwin.\
system-judges-each-part
error: Cannot build '/nix/store/lvq948h202w1y515cz4g89fjgynya8d5-system-0.1.0.drv'.
       Reason: 3 dependencies failed.
       Output paths:
         /nix/store/zixb1jiaqlxc129m1lgdzx8mdl1w8my1-system-0.1.0
";

    #[test]
    fn a_mac_comparison_ended_before_a_builder_took_it_compared_nothing() {
        let ours = b"comparing aarch64-darwin's checks at 7103329c5f54";
        assert_eq!(
            mac_ending(None, Some(ours)),
            Some(UNCOMPARED),
            "a comparison ended before any builder took it was read as a stall"
        );
    }

    #[test]
    fn a_mac_comparison_stopped_once_a_builder_took_it_is_a_stall() {
        assert_eq!(
            mac_ending(None, Some(b"nix-offload: building on a-mac")),
            None,
            "a comparison a builder took and that was stopped wasn't read as a stall"
        );
        assert_eq!(
            mac_ending(None, None),
            None,
            "a stopped comparison whose log couldn't be read wasn't left a stall"
        );
    }

    #[test]
    fn a_mac_comparison_failing_on_host_lookups_alone_compared_nothing() {
        assert_eq!(
            mac_ending(Some(1), Some(LOOKUPS.as_bytes())),
            Some(UNCOMPARED),
            "a comparison that failed only because its builder couldn't name a cache counted"
        );
        let built_here = format!("{LOOKUPS}{CHANGED_FAILED}\n");
        assert_eq!(
            mac_ending(Some(1), Some(built_here.as_bytes())),
            Some(UNCOMPARED),
            "changed-checks' own word that a check failed, said after any failed build, counted"
        );
    }

    #[test]
    fn a_cache_that_timed_out_refused_or_was_down_compared_nothing_but_a_404_fails() {
        let resolved = "Could not resolve hostname (6) Could not resolve host: cache.nixos.org\n";
        // Only the error line ends on the host; the warning retries.
        let error = LOOKUPS
            .find(resolved)
            .expect("the error line's lookup words are in the fixture");
        let failed_with = |why: &str| {
            format!(
                "{}{why}\n{}",
                &LOOKUPS[..error],
                &LOOKUPS[error + resolved.len()..]
            )
        };
        // One of curl's words each, so a word dropped from the list fails
        // its own case.
        let each = [
            "Could not resolve host: cache.nixos.org",
            "(28) Resolving timed out after 5000 milliseconds",
            "Timeout was reached (28)",
            "Failed to connect to cache.nixos.org port 443 after 3 ms",
            "Couldn't connect to server (7)",
            "Recv failure: Connection reset by peer (56)",
            "SSL connect error (35)",
            "OpenSSL SSL_connect: SSL_ERROR_SYSCALL in connection to cache.nixos.org:443",
            "HTTP error 500 (curl error: Internal Server Error)",
            "HTTP error 502 (curl error: Bad Gateway)",
            "HTTP error 503 (curl error: Service Unavailable)",
            "HTTP error 504 (curl error: Gateway Timeout)",
        ];
        for word in NETWORK_WORDS {
            assert_eq!(
                each.iter().filter(|why| why.contains(word)).count(),
                1,
                "no case, or more than one, holds the network's word {word}"
            );
        }
        for why in each {
            assert_eq!(
                mac_ending(Some(1), Some(failed_with(why).as_bytes())),
                Some(UNCOMPARED),
                "a comparison that failed only on a cache it couldn't reach counted: {why}"
            );
        }
        let one = LOOKUPS.replace("3 dependencies failed.", "1 dependency failed.");
        assert_eq!(
            mac_ending(Some(1), Some(one.as_bytes())),
            Some(UNCOMPARED),
            "a build whose one dependency failed on the network alone counted"
        );
        let missing = failed_with("HTTP error 404 (curl error: Not Found)");
        assert_eq!(
            mac_ending(Some(1), Some(missing.as_bytes())),
            Some(1),
            "a download missing from where the change named was let off as the network"
        );
        // A fetch the change names at evaluation, from a host that doesn't
        // resolve, fails with the same words as the caches did.
        let fetched = "error: unable to download 'https://example.invalid/source.tar.gz': \
                       Could not resolve hostname (6) Could not resolve host: example.invalid\n";
        assert_eq!(
            mac_ending(Some(1), Some(fetched.as_bytes())),
            Some(1),
            "a fetch the change names, from a host that doesn't resolve, was let off"
        );
        // A cache's narinfo, as well as its nar, is the cache's.
        let narinfo =
            format!("error: unable to download 'https://cache.nixos.org/0000.narinfo': {resolved}");
        assert_eq!(
            mac_ending(Some(1), Some(narinfo.as_bytes())),
            Some(UNCOMPARED),
            "a cache's narinfo it couldn't reach counted"
        );
    }

    #[test]
    fn a_mac_comparison_that_names_no_unresolved_host_still_fails() {
        let words = "error: Cannot build '/nix/store/x-check.drv'.\n       \
                     Reason: 2 dependencies failed.\n";
        assert_eq!(
            mac_ending(Some(1), Some(words.as_bytes())),
            Some(1),
            "a failure whose every line could follow a lookup's, but with no lookup, was let off"
        );
    }

    #[test]
    fn a_mac_comparison_that_failed_beside_lookups_still_fails() {
        let built = format!(
            "{LOOKUPS}error: Cannot build '/nix/store/x-check.drv'.\n       \
             Reason: builder failed with exit code 1.\n"
        );
        let evaluated = format!("{LOOKUPS}error: attribute 'example-darwin' missing\n");
        let bare = format!("{LOOKUPS}error:\n       \u{2026} while evaluating\n");
        let told = format!("{LOOKUPS}\u{2715} a check the change touched no longer evaluates\n");
        for (words, what) in [
            (built, "a build that failed"),
            (evaluated, "an evaluation that failed"),
            (bare, "an evaluation whose error is on the lines after"),
            (told, "a check that no longer evaluates"),
        ] {
            assert_eq!(
                mac_ending(Some(1), Some(words.as_bytes())),
                Some(1),
                "{what} was let off because its builder also couldn't name a cache"
            );
        }
    }

    #[test]
    fn a_mac_comparison_that_passed_or_was_unsent_stays_so() {
        assert_eq!(
            mac_ending(Some(0), Some(LOOKUPS.as_bytes())),
            Some(0),
            "a pass was turned uncompared"
        );
        assert_eq!(
            mac_ending(Some(UNREACHED), Some(b"")),
            Some(UNSENT),
            "an unreached Mac wasn't read as unsent"
        );
        for code in [UNSENT, UNREACHED] {
            assert_eq!(
                mac_ending(Some(code), Some(LOOKUPS.as_bytes())),
                Some(UNSENT),
                "a run no Mac took waited a round, rather than being sent again"
            );
        }
        assert_eq!(
            mac_ending(Some(1), Some(b"Reason: builder failed with exit code 1.")),
            Some(1),
            "a plain failure was changed"
        );
        assert_eq!(
            mac_ending(
                Some(1),
                Some(b"error: unable to download 'x': HTTP error 404")
            ),
            Some(1),
            "a failure naming no unresolved host was let off"
        );
    }

    #[test]
    fn a_mac_comparisons_ending_is_read_from_its_own_part_of_the_log() {
        let log = std::env::temp_dir().join(format!("merge-net-ours-{}.log", std::process::id()));
        fs::write(&log, "nix-offload: building on a-mac, /tmp/compare\n").unwrap();
        let clock = Clock::new(&log, Instant::now());
        let mut file = fs::OpenOptions::new().append(true).open(&log).unwrap();
        writeln!(file, "comparing aarch64-darwin's checks at 7103329c5f54").unwrap();
        assert_eq!(
            mac_ended(None, &clock, &log),
            Some(UNCOMPARED),
            "a run ended before a builder took it was read as taken from the run before"
        );
        assert!(
            fs::read_to_string(&log)
                .unwrap()
                .ends_with("this compared nothing: it was ended before any Mac took it\n"),
            "the log net.log points at didn't say why the run compared nothing"
        );
        fs::remove_file(&log).unwrap();
    }
}
