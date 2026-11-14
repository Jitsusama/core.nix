//! Whether a builder still builds, proven with a tiny build of its own
//! every minute, so one that has stopped answering is taken out of every
//! builder list within two. On 8 November a switch hung 20 minutes on 37
//! pieces sent to a lent builder that had run out of memory: its line
//! stood, its tunnel took connections, and nothing noticed.
//!
//! A proof builds a derivation no store holds, its builder `/bin/sh`
//! writing a fresh nonce to its output, so nothing caches it and only a
//! builder that evaluates, builds and answers passes. A lent builder is
//! proven over the ssh its `there` builds take, with its store's key and
//! host key, and then carries a fresh 256 KiB up over its store as a
//! copy goes, so a way to it that can't carry a copy fails it too; a
//! builder beside, reached only by root's ssh, through the
//! daemon's build hook, as every build sent to it is, passing once its
//! output comes back. The daemon takes a builder line only from an account
//! it trusts, so where root's prover runs (`src/standing.rs`) it proves
//! those, and a lender reads what it says from [`STANDING`].
//!
//! A proof also reads how many bytes its builder's store has free: over
//! ssh with a df once it has built, and through the daemon with this
//! program's own df, taken along into the proof's sandbox, whose shell
//! has none, its answer coming back in the build's log. A builder that
//! passes with less than `FLOOR` free is full, and takes no builds until
//! it has room: a builder beside is left out of every list, a VM builder
//! or second lent builder never stands in, and the one lent hands any
//! work to one with room, staying named for big work, since it collects
//! at its own min-free as it builds and isn't given back for it.
//!
//! Each lender says how its builder stands in its folder's `health.json`,
//! written every tick, for a status card to read:
//!
//! ```json
//! {"at": 1793725313, "builder": "lend-1793725000", "state": "passing",
//!  "lastSuccess": 1793725300, "lastFailure": null, "consecutiveFailures": 0,
//!  "freeBytes": 81604378624,
//!  "beside": [{"builder": "work-linux", "state": "full",
//!              "lastSuccess": 1793725290, "lastFailure": 1793720000,
//!              "consecutiveFailures": 0, "freeBytes": 6001344512}]}
//! ```
//!
//! `at` is when it was written and every time is in seconds since the
//! epoch, null when there hasn't been one. `builder` is null while none is
//! lent. `state` is `none` while none is lent, `unproven` before its first
//! proof ends, `passing` once its last passed, `failing` after one failure,
//! and `dropped` after two in a row, which takes a lent builder out of
//! every list and gives it back, and leaves a builder beside out of every
//! list until it passes again; `full` once one passes with its store under
//! the floor. `freeBytes` is what its store had free when its last proof
//! passed, null where it didn't say. `stalled` is true while its last
//! proof built but couldn't carry what it was sent up, so the builder
//! builds and the way to it is what fails. `beside` is there only in a
//! folder whose lender names builders beside.
//!
//! Long because its tests, a third of it, stay beside the private pieces
//! they prove, the proof's clock and what it keeps of a builder's builds.

use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::there;

/// How often a builder is proven.
pub const EVERY: Duration = Duration::from_secs(60);

/// How long a proof may take, ssh and Nix's start included, before it
/// fails.
pub const WITHIN: Duration = Duration::from_secs(45);

/// How long ssh has to reach a builder, in seconds.
pub const CONNECTING: u32 = 10;

/// How many proofs in a row fail before a builder is left out.
pub const FAILS: u32 = 2;

/// How old a passing proof another lender wrote may be and still count:
/// three minutes is three proofs, so one whose lender stopped writing is
/// never taken for an answer.
pub const FRESH: u64 = 180;

/// The file a lender says how its builders stand in, in its folder.
pub const HEALTH: &str = "health.json";

/// Where root's prover says how each standing builder stands
/// (`src/standing.rs`), for a lender to read rather than prove them
/// itself, which only an account the daemon trusts can.
pub const STANDING: &str = "/run/nix-offload-standing/health.json";

/// The least a builder's store may have free and still take builds, in
/// bytes: one with less is left out until it has room again. On 9
/// November the work Mac's builder had 6 GB, and every check sent there
/// failed for room while its proofs passed.
pub const FLOOR: u64 = 15_000_000_000;

/// How a proof ended: whether it passed, the bytes its builder's store
/// had free, where it said, and whether it built but couldn't carry
/// [`CARRIED`] bytes up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Proven {
    pub passed: bool,
    pub free: Option<u64>,
    pub stalled: bool,
}

impl From<bool> for Proven {
    fn from(passed: bool) -> Self {
        Self {
            passed,
            ..Self::default()
        }
    }
}

/// How a builder stands, as its proofs say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// None is lent.
    None,
    /// Its first proof hasn't ended.
    Unproven,
    /// Its last proof passed.
    Passing,
    /// Its last proof failed, the one before it not.
    Failing,
    /// Its last `FAILS` proofs failed, so it's left out.
    Dropped,
    /// The last of its proofs that passed found less than `FLOOR` free in
    /// its store, and fewer than `FAILS` have failed since: a failure says
    /// nothing of room, so it's no reason to send it builds again.
    Full,
}

impl State {
    pub fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Unproven => "unproven",
            Self::Passing => "passing",
            Self::Failing => "failing",
            Self::Dropped => "dropped",
            Self::Full => "full",
        }
    }
}

/// What a builder's proofs have said.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Health {
    pub builder: String,
    pub last_success: Option<u64>,
    pub last_failure: Option<u64>,
    /// The proofs that failed since the last that passed.
    pub failures: u32,
    /// The bytes its store had free when its last proof passed, where it
    /// said.
    pub free: Option<u64>,
    /// Whether its last proof built but couldn't carry [`CARRIED`] bytes
    /// up: the builder builds, and the way to it is what fails.
    pub stalled: bool,
}

impl Health {
    pub fn of(builder: &str) -> Self {
        Self {
            builder: builder.to_owned(),
            ..Self::default()
        }
    }

    /// Take in a proof that ended `at`.
    pub fn heard(&mut self, proven: impl Into<Proven>, at: u64) {
        let Proven {
            passed,
            free,
            stalled,
        } = proven.into();
        self.stalled = stalled && !passed;
        if passed {
            self.free = free;
            self.last_success = Some(at);
            self.failures = 0;
        } else {
            self.last_failure = Some(at);
            self.failures += 1;
        }
    }

    pub fn state(&self) -> State {
        match (self.failures, self.last_success) {
            (failures, _) if failures >= FAILS => State::Dropped,
            _ if self.free.is_some_and(|free| free < FLOOR) => State::Full,
            (0, None) => State::Unproven,
            (0, Some(_)) => State::Passing,
            _ => State::Failing,
        }
    }

    /// Whether it's left out of every list, having failed `FAILS` proofs
    /// in a row.
    pub fn dropped(&self) -> bool {
        self.state() == State::Dropped
    }

    /// Whether it takes no builds: dropped, or full.
    pub fn left_out(&self) -> bool {
        matches!(self.state(), State::Dropped | State::Full)
    }

    /// Its health as [`Health::json`] wrote it, where it reads as one.
    pub fn from_json(said: &Value) -> Option<Self> {
        let at = |name: &str| said[name].as_u64();
        Some(Self {
            builder: said["builder"].as_str()?.to_owned(),
            last_success: at("lastSuccess"),
            last_failure: at("lastFailure"),
            failures: u32::try_from(said["consecutiveFailures"].as_u64()?).ok()?,
            free: at("freeBytes"),
            stalled: said["stalled"].as_bool().unwrap_or(false),
        })
    }

    pub fn json(&self) -> Value {
        json!({
            "builder": self.builder,
            "state": self.state().name(),
            "lastSuccess": self.last_success,
            "lastFailure": self.last_failure,
            "consecutiveFailures": self.failures,
            "freeBytes": self.free,
            "stalled": self.stalled,
        })
    }
}

/// Whether a proof is due, the last having begun at `began`.
pub fn due(began: Option<Instant>, now: Instant) -> bool {
    began.is_none_or(|began| now.saturating_duration_since(began) >= EVERY)
}

/// One builder's proofs, one at a time, each on a thread of its own so a
/// builder that hangs never holds the lender's tick.
#[derive(Default)]
pub struct Prover {
    began: Option<Instant>,
    running: Option<mpsc::Receiver<Proven>>,
}

impl Prover {
    /// What the proof running ended with, if it has ended, starting the
    /// next with `prove` once one is due and none runs.
    pub fn tick<P: Into<Proven>>(
        &mut self,
        now: Instant,
        prove: impl FnOnce() -> P + Send + 'static,
    ) -> Option<Proven> {
        let heard = match &self.running {
            Some(running) => match running.try_recv() {
                Ok(passed) => Some(passed),
                Err(mpsc::TryRecvError::Empty) => return None,
                // A proof that panicked proved nothing.
                Err(mpsc::TryRecvError::Disconnected) => Some(Proven::default()),
            },
            None => None,
        };
        self.running = None;
        if due(self.began, now) {
            let (said, running) = mpsc::channel();
            thread::spawn(move || {
                let _ = said.send(prove().into());
            });
            self.began = Some(now);
            self.running = Some(running);
        }
        heard
    }
}

/// A builder's health and its proofs.
pub struct Keeper {
    pub health: Health,
    prover: Prover,
}

impl Keeper {
    pub fn of(builder: &str) -> Self {
        Self::going_on(Health::of(builder))
    }

    /// A keeper that goes on from `earlier`, how its builder stood when
    /// the keeper before it last wrote, so one started again doesn't name
    /// a builder that was full or dropped before its own first proof ends.
    pub fn going_on(earlier: Health) -> Self {
        Self {
            health: earlier,
            prover: Prover::default(),
        }
    }

    /// Take in the proof that just ended, if one did, at `at`, and start
    /// the next with `prove` once one is due; what the one that ended said.
    pub fn keep(
        &mut self,
        now: Instant,
        at: u64,
        prove: impl FnOnce() -> Proven + Send + 'static,
    ) -> Option<bool> {
        let heard = self.prover.tick(now, prove)?;
        self.health.heard(heard, at);
        Some(heard.passed)
    }
}

/// A nonce no proof before has used.
fn nonce() -> String {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    format!("{since}-{}", std::process::id())
}

/// What starts the line a proof says its builder's free bytes on, after
/// it.
const FREE: &str = "nix-offload-free ";

/// What a proof over ssh asks once it has built: the free bytes of the
/// builder's store, on a line of standard error after [`FREE`].
const FREE_ASKED: &str =
    "echo \"nix-offload-free $(df -B1 --output=avail /nix/store | tail -n 1)\" >&2";

/// The free bytes a line of a proof's standard error says, if it says
/// them.
fn said_free(line: &str) -> Option<u64> {
    let (_, free) = line.split_once(FREE)?;
    free.trim().parse().ok()
}

/// The derivation a proof builds, as a Nix expression: `/bin/sh` writes
/// `nonce` to its output, so no store holds it. It's for `system`, or the
/// builder's own when none is given, and requires `features`, since a
/// builder whose line makes a feature mandatory takes only builds asking
/// for it. Given `coreutils`, it also prints its store's free bytes with
/// their df, which it takes along as an input: a sandbox's own shell has
/// none, and its store is the builder's own filesystem.
pub fn expression(
    nonce: &str,
    system: Option<&str>,
    features: &[&str],
    coreutils: Option<&str>,
) -> String {
    let string = |text: &str| format!("\"{}\"", text.replace(['"', '\\', '$'], ""));
    let system = system.map_or_else(|| "builtins.currentSystem".to_owned(), string);
    let features = features.iter().fold(String::new(), |mut all, feature| {
        all.push(' ');
        all.push_str(&string(feature));
        all
    });
    let measured = coreutils.map_or_else(String::new, |coreutils| {
        format!(
            "; set -- $(${{builtins.storePath {}}}/bin/df -B1 --output=avail /nix/store); \
             echo {FREE}$2",
            string(coreutils)
        )
    });
    format!(
        "derivation {{ name = \"nix-offload-proof\"; system = {system}; builder = \"/bin/sh\"; \
         args = [ \"-c\" \"echo {} > $out{measured}\" ]; requiredSystemFeatures = [{features} ]; }}",
        nonce.replace(
            |byte: char| !byte.is_ascii_alphanumeric() && byte != '-',
            ""
        ),
    )
}

/// Prove the builder whose store `store` names (its first line) over ssh,
/// as `there` reaches it: never asking, with a `CONNECTING` second connect
/// timeout, its store's key and host key, building a fresh derivation
/// there within `within`. `scratch` holds the host key and ssh's log while
/// it runs.
pub fn over_ssh(store: &str, scratch: &Path, ssh: &str, within: Duration) -> Proven {
    let Some(lent) = store
        .lines()
        .next()
        .and_then(|line| there::builder(line.trim()))
    else {
        return Proven::default();
    };
    let Ok((mut command, known, said)) = there::reach(&lent, scratch, ssh) else {
        return Proven::default();
    };
    let build = format!(
        "nix build --no-link --impure --option substitute false --expr {} && {} && {FREE_ASKED}",
        there::quoted(&expression(&nonce(), None, &[], None)),
        crate::progress::asked_said()
    );
    command
        .args(["-o", &format!("ConnectTimeout={CONNECTING}")])
        .arg(&lent.host)
        .arg(build);
    let kept = scratch.with_file_name(BUILDS);
    let passed = ended_within(&mut command, within, &lent.host, None, Some(&kept));
    for file in [&known, &said] {
        let _ = fs::remove_file(file);
    }
    passed
}

/// How long a builder that fails its proofs while builds still reach it
/// has to answer at all, its daemon saying which store it is: a live one
/// answers in well under a second, however busy.
pub const ANSWERING: Duration = Duration::from_secs(20);

/// Whether the builder whose store `store` names (its first line) answers
/// at all over ssh, as `there` reaches it, within `within`: its daemon says
/// which store it is, which waits for no build slot. A builder whose slots
/// are all taken builds no proof in time though it builds what reaches it,
/// so this tells a busy builder from a lost one. `scratch` holds its host
/// key and ssh's log while it runs.
pub fn answers_over_ssh(store: &str, scratch: &Path, ssh: &str, within: Duration) -> bool {
    let Some(lent) = store
        .lines()
        .next()
        .and_then(|line| there::builder(line.trim()))
    else {
        return false;
    };
    let Ok((mut command, known, said)) = there::reach(&lent, scratch, ssh) else {
        return false;
    };
    command
        .args(["-o", &format!("ConnectTimeout={CONNECTING}")])
        .arg(&lent.host)
        .arg("nix store info")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let answered = command
        .spawn()
        .ok()
        .and_then(|mut running| there::finished_within(&mut running, within))
        .is_some_and(|status| status.success());
    for file in [&known, &said] {
        let _ = fs::remove_file(file);
    }
    answered
}

/// How many bytes a lent builder's proof carries up, fresh each time, as
/// a copy goes: built proofs passed all through 13 November's evening
/// while copies up to both lent builders stalled, since a proof sent
/// next to nothing. At 6.6 Mbit/s they take about 0.3 s of the uplink a
/// minute, under 1% of it, for each builder lent.
pub const CARRIED: usize = 256 * 1024;

/// How long carrying [`CARRIED`] bytes up may take: under 20 s is more
/// than 105 kbit/s, and a way slower than that carries no copy worth
/// waiting on.
pub const CARRYING: Duration = Duration::from_secs(20);

/// A proof that `built`, passing only once its builder `carries` a copy
/// up too, and stalled where it built but couldn't. One that didn't build
/// isn't asked to carry.
pub fn carrying(built: Proven, carries: impl FnOnce() -> bool) -> Proven {
    if !built.passed || carries() {
        return built;
    }
    Proven {
        passed: false,
        stalled: true,
        ..built
    }
}

/// Whether the builder whose store `store` names (its first line) takes
/// [`CARRIED`] bytes no store holds within `within`, copied up with `nix`
/// over its store, as every copy up goes: by the relay, the tunnel and
/// ssh-ng to its daemon. They go as a file added here, named by its
/// content, so the builder takes it unsigned; no root holds it in either
/// store, so each collection takes it. `scratch` holds the file while
/// it's added.
pub fn carries(store: &str, scratch: &Path, nix: &str, within: Duration) -> bool {
    let Some(lent) = store
        .lines()
        .next()
        .and_then(|line| there::builder(line.trim()))
    else {
        return false;
    };
    let Some(path) = added(scratch, nix) else {
        eprintln!("nix-offload: couldn't add what a proof carries to this store");
        return false;
    };
    let carried = there::carried_up(&path, &lent, nix, within);
    if !carried {
        eprintln!(
            "nix-offload: {} built its proof but couldn't take {} KiB within {} s",
            lent.host,
            CARRIED / 1024,
            within.as_secs()
        );
    }
    carried
}

/// A fresh path of [`CARRIED`] random bytes in this store, added with
/// `nix` from a file in `scratch`, which goes once it's added.
fn added(scratch: &Path, nix: &str) -> Option<String> {
    use std::io::Read;
    fs::create_dir_all(scratch).ok()?;
    let file = scratch.join(format!("carried-{}", nonce()));
    let mut bytes = Vec::with_capacity(CARRIED);
    let read = fs::File::open("/dev/urandom")
        .and_then(|random| random.take(CARRIED as u64).read_to_end(&mut bytes));
    let added = read.ok().filter(|read| *read == CARRIED).and_then(|_| {
        fs::write(&file, &bytes).ok()?;
        let mut adding = Command::new(nix);
        adding
            .args([
                "store",
                "add",
                "--mode",
                "flat",
                "--name",
                "nix-offload-carried",
            ])
            .arg(&file)
            .stdin(Stdio::null())
            .stderr(Stdio::null());
        let out = adding.output().ok()?;
        let path = String::from_utf8(out.stdout).ok()?.trim().to_owned();
        (out.status.success() && path.starts_with('/')).then_some(path)
    });
    let _ = fs::remove_file(&file);
    added
}

/// What Nix says once a builder has built a proof and its output comes
/// back, which is as far as a proof through the daemon need go: this
/// store takes nothing while a collection holds its lock, as the disk
/// guard's `--print-dead` did for minutes at a time on 8 November, and
/// that's no fault of the builder's.
const BUILT_THERE: &str = "-nix-offload-proof' from '";

/// Prove the builder a machines line names through the daemon's build
/// hook, as every build Nix sends there goes, by root's ssh: nothing is
/// built here, and the line is given slots of its own, so a proof never
/// waits behind the builds filling its own. It passes once its output
/// comes back, waiting for no lock here.
pub fn through_daemon(line: &str, nix: &str, within: Duration) -> Proven {
    proven_through(line, nix, within, COREUTILS)
}

/// The coreutils this program was built with, whose df a proof through
/// the daemon measures its builder's store with.
const COREUTILS: Option<&str> = option_env!("NIX_OFFLOAD_COREUTILS");

/// The system this machine builds for, as Nix names it.
fn here() -> String {
    format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS)
}

/// [`through_daemon`], measuring the builder's store with `coreutils`' df
/// where its proof is for this machine's own system.
fn proven_through(line: &str, nix: &str, within: Duration, coreutils: Option<&str>) -> Proven {
    let fields: Vec<&str> = line.split_whitespace().collect();
    let Some(uri) = fields.first() else {
        return Proven::default();
    };
    let field = |at: usize| fields.get(at).copied().filter(|field| *field != "-");
    let system = field(1).and_then(|systems| systems.split(',').next());
    let mandatory: Vec<&str> = field(6)
        .map(|features| {
            features
                .split(',')
                .filter(|feature| !feature.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let separated = if uri.contains('?') { '&' } else { '?' };
    let slots = format!("{uri}{separated}max-connections=1");
    let mut own = fields.clone();
    own.resize(own.len().max(4), "-");
    own[0] = &slots;
    own[3] = "1";
    // This machine's df runs only on a builder of its own system.
    let coreutils = coreutils.filter(|_| system.is_none_or(|system| system == here()));
    let mut command = Command::new(nix);
    command
        .args(["build", "--no-link", "--impure", "--max-jobs", "0", "-L"])
        // Root's Nix may not turn the command on, as a NixOS machine's
        // doesn't unless told, and the proof would fail every builder.
        .args(["--extra-experimental-features", "nix-command"])
        .args(["--option", "substitute", "false", "--builders"])
        .arg(own.join(" "))
        .arg("--expr")
        .arg(expression(&nonce(), system, &mandatory, coreutils));
    ended_within(&mut command, within, &named(line), Some(BUILT_THERE), None)
}

/// The builder a machines line names, as a person reads it: its host.
pub fn named(line: &str) -> String {
    let uri = line.split_whitespace().next().unwrap_or_default();
    let rest = uri.split_once("://").map_or(uri, |(_, rest)| rest);
    let host = rest.split(['?', '/']).next().unwrap_or(rest);
    host.rsplit_once('@')
        .map_or(host, |(_, host)| host)
        .to_owned()
}

/// Whether `command` succeeds within `within`, or says `answered` on its
/// way there, killed with everything it started once either is past;
/// saying why the proof of `builder` failed.
fn ended_within(
    command: &mut Command,
    within: Duration,
    builder: &str,
    answered: Option<&str>,
    figures: Option<&Path>,
) -> Proven {
    let mut free = None;
    let passed = ended(command, within, builder, answered, figures, &mut free);
    Proven {
        passed,
        free,
        stalled: false,
    }
}

/// [`ended_within`], keeping the free bytes the proof said in `free`.
fn ended(
    command: &mut Command,
    within: Duration,
    builder: &str,
    answered: Option<&str>,
    figures: Option<&Path>,
    free: &mut Option<u64>,
) -> bool {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .process_group(0);
    let Ok(mut running) = command.spawn() else {
        eprintln!("nix-offload: couldn't start the proof of {builder}");
        return false;
    };
    let (said, heard) = mpsc::channel();
    if let Some(errors) = running.stderr.take() {
        thread::spawn(move || {
            for line in BufReader::new(errors).lines().map_while(Result::ok) {
                if said.send(line).is_err() {
                    break;
                }
            }
        });
    }
    let ends = |running: &mut std::process::Child| {
        if let Ok(pid) = libc::pid_t::try_from(running.id()) {
            // SAFETY: a signal to our own child's process group, which
            // it leads.
            unsafe { libc::kill(-pid, libc::SIGKILL) };
        }
        let _ = running.wait();
    };
    let began = Instant::now();
    let mut why = None;
    let status = loop {
        for line in heard.try_iter() {
            if let (Some(kept), Some(read)) = (figures, crate::progress::heard(&line)) {
                keep_builds(kept, read);
                continue;
            }
            if let Some(said) = said_free(&line) {
                *free = Some(said);
                continue;
            }
            if answered.is_some_and(|answered| line.contains(answered)) {
                ends(&mut running);
                return true;
            }
            if !line.trim().is_empty() {
                why = Some(line.trim().to_owned());
            }
        }
        match running.try_wait() {
            Ok(Some(status)) => break Some(status),
            Err(_) => break None,
            Ok(None) if began.elapsed() >= within => {
                ends(&mut running);
                break None;
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
        }
    };
    if status.is_some_and(|status| status.success()) {
        // Its figures may come after it ended, a line behind.
        let ending = Instant::now() + Duration::from_millis(200);
        while let Ok(line) = heard.recv_timeout(ending.saturating_duration_since(Instant::now())) {
            if let (Some(kept), Some(read)) = (figures, crate::progress::heard(&line)) {
                keep_builds(kept, read);
            }
            if let Some(said) = said_free(&line) {
                *free = Some(said);
            }
        }
        return true;
    }
    // What it said last, before it ended.
    let ending = Instant::now() + Duration::from_secs(1);
    while let Ok(line) = heard.recv_timeout(ending.saturating_duration_since(Instant::now())) {
        if !line.trim().is_empty() {
            why = Some(line.trim().to_owned());
        }
    }
    match (status, why) {
        (None, _) => eprintln!(
            "nix-offload: {builder} didn't build its proof within {} s",
            within.as_secs()
        ),
        (Some(_), Some(why)) => eprintln!("nix-offload: {builder} failed its proof: {why}"),
        (Some(status), None) => eprintln!("nix-offload: {builder} failed its proof ({status})"),
    }
    false
}

/// The file a lender keeps its builder's last figures in, beside its
/// proofs' folder, for `health.json` to say as `builds`.
pub const BUILDS: &str = "builds.json";

/// Keeps a builder's figures, `read` now, at `kept`.
fn keep_builds(kept: &Path, read: (u64, u64)) {
    let at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let writing = kept.with_extension("writing");
    let said = crate::progress::kept(read, at).to_string();
    if fs::write(&writing, said)
        .and_then(|()| fs::rename(&writing, kept))
        .is_err()
    {
        let _ = fs::remove_file(&writing);
    }
}

/// Write `dir`'s `health.json`, at `at`: the builder lent, if one is or
/// the last was left out, each builder beside, where `beside` gives them,
/// and the lent builder's last figures as `builds`, which say how far its
/// builds have got: on 9 November a switch stopped a quiet build there
/// that was still compiling.
pub fn write(dir: &Path, lent: Option<&Health>, beside: Option<&[Health]>, at: u64) {
    let mut said = lent.map_or_else(
        || {
            json!({
                "builder": null,
                "state": State::None.name(),
                "lastSuccess": null,
                "lastFailure": null,
                "consecutiveFailures": 0,
                "freeBytes": null,
                "stalled": false,
            })
        },
        Health::json,
    );
    said["at"] = at.into();
    let builds = fs::read_to_string(dir.join(BUILDS))
        .ok()
        .and_then(|kept| serde_json::from_str::<Value>(&kept).ok());
    if let (Some(builds), Some(_)) = (builds, lent) {
        said["builds"] = builds;
    }
    if let Some(beside) = beside {
        said["beside"] = beside.iter().map(Health::json).collect();
    }
    let path = dir.join(HEALTH);
    let writing = path.with_extension("writing");
    let written =
        fs::write(&writing, format!("{said}\n")).and_then(|()| fs::rename(&writing, &path));
    if let Err(error) = written {
        eprintln!("nix-offload: writing {}: {error}", path.display());
    }
}

/// How the standing builder `uri` names stands, as root's prover said in
/// `path` as of `now`: `None` with no prover there, so the lender proves
/// it itself as before; dropped once the prover has written nothing for
/// `FRESH`, since a stopped prover vouches for nothing; and unproven when
/// it names no such builder.
pub fn standing_health(path: &Path, uri: &str, named: &str, now: u64) -> Option<Health> {
    let text = fs::read_to_string(path).ok()?;
    let said: Value = serde_json::from_str(&text).unwrap_or_default();
    let mut health = Health::of(named);
    if said["at"].as_u64().is_none_or(|at| at + FRESH < now) {
        health.failures = FAILS;
        health.last_failure = Some(now);
        return Some(health);
    }
    let builders = said["builders"].as_array().map_or(&[][..], Vec::as_slice);
    Some(
        builders
            .iter()
            .find(|builder| builder["uri"] == uri)
            .and_then(Health::from_json)
            .unwrap_or(health),
    )
}

/// The builders the health file at `path` last said under `list`, as
/// each keeper wrote them, for keepers started again to go on from: on 11
/// November a full builder beside read as unproven after each restart,
/// and so took builds, until its first proof ended.
pub fn earlier(path: &Path, list: &str) -> Vec<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|said| said[list].as_array().cloned())
        .unwrap_or_default()
}

/// Whether the builder `dir`'s lender keeps passed its last proof, no
/// longer than `FRESH` before `now`.
pub fn passes_in(dir: &Path, now: u64) -> bool {
    let Ok(text) = fs::read_to_string(dir.join(HEALTH)) else {
        return false;
    };
    let said: Value = serde_json::from_str(&text).unwrap_or_default();
    said["state"] == State::Passing.name()
        && said["lastSuccess"]
            .as_u64()
            .is_some_and(|success| success + FRESH >= now)
}

/// Whether the process `pid` ends within 5 s: once gone, or a zombie no
/// one has reaped, as one orphaned in a build's sandbox can stay.
#[cfg(test)]
pub fn ends(pid: i32) -> bool {
    let ending = Instant::now() + Duration::from_secs(5);
    loop {
        // SAFETY: signal 0 only asks whether the process exists.
        let exists = unsafe { libc::kill(pid, 0) } == 0;
        let zombie = fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|stat| {
            stat.rsplit_once(')')
                .is_some_and(|(_, rest)| rest.trim_start().starts_with('Z'))
        });
        if !exists || zombie {
            return true;
        }
        if Instant::now() >= ending {
            return false;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_builder_is_dropped_after_two_failed_proofs_in_a_row_and_back_once_one_passes() {
        let mut health = Health::of("lend-5");
        assert_eq!(health.state(), State::Unproven);
        health.heard(true, 10);
        assert_eq!(health.state(), State::Passing);
        health.heard(false, 70);
        assert_eq!(health.state(), State::Failing);
        assert!(!health.dropped(), "one failure is a blip");
        health.heard(false, 130);
        assert!(health.dropped());
        assert_eq!(
            (health.last_success, health.last_failure, health.failures),
            (Some(10), Some(130), 2)
        );
        health.heard(true, 190);
        assert_eq!(health.state(), State::Passing);
        assert_eq!(health.failures, 0);
        let mut fresh = Health::of("lend-6");
        fresh.heard(false, 10);
        fresh.heard(false, 70);
        assert!(fresh.dropped(), "one never proven is dropped too");
    }

    // On 13 November copies up to both lent builders stalled for minutes
    // while every proof passed: a proof built a derivation there, but
    // carried next to nothing up.
    #[test]
    fn a_builder_that_builds_but_cant_carry_a_copy_up_fails_as_stalled() {
        let built = Proven {
            passed: true,
            free: Some(FLOOR),
            ..Proven::default()
        };
        let carried = carrying(built, || true);
        assert!(carried.passed && !carried.stalled, "{carried:?}");
        assert_eq!(carried.free, Some(FLOOR), "its room still counts");
        let stalled = carrying(built, || false);
        assert!(!stalled.passed && stalled.stalled, "{stalled:?}");
        let mut asked = false;
        let unbuilt = carrying(Proven::default(), || {
            asked = true;
            true
        });
        assert!(!unbuilt.passed && !unbuilt.stalled, "{unbuilt:?}");
        assert!(!asked, "one that didn't build isn't asked to carry");

        let mut health = Health::of("lend-7");
        health.heard(built, 10);
        health.heard(stalled, 70);
        health.heard(stalled, 130);
        assert!(health.dropped(), "a link that can't carry takes no builds");
        assert!(health.stalled, "since it built");
        assert_eq!(health.json()["stalled"], true);
        let read = Health::from_json(&health.json()).expect("its health");
        assert!(read.stalled, "a keeper started again reads it");
        health.heard(false, 190);
        assert!(!health.stalled, "one that didn't build isn't stalled");
        health.heard(stalled, 250);
        health.heard(built, 310);
        assert!(!health.stalled && !health.dropped(), "carried again");
    }

    #[test]
    fn a_lent_builder_carries_a_fresh_path_up_over_its_store_within_its_time() {
        let dir = tempfile::tempdir().unwrap();
        let at = dir.path().display();
        let nix = stand_in(
            dir.path(),
            "nix",
            &format!(
                "case \"$1 $2\" in \"store add\") for last; do :; done; \
                 wc -c < \"$last\" > {at}/size; cp \"$last\" {at}/carried; \
                 echo /nix/store/aaaa-nix-offload-carried; exit 0 ;; esac; \
                 echo \"$NIX_SSHOPTS\" > {at}/sshopts; exit 0"
            ),
        );
        let scratch = dir.path().join("proving");
        assert!(
            carries(STORE, &scratch, &nix, CARRYING),
            "a copy that lands carries"
        );
        let given = fs::read_to_string(dir.path().join("given")).unwrap();
        let words: Vec<&str> = given.lines().collect();
        assert_eq!(
            words,
            [
                "copy",
                "--to",
                STORE.trim(),
                "/nix/store/aaaa-nix-offload-carried"
            ]
        );
        let size = fs::read_to_string(dir.path().join("size")).unwrap();
        assert_eq!(
            size.trim(),
            CARRIED.to_string(),
            "it carries {CARRIED} bytes"
        );
        let first = fs::read(dir.path().join("carried")).unwrap();
        let sshopts = fs::read_to_string(dir.path().join("sshopts")).unwrap();
        assert!(
            sshopts.contains("-o ServerAliveInterval=5 -o ServerAliveCountMax=3"),
            "{sshopts}"
        );
        assert!(
            fs::read_dir(&scratch).unwrap().next().is_none(),
            "what it carried is cleaned up here"
        );
        assert!(carries(STORE, &scratch, &nix, CARRYING), "it carries again");
        let second = fs::read(dir.path().join("carried")).unwrap();
        assert_ne!(first, second, "each proof carries what no store holds");

        let failing = stand_in(
            dir.path(),
            "nix",
            "case \"$1 $2\" in \"store add\") echo /nix/store/aaaa-c; exit 0 ;; esac; exit 1",
        );
        assert!(
            !carries(STORE, &scratch, &failing, CARRYING),
            "a copy that fails doesn't carry"
        );
        let hanging = stand_in(
            dir.path(),
            "nix",
            "case \"$1 $2\" in \"store add\") echo /nix/store/aaaa-c; exit 0 ;; esac; \
             exec tail -f /dev/null",
        );
        let began = Instant::now();
        assert!(
            !carries(STORE, &scratch, &hanging, Duration::from_secs(1)),
            "a copy past its time doesn't carry"
        );
        assert!(
            began.elapsed() < Duration::from_secs(10),
            "a copy that stalls is cut"
        );
        let unadded = stand_in(dir.path(), "nix", "exit 1");
        assert!(
            !carries(STORE, &scratch, &unadded, CARRYING),
            "nothing to carry, no carry"
        );
        assert!(!carries("", &scratch, &nix, CARRYING), "no store, no carry");
    }

    // On 9 November the work Mac's builder had 6 GB free, and every check
    // sent there failed for room while its proofs passed.
    #[test]
    fn a_builder_whose_store_is_under_the_floor_is_full_until_it_has_room() {
        let mut health = Health::of("work-linux");
        health.heard(true, 10);
        assert_eq!(health.state(), State::Passing, "no reading, no floor");
        health.heard(
            Proven {
                passed: true,
                free: Some(FLOOR - 1),
                ..Proven::default()
            },
            70,
        );
        assert_eq!(health.state(), State::Full);
        assert!(health.left_out(), "a full builder takes no builds");
        assert!(!health.dropped(), "nor is it given back");
        health.heard(false, 130);
        assert_eq!(
            health.free,
            Some(FLOOR - 1),
            "a failure says nothing of room"
        );
        assert_eq!(health.state(), State::Full, "one failure leaves it full");
        assert!(health.left_out(), "so it still takes no builds");
        health.heard(false, 190);
        assert_eq!(health.state(), State::Dropped, "two drop it");
        health.heard(
            Proven {
                passed: true,
                free: Some(FLOOR),
                ..Proven::default()
            },
            190,
        );
        assert_eq!(health.state(), State::Passing, "room again");
        assert!(!health.left_out(), "a builder at the floor takes builds");
        assert_eq!(health.json()["freeBytes"], FLOOR);
    }

    #[test]
    fn a_keeper_started_again_goes_on_from_what_the_last_wrote() {
        let dir = tempfile::tempdir().expect("a folder");
        let mut full = Health::of("work-linux");
        full.heard(
            Proven {
                passed: true,
                free: Some(FLOOR - 1),
                ..Proven::default()
            },
            10,
        );
        write(dir.path(), None, Some(&[full]), 20);
        let said = earlier(&dir.path().join(HEALTH), "beside");
        let health = said
            .iter()
            .find_map(Health::from_json)
            .expect("the builder beside, read back");
        let keeper = Keeper::going_on(health);
        assert_eq!(
            keeper.health.state(),
            State::Full,
            "full before its own first proof: {said:?}"
        );
        assert!(
            earlier(&dir.path().join("none.json"), "beside").is_empty(),
            "no file, nothing earlier"
        );
    }

    #[test]
    fn a_proof_is_due_a_minute_after_the_last_began() {
        let began = Instant::now();
        assert!(due(None, began));
        assert!(!due(Some(began), began + Duration::from_secs(59)));
        assert!(due(Some(began), began + EVERY));
    }

    /// What a prover heard, ticking until it hears or 5 s pass.
    fn heard(prover: &mut Prover, now: Instant, passes: bool) -> Option<bool> {
        let ending = Instant::now() + Duration::from_secs(5);
        while Instant::now() < ending {
            if let Some(heard) = prover.tick(now, move || passes) {
                return Some(heard.passed);
            }
            thread::sleep(Duration::from_millis(10));
        }
        None
    }

    #[test]
    fn proofs_run_one_at_a_time_a_minute_apart_and_say_how_they_ended() {
        let began = Instant::now();
        let mut prover = Prover::default();
        let (started, ran) = mpsc::channel();
        let (go, wait) = mpsc::channel::<()>();
        assert_eq!(
            prover.tick(began, move || {
                let _ = started.send(());
                wait.recv().is_ok()
            }),
            None
        );
        ran.recv_timeout(Duration::from_secs(5))
            .expect("it started");
        let later = began + EVERY * 2;
        assert_eq!(
            prover.tick(later, || -> bool {
                panic!("a second ran beside the first")
            }),
            None,
            "one running holds the next back"
        );
        go.send(()).expect("told to end");
        assert_eq!(heard(&mut prover, began, false), Some(true));
        assert_eq!(
            prover.tick(began + Duration::from_secs(30), || -> bool {
                panic!("one ran within the minute")
            }),
            None,
        );
        assert_eq!(prover.tick(later, || false), None, "it starts now");
        assert_eq!(heard(&mut prover, later, false), Some(false));
    }

    #[test]
    fn a_proof_builds_what_no_store_holds_with_what_its_builder_requires() {
        assert_eq!(
            expression("17-3", None, &[], None),
            "derivation { name = \"nix-offload-proof\"; system = builtins.currentSystem; \
             builder = \"/bin/sh\"; args = [ \"-c\" \"echo 17-3 > $out\" ]; \
             requiredSystemFeatures = [ ]; }"
        );
        assert!(
            expression("17-3", Some("x86_64-linux"), &["own", "big-parallel"], None).contains(
                "system = \"x86_64-linux\"; builder = \"/bin/sh\"; \
                 args = [ \"-c\" \"echo 17-3 > $out\" ]; \
                 requiredSystemFeatures = [ \"own\" \"big-parallel\" ]; }"
            )
        );
        assert_ne!(nonce(), nonce(), "each proof's own");
        let measuring = expression("17-3", None, &[], Some("/nix/store/abc-coreutils"));
        assert!(
            measuring.contains(
                "\"echo 17-3 > $out; set -- $(${builtins.storePath \"/nix/store/abc-coreutils\"}\
                 /bin/df -B1 --output=avail /nix/store); echo nix-offload-free $2\""
            ),
            "{measuring}"
        );
    }

    #[test]
    fn a_builder_is_named_by_its_host() {
        assert_eq!(
            named("ssh-ng://work-linux x86_64-linux /k 4 1 - - AAAA"),
            "work-linux"
        );
        assert_eq!(
            named("ssh-ng://builder@home?compress=true x86_64-linux"),
            "home"
        );
    }

    /// A stand-in for a program: records its words a line each in
    /// `given`, beside it, then runs `then`.
    fn stand_in(dir: &Path, name: &str, then: &str) -> String {
        let path = dir.join(name);
        crate::testing::executable(
            &path,
            &format!(
                "#!/bin/sh\n\
                 for word in \"$@\"; do printf '%s\\n' \"$word\"; done > \"{}\"\n\
                 {then}\n",
                dir.join("given").display()
            ),
        );
        path.display().to_string()
    }

    const STORE: &str = "ssh-ng://nix-offload?compress=true&ssh-key=/key&\
                         base64-ssh-public-host-key=c3NoLWVkMjU1MTkgQUFBQQ==\n";

    #[test]
    fn a_lent_builder_is_proven_over_ssh_with_its_key_and_host_key() {
        let dir = tempfile::tempdir().unwrap();
        let ssh = stand_in(dir.path(), "ssh", "exit 0");
        let scratch = dir.path().join("proving");
        assert!(over_ssh(STORE, &scratch, &ssh, WITHIN).passed);
        let given = fs::read_to_string(dir.path().join("given")).unwrap();
        let words: Vec<&str> = given.lines().collect();
        for option in [
            "BatchMode=yes",
            "ConnectTimeout=10",
            "StrictHostKeyChecking=yes",
            "HostKeyAlias=nix-offload",
        ] {
            assert!(words.contains(&option), "{option} in {given}");
        }
        assert!(
            words.windows(2).any(|pair| pair == ["-i", "/key"]),
            "{given}"
        );
        let (host, command) = (words[words.len() - 2], words[words.len() - 1]);
        assert_eq!(host, "nix-offload");
        assert!(
            command.starts_with("nix build --no-link --impure "),
            "{command}"
        );
        assert!(command.contains("--expr 'derivation {"), "{command}");
        assert!(command.contains("builder = \"/bin/sh\""), "{command}");
        assert!(
            fs::read_dir(&scratch).unwrap().next().is_none(),
            "its host key and log are cleaned up"
        );
        let failing = stand_in(dir.path(), "ssh", "exit 255");
        assert!(!over_ssh(STORE, &scratch, &failing, WITHIN).passed);
        assert!(
            !over_ssh("", &scratch, &ssh, WITHIN).passed,
            "no store, no proof"
        );
    }

    #[test]
    fn a_busy_builder_is_asked_only_whether_its_daemon_answers() {
        let dir = tempfile::tempdir().unwrap();
        let ssh = stand_in(dir.path(), "ssh", "exit 0");
        let scratch = dir.path().join("answering");
        assert!(
            answers_over_ssh(STORE, &scratch, &ssh, ANSWERING),
            "a daemon that says its store answers"
        );
        let given = fs::read_to_string(dir.path().join("given")).unwrap();
        let words: Vec<&str> = given.lines().collect();
        for option in ["ConnectTimeout=10", "HostKeyAlias=nix-offload"] {
            assert!(words.contains(&option), "{option} in {given}");
        }
        assert_eq!(words[words.len() - 2..], ["nix-offload", "nix store info"]);
        assert!(
            fs::read_dir(&scratch).unwrap().next().is_none(),
            "its host key and log are cleaned up"
        );
        let refused = stand_in(dir.path(), "ssh", "exit 255");
        assert!(
            !answers_over_ssh(STORE, &scratch, &refused, ANSWERING),
            "one ssh can't reach doesn't answer"
        );
        let silent = stand_in(dir.path(), "ssh", "exec tail -f /dev/null");
        let began = Instant::now();
        assert!(
            !answers_over_ssh(STORE, &scratch, &silent, Duration::from_secs(1)),
            "one that says nothing doesn't answer"
        );
        assert!(
            began.elapsed() < Duration::from_secs(10),
            "one that says nothing is given up on"
        );
        assert!(
            !answers_over_ssh("", &scratch, &ssh, ANSWERING),
            "no store, no answer"
        );
    }

    #[test]
    fn a_lent_builder_says_how_much_its_store_has_free_once_it_has_built() {
        let dir = tempfile::tempdir().unwrap();
        let ssh = stand_in(
            dir.path(),
            "ssh",
            "echo 'nix-offload-free 6000000000' >&2; exit 0",
        );
        let proven = over_ssh(STORE, &dir.path().join("proving"), &ssh, WITHIN);
        assert_eq!(
            proven,
            Proven {
                passed: true,
                free: Some(6_000_000_000),
                ..Proven::default()
            }
        );
        let given = fs::read_to_string(dir.path().join("given")).unwrap();
        assert!(
            given.contains("df -B1 --output=avail /nix/store"),
            "{given}"
        );
        let silent = stand_in(dir.path(), "ssh", "exit 0");
        assert_eq!(
            over_ssh(STORE, &dir.path().join("proving"), &silent, WITHIN).free,
            None
        );
    }

    // On 9 November a switch stopped a quiet build on a lent builder that
    // was still compiling, since nothing said how far it had got.
    #[test]
    fn a_passing_proof_keeps_its_builders_figures_for_health_to_say() {
        let dir = tempfile::tempdir().unwrap();
        let ssh = stand_in(
            dir.path(),
            "ssh",
            "echo 'nix-offload-builds 5 2 ' >&2; exit 0",
        );
        let scratch = dir.path().join("proving.d");
        assert!(over_ssh(STORE, &scratch, &ssh, WITHIN).passed);
        let given = fs::read_to_string(dir.path().join("given")).unwrap();
        assert!(given.contains("&& { printf"), "{given}");
        let mut health = Health::of("lend-1");
        health.heard(true, 90);
        write(dir.path(), Some(&health), None, 100);
        let said: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join(HEALTH)).unwrap()).unwrap();
        assert_eq!(said["builds"]["cpuUsec"], 5);
        assert_eq!(said["builds"]["writtenBytes"], 1024);
        assert!(said["builds"]["at"].as_u64().is_some_and(|at| at > 0));
        write(dir.path(), None, None, 101);
        let said: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join(HEALTH)).unwrap()).unwrap();
        assert!(
            said.get("builds").is_none(),
            "none lent, no figures: {said}"
        );
    }

    #[test]
    fn a_proof_that_hangs_fails_within_its_time_and_ends_what_it_started() {
        let dir = tempfile::tempdir().unwrap();
        let ssh = stand_in(
            dir.path(),
            "ssh",
            "sleep 60 & echo $! > \"$(dirname \"$0\")/child\"; wait",
        );
        let began = Instant::now();
        assert!(
            !over_ssh(
                STORE,
                &dir.path().join("proving"),
                &ssh,
                Duration::from_millis(500)
            )
            .passed
        );
        assert!(
            began.elapsed() < Duration::from_secs(5),
            "{:?}",
            began.elapsed()
        );
        let child: i32 = fs::read_to_string(dir.path().join("child"))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert!(ends(child), "what it started runs on");
    }

    #[test]
    fn a_builder_beside_is_proven_through_the_daemon_in_slots_of_its_own() {
        let dir = tempfile::tempdir().unwrap();
        let nix = stand_in(dir.path(), "nix", "exit 0");
        let line = "ssh-ng://work-linux x86_64-linux,aarch64-linux /etc/nix/key 4 0.5 \
                    big-parallel,own own AAAA";
        assert!(through_daemon(line, &nix, WITHIN).passed);
        let given = fs::read_to_string(dir.path().join("given")).unwrap();
        let words: Vec<&str> = given.lines().collect();
        assert_eq!(
            &words[..5],
            ["build", "--no-link", "--impure", "--max-jobs", "0"],
            "nothing is built here"
        );
        let at = words.iter().position(|word| *word == "--builders").unwrap();
        assert_eq!(
            words[at + 1],
            "ssh-ng://work-linux?max-connections=1 x86_64-linux,aarch64-linux /etc/nix/key 1 \
             0.5 big-parallel,own own AAAA"
        );
        let expression = words[words.len() - 1];
        assert!(
            expression.contains("system = \"x86_64-linux\""),
            "{expression}"
        );
        assert!(
            expression.contains("requiredSystemFeatures = [ \"own\" ]"),
            "{expression}"
        );
        let failing = stand_in(
            dir.path(),
            "nix",
            "echo 'cannot build on work-linux' >&2; exit 1",
        );
        assert!(!through_daemon(line, &failing, WITHIN).passed);
    }

    // Only root's ssh reaches a builder beside, so its proof measures the
    // store from inside its sandbox, with a df it brings along, and says
    // so in the build log that comes back before its output does.
    #[test]
    fn a_builder_beside_says_how_much_its_store_has_free_in_its_proofs_log() {
        let dir = tempfile::tempdir().unwrap();
        let copying = "copying path '/nix/store/abc-nix-offload-proof' from 'ssh-ng://work-linux'";
        let nix = stand_in(
            dir.path(),
            "nix",
            &format!(
                "echo 'nix-offload-proof> nix-offload-free 7000000000' >&2; \
                 echo \"{copying}...\" >&2; sleep 60"
            ),
        );
        let coreutils = Some("/nix/store/abc-coreutils");
        let line = format!("ssh-ng://work-linux {} - 4 1 - - -", here());
        let proven = proven_through(&line, &nix, Duration::from_secs(10), coreutils);
        assert_eq!(
            proven,
            Proven {
                passed: true,
                free: Some(7_000_000_000),
                ..Proven::default()
            }
        );
        let given = fs::read_to_string(dir.path().join("given")).unwrap();
        let words: Vec<&str> = given.lines().collect();
        assert!(words.contains(&"-L"), "its log comes back: {given}");
        assert!(words[words.len() - 1].contains("/bin/df -B1"), "{given}");
        let elsewhere = "ssh-ng://work-linux riscv64-linux - 4 1 - - -";
        proven_through(elsewhere, &nix, Duration::from_secs(10), coreutils);
        let given = fs::read_to_string(dir.path().join("given")).unwrap();
        assert!(
            !given.contains("/bin/df"),
            "this machine's df runs only on its own system: {given}"
        );
    }

    // This store takes nothing while a collection holds its lock, as the
    // disk guard's did for minutes on 8 November, so a proof that built
    // there passes once its output comes back, however long this store
    // keeps it waiting.
    #[test]
    fn a_builder_beside_passes_once_its_proof_comes_back_whatever_holds_this_store() {
        let dir = tempfile::tempdir().unwrap();
        let line = "ssh-ng://work-linux x86_64-linux - 4 1 - - -";
        let copying = "copying path '/nix/store/abc-nix-offload-proof' from 'ssh-ng://work-linux'";
        let nix = stand_in(
            dir.path(),
            "nix",
            &format!("echo \"{copying}...\" >&2; sleep 60"),
        );
        let began = Instant::now();
        assert!(through_daemon(line, &nix, Duration::from_secs(10)).passed);
        assert!(
            began.elapsed() < Duration::from_secs(5),
            "{:?}",
            began.elapsed()
        );
        let stuck = stand_in(dir.path(), "nix", "echo 'copying 1 paths...' >&2; sleep 60");
        assert!(!through_daemon(line, &stuck, Duration::from_millis(500)).passed);
    }

    #[test]
    fn health_is_written_whole_and_read_as_passing_only_while_fresh() {
        let dir = tempfile::tempdir().unwrap();
        let mut health = Health::of("lend-5");
        health.heard(true, 1000);
        let mut beside = Health::of("work-linux");
        beside.heard(false, 990);
        beside.heard(false, 1050);
        write(dir.path(), Some(&health), Some(&[beside]), 1060);
        let said: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join(HEALTH)).unwrap()).unwrap();
        assert_eq!(
            said,
            json!({
                "at": 1060,
                "builder": "lend-5",
                "state": "passing",
                "lastSuccess": 1000,
                "lastFailure": null,
                "consecutiveFailures": 0,
                "freeBytes": null,
                "stalled": false,
                "beside": [{
                    "builder": "work-linux",
                    "state": "dropped",
                    "lastSuccess": null,
                    "lastFailure": 1050,
                    "consecutiveFailures": 2,
                    "freeBytes": null,
                    "stalled": false,
                }],
            })
        );
        assert!(passes_in(dir.path(), 1000 + FRESH));
        assert!(!passes_in(dir.path(), 1001 + FRESH), "a lender gone quiet");
        health.heard(
            Proven {
                passed: true,
                free: Some(1),
                ..Proven::default()
            },
            1000,
        );
        write(dir.path(), Some(&health), None, 1060);
        assert!(
            !passes_in(dir.path(), 1060),
            "a full one stands in for none"
        );
        write(dir.path(), None, None, 1070);
        let said: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join(HEALTH)).unwrap()).unwrap();
        assert_eq!(said["state"], "none");
        assert_eq!(said["builder"], Value::Null);
        assert!(said.get("beside").is_none());
        assert!(!passes_in(dir.path(), 1070));
        assert!(!passes_in(&dir.path().join("nowhere"), 1070));
    }
}
