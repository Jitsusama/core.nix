//! `nix-offload there --changed [ARGUMENT...]`: runs changed-checks for
//! the git tree it's started in on another machine, with the same
//! arguments, so neither of its evaluations runs here.
//!
//! Each evaluation of a change's checks held about 4.8 GB and several
//! cores, and every session changing core ran its own beside the person's
//! work: on 30 October they were a steady part of a load of 92 to 96. The
//! tree goes as `there` sends one, its commit by name when GitHub has it,
//! or the commits since as a bundle of a few kilobytes. The far side keeps
//! a mirror of the repository, checks the commit out beside origin's main,
//! and runs that commit's own changed-checks, so what it names is what the
//! tree's dev shell would have named. A run kept to the lent builders
//! runs this machine's instead ([`OWN`]), sent up ahead of it, since a
//! fenced session's commit is the session's to write.
//!
//! Where: a lent builder while one stands, which evaluates and builds in
//! one place, the one that keeps the run's base evaluated tried first,
//! else every other run trying the second first, and asking for it
//! while it lives, and while a lender runs, a builder waited for once
//! neither answers.
//! With no lender running, the first evaluator that answers and takes the
//! repository's owner, named in `nix-offload/evaluators` under XDG's
//! config folder, which only
//! evaluates, and the checks it names are built by `there` as any build
//! is. Each takes one run at a time, the rest waiting their [`turn`],
//! and one that names nothing within [`EVALUATING_LONGEST`] is left for
//! the next. With none, it exits [`UNBUILT`] while a lender runs, so nothing is
//! evaluated here unasked, and otherwise [`NOT_TAKEN`], and changed-checks
//! runs here.
//!
//! A run that passed at a commit is kept with what it named
//! (`passed.rs`), since a builder lent since evaluated both sides again,
//! 400 to 900 s, to build nothing new. The same run again there says what
//! it named and goes nowhere; another is told, as `CHANGED_CHECKS_PASSED`,
//! which checks passed at the commit, so the far side builds none again.

use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use crate::there::{self, Sending, Source};

/// What `there --changed` exits with when nothing elsewhere took the run,
/// so changed-checks runs it here: sysexits.h's temporary failure, which
/// no changed-checks run gives.
pub const NOT_TAKEN: u8 = 75;

/// What it exits with when an evaluator named the checks but no builder
/// took their build: sysexits.h's unavailable service, apart from
/// [`NOT_TAKEN`], so the run isn't done here instead.
pub const UNBUILT: u8 = 69;

/// How long an evaluator has to name the checks before it's left, unless
/// `NIX_OFFLOAD_EVALUATING_LONGEST` says how many seconds. On 8 November
/// six runs sat on one evaluator for up to three hours, having printed
/// nothing since connecting, while its own Nix said its database was busy
/// with the others; a run there evaluated both sides in 400 to 900 s.
pub const EVALUATING_LONGEST: Duration = Duration::from_mins(20);

/// How long an evaluator has, from the environment or
/// [`EVALUATING_LONGEST`].
pub fn evaluating_longest() -> Duration {
    std::env::var("NIX_OFFLOAD_EVALUATING_LONGEST")
        .ok()
        .and_then(|seconds| seconds.parse().ok())
        .map_or(EVALUATING_LONGEST, Duration::from_secs)
}

/// What the far side's changed-checks is told, so it runs there and never
/// sends itself on, and so the far side's slices treat it as other work.
const HERE: &str = "CHANGED_CHECKS_HERE=1 WORK_RUNS_ELSEWHERE=1";

/// The changed-checks this nix-offload was built beside, by the program's
/// store path, as `package.nix` names it, which a run kept to the lent
/// builders runs rather than the commit's own. A fenced session writes
/// its commits, and the commit's own changed-checks would run whatever
/// the session put there, on the builder and as the person, and print
/// what the record of passing keeps (12 November).
pub const OWN: Option<&str> = option_env!("NIX_OFFLOAD_CHANGED_CHECKS");

/// The changed-checks a run runs by its store path rather than the
/// commit's own: none, unless the run is kept to the lent builders
/// (`lent_only`), which runs [`OWN`] and no other, so it's refused
/// without one.
pub fn run_by(lent_only: bool) -> Result<Option<&'static str>, String> {
    match (lent_only, OWN) {
        (false, _) => Ok(None),
        (true, Some(own)) => Ok(Some(own)),
        (true, None) => {
            Err("this nix-offload was built without a changed-checks of its own".to_owned())
        }
    }
}

/// The store path holding the program `own`, which goes up to the builder
/// ahead of a run that names it: `/nix/store/HASH-NAME` of
/// `/nix/store/HASH-NAME/bin/changed-checks`.
pub fn package_of(own: &str) -> String {
    own.splitn(5, '/').take(4).collect::<Vec<_>>().join("/")
}

/// A machine that evaluates, from a line of `nix-offload/evaluators`:
/// `ADDRESS HOST OWNER,OWNER`, where the address is what a quick
/// connection is tried at, `host:port`, the host is what ssh is given, and
/// the owners are those on GitHub whose repositories it takes. A machine
/// may be one only some work may reach, so it's given only what it names.
#[derive(Debug, PartialEq, Eq)]
pub struct Evaluator {
    pub address: String,
    pub host: String,
    pub owners: Vec<String>,
}

impl Evaluator {
    /// Whether it takes the repository `github` names, `owner/name`.
    pub fn takes(&self, github: &str) -> bool {
        github
            .split('/')
            .next()
            .is_some_and(|owner| self.owners.iter().any(|taken| taken == owner))
    }
}

/// The evaluators in a file of them, a line each. Blank lines and those
/// starting `#` say nothing, and a line short of a word names none.
pub fn evaluators(text: &str) -> Vec<Evaluator> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            Some(Evaluator {
                address: words.next()?.to_owned(),
                host: words.next()?.to_owned(),
                owners: words.next()?.split(',').map(str::to_owned).collect(),
            })
        })
        .collect()
}

/// The command the far side's shell is given to run changed-checks with
/// `arguments` on the commit `source` names, and the bundle it reads from
/// its standard input, when the commit isn't on GitHub. It's told which
/// checks, by name, `passed` at that commit already, as
/// `CHANGED_CHECKS_PASSED`, the commit and then each name, so it builds
/// none of them again; a changed-checks from before that builds them.
/// It's the commit's own changed-checks, unless `own` names a store path
/// to run instead ([`OWN`]).
pub fn sending(
    source: Source,
    arguments: &[String],
    passed: &[String],
    own: Option<&str>,
) -> Sending {
    let commit = match &source {
        Source::Pushed { commit, .. } | Source::Bundled { commit, .. } => commit.clone(),
    };
    let (checkout, bundle) = checked_out(source);
    let words: Vec<String> = arguments.iter().map(|word| there::quoted(word)).collect();
    let told = if passed.is_empty() {
        String::new()
    } else {
        let named = std::iter::once(commit.as_str())
            .chain(passed.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ");
        format!("CHANGED_CHECKS_PASSED={} ", there::quoted(&named))
    };
    // The builder's gate reads the mark and lets a run sent first past
    // the sessions' runs waiting for a turn at evaluating.
    let first = if there::first() {
        format!("{}={} ", there::FIRST.0, there::FIRST.1)
    } else {
        String::new()
    };
    Sending {
        command: format!(
            "{checkout}{told}{HERE} {first}{} {}",
            own.map_or_else(|| "nix run .#changed-checks --".to_owned(), there::quoted),
            words.join(" ")
        ),
        bundle,
    }
}

/// What the far side's shell runs to check out the commit `source` names,
/// in a folder of its own gone when the shell ends, and to go into the
/// flake's folder there, each command after it ending `; `, and the
/// bundle it reads from its standard input, when the commit isn't on
/// GitHub.
pub fn checked_out(source: Source) -> (String, Option<PathBuf>) {
    let (github, base, commit, dir, bundle) = match source {
        Source::Pushed {
            github,
            commit,
            dir,
        } => (github, commit.clone(), commit, dir, None),
        Source::Bundled {
            github,
            base,
            commit,
            dir,
            bundle,
            tip,
        } => (github, base, commit, dir, Some((bundle, tip))),
    };
    // Kept between runs, so each fetches only what's new; a name per
    // repository, as `owner-name`.
    let mirror = format!(
        "\"$HOME\"/.cache/nix-offload/changed/{}.git",
        github.replace('/', "-")
    );
    // A folder named for the far side's shell, gone when it ends. Named
    // for this process instead, a run sent again after its connection
    // dropped shared the folder with the first, still running there,
    // which removed it as it ended: "cannot get cwd" on 8 November.
    let into = "/tmp/nix-offload-changed-$$";
    let unbundled = bundle.as_ref().map_or_else(String::new, |(_, tip)| {
        format!(
            "cat > {into}/there.bundle; \
             git -C {into}/tree fetch -q {into}/there.bundle {tip}:refs/heads/there; "
        )
    });
    // Runs at once share the mirror, so each holds its lock from making it
    // to cloning from it: two fetches moving main at once failed one with
    // "cannot lock ref 'refs/heads/main'" on 11 November. A far side with
    // no flock, as macOS has none, goes on without the lock, as before.
    let command = format!(
        "set -e; rm -rf {into}; mkdir -p {into}; trap 'rm -rf {into}' EXIT; \
         command -v git >/dev/null || \
         git() {{ nix shell nixpkgs#git -c git \"$@\"; }}; \
         command -v flock >/dev/null || flock() {{ :; }}; \
         mkdir -p \"$HOME\"/.cache/nix-offload/changed; \
         {{ flock 9; \
         [ -d {mirror} ] || git init -q --bare {mirror}; \
         git -C {mirror} fetch -q https://github.com/{github} \
         +refs/heads/main:refs/heads/main {base}; \
         git -C {mirror} symbolic-ref HEAD refs/heads/main; \
         git clone -q --shared {mirror} {into}/tree; \
         }} 9> {mirror}.lock; \
         {unbundled}\
         git -C {into}/tree checkout -q -B nix-offload-changed {commit}; \
         cd {into}/tree/{}; ",
        dir.as_deref().unwrap_or("."),
    );
    (command, bundle.map(|(bundle, _)| bundle))
}

/// What a run there asked for, read from changed-checks' own arguments:
/// whether it builds, whether VM tests are among what it builds, and the
/// system it was told.
#[derive(Debug, PartialEq, Eq)]
pub struct Wanted {
    pub build: bool,
    pub vm: bool,
    pub system: Option<String>,
}

pub fn wanted(arguments: &[String]) -> Wanted {
    let (mut whole, mut no_vm, mut name_only, mut system) = (false, false, false, None);
    let mut words = arguments.iter();
    while let Some(word) = words.next() {
        match word.as_str() {
            "--build" => whole = true,
            "--no-vm" => no_vm = true,
            "--name-only" => name_only = true,
            "--system" => system = words.next().cloned(),
            _ => {}
        }
    }
    // As changed-checks reads them: it builds what it names unless only
    // asked to name it, and VM tests only with --build and no --no-vm.
    Wanted {
        build: !name_only,
        vm: whole && !no_vm,
        system,
    }
}

/// The checks to build from what changed-checks printed, a kind and a name
/// a line: those added and changed, VM tests only when wanted, as
/// changed-checks itself would build them.
pub fn to_build(printed: &str, vm: bool) -> Vec<String> {
    printed
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let kind = words.next()?;
            let name = words.next()?;
            (matches!(kind, "added" | "changed") && (vm || !name.starts_with("vm-")))
                .then(|| name.to_owned())
        })
        .collect()
}

/// Where the net keeps the tests it holds in quarantine, as changed-checks'
/// own `quarantine.rs` reads it: a test that failed and then passed on the
/// same input, whose failure says nothing about a change.
pub const QUARANTINE: &str = "tools/merge-net/quarantine.json";

/// A test main holds in quarantine, and why its failure blocks nothing.
#[derive(Debug, PartialEq, Eq)]
pub struct Held {
    pub test: String,
    pub why: String,
}

/// The quarantine list as origin's main holds it in the tree at `folder`,
/// read from main rather than the tree, so a branch can't quarantine its
/// own failure. A tree without one quarantines nothing, and so does a list
/// that doesn't read, said so, so every failure still blocks.
pub fn held_on_main(folder: &Path) -> Vec<Held> {
    let shown = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args(["show", &format!("origin/main:{QUARANTINE}")])
        .stderr(Stdio::null())
        .output();
    let Some(shown) = shown.ok().filter(|shown| shown.status.success()) else {
        return Vec::new();
    };
    held(&String::from_utf8_lossy(&shown.stdout)).unwrap_or_else(|why| {
        eprintln!(
            "nix-offload: {QUARANTINE} on main doesn't read, so nothing is quarantined: {why}"
        );
        Vec::new()
    })
}

/// Read a quarantine list, each entry's test with why it blocks nothing,
/// said as changed-checks says it.
pub fn held(listed: &str) -> Result<Vec<Held>, String> {
    let list: serde_json::Value =
        serde_json::from_str(listed).map_err(|error| error.to_string())?;
    let entries = list.as_array().ok_or("it isn't a list")?;
    entries
        .iter()
        .map(|entry| {
            let test = entry["test"]
                .as_str()
                .ok_or_else(|| format!("an entry names no test: {entry}"))?;
            let reason = entry["reason"].as_str().unwrap_or("no reason given");
            let owner = entry["owner_pr"]
                .as_u64()
                .map_or_else(String::new, |pr| format!("; #{pr} owns it"));
            Ok(Held {
                test: test.to_owned(),
                why: format!("main quarantines it ({reason}{owner})"),
            })
        })
        .collect()
}

/// `targets` parted into those whose failure blocks and those main holds,
/// each with why. A test's part, such as its gallery, `vm-a.gallery`, is
/// held with its test, since it fails as its test does.
pub fn apart(targets: Vec<String>, held: &[Held]) -> (Vec<String>, Vec<(String, String)>) {
    let mut blocking = Vec::new();
    let mut alone = Vec::new();
    for target in targets {
        let test = target.split('.').next().unwrap_or(&target);
        match held.iter().find(|held| held.test == test) {
            Some(held) => alone.push((target, held.why.clone())),
            None => blocking.push(target),
        }
    }
    (blocking, alone)
}

/// Build each held test alone with `build`, after the rest, so a failure is
/// known for the test it was, and say of each that failed that it blocks
/// nothing: changed-checks' own run does the same, and without it every
/// run sent to an evaluator failed on a test main holds, and left no
/// record of passing for the gate (9 November).
pub fn built_alone(held: &[(String, String)], build: &mut dyn FnMut(&[String]) -> i32) {
    for (name, why) in held {
        if build(std::slice::from_ref(name)) != 0 {
            eprintln!("nix-offload: {name} failed, but {why}, so it blocks nothing");
        }
    }
}

/// The two lent builders in the order run `run` tries them: every other
/// run tries the second first. Each evaluation waits on its builder for a
/// turn, and on 8 November 33 runs waited on the first for its 4 while the
/// second stood idle, so the first's queue was hours long.
pub fn in_turn<T>(lenders: [T; 2], run: u32) -> [T; 2] {
    let [first, second] = lenders;
    if run.is_multiple_of(2) {
        [first, second]
    } else {
        [second, first]
    }
}

/// The two lent builders with the one that `kept` says holds the run's
/// base evaluated first, else as they came. keep-main keeps each head of
/// main on one builder, and on 12 November 23 of 54 runs evaluated their
/// base again, 119 minutes in all, on the other: one at 10:55 paid 230 s
/// for a head kept at 10:51.
pub fn kept_first<T>(lenders: [T; 2], kept: [bool; 2]) -> [T; 2] {
    let [first, second] = lenders;
    if !kept[0] && kept[1] {
        [second, first]
    } else {
        [first, second]
    }
}

/// What a run with `arguments` compares with: the `--base` given, else
/// origin's main.
pub fn theirs(arguments: &[String]) -> &str {
    arguments
        .iter()
        .position(|word| word == "--base")
        .and_then(|at| arguments.get(at + 1))
        .map_or("origin/main", String::as_str)
}

/// The commit a run with `arguments` at `commit` compares with, as the
/// far side's changed-checks finds it: its merge base with the `--base`
/// given, else with origin's main, here in `folder`, or none when git
/// can't say.
pub fn base_of(arguments: &[String], folder: &Path, commit: &str) -> Option<String> {
    let theirs = theirs(arguments);
    // A fenced session's tree may name programs for git to start, as
    // there.rs's git says, and finding a merge base needs none.
    let found = Command::new("git")
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "core.fsmonitor=false",
        ])
        .arg("-C")
        .arg(folder)
        .args(["merge-base", commit, theirs])
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|found| found.status.success())?;
    Some(String::from_utf8_lossy(&found.stdout).trim().to_owned())
}

/// What a lent builder is asked to learn whether its changed-checks keeps
/// `base`'s checks for `system` evaluated, ending 0 when it does: the file
/// changed-checks writes, by rename once whole, under XDG's cache folder.
/// None for a base or system the shell could read as more than a word.
pub fn keeps_base(base: &str, system: &str) -> Option<String> {
    let plain =
        |word: &str, allowed: fn(char) -> bool| !word.is_empty() && word.chars().all(allowed);
    let named = plain(base, |c| c.is_ascii_hexdigit())
        && plain(system, |c| {
            c.is_ascii_alphanumeric() || c == '_' || c == '-'
        });
    named.then(|| {
        let cache = "${XDG_CACHE_HOME:-$HOME/.cache}";
        format!("test -e \"{cache}/changed-checks/{base}-{system}.json\"")
    })
}

/// What the lent builder that ran a run is asked to print the checks it
/// keeps evaluated at `base` for `system` by, and what the other is asked
/// to take them in by, from its standard input. keep-main keeps each head
/// on one builder, and a run's evaluated base stays on the builder that
/// ran it, so from 11:00 to 15:00 on 12 November 78 runs evaluated a base
/// of only 46: 32 were the other builder evaluating, at about 280 s each,
/// what one had kept. Kept derivations name the same paths on either. The
/// file is only ever whole, since changed-checks renames it into place,
/// and it's taken in the same way, under a name of the writer's own so it
/// never meets changed-checks' writing or another gate's copy of the same
/// base, removed when it fails, and left alone where one is kept.
/// `record` is one of [`KEPT_AS`]. None for a base or system the shell
/// could read as more than a word, or a record changed-checks keeps none
/// of.
pub fn sharing_kept(base: &str, system: &str, record: &str) -> Option<(String, String)> {
    keeps_base(base, system)?;
    if !KEPT_AS.contains(&record) {
        return None;
    }
    let folder = "${XDG_CACHE_HOME:-$HOME/.cache}/changed-checks";
    let kept = format!("{folder}/{base}-{system}");
    let read = format!("cat \"{kept}.{record}\"");
    let take = format!(
        "test -e \"{kept}.{record}\" || {{ mkdir -p \"{folder}\" && \
         t=\"{kept}.{record}.copying.$$\" && {{ cat > \"$t\" && test -s \"$t\" && \
         mv \"$t\" \"{kept}.{record}\" || {{ rm -f \"$t\"; false; }}; }}; }}"
    );
    Some((read, take))
}

/// What changed-checks keeps of an evaluated base, each file named after
/// it and its system: the checks' derivations, then what its VM tests
/// read, which a gate uses only beside the derivations. A record a
/// builder hasn't kept, as when tracing failed, isn't copied.
pub const KEPT_AS: [&str; 2] = ["json", "reads.json"];

/// Waits for `host`'s turn to evaluate, had while what this gives is
/// held, so an evaluator takes one run at a time and the rest wait here:
/// on 8 November six runs at once sat on one for up to three hours, its
/// Nix saying its database was busy. The turn is kept here rather than on
/// the evaluator, so its clock counts only the evaluating, never the wait.
/// A turn that can't be waited for is said, and the run goes on without.
pub fn turn(scratch: &Path, host: &str) -> Option<fs::File> {
    waited_turn(scratch, host)
        .inspect_err(|error| eprintln!("nix-offload: couldn't wait for {host}'s turn: {error}"))
        .ok()
}

fn waited_turn(scratch: &Path, host: &str) -> io::Result<fs::File> {
    fs::create_dir_all(scratch)?;
    let held = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(scratch.join(format!("evaluating-{host}")))?;
    let locked = |how| {
        // SAFETY: flock only locks the file this holds open.
        if unsafe { libc::flock(held.as_raw_fd(), how) } == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    };
    match locked(libc::LOCK_EX | libc::LOCK_NB) {
        Err(error) if error.raw_os_error() == Some(libc::EWOULDBLOCK) => {
            eprintln!("nix-offload: {host} evaluates another run, so this waits its turn");
            locked(libc::LOCK_EX)?;
        }
        locked => locked?,
    }
    Ok(held)
}

/// Runs `command` on `host` by ssh, sending `bundle` on its standard
/// input, and gives its status and what it printed, which comes here as
/// it goes. Its log goes to this one's standard error. One that hasn't
/// ended within `longest` is ended, and given as unreached, so the next
/// evaluator is asked.
pub fn evaluated(
    ssh: &str,
    host: &str,
    sending: &Sending,
    scratch: &Path,
    longest: Duration,
) -> Result<(i32, String), String> {
    fs::create_dir_all(scratch).map_err(|error| format!("{}: {error}", scratch.display()))?;
    let said = there::mine(scratch, "evaluator-ssh.log");
    let mut command = there::reaching(ssh, &said);
    command.arg(host).arg(there::far(&sending.command));
    command.stdout(Stdio::piped());
    // The far side ends once its ssh does, but an ssh whose caller was
    // ended went on alone: on 8 November two sat under the account's
    // manager for over an hour after theirs were gone.
    // SAFETY: prctl is async-signal-safe and touches only the child.
    unsafe {
        command.pre_exec(|| {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) == 0 {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            }
        });
    }
    if let Some(bundle) = &sending.bundle {
        let bundle = fs::File::open(bundle).map_err(|error| format!("the bundle: {error}"))?;
        command.stdin(bundle);
    }
    let mut running = command
        .spawn()
        .map_err(|error| format!("couldn't run {ssh}: {error}"))?;
    let clock = left_after(running.id(), longest);
    let mut printed = String::new();
    if let Some(output) = running.stdout.take() {
        let mut out = std::io::stdout();
        for line in BufReader::new(output).lines().map_while(Result::ok) {
            let _ = writeln!(out, "{line}");
            printed.push_str(&line);
            printed.push('\n');
        }
    }
    // Told before the ssh is waited for, so it's never ended once its
    // number may belong to another.
    let left = clock.stop();
    let status = running
        .wait()
        .map_err(|error| format!("couldn't run {ssh}: {error}"))?;
    let _ = fs::remove_file(&said);
    if left {
        let within = match longest.as_secs() {
            seconds if seconds >= 60 => format!("{} minutes", seconds.div_ceil(60)),
            seconds => format!("{seconds} seconds"),
        };
        eprintln!("nix-offload: {host} named nothing within {within}, so it's left");
        return Ok((there::UNREACHED, printed));
    }
    Ok((status.code().unwrap_or(there::UNREACHED), printed))
}

/// A clock that ends a process unless it's stopped first.
struct Clock {
    stopping: mpsc::Sender<()>,
    ended: thread::JoinHandle<bool>,
}

impl Clock {
    /// Stops it, and says whether it had ended the process already.
    fn stop(self) -> bool {
        let _ = self.stopping.send(());
        self.ended.join().unwrap_or(false)
    }
}

/// Ends process `pid` once `longest` passes, unless stopped before.
fn left_after(pid: u32, longest: Duration) -> Clock {
    let (stopping, stopped) = mpsc::channel();
    let ended = thread::spawn(move || {
        let late = matches!(
            stopped.recv_timeout(longest),
            Err(RecvTimeoutError::Timeout)
        );
        if late && let Ok(pid) = libc::pid_t::try_from(pid) {
            // SAFETY: the process is this one's child, not yet waited for.
            unsafe { libc::kill(pid, libc::SIGTERM) };
        }
        late
    });
    Clock { stopping, ended }
}

/// The command that builds `checks` of the tree `folder` for `system`, as
/// `there` builds anything: this same program, so it waits for a builder,
/// sends the tree and journals as any `there` does.
pub fn building(folder: &Path, system: &str, checks: &[String]) -> Command {
    let mut command = Command::new("/proc/self/exe");
    command
        .args(["there", "-L", "--keep-going"])
        .arg(folder)
        .args(
            checks
                .iter()
                .map(|check| format!("checks.{system}.{check}")),
        );
    command
}

pub mod runs;

#[cfg(test)]
mod tests;
