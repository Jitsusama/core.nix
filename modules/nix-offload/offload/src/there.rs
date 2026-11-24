//! `nix-offload there [-L] [--keep-going] [--apply FUNCTION] [--out DIR]
//! [--print-out-paths] [--here] [--here-when-idle] [--outlive] [--bulk] [--first]
//! FLAKE ATTR...`, or `FLAKE#ATTR...` of one flake, or
//! `DRV...` in place of the flake and its attributes: builds a flake's
//! attributes on the lent builder by sending it a command, or here with
//! `--here` when none takes it, or with `--here-when-idle` while this
//! machine is idle and they hold no VM test. With `--bulk`, its VM tests
//! waiting for room let any run's that isn't go first, and with `--first`
//! they go ahead of every other run's, as its changed-checks evaluation
//! does for a turn on the lent builder.
//!
//! The build hook builds a build's inputs here and copies each up, and a
//! VM test took 1025 s that way against 60 s here, nearly all of it the
//! upload. So a flake the builder can fetch itself, such as a pushed
//! commit of a public repository, goes as one command: the builder
//! fetches it and every input, evaluates and builds, and only the command
//! goes up while the log comes down. Evaluation leaves this machine too.
//! A flake that's a git tree here goes the same way at its commit when
//! GitHub has it, clean. Otherwise its commits since the newest one GitHub
//! has go up as a git bundle, a few kilobytes, with any change to its
//! tracked files as one more commit made without touching its branch or
//! index, and the builder fetches the rest from GitHub.
//!
//! With no builder standing, it asks the lender for one and waits, since
//! one stands in about 70 s, while a VM test here holds the person's
//! machine for minutes, and a lender restarting is waited for too. Each
//! wait says how long it may last. With no lender running, none being
//! made, or none reached, it builds nothing and says so, unless `--here`
//! asks it to build here.
//!
//! `--apply` builds a Nix function applied to the one attribute, and
//! `--out` copies its output back into a folder, as a VM test's pictures.
//! `--print-out-paths` brings what was built into this machine's store,
//! from wherever it was built, and prints its paths as `nix build` does,
//! so a system built elsewhere can be sent on or switched to.
//! `--outlive` lets what's sent to the lent builder run to its end there
//! once its caller is gone, for up to a builder's life, as a prebuild
//! asks, and the lender holds that builder meanwhile, though no ssh here
//! reaches it. The same build sent again while it runs attaches to what it
//! builds through Nix's lock on each derivation, as any run sent again does.
//! `--never-ask`, given first, has it use only a builder that already
//! stands, and never ask the lender for one, for a run not worth a
//! builder of its own, as keeping main's checks ahead isn't: asked at
//! 00:11 on 6 November, through a tunnel gone dead, it asked for one.
//! Nor does it wait on the same changed-checks run going in its tree: it
//! ends at once as one no builder took (`running.rs`).
//! `there --takes` says which of these it takes, a word a line.
//!
//! Derivations, given by their paths in place of a flake and its
//! attributes, are what a build tool that evaluates its own targets
//! hands on. No machine could fetch them, so they're copied up to the
//! lent builder with what they're built from, a few megabytes, and built
//! there, never on a machine nearby. What the builder would build that
//! this machine already holds, and no cache has, goes up built, as a
//! kernel built here does, rather than being built again.
//!
//! A VM test, an attribute a pattern names or a derivation whose features
//! ask for kvm or nixos-test, goes to a builder of its own where the
//! machine lends one, and nowhere else (main.rs says how).
//!
//! An attribute for a system a machine nearby takes, named by its second
//! word as in `checks.aarch64-darwin.x`, goes to that machine instead
//! while it answers, since this one can't build it at all. A flake that's
//! a folder here, which no machine could fetch, goes with it as a
//! tarball, and any input its lock pins to a store path is copied too, so
//! the nearby machine fetches the rest itself and can read inputs only it
//! has. There it waits for one of the machine's turns, [`NEARBY_AT_ONCE`]
//! of them shared by every run sent to it, before it builds.
//!
//! A caller may rewrite any `nix build --no-link` into this without ever
//! failing for the routing: with no builder, or none ssh can reach, the
//! same build runs here, saying so. It exits with the build's own status,
//! or the first that failed, and journals where each part ran and what it
//! sent, in `there.jsonl` beside the lender's journal.
//!
//! A check that passed at a commit isn't built again, wherever it would
//! go, since no builder lent since may hold it: a pass is kept by its
//! commit (`passed.rs`), and a check kept is journalled `already`, built
//! nowhere.

use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::archived;
use crate::held;
use crate::kept;
use crate::passed;
use crate::proof;

/// What a `there` was asked to build, and how.
#[derive(Debug, PartialEq, Eq)]
pub struct Asked {
    flake: String,
    attributes: Vec<String>,
    /// The flags passed through to `nix build`: `-L` and `--keep-going`.
    flags: Vec<String>,
    /// A Nix function applied to the one attribute before it's built, as
    /// `--apply` gives it.
    apply: Option<String>,
    /// Where the one attribute's output is copied back to, as plain files,
    /// as `--out` gives it.
    out: Option<PathBuf>,
    /// Whether what's built is brought into this machine's store and its
    /// paths printed, as `--print-out-paths` asks.
    print: bool,
    /// When it may build here rather than be sent.
    here: Here,
    /// What the build does there once this caller is gone.
    pub alone: Alone,
    /// Whether it's one of many sent at once, as `--bulk` says, whose VM
    /// tests waiting for room let any run's that isn't go first.
    bulk: bool,
    /// Whether the attributes are derivations' paths, with no flake.
    derivations: bool,
    /// The commit a git tree's flake was at when this was asked, which is
    /// what's built, however long it waits for a builder.
    pinned: Option<Pinned>,
}

/// A git tree's flake as it stood when a run began: the commit it's built
/// at, made of its changes when it had some, the tree of files that
/// commit holds, and where its pass is kept, when a commit already named
/// it. Read again once a builder stands, a branch moved on meanwhile was
/// built instead, on 5 November, and kept as passed where it began.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Pinned {
    commit: String,
    tree: String,
    at: Option<passed::At>,
}

/// When a build may be made here rather than sent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Here {
    /// When no builder takes it, as `--here` says.
    unsent: bool,
    /// While this machine is idle, as `--here-when-idle` says, sparing
    /// the trip there.
    idle: bool,
}

/// What a build sent far does once its caller is gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alone {
    /// It ends once no caller has claimed it for `GRACE` seconds.
    Ends,
    /// It goes on to its end, for up to `OUTLIVE` seconds, as `--outlive`
    /// asks.
    GoesOn,
}

impl Alone {
    /// How its command is sent far.
    fn wrap(self) -> fn(&str) -> String {
        match self {
            Alone::Ends => far_saying_where,
            Alone::GoesOn => outliving,
        }
    }

    /// How long, in seconds, what it builds goes on there with no caller.
    fn lasts(self) -> u32 {
        match self {
            Alone::Ends => GRACE,
            Alone::GoesOn => OUTLIVE,
        }
    }
}

/// Reads `there`'s arguments: its flags, in any place, then the flake and
/// at least one attribute, or installables naming one flake, as
/// `.#checks.x.a .#checks.x.b`. `--apply FUNCTION` and `--out DIR` take one
/// attribute only, and `--out` not with `--print-out-paths`. Anything
/// else is refused, since only a build whose
/// every word is known can be sent whole.
pub fn asked(arguments: &[String]) -> Option<Asked> {
    let mut flags = Vec::new();
    let mut words = Vec::new();
    let (mut apply, mut out, mut print, mut here) = (None, None, false, Here::default());
    let (mut alone, mut bulk) = (Alone::Ends, false);
    let mut arguments = arguments.iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "-L" | "--print-build-logs" | "--keep-going" => {
                if !flags.contains(argument) {
                    flags.push(argument.clone());
                }
            }
            "--apply" => apply = Some(arguments.next()?.clone()),
            "--out" => out = Some(PathBuf::from(arguments.next()?)),
            "--print-out-paths" => print = true,
            "--here" => here.unsent = true,
            "--here-when-idle" => here.idle = true,
            "--outlive" => alone = Alone::GoesOn,
            "--bulk" => bulk = true,
            flag if flag.starts_with('-') => return None,
            word => words.push(word.to_owned()),
        }
    }
    if words.first().is_some_and(|word| derivation(word)) {
        let whole = words.iter().all(|word| derivation(word));
        return (whole && apply.is_none() && out.is_none()).then(|| Asked {
            flake: String::new(),
            attributes: words,
            flags,
            apply,
            out,
            print,
            here,
            alone,
            bulk,
            derivations: true,
            pinned: None,
        });
    }
    let words = installed(words)?;
    let (flake, attributes) = words.split_first()?;
    let one = attributes.len() == 1;
    let applied_to_one = one || (apply.is_none() && out.is_none());
    (!attributes.is_empty() && applied_to_one && !(print && out.is_some())).then(|| Asked {
        flake: flake.clone(),
        attributes: attributes.to_vec(),
        flags,
        apply,
        out,
        print,
        here,
        alone,
        bulk,
        derivations: false,
        pinned: None,
    })
}

/// `words` as a flake and its attributes: as they are when none names an
/// installable, and split at each `#` when all do, as `nix build` takes
/// them. Sessions wrote `there .#checks.x.a` 15 times from 8 to 10
/// November, each taken as a flake no builder could read and answered
/// as if none were lent. Some naming installables and some not, or
/// installables of two flakes, ask nothing.
fn installed(words: Vec<String>) -> Option<Vec<String>> {
    if !words.iter().any(|word| word.contains('#')) {
        return Some(words);
    }
    let split: Option<Vec<(&str, &str)>> = words.iter().map(|word| word.split_once('#')).collect();
    let split = split?;
    let flake = split.first()?.0;
    let one_flake = split
        .iter()
        .all(|(each, attribute)| *each == flake && !attribute.is_empty());
    one_flake.then(|| {
        let flake = if flake.is_empty() { "." } else { flake };
        std::iter::once(flake)
            .chain(split.iter().map(|(_, attribute)| *attribute))
            .map(str::to_owned)
            .collect()
    })
}

/// Whether `word` is a derivation's path in the store, with `^` and the
/// outputs wanted or without.
fn derivation(word: &str) -> bool {
    let path = word.split('^').next().unwrap_or(word);
    // A store path's name is exact, so `.DRV` names no derivation.
    path.starts_with("/nix/store/")
        && Path::new(path)
            .extension()
            .is_some_and(|extension| extension == "drv")
}

impl Asked {
    /// Whether it may ever be built here, by `--here` or
    /// `--here-when-idle`.
    pub fn builds_here(&self) -> bool {
        self.here.unsent || self.here.idle
    }

    /// Where the flake is when what's asked is checks whose passing is all
    /// that comes of them, so one that passed there needn't be built again:
    /// a GitHub flake at a whole commit, or a git tree here that's its
    /// commit. Nothing applied, copied back or brought here, whose output
    /// is wanted, and nothing but checks, which are built for their passing.
    pub fn kept_at(&self) -> Option<passed::At> {
        let checks = self
            .attributes
            .iter()
            .all(|attribute| attribute.starts_with("checks."));
        if self.derivations || self.apply.is_some() || self.out.is_some() || self.print || !checks {
            return None;
        }
        self.at()
    }

    /// Where the flake is, and what's run as one line, when what's asked
    /// is a VM test's subtests, as run-subtests sends them, with their
    /// pictures copied back: run again at the same commit with the same
    /// words, they pass again and take the same pictures.
    pub fn subtests_at(&self) -> Option<(passed::At, String)> {
        let ([attribute], Some(apply), Some(_)) =
            (self.attributes.as_slice(), &self.apply, &self.out)
        else {
            return None;
        };
        let subtests = attribute.starts_with("legacyPackages.") && attribute.ends_with(".subtests");
        if self.derivations || self.print || !subtests {
            return None;
        }
        Some((self.at()?, format!("{attribute} {apply}")))
    }

    /// Where the flake is in its repository, when a commit names all it
    /// reads: none for a `path:` flake, which reads what git doesn't.
    fn at(&self) -> Option<passed::At> {
        match folder_of(&self.flake) {
            Some(_) if self.flake.starts_with("path:") => None,
            Some(_) if self.pinned.is_some() => self.pinned.as_ref()?.at.clone(),
            Some(folder) => clean_at(&folder),
            None => passed::At::of_github(&self.flake),
        }
    }

    /// Where the one attribute's output is copied back to, if anywhere.
    pub fn out(&self) -> Option<&Path> {
        self.out.as_deref()
    }

    /// The attributes asked for.
    pub fn attributes(&self) -> &[String] {
        &self.attributes
    }

    /// The same build without `passed`, or none when nothing's left.
    pub fn without(&self, passed: &[String]) -> Option<Self> {
        let left: Vec<String> = self
            .attributes
            .iter()
            .filter(|attribute| !passed.contains(attribute))
            .cloned()
            .collect();
        (!left.is_empty()).then(|| self.with(&self.flake, left))
    }

    /// The same build of other attributes, or of the same flake named
    /// otherwise.
    fn with(&self, flake: &str, attributes: Vec<String>) -> Self {
        Self {
            flake: flake.to_owned(),
            attributes,
            flags: self.flags.clone(),
            apply: self.apply.clone(),
            out: self.out.clone(),
            print: self.print,
            here: self.here,
            alone: self.alone,
            bulk: self.bulk,
            derivations: self.derivations,
            pinned: self.pinned.clone().filter(|_| flake == self.flake),
        }
    }

    /// The same, its git tree's flake pinned at the commit it's at now,
    /// which is then all that's built of it and all its pass is kept at.
    /// A flake that isn't a git tree here, or that git can't commit, stays
    /// as it was.
    #[must_use]
    pub fn pinned(mut self, scratch: &Path) -> Self {
        let folder = folder_of(&self.flake).filter(|_| !self.flake.starts_with("path:"));
        if let Some(folder) = folder.filter(|_| !self.derivations) {
            self.pinned = pinned_at(&folder, scratch);
        }
        self
    }

    /// `nix build`'s arguments for this, wherever it runs.
    fn build(&self) -> Vec<String> {
        self.build_of(&self.flake, false)
    }

    /// `nix build`'s arguments for this from `flake`. An applied function
    /// is built as an expression, which a flake that isn't locked, as one
    /// here is, needs `--impure` to read.
    fn build_of(&self, flake: &str, locked: bool) -> Vec<String> {
        let mut words: Vec<String> = ["build", "--no-link"]
            .iter()
            .map(|word| (*word).to_owned())
            .collect();
        if self.out.is_some() || self.print {
            words.push("--print-out-paths".to_owned());
        }
        words.extend(self.flags.iter().cloned());
        match &self.apply {
            // A derivation named alone is built whole, as `nix build` takes
            // one only with the outputs it's to build.
            _ if self.derivations => {
                words.extend(self.attributes.iter().map(|path| {
                    if path.contains('^') {
                        path.clone()
                    } else {
                        format!("{path}^*")
                    }
                }));
            }
            Some(function) => {
                if !locked {
                    words.push("--impure".to_owned());
                }
                words.push("--expr".to_owned());
                words.extend(self.attributes.iter().map(|attribute| {
                    format!(
                        "({function}) (builtins.getFlake {}).{attribute}",
                        nix_string(flake)
                    )
                }));
            }
            None => words.extend(
                self.attributes
                    .iter()
                    .map(|attribute| format!("{flake}#{attribute}")),
            ),
        }
        words
    }

    /// The command the builder's shell is given.
    fn command(&self) -> String {
        self.command_of(&self.flake)
    }

    /// The command a builder's shell is given to build this from `flake`,
    /// which there is always locked. With `--out`, the output comes back
    /// as a tarball on its standard output, the log staying on its error,
    /// its links followed as [`copied`] follows them here: a link into the
    /// store points at a path this machine may not hold, and on 23
    /// November a symlinkJoin's came back dangling.
    fn command_of(&self, flake: &str) -> String {
        self.command_in(flake, true)
    }

    /// The same, from a `flake` that's locked or, as a folder unpacked
    /// there is, not.
    fn command_in(&self, flake: &str, locked: bool) -> String {
        let build = std::iter::once("nix".to_owned())
            .chain(self.build_of(flake, locked).iter().map(|word| quoted(word)))
            .collect::<Vec<_>>()
            .join(" ");
        if self.out.is_some() {
            format!("out=$({build}) && tar -czhf - -C \"$out\" .")
        } else {
            build
        }
    }
}

/// A string as Nix reads one, quoted.
fn nix_string(text: &str) -> String {
    format!(
        "\"{}\"",
        text.replace('\\', r"\\")
            .replace('"', "\\\"")
            .replace("${", "\\${")
    )
}

/// The lent builder, as nix-offload names its store: where ssh reaches it,
/// the key it takes and its host key.
#[derive(Debug, PartialEq, Eq)]
pub struct Builder {
    /// The host, as ssh is given it, with its user where the store names one.
    pub host: String,
    /// The key the builder takes, where the store names one.
    pub key: Option<String>,
    /// The builder's host key, as a line of a known hosts file has it.
    pub host_key: Option<String>,
    /// Its store, as Nix is given one to copy from.
    pub store: String,
}

/// The builder an ssh store's URI names, as Nix writes one:
/// `ssh-ng://host?ssh-key=...&base64-ssh-public-host-key=...`.
pub fn builder(store: &str) -> Option<Builder> {
    let rest = store
        .strip_prefix("ssh-ng://")
        .or_else(|| store.strip_prefix("ssh://"))?;
    let (host, settings) = rest.split_once('?').unwrap_or((rest, ""));
    if host.is_empty() {
        return None;
    }
    let setting = |name: &str| {
        settings
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.to_owned())
    };
    Some(Builder {
        host: host.to_owned(),
        key: setting("ssh-key"),
        host_key: setting("base64-ssh-public-host-key").and_then(|key| decoded(&key)),
        store: store.to_owned(),
    })
}

/// ssh's own status when it couldn't reach the builder or lost it, which
/// no build gives.
pub const UNREACHED: i32 = 255;

/// What a lent builder's gate ends a run marked [`MOVES`] with when the
/// run would have to wait there: sysexits.h's protocol error, which
/// neither ssh nor a build gives.
pub const BUSY: i32 = 76;

/// What a run's command opens with when it may go to the other lent
/// builder, so the gate of one that would make it wait says so at once,
/// as [`BUSY`]: on 8 November runs queued at one builder for an hour
/// while the other stood idle. A gate from before it runs it, waiting.
pub const MOVES: &str = ": NIX_OFFLOAD_MOVES=1; ";

/// Where a build ran and how it ended.
#[derive(Debug, PartialEq, Eq)]
pub struct Ran {
    /// The builder's host, or none when it ran here.
    pub on: Option<String>,
    /// The build's status.
    pub status: i32,
    /// What ssh sent the builder, as its own count says.
    pub sent: Option<u64>,
}

/// Builds what was asked on the builder `store` names (the store file's
/// first line), or here when there's none or ssh can't reach it. `scratch`
/// holds, for the length of the build, the builder's host key as ssh
/// reads it and ssh's own log; `ssh` and `nix` are the programs run.
/// Builds `asked` on the lent builder `store` names, or here without one.
/// A builder lost mid-build, as a switch's takeback restarts its tunnel,
/// is asked `again` for, a store's text or none, and the build is sent
/// once more: built here, a VM test held the person's machine for minutes
/// on 30 October while the builder stood again within 20 s. `again` is
/// given when the wait began: each loss after the build sent again ran
/// `waiting.worked` starts a wait of its own, since two switches 32
/// minutes apart on 3 November found the second loss's wait spent by the
/// first, while a loss sooner goes on with the last wait, so one that
/// never reaches a builder still stops within `waiting.longest`.
pub fn run(
    asked: &Asked,
    store: Option<&str>,
    again: &dyn Fn(std::time::Instant) -> Option<String>,
    waiting: Waiting,
    scratch: &Path,
    [ssh, nix]: [&str; 2],
    signatures: Signatures,
) -> Ran {
    let mut outage: Option<std::time::Instant> = None;
    // Whether the last builder tried refused the host key its line pins,
    // so a run that ends unsent says that rather than that ssh found none.
    let mut refused_last = false;
    let lent = store.and_then(|store| builder(store.lines().next()?.trim()));
    let Some(mut lent) = lent else {
        return unsent(asked, nix, "no builder is lent");
    };
    // What the flake's lock takes from this machine alone: store paths go
    // ahead of it, and a failure there for want of the rest comes back to
    // be made here (`kept.rs`).
    let mut inputs = folder_of(&asked.flake)
        .filter(|_| !asked.derivations)
        .and_then(|folder| fs::read_to_string(folder.join("flake.lock")).ok())
        .map(|lock| kept::inputs_of(&lock))
        .unwrap_or_default();
    // A repository elsewhere that only this machine may fetch goes ahead as
    // the copy it holds, but never under --lent-only, which sends nothing
    // of this machine's store.
    if !lent_only() {
        inputs.sent.extend(held_here(&inputs.fetched, nix));
        inputs.sent.sort();
        inputs.sent.dedup();
    }
    let reached = sending_of(asked, scratch).map(|mut sending| {
        sending.command = kept::wrapped(&sending.command, &inputs.kept);
        sending
    });
    let reached = reached.and_then(|sending| {
        let cuts = Cuts::default();
        let ran = loop {
            let mut tried = std::time::Instant::now();
            // Copied to each builder that stands: one stood again holds none.
            let copied = match all_up(asked, &inputs, &lent, scratch, [ssh, nix], &cuts) {
                Err(untaken) => break Err(untaken),
                Ok(copied) => copied,
            };
            let building = copied.is_ok();
            let refused = matches!(&copied, Err(uncopied) if uncopied.refused);
            refused_last = refused;
            let ran = match copied {
                Ok(()) => {
                    let bring = Bring::asked(asked, nix, signatures);
                    tried = std::time::Instant::now();
                    let ran = on_with(&lent, &sending, bring, scratch, ssh, asked.alone.wrap());
                    if !matches!(&ran, Ok(ran) if ran.status == UNREACHED) {
                        break ran.map_err(Untaken::Waits);
                    }
                    ran
                }
                Err(uncopied) => {
                    eprintln!("nix-offload: {}", uncopied.why);
                    cuts.heard(&uncopied);
                    Ok(Ran {
                        on: Some(lent.host.clone()),
                        status: UNREACHED,
                        sent: None,
                    })
                }
            };
            let lost = std::time::Instant::now();
            let since = match outage {
                Some(since) if lost.duration_since(tried) < waiting.worked => since,
                _ => lost,
            };
            outage = Some(since);
            say_lost(
                &lent,
                waiting,
                since == lost,
                building.then(|| asked.alone.lasts()),
                refused,
            );
            // A refused key goes to whatever the line names next, the same
            // key included, until the outage's `longest`: each try pins
            // only the line's key, so strict checking stands, and the old
            // builder may still answer for a moment after the line names
            // its successor.
            match again(since).and_then(|store| builder(store.lines().next()?.trim())) {
                Some(stood) => lent = stood,
                None => break ran.map_err(Untaken::Waits),
            }
        };
        if let Some(bundle) = &sending.bundle {
            let _ = fs::remove_file(bundle);
        }
        ran
    });
    match reached {
        // Only a command `kept::wrapped` changed says this for a kept
        // input; any other 65 is the build's own.
        Ok(ran) if ran.status == kept::KEPT && !inputs.kept.is_empty() => stays(
            asked,
            nix,
            &format!(
                "its evaluation on the builder read {}, which this machine alone holds",
                inputs.kept_names()
            ),
        ),
        Ok(ran) if ran.status != UNREACHED => ran,
        Ok(_) if refused_last => unsent(
            asked,
            nix,
            &format!(
                "ssh couldn't verify the host key {} showed against the one its lender's \
                 line pins",
                lent.host
            ),
        ),
        Ok(_) => unsent(asked, nix, &format!("ssh couldn't reach {}", lent.host)),
        Err(Untaken::Waits(why)) => unsent(asked, nix, &why),
        Err(Untaken::Stays(why)) => stays(asked, nix, &why),
    }
}

/// Says `lent` was lost, or `refused` the host key its line pins, and how
/// long this waits for a builder, from now when `fresh` or from the last
/// loss. A builder that stayed up goes on
/// with what it was building for `far`'s grace period, `building` seconds,
/// and the build sent again waits on that, so it says how it ends but shows
/// none of its log: a VM test cut off mid-test on 2 November passed there,
/// and only its exit said so. One lost while copying up had nothing
/// building.
fn say_lost(lent: &Builder, waiting: Waiting, fresh: bool, building: Option<u32>, refused: bool) {
    let minutes = waiting.longest.as_secs().div_ceil(60);
    let from = if fresh { "now" } else { "its last loss" };
    if refused {
        eprintln!(
            "nix-offload: ssh couldn't verify the host key {} showed, as when its lender \
             hands its name to another builder or the copy's pinned key file is gone, so \
             this reads the lender's line again and sends it to the builder that line \
             pins, for up to {minutes} minutes from {from}",
            lent.host
        );
    } else {
        eprintln!(
            "nix-offload: ssh lost {}, so this waits for a builder to stand again, for up \
             to {minutes} minutes from {from}",
            lent.host
        );
    }
    if let Some(lasts) = building {
        eprintln!(
            "nix-offload: if it stands again within {} minutes, what it was building \
             goes on there, and this shows only how it ends; `nix log` on the builder \
             has the rest",
            lasts / 60
        );
    }
}

/// Copies up to `lent` what `asked` builds from: its derivations, when it
/// names them, and the inputs this machine alone holds. A copy failing on
/// its own account is why no builder takes it; one whose ssh lost the
/// builder goes as a run whose ssh lost it does, to a builder standing
/// again or the one beside.
fn all_up(
    asked: &Asked,
    inputs: &kept::Inputs,
    lent: &Builder,
    scratch: &Path,
    [ssh, nix]: [&str; 2],
    cuts: &Cuts,
) -> Result<Result<(), Uncopied>, Untaken> {
    let alive = &cuts.alive();
    match asked
        .derivations
        .then(|| up(asked, lent, scratch, [ssh, nix], alive))
    {
        Some(Err(uncopied)) if !uncopied.lost => Err(Untaken::Waits(uncopied.why)),
        Some(Err(uncopied)) => Ok(Err(uncopied)),
        _ => match inputs_up(inputs, lent, nix, alive) {
            Err(uncopied) if !uncopied.lost => Err(Untaken::Stays(uncopied.why)),
            copied => Ok(copied),
        },
    }
}

/// The store copies this machine holds of the repositories elsewhere that
/// `fetched` names by what they hold, found where Nix stores a fetched
/// repository. One it doesn't hold goes unnamed, for the builder to fetch
/// itself as before.
fn held_here(fetched: &[(String, String)], nix: &str) -> Vec<String> {
    let nix_store = Path::new(nix).with_file_name("nix-store");
    let asked = |args: &[&str]| {
        Command::new(&nix_store)
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()
            .filter(|said| said.status.success())
    };
    fetched
        .iter()
        .filter_map(|(_, held)| {
            let printed = asked(&[
                "--print-fixed-path",
                "--recursive",
                "sha256",
                held,
                "source",
            ])?;
            let path = String::from_utf8(printed.stdout).ok()?.trim().to_owned();
            asked(&["--check-validity", &path]).map(|_| path)
        })
        .collect()
}

/// Sends `lent` the store paths the flake takes from this machine alone,
/// when it takes any, its ssh given the keepalive `alive`.
fn inputs_up(
    inputs: &kept::Inputs,
    lent: &Builder,
    nix: &str,
    alive: &str,
) -> Result<(), Uncopied> {
    if inputs.sent.is_empty() {
        return Ok(());
    }
    if lent_only() {
        return Err(Uncopied::failed(format!(
            "it takes {} from this machine alone, and --lent-only sends none of it",
            inputs.sent.join(", ")
        )));
    }
    eprintln!(
        "nix-offload: sending {} the flake takes from this machine alone to {}",
        inputs.sent.join(", "),
        lent.host
    );
    sent_up(&inputs.sent, lent, nix, send_within(0), alive)
}

/// What the builder's shell is sent to build `asked`: its command, and the
/// commits or tarball its standard input carries, from `scratch`.
fn sending_of(asked: &Asked, scratch: &Path) -> Result<Sending, Untaken> {
    match folder_of(&asked.flake) {
        _ if asked.derivations => Ok(Sending {
            command: asked.command(),
            bundle: None,
        }),
        None => Ok(Sending {
            command: asked.command(),
            bundle: None,
        }),
        // Its files, which a fenced session can write, never go.
        Some(_) if lent_only() && asked.flake.starts_with("path:") => Err(Untaken::Stays(
            "--lent-only sends a git tree's commit, never a folder's files".to_owned(),
        )),
        // A `path:` flake is read as its folder holds it, git or not.
        Some(folder) if asked.flake.starts_with("path:") => packed(&folder, asked, scratch, true),
        Some(folder) if lent_only() => source(
            &folder,
            scratch,
            asked.pinned.as_ref().map(|it| &*it.commit),
            Sends::Committed,
        )
        .map(|source| source.sending(asked))
        .map_err(Untaken::Stays),
        Some(folder) => source(
            &folder,
            scratch,
            asked.pinned.as_ref().map(|it| &*it.commit),
            Sends::Changes,
        )
        .map(|source| source.sending(asked))
        .or_else(|why| {
            eprintln!("nix-offload: {why}, so it goes as a tarball");
            packed(&folder, asked, scratch, false)
        }),
    }
}

/// The systemd-run this program was built with, where the system has one.
const SYSTEMD_RUN: Option<&str> = option_env!("NIX_OFFLOAD_SYSTEMD_RUN");

/// How much of what an upload reads stays in memory once it's read.
const UPLOAD_MEMORY: &str = "MemoryHigh=1G";

/// A command running `nix` to copy store paths away from this machine. The
/// account's copy reads the store's files itself, so their cache counts
/// against its caller's own memory: on 10 November a cargo proof's scope
/// held 5.6 to 12 GB of it, which its windows' memory then paid for. Each
/// upload runs in a scope of its own beside its caller, where
/// [`UPLOAD_MEMORY`] reclaims that cache as it goes. With no systemd or no
/// user manager to ask, it runs as it is. Its ssh has the keepalive
/// `alive` after what the run's own `NIX_SSHOPTS` says.
fn upload(nix: &str, alive: &str) -> Command {
    uploading(nix, upload_scope(), alive)
}

/// The systemd-run an upload's scope is asked of, where this machine has
/// one and a user manager's bus to ask.
fn upload_scope() -> Option<&'static str> {
    let bus = std::env::var_os("XDG_RUNTIME_DIR")
        .is_some_and(|runtime| Path::new(&runtime).join("bus").exists());
    SYSTEMD_RUN.filter(|_| bus)
}

/// [`upload`], through `systemd_run` where one may be asked.
fn uploading(nix: &str, systemd_run: Option<&str>, alive: &str) -> Command {
    // A scope runs its command in systemd-run's own process, so ending it
    // ends the copy, as when the copy runs alone.
    let mut command = systemd_run.map_or_else(
        || Command::new(nix),
        |systemd_run| {
            let mut command = Command::new(systemd_run);
            command
                .args([
                    "--user",
                    "--scope",
                    "--quiet",
                    "--collect",
                    "--slice-inherit",
                ])
                .args(["-p", UPLOAD_MEMORY, "--", nix]);
            command
        },
    );
    command.env(
        "NIX_SSHOPTS",
        ahead_of_alive(std::env::var("NIX_SSHOPTS").ok().as_deref(), alive),
    );
    command
}

/// What a copy up tells its ssh after what `NIX_SSHOPTS` says, unless
/// that says otherwise: give the builder up once nothing at all has come
/// back from it for 15 s. A builder taking what it's sent says so as it goes,
/// ssh's window opening as its side reads, so one nothing reaches is cut,
/// and so is one stalled 15 s, however it would have gone on. On 13
/// November copies up
/// to both lent builders stalled behind a full uplink and waited out the
/// 60 s of keepalives the builder's ssh entry gives, while ssh's window
/// and the relay's buffers held what Nix said it had sent. That is a
/// run's first copy's; one sent again after a cut waits longer
/// ([`copy_alive`]).
pub const COPY_ALIVE: &str = "-o ServerAliveInterval=5 -o ServerAliveCountMax=3";

/// The keepalive a run's copy up has after `cuts` of its copies went
/// unanswered: 15 s for the first, then a minute. A copy up an uplink
/// that's lossy and full can stall past 15 s and then move again: on 17
/// November a 24 MB base sent beside a run copying to every lender was
/// cut at 15 s on each try, while a 9.8 MB one reached its builder whole
/// at 18 to 45 KB/s. A minute, not more, since a run waits at most four
/// for a builder to stand again after a loss.
fn copy_alive(cuts: u32) -> String {
    format!(
        "-o ServerAliveInterval={ALIVE_EVERY} -o ServerAliveCountMax={}",
        alive_count(cuts)
    )
}

/// Seconds between a copy up's keepalives.
const ALIVE_EVERY: u32 = 5;

/// How many keepalives in a row a copy up may have go unanswered after
/// `cuts` ([`copy_alive`]).
fn alive_count(cuts: u32) -> u32 {
    if cuts == 0 { 3 } else { 12 }
}

/// How long, in seconds, a copy up waits on an unanswered keepalive after
/// `cuts` of its run's copies were cut ([`copy_alive`]).
fn alive_for(cuts: u32) -> u32 {
    ALIVE_EVERY * alive_count(cuts)
}

/// `given` ahead of the copy's keepalive `alive`, since ssh keeps the first value it
/// reads for each option: a run whose copy is too slow for 15 s, behind
/// an uplink others fill, can then give it longer. On 16 November a
/// 267 MB copy up was cut four times that way, its run's own
/// `ServerAliveCountMax` read second and ignored.
fn ahead_of_alive(given: Option<&str>, alive: &str) -> String {
    match given {
        Some(given) if !given.trim().is_empty() => format!("{given} {alive}"),
        _ => alive.to_owned(),
    }
}

/// What ssh says when its keepalive goes unanswered, as `Timeout, server
/// localhost not responding.`: a stall, which waiting longer may outlast,
/// rather than a connection refused or reset, which it can't.
const CUT_SAID: &str = "not responding";

/// What Nix, or the ssh under it, says when ssh lost the builder midway
/// through a copy, rather than the copy failing on its own account.
const LOST_SAID: [&str; 5] = [
    CUT_SAID,
    "Nix daemon disconnected",
    "cannot open connection to remote store",
    "Connection closed",
    "Connection reset",
];

/// What ssh says when it refuses the host key a builder shows, most often
/// one other than the key pinned. A lender hands its name over to a new
/// builder with a key of its own: at 09:08:59 on 21 November vm-3's did,
/// and a run that read the old line at 09:08:31 waited its turn to send,
/// reached the new builder at 09:12 and was refused, building nothing,
/// since Nix then says only that it couldn't start ssh, which `LOST_SAID`
/// doesn't hold. Each try pins only its line's key, so strict checking
/// stands however often it's sent again.
const REFUSED_SAID: &str = "Host key verification failed";

/// What systemd-run says when the user manager's bus won't take it, so
/// the copy's scope was never made and the copy never ran, as `Failed to
/// connect to user scope bus via local transport: Connection refused`.
/// The bus refuses while a switch reloads it: at 23:53 on 23 November
/// copies that had waited 58 to 146 minutes for a turn failed so in a
/// second, and their runs gave the builds up.
const UNSCOPED_SAID: [&str; 2] = ["Failed to connect to", "bus"];

/// Why a copy up failed, whether ssh lost the builder under it, which a
/// builder standing again, or the one beside, may take instead, and
/// whether that was ssh's keepalive going unanswered, after which the
/// copy sent again waits longer ([`copy_alive`]).
struct Uncopied {
    why: String,
    lost: bool,
    cut: bool,
    refused: bool,
}

impl Uncopied {
    fn failed(why: String) -> Self {
        Self {
            why,
            lost: false,
            cut: false,
            refused: false,
        }
    }

    /// `why`, or why and that ssh lost the builder, as its copy `heard`. A
    /// builder that refused its pinned key is lost too, since its lender
    /// may have handed its name to another.
    fn of(why: String, heard: Heard) -> Self {
        let SshHeard { lost, cut, refused } = heard.ssh;
        if refused {
            Self {
                why: format!("{why}, since ssh couldn't verify the host key it showed"),
                lost: true,
                cut,
                refused,
            }
        } else if lost {
            Self {
                why: format!("{why}, since ssh lost it"),
                lost,
                cut,
                refused,
            }
        } else {
            Self::failed(why)
        }
    }
}

/// A copy running, whose standard error is passed on as it comes, heard
/// for whether it says ssh lost the builder, whether its keepalive went
/// unanswered, and whether the builder refused its pinned key.
struct Copying {
    child: std::process::Child,
    heard: std::sync::mpsc::Receiver<Heard>,
}

/// What a copy's standard error said: of its ssh, and whether its scope
/// couldn't be made ([`UNSCOPED_SAID`]).
#[derive(Clone, Copy, Default)]
struct Heard {
    ssh: SshHeard,
    unscoped: bool,
}

/// What a copy's standard error said of ssh: that it lost the builder,
/// that its keepalive went unanswered, and that the builder's host key
/// wasn't the one pinned.
#[derive(Clone, Copy, Default)]
struct SshHeard {
    lost: bool,
    cut: bool,
    refused: bool,
}

/// How many of a run's copies up went unanswered by ssh's keepalive, so
/// its later copies wait longer ([`copy_alive`]): kept by each run, as one
/// process sends to several builders.
#[derive(Default)]
pub struct Cuts(std::cell::Cell<u32>);

impl Cuts {
    /// The keepalive the run's next copy up has.
    pub fn alive(&self) -> String {
        copy_alive(self.0.get())
    }

    /// Counts `uncopied` when its keepalive went unanswered, and says how
    /// long the next copy waits.
    fn heard(&self, uncopied: &Uncopied) {
        if uncopied.cut {
            self.0.set(self.0.get().saturating_add(1));
            eprintln!(
                "nix-offload: its keepalive went unanswered, so a copy sent again in this run \
                 waits up to {} s for the builder, where NIX_SSHOPTS gives no keepalive of its \
                 own",
                alive_for(self.0.get())
            );
        }
    }
}

/// Makes `command`, a copy holding an uplink turn or an ssh holding a
/// changed-checks turn, end with the run that starts it, so one killed
/// alone leaves no copy sending outside the uplink's turns, nor a run's
/// ssh to its builder outside the builder's. Linux sends it SIGTERM, which
/// `nix copy` and ssh end on, once the thread that started it ends, so
/// each is waited on in the thread that started it, as every one is. A
/// run that died as it started, before the signal was asked for, is seen
/// as its parent changing, and it doesn't start; that branch is proven
/// only by reading, since no test can kill a run between its fork and its
/// child's `prctl`. A Mac has no such signal, and there either can outlive
/// its run.
fn dies_with_run(command: &mut Command) -> &mut Command {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        // SAFETY: getpid can't fail and has no preconditions.
        let parent = unsafe { libc::getpid() };
        // SAFETY: prctl and getppid are async-signal-safe and touch only
        // the child.
        unsafe {
            command.pre_exec(move || {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::getppid() != parent {
                    return Err(std::io::Error::from_raw_os_error(libc::ESRCH));
                }
                Ok(())
            });
        }
    }
    command
}

impl Copying {
    fn start(command: &mut Command) -> std::io::Result<Self> {
        use std::io::BufRead;
        let mut child = dies_with_run(command).stderr(Stdio::piped()).spawn()?;
        let (said, heard) = std::sync::mpsc::channel();
        if let Some(errors) = child.stderr.take() {
            std::thread::spawn(move || {
                let mut heard = Heard::default();
                for line in std::io::BufReader::new(errors)
                    .split(b'\n')
                    .map_while(Result::ok)
                {
                    let line = String::from_utf8_lossy(&line);
                    heard.ssh.lost |= LOST_SAID.iter().any(|said| line.contains(said));
                    heard.ssh.cut |= line.contains(CUT_SAID);
                    heard.ssh.refused |= line.contains(REFUSED_SAID);
                    heard.unscoped |= UNSCOPED_SAID.iter().all(|said| line.contains(said));
                    eprintln!("{line}");
                }
                let _ = said.send(heard);
            });
        }
        Ok(Self { child, heard })
    }

    /// What it said of ssh, once it has ended. It's read once.
    fn heard(&self) -> Heard {
        self.heard
            .recv_timeout(std::time::Duration::from_secs(1))
            .unwrap_or_default()
    }
}

/// Whether `lent` takes the store path `path` within `within`, copied up
/// as every copy is but alone and quiet, for a proof: nothing fetches it
/// elsewhere, since no cache holds it.
pub fn carried_up(path: &str, lent: &Builder, nix: &str, within: std::time::Duration) -> bool {
    // ssh's control socket goes in TMPDIR, as when what's built comes back.
    uploading(nix, None, COPY_ALIVE)
        .args(["copy", "--to", &lent.store, path])
        .env("TMPDIR", "/tmp")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()
        .and_then(|mut carrying| finished_within(&mut carrying, within))
        .is_some_and(|status| status.success())
}

/// Sends the store paths `sent` and what they reference to the lent
/// builder ahead of what reads them there, fetched here first where this
/// store lacks them, within `within`. They go as held outputs do, signed
/// or content-addressed, since the builder trusts no account: the source
/// a lock pins is named by its content, and what this machine builds it
/// signs. Where one can't go, nothing that reads it is sent.
/// Its ssh has the keepalive the run's `cuts` give, and a cut is counted
/// there.
pub fn paths_up(
    sent: &[String],
    lent: &Builder,
    nix: &str,
    within: std::time::Duration,
    cuts: &Cuts,
) -> Result<(), String> {
    sent_up(sent, lent, nix, within, &cuts.alive()).map_err(|uncopied| {
        cuts.heard(&uncopied);
        uncopied.why
    })
}

/// [`paths_up`], its ssh given the keepalive `alive`, saying whether ssh
/// lost the builder when it fails.
fn sent_up(
    sent: &[String],
    lent: &Builder,
    nix: &str,
    within: std::time::Duration,
    alive: &str,
) -> Result<(), Uncopied> {
    if sent.is_empty() {
        return Ok(());
    }
    let sending = |systemd_run: Option<&str>| {
        // ssh's control socket goes in TMPDIR, as when what's built comes back.
        let mut copying = Copying::start(
            uploading(nix, systemd_run, alive)
                .args(["copy", "--to", &lent.store, "--substitute-on-destination"])
                .args(sent)
                .env("TMPDIR", "/tmp"),
        )
        .map_err(|error| Uncopied::failed(format!("couldn't run {nix}: {error}")))?;
        let ended = finished_within(&mut copying.child, within);
        Ok::<_, Uncopied>((
            ended.is_some_and(|status| status.success()),
            copying.heard(),
        ))
    };
    let scope = upload_scope();
    let (mut went, mut heard) = sending(scope)?;
    // A scope the bus refused never ran the copy, so it goes again
    // outside one ([`copied_up`]).
    if !went && scope.is_some() && heard.unscoped {
        eprintln!(
            "nix-offload: the send couldn't be given a scope of its own, since the user \
             manager's bus refused it, so it goes again outside one"
        );
        (went, heard) = sending(None)?;
    }
    if went {
        return Ok(());
    }
    Err(Uncopied::of(
        format!("{} couldn't be sent to the builder", sent.join(", ")),
        heard,
    ))
}

/// Why no builder took a build: one a builder standing again may take, or
/// one no builder ever can, since it takes something from this machine
/// alone.
enum Untaken {
    Waits(String),
    Stays(String),
}

/// What `there` exits with when a build can't leave this machine and it
/// wasn't asked to build here: sysexits.h's usage error, since asking
/// again changes nothing, where 75 sends a caller round to wait for a
/// builder. It sent a run of 93 checks round three times on 3 November.
pub const STAYS: i32 = 64;

/// The system a derivation's file, as Nix writes one, builds for: the
/// string after its outputs, its inputs' derivations and its sources.
pub fn derivation_system(written: &str) -> Option<String> {
    let fields = written.strip_prefix("Derive(")?;
    let (mut depth, mut commas, mut quoted, mut escaped) = (0_usize, 0, false, false);
    let mut system: Option<String> = None;
    for character in fields.chars() {
        if let Some(read) = system.as_mut() {
            match (escaped, character) {
                (false, '\\') => escaped = true,
                (false, '"') => return system,
                _ => {
                    read.push(character);
                    escaped = false;
                }
            }
            continue;
        }
        match (quoted, escaped, character) {
            (true, false, '\\') => escaped = true,
            (true, false, '"') => quoted = false,
            (true, ..) => escaped = false,
            (false, _, '"') if depth == 0 && commas == 3 => system = Some(String::new()),
            (false, _, '"') => quoted = true,
            (false, _, '[' | '(') => depth += 1,
            (false, _, ']' | ')') => depth = depth.checked_sub(1)?,
            (false, _, ',') if depth == 0 => commas += 1,
            _ => {}
        }
    }
    None
}

/// What the lent builder `dir`'s lender lends builds for, as its machines
/// line says, and on Linux this machine's own system where none is lent
/// yet, since the builder asked for then is built as this one is. The
/// line is the one naming the store's host: root reaches it as another
/// account, by a URI of its own, so the two never match whole.
pub fn lent_systems(dir: &Path) -> Vec<String> {
    let store = fs::read_to_string(dir.join("store")).unwrap_or_default();
    let store = store.lines().next().unwrap_or_default().trim();
    let machines = fs::read_to_string(dir.join("machines")).unwrap_or_default();
    let named = machines
        .lines()
        .filter(|_| !store.is_empty())
        .find_map(|line| {
            let (_, rest) = line.split_once(' ')?;
            (proof::named(line) == proof::named(store))
                .then(|| rest.split_whitespace().next().map(str::to_owned))?
        })
        .map(|systems| systems.split(',').map(str::to_owned).collect());
    named.unwrap_or_else(|| {
        if cfg!(target_os = "linux") {
            vec![format!("{}-linux", std::env::consts::ARCH)]
        } else {
            Vec::new()
        }
    })
}

/// What of `part`, which a machine nearby couldn't take, may go to `dir`'s
/// lent builder instead, and the attributes that may not: those naming a
/// system it doesn't build for, which could only fail there. On 15
/// November a gate's checks for a Mac went to the lent builder when the
/// folder couldn't be sent to the Mac, and 22 failed for want of one.
/// Asked to build here, all of it goes, as asked, since Nix here may
/// have builders of its own.
pub fn lent_builds(part: &Asked, dir: &Path) -> (Option<Asked>, Vec<String>) {
    let lent = lent_systems(dir);
    let (goes, stays): (Vec<String>, Vec<String>) =
        part.attributes.iter().cloned().partition(|attribute| {
            part.builds_here()
                || lent.is_empty()
                || system_of(attribute)
                    .is_none_or(|system| lent.iter().any(|built| built == system))
        });
    let goes = (!goes.is_empty()).then(|| part.with(&part.flake, goes));
    (goes, stays)
}

/// For a run of derivations, the first that no lent builder builds for,
/// what it builds for and what they build for instead.
pub fn foreign_derivation(asked: &Asked, dir: &Path) -> Option<(String, String, Vec<String>)> {
    if !asked.derivations {
        return None;
    }
    let systems = lent_systems(dir);
    let names: Vec<&str> = systems.iter().map(String::as_str).collect();
    let (path, system) = foreign(&asked.attributes, &names, |path| {
        fs::read_to_string(path).ok()
    })?;
    Some((path, system, systems))
}

/// The first of the derivations at `paths` that builds for none of the
/// lent builders' `systems`, and what it builds for, read by `read`: on
/// 9 November an Arm build was sent to a lender that builds for Intel
/// alone, and began copying its kernel up. None when the systems aren't
/// known or each derivation's can't be read.
pub fn foreign(
    paths: &[String],
    systems: &[&str],
    read: impl Fn(&str) -> Option<String>,
) -> Option<(String, String)> {
    if systems.is_empty() {
        return None;
    }
    paths.iter().find_map(|word| {
        let path = word.split('^').next().unwrap_or(word);
        let system = derivation_system(&read(path)?)?;
        (!systems.contains(&system.as_str())).then(|| (path.to_owned(), system))
    })
}

/// A build no builder can take, `why`: built here only when asked to with
/// `--here`, and otherwise refused with the ways it could go.
fn stays(asked: &Asked, nix: &str, why: &str) -> Ran {
    if asked.here.unsent {
        eprintln!("nix-offload: {why}, so this builds here");
        return here(asked, nix);
    }
    eprintln!(
        "nix-offload: {why}, so no builder can build it; evaluate it here and give its \
         derivations, or give --here to build on this machine"
    );
    Ran {
        on: None,
        status: STAYS,
        sent: None,
    }
}

/// Puts what the derivations asked for are built from in the builder's
/// store: the derivations themselves, and the outputs this machine holds
/// of those it would build.
fn up(
    asked: &Asked,
    lent: &Builder,
    scratch: &Path,
    [ssh, nix]: [&str; 2],
    alive: &str,
) -> Result<(), Uncopied> {
    let paths: Vec<&str> = asked
        .attributes
        .iter()
        .map(|path| path.split('^').next().unwrap_or(path))
        .collect();
    derivations_up(&paths, lent, scratch, rank(asked), [ssh, nix], alive)?;
    held_up(asked, &paths, lent, scratch, [ssh, nix]);
    Ok(())
}

/// Copies the derivations at `paths`, and what they're built from, into
/// the builder's store, which builds only what it holds. The builder is
/// asked once which of them it lacks, and only those go: `nix copy` asks
/// it path by path, and through the tunnel that took 79 s on 1 November
/// for the 710 a one-line build needs, every one already there, against
/// 4 s asked once. Without an answer they go the slow way. Each is named
/// by its contents, so the builder takes them unsigned. What they're built
/// from can be outputs a cache holds, which the builder fetches itself
/// rather than being sent: on 10 November one run's 1.1 GB took 77
/// minutes to go up, 290 MB of it from cache.nixos.org.
fn derivations_up(
    paths: &[&str],
    lent: &Builder,
    scratch: &Path,
    rank: Rank,
    [ssh, nix]: [&str; 2],
    alive: &str,
) -> Result<(), Uncopied> {
    let mut copying: Vec<String> = Vec::new();
    // Only what the closure holds goes, so a builder's answer can't name
    // other paths of this store to send, and each is named once.
    let size = match closure(paths, nix).and_then(|closure| {
        let within: std::collections::HashSet<&String> = closure.iter().collect();
        lacking(lent, &closure, scratch, ssh).map(|lacking| {
            let (asked, beyond): (BTreeSet<_>, BTreeSet<_>) =
                lacking.into_iter().partition(|path| within.contains(path));
            let named = match beyond.len() {
                0 => None,
                1 => Some(
                    "1 path it lacks that isn't among what was asked, which doesn't go".to_owned(),
                ),
                count => Some(format!(
                    "{count} paths it lacks that aren't among what was asked, which don't go"
                )),
            };
            if let Some(named) = named {
                eprintln!("nix-offload: the lent builder {} named {named}", lent.host);
            }
            asked.into_iter().collect::<Vec<_>>()
        })
    }) {
        Some(lacking) if lacking.is_empty() => return Ok(()),
        Some(lacking) => {
            let (count, them) = match lacking.len() {
                1 => ("1 derivation".to_owned(), "it"),
                count => (format!("{count} derivations"), "them"),
            };
            eprintln!(
                "nix-offload: copying {count} up to the lent builder {}, which lacks {them}",
                lent.host
            );
            let size = stored_size(
                Command::new(nix)
                    .args(["path-info", "--json"])
                    .args(&lacking),
            );
            copying.extend(
                ["copy", "--no-recursive", "--substitute-on-destination"]
                    .into_iter()
                    .chain(["--to", &lent.store])
                    .map(str::to_owned),
            );
            copying.extend(lacking);
            Some(size)
        }
        None => {
            eprintln!(
                "nix-offload: copying the derivations up to the lent builder {} with all \
                 they're built from, since what it lacks is unknown",
                lent.host
            );
            copying.extend(
                ["copy", "--derivation", "--substitute-on-destination"]
                    .into_iter()
                    .chain(["--to", &lent.store])
                    .chain(paths.iter().copied())
                    .map(str::to_owned),
            );
            // Its size would take a second walk of a closure the first
            // couldn't read, so its turn is leased for the least instead.
            None
        }
    };
    let about = match size {
        None => format!("to {}, of a size unknown here,", lent.host),
        Some(0) => format!("to {}, of a size this machine couldn't read,", lent.host),
        Some(size) => format!("of {} to {}", sized(size), lent.host),
    };
    let turn = uplink_turn(rank, &about);
    let within = turn_within(size);
    let (ended, heard) = copied_up([nix, alive], upload_scope(), &copying, turn, within, &about)
        .map_err(Uncopied::failed)?;
    match ended {
        Ok(status) if status.success() => Ok(()),
        _ => Err(Uncopied::of(
            "the derivations couldn't be copied up to the builder".to_owned(),
            heard,
        )),
    }
}

/// How a copy up of `args` to `nix` ended and what it heard, its ssh
/// given the keepalive `alive`, run in a scope of `systemd_run`'s where
/// one is named ([`uploading`]) and waited on as [`waited_giving_back`]
/// does with its uplink `turn`. When its scope couldn't be made
/// ([`UNSCOPED_SAID`]), the copy never ran, so it runs once again outside
/// one, keeping its turn, rather than give up the wait that won it. Why
/// not when `nix` can't be run.
fn copied_up(
    [nix, alive]: [&str; 2],
    systemd_run: Option<&str>,
    args: &[String],
    mut turn: Option<Turn>,
    within: Option<(std::time::Duration, &str)>,
    about: &str,
) -> Result<(std::io::Result<std::process::ExitStatus>, Heard), String> {
    let mut say = |line: String| eprintln!("{line}");
    let mut run = |command: &mut Command, turn: &mut Option<Turn>| {
        // ssh's control socket goes in TMPDIR, as when what's built comes back.
        let mut copied = Copying::start(command.args(args).env("TMPDIR", "/tmp"))
            .map_err(|error| format!("couldn't run {nix}: {error}"))?;
        let ended = waited_giving_back(&mut copied.child, turn, within, about, &mut say);
        Ok::<_, String>((ended, copied.heard()))
    };
    let (ended, heard) = run(&mut uploading(nix, systemd_run, alive), &mut turn)?;
    let ran = matches!(&ended, Ok(status) if status.success());
    if ran || systemd_run.is_none() || !heard.unscoped {
        return Ok((ended, heard));
    }
    eprintln!(
        "nix-offload: the copy up {about} couldn't be given a scope of its own, since the user \
         manager's bus refused it, so it goes again outside one, keeping its turn"
    );
    run(&mut uploading(nix, None, alive), &mut turn)
}

/// How many bytes the store paths `nix path-info --json` prints from
/// `listing` hold, or none when it prints nothing.
fn stored_size(listing: &mut Command) -> u64 {
    held::described(&json_of(listing))
        .values()
        .map(|stored| stored.size)
        .sum()
}

/// How `copy` ended, its uplink `turn` given back once the copy has held
/// it past `within` ([`turn_within`], with why that's its lease), while it
/// goes on. A copy of many small paths through a tunnel goes at the
/// tunnel's round trips, not the link's speed: on 22 November one went at
/// 25 to 140 KB/s for two hours, held one of the three turns all along,
/// and runs that would have filled the link waited behind it. A turn is
/// given back only for one of [`UPLOADS`] given turns, held to the copy's
/// end, so the runs of one folder of turns send no more than twice as
/// many copies of derivations and held outputs at once: on Linux a copy
/// ends with its run ([`dies_with_run`]), while on a Mac one whose run was
/// killed alone goes on outside the count. Sends that take no turn, a
/// run's inputs ([`inputs_up`]), `--copy` paths ([`paths_up`]) and every
/// copy of a run sent first, aren't counted. With none free it keeps its
/// own until one is, saying so once and naming who holds them; with some
/// of their files unopenable it waits on the rest, and with none it keeps
/// it to the end, saying why each time. `about` says which copy, for the lines it `say`s.
/// With no lease, or no turn, it waits on the copy alone. The turn is
/// taken from `turn` only when it's given back, so a caller sending the
/// copy again ([`copied_up`]) still holds one it kept.
fn waited_giving_back(
    copy: &mut std::process::Child,
    turn: &mut Option<Turn>,
    within: Option<(std::time::Duration, &str)>,
    about: &str,
    say: &mut dyn FnMut(String),
) -> std::io::Result<std::process::ExitStatus> {
    let began = std::time::Instant::now();
    let Some(folder) = turn.as_ref().map(|turn| turn.folder.clone()) else {
        return copy.wait();
    };
    // With no lease the turn is held to the copy's end, not let go here.
    let Some((within, why)) = within else {
        return copy.wait();
    };
    // Opened once the lease is up, and kept open while they're looked at.
    let mut given: Option<Result<Vec<fs::File>, String>> = None;
    // Why given turns couldn't be locked, said once each as it's found.
    let mut unlocked = Vec::new();
    let mut kept = false;
    loop {
        if let Some(status) = copy.try_wait()? {
            return Ok(status);
        }
        if began.elapsed() >= within {
            let files = given.get_or_insert_with(|| {
                let (files, unopened) = turn_files(&folder, "uplink-given");
                if files.is_empty() {
                    return Err(unopened.join("; "));
                }
                if !unopened.is_empty() {
                    say(format!(
                        "nix-offload: the copy up {about} can't open every given turn ({}), so \
                         it gives its own back only for one of the other {}",
                        unopened.join("; "),
                        files.len()
                    ));
                }
                Ok(files)
            });
            let taken = match files {
                Ok(files) => match free_turn(files, &mut unlocked) {
                    Some(free) => {
                        let had = began.elapsed();
                        holding(&free, about);
                        *turn = None;
                        say(format!(
                            "nix-offload: the copy up {about} has held its turn for {}, {why}, \
                             so it goes on without it, and the turn is free for another copy",
                            lasting(had)
                        ));
                        let status = copy.wait();
                        drop(free);
                        return status;
                    }
                    None if !unlocked.is_empty() => {
                        let why = unlocked.join("; ");
                        unlocked.clear();
                        Some(if files.is_empty() {
                            format!(
                                "no given turn in {} can be locked ({why}), so it keeps it to \
                                 the end",
                                folder.display()
                            )
                        } else {
                            format!(
                                "a given turn in {} can't be locked ({why}), so it gives its \
                                 own back only for one of the other {}",
                                folder.display(),
                                files.len()
                            )
                        })
                    }
                    None => (!kept).then(|| {
                        format!(
                            "{} already go on past theirs ({}), so it keeps it until one ends",
                            copies(files.len()),
                            holders(&folder, "uplink-given")
                        )
                    }),
                },
                Err(why) => (!kept).then(|| {
                    format!(
                        "the given turns' files can't be opened ({why}), so it keeps it to the end"
                    )
                }),
            };
            if let Some(taken) = taken {
                say(format!(
                    "nix-offload: the copy up {about} has held its turn for {}, {why}, but \
                     {taken}",
                    lasting(within)
                ));
                kept = true;
            }
            if matches!(&given, Some(Err(_)))
                || matches!(&given, Some(Ok(files)) if files.is_empty())
            {
                return copy.wait();
            }
        }
        // Looked at 10 times a second, so a copy ending frees its turn
        // for the next within a tenth of a second.
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// The first of `files`, turns, that's free, taken out of them and held;
/// none when every one is held elsewhere. One that can't be locked at all
/// is taken out of them, why added to `unlocked`, and the rest looked at,
/// as [`turn_in`] does, so one bad file doesn't keep a copy from a turn
/// free beside it.
fn free_turn(files: &mut Vec<fs::File>, unlocked: &mut Vec<String>) -> Option<fs::File> {
    let mut at = 0;
    while at < files.len() {
        match files[at].try_lock() {
            Ok(()) => return Some(files.swap_remove(at)),
            Err(fs::TryLockError::WouldBlock) => at += 1,
            Err(fs::TryLockError::Error(error)) => {
                unlocked.push(error.to_string());
                drop(files.swap_remove(at));
            }
        }
    }
    None
}

/// How many copies `count` is, in words.
fn copies(count: usize) -> String {
    match count {
        1 => "1 copy".to_owned(),
        count => format!("{count} copies"),
    }
}

/// Writes into `turn`, held, who holds it: this run's pid and `about`, its
/// copy, so `cat` of the folder names the runs holding every turn. Where
/// it can't be written it's held all the same, its holder unnamed. The
/// words go at the file's start, since emptying it leaves the file's
/// offset where a write before left it, and a second write there came
/// after a run of NULs that no reader took for a holder.
fn holding(turn: &fs::File, about: &str) {
    use std::os::unix::fs::FileExt;
    let words = format!("pid {}, copying up {about}\n", std::process::id());
    let _ = turn
        .set_len(0)
        .and_then(|()| turn.write_all_at(words.as_bytes(), 0));
}

/// The turn's file at `at`, made where `make` says and it isn't there,
/// for writing then or reading otherwise; why not when it can't be opened
/// or isn't a plain file of this account's own, with no other name. A
/// link isn't followed, so no lock is pointed at a file elsewhere; it's
/// opened without waiting, so a FIFO planted at its name is refused
/// rather than hanging the open for good; and one with a second name is
/// refused, since emptying it would empty a file elsewhere.
pub fn open_turn(at: &Path, make: bool) -> Result<fs::File, String> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let said = |why: &dyn std::fmt::Display| format!("{}: {why}", at.display());
    let file = fs::OpenOptions::new()
        .read(!make)
        .write(make)
        .create(make)
        .truncate(false)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(at)
        .map_err(|error| said(&error))?;
    let found = file.metadata().map_err(|error| said(&error))?;
    // SAFETY: geteuid has no preconditions and can't fail.
    let uid = unsafe { libc::geteuid() };
    if !found.is_file() || found.nlink() != 1 || found.uid() != uid {
        return Err(said(&"not a plain file of this account's alone"));
    }
    Ok(file)
}

/// Who holds the turns in `folder` named `kind`, as each holder wrote it
/// ([`holding`]), for a line saying why a copy waits on them. Only a turn
/// held now is named, since one let go keeps its last holder's words, and
/// only by a holder still running: a turn is taken before its holder
/// writes its words, so for an instant it may hold the last holder's,
/// whose run has mostly ended, or name one that has just let it go. A
/// holder is looked for with a shared lock, so two waiters looking at
/// once don't take each other's look for a holder, and no turn's file
/// that's a link is read.
fn holders(folder: &Path, kind: &str) -> String {
    use std::io::Read;
    let named: Vec<String> = (1..=UPLOADS)
        .filter_map(|turn| {
            let mut file = open_turn(&folder.join(format!("{kind}-{turn}")), false).ok()?;
            if !matches!(file.try_lock_shared(), Err(fs::TryLockError::WouldBlock)) {
                return None;
            }
            let mut holder = String::new();
            file.read_to_string(&mut holder).ok()?;
            Some(holder.trim().to_owned())
        })
        .filter(|holder| still_running(holder))
        .collect();
    if named.is_empty() {
        "none of them named".to_owned()
    } else {
        named.join("; ")
    }
}

/// Whether the run a turn's `holder` words name ([`holding`]) still runs,
/// or the words name none, so they can't be judged. A pid of 0 or below
/// names no run, since asking after it asks after a whole group.
fn still_running(holder: &str) -> bool {
    let Some(pid) = holder
        .strip_prefix("pid ")
        .and_then(|rest| rest.split(',').next())
        .and_then(|pid| pid.parse::<libc::pid_t>().ok())
    else {
        return !holder.is_empty();
    };
    if pid <= 0 {
        return false;
    }
    // SAFETY: a kill of signal 0 only asks whether the process is there.
    let asked = unsafe { libc::kill(pid, 0) };
    asked == 0 || std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
}

/// How long a copy up of `bytes` holds its uplink turn before it gives it
/// back, and why that long, for the line that says so; none when it holds
/// it to its end. A copy whose bytes were read holds it for what they take
/// at a third of [`SENDS_AT`], since [`UPLOADS`] copies share the link,
/// and never less than [`SEND_LEAST`], so a copy sharing the link fairly
/// ends within its turn and only one going slower gives it up. A copy
/// whose bytes read as none, which only a size it couldn't read does,
/// holds it to its end, since a lease could be too short for any size;
/// one whose size wasn't asked, as when what the builder lacks is unknown,
/// holds it for [`SEND_LEAST`]. `NIX_OFFLOAD_TURN_WITHIN` gives any copy
/// its lease in seconds, exactly. It's apart from [`send_within`], which
/// stops a held send, so neither moves the other.
fn turn_within(bytes: Option<u64>) -> Option<(std::time::Duration, &'static str)> {
    turn_within_of(bytes, seconds_of("NIX_OFFLOAD_TURN_WITHIN"))
}

/// [`turn_within`], with `knob` the seconds its variable gives, if any.
fn turn_within_of(
    bytes: Option<u64>,
    knob: Option<u64>,
) -> Option<(std::time::Duration, &'static str)> {
    if let Some(seconds) = knob {
        return Some((
            std::time::Duration::from_secs(seconds),
            "as long as NIX_OFFLOAD_TURN_WITHIN gives one",
        ));
    }
    let share = SENDS_AT / u64::from(UPLOADS);
    match bytes {
        None => Some((SEND_LEAST, "the least any copy holds one")),
        Some(0) => None,
        Some(bytes) => Some((
            (SEND_STARTS + std::time::Duration::from_secs(bytes / share)).max(SEND_LEAST),
            "more than its share of this machine's uplink would take",
        )),
    }
}

/// How many copies go up from this machine at once. On 10 November 30
/// runs sent at once shared an uplink of 11 Mbit/s, and one run's 1.1 GB
/// took 77 minutes. A few at once keep the link full, and the rest wait
/// their turn and then go at its whole speed, rather than all crawling.
const UPLOADS: u32 = 3;

/// A turn at sending up from this machine, had while its file is held,
/// so one that dies frees it, and the folder its turns are kept in. The
/// file is never read, only kept open, since its lock is the turn.
struct Turn {
    _held: fs::File,
    folder: PathBuf,
}

/// The seconds the variable `name` gives, if it's set to a whole number
/// of them; one set to anything else is said to be ignored, once a run,
/// though every copy reads it, so a person turning the knob sees why it
/// didn't move.
fn seconds_of(name: &str) -> Option<u64> {
    static WARNED: std::sync::Mutex<BTreeSet<String>> = std::sync::Mutex::new(BTreeSet::new());
    let set = std::env::var(name).ok()?;
    let (seconds, warning) = knob_seconds(name, &set, &WARNED);
    if let Some(warning) = warning {
        eprintln!("nix-offload: {warning}");
    }
    seconds
}

/// The seconds the knob `name`, `set`, gives, and what to say of it when
/// it gives none and `warned` doesn't yet hold it, which it then does.
fn knob_seconds(
    name: &str,
    set: &str,
    warned: &std::sync::Mutex<BTreeSet<String>>,
) -> (Option<u64>, Option<String>) {
    let seconds = seconds_in(set);
    let first = seconds.is_none()
        && warned
            .lock()
            .map_or(true, |mut warned| warned.insert(name.to_owned()));
    let warning =
        first.then(|| format!("{name} is {set:?}, not a whole number of seconds, so it's ignored"));
    (seconds, warning)
}

/// The whole seconds `set` gives, spaces around them aside.
fn seconds_in(set: &str) -> Option<u64> {
    set.trim().parse().ok()
}

/// A turn at sending up from this machine, for the copy `about` names.
/// A run sent first takes none, as the net's go ahead of sessions'. When
/// none can be had, the copy goes on without one.
fn uplink_turn(rank: Rank, about: &str) -> Option<Turn> {
    if rank == Rank::First {
        return None;
    }
    let folder = turns_folder()?;
    let held = turn_in(&folder, about, TURN_SAID_EVERY, &mut |line| {
        eprintln!("{line}");
    })?;
    Some(Turn {
        _held: held,
        folder,
    })
}

/// The folder the uplink's turns are kept in: the account's runtime
/// folder's, or with none one of its own in the temp folder, made only
/// it may enter. On Linux that's the `/tmp` every account writes to, so a
/// folder there of another's, or a link, is refused, saying so, rather
/// than letting another hold every turn or point one at a file of ours.
/// changed-checks' runs keep their turns there too (`changed/turns.rs`).
/// With neither usable, as where another account made the temp folder's
/// name first and no runtime folder is set, the turns' limits are lifted
/// rather than every copy wedged, which each copy says: a guard on load,
/// it fails open.
pub fn turns_folder() -> Option<PathBuf> {
    // SAFETY: geteuid has no preconditions and can't fail.
    let uid = unsafe { libc::geteuid() };
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);
    turns_folder_in(runtime, &std::env::temp_dir(), uid)
        .map_err(|why| eprintln!("nix-offload: {why}, so this takes no turn at sending"))
        .ok()
}

/// [`turns_folder`], under the runtime folder `runtime` if there is one
/// and in `temp` otherwise, or in `temp` where the runtime folder's can't
/// be used, as one inherited through `sudo -E` or left after its account
/// logged out, for the account `uid`; why not, of each, when neither is
/// that account's own folder. A runtime folder is used only when its
/// path is absolute, as the XDG spec asks, and it's a folder of that
/// account's that no other may write, as logind makes it: an empty or
/// relative one would keep each working folder's turns apart, and
/// another account that can write the folder itself could move the
/// turns folder aside and put its own in its place. One others may only
/// read is used, since the turns folder within is closed to them. Only
/// the folder itself is looked at, not those above it, which logind
/// makes root's alone.
fn turns_folder_in(runtime: Option<PathBuf>, temp: &Path, uid: u32) -> Result<PathBuf, String> {
    use std::os::unix::fs::MetadataExt;
    let in_temp = || turn_folder_at(&temp.join(format!("nix-offload-there-{uid}")), uid);
    let Some(runtime) = runtime else {
        return in_temp();
    };
    let alone = runtime.is_absolute()
        && fs::symlink_metadata(&runtime)
            .is_ok_and(|found| found.is_dir() && found.uid() == uid && found.mode() & 0o022 == 0);
    let ours = if alone {
        turn_folder_at(&runtime.join("nix-offload-there"), uid)
    } else {
        Err(format!(
            "{} isn't a runtime folder of this account's alone",
            runtime.display()
        ))
    };
    ours.or_else(|first| in_temp().map_err(|then| format!("{first}, and {then}")))
}

/// The folder `folder`, made only the account `uid` may enter where it
/// isn't there; why not when it can't be made or isn't that account's own.
fn turn_folder_at(folder: &Path, uid: u32) -> Result<PathBuf, String> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
    let folder = folder.to_owned();
    let created = fs::DirBuilder::new().mode(0o700).create(&folder);
    let ours = fs::symlink_metadata(&folder)
        .ok()
        .filter(|made| made.is_dir() && made.uid() == uid);
    let Some(made) = ours else {
        return Err(match created {
            Err(error) if error.kind() != std::io::ErrorKind::AlreadyExists => {
                format!("{} can't be made ({error})", folder.display())
            }
            _ => format!("{} isn't a folder of this account's", folder.display()),
        });
    };
    // One made before turns were kept apart may be open to others.
    if made.mode() & 0o077 != 0 {
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o700)).map_err(|error| {
            format!(
                "{} is open to other accounts and can't be closed ({error})",
                folder.display()
            )
        })?;
    }
    Ok(folder)
}

/// One of the [`UPLOADS`] turns in `folder`, the first free, for the copy
/// `about` names, waited for while all are taken, `say`ing so then and
/// again `every` ([`TURN_SAID_EVERY`]) with how long it has waited. Each is looked at
/// in turn 10 times a second, so a waiter takes whichever frees first,
/// rather than waiting on one while another stands free: on 22 November a
/// system build waited on its one while another copy's had ended. Waiters
/// take freed turns first come, first served, each holding a [`Ticket`]
/// and trying the turns only while no live waiter came before it: on 23
/// November, with no order kept, Aquifer's copies waited over 80 minutes
/// three times while copies come later took each freed turn. One that
/// can't take its place in line says so and tries the turns as they free,
/// as before. A turn whose file can't be opened or locked is said
/// once and left out, so the copy waits on the rest; none when no turn's
/// file can be opened or locked at all, which is said too, so a copy then
/// goes on without one rather than waiting for ever.
fn turn_in(
    folder: &Path,
    about: &str,
    every: std::time::Duration,
    say: &mut dyn FnMut(String),
) -> Option<fs::File> {
    let (mut turns, unopened) = turn_files(folder, "uplink");
    let mut unusable = unopened;
    let began = std::time::Instant::now();
    let mut said: Option<std::time::Instant> = None;
    let mut ticket = Ticket::take(folder)
        .map_err(|why| {
            say(format!(
                "nix-offload: this copy can't take its place in line for the uplink ({why}), \
                 so it takes the first turn it finds free"
            ));
        })
        .ok();
    loop {
        let ahead = ticket.as_mut().map_or(0, |ticket| {
            ticket.touch();
            ticket.ahead(folder)
        });
        let mut at = if ahead == 0 { 0 } else { turns.len() };
        while at < turns.len() {
            match turns[at].try_lock() {
                Ok(()) => {
                    let turn = turns.swap_remove(at);
                    holding(&turn, about);
                    return Some(turn);
                }
                Err(fs::TryLockError::WouldBlock) => at += 1,
                Err(fs::TryLockError::Error(error)) => {
                    unusable.push(format!("a turn can't be locked: {error}"));
                    drop(turns.swap_remove(at));
                }
            }
        }
        if !unusable.is_empty() {
            let rest = match turns.len() {
                0 => "so this copy takes no turn at sending".to_owned(),
                1 => "so this copy waits on the other turn".to_owned(),
                count => format!("so this copy waits on the other {count}"),
            };
            say(format!(
                "nix-offload: the uplink's turns in {} aren't all usable ({}), {rest}",
                folder.display(),
                unusable.join("; ")
            ));
            unusable.clear();
        }
        if turns.is_empty() {
            return None;
        }
        if said.is_none_or(|at| at.elapsed() >= every) {
            let first = match ahead {
                0 => String::new(),
                ahead => format!(", and {} waiting that came first", copies(ahead)),
            };
            if said.is_none() && ahead == 0 {
                say(format!(
                    "nix-offload: {} already go up from this machine ({}), so this waits its \
                     turn to send",
                    copies(turns.len()),
                    holders(folder, "uplink")
                ));
            } else if said.is_none() {
                say(format!(
                    "nix-offload: {} waiting came first, behind those sending ({}), so this \
                     waits its turn to send",
                    copies(ahead),
                    holders(folder, "uplink")
                ));
            } else {
                say(format!(
                    "nix-offload: this has waited {} for a turn to send, behind {}{first}",
                    lasting(began.elapsed()),
                    holders(folder, "uplink")
                ));
            }
            said = Some(std::time::Instant::now());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// How often a copy waiting for a turn says again that it waits, so a
/// wait that never ends shows in its run's log as one, not as a run gone
/// quiet.
const TURN_SAID_EVERY: std::time::Duration = std::time::Duration::from_mins(5);

/// How a waiter's ticket's name begins, before when it came and its pid.
const TICKET: &str = "wait-uplink-";

/// How long a waiter's ticket may go untouched and still hold its place.
/// A waiter touches it each second, so only one stopped or starved this
/// long loses its place, and none can keep the line waiting on it.
const TICKET_STALE: std::time::Duration = std::time::Duration::from_secs(5);

/// A copy's place in line for an uplink turn ([`turn_in`]): a file in the
/// turns folder named for when it came, held locked while the copy waits
/// and taken away once it stops. A waiter that dies leaves its file
/// unlocked, and one stopped leaves it untouched, so neither keeps those
/// after it waiting.
struct Ticket {
    file: fs::File,
    path: PathBuf,
    name: String,
    touched: std::time::Instant,
}

impl Ticket {
    /// A place in line in `folder`, after every copy already waiting; why
    /// not when its file can't be made, locked or named. It's locked
    /// before it takes its name, so no other waiter finds it unlocked and
    /// takes it for a dead one's.
    fn take(folder: &Path) -> Result<Self, String> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos();
        let pid = std::process::id();
        let making = folder.join(format!(".making-{pid}-{nanos}"));
        let file = open_turn(&making, true)?;
        let named = match file.try_lock() {
            Ok(()) => {
                let name = format!("{TICKET}{nanos:020}-{pid}");
                let path = folder.join(&name);
                match fs::rename(&making, &path) {
                    Ok(()) => Ok((name, path)),
                    Err(error) => Err(format!("{}: {error}", path.display())),
                }
            }
            Err(fs::TryLockError::WouldBlock) => {
                Err(format!("{} is held elsewhere", making.display()))
            }
            Err(fs::TryLockError::Error(error)) => Err(format!("{}: {error}", making.display())),
        };
        let (name, path) = named.inspect_err(|_| {
            let _ = fs::remove_file(&making);
        })?;
        Ok(Self {
            file,
            path,
            name,
            touched: std::time::Instant::now(),
        })
    }

    /// Touches the ticket, at most once a second, so it reads as a live
    /// waiter's.
    fn touch(&mut self) {
        if self.touched.elapsed() >= std::time::Duration::from_secs(1) {
            let _ = self.file.set_modified(std::time::SystemTime::now());
            self.touched = std::time::Instant::now();
        }
    }

    /// How many live waiters in `folder` came before this one ([`live`]).
    fn ahead(&self, folder: &Path) -> usize {
        let Ok(entries) = fs::read_dir(folder) else {
            return 0;
        };
        entries
            .flatten()
            .filter(|entry| {
                entry.file_name().to_str().is_some_and(|name| {
                    name.starts_with(TICKET) && name < self.name.as_str() && live(&entry.path())
                })
            })
            .count()
    }
}

impl Drop for Ticket {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Whether the ticket at `at` is a live waiter's. One left unlocked was a
/// waiter's that died, and is taken away; one locked but untouched for
/// [`TICKET_STALE`] is a stopped waiter's, passed over but left, so it
/// takes its place again if it goes on. A time in the future, the clock
/// set back, reads as touched now.
fn live(at: &Path) -> bool {
    let Ok(file) = open_turn(at, false) else {
        return false;
    };
    match file.try_lock_shared() {
        Ok(()) => {
            let _ = fs::remove_file(at);
            false
        }
        Err(fs::TryLockError::WouldBlock) => file
            .metadata()
            .and_then(|found| found.modified())
            .is_ok_and(|touched| !matches!(touched.elapsed(), Ok(age) if age >= TICKET_STALE)),
        Err(fs::TryLockError::Error(_)) => false,
    }
}

/// The files of the [`UPLOADS`] turns in `folder` named `kind` that open,
/// opened once, and why each of the rest can't be ([`open_turn`]).
fn turn_files(folder: &Path, kind: &str) -> (Vec<fs::File>, Vec<String>) {
    let (mut opened, mut unopened) = (Vec::new(), Vec::new());
    for turn in 1..=UPLOADS {
        match open_turn(&folder.join(format!("{kind}-{turn}")), true) {
            Ok(file) => opened.push(file),
            Err(why) => unopened.push(why),
        }
    }
    (opened, unopened)
}

/// Every path the derivations at `paths` are built from, themselves
/// among them, as this machine's store has them.
fn closure(paths: &[&str], nix: &str) -> Option<Vec<String>> {
    let listed = Command::new(nix)
        .args(["path-info", "--recursive", "--derivation"])
        .args(paths)
        .stderr(Stdio::inherit())
        .output()
        .ok()
        .filter(|listed| listed.status.success())?;
    let mut closure: Vec<String> = String::from_utf8_lossy(&listed.stdout)
        .lines()
        .map(str::to_owned)
        .collect();
    for path in paths {
        if !closure.iter().any(|had| had == path) {
            closure.push((*path).to_owned());
        }
    }
    Some(closure)
}

/// What the builder is asked, with the paths on its standard input: those
/// its store lacks, a line each.
const LACKING: &str = "xargs -r nix-store --check-validity --print-invalid";

/// Which of `paths` the lent builder lacks, asked in one go, or none when
/// it couldn't say.
fn lacking(lent: &Builder, paths: &[String], scratch: &Path, ssh: &str) -> Option<Vec<String>> {
    let answer = answered(lent, LACKING, paths, scratch, ssh)?;
    Some(
        answer
            .lines()
            .filter(|line| line.starts_with("/nix/store/"))
            .map(str::to_owned)
            .collect(),
    )
}

/// What the lent builder prints running `command` with `lines` on its
/// standard input, a line each, or none when it couldn't say. They go as
/// a file, since a closure of thousands would fill a pipe while the
/// answer waits to be read.
fn answered<Line: AsRef<str>>(
    lent: &Builder,
    command: &str,
    lines: &[Line],
    scratch: &Path,
    ssh: &str,
) -> Option<String> {
    let (mut asking, known, said) = reach(lent, scratch, ssh).ok()?;
    let given = mine(scratch, "asking");
    let text: String = lines
        .iter()
        .map(|line| line.as_ref().to_owned() + "\n")
        .collect();
    let answer = fs::write(&given, text)
        .and_then(|()| fs::File::open(&given))
        .and_then(|input| {
            asking
                .arg(&lent.host)
                .arg(command)
                .stdin(input)
                .stderr(Stdio::inherit())
                .output()
        });
    for file in [&given, &known, &said] {
        let _ = fs::remove_file(file);
    }
    let answer = answer.ok().filter(|answer| answer.status.success())?;
    Some(String::from_utf8_lossy(&answer.stdout).into_owned())
}

/// What the builder is asked, with the derivations on its standard input:
/// what building them would take, which Nix says on its standard error.
const PLANS: &str = "xargs -r nix-store --realise --dry-run 2>&1";

/// Sends up the outputs this machine holds of what the builder would
/// build, which no cache has, so it isn't built again: a fresh builder
/// compiled the running system's own kernel for over an hour on 3
/// November, held here since it was built here. The builder is asked once
/// what it would build, in 2.4 s through the tunnel for a whole system,
/// and [`held::instead`] says which of those go up. What they reference
/// goes only where no cache gives it the builder, which fetches the rest
/// itself, as it would for a build. A download among them, sent or
/// referenced, the builder fetches from its source first, and only one
/// it couldn't goes up: on 9 November a proxy refused a lent builder a Go
/// zone's modules, which this machine held. The builder's account is one
/// its daemon doesn't trust, since a trusted one is root there, so only
/// what's signed by a key it trusts, as this machine's own is, or
/// content-addressed goes up; an output built here before this machine
/// signed what it built the builder builds itself.
///
/// When any of it fails, or the send falls behind [`send_within`], the
/// builder builds what it still lacks, and of what that's built from only
/// the downloads this machine holds go up, the same way: on 10 November
/// a Go zone's 106 MB of modules hadn't gone up in 15 minutes, and the
/// builder, left to build them, was refused the 18 MB of downloads they
/// take. The rest it builds, since what a big output is built from is
/// often bigger.
fn held_up(asked: &Asked, paths: &[&str], lent: &Builder, scratch: &Path, [ssh, nix]: [&str; 2]) {
    // Said before it's asked, since a fresh builder asks its caches about
    // every path the build takes, which can run for minutes.
    eprintln!(
        "nix-offload: asking the lent builder {} what it would build of {}",
        lent.host,
        if paths.len() == 1 { "it" } else { "them" }
    );
    let Some(planned) = answered(lent, PLANS, paths, scratch, ssh).map(|said| held::planned(&said))
    else {
        return;
    };
    if planned.is_empty() {
        return;
    }
    let shown = json_of(
        Command::new(nix)
            .args(["derivation", "show"])
            .args(&planned),
    );
    let derivations = held::derivations(&shown);
    let fixed = held::fixed(&shown);
    let outputs = derivations
        .values()
        .flat_map(|derivation| derivation.outputs.values());
    let mut holds = held::signed(&json_of(
        Command::new(nix)
            .args(["path-info", "--json"])
            .args(outputs),
    ));
    let wanted: Vec<_> = asked
        .attributes
        .iter()
        .map(|word| held::wanted(word))
        .collect();
    let mut sending = held::instead(&planned, &derivations, &wanted, &holds);
    // What went up stays held, so what it's built from isn't sent again.
    let mut up = BTreeSet::new();
    while !sending.is_empty()
        && !sent_held(
            &sending,
            (&planned, &fixed, &holds),
            lent,
            scratch,
            rank(asked),
            [ssh, nix],
        )
    {
        let lacks: BTreeSet<String> = lacking(lent, &sending, scratch, ssh)
            .unwrap_or_else(|| sending.clone())
            .into_iter()
            .collect();
        if lacks.is_empty() {
            return;
        }
        up.extend(
            sending
                .iter()
                .filter(|path| !lacks.contains(*path))
                .cloned(),
        );
        // Each round holds fewer, so the rounds end.
        holds.retain(|path, _| {
            !lacks.contains(path) && (fixed.contains_key(path) || up.contains(path))
        });
        sending = held::instead(&planned, &derivations, &wanted, &holds)
            .into_iter()
            .filter(|path| !up.contains(path))
            .collect();
        if !sending.is_empty() {
            let (outputs, them) = match lacks.len() {
                1 => ("1 output".to_owned(), "it"),
                count => (format!("{count} outputs"), "them"),
            };
            eprintln!(
                "nix-offload: the builder lacks {outputs} that didn't go up and builds {them} \
                 itself, so the downloads this machine holds for {them} go up where it can't \
                 fetch them"
            );
        }
    }
}

/// Sends `sending` up, of those `holds` says this machine holds, after
/// the builder has fetched what downloads it can among them and what they
/// reference, and says whether nothing is left to send: false when the
/// send failed or fell behind [`send_within`]. It takes an uplink turn at
/// its `rank` only once the builder's fetch is done and something is left
/// to send, and holds it for the send alone: taken before the fetch, which
/// has no bound, a turn sat idle for as long as the builder downloaded,
/// while other copies waited behind it.
fn sent_held(
    sending: &[String],
    (planned, fixed, holds): (
        &BTreeSet<String>,
        &std::collections::BTreeMap<String, String>,
        &std::collections::BTreeMap<String, u64>,
    ),
    lent: &Builder,
    scratch: &Path,
    rank: Rank,
    [ssh, nix]: [&str; 2],
) -> bool {
    // `nix copy` sends what they reference too, wherever no cache gives
    // it the builder, so a download is fetched first at its source.
    let closure = held::described(&json_of(
        Command::new(nix)
            .args(["path-info", "--recursive", "--json"])
            .args(sending),
    ));
    let fetching = held::fetched(fixed, &closure);
    let fetched = fetched_first(
        &fetching,
        held::fetched_size(fixed, &closure, &fetching),
        lent,
        scratch,
        ssh,
    );
    // A download the builder fetched itself goes up no more.
    let sending: Vec<&String> = sending
        .iter()
        .filter(|path| {
            !fixed
                .get(*path)
                .is_some_and(|derivation| fetched.contains(derivation))
        })
        .collect();
    if sending.is_empty() {
        return true;
    }
    let own: u64 = sending.iter().filter_map(|path| holds.get(*path)).sum();
    let size = held::sent_up(planned, fixed, &closure, &fetched).max(own);
    let (outputs, them) = match sending.len() {
        1 => ("1 output".to_owned(), "it"),
        count => (format!("{count} outputs"), "them"),
    };
    eprintln!(
        "nix-offload: sending {outputs} of {} this machine holds rather than building {them} \
         there",
        sized(size)
    );
    // ssh's control socket goes in TMPDIR, as when what's built comes back.
    let within = send_within(size);
    let about = format!("of {} of held outputs to {}", sized(size), lent.host);
    let _turn = uplink_turn(rank, &about);
    let mut copying = upload(nix, COPY_ALIVE);
    let ended = dies_with_run(&mut copying)
        .args(["copy", "--to", &lent.store, "--substitute-on-destination"])
        .args(&sending)
        .env("TMPDIR", "/tmp")
        .spawn()
        .map(|mut copy| finished_within(&mut copy, within));
    match ended {
        Ok(Some(status)) if status.success() => return true,
        Ok(None) => eprintln!(
            "nix-offload: the send took longer than {}, slower than this machine's uplink, so \
             it stops and the builder builds what didn't go up",
            lasting(within)
        ),
        _ => eprintln!("nix-offload: they couldn't be sent, so the builder builds {them}"),
    }
    false
}

/// How fast held outputs must go up, in bytes a second, for sending them
/// to be worth waiting on: about all of an uplink of 10 Mbit/s, as this
/// machine's is. On 9 November a send of 1.4 GB shared one with four
/// other tunnels, fell to 25 KB/s and was on course to outlast its run's
/// two hours, when the builder could build what it lacked. What went up by
/// then stays there.
const SENDS_AT: u64 = 1_000_000;

/// What a send is given before its pace counts, for ssh to connect and
/// Nix to ask the builder what it holds.
const SEND_STARTS: std::time::Duration = std::time::Duration::from_mins(2);

/// What any send is given, however small, before it's stopped: a small
/// one costs little to wait on, and what it holds the builder may not be
/// able to build. On 9 November a Go zone's 106 MB of modules went at
/// 180 kB/s, which takes 10 minutes, and was stopped at 4, leaving the
/// builder to fetch what they're built from, which a proxy refused it.
const SEND_LEAST: std::time::Duration = std::time::Duration::from_mins(15);

/// How long a send of `size` bytes may take at [`SENDS_AT`], and never
/// less than [`SEND_LEAST`], unless `NIX_OFFLOAD_SEND_WITHIN` says
/// otherwise in seconds.
pub fn send_within(size: u64) -> std::time::Duration {
    seconds_of("NIX_OFFLOAD_SEND_WITHIN").map_or_else(
        || (SEND_STARTS + std::time::Duration::from_secs(size / SENDS_AT)).max(SEND_LEAST),
        std::time::Duration::from_secs,
    )
}

/// How `send` ended, or none once it ran past `within` and was told to
/// end, which Nix does by ending its ssh and keeping what it had added.
pub fn finished_within(
    send: &mut std::process::Child,
    within: std::time::Duration,
) -> Option<std::process::ExitStatus> {
    let began = std::time::Instant::now();
    while began.elapsed() < within {
        match send.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
            Err(_) => break,
        }
    }
    if let Ok(pid) = libc::pid_t::try_from(send.id()) {
        // SAFETY: a signal to our own child, which hasn't been waited on.
        unsafe { libc::kill(pid, libc::SIGTERM) };
    }
    let told = std::time::Instant::now();
    while told.elapsed() < std::time::Duration::from_secs(5) {
        if send.try_wait().is_ok_and(|status| status.is_some()) {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let _ = send.kill();
    let _ = send.wait();
    None
}

/// A while, as a person reads it: in seconds under two minutes, else in
/// minutes, rounded up.
fn lasting(while_: std::time::Duration) -> String {
    match while_.as_secs() {
        1 => "1 second".to_owned(),
        seconds if seconds < 120 => format!("{seconds} seconds"),
        seconds => format!("{} minutes", seconds.div_ceil(60)),
    }
}

/// What the builder is asked, with fixed derivations on its standard
/// input: to fetch their outputs from their own sources, several at once,
/// printing each it fetched. One it can't fetch, such as a file only this
/// machine was given, leaves the rest fetched there still, so the copy
/// after skips them and only what's printed is counted as fetched.
const FETCHES: &str = "xargs -r -P 8 -I{} sh -c 'nix-store --realise \"$1\" > /dev/null && echo \"$1\"' fetch {}; true";

/// Has the builder fetch `fetching` for itself, saying so, and answers
/// them once it has, or none when it couldn't, so they go up from here
/// as before.
fn fetched_first(
    fetching: &[String],
    size: u64,
    lent: &Builder,
    scratch: &Path,
    ssh: &str,
) -> Vec<String> {
    if fetching.is_empty() {
        return Vec::new();
    }
    let downloads = match fetching.len() {
        1 => "1 download".to_owned(),
        count => format!("{count} downloads"),
    };
    eprintln!(
        "nix-offload: the builder fetches {downloads} of {} from their source itself, rather \
         than have this machine send them",
        sized(size)
    );
    let said = answered(lent, FETCHES, fetching, scratch, ssh).unwrap_or_default();
    let fetched = fetched_of(&said, fetching);
    if fetched.len() < fetching.len() {
        eprintln!(
            "nix-offload: the builder fetched {} of {}, so what it lacks goes up from here",
            fetched.len(),
            fetching.len()
        );
    }
    fetched
}

/// Of `fetching`, those the builder said it fetched, a line each.
fn fetched_of(said: &str, fetching: &[String]) -> Vec<String> {
    let printed: std::collections::BTreeSet<&str> = said.lines().map(str::trim).collect();
    fetching
        .iter()
        .filter(|derivation| printed.contains(derivation.as_str()))
        .cloned()
        .collect()
}

/// What `command` printed as JSON, or null when it printed none.
fn json_of(command: &mut Command) -> serde_json::Value {
    command
        .stderr(Stdio::null())
        .output()
        .ok()
        .and_then(|output| serde_json::from_slice(&output.stdout).ok())
        .unwrap_or_default()
}

/// What `there` exits with when no builder took a build and it wasn't
/// asked to build here: sysexits.h's temporary failure, as `--changed`
/// gives when nowhere took its run.
pub const UNSENT: i32 = 75;

/// A build no builder took, `why`: built here only when asked to with
/// `--here`. Built here unasked after midnight on 30 October, with no
/// builder lent, every session's builds ran beside the person's work at a
/// load of 145. It says not to ask again at once, since a lender that runs
/// was already waited on for as long as a builder takes, and an agent's
/// loop that did asked 40 times in 12 minutes on 3 November.
fn unsent(asked: &Asked, nix: &str, why: &str) -> Ran {
    if asked.here.unsent {
        eprintln!("nix-offload: {why}, so this builds here");
        return here(asked, nix);
    }
    eprintln!(
        "nix-offload: {why}, so nothing was built; asking again at once finds none either, \
         so ask again once a builder is lent, or give --here to build on this machine"
    );
    Ran {
        on: None,
        status: UNSENT,
        sent: None,
    }
}

/// The CPU pressure, as /proc/pressure/cpu's "some" line gives it for
/// the last 10 and 60 seconds, below which this machine is idle: from
/// 14:00 to 20:15 on 5 November it stood at 46% on average beside the
/// checks built here, and at 43 to 55% at a load of 53.
const IDLE: f64 = 10.0;

/// Whether `pressure`, /proc/pressure/cpu as read, says this machine is
/// idle, both of its last 10 and 60 seconds below [`IDLE`]; one that
/// can't be read is taken as busy.
pub fn idle(pressure: &str) -> bool {
    let Some(some) = pressure.lines().find_map(|line| line.strip_prefix("some ")) else {
        return false;
    };
    let average = |name: &str| {
        some.split_whitespace()
            .find_map(|field| field.strip_prefix(name)?.parse::<f64>().ok())
    };
    ["avg10=", "avg60="]
        .iter()
        .all(|name| average(name).is_some_and(|value| value < IDLE))
}

/// The CPU pressure over the last 10 seconds, as /proc/pressure/cpu's
/// "some" line gives it, from which a build begun here while this machine
/// was idle gives way to the builders. At 01:20 on 14 November one began
/// idle, then built 25 Rust programs here for 24 minutes beside a switch,
/// at a load near 96, since idleness was judged only as it began.
const GROWN_BUSY: f64 = 25.0;

/// How often a build begun here while idle reads the pressure again.
const WATCHED: std::time::Duration = std::time::Duration::from_secs(2);

/// Whether `pressure`, /proc/pressure/cpu as read, says this machine is
/// busy now, its last 10 seconds at [`GROWN_BUSY`] or over; one that can't be
/// read is taken as busy, as [`idle`] takes it.
pub fn busy(pressure: &str) -> bool {
    pressure
        .lines()
        .find_map(|line| line.strip_prefix("some "))
        .and_then(|some| {
            some.split_whitespace()
                .find_map(|field| field.strip_prefix("avg10=")?.parse::<f64>().ok())
        })
        .is_none_or(|now| now >= GROWN_BUSY)
}

/// `asked` built here, when it asks to be while this machine is idle,
/// as `pressure` says it is, and it holds no VM test by `patterns`: a VM
/// test here holds the person's machine for minutes. None otherwise, nor
/// for derivations, which name no attribute to judge, nor once `pressure`
/// says this machine grew busy while it built, when the build here is
/// ended and the rest goes to the builders.
pub fn here_when_idle(
    asked: &Asked,
    pressure: &dyn Fn() -> String,
    patterns: &[String],
    nix: &str,
) -> Option<Ran> {
    if !asked.here.idle || asked.derivations || !idle(&pressure()) {
        return None;
    }
    if vm_tests(asked, patterns, &BTreeSet::new()).0.is_some() {
        return None;
    }
    eprintln!("nix-offload: this machine is idle, so this builds here");
    let ran = here_until(asked, nix, &|| busy(&pressure()));
    if ran.is_none() {
        eprintln!(
            "nix-offload: this machine grew busy, so what's left of this goes to the builders"
        );
    }
    ran
}

fn here(asked: &Asked, nix: &str) -> Ran {
    let mut command = Command::new(nix);
    command.args(asked.build());
    let status = match &asked.out {
        None => command
            .status()
            .map_or(1, |status| status.code().unwrap_or(1)),
        // Its log still shows: output() would keep standard error too.
        Some(out) => command
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .output()
            .map_or(1, |built| {
                let path = String::from_utf8_lossy(&built.stdout).trim().to_owned();
                match built.status.code() {
                    Some(0) => copied(Path::new(&path), out),
                    code => code.unwrap_or(1),
                }
            }),
    };
    Ran {
        on: None,
        status,
        sent: None,
    }
}

/// `asked` built here, as [`here`] builds it, unless `busy` says so as it
/// builds, read every [`WATCHED`]: then the build is told to end, what it
/// built stays in the store, and there's none.
fn here_until(asked: &Asked, nix: &str, busy: &dyn Fn() -> bool) -> Option<Ran> {
    use std::io::Read;
    let mut command = Command::new(nix);
    command.args(asked.build());
    if asked.out.is_some() {
        command.stdout(Stdio::piped());
    }
    let Ok(mut build) = command.spawn() else {
        return Some(Ran {
            on: None,
            status: 1,
            sent: None,
        });
    };
    let mut read = std::time::Instant::now();
    let status = loop {
        match build.try_wait() {
            Ok(Some(status)) => break status.code().unwrap_or(1),
            Ok(None) => {}
            Err(_) => break 1,
        }
        if read.elapsed() >= WATCHED {
            if busy() {
                ended(&mut build);
                return None;
            }
            read = std::time::Instant::now();
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    let status = match &asked.out {
        Some(out) if status == 0 => {
            let mut said = String::new();
            let _ = build
                .stdout
                .take()
                .map(|mut stdout| stdout.read_to_string(&mut said));
            copied(Path::new(said.trim()), out)
        }
        _ => status,
    };
    Some(Ran {
        on: None,
        status,
        sent: None,
    })
}

/// Ends `build`: told to, then made to after 10 s.
fn ended(build: &mut std::process::Child) {
    if let Ok(pid) = libc::pid_t::try_from(build.id()) {
        // SAFETY: a signal to our own child, which hasn't been waited on.
        unsafe { libc::kill(pid, libc::SIGTERM) };
    }
    let told = std::time::Instant::now();
    while told.elapsed() < std::time::Duration::from_secs(10) {
        if build.try_wait().is_ok_and(|status| status.is_some()) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let _ = build.kill();
    let _ = build.wait();
}

/// Copies a build's output into `out` as plain files the account owns,
/// giving 0 once it has.
pub fn copied(built: &Path, out: &Path) -> i32 {
    let copied = fs::create_dir_all(out).is_ok()
        && Command::new("cp")
            .args(["-rL", "--no-preserve=mode,ownership"])
            .arg(built.join("."))
            .arg(out)
            .status()
            .is_ok_and(|status| status.success());
    if copied {
        0
    } else {
        eprintln!(
            "nix-offload: couldn't copy {} to {}",
            built.display(),
            out.display()
        );
        1
    }
}

/// The files of a folder in a git tree, NUL-separated, that `path:` reads
/// and its git doesn't ignore.
const UNIGNORED: &[&str] = &[
    "ls-files",
    "-z",
    "--cached",
    "--others",
    "--exclude-standard",
];

/// A flake that's a folder here, sent to the builder as a tarball and
/// built there as a `path:` flake, its inputs fetched there: `whole`, the
/// folder as `path:` reads it, but its git history, any result links and
/// what its git tree ignores, and otherwise the files its git tree
/// tracks, as Nix reads a git tree's flake. It costs the folder's size,
/// 4.7 MB for a checkout of this repository, so a commit GitHub has is
/// sent by name instead, and its size is said before it goes. Ignored
/// files stay here even where `path:` would read them: a Rust tree's
/// target folder held 793 MB on 30 October, which no build reads and
/// whose upload took longer than the build.
fn packed(folder: &Path, asked: &Asked, scratch: &Path, whole: bool) -> Result<Sending, Untaken> {
    fs::create_dir_all(scratch)
        .map_err(|error| Untaken::Waits(format!("{}: {error}", scratch.display())))?;
    let tarball = mine(scratch, "folder.tgz");
    let mut tar = Command::new("tar");
    tar.arg("-czf").arg(&tarball).arg("-C").arg(folder);
    let listing: &[&str] = if whole {
        UNIGNORED
    } else {
        &["ls-files", "-z"]
    };
    let packed = match git(folder, listing, None) {
        Some(files) => {
            // A tracked file deleted and not yet committed is listed but
            // gone, which a build of the folder doesn't read either.
            tar.args(["--ignore-failed-read", "--null", "-T", "-"])
                .stdin(Stdio::piped());
            tar.spawn().and_then(|mut packing| {
                if let Some(mut input) = packing.stdin.take() {
                    std::io::Write::write_all(&mut input, files.as_bytes())?;
                }
                packing.wait()
            })
        }
        None => tar
            .args(["--exclude=./.git", "--exclude=./result*", "."])
            .status(),
    };
    if !packed.is_ok_and(|status| status.success()) {
        let _ = fs::remove_file(&tarball);
        return Err(Untaken::Waits(format!(
            "{} couldn't be packed",
            folder.display()
        )));
    }
    let size = fs::metadata(&tarball).map_or(0, |file| file.len());
    eprintln!(
        "nix-offload: sending {} as a tarball of {}",
        folder.display(),
        sized(size)
    );
    let into = far_folder();
    let command = format!(
        "set -e; rm -rf {into}; mkdir -p {into}; trap 'rm -rf {into}' EXIT; \
         tar -xzf - -C {into}; {}",
        asked.command_in(&format!("path:{into}"), false)
    );
    Ok(Sending {
        command,
        bundle: Some(tarball),
    })
}

/// A folder on the builder named for this run's part, gone when its shell
/// ends: a run's parts go at once, and two may go to one builder when a
/// second VM builder doesn't come.
fn far_folder() -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!(
        "/tmp/nix-offload-there-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

/// What the lent builder is given for a build: the command its shell
/// runs, and a file sent on its standard input, a bundle of commits it
/// lacks or a tarball of a folder.
pub struct Sending {
    pub command: String,
    pub bundle: Option<PathBuf>,
}

/// How a flake that's a git tree here reaches the builder: as a commit
/// GitHub has, or as the commits since the newest one it has, bundled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    Pushed {
        /// The repository on GitHub, `owner/name`.
        github: String,
        commit: String,
        /// The flake's folder in the repository, when not its top.
        dir: Option<String>,
    },
    Bundled {
        /// The repository on GitHub, `owner/name`.
        github: String,
        base: String,
        commit: String,
        /// The flake's folder in the repository, when not its top.
        dir: Option<String>,
        bundle: PathBuf,
        /// The ref the bundle carries `commit` under, one per run.
        tip: String,
    },
}

impl Source {
    fn sending(self, asked: &Asked) -> Sending {
        match self {
            Source::Pushed {
                github,
                commit,
                dir,
            } => Sending {
                command: asked.command_of(&format!(
                    "github:{github}/{commit}{}",
                    dir.as_deref()
                        .map(|dir| format!("?dir={dir}"))
                        .unwrap_or_default()
                )),
                bundle: None,
            },
            Source::Bundled {
                github,
                base,
                commit,
                dir,
                bundle,
                tip,
            } => {
                let into = far_folder();
                let flake = format!(
                    "git+file://{into}?rev={commit}{}",
                    dir.map(|dir| format!("&dir={dir}")).unwrap_or_default()
                );
                // A builder's start-up installs git, and one whose
                // install failed borrows it from Nix.
                let command = format!(
                    "set -e; rm -rf {into}; mkdir -p {into}; trap 'rm -rf {into}' EXIT; \
                     cd {into}; cat > there.bundle; \
                     command -v git >/dev/null || \
                     git() {{ nix shell nixpkgs#git -c git \"$@\"; }}; \
                     git init -q; git fetch -q https://github.com/{github} {base}; \
                     git fetch -q there.bundle {tip}:refs/heads/there; {}",
                    asked.command_of(&flake)
                );
                Sending {
                    command,
                    bundle: Some(bundle),
                }
            }
        }
    }
}

/// What `there --takes` says it takes: a flake that's a git tree here,
/// `--apply`, `--out`, waiting for a builder it asks for, a flake that's a
/// folder no commit names, `--changed`, changed-checks run elsewhere,
/// `--here`, a build no builder took made here, `--here-when-idle`, one
/// made here while this machine is idle, `--print-out-paths`,
/// what was built brought into this store, derivations' paths,
/// `--outlive`, a build that goes on once its caller is gone, `--bulk`,
/// one whose VM tests let others' go first, `--never-ask`, a run
/// that asks the lender for no builder, `--first`, one that goes ahead of
/// every other run waiting for room or for a turn, `--run`, a command
/// run on a lent builder (`command.rs`), and `--lent-only`, a run that
/// goes to a lent builder or nowhere ([`LENT_ONLY`]).
pub const TAKES: [&str; 16] = [
    "tree",
    "apply",
    "out",
    "wants",
    "folder",
    "changed",
    "here",
    "here-when-idle",
    "print",
    "derivation",
    "outlive",
    "bulk",
    "never-ask",
    "first",
    "run",
    "lent-only",
];

/// The variable, with its value, that marks a run sent first, as
/// `there --first` sets it: the net's runs are, since on 8 November it
/// tested nothing for an hour while sessions' runs held the builders.
pub const FIRST: (&str, &str) = ("NIX_OFFLOAD_FIRST", "1");

/// Whether this run was sent first.
pub fn first() -> bool {
    std::env::var_os(FIRST.0).is_some_and(|value| value == FIRST.1)
}

/// The variable, with its value, that keeps a run to the lent builders, as
/// `there --lent-only` sets it for everything the run starts.
///
/// A fenced session's runs go out as the person, and a nearby machine, an
/// evaluator or this one would build or evaluate as them, so such a run
/// goes to a lent builder or nowhere. It sends a git tree's HEAD alone,
/// as `--run` does ([`Sends::Committed`]), never a tarball of its files,
/// nor anything the flake takes from this machine alone, and never builds
/// here.
pub const LENT_ONLY: (&str, &str) = ("NIX_OFFLOAD_LENT_ONLY", "1");

/// Whether this run goes to the lent builders alone.
pub fn lent_only() -> bool {
    std::env::var_os(LENT_ONLY.0).is_some_and(|value| value == LENT_ONLY.1)
}

/// Where a run waiting for a VM builder's room stands among the others:
/// one sent first ahead of every other, and a bulk one behind them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rank {
    First,
    Plain,
    Bulk,
}

/// A ref for one bundle to carry its commit under, here only while it's
/// made. Worktrees share their repository's refs, so runs from two of them
/// sharing one name would each bundle whatever commit the other last put
/// there, as happened on 5 November; each is named for its run instead.
fn bundled() -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!(
        "refs/nix-offload/there-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

/// The GitHub repository a remote's URL names, `owner/name`.
fn github_of(url: &str) -> Option<String> {
    let path = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("git@github.com:"))
        .or_else(|| url.strip_prefix("ssh://git@github.com/"))?;
    let path = path.trim_end_matches('/').trim_end_matches(".git");
    (path.split('/').count() == 2 && !path.starts_with('/')).then(|| path.to_owned())
}

/// What the caller's environment may say of where a repository's files
/// and objects are, none of which git is let read here: `top` alone names
/// the repository.
const ELSEWHERE: [&str; 6] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_COMMON_DIR",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
];

/// git's answer, trimmed, in `top`, or none when it fails.
///
/// A run sent from a fenced session reads a tree the fence lets it write,
/// so nothing in it may start a program or reach outside it: no hook,
/// such as the reference transaction `update-ref` runs, no file system
/// monitor, which a diff would start, no replaced object and none from a
/// caller's alternates. Nor is a missing object fetched: a partial clone's
/// promisor remote, which the tree's config names, would run its transport
/// as the person.
fn git(top: &Path, arguments: &[&str], index: Option<&Path>) -> Option<String> {
    git_with(top, arguments, index, &[])
}

/// [`git`], with `vars` set as well.
pub fn git_with(
    top: &Path,
    arguments: &[&str],
    index: Option<&Path>,
    vars: &[(&str, &str)],
) -> Option<String> {
    let mut command = Command::new("git");
    command
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "core.fsmonitor=false",
        ])
        .arg("-C")
        .arg(top)
        .args(arguments)
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("GIT_NO_LAZY_FETCH", "1")
        .envs(vars.iter().copied());
    for name in ELSEWHERE {
        command.env_remove(name);
    }
    if let Some(index) = index {
        command.env("GIT_INDEX_FILE", index);
    }
    let output = command.stderr(Stdio::null()).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// How the flake in `folder` reaches the builder, at the commit it was
/// `pinned` at, else at the one [`committed`] makes of it now. The newest
/// commit GitHub has is where it meets its upstream branch, else origin's
/// main. A tree that isn't git, or whose origin isn't on GitHub, can't
/// reach it this way; `--run` sends the latter's files ([`for_run`]).
pub fn source(
    folder: &Path,
    scratch: &Path,
    pinned: Option<&str>,
    sends: Sends,
) -> Result<Source, String> {
    let folder =
        fs::canonicalize(folder).map_err(|error| format!("{}: {error}", folder.display()))?;
    let not = |what: &str| format!("{} {what}", folder.display());
    let top = git(&folder, &["rev-parse", "--show-toplevel"], None)
        .map(PathBuf::from)
        .ok_or_else(|| not("isn't in a git tree"))?;
    let github = git(&top, &["remote", "get-url", "origin"], None)
        .as_deref()
        .and_then(github_of)
        .ok_or_else(|| not("has no origin on GitHub"))?;
    let dir = folder
        .strip_prefix(&top)
        .ok()
        .map(|dir| dir.to_string_lossy().into_owned())
        .filter(|dir| !dir.is_empty());
    let commit = match (pinned, sends) {
        (Some(commit), _) => commit.to_owned(),
        (None, Sends::Changes) => committed(&top, scratch).map_err(|why| not(&why))?,
        (None, Sends::Committed) => head_alone(&top, scratch).map_err(|why| not(&why))?,
    };
    let base = ["@{upstream}", "origin/main"]
        .iter()
        .find_map(|theirs| git(&top, &["merge-base", &commit, theirs], None))
        .ok_or_else(|| not("shares no commit with origin"))?;
    sourced(&top, github, commit, dir, base, scratch).map_err(not)
}

/// Where the flake in `folder` is pinned now: its commit, as [`source`]
/// makes one, and where its pass is kept when it had no changes.
fn pinned_at(folder: &Path, scratch: &Path) -> Option<Pinned> {
    let folder = fs::canonicalize(folder).ok()?;
    let top = PathBuf::from(git(&folder, &["rev-parse", "--show-toplevel"], None)?);
    let commit = committed(&top, scratch).ok()?;
    let tree = git(&top, &["rev-parse", &format!("{commit}^{{tree}}")], None)?;
    let at = clean_at(&folder).filter(|at| at.commit == commit);
    Some(Pinned { commit, tree, at })
}

/// The tree of files the flake in `folder` would be committed as now.
fn tree_now(folder: &Path, scratch: &Path) -> Option<String> {
    let top = PathBuf::from(git(folder, &["rev-parse", "--show-toplevel"], None)?);
    let commit = committed(&top, scratch).ok()?;
    git(&top, &["rev-parse", &format!("{commit}^{{tree}}")], None)
}

/// What [`source`] sends of a tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sends {
    /// Its HEAD, with its changes as one more commit, as Nix reads a tree.
    Changes,
    /// Its HEAD alone, refused while it has changes, for a command run as
    /// the person from a session fenced off from much of what they own.
    Committed,
}

/// The empty tree's hash, which git takes its attributes from, rather than
/// the tree's own, when `GIT_ATTR_SOURCE` names it.
pub(crate) const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

/// The HEAD of the git tree `top`, refused while it has changes or borrows
/// objects from another repository.
///
/// `there --run` runs as the person, out of a fence that hides some of
/// what they own from the session asking, and on 11 November a tree that
/// session could write was shown to send more than its own commits. Its
/// index and files went into the commit, so a gitlink whose `.git` names
/// a hidden repository had that repository's HEAD read through, and its
/// attributes picked filters git ran outside the fence. So only HEAD's
/// objects go: a gitlink in them is a hash the builder never fetches, the
/// diff that finds changes reads no attribute, no submodule and none of
/// the repository's config, in a repository of the run's own in `scratch`,
/// and a tree with alternates, whose objects could be any repository's, is
/// refused.
fn head_alone(top: &Path, scratch: &Path) -> Result<String, String> {
    let borrows = ["objects/info/alternates", "objects/info/http-alternates"]
        .iter()
        .any(|one| {
            git(top, &["rev-parse", "--git-path", one], None)
                .is_none_or(|path| top.join(path).exists())
        });
    if borrows {
        return Err(
            "borrows objects from another repository, so --run sends nothing of it".to_owned(),
        );
    }
    let head = git(top, &["rev-parse", "--verify", "HEAD^{commit}"], None)
        .ok_or_else(|| "has no commit".to_owned())?;
    if archived::unchanged(top, &head, scratch)? {
        Ok(head)
    } else {
        Err(
            "has changes not committed, and --run sends only what is, so commit them first"
                .to_owned(),
        )
    }
}

/// How `--run` sends the tree at `folder`: by [`source`] while its origin
/// is on GitHub, else as HEAD's files (`archived.rs`), since a lent
/// builder holds no credential to fetch from anywhere else.
#[derive(Debug)]
pub enum ForRun {
    Git(Source),
    Archived(archived::Archived),
}

impl ForRun {
    /// Where a pass of the run is kept, what the far side's shell runs to
    /// check it out, and what that reads from its standard input. No pass
    /// is kept from files the session's own cones cut, since a gate taking
    /// it would take a pass of the whole commit.
    pub fn sent(self) -> (Option<passed::At>, String, Option<PathBuf>) {
        match self {
            Self::Git(source) => {
                let at = at_of(&source);
                let (checkout, bundle) = crate::changed::checked_out(source);
                (at, checkout, bundle)
            }
            Self::Archived(archived) => {
                let (checkout, archive) = archived::unpacked(archived);
                (None, checkout, Some(archive))
            }
        }
    }
}

/// How `--run` sends the tree at `folder`, any archive made in `scratch`.
pub fn for_run(folder: &Path, scratch: &Path) -> Result<ForRun, String> {
    let canonical =
        fs::canonicalize(folder).map_err(|error| format!("{}: {error}", folder.display()))?;
    let not = |what: &str| format!("{} {what}", canonical.display());
    let top = git(&canonical, &["rev-parse", "--show-toplevel"], None)
        .map(PathBuf::from)
        .ok_or_else(|| not("isn't in a git tree"))?;
    its_own(&canonical, &top).map_err(not)?;
    let on_github = git(&top, &["remote", "get-url", "origin"], None)
        .as_deref()
        .and_then(github_of)
        .is_some();
    if on_github {
        return source(folder, scratch, None, Sends::Committed).map(ForRun::Git);
    }
    let commit = head_alone(&top, scratch).map_err(|why| not(&why))?;
    let dir = canonical
        .strip_prefix(&top)
        .ok()
        .map(|dir| dir.to_string_lossy().into_owned())
        .filter(|dir| !dir.is_empty());
    let archived = archived::packed(&top, &commit, dir, scratch, archived::LARGEST)
        .map_err(|why| not(&why))?;
    let size = fs::metadata(&archived.archive).map_or(0, |file| file.len());
    eprintln!(
        "nix-offload: sending HEAD of {}, whose origin isn't on GitHub, as a tarball of {}; \
         no pass is kept from a run sent as files",
        canonical.display(),
        sized(size)
    );
    Ok(ForRun::Archived(archived))
}

/// Whether `top`, which git found for `canonical`, is that tree's own
/// repository. A fenced session can write its repository's config, and a
/// linked worktree's `commondir`, so either could name another repository
/// it can't read, whose files `--run` would then send as the person:
/// `core.worktree` makes git's top any folder, and `commondir` takes the
/// objects and index from any git folder. So `--run` is refused a tree
/// outside its top, one whose config sets `core.worktree`, a top whose git
/// folder isn't the tree's own, and a git folder that is neither the common
/// one, as the top's `.git` folder, nor one of its worktrees whose `gitdir`
/// names the top's `.git` file: a `.git` file can name any repository's git
/// folder, or a worktree of it that the session didn't make. Each is asked
/// from the tree itself, since from a top elsewhere git would read that
/// repository's config instead. A submodule's git folder sets
/// `core.worktree`, so `--run` is refused one.
fn its_own(canonical: &Path, top: &Path) -> Result<(), &'static str> {
    if !canonical.starts_with(top) {
        return Err("isn't within the repository git finds for it");
    }
    if git(canonical, &["config", "--get", "core.worktree"], None).is_some() {
        return Err("has a repository whose config names its files elsewhere");
    }
    let at = |from: &Path, what: &str| {
        git(from, &["rev-parse", "--path-format=absolute", what], None)
            .and_then(|dir| fs::canonicalize(dir).ok())
    };
    let (Some(dir), Some(common)) = (
        at(canonical, "--git-dir"),
        at(canonical, "--git-common-dir"),
    ) else {
        return Err("has no git folder that can be found");
    };
    if at(top, "--git-dir").as_ref() != Some(&dir) {
        return Err("isn't within the repository git finds for it");
    }
    let dotgit = fs::canonicalize(top.join(".git")).ok();
    let worktree = dir.parent().filter(|parent| parent.ends_with("worktrees"));
    let named = || {
        fs::read_to_string(dir.join("gitdir"))
            .ok()
            .and_then(|named| fs::canonicalize(dir.join(named.trim())).ok())
    };
    let own = if dir == common {
        dotgit.as_ref() == Some(&dir)
    } else {
        worktree.and_then(Path::parent) == Some(common.as_path())
            && dotgit.is_some()
            && named() == dotgit
    };
    if own {
        Ok(())
    } else {
        Err("has a git folder whose objects are another repository's")
    }
}

/// The commit the git tree `top` is built at: its HEAD, or, with changes to
/// the files its index holds, staged new ones included, a commit of them
/// made without touching the branch or the index, as `nix build` of the
/// tree here would read them.
fn committed(top: &Path, scratch: &Path) -> Result<String, String> {
    if lent_only() {
        return head_alone(top, scratch);
    }
    let head = git(top, &["rev-parse", "HEAD"], None).ok_or("has no commit")?;
    if git(top, &["diff", "--quiet", "HEAD"], None).is_some() {
        Ok(head)
    } else {
        fs::create_dir_all(scratch)
            .map_err(|_| "has no scratch to commit its changes in".to_owned())?;
        let index = mine(scratch, "index");
        // Seeded from the tree's own index, as Nix reads a tree, so a file
        // staged, or added with -N, goes too; `add -u` then takes every
        // tracked file as it stands, and anything untracked stays here.
        let made = git(top, &["rev-parse", "--git-path", "index"], None)
            .map(|own| top.join(own))
            .filter(|own| fs::copy(own, &index).is_ok())
            .and_then(|_| git(top, &["add", "-u"], Some(&index)))
            .and_then(|_| git(top, &["write-tree"], Some(&index)))
            .and_then(|tree| {
                git(
                    top,
                    &[
                        "commit-tree",
                        &tree,
                        "-p",
                        &head,
                        "-m",
                        "nix-offload there: uncommitted changes",
                    ],
                    None,
                )
            });
        let _ = fs::remove_file(&index);
        made.ok_or_else(|| "has changes git couldn't commit".to_owned())
    }
}

/// How `commit` of the git tree `top` reaches the builder: by its commit
/// once GitHub has it, as `base` is, else as a bundle over `base`.
fn sourced(
    top: &Path,
    github: String,
    commit: String,
    dir: Option<String>,
    base: String,
    scratch: &Path,
) -> Result<Source, &'static str> {
    if base == commit {
        return Ok(Source::Pushed {
            github,
            commit,
            dir,
        });
    }
    let bundle = mine(scratch, "bundle");
    fs::create_dir_all(scratch).map_err(|_| "has no scratch to bundle it in")?;
    let tip = bundled();
    let referenced = git(top, &["update-ref", &tip, &commit], None).is_some();
    let bundled = referenced
        && git(
            top,
            &[
                "bundle",
                "create",
                "-q",
                &bundle.to_string_lossy(),
                &format!("^{base}"),
                &tip,
            ],
            None,
        )
        .is_some();
    let _ = git(top, &["update-ref", "-d", &tip], None);
    if !bundled {
        return Err("couldn't be bundled");
    }
    Ok(Source::Bundled {
        github,
        base,
        commit,
        dir,
        bundle,
        tip,
    })
}

/// How many seconds a run cut off from its caller goes on, waiting for the
/// same run to be sent again, before it ends. A lender restarting cuts
/// every ssh to its builder, and a run sends again once one stands, which
/// the lender's two minutes for a restart bound; five minutes covers that.
const GRACE: u32 = 300;

/// `command` as a far shell is given it, so it ends once its caller has
/// been gone for `GRACE` seconds with nothing sent again to claim it.
///
/// A shell ssh starts without a terminal hears nothing when its session
/// ends, and a build or an evaluation waiting on its builds never writes,
/// so on 1 November runs whose callers were a day gone still held builders.
/// sshd starts the shell as its session's process group, under a process
/// of the same account, so a watcher that sees no caller left ends the
/// whole group: the shell, what it started, and the watcher. Its streams go
/// nowhere, so it never holds the session open. Hung up or ended, the shell
/// exits, so the command's own EXIT trap still cleans up; the `exit` after
/// it keeps zsh from running its last command in the shell's own process,
/// which would leave nothing to run that trap. The command's code is kept
/// as `ended`, never `status`, which zsh holds read-only: a run sent to a
/// Mac ended as failed whatever its command did, so from 8 to 15 November
/// no fetch asked of one passed, and only a folder's builds went there.
///
/// Ending at once lost work: on 3 November a lender's restart cut a VM
/// test mid-compile, and the run sent again built linux from the start.
/// So each caller claims the run in a folder named for the command, which
/// is the same each time one run sends it and differs between runs, since
/// it names this machine's pid. The watcher ends the group only once no
/// claimer has lived for the grace period. The run sent again waits on
/// what the first is building through Nix's lock on each derivation.
/// Once the command itself has ended, the watcher ends too, as there's
/// nothing left to wait for, ending what the command left. However the
/// shell ends, whether the command returns, calls exit, fails under
/// `set -e` or is hung up, it takes its own claim on the way out, and the
/// folder once no other claims it, keeping the command's status: a claim
/// left behind names a pid that the machine gives out again, and on 18
/// November three of one Mac's 516 such claims named its daemons, so a
/// watcher would hold that run for callers long gone.
pub fn far(command: &str) -> String {
    far_within(
        command,
        &format!("${{NIX_OFFLOAD_GRACE:-{GRACE}}}"),
        Far::Asks,
    )
}

/// `command` as `far` sends it, saying as it starts and as it ends which
/// machine it runs on ([`Far::Runs`]): for a build or a run a person
/// reads, where `far` is for the asks a lender makes every minute.
pub fn far_saying_where(command: &str) -> String {
    far_within(
        command,
        &format!("${{NIX_OFFLOAD_GRACE:-{GRACE}}}"),
        Far::Runs,
    )
}

/// `command` as `far_saying_where` sends it, ending once its caller has
/// been gone 15 seconds, or `NIX_OFFLOAD_RUN_GRACE`'s: a command run with
/// `--run` builds nothing another run could wait on, so there's nothing to
/// keep it for, and a suite left running holds a builder others wait for.
/// A lost link reattaches by sending again, so each reattach says where
/// the run is too.
pub fn far_brief(command: &str) -> String {
    far_within(command, "${NIX_OFFLOAD_RUN_GRACE:-15}", Far::Runs)
}

/// What a command sent far is, for what its shell says of itself.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Far {
    /// An ask, as a lender's of its builder's load, which says nothing.
    Asks,
    /// A build or a run, which says as it starts and as it ends which
    /// machine it runs on, by that machine's own name: a lender's builder
    /// lines name its alias, which each new instance takes over, so a log
    /// that names only the alias can't say which instance a run was on.
    Runs,
    /// A run as `Runs` says, that runs to its end even once every caller
    /// is gone ([`outliving`]).
    Outlives,
}

/// What a run's far shell says as it starts, to its stderr: the machine's
/// own name, which on a lent builder is its instance's. It's said in a
/// shell of its own, as the end is, so a caller already gone ends that
/// shell rather than the one about to run the command.
const RUNNING_ON: &str = "(echo \"nix-offload: running on $(uname -n)\" >&2);";

/// What it says as it ends, in its `EXIT` trap, with the status it ends
/// with. It's said in a shell of its own, so a caller gone, whose stderr
/// no longer takes it, ends that shell rather than the trap.
const RAN_ON: &str = "(echo \"nix-offload: ran on $(uname -n), ending $ended\" >&2);";

/// How long, at most, a run asked to outlive its callers goes on alone: as
/// long as a builder lives.
pub const OUTLIVE: u32 = 12 * 3600;

/// A command sent far that runs to its end even once every caller is gone,
/// as a prebuild asks: one whose caller hit its own time limit at 05:14 on
/// 3 November lost the kernel it was compiling. While it runs, a file named
/// for its shell says so, which `OUTLIVING_ASKED` reads, and the lender
/// holds the builder meanwhile, whoever asked.
pub fn outliving(command: &str) -> String {
    far_within(
        command,
        &format!("${{NIX_OFFLOAD_OUTLIVE:-{OUTLIVE}}}"),
        Far::Outlives,
    )
}

/// What a builder is sent to ask whether a run outliving its callers still
/// runs there: it prints `outliving` for each whose shell lives. A file
/// left by one killed outright names a shell that's gone, so it counts for
/// nothing.
pub const OUTLIVING_ASKED: &str = "for run in \"${TMPDIR:-/tmp}\"/nix-offload-outliving-*; do \
     [ -e \"$run\" ] && kill -0 \"${run##*-}\" 2>/dev/null && echo outliving; done; true";

/// Whether a run outliving its callers still runs on the builder `store`
/// names, as the lender asks before giving back one nothing here reaches.
/// A builder that can't be asked holds none.
pub fn outlives_on(store: &str, scratch: &Path, ssh: &str) -> bool {
    let Some(lent) = store.lines().next().and_then(|line| builder(line.trim())) else {
        return false;
    };
    let asking = Sending {
        command: OUTLIVING_ASKED.to_owned(),
        bundle: None,
    };
    printed_on(&lent, &asking, scratch, ssh).is_ok_and(|(ran, printed)| {
        ran.status == 0 && printed.lines().any(|line| line == "outliving")
    })
}

/// `far`'s wrapper, ending what's left once no caller has lived for `grace`
/// seconds, as the far shell reads them. A run, as `far_saying_where`
/// makes it, says where it runs, and one that outlives its callers names itself for
/// `OUTLIVING_ASKED` while it runs.
fn far_within(command: &str, grace: &str, far: Far) -> String {
    let named_for = crate::running::stable_digest(&[command.as_bytes()]);
    let claims = format!("${{TMPDIR:-/tmp}}/nix-offload-run-{named_for:016x}");
    let (running_on, ran_on) = if far == Far::Asks {
        ("", "")
    } else {
        (RUNNING_ON, RAN_ON)
    };
    let (named, unnamed) = if far == Far::Outlives {
        (
            "outliving=\"${TMPDIR:-/tmp}/nix-offload-outliving-$$\"; : > \"$outliving\"; ",
            "rm -f \"$outliving\"; ",
        )
    } else {
        ("", "")
    };
    // A run ending as another sends it again can take the folder between
    // the new one's mkdir and its claim, so a claim that finds no folder
    // is made once more.
    let claim = "mkdir -p \"$claims\" && : > \"$claims/$PPID\"";
    // What says the builder is full holds the shell's stderr, and reads a
    // shell its caller hasn't reaped yet as still there, so a caller
    // reading to the end before it reaps would wait on it for ever: the
    // shell ends it on the way out.
    format!(
        "{named}claims=\"{claims}\"; {{ {claim}; }} 2>/dev/null || {{ {claim}; }}; \
         (gone=0; while [ \"$gone\" -lt \"{grace}\" ]; do sleep 1; \
         kill -0 $$ 2>/dev/null || break; \
         held=; for caller in $(ls \"$claims\" 2>/dev/null); do \
         if kill -0 \"$caller\" 2>/dev/null; then held=1; else rm -f \"$claims/$caller\"; fi; \
         done; if [ -n \"$held\" ]; then gone=0; else gone=$((gone + 1)); fi; done; \
         {unnamed}rmdir \"$claims\" 2>/dev/null; kill -TERM 0) \
         </dev/null >/dev/null 2>&1 & \
         {ROOM_SAID} saying=$!; \
         nix_offload_given() {{ kill $saying 2>/dev/null; {unnamed}rm -f \"$claims/$PPID\"; rmdir \"$claims\" 2>/dev/null; }}; \
         trap 'ended=$?; nix_offload_given || :; {ran_on} exit $ended' EXIT; \
         trap 'exit 143' HUP TERM; {running_on} {command}"
    )
}

/// How many builds sent to a machine nearby it runs at once. On 16
/// November a Mac on 12 processors ran five at once, four of them sent
/// by different runs, at load 75 to 94, and lost its DNS: Nix holds each
/// client to its own `max-jobs`, and the Mac's load was read once for
/// each run, before any of them had started.
pub const NEARBY_AT_ONCE: u32 = 2;

/// What a shell runs to name the boot it runs in: Linux's boot id, or a
/// Mac's boot session's. Both are fixed for the whole boot, where a Mac's
/// boot time moves when its clock is stepped.
const BOOT_READ: &str = "cat /proc/sys/kernel/random/boot_id 2>/dev/null || \
     PATH=\"$PATH:/usr/sbin\" sysctl -n kern.bootsessionuuid 2>/dev/null";

/// What a shell runs to name the folder its machine's turns are in, so a
/// run taking a turn and one counting them read the same.
const TURNS_READ: &str = "turns=\"${NIX_OFFLOAD_TURNS:-$HOME/.cache/nix-offload/turns}\";";

/// `command`, run once it holds one of [`NEARBY_AT_ONCE`] turns on the
/// machine it's sent to, however many runs from however many machines
/// send it builds. Each turn's run builds up to the machine's own
/// `max-jobs` at once. A turn is a link in `NIX_OFFLOAD_TURNS`, or
/// `~/.cache/nix-offload/turns`, so one account's, naming the shell that
/// holds it and the boot it was taken in, made with `ln -s` since that
/// fails if the link is already there. The shell gives it back as it
/// ends, unless another run has taken it since. One whose shell is gone,
/// killed outright or in an earlier boot, or that names no shell or a
/// pid of 8 digits or more, past Linux's highest and a Mac's, which zsh
/// can read as -1 and so as live, is taken by the next to look. A run
/// waiting says so at once and about every 5 minutes, or
/// `NIX_OFFLOAD_TURN_EVERY` seconds, naming the shells that hold the
/// turns, so it isn't read as hung and a turn held by a pid reused since
/// can be found. A turn
/// that can't be made, as a folder in its place or a dead one in a folder
/// that can't be written, is passed over for the others;
/// where none can be on two looks a second apart, since a dead turn
/// another run is taking back at that moment reads as one that can't be,
/// or the boot can't be read to name one by, a run builds without one and
/// says so, rather than wait for a turn that can never be had. Taking a
/// dead turn back is a look, then a remove, so a run can remove a turn
/// another took between the two: each run looks a second after taking its
/// turn, or `NIX_OFFLOAD_TURN_LOOK` seconds (a second where `sleep`
/// refuses that), and if it no longer holds it looks for a turn again as
/// long after. Only a run stalled longer than that look between its look
/// and its remove lets an extra build in, until the one whose link it
/// replaced ends; it never leaves a run waiting for a turn nobody holds.
/// A turn taken from it by a run in another boot shows the folder is
/// shared by two boots, as a home two machines share makes it, where each
/// reads the other's turns as dead and the turns bound nothing, so the run
/// builds without one and says so rather than take turns back from the
/// other boot for ever. It bounds only runs that reach the machine as one
/// account in one pid namespace: two namespaces of one boot sharing a home
/// read each other's turns as dead too, and nothing here tells them apart.
pub fn in_turn_nearby(command: &str) -> String {
    format!(
        "{TURNS_READ} \
         boot=$({BOOT_READ}); mine=\"$$ $boot\"; \
         here=$(uname -n); turn=; waited=0; unmadebefore=; \
         trap 'k=1; while [ \"$k\" -le {NEARBY_AT_ONCE} ]; do \
         if [ \"$(readlink \"$turns/$k\" 2>/dev/null)\" = \"$mine\" ]; then \
         rm -f \"$turns/$k\"; fi; \
         k=$((k + 1)); done' EXIT; \
         if [ -z \"$boot\" ]; then turn=none; fi; \
         while [ -z \"$turn\" ]; do mkdir -p \"$turns\" 2>/dev/null; n=1; unmade=0; holders=; \
         while [ \"$n\" -le {NEARBY_AT_ONCE} ]; do \
         if [ -d \"$turns/$n\" ] && [ ! -L \"$turns/$n\" ]; then \
         unmade=$((unmade + 1)); n=$((n + 1)); continue; fi; \
         if ln -s \"$mine\" \"$turns/$n\" 2>/dev/null; then turn=\"$turns/$n\"; break; fi; \
         holder=$(readlink \"$turns/$n\" 2>/dev/null); \
         if [ -z \"$holder\" ]; then \
         if ln -s \"$mine\" \"$turns/$n\" 2>/dev/null; then turn=\"$turns/$n\"; break; fi; \
         holder=$(readlink \"$turns/$n\" 2>/dev/null); \
         if [ -z \"$holder\" ]; then unmade=$((unmade + 1)); n=$((n + 1)); continue; fi; fi; \
         pid=${{holder%% *}}; case \"$pid\" in ''|*[!0-9]*|0*|????????*) pid=; esac; \
         if [ -z \"$pid\" ] || [ \"${{holder#* }}\" != \"$boot\" ] || [ \"$pid\" = \"$$\" ] || \
         ! kill -0 \"$pid\" 2>/dev/null; then \
         if rm -f \"$turns/$n\" 2>/dev/null; then continue; fi; unmade=$((unmade + 1)); \
         else holders=\"${{holders:+$holders, }}$pid\"; fi; \
         n=$((n + 1)); done; \
         if [ -n \"$turn\" ]; then sleep \"${{NIX_OFFLOAD_TURN_LOOK:-1}}\" 2>/dev/null || sleep 1; \
         taker=$(readlink \"$turn\" 2>/dev/null); \
         if [ \"$taker\" = \"$mine\" ]; then break; fi; \
         if [ -n \"$taker\" ] && [ \"${{taker#* }}\" != \"$boot\" ]; then turn=shared; break; fi; \
         turn=; unmadebefore=; sleep \"${{NIX_OFFLOAD_TURN_LOOK:-1}}\" 2>/dev/null || sleep 1; \
         continue; fi; \
         if [ \"$unmade\" -ge {NEARBY_AT_ONCE} ]; then \
         if [ -n \"$unmadebefore\" ]; then turn=none; break; fi; \
         unmadebefore=1; sleep 1; continue; fi; \
         unmadebefore=; \
         if [ $((waited % ${{NIX_OFFLOAD_TURN_EVERY:-300}})) -eq 0 ]; then also=; \
         if [ \"$unmade\" -gt 0 ]; then also=\", with $unmade more it can't make\"; fi; \
         echo \"nix-offload: $here already runs as many builds sent to it as it takes at once, \
         so this waits there for its turn, held in $turns by shells $holders$also\" >&2; fi; \
         sleep 1; waited=$((waited + 1)); done; \
         if [ -z \"$boot\" ]; then echo \"nix-offload: the boot of $here can't be read from \
         its boot_id or kern.bootsessionuuid, so this builds without a turn\" >&2; \
         elif [ \"$turn\" = none ]; then echo \"nix-offload: $turns on $here can't hold its turns, \
         so this builds without one\" >&2; \
         elif [ \"$turn\" = shared ]; then echo \"nix-offload: $turns on $here is shared \
         with another boot, which took this run's turn, so the turns can't bound its builds \
         and this builds without one; NIX_OFFLOAD_TURNS naming a folder of each machine's \
         own bounds them\" >&2; fi; {command}"
    )
}

/// What a run sent far says every `NIX_OFFLOAD_ROOM_EVERY` seconds, 300 by
/// default, while its builder runs as many builds as it runs at once, so a
/// run waiting there for room is never read as a hung one: on 8 November
/// the net's VM test said nothing for 30 minutes on a busy VM builder and
/// held main as a stall. A builder with room says nothing, so a hung build
/// stays silent. Builds are counted by their users, a build user's or an
/// allocated range of 65536 from Nix's first, and nothing is said where
/// either can't be read.
const ROOM_SAID: &str = "(said=0; while kill -0 $$ 2>/dev/null; do sleep 1; \
     said=$((said + 1)); [ \"$said\" -lt \"${NIX_OFFLOAD_ROOM_EVERY:-300}\" ] && continue; \
     said=0; most=$(nix config show max-jobs 2>/dev/null); \
     running=$(ps -eo uid= 2>/dev/null | awk '$1 >= 872415232 \
     { print int(($1 - 872415232) / 65536) } $1 > 30000 && $1 < 31000 { print \"b\" $1 }' \
     | sort -u | wc -l); \
     if [ -n \"$most\" ] && [ \"$running\" -ge \"$most\" ] 2>/dev/null; then \
     echo \"nix-offload: the builder runs $running builds, as many as it runs at once, \
     so this may wait there for room\" >&2; fi; done) </dev/null >/dev/null &";

/// ssh to `host`, never asking anything, logging where `said` names, whose
/// verbose log ends with what it sent, which is what a build put on the
/// connection. It trusts only a host key it already holds, as every Nix
/// copy's ssh does ([`crate::asking::NEVER_ASKS`]), so a nearby machine
/// whose key isn't held is refused here, before anything is built on it,
/// not at the copy back.
pub fn reaching(ssh: &str, said: &Path) -> Command {
    let mut command = Command::new(ssh);
    command
        .args(["-o", "BatchMode=yes", "-o", "StrictHostKeyChecking=yes"])
        .args(["-o", "LogLevel=VERBOSE"])
        // A tunnel's connection can die with nothing on either end
        // closing it, and then ssh waits on it forever: on 30 October it
        // waited 30 minutes under a build, saying nothing. Unanswered for
        // a minute, it's lost, which a lent builder's run sends again.
        .args([
            "-o",
            "ServerAliveInterval=15",
            "-o",
            "ServerAliveCountMax=4",
        ])
        .arg("-E")
        .arg(said);
    command
}

/// A size in bytes as a person reads it, to a tenth of its unit.
pub fn sized(bytes: u64) -> String {
    let mut scale = 1;
    for unit in ["KB", "MB", "GB"] {
        scale *= 1000;
        if bytes < scale * 1000 || unit == "GB" {
            let tenths = bytes / (scale / 10);
            if bytes >= scale {
                return format!("{}.{} {unit}", tenths / 10, tenths % 10);
            }
            break;
        }
    }
    format!("{bytes} bytes")
}

/// A file of this run's own in `scratch`.
pub fn mine(scratch: &Path, name: &str) -> PathBuf {
    scratch.join(format!("{name}-{}", std::process::id()))
}

/// What comes back from a build elsewhere besides its log and status.
#[derive(Clone, Copy)]
pub enum Bring<'a> {
    Nothing,
    /// Its output, unpacked into this folder.
    Folder(&'a Path),
    /// Its output paths, copied into this machine's store by this nix,
    /// their signatures taken so.
    Store(&'a str, Signatures),
}

impl<'a> Bring<'a> {
    /// What `asked` wants back: its output in the folder it named, its
    /// output paths in this store through `nix`, taken by `signatures`,
    /// when it prints them, or else nothing.
    fn asked(asked: &'a Asked, nix: &'a str, signatures: Signatures) -> Self {
        match (&asked.out, asked.print) {
            (Some(out), _) => Bring::Folder(out),
            (None, true) => Bring::Store(nix, signatures),
            (None, false) => Bring::Nothing,
        }
    }
}

/// Whether what a builder built is checked for a signature Nix here
/// trusts as it comes back into this store.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Signatures {
    Checked,
    /// Taken as it comes, as only an account Nix here trusts may: what a
    /// lent builder built while its lender names no key it signs with.
    /// A machine nearby's work is never taken so; it comes back checked
    /// or not at all.
    Unchecked,
}

/// How what the lent builders build is taken, as the lender's folder
/// `dir` says in `builder-keys`: checked where it names the keys they
/// sign with, which Nix here trusts, and unchecked where it names none.
pub fn signatures(dir: &Path) -> Signatures {
    if fs::read_to_string(dir.join("builder-keys")).is_ok_and(|keys| !keys.trim().is_empty()) {
        Signatures::Checked
    } else {
        Signatures::Unchecked
    }
}

/// How long a checked copy back that failed waits before trying again,
/// unless `NIX_OFFLOAD_SIGNED_WITHIN` says otherwise in seconds. A lent
/// builder signs what it builds as it builds it, but a path it was sent
/// from here only as its push signs the closures it uploads, every 20 s.
const SIGNED_WITHIN: std::time::Duration = std::time::Duration::from_secs(30);

fn signed_within() -> std::time::Duration {
    std::env::var("NIX_OFFLOAD_SIGNED_WITHIN")
        .ok()
        .and_then(|seconds| seconds.parse().ok())
        .map_or(SIGNED_WITHIN, std::time::Duration::from_secs)
}

pub fn on(
    lent: &Builder,
    sending: &Sending,
    bring: Bring,
    scratch: &Path,
    ssh: &str,
) -> Result<Ran, String> {
    on_with(lent, sending, bring, scratch, ssh, far_saying_where)
}

/// Builds as `on` does, with the command sent as `wrap` makes it:
/// `far_saying_where`, or `outliving` for a run asked to outlive its
/// caller.
pub fn on_with(
    lent: &Builder,
    sending: &Sending,
    bring: Bring,
    scratch: &Path,
    ssh: &str,
    wrap: fn(&str) -> String,
) -> Result<Ran, String> {
    let (mut command, known, said) = reach(lent, scratch, ssh)?;
    // Named by its host, since the VM tests' builder is lent too: said
    // alike, a VM test timed out on its own builder read as one sent the
    // lent way.
    eprintln!("nix-offload: building on the lent builder {}", lent.host);
    command.arg(&lent.host).arg(wrap(&sending.command));
    // Ended with its run, as `captured_on`'s is: a run sent apart holds a
    // changed-checks turn too, and its ssh, waited on below in this same
    // thread, would otherwise keep the builder's shell, uncounted, after
    // the run that held the turn was killed.
    dies_with_run(&mut command);
    if let Some(bundle) = &sending.bundle {
        let bundle = fs::File::open(bundle).map_err(|error| format!("the bundle: {error}"))?;
        command.stdin(bundle);
    }
    let status = match bring {
        Bring::Nothing => command.status(),
        Bring::Folder(out) => unpacked_into(&mut command, out),
        Bring::Store(nix, signatures) => brought_from(&mut command, &lent.store, nix, signatures),
    };
    let sent = fs::read_to_string(&said).ok().and_then(|log| sent(&log));
    let _ = fs::remove_file(&known);
    let _ = fs::remove_file(&said);
    let status = status.map_err(|error| format!("couldn't run {ssh}: {error}"))?;
    Ok(Ran {
        on: Some(lent.host.clone()),
        // A build killed by a signal there comes back as ssh's status.
        status: status.code().unwrap_or(UNREACHED),
        sent,
    })
}

/// Runs a command on the lent builder as `on` does, bringing nothing back,
/// and gives what it printed rather than passing it on.
pub fn printed_on(
    lent: &Builder,
    sending: &Sending,
    scratch: &Path,
    ssh: &str,
) -> Result<(Ran, String), String> {
    captured_on(lent, sending, scratch, ssh, false)
}

/// Runs a command on the lent builder as `on` does, bringing nothing back,
/// passing on what it prints as it comes and giving it too.
pub fn teed_on(
    lent: &Builder,
    sending: &Sending,
    scratch: &Path,
    ssh: &str,
) -> Result<(Ran, String), String> {
    captured_on(lent, sending, scratch, ssh, true)
}

/// What `printed_on` and `teed_on` share: what's printed is kept, and
/// passed on a line at a time as it comes when `tee` says.
fn captured_on(
    lent: &Builder,
    sending: &Sending,
    scratch: &Path,
    ssh: &str,
    tee: bool,
) -> Result<(Ran, String), String> {
    use std::io::BufRead;
    let (mut command, known, said) = reach(lent, scratch, ssh)?;
    command
        .arg(&lent.host)
        // Only a run passed on as it comes says where it runs: one whose
        // print is kept is an ask, read by its caller, not a person.
        .arg(if tee {
            far_saying_where(&sending.command)
        } else {
            far(&sending.command)
        })
        .stdout(Stdio::piped());
    // A changed-checks turn (`changed/turns.rs`) is held for as long as
    // its run lives, and a run killed alone frees it at once: ended with
    // the run, this ssh leaves the builder's shell without a caller, so
    // `far`'s watcher ends it within its grace, where an ssh left running
    // would keep it, uncounted, for as long as the ssh lived.
    dies_with_run(&mut command);
    if let Some(bundle) = &sending.bundle {
        let bundle = fs::File::open(bundle).map_err(|error| format!("the bundle: {error}"))?;
        command.stdin(bundle);
    }
    let output = command.spawn().and_then(|mut running| {
        let mut printed = Vec::new();
        if let Some(out) = running.stdout.take() {
            let mut here = std::io::stdout();
            let mut out = std::io::BufReader::new(out);
            let mut line = Vec::new();
            while out.read_until(b'\n', &mut line)? > 0 {
                if tee {
                    let _ = here.write_all(&line).and_then(|()| here.flush());
                }
                printed.append(&mut line);
            }
        }
        let status = running.wait()?;
        Ok(std::process::Output {
            status,
            stdout: printed,
            stderr: Vec::new(),
        })
    });
    let sent = fs::read_to_string(&said).ok().and_then(|log| sent(&log));
    let _ = fs::remove_file(&known);
    let _ = fs::remove_file(&said);
    let output = output.map_err(|error| format!("couldn't run {ssh}: {error}"))?;
    let ran = Ran {
        on: Some(lent.host.clone()),
        status: output.status.code().unwrap_or(UNREACHED),
        sent,
    };
    Ok((ran, String::from_utf8_lossy(&output.stdout).into_owned()))
}

/// ssh to the lent builder with its key and host key, as every command
/// sent there goes, and the two files it writes in `scratch`, the host
/// key and ssh's log, to remove once it's run.
pub fn reach(
    lent: &Builder,
    scratch: &Path,
    ssh: &str,
) -> Result<(Command, PathBuf, PathBuf), String> {
    fs::create_dir_all(scratch).map_err(|error| format!("{}: {error}", scratch.display()))?;
    let known = mine(scratch, "known");
    let said = mine(scratch, "ssh.log");
    let alias = lent.host.rsplit('@').next().unwrap_or(&lent.host);
    let mut command = reaching(ssh, &said);
    // A builder with a key of its own is never reached through the agent
    // while that key can be read: ssh asks the agent first even then, and
    // on 8 November an agent waiting on its person stalled every lender's
    // ssh at login for 18 minutes, so each builder failed its proofs and
    // was given back. Inside pi's agent fence the key is hidden, reading
    // empty or refused, and the fence's own agent holds it instead and
    // signs only these logins; ssh then finds the key's public half beside
    // it and asks that agent, which answers within 5 s or is given up on.
    if let Some(key) = &lent.key {
        command.args(["-o", "IdentitiesOnly=yes"]);
        if !hidden(Path::new(key)) {
            command.args(["-o", "IdentityAgent=none"]);
        } else if let Some(host_key) = &lent.host_key {
            vouched(&vouching(), alias, host_key, VOUCHED_PATIENCE)?;
        }
        command.args(["-i", key]);
    }
    if let Some(host_key) = &lent.host_key {
        fs::write(&known, format!("{alias} {host_key}\n"))
            .map_err(|error| format!("{}: {error}", known.display()))?;
        command
            .args(["-o", "StrictHostKeyChecking=yes", "-o"])
            .arg(format!("UserKnownHostsFile={}", known.display()))
            .args(["-o", &format!("HostKeyAlias={alias}")]);
    }
    Ok((command, known, said))
}

/// Root's file of the builders it vouches for now, the only ones the
/// fence's agent signs a builder's login for, one a line as `<lender>
/// <instance> <key-type> <base64-key>`.
const VOUCHED: &str = "/run/nix-offload-lines/lent";

/// How long a fenced run waits for root to vouch for a builder it was
/// just lent: root asks the cloud for the builder, its disk and its
/// audit log first, which on 14 November took 12 s, and can take
/// minutes.
const VOUCHED_PATIENCE: Duration = Duration::from_secs(60);

/// Wait until `file` vouches for `lender`'s builder with `host_key`, for
/// at most `patience`, saying so; past it, why the login can't be
/// signed. Only a run whose key is hidden waits, since only the fence's
/// agent reads the file.
fn vouched(file: &Path, lender: &str, host_key: &str, patience: Duration) -> Result<(), String> {
    let started = std::time::Instant::now();
    let mut said = false;
    loop {
        if fs::read_to_string(file).is_ok_and(|text| names(&text, lender, host_key)) {
            return Ok(());
        }
        if started.elapsed() >= patience {
            return Err(format!(
                "root hasn't vouched for {lender}'s builder in {} within {} s, so the fence's \
                 agent won't sign its login",
                file.display(),
                patience.as_secs()
            ));
        }
        if !said {
            eprintln!(
                "there: waiting up to {} s for root to vouch for {lender}'s builder",
                patience.as_secs()
            );
            said = true;
        }
        // Root writes the file whole once it vouches; a second between
        // reads costs a waiting run little and the file's reader less.
        std::thread::sleep(Duration::from_secs(1));
    }
}

/// Whether root's `text`, read from [`VOUCHED`], names `lender`'s builder
/// with `host_key`.
fn names(text: &str, lender: &str, host_key: &str) -> bool {
    let wanted = format!("{lender} ");
    text.lines().any(|line| {
        line.strip_prefix(&wanted)
            .and_then(|rest| rest.split_once(' '))
            .is_some_and(|(_, key)| key == host_key)
    })
}

/// Root's file of the builders it vouches for, or the one
/// `NIX_OFFLOAD_VOUCHED` names for a test; set empty, it's ignored and
/// root's file is read. Naming a file that isn't there vouches for every
/// builder, as a machine without nix-offload-lines does: this check keeps
/// runs off builders root won't trust, and root's daemon trust and the
/// fence's agent are what refuse them, which no variable of the caller's
/// changes. The crate's own tests never read the machine's file, which
/// names none of their builders.
fn vouching() -> PathBuf {
    match std::env::var_os("NIX_OFFLOAD_VOUCHED") {
        Some(file) if !file.is_empty() => PathBuf::from(file),
        _ if cfg!(test) => PathBuf::from("/nonexistent/nix-offload-lines/lent"),
        _ => PathBuf::from(VOUCHED),
    }
}

/// Whether root vouches for the builder `store` names, as `file` says:
/// its lender and its host key on one line. A machine without the file
/// vouches for every builder, since nothing there keeps a list; one whose
/// file can't be read, or a store pinning no host key, for none. On 22
/// November a run sent to one lender spilled to another whose builder
/// root no longer vouched for, and outputs built on builders it didn't
/// name came back trusted.
fn vouched_in(file: &Path, store: &str) -> bool {
    let text = match fs::read_to_string(file) {
        Ok(text) => text,
        Err(error) => return error.kind() == std::io::ErrorKind::NotFound,
    };
    let Some(lent) = builder(store.lines().next().unwrap_or_default().trim()) else {
        return false;
    };
    let alias = lent.host.rsplit('@').next().unwrap_or(&lent.host);
    lent.host_key
        .as_deref()
        .is_some_and(|host_key| names(&text, alias, host_key))
}

/// The store `dir`'s lender names while its builder stands and root
/// vouches for it; none while it stands unvouched, said once a folder,
/// so nothing is sent or spilled to a builder root doesn't name. Root
/// vouches for a new builder within seconds to minutes of its standing,
/// so one waited on is taken once it does.
pub fn vouched_store(dir: &Path) -> Option<String> {
    vouched_store_in(dir, &vouching())
}

/// [`vouched_store`], as root's `file` says.
fn vouched_store_in(dir: &Path, file: &Path) -> Option<String> {
    // Each builder is said once a process, however often it's looked at,
    // so a lender that reads it every tick says each new one again.
    static SAID: std::sync::Mutex<BTreeSet<(PathBuf, String)>> =
        std::sync::Mutex::new(BTreeSet::new());
    let store = fs::read_to_string(dir.join("store"))
        .ok()
        .filter(|store| !store.trim().is_empty())?;
    if vouched_in(file, &store) {
        return Some(store);
    }
    if SAID
        .lock()
        .is_ok_and(|mut said| said.insert((dir.to_path_buf(), store.clone())))
    {
        // A file that can't be read vouches for none, as a sandbox not
        // granted /run/nix-offload-lines finds, which is no fault of root's.
        match fs::metadata(file) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => eprintln!(
                "nix-offload: {} can't be read ({error}), so the builder in {} counts as one \
                 root doesn't vouch for, and nothing goes to it",
                file.display(),
                dir.display()
            ),
            _ => eprintln!(
                "nix-offload: root doesn't vouch for the builder in {} as {} says, so nothing \
                 goes to it until it does; root's reason is in `journalctl -u nix-offload-lines`",
                dir.display(),
                file.display()
            ),
        }
    }
    None
}

/// Whether `dir`'s lender names a builder that stands, vouched for or not.
pub fn builder_stands(dir: &Path) -> bool {
    fs::read_to_string(dir.join("store")).is_ok_and(|store| !store.trim().is_empty())
}

/// Whether `dir`'s builder stands but root doesn't vouch for it.
pub fn unvouched(dir: &Path) -> bool {
    builder_stands(dir) && vouched_store(dir).is_none()
}

/// Whether `file` is a key hidden from this process, as the fence hides
/// one: it reads empty, or reading it is refused. A key that isn't there
/// at all isn't hidden, so ssh fails on it as it always has rather than
/// asking an agent.
fn hidden(file: &Path) -> bool {
    match fs::File::open(file).and_then(|opened| opened.metadata()) {
        Ok(metadata) => metadata.len() == 0,
        Err(error) => error.kind() == std::io::ErrorKind::PermissionDenied,
    }
}

/// Runs `command`, unpacking the tarball it writes into `out`, and gives
/// its status. A build that failed writes none, so nothing is unpacked.
/// One that passed but whose output couldn't be unpacked has failed too,
/// with 1, since what it was asked for isn't here: on 4 November a
/// gallery's run passed with its pictures unpacked nowhere, was kept as
/// passed, and the next at that commit copied an older folder's.
fn unpacked_into(command: &mut Command, out: &Path) -> std::io::Result<std::process::ExitStatus> {
    use std::os::unix::process::ExitStatusExt;
    fs::create_dir_all(out)?;
    // One an older run left read-only would refuse what's unpacked into it.
    writable(out);
    let mut running = command.stdout(Stdio::piped()).spawn()?;
    let tarball = running
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("no output to unpack"))?;
    let unpacking = Command::new("tar")
        .arg("-xzf")
        .arg("-")
        .arg("-C")
        .arg(out)
        .stdin(tarball)
        .stderr(Stdio::null())
        .status();
    let status = running.wait()?;
    writable(out);
    if status.success() && !unpacking.is_ok_and(|unpacked| unpacked.success()) {
        eprintln!(
            "nix-offload: couldn't unpack the output into {}, so the run failed",
            out.display()
        );
        return Ok(std::process::ExitStatus::from_raw(1 << 8));
    }
    Ok(status)
}

/// Lets the account write to `path` and everything in it. tar keeps the
/// modes an output had in the store, where nothing is writable, so on 6
/// November a run left its folder read-only and the next into it couldn't
/// unpack. A link is left alone, since its mode is its target's.
fn writable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let Ok(held) = fs::symlink_metadata(path) else {
        return;
    };
    if held.file_type().is_symlink() {
        return;
    }
    let mode = held.permissions().mode();
    if mode & 0o200 == 0 {
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(mode | 0o200));
    }
    if held.is_dir() {
        for entry in fs::read_dir(path).into_iter().flatten().flatten() {
            writable(&entry.path());
        }
    }
}

/// How many times a copy back is tried before `there` gives up on it. A
/// tunnel's connection can stop part way through a big output and ssh
/// gives up on it: twice running on 3 November, on a 9.5 GB path. Each try
/// skips the paths already here whole, though one cut off part way comes
/// again from its start, since Nix can't take up a path where it stopped.
const COPY_TRIES: u32 = 3;

/// Runs `command`, a build that prints its output paths, then copies them
/// from `store` into this machine's, trying again up to [`COPY_TRIES`]
/// times when the copy breaks off, and prints them once they're here, so
/// a caller never reads a path this store lacks. Its status is the build's,
/// or 1 when every copy fails. Each path comes signed or, `Unchecked`,
/// however it comes, which this machine's daemon allows only an account
/// it trusts. A machine nearby's paths go through [`signed_for_here`]
/// and [`copied_back`] instead, checked always.
fn brought_from(
    command: &mut Command,
    store: &str,
    nix: &str,
    signatures: Signatures,
) -> std::io::Result<std::process::ExitStatus> {
    let (built, paths) = built_paths(command)?;
    if !built.success() {
        return Ok(built);
    }
    copied_back(built, &paths, store, nix, signatures)
}

/// Runs `command`, a build that prints its output paths, and gives its
/// status and the paths it printed.
fn built_paths(command: &mut Command) -> std::io::Result<(std::process::ExitStatus, Vec<String>)> {
    // output() would capture stderr too, and drop the builder's log and
    // why a build failed, so it goes on to the caller as it comes.
    let built = command
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()?;
    let paths = String::from_utf8_lossy(&built.stdout)
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    Ok((built.status, paths))
}

/// Whether what `paths` on `store` are signed with is a key Nix here
/// trusts, or they're content-addressed, so they may come back checked. A
/// machine nearby signs what it builds as it builds it or never, so this
/// is asked once rather than waited for, as a lent builder's push is.
fn signed_for_here(nix: &str, store: &str, paths: &[String]) -> bool {
    let said = |command: &mut Command| {
        command
            .env("TMPDIR", "/tmp")
            .stderr(Stdio::null())
            .output()
            .ok()
            .filter(|said| said.status.success())
            .map(|said| String::from_utf8_lossy(&said.stdout).into_owned())
    };
    let Some(trusted) = said(Command::new(nix).args(["config", "show", "trusted-public-keys"]))
    else {
        return false;
    };
    said(
        Command::new(nix)
            .args(["path-info", "--json", "--store", store])
            .args(paths),
    )
    .and_then(|info| serde_json::from_str(&info).ok())
    .is_some_and(|info| held::trusted_by(&info, &trusted.split_whitespace().collect::<Vec<_>>()))
}

/// Copies `paths`, which a build ending `built` printed, from `store` into
/// this machine's, as [`brought_from`] says.
fn copied_back(
    built: std::process::ExitStatus,
    paths: &[String],
    store: &str,
    nix: &str,
    signatures: Signatures,
) -> std::io::Result<std::process::ExitStatus> {
    use std::os::unix::process::ExitStatusExt;
    let mut tries = 1;
    loop {
        // ssh's control socket goes in TMPDIR, and a long one is past what
        // a socket's path may be.
        let copied = Command::new(nix)
            .arg("copy")
            .args(match signatures {
                Signatures::Checked => &[][..],
                Signatures::Unchecked => &["--no-check-sigs"][..],
            })
            .args(["--from", store])
            .args(paths)
            .env("TMPDIR", "/tmp")
            .status()?;
        if copied.success() {
            break;
        }
        if tries == COPY_TRIES {
            eprintln!(
                "nix-offload: couldn't bring what was built from {store} into this store, \
                 in {COPY_TRIES} tries"
            );
            return Ok(std::process::ExitStatus::from_raw(1 << 8));
        }
        tries += 1;
        eprintln!(
            "nix-offload: bringing back what was built broke off, so it's tried again \
             ({tries} of {COPY_TRIES}): the paths that came whole stay, and one cut off \
             part way comes again from its start"
        );
        if signatures == Signatures::Checked {
            let within = signed_within();
            eprintln!(
                "nix-offload: what came back may hold a path the builder signs only as it \
                 pushes, so this waits {} s for it first",
                within.as_secs()
            );
            std::thread::sleep(within);
        }
    }
    let mut stdout = std::io::stdout();
    for path in paths {
        let _ = writeln!(stdout, "{path}");
    }
    Ok(built)
}

/// How a build waits for a builder when none stands: how long the lender
/// has to start making one, since it reads what's wanted every 20 s, how
/// long a lender that stopped may take to start again, and how long one
/// may take to stand, about 70 s on 29 October, and how long a build sent
/// again must have run for its loss to start a wait of its own. A run
/// given `--never-ask` doesn't ask at all, so it waits for nothing.
#[derive(Clone, Copy)]
pub struct Waiting {
    pub asks: bool,
    pub answered: std::time::Duration,
    pub restarting: std::time::Duration,
    pub longest: std::time::Duration,
    pub worked: std::time::Duration,
}

pub const WAITING: Waiting = Waiting {
    asks: true,
    answered: std::time::Duration::from_secs(30),
    // A switch stops the lender and starts it again once it's done the
    // rest: 1 s apart at 20:15 on 2 November, and 35 s apart at 21:14.
    restarting: std::time::Duration::from_secs(120),
    longest: std::time::Duration::from_secs(240),
    worked: std::time::Duration::from_secs(300),
};

/// A builder whose CPUs are this busy, as the share of the last minute
/// some task there waited for one, is full: every build sent it is slowed.
/// At 00:33 on 3 November the lent builder ran at load 50 to 63 on its 32
/// CPUs, waiting 48 to 55% of the time.
pub const FULL: f64 = 40.0;

/// The share of the last minute some task waited for a CPU, as
/// /proc/pressure/cpu's `some avg60` says, in percent.
pub fn pressure_of(text: &str) -> Option<f64> {
    let some = text.lines().find(|line| line.starts_with("some "))?;
    some.split_whitespace()
        .find_map(|field| field.strip_prefix("avg60="))?
        .parse()
        .ok()
}

/// The CPU pressure on the builder a store names, read there, or none
/// when it can't be read.
pub fn pressure(store: &str, scratch: &Path, ssh: &str) -> Option<f64> {
    let builder = builder(store.lines().next()?.trim())?;
    let reading = Sending {
        command: "cat /proc/pressure/cpu".to_owned(),
        bundle: None,
    };
    let (ran, printed) = printed_on(&builder, &reading, scratch, ssh).ok()?;
    (ran.status == 0).then(|| pressure_of(&printed)).flatten()
}

/// A second builder as a build going there finds it.
pub enum Second {
    /// None stands, so one is asked for and waited on.
    Unlent,
    /// One stands, with its CPU pressure when that can be read.
    Stands(Option<f64>),
}

/// Whether a build goes to a second builder rather than the first, given
/// the first's CPU pressure: only while the first is full, so the second
/// takes what overflows and is given back idle once the first has room,
/// and never to one standing that's as busy or can't be read. A first
/// whose pressure can't be read goes to a second that stands and answers,
/// since a tunnel that died answers nothing: on 7 November one stopped
/// answering at 20:51 and every build waited on it while the second stood
/// idle. With no second that answers, it keeps the build, whose own run
/// says what's wrong with it.
pub fn to_second(first: Option<f64>, second: &Second) -> bool {
    if first.is_none() {
        return matches!(second, Second::Stands(Some(_)));
    }
    first.is_some_and(|first| {
        first >= FULL
            && match second {
                Second::Unlent => true,
                Second::Stands(second) => second.is_some_and(|second| second < first),
            }
    })
}

/// Whether a pid runs, as signal 0 says.
fn runs(pid: i32) -> bool {
    // SAFETY: signal 0 only asks whether the process exists.
    pid > 0
        && stands(
            unsafe { libc::kill(pid, 0) },
            &std::io::Error::last_os_error(),
        )
}

/// Whether a process stands, as signal 0's answer `said` and the error
/// it left say. One this caller may not signal still stands: a fenced
/// session's Landlock refuses every signal outside it, so from 22:30 on
/// 13 November its runs took the lender for stopped, gave up on a
/// builder after one lost connection, and built about 45 of them here.
fn stands(said: i32, error: &std::io::Error) -> bool {
    said == 0 || error.raw_os_error() == Some(libc::EPERM)
}

/// The pid a file holds, when it holds one.
fn pid_in(path: &Path) -> Option<i32> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

/// Whether `dir`'s lender runs: it holds its `lender` file locked while it
/// does, which a fenced run sees though the fence gives it a pid namespace
/// of its own, where the pid the file holds names nothing; or that pid
/// runs, as a run beside the lender sees.
fn lender_stands(dir: &Path) -> bool {
    let lender = dir.join("lender");
    held(&lender) || pid_in(&lender).is_some_and(runs)
}

/// Whether another process holds `path` locked, as a lender does its
/// `lender` file and a waiting run its want: an exclusive lock is refused.
fn held(path: &Path) -> bool {
    fs::File::open(path)
        .is_ok_and(|file| matches!(file.try_lock(), Err(fs::TryLockError::WouldBlock)))
}

/// Where runs say they want `dir`'s lender to lend: beside its folder, in
/// `offload-wants`, by the folder's name, rather than in it, since a fence
/// gives its commands the lender folders only to read.
pub fn wants_of(dir: &Path) -> PathBuf {
    let name = dir.file_name().unwrap_or(dir.as_os_str());
    dir.with_file_name("offload-wants").join(name)
}

/// A run's want of a lender: a file in [`wants_of`] its folder, locked
/// while the run holds it, and taken away once it's let go. The kernel lets
/// the lock go the moment the run ends, however it ends, and the lender
/// takes away a want nobody holds.
pub struct Want {
    path: PathBuf,
    _held: fs::File,
}

impl Drop for Want {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Say this run wants `dir`'s lender to lend, while what's given back is
/// held. A want the lender took away as nobody's, between its making and
/// its lock, is made again.
fn wanting(dir: &Path) -> Option<Want> {
    use std::os::unix::fs::MetadataExt;
    let wants = wants_of(dir);
    fs::create_dir_all(&wants).ok()?;
    // The pid alone may name another run's want, since each fence counts
    // pids from one of its own.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    for again in 0..3 {
        let path = wants.join(format!("{}-{stamp}-{again}", std::process::id()));
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .ok()?;
        file.lock().ok()?;
        let named = fs::metadata(&path).ok().map(|named| named.ino());
        if named.is_some() && named == file.metadata().ok().map(|held| held.ino()) {
            return Some(Want { path, _held: file });
        }
    }
    None
}

/// Waits for a builder when none stands, rather than building here: it
/// says it wants one by a [`Want`] it holds, which the lender counts as
/// queued work, for as long as it waits. It
/// waits only while `dir`'s lender runs, as [`lender_stands`] sees, and
/// gives up once that lender hasn't begun making one (`lending` is empty)
/// within `answered`, or none stands within `longest`. A lender that
/// stopped, its pid left there, is waited for up to `restarting` first,
/// since a switch starts one again within seconds, and one superseded
/// ends just as a build that would have used its builder asks. It gives
/// the store's text once one stands.
pub fn waited(dir: &Path, waiting: Waiting) -> Option<String> {
    waited_vouching(dir, waiting, &vouching())
}

/// [`waited`], with root's `file` saying which builders it vouches for.
/// A builder that stands is the lender's answer, so a run waits up to
/// `longest` for root to vouch for it rather than giving up after
/// `answered`: on 22 November one waited 30 s, not the minutes root may
/// take.
fn waited_vouching(dir: &Path, waiting: Waiting, file: &Path) -> Option<String> {
    if !waiting.asks {
        if builder_stands(dir) {
            eprintln!(
                "nix-offload: the builder that stands isn't one root vouches for, and \
                 --never-ask waits for nothing"
            );
        } else {
            eprintln!("nix-offload: no builder stands, and --never-ask leaves the lender unasked");
        }
        return None;
    }
    if !lender_runs(dir, waiting) {
        return None;
    }
    // A run held for root its whole wait before it was sent has none left.
    if waiting.longest.is_zero() {
        eprintln!(
            "nix-offload: this run spent its wait holding for root to vouch for a VM builder, \
             so it waits no longer"
        );
        return None;
    }
    let want = wanting(dir)?;
    let minutes = waiting.longest.as_secs().div_ceil(60);
    if builder_stands(dir) {
        eprintln!(
            "nix-offload: a builder stands that root doesn't vouch for yet, so this waits for \
             it to, for up to {minutes} minutes"
        );
    } else {
        eprintln!(
            "nix-offload: no builder stands, so this asks for one and waits for it, for up to \
             {minutes} minutes"
        );
    }
    let began = std::time::Instant::now();
    let read = |name: &str| {
        fs::read_to_string(dir.join(name))
            .ok()
            .filter(|text| !text.trim().is_empty())
    };
    let stood = loop {
        if let Some(store) = vouched_store_in(dir, file) {
            break Some(store);
        }
        let so_far = began.elapsed();
        let unanswered = read("lending").is_none() && read("store").is_none();
        if so_far >= waiting.longest || (so_far >= waiting.answered && unanswered) {
            break None;
        }
        std::thread::sleep(std::time::Duration::from_millis(250).min(waiting.answered));
    };
    drop(want);
    stood
}

/// Whether `dir`'s lender runs, waiting up to `waiting.restarting` for one
/// that stopped, its pid left behind, to start again; with none ever
/// started here, it says so at once.
fn lender_runs(dir: &Path, waiting: Waiting) -> bool {
    let lender = dir.join("lender");
    if lender_stands(dir) {
        return true;
    }
    if !lender.exists() {
        return false;
    }
    eprintln!(
        "nix-offload: its lender is starting again, so this waits for it, for up to {} minutes",
        waiting.restarting.as_secs().div_ceil(60)
    );
    let began = std::time::Instant::now();
    while began.elapsed() < waiting.restarting {
        std::thread::sleep(std::time::Duration::from_millis(250).min(waiting.answered));
        if lender_stands(dir) {
            return true;
        }
    }
    false
}

/// A builder's store once one stands again after ssh lost it, giving up
/// `waiting.longest` after `since`: it looks every few seconds, since the
/// store may still name the builder whose tunnel just went, and waits as
/// `waited` does once the store is empty. Only a lender running stands one
/// again, but a build is most often lost because its lender restarted, as
/// a switch does, so a lender that stopped, whose pid is left in `dir`'s
/// `lender`, is waited for up to `waiting.restarting` after `since`, and
/// one that never ran here up to `waiting.answered`.
pub fn stood_again(dir: &Path, waiting: Waiting, since: std::time::Instant) -> Option<String> {
    loop {
        if since.elapsed() >= waiting.longest {
            return None;
        }
        std::thread::sleep(AGAIN.min(waiting.answered));
        if lender_stands(dir) {
            return vouched_store(dir).or_else(|| waited(dir, waiting));
        }
        let bound = if dir.join("lender").exists() {
            waiting.restarting
        } else {
            waiting.answered
        };
        if since.elapsed() >= bound {
            return None;
        }
    }
}

/// How long a build lost mid-way waits before it's sent again.
const AGAIN: std::time::Duration = std::time::Duration::from_secs(5);

/// Whether a run is waiting for `dir`'s lender to lend: a want in
/// [`wants_of`] it is held. One nobody holds is taken away here, while
/// this holds its lock, so a run locking it meanwhile finds it gone and
/// makes another.
pub fn wanted(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(wants_of(dir)) else {
        return false;
    };
    let mut waiting = false;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let Ok(file) = fs::File::open(&path) else {
            continue;
        };
        if matches!(file.try_lock(), Err(fs::TryLockError::WouldBlock)) {
            waiting = true;
        } else {
            let _ = fs::remove_file(&path);
        }
    }
    waiting
}

/// A machine nearby that builds for systems this one can't, such as a Mac
/// at home for its own: where it answers, `host:port`, the host ssh is
/// given, the systems it takes, and the attributes it takes whatever their
/// system, as patterns where `*` stands for any run of characters. A Mac's
/// Linux builder runs under emulation, so it may take the checks that are
/// only evaluation, which it does itself, without taking every Linux build.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Nearby {
    pub address: String,
    pub host: String,
    pub systems: Vec<String>,
    pub attributes: Vec<String>,
}

impl Nearby {
    fn takes(&self, attribute: &str) -> bool {
        system_of(attribute).is_some_and(|system| self.systems.iter().any(|s| s == system))
            || self
                .attributes
                .iter()
                .any(|pattern| matches(pattern, attribute))
    }
}

/// Whether `text` is what `pattern` describes, where each `*` stands for
/// any run of characters, none included.
fn matches(pattern: &str, text: &str) -> bool {
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or_default();
    let Some(mut rest) = text.strip_prefix(first) else {
        return false;
    };
    let mut parts: Vec<&str> = parts.collect();
    let Some(last) = parts.pop() else {
        // No star: the whole of it had to be the first part.
        return rest.is_empty();
    };
    for part in parts {
        match rest.find(part) {
            Some(at) => rest = &rest[at + part.len()..],
            None => return false,
        }
    }
    rest.ends_with(last)
}

/// The machines nearby in a file of them, a line each:
/// `ADDRESS HOST SYSTEM,SYSTEM [PATTERN,PATTERN]`. Blank lines and those
/// starting `#` say nothing, and a line short of a word names no machine.
pub fn nearby(text: &str) -> Vec<Nearby> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let (address, host, systems) = (words.next()?, words.next()?, words.next()?);
            Some(Nearby {
                address: address.to_owned(),
                host: host.to_owned(),
                systems: systems.split(',').map(str::to_owned).collect(),
                attributes: words
                    .next()
                    .map(|patterns| patterns.split(',').map(str::to_owned).collect())
                    .unwrap_or_default(),
            })
        })
        .collect()
}

/// The system an attribute is built for: its second word, as flakes name
/// their outputs, `checks.aarch64-darwin.x`.
fn system_of(attribute: &str) -> Option<&str> {
    attribute.split('.').nth(1)
}

/// What was asked, shared out: each attribute a machine nearby takes, by
/// its system or by name, goes to the first that does and `answers`, and
/// the rest is left for the lent builder or here. Whether one answers is
/// asked once.
pub fn shared<'a>(
    asked: &Asked,
    nearby: &'a [Nearby],
    answers: impl Fn(&Nearby) -> bool,
) -> (Vec<(&'a Nearby, Asked)>, Option<Asked>) {
    // An applied function or an output copied back goes the lent way,
    // which knows them, and derivations too: they may hold what only the
    // lent builder may build.
    if asked.apply.is_some() || asked.out.is_some() || asked.derivations {
        return (
            Vec::new(),
            Some(asked.with(&asked.flake, asked.attributes.clone())),
        );
    }
    // Only a machine that takes something asked is asked whether it
    // answers: an away machine's name can take seconds to fail to resolve.
    let answering: Vec<&Nearby> = nearby
        .iter()
        .filter(|machine| {
            asked
                .attributes
                .iter()
                .any(|attribute| machine.takes(attribute))
        })
        .filter(|machine| {
            let answering = answers(machine);
            // Said, so a machine nearby that builds nothing for a while
            // is seen to be away rather than thought idle.
            if !answering {
                eprintln!(
                    "nix-offload: {} nearby doesn't answer, so what it takes goes the other way",
                    machine.host
                );
            }
            answering
        })
        .collect();
    let mut parts: Vec<(&Nearby, Vec<String>)> = Vec::new();
    let mut left = Vec::new();
    for attribute in &asked.attributes {
        let taker = answering.iter().find(|machine| machine.takes(attribute));
        match taker {
            Some(machine) => match parts.iter_mut().find(|(had, _)| had == machine) {
                Some((_, attributes)) => attributes.push(attribute.clone()),
                None => parts.push((machine, vec![attribute.clone()])),
            },
            None => left.push(attribute.clone()),
        }
    }
    let parts = parts
        .into_iter()
        .map(|(machine, attributes)| (machine, asked.with(&asked.flake, attributes)))
        .collect();
    let left = (!left.is_empty()).then(|| asked.with(&asked.flake, left));
    (parts, left)
}

/// What's left after [`shared`], parted from the attributes built for a
/// system other than `system`, this machine's and so its lent builders',
/// that a machine nearby takes by its system: [`shared`] left them since
/// none of those answers, and a lent builder can't build for that system.
/// On 16 November one for a Mac's went to a Linux lent builder while the
/// Mac was away, and failed on the platform, saying nothing of the check.
pub fn owed(left: Option<Asked>, nearby: &[Nearby], system: &str) -> (Option<Asked>, Vec<String>) {
    let Some(left) = left else {
        return (None, Vec::new());
    };
    let only_nearby = |attribute: &String| {
        system_of(attribute).is_some_and(|built_for| {
            built_for != system
                && nearby
                    .iter()
                    .any(|machine| machine.systems.iter().any(|s| s == built_for))
        })
    };
    let (owed, rest): (Vec<String>, Vec<String>) =
        left.attributes.iter().cloned().partition(only_nearby);
    let rest = (!rest.is_empty()).then(|| left.with(&left.flake, rest));
    (rest, owed)
}

/// What was asked, split: the VM tests, for a builder of their own so they
/// never share one with big builds, and the rest. An attribute is one when
/// a pattern in `patterns` describes it, an applied function's or an
/// output copied back's one attribute too, as run-subtests and
/// shots-changed build a VM test; a derivation when it's among
/// `featured`, those [`needing_vm`] found, whatever its name.
pub fn vm_tests(
    asked: &Asked,
    patterns: &[String],
    featured: &BTreeSet<String>,
) -> (Option<Asked>, Option<Asked>) {
    let (vm, rest): (Vec<String>, Vec<String>) =
        asked.attributes.iter().cloned().partition(|attribute| {
            if asked.derivations {
                featured.contains(attribute.split('^').next().unwrap_or(attribute))
            } else {
                patterns.iter().any(|pattern| matches(pattern, attribute))
            }
        });
    let part = |attributes: Vec<String>| {
        (!attributes.is_empty()).then(|| asked.with(&asked.flake, attributes))
    };
    (part(vm), part(rest))
}

/// How many of `count` VM tests go to each VM builder, in order, each
/// running `builds` at once, with `busy` already sent to each by runs
/// still going: each goes to the first with room, and once none has, to
/// the least busy, the earliest when they're even. On 3 November a third
/// of VM tests failed with 7 to 12 runs overlapping, those past the
/// builder's room waiting their turn there.
pub fn spread(count: usize, busy: &[usize], builds: usize) -> Vec<usize> {
    let mut busy = busy.to_vec();
    let mut sent = vec![0; busy.len()];
    for _ in 0..count {
        let roomy = busy.iter().position(|tests| *tests < builds);
        let least = busy
            .iter()
            .enumerate()
            .min_by_key(|(_, tests)| **tests)
            .map(|(index, _)| index);
        let Some(to) = roomy.or(least) else {
            break;
        };
        busy[to] += 1;
        sent[to] += 1;
    }
    sent
}

/// VM tests asked for, split between VM builders as `spread` says, in the
/// order they were asked: a part for each, none where it's sent none.
pub fn overflow(asked: &Asked, busy: &[usize], builds: usize) -> Vec<Option<Asked>> {
    let mut left = asked.attributes.as_slice();
    spread(left.len(), busy, builds)
        .into_iter()
        .map(|tests| {
            let (these, rest) = left.split_at(tests);
            left = rest;
            (!these.is_empty()).then(|| asked.with(&asked.flake, these.to_vec()))
        })
        .collect()
}

/// How many attributes a part asks for.
pub fn count(asked: &Asked) -> usize {
    asked.attributes.len()
}

/// Where a part waits for room, as `--first` and `--bulk` ask, the
/// first winning.
pub fn rank(asked: &Asked) -> Rank {
    if first() {
        Rank::First
    } else if asked.bulk {
        Rank::Bulk
    } else {
        Rank::Plain
    }
}

/// The VM tests a run has sent a VM builder, counted while it goes on: a
/// file in its lender's folder's `sending`, named for the run's pid, that
/// holds how many, and goes once it's dropped.
pub struct Claim(PathBuf);

impl Drop for Claim {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Says `count` VM tests go to the builder `dir`'s lender lends, for as
/// long as what's given lives; none when the folder can't be written.
pub fn claim(dir: &Path, count: usize) -> Option<Claim> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    // One run may send two parts to one folder, when a second builder
    // doesn't come.
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let sending = dir.join("sending");
    fs::create_dir_all(&sending).ok()?;
    let path = sending.join(format!(
        "{}.{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&path, format!("{count}\n")).ok()?;
    Some(Claim(path))
}

/// How many VM tests a VM builder runs at once where the machine doesn't
/// say: as many as its helper is told when told none.
pub const VM_ROOM: usize = 4;

/// Says `count` VM tests go to the builder `dir`'s lender lends, once
/// there's room for them beside those runs still going have sent it, as
/// many as `room`: Nix's max-jobs bounds each client's build, not the
/// builder, so on 4 November one told 4 at once ran 15, and timing-
/// sensitive tests failed. A run may build `room` at once, however many
/// it asks, and it goes when nothing else is sent, so one past the room
/// waits for none. It waits, saying so once, checking and claiming under a
/// lock so two runs can't both take the last room; with none to be had,
/// as where the folder can't be written, it goes at once. Runs waiting go
/// in the order they came, each holding its place in the folder's
/// `waiting`, since one past the room goes only once nothing else is
/// sent, and smaller runs coming after it kept finding room first. One
/// that fits may still pass one that doesn't, for [`PASSING_FOR`] after
/// that one came, so room isn't left idle behind it ([`ahead`] says why).
///
/// While it waits, a run goes to the first of `others`, the other VM
/// builders, that has room and nothing waiting, and says which it went to:
/// a run's split is decided as it starts, so on 4 November ten runs waited
/// 51 minutes on the first while the second sat idle. Past
/// [`ASKS_AFTER`], it asks one given back to stand again, while what
/// waits passes the room of those coming ([`ask_for_room`]).
///
/// A `bulk` run lets every run that isn't go before it, those waiting
/// and those coming while it waits, and they wait for no bulk run: the
/// after-merge net sends dozens of VM tests a round, and on 5 November a
/// session's few waited behind the whole round. A run sent first goes
/// ahead of every run that isn't, bulk or not.
///
/// It gives the builder's folder, the room claimed there, and what is left
/// of `waiting` once the time it held for root is taken from its
/// `longest`, for the run's first wait as it's sent, so a run waits on
/// root once, not here and again once sent. Later waits, for a builder
/// lost mid-run, are the run's own.
pub fn admitted<'a>(
    dir: &'a Path,
    others: &[&'a Path],
    count: usize,
    room: usize,
    rank: Rank,
    waiting: Waiting,
) -> (&'a Path, Option<Claim>, Waiting) {
    let (dir, claim, held) = admitted_holding(dir, others, count, room, rank, waiting);
    let left = Waiting {
        longest: waiting.longest.saturating_sub(held),
        ..waiting
    };
    (dir, claim, left)
}

/// [`admitted`], giving how long it held for root rather than what's left.
fn admitted_holding<'a>(
    dir: &'a Path,
    others: &[&'a Path],
    count: usize,
    room: usize,
    rank: Rank,
    waiting: Waiting,
) -> (&'a Path, Option<Claim>, std::time::Duration) {
    use std::os::fd::AsRawFd;
    let wants = count.clamp(1, room.max(1));
    let mut said = false;
    let mut held_said = false;
    let mut place: Option<Claim> = None;
    // Time held for root, counted only while holding, so a run that
    // waited for room before a hand-over is still held its full time.
    let mut held = std::time::Duration::ZERO;
    let mut holding_since: Option<std::time::Instant> = None;
    let came = std::time::Instant::now();
    let mut asked: Option<Want> = None;
    loop {
        let was_holding = holding_since.take();
        // A builder root doesn't vouch for takes no room, so a run isn't
        // held there waiting on root while another stands that root does
        // vouch for. It queues on that one instead, in its order, with
        // this one among its others: so it's taken there in its turn,
        // rather than passed by every run that queued there after it, and
        // still goes back here if root vouches for this one and it has
        // room first.
        if unvouched(dir) {
            if let Some(vouched) = others
                .iter()
                .copied()
                .find(|other| vouched_store(other).is_some())
            {
                eprintln!(
                    "nix-offload: root doesn't vouch for this VM builder yet, so this waits on \
                     {} instead, or for root, whichever comes first",
                    vouched.display()
                );
                drop(place);
                let rest: Vec<&Path> = std::iter::once(dir)
                    .chain(others.iter().copied().filter(|other| *other != vouched))
                    .collect();
                let left = Waiting {
                    longest: waiting.longest.saturating_sub(held),
                    ..waiting
                };
                let (dir, claim, more) = admitted_holding(vouched, &rest, count, room, rank, left);
                return (dir, claim, held + more);
            }
            // With none vouched, nothing is claimed while root may yet
            // vouch for this one or another, for as long as a run sent
            // waits on root: claimed here at once, a run stayed held on
            // it after another was vouched with room. Past that, it
            // claims room here, and is sent with that wait spent. A run
            // that never asks waits for nothing, so it isn't held.
            if waiting.asks && held < waiting.longest {
                let now = std::time::Instant::now();
                held += was_holding.map_or(std::time::Duration::ZERO, |since| now - since);
                holding_since = Some(now);
                if !held_said {
                    eprintln!(
                        "nix-offload: root vouches for none of the VM builders yet, so this \
                         waits for it to"
                    );
                    held_said = true;
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
                continue;
            }
        }
        let Ok(lock) = fs::File::create(dir.join("sending.lock")) else {
            return (dir, claim(dir, wants), held);
        };
        // SAFETY: flock only locks the file it's given, open until dropped.
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX) } != 0 {
            return (dir, claim(dir, wants), held);
        }
        let busy = claimed(dir);
        let before = ahead(dir, place.as_ref(), rank, busy, room);
        if before == 0 && (busy == 0 || busy + wants <= room) {
            // Claimed, and its place given up, before the lock is let go,
            // as dropping it does.
            let claimed = claim(dir, wants);
            drop(place);
            return (dir, claimed, held);
        }
        if place.is_none() {
            place = queued(dir, wants, rank);
        }
        // Let go before the other's is taken, so two runs waiting on each
        // other's builders never hold one lock each and wait on the other.
        drop(lock);
        // Waiting on a warm builder, never one that's cold, though one
        // warming now may be warm by the next look.
        let here_warm = warm(dir);
        if let Some((other, claimed)) = others
            .iter()
            .filter(|other| !unvouched(other) && (!here_warm || warm(other)))
            .find_map(|other| Some((*other, free(other, wants, room, rank)?)))
        {
            eprintln!(
                "nix-offload: another VM builder has room now and nothing waiting, so this goes \
                 there"
            );
            drop(place);
            return (other, Some(claimed), held);
        }
        // A run given `--never-ask` asks no lender for anything, room
        // included, so it only waits for the builders already standing.
        if waiting.asks && asked.is_none() && came.elapsed() >= ASKS_AFTER {
            asked = ask_for_room(dir, others, room);
        }
        if !said {
            let queue = match before {
                0 => String::new(),
                1 => ", and a run waits ahead of this".to_owned(),
                runs => format!(", and {runs} runs wait ahead of this"),
            };
            eprintln!(
                "nix-offload: the VM builder has {busy} VM tests of runs here and room for \
                 {room}{queue}, so this waits for room"
            );
            if rank == Rank::Bulk {
                eprintln!("nix-offload: this run is bulk, so runs that aren't go first");
            }
            said = true;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

/// How long a run waits for room before it asks a VM builder that was
/// given back to stand again, as [`ask_for_room`] does.
const ASKS_AFTER: std::time::Duration = std::time::Duration::from_mins(1);

/// Asks one of `others` to stand for the runs waiting for room, by a
/// [`Want`] held while this run waits, if [`to_ask`] names one; says so.
/// A builder given back stood again only for a run that asked as it
/// began, so runs coming once the standing ones were full waited beside
/// lanes nobody asked for: at 15:45 on 23 November 17 parts waited on
/// three VM builders, while two given back at 15:13 and 15:20 stayed down.
fn ask_for_room(dir: &Path, others: &[&Path], room: usize) -> Option<Want> {
    let lane = to_ask(dir, others, room)?;
    let want = wanting(lane)?;
    eprintln!(
        "nix-offload: the standing VM builders are full, so this asks {} to stand for the \
         runs waiting",
        lane.display()
    );
    Some(want)
}

/// Of `others`, the first nobody stands up or asks for, if the VM tests
/// waiting on `dir` and on them pass the `room` of the cold builders
/// already standing or asked for: one more is asked for only while those
/// coming can't take what waits.
fn to_ask<'a>(dir: &Path, others: &[&'a Path], room: usize) -> Option<&'a Path> {
    let lanes = || std::iter::once(dir).chain(others.iter().copied());
    let waits: usize = lanes().map(waiting).sum();
    let rising = lanes()
        .filter(|lane| !warm(lane) && (coming(lane) || wanted(lane)))
        .count();
    if waits <= rising * room {
        return None;
    }
    others
        .iter()
        .copied()
        .find(|other| !coming(other) && !wanted(other))
}

/// A claim of `wants` VM tests on the builder `dir`'s lender lends, taken
/// under its lock only while no run waits there ahead of a run that's
/// at its `rank` and it has room for them as [`admitted`] counts it; none
/// otherwise.
fn free(dir: &Path, wants: usize, room: usize, rank: Rank) -> Option<Claim> {
    use std::os::fd::AsRawFd;
    let lock = fs::File::create(dir.join("sending.lock")).ok()?;
    // SAFETY: flock only locks the file it's given, open until dropped.
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX) } != 0 {
        return None;
    }
    let busy = claimed(dir);
    if ahead(dir, None, rank, busy, room) == 0 && (busy == 0 || busy + wants <= room) {
        claim(dir, wants)
    } else {
        None
    }
}

/// A place among the runs waiting for room in `dir`, after every run
/// already there: named by when it was taken, then this run's pid, so
/// they sort in the order they came and one whose run ended is seen, and
/// a [`BULK`] or [`FIRST_PLACE`] run's marked so. It holds how many VM
/// tests the run `wants`, which [`waiting`] counts.
fn queued(dir: &Path, wants: usize, rank: Rank) -> Option<Claim> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let waiting = dir.join("waiting");
    fs::create_dir_all(&waiting).ok()?;
    let came = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos();
    let path = waiting.join(format!(
        "{came:020}.{}.{}{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
        match rank {
            Rank::First => FIRST_PLACE,
            Rank::Plain => "",
            Rank::Bulk => BULK,
        }
    ));
    fs::write(&path, format!("{wants}\n")).ok()?;
    Some(Claim(path))
}

/// How many VM tests runs still going wait to send the builder `dir`'s
/// lender lends, as their places in its `waiting` say. A place whose run
/// ended is left to [`ahead`] to take away.
pub fn waiting(dir: &Path) -> usize {
    let Ok(entries) = fs::read_dir(dir.join("waiting")) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .and_then(|name| name.split('.').nth(1)?.parse().ok())
                .is_some_and(runs)
        })
        .filter_map(|entry| fs::read_to_string(entry.path()).ok())
        .filter_map(|wants| wants.trim().parse::<usize>().ok())
        .sum()
}

/// What ends a bulk run's place among those waiting.
const BULK: &str = ".bulk";

/// What ends the place of a run sent first.
const FIRST_PLACE: &str = ".first";

/// How long a run waiting for room may be passed by ones behind it that
/// fit where it doesn't, unless `NIX_OFFLOAD_PASSING_FOR` says otherwise
/// in seconds: past it, nothing passes it, so the builder empties for it.
const PASSING_FOR: std::time::Duration = std::time::Duration::from_mins(15);

fn passing_for() -> std::time::Duration {
    std::env::var("NIX_OFFLOAD_PASSING_FOR")
        .ok()
        .and_then(|seconds| seconds.parse().ok())
        .map_or(PASSING_FOR, std::time::Duration::from_secs)
}

/// How many runs still going wait for room in `dir` ahead of the one at
/// `place`, or at all for one with no place yet, and hold it back: every
/// run of a rank ahead of its `rank`, and those of its own that came
/// before it, unless it doesn't fit beside the `busy` of the `room` yet
/// and hasn't waited [`PASSING_FOR`]. On 8 November three places sat idle
/// on two VM builders while nine parts waited, some for 70 minutes,
/// behind a part of 4 that could go only once its builder was empty. A
/// place whose run ended without giving it up is taken away here.
fn ahead(dir: &Path, place: Option<&Claim>, rank: Rank, busy: usize, room: usize) -> usize {
    let Ok(entries) = fs::read_dir(dir.join("waiting")) else {
        return 0;
    };
    let mine = place.and_then(|place| place.0.file_name().map(std::ffi::OsStr::to_owned));
    let passing = passing_for();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let mut before = 0;
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        let pid = name
            .to_str()
            .and_then(|name| name.split('.').nth(1)?.parse().ok());
        let came_before = mine.as_ref().is_none_or(|mine| name < *mine);
        let theirs = match name.to_str() {
            Some(name) if name.ends_with(FIRST_PLACE) => Rank::First,
            Some(name) if name.ends_with(BULK) => Rank::Bulk,
            _ => Rank::Plain,
        };
        if !pid.is_some_and(runs) {
            let _ = fs::remove_file(entry.path());
        } else if (theirs < rank || (theirs == rank && came_before))
            && holds_back(&entry.path(), name.to_str(), busy, room, now, passing)
        {
            before += 1;
        }
    }
    before
}

/// Whether the run waiting at `place`, named `name`, holds back those
/// behind it: it fits beside the `busy` of the `room`, so it goes next,
/// or it has waited `passing` already, or what it wants can't be read.
fn holds_back(
    place: &Path,
    name: Option<&str>,
    busy: usize,
    room: usize,
    now: std::time::Duration,
    passing: std::time::Duration,
) -> bool {
    let wants = fs::read_to_string(place)
        .ok()
        .and_then(|wants| wants.trim().parse::<usize>().ok());
    let came = name
        .and_then(|name| name.split('.').next()?.parse::<u128>().ok())
        .and_then(|nanos| u64::try_from(nanos / 1_000_000_000).ok())
        .map(std::time::Duration::from_secs);
    let (Some(wants), Some(came)) = (wants, came) else {
        return true;
    };
    busy == 0 || busy + wants <= room || now.saturating_sub(came) >= passing
}

/// How many VM tests runs still going have sent the builder `dir`'s
/// lender lends. A claim whose run ended without taking it away, as one
/// killed does, is taken away here.
pub fn claimed(dir: &Path) -> usize {
    let Ok(entries) = fs::read_dir(dir.join("sending")) else {
        return 0;
    };
    let mut sent = 0;
    for entry in entries.filter_map(Result::ok) {
        let pid = entry
            .file_name()
            .to_str()
            .and_then(|name| name.split('.').next()?.parse().ok());
        if pid.is_some_and(runs) {
            sent += fs::read_to_string(entry.path())
                .ok()
                .and_then(|text| text.trim().parse::<usize>().ok())
                .unwrap_or(0);
        } else {
            let _ = fs::remove_file(entry.path());
        }
    }
    sent
}

/// What a VM test needs of the machine that builds it, as its derivation's
/// `requiredSystemFeatures` says: the lent lines offer neither, so Nix
/// never sends one there, but a derivation sent there whole is built
/// there whatever it needs.
const VM_FEATURES: [&str; 2] = ["kvm", "nixos-test"];

/// Of the derivations `nix derivation show` printed, by their paths, those
/// that need a VM test's features. Its format has changed: now they're
/// under `derivations` and named without the store's folder; and their
/// features are a string of words in `env`, or a list in `structuredAttrs`
/// or, before that, in `env`'s `__json`.
pub fn needing_vm(shown: &serde_json::Value) -> BTreeSet<String> {
    let all = shown.get("derivations").unwrap_or(shown);
    let features = |derivation: &serde_json::Value| -> Vec<String> {
        let env = derivation.get("env");
        if let Some(words) = env
            .and_then(|env| env.get("requiredSystemFeatures"))
            .and_then(serde_json::Value::as_str)
        {
            return words.split_whitespace().map(str::to_owned).collect();
        }
        let structured = derivation.get("structuredAttrs").cloned().or_else(|| {
            env.and_then(|env| env.get("__json"))
                .and_then(serde_json::Value::as_str)
                .and_then(|json| serde_json::from_str(json).ok())
        });
        structured
            .as_ref()
            .and_then(|attrs| attrs.get("requiredSystemFeatures"))
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(str::to_owned)
            .collect()
    };
    all.as_object()
        .into_iter()
        .flatten()
        .filter(|(_, derivation)| {
            features(derivation)
                .iter()
                .any(|feature| VM_FEATURES.contains(&feature.as_str()))
        })
        .map(|(path, _)| {
            if path.starts_with('/') {
                path.clone()
            } else {
                format!("/nix/store/{path}")
            }
        })
        .collect()
}

/// The derivations asked for that need a VM test's features, as this
/// machine's store shows them, which needs no evaluation; none when what
/// was asked are attributes, or the store can't show them.
pub fn featured(asked: &Asked, nix: &str) -> BTreeSet<String> {
    if !asked.derivations {
        return BTreeSet::new();
    }
    let paths = asked
        .attributes
        .iter()
        .map(|path| path.split('^').next().unwrap_or(path));
    needing_vm(&json_of(
        Command::new(nix).args(["derivation", "show"]).args(paths),
    ))
}

/// Whether a lender runs for `dir`, as its `lender` file says.
pub fn lends(dir: &Path) -> bool {
    lender_stands(dir)
}

/// The file whose being in a VM lender's folder keeps its lane for the
/// runs that name the folder, so none other spills onto it: the machine
/// declares it, as for a bare metal lane whose builder costs about 21
/// times an n2 lane's for the same four builds.
pub const NAMED_ONLY: &str = "named-only";

/// Whether `dir`'s lane takes only the runs that name it, as its
/// [`NAMED_ONLY`] file says.
pub fn named_only(dir: &Path) -> bool {
    dir.join(NAMED_ONLY).exists()
}

/// The file a VM lender keeps in its folder while the builder it has just
/// stood builds what warms it, holding the lender's pid and the builder's
/// name, so one a lender left as it stopped says nothing.
pub const WARMING: &str = "warming";

/// Whether `dir`'s builder stands and is warm: a lender that stood it
/// has finished warming it, or never did. On 6 November each fresh VM
/// builder's first tests took 65 to 80 minutes building the desktop for
/// themselves, against 2 to 12 once warm, so VM tests go to a warm one
/// while any stands.
pub fn warm(dir: &Path) -> bool {
    let stands = vouched_store(dir).is_some();
    let warming = fs::read_to_string(dir.join(WARMING)).is_ok_and(|held| {
        held.split_whitespace()
            .next()
            .and_then(|pid| pid.parse().ok())
            .is_some_and(runs)
    });
    stands && !warming
}

/// Whether `dir`'s lender is making its builder ready or warming it, so
/// it will have room for VM tests without being asked.
pub fn coming(dir: &Path) -> bool {
    let said =
        |name: &str| fs::read_to_string(dir.join(name)).is_ok_and(|text| !text.trim().is_empty());
    said("lending") || said("store")
}

/// Asks `dir`'s lender for a builder while this run lives, as a build
/// waiting for one does, without sending it anything: the VM tests go to
/// the warm builders, and this one warms for those after.
pub fn want(dir: &Path) {
    if let Some(want) = wanting(dir) {
        HELD_WANTS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(want);
    }
}

/// The wants [`want`] makes, held until the run ends.
static HELD_WANTS: std::sync::Mutex<Vec<Want>> = std::sync::Mutex::new(Vec::new());

/// How many cold VM builders to ask for, besides those `coming`, so that
/// `count` VM tests past the room of the warm builders, which have `busy`
/// sent to them and run `builds` at once each, find room once those warm.
pub fn to_warm(count: usize, busy: &[usize], builds: usize, coming: usize) -> usize {
    let past = (busy.iter().sum::<usize>() + count).saturating_sub(busy.len() * builds);
    past.div_ceil(builds.max(1)).saturating_sub(coming)
}

/// Has the lent builder build `installables` for itself, so the VM tests
/// sent there after find most of what they need already built; whether
/// it built them all.
pub fn warmed(lent: &Builder, installables: &[String], scratch: &Path, ssh: &str) -> bool {
    let Ok((mut warming, known, said)) = reach(lent, scratch, ssh) else {
        return false;
    };
    let command = ["nix", "build", "--no-link", "--keep-going"]
        .into_iter()
        .map(str::to_owned)
        .chain(installables.iter().map(|installable| quoted(installable)))
        .collect::<Vec<_>>()
        .join(" ");
    let built = warming
        .arg(&lent.host)
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    for file in [&known, &said] {
        let _ = fs::remove_file(file);
    }
    built
}

/// The patterns of VM tests' attributes in a file of them, a line each;
/// blank lines and those starting `#` say nothing.
pub fn patterns(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

/// The folder here a flake is, when it's one no other machine could fetch:
/// `path:DIR`, or a path itself.
fn folder_of(flake: &str) -> Option<PathBuf> {
    let path = flake.strip_prefix("path:").unwrap_or(flake);
    let path = path.split('?').next().unwrap_or(path);
    (path.starts_with('/') || path.starts_with('.')).then(|| PathBuf::from(path))
}

/// The store paths a flake's lock pins inputs to, which only a store that
/// already has them can read: a nearby machine is sent them.
fn pinned(lock: &str) -> Vec<String> {
    let Ok(lock) = serde_json::from_str::<serde_json::Value>(lock) else {
        return Vec::new();
    };
    let mut found: Vec<String> = lock["nodes"]
        .as_object()
        .into_iter()
        .flat_map(|nodes| nodes.values())
        .filter(|node| node["locked"]["type"] == "path")
        .filter_map(|node| node["locked"]["path"].as_str())
        .filter(|path| path.starts_with("/nix/store/"))
        .map(str::to_owned)
        .collect();
    found.sort();
    found.dedup();
    found
}

/// Where on a nearby machine, under its account's home, a flake sent as a
/// folder is unpacked for the length of its build.
fn unpacked() -> String {
    format!(".cache/nix-offload/there/{}", std::process::id())
}

/// Whether a machine nearby can fetch the flake `asked` names, asked once
/// before anything is built there. A folder here goes with the build, so
/// it's never fetched. One that answers ssh but can't fetch fails every
/// build: on 2 November a Mac whose resolver was down failed two runs in
/// a row on `Could not resolve host: github.com`, while the lent builder
/// could have built them. The fetch is kept, so the build reuses it.
pub fn fetches(machine: &Nearby, asked: &Asked, scratch: &Path, ssh: &str) -> bool {
    if folder_of(&asked.flake).is_some() {
        return true;
    }
    if fs::create_dir_all(scratch).is_err() {
        return false;
    }
    let said = mine(scratch, "fetch.log");
    let fetched = reaching(ssh, &said)
        .arg(&machine.host)
        .arg(far(&format!(
            "nix flake metadata {} > /dev/null",
            quoted(&asked.flake)
        )))
        .stdout(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    let _ = fs::remove_file(&said);
    if !fetched {
        eprintln!(
            "nix-offload: {} couldn't fetch {}, so what it takes goes the other way",
            machine.host, asked.flake
        );
    }
    fetched
}

/// How busy a Mac nearby is: its load over the last minute and the last
/// five, and its processors, as its `sysctl` says them. The last minute
/// alone isn't enough, since a Mac's builds come and go in bursts: at
/// 08:58 on 17 November two runs went to a Mac whose five-minute load was
/// 60 to 80 on 12 processors, through a gate that read the last minute in
/// a lull. Nor is five alone, which a build its person has just begun
/// takes a minute or two to reach. Nor is load alone: at 04:50 on 18
/// November the net's comparison went to a Mac at 35 over five, under its
/// room, while a session's darwin build already ran there, and the two
/// took it to 60. So it counts the runs sent to it that build there now
/// too, by the turns they hold ([`in_turn_nearby`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Load {
    pub minute: f64,
    pub five: f64,
    pub cores: u32,
    pub building: u32,
}

/// Runnable work for each processor past which a Mac has no room. On 16
/// November seven gates' comparisons went to a Mac at load 110 to 130 on
/// 12 processors, and its person's own build, which needs it under 40,
/// missed twice; its builder sessions are kept under 36, three a core.
const ROOM_PER_CORE: f64 = 3.0;

/// The longest a Mac takes to say its load. One too busy to answer in
/// this has no room either.
const LOAD_PATIENCE: Duration = Duration::from_secs(30);

impl Load {
    /// Read from `sysctl -n vm.loadavg hw.ncpu`, as `{ 120.98 88.31 65.93 }`
    /// then `12`, the first two averages being the last minute's and the
    /// last five's, or none when it says something else: a figure missing
    /// or one too many, or one that isn't a load, so it's never read from
    /// the wrong place. A line after the processors, as [`building_read`]
    /// says, is how many runs sent to it build there now, and none is read
    /// as none, as a Mac's own `sysctl` says it to `nix-offload busy`.
    pub fn parse(said: &str) -> Option<Load> {
        let mut lines = said.lines();
        let averages = lines.next()?.trim();
        let averages = averages.strip_prefix('{')?.strip_suffix('}')?;
        let mut figures = averages.split_whitespace().map(|word| {
            word.parse::<f64>()
                .ok()
                .filter(|figure| figure.is_finite() && *figure >= 0.0)
        });
        let (minute, five, _fifteen) = (figures.next()??, figures.next()??, figures.next()??);
        if figures.next().is_some() {
            return None;
        }
        let cores = lines
            .next()?
            .trim()
            .parse::<u32>()
            .ok()
            .filter(|&cores| cores > 0)?;
        let building = match lines.next() {
            Some(line) => line.trim().parse::<u32>().ok()?,
            None => 0,
        };
        if lines.next().is_some() {
            return None;
        }
        Some(Load {
            minute,
            five,
            cores,
            building,
        })
    }

    /// Whether the Mac has no room for a comparison: either load past
    /// three a processor.
    pub fn over(self) -> bool {
        self.minute.max(self.five) > f64::from(self.cores) * ROOM_PER_CORE
    }

    /// The load as a person reads it, as `load 121.0 over a minute and
    /// 88.3 over five on 12 processors`, to a tenth, so a Mac just past its
    /// room never reads as the load that has room.
    pub fn said(self) -> String {
        format!(
            "load {:.1} over a minute and {:.1} over five on {} processors",
            self.minute, self.five, self.cores
        )
    }
}

/// What a shell runs to say how many runs sent to its machine build there
/// now: its [`NEARBY_AT_ONCE`] turns ([`in_turn_nearby`]) held by a shell
/// alive in this boot, as a run waiting for a turn reads them. A machine
/// with no folder builds none.
pub fn building_read() -> String {
    format!(
        "{TURNS_READ} \
         boot=$({BOOT_READ}); building=0; k=1; \
         while [ \"$k\" -le {NEARBY_AT_ONCE} ]; do holder=$(readlink \"$turns/$k\" 2>/dev/null); \
         k=$((k + 1)); pid=${{holder%% *}}; \
         case \"$pid\" in ''|*[!0-9]*|0*|????????*) continue;; esac; \
         if [ -n \"$boot\" ] && [ \"${{holder#* }}\" = \"$boot\" ] && \
         kill -0 \"$pid\" 2>/dev/null; then building=$((building + 1)); fi; done; \
         echo \"$building\""
    )
}

/// What a Mac runs to say its load ([`Load::parse`]): its load and
/// processors, then the runs it builds ([`building_read`]), counted only
/// once the load was said, so a Mac whose `sysctl` fails says nothing.
pub fn load_read() -> String {
    format!("sysctl -n vm.loadavg hw.ncpu && {{ {}; }}", building_read())
}

/// A Mac nearby's load, `Err` saying why when it couldn't be asked here or
/// took past [`LOAD_PATIENCE`] to say it, and `Ok(None)` when it said
/// nothing a load reads from, as an ssh that couldn't reach it says
/// nothing. Every caller waits on either (`changed::mac::room`).
pub fn load_of(machine: &Nearby, scratch: &Path, ssh: &str) -> Result<Option<Load>, String> {
    if let Err(error) = fs::create_dir_all(scratch) {
        return Err(format!(
            "{}'s load couldn't be asked, since {} couldn't be made: {error}",
            machine.host,
            scratch.display()
        ));
    }
    let said = mine(scratch, "load.log");
    let spawned = reaching(ssh, &said)
        .args(["-o", "ConnectTimeout=20"])
        .arg(&machine.host)
        .arg(far(&load_read()))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let mut asking = match spawned {
        Ok(asking) => asking,
        Err(error) => {
            return Err(format!(
                "{}'s load couldn't be asked, since {ssh} wouldn't start: {error}",
                machine.host
            ));
        }
    };
    let began = Instant::now();
    let answered = loop {
        match asking.try_wait() {
            Ok(Some(ended)) => break Some(ended),
            // Asked each tenth of a second: an idle Mac says it at once.
            Ok(None) if began.elapsed() < LOAD_PATIENCE => {
                thread::sleep(Duration::from_millis(100));
            }
            _ => break None,
        }
    };
    let _ = fs::remove_file(&said);
    // ssh ends 255 on its own failure, a refused key or no route, which
    // is worth telling from a Mac that said nothing a load reads from.
    if answered.is_some_and(|ended| ended.code() == Some(255)) {
        return Err(format!(
            "ssh couldn't reach {} to ask its load",
            machine.host
        ));
    }
    if answered.is_none() {
        let _ = asking.kill();
        let _ = asking.wait();
        return Err(format!(
            "{} took past {} s to say its load",
            machine.host,
            LOAD_PATIENCE.as_secs()
        ));
    }
    let mut out = String::new();
    if let Some(mut stdout) = asking.stdout.take() {
        let _ = std::io::Read::read_to_string(&mut stdout, &mut out);
    }
    Ok(Load::parse(&out))
}

/// `part` put back with `rest`, what was left for the lent builder or
/// here, as one build of both's attributes.
pub fn rejoined(rest: Option<Asked>, part: &Asked) -> Asked {
    match rest {
        Some(rest) => {
            let mut attributes = rest.attributes.clone();
            attributes.extend(part.attributes.iter().cloned());
            rest.with(&rest.flake, attributes)
        }
        None => part.with(&part.flake, part.attributes.clone()),
    }
}

/// Builds what was asked on a machine nearby, sending a flake that's a
/// folder here with it, or none when ssh can't reach it, so what it was
/// to take goes with the rest. It was built here instead, unasked: on 8
/// November a run's part for a Mac gone quiet compiled biome here for 55
/// minutes beside Joel's work. `tar` and `nix` are the programs run here,
/// beside `ssh`.
pub fn nearby_run(
    machine: &Nearby,
    asked: &Asked,
    scratch: &Path,
    programs: [&str; 3],
) -> Option<Ran> {
    let [ssh, tar, nix] = programs;
    let why = match sent_nearby(machine, asked, scratch, ssh, tar, nix) {
        Ok(ran) if ran.status != UNREACHED => return Some(ran),
        Ok(_) => format!("ssh couldn't reach {}", machine.host),
        Err(error) => error,
    };
    eprintln!("nix-offload: {why}, so what it takes goes the other way");
    None
}

fn sent_nearby(
    machine: &Nearby,
    asked: &Asked,
    scratch: &Path,
    ssh: &str,
    tar: &str,
    nix: &str,
) -> Result<Ran, String> {
    fs::create_dir_all(scratch).map_err(|error| format!("{}: {error}", scratch.display()))?;
    let said = mine(scratch, "ssh.log");
    let mut sent: Option<u64> = None;
    let command = match folder_of(&asked.flake) {
        None => asked.command(),
        // A nearby machine is sent the tree's files as they stand, so once
        // they've moved from the commit this run is pinned at, it fails
        // rather than build, and be kept as passed at, what wasn't asked.
        Some(folder)
            if asked.pinned.as_ref().is_some_and(|pinned| {
                tree_now(&folder, scratch).as_ref() != Some(&pinned.tree)
            }) =>
        {
            eprintln!(
                "nix-offload: {} moved on from the commit this run began at, so {} builds \
                 nothing of it; run it again to build it as it is",
                folder.display(),
                machine.host
            );
            return Ok(Ran {
                on: Some(machine.host.clone()),
                status: 1,
                sent,
            });
        }
        Some(folder) => {
            let into = unpacked();
            let listed = mine(scratch, "files");
            let status = send(&folder, &into, &machine.host, &said, &listed, ssh, tar);
            let _ = fs::remove_file(&listed);
            let status = status?;
            counted(&mut sent, &said);
            if status != 0 {
                return Ok(Ran {
                    on: Some(machine.host.clone()),
                    status,
                    sent,
                });
            }
            copy_pinned(&folder, &machine.host, nix);
            // Built where it was unpacked, which goes once the build ends,
            // its status kept.
            format!(
                "cd {into} && {}; ended=$?; cd && rm -rf {into}; exit $ended",
                asked.with("path:.", asked.attributes.clone()).command()
            )
        }
    };
    eprintln!("nix-offload: building on {}, {}", machine.host, asked.flake);
    let mut reached = reaching(ssh, &said);
    reached
        .arg(&machine.host)
        .arg(far_saying_where(&in_turn_nearby(&command)));
    let unrun = |error: std::io::Error| format!("couldn't run {ssh}: {error}");
    let status = if asked.print {
        // What a machine nearby built comes here only signed by a key Nix
        // here trusts, never unchecked, so an account Nix doesn't trust
        // (ISXEG8's N5) brings nothing it can't check. One that signs
        // with no such key is left there, and what it took goes the
        // other way, to be built where it's signed or here.
        let store = format!("ssh-ng://{}", machine.host);
        let (built, paths) = built_paths(&mut reached).map_err(unrun)?;
        if built.success() && !signed_for_here(nix, &store, &paths) {
            counted(&mut sent, &said);
            return Err(format!(
                "{} signed what it built with no key Nix here trusts, so it isn't taken \
                 unchecked",
                machine.host
            ));
        }
        if built.success() {
            copied_back(built, &paths, &store, nix, Signatures::Checked).map_err(unrun)?
        } else {
            built
        }
    } else {
        reached.status().map_err(unrun)?
    };
    counted(&mut sent, &said);
    Ok(Ran {
        on: Some(machine.host.clone()),
        status: status.code().unwrap_or(UNREACHED),
        sent,
    })
}

/// Adds what ssh's log at `said` says it sent to `sent`, and drops the log.
fn counted(sent: &mut Option<u64>, said: &Path) {
    if let Some(bytes) = fs::read_to_string(said)
        .ok()
        .and_then(|log| self::sent(&log))
    {
        *sent = Some(sent.unwrap_or(0) + bytes);
    }
    let _ = fs::remove_file(said);
}

/// Sends `folder` to `into` on `host` as a compressed tarball and gives
/// ssh's status. In a git tree it holds the files git doesn't ignore,
/// listed in `listed`, as `packed` sends them: a working tree's zone held
/// 509 MB on 30 October, almost all one ignored target folder. Outside
/// one it holds the folder but its git history and the `result` links
/// builds leave there.
fn send(
    folder: &Path,
    into: &str,
    host: &str,
    said: &Path,
    listed: &Path,
    ssh: &str,
    tar: &str,
) -> Result<i32, String> {
    let mut packing = Command::new(tar);
    packing.arg("-czf").arg("-").arg("-C").arg(folder);
    match git(folder, UNIGNORED, None) {
        Some(files) => {
            fs::write(listed, files).map_err(|error| format!("{}: {error}", listed.display()))?;
            // A tracked file deleted and not yet committed is listed but
            // gone, which a build of the folder doesn't read either.
            packing
                .args(["--ignore-failed-read", "--null", "-T"])
                .arg(listed);
        }
        None => {
            packing.args(["--exclude=./.git", "--exclude=./result*", "."]);
        }
    }
    let mut packing = packing
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("couldn't run {tar}: {error}"))?;
    let packed = packing
        .stdout
        .take()
        .ok_or_else(|| format!("{tar} gave nothing to send"))?;
    let status = reaching(ssh, said)
        .arg(host)
        .arg(format!(
            "rm -rf {into} && mkdir -p {into} && tar -xzf - -C {into}"
        ))
        .stdin(packed)
        .status()
        .map_err(|error| format!("couldn't run {ssh}: {error}"))?;
    let packing = packing
        .wait()
        .map_err(|error| format!("couldn't run {tar}: {error}"))?;
    let status = status.code().unwrap_or(UNREACHED);
    // ssh refused, or lost, ends first, and tar is left writing to a
    // closed pipe: on 15 November a Mac that refused a gate's key was
    // said to be a tree tar couldn't pack.
    if status == UNREACHED {
        return Err(format!(
            "{ssh} couldn't reach {host} to send it {}",
            folder.display()
        ));
    }
    if !packing.success() {
        return Err(format!("{tar} couldn't pack {}", folder.display()));
    }
    Ok(status)
}

/// Copies to `host` the store paths `folder`'s lock pins inputs to. One
/// that can't be copied is said and left, since a build that never reads
/// that input still builds.
fn copy_pinned(folder: &Path, host: &str, nix: &str) {
    let paths = fs::read_to_string(folder.join("flake.lock"))
        .map(|lock| pinned(&lock))
        .unwrap_or_default();
    if paths.is_empty() {
        return;
    }
    // ssh's control socket goes in TMPDIR, and a long one is past what a
    // socket's path may be. What this machine built before it signed, or
    // on a machine that doesn't sign, carries no signature the machine
    // nearby trusts, so it takes the copy on its account's trust.
    let copied = upload(nix, COPY_ALIVE)
        .args([
            "copy",
            "--no-check-sigs",
            "--to",
            &format!("ssh-ng://{host}"),
        ])
        .args(&paths)
        .env("TMPDIR", "/tmp")
        .status();
    if !copied.is_ok_and(|status| status.success()) {
        eprintln!("nix-offload: couldn't copy the inputs pinned to the store to {host}");
    }
}

/// Adds a line for a build to `journal`: when it ended, how long it took,
/// what it built, where, its status and what it sent, and, where known,
/// `waited`, the seconds of that it waited for room on its builder and for
/// the builder to stand, so a queue shows before it's felt.
pub fn note(journal: &Path, at: u64, seconds: u64, waited: Option<u64>, asked: &Asked, ran: &Ran) {
    let mut line = serde_json::json!({
        "at": at,
        "seconds": seconds,
        "flake": asked.flake,
        "attributes": asked.attributes,
        "on": ran.on,
        "status": ran.status,
        "sent": ran.sent,
    });
    if let Some(waited) = waited {
        line["waited"] = waited.into();
    }
    let written = journal
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| {
            fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(journal)
        })
        .and_then(|mut file| writeln!(file, "{line}"));
    if let Err(error) = written {
        eprintln!(
            "nix-offload: couldn't journal the build in {}: {error}",
            journal.display()
        );
    }
}

/// Adds a line for checks `asked` names that weren't built, having passed
/// at their commit already: built nowhere, they sent nothing.
pub fn note_already(journal: &Path, at: u64, asked: &Asked, already: &[String]) {
    let line = serde_json::json!({
        "at": at,
        "seconds": 0,
        "flake": asked.flake,
        "attributes": already,
        "on": null,
        "status": 0,
        "sent": null,
        "already": true,
    });
    let written = journal
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| {
            fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(journal)
        })
        .and_then(|mut file| writeln!(file, "{line}"));
    if let Err(error) = written {
        eprintln!(
            "nix-offload: couldn't journal the build in {}: {error}",
            journal.display()
        );
    }
}

/// Where the flake in `folder` is when its git tree is its commit: none
/// with any change to what git tracks, which no commit names yet.
pub fn clean_at(folder: &Path) -> Option<passed::At> {
    let folder = fs::canonicalize(folder).ok()?;
    let top = PathBuf::from(git(&folder, &["rev-parse", "--show-toplevel"], None)?);
    git(&top, &["diff", "--quiet", "HEAD"], None)?;
    let head = git(&top, &["rev-parse", "HEAD"], None)?;
    let dir = folder
        .strip_prefix(&top)
        .ok()
        .map(|dir| dir.to_string_lossy().into_owned())
        .filter(|dir| !dir.is_empty());
    passed::At::new(&head, dir)
}

/// Where a source's flake is in its repository.
pub fn at_of(source: &Source) -> Option<passed::At> {
    match source {
        Source::Pushed { commit, dir, .. } | Source::Bundled { commit, dir, .. } => {
            passed::At::new(commit, dir.clone())
        }
    }
}

/// What ssh's verbose log says it sent, from its closing count:
/// `Transferred: sent 4321, received 98765 bytes, in 3.2 seconds`.
fn sent(log: &str) -> Option<u64> {
    log.lines().rev().find_map(|line| {
        line.split_once("Transferred: sent ")?
            .1
            .split_once(',')?
            .0
            .parse()
            .ok()
    })
}

/// A word the builder's shell reads as itself.
pub fn quoted(word: &str) -> String {
    if word
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b"-_./:".contains(&byte))
    {
        return word.to_owned();
    }
    format!("'{}'", word.replace('\'', r"'\''"))
}

/// Base64 as text, or none when it isn't both. Nix's store URIs carry a
/// host key this way, and it's the only base64 this reads, so it's
/// decoded here rather than with a crate.
pub(crate) fn decoded(encoded: &str) -> Option<String> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut bits: u32 = 0;
    let mut held = 0;
    let mut bytes = Vec::new();
    for byte in encoded.trim_end_matches('=').bytes() {
        let value = ALPHABET.iter().position(|letter| *letter == byte)?;
        bits = (bits << 6) | u32::try_from(value).ok()?;
        held += 6;
        if held >= 8 {
            held -= 8;
            bytes.push(u8::try_from(bits >> held).ok()?);
            bits &= (1 << held) - 1;
        }
    }
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::os::unix::process::CommandExt;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn a_macs_load_is_read_from_its_first_two_averages_or_not_at_all() {
        assert_eq!(
            Load::parse("{ 120.98 88.31 65.93 }\n12\n"),
            Some(Load {
                minute: 120.98,
                five: 88.31,
                cores: 12,
                building: 0,
            }),
            "a Mac's sysctl wasn't read as its minute's and five minutes' loads"
        );
        assert_eq!(
            Load::parse("{ 20.00 35.00 40.00 }\n12\n1\n").map(|load| load.building),
            Some(1),
            "the runs building on a Mac weren't read from the line after its processors"
        );
        assert_eq!(
            Load::parse("{20.00 60.00 80.00}\n12\n").map(|load| load.five),
            Some(60.0),
            "a brace against a figure shifted which figure was read"
        );
        assert_eq!(
            Load::parse("{ 0.00 0.00 0.00 }\n12\n").map(|load| load.minute),
            Some(0.0),
            "an idle Mac's load wasn't read"
        );
        for unread in [
            "{ 20.00 }\n12\n",
            "20.00 60.00 80.00\n12\n",
            "{ 20.00 NaN 80.00 }\n12\n",
            "{ 20.00 inf 80.00 }\n12\n",
            "{ 20.00 -1.00 80.00 }\n12\n",
            "{ 20.00 60.00 80.00 }\n0\n",
            "{ 20.00 60.00 80.00 }\n",
            "{ 20.00 60.00 }\n12\n",
            "{ 20.00 60.00 80.00 90.00 }\n12\n",
            "{ 20.00 60.00 80.00 }\n12\nmany\n",
            "{ 20.00 60.00 80.00 }\n12\n1\n2\n",
        ] {
            assert_eq!(
                Load::parse(unread),
                None,
                "{unread:?} was read as a load, perhaps from the wrong figure"
            );
        }
    }

    #[test]
    fn a_macs_builds_are_counted_by_the_live_turns_of_its_own_boot() {
        let folder = tempfile::tempdir().unwrap();
        let turns = folder.path().join("turns");
        fs::create_dir_all(&turns).unwrap();
        let boot = Command::new("sh").args(["-c", BOOT_READ]).output().unwrap();
        let boot = String::from_utf8(boot.stdout).unwrap().trim().to_owned();
        let mut live = Command::new("sleep").arg("30").spawn().unwrap();
        let pid = live.id();
        // Each holder but the first is passed over for one reason alone.
        let holders = [
            (format!("{pid} {boot}"), 1, "a live shell of this boot"),
            (
                format!("{pid} another-boot"),
                0,
                "a live pid of another boot",
            ),
            (format!("9999999 {boot}"), 0, "a dead pid of this boot"),
            (
                format!("0 {boot}"),
                0,
                "pid 0, which kill reads as its own group",
            ),
            (format!("0{pid} {boot}"), 0, "a pid led by 0"),
            (
                format!("99999999999999999999 {boot}"),
                0,
                "a pid past any machine's",
            ),
        ];
        // Every count is read before any is judged, so the live shell is
        // killed however the test ends.
        let mut read = Vec::new();
        for shell in ["dash", "zsh"] {
            let counted = |turns: &Path| {
                Command::new(shell)
                    .args(["-c", &building_read()])
                    .env("NIX_OFFLOAD_TURNS", turns)
                    .output()
                    .map_or_else(
                        |error| format!("{error}"),
                        |said| String::from_utf8_lossy(&said.stdout).into_owned(),
                    )
            };
            for (holder, building, what) in &holders {
                let _ = fs::remove_file(turns.join("1"));
                std::os::unix::fs::symlink(holder, turns.join("1")).unwrap();
                read.push((
                    counted(&turns),
                    format!("{building}\n"),
                    format!("{shell}: a turn held by {what} was counted wrongly"),
                ));
            }
            let _ = fs::remove_file(turns.join("1"));
            let past = turns.join((NEARBY_AT_ONCE + 1).to_string());
            std::os::unix::fs::symlink(format!("{pid} {boot}"), &past).unwrap();
            read.push((
                counted(&turns),
                "0\n".to_owned(),
                format!("{shell}: a link past the machine's turns was counted as one"),
            ));
            let _ = fs::remove_file(&past);
            read.push((
                counted(&folder.path().join("none")),
                "0\n".to_owned(),
                format!("{shell}: a Mac with no turns folder wasn't read as building nothing"),
            ));
        }
        live.kill().unwrap();
        live.wait().unwrap();
        for (counted, building, why) in read {
            assert_eq!(counted, building, "{why}");
        }
    }

    #[test]
    fn a_macs_load_is_read_whole_and_counted_only_once_said() {
        use std::os::unix::fs::PermissionsExt;
        let folder = tempfile::tempdir().unwrap();
        let bin = folder.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        let sysctl = bin.join("sysctl");
        // The fake goes first on PATH inside the command: handed in, the
        // Mac's own sysctl answered, and on 18 November nix-offload didn't
        // build on a Mac. It says 7 processors, which no Mac here has, so
        // the real one answering fails the first assertion too.
        let reading = first_on_path(&bin, &load_read());
        for shell in ["dash", "zsh"] {
            let read = |says: &str| {
                fs::write(&sysctl, format!("#!/bin/sh\n{says}\n")).unwrap();
                fs::set_permissions(&sysctl, fs::Permissions::from_mode(0o755)).unwrap();
                let said = Command::new(shell)
                    .args(["-c", &reading])
                    .env("NIX_OFFLOAD_TURNS", folder.path().join("none"))
                    .output()
                    .unwrap();
                String::from_utf8(said.stdout).unwrap()
            };
            let said = read("printf '{ 20.00 35.00 30.00 }\\n7\\n'");
            assert_eq!(
                Load::parse(&said).map(|load| (load.cores, load.building)),
                Some((7, 0)),
                "{shell}: a Mac's whole answer didn't read as its load: {said:?}"
            );
            assert_eq!(
                read("exit 1"),
                "",
                "{shell}: a Mac whose sysctl failed still counted its builds"
            );
        }
    }

    #[test]
    fn a_mac_has_no_room_once_either_load_is_past_three_a_processor() {
        let at = |minute: f64, five: f64| Load {
            minute,
            five,
            cores: 12,
            building: 0,
        };
        assert!(
            at(20.0, 60.0).over(),
            "a Mac busy over five minutes had room in a lull"
        );
        assert!(
            at(60.0, 20.0).over(),
            "a Mac whose build has just begun had room"
        );
        assert!(
            !at(36.0, 36.0).over(),
            "a Mac at three a processor had no room"
        );
    }

    #[test]
    fn a_runs_own_ssh_options_come_before_the_copys_keepalive() {
        let given = "-o ServerAliveCountMax=40 -o Compression=yes";
        assert_eq!(
            ahead_of_alive(Some(given), COPY_ALIVE),
            format!("{given} {COPY_ALIVE}"),
            "the run's own options came after the copy's, where ssh ignores them"
        );
        assert_eq!(
            ahead_of_alive(Some(" "), COPY_ALIVE),
            COPY_ALIVE,
            "blank options weren't left out"
        );
        assert_eq!(
            ahead_of_alive(None, COPY_ALIVE),
            COPY_ALIVE,
            "with none given, the keepalive was lost"
        );
    }

    #[test]
    fn a_copy_refused_the_pinned_key_lost_the_builder() {
        // As `nix copy` says it once a lender has handed its name to a
        // builder with another key: nothing `LOST_SAID` holds, so only the
        // refusal makes the builder lost.
        let mut copying = Copying::start(Command::new("sh").args([
            "-c",
            "echo 'Host key verification failed.' >&2; \
             echo \"error: failed to start SSH master connection to 'nix-offload'\" >&2; \
             exit 1",
        ]))
        .unwrap();
        copying.child.wait().unwrap();
        let uncopied = Uncopied::of("x went up to nothing".to_owned(), copying.heard());
        assert!(uncopied.refused, "the refusal went unheard");
        assert!(uncopied.lost, "a refused builder wasn't taken for lost");
        assert!(
            uncopied
                .why
                .ends_with("since ssh couldn't verify the host key it showed"),
            "it doesn't say why: {}",
            uncopied.why
        );
        let mut failed =
            Copying::start(Command::new("sh").args(["-c", "echo no >&2; exit 1"])).unwrap();
        failed.child.wait().unwrap();
        let uncopied = Uncopied::of("x".to_owned(), failed.heard());
        assert!(
            !uncopied.refused && !uncopied.lost,
            "a copy's own failure read as ssh's"
        );
    }

    #[test]
    fn a_copy_sent_again_after_its_keepalive_went_unanswered_waits_a_minute() {
        let waits = |cuts: u32| {
            let alive = copy_alive(cuts);
            let count: u64 = alive.rsplit('=').next().unwrap().parse().unwrap();
            assert!(
                alive.starts_with("-o ServerAliveInterval=5 "),
                "the keepalive's interval moved: {alive}"
            );
            5 * count
        };
        assert_eq!(copy_alive(0), COPY_ALIVE, "the first copy's cut moved");
        assert_eq!(
            [1, 2, 40].map(waits),
            [60, 60, 60],
            "a copy sent again wasn't given a minute"
        );
        assert_eq!(
            [0, 1, 40].map(waits),
            [0, 1, 40].map(|cuts| u64::from(alive_for(cuts))),
            "what the run says it waits isn't what ssh is given"
        );
    }

    #[test]
    fn an_upload_runs_in_a_scope_of_its_own_where_systemd_can_make_one() {
        let args = |command: &Command| -> Vec<String> {
            command
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect()
        };
        let scoped = uploading("/bin/nix", Some("/bin/systemd-run"), COPY_ALIVE);
        assert_eq!(scoped.get_program(), "/bin/systemd-run");
        assert_eq!(
            args(&scoped),
            [
                "--user",
                "--scope",
                "--quiet",
                "--collect",
                "--slice-inherit",
                "-p",
                "MemoryHigh=1G",
                "--",
                "/bin/nix"
            ],
            "beside its caller, its cache bounded"
        );
        let alone = uploading("/bin/nix", None, COPY_ALIVE);
        assert_eq!(alone.get_program(), "/bin/nix");
        assert_eq!(
            args(&alone),
            Vec::<String>::new(),
            "as it is, with no systemd"
        );
    }

    #[test]
    fn a_send_is_given_two_minutes_and_its_size_at_a_megabyte_a_second_or_fifteen() {
        // A small one is given 15 minutes: on 9 November a Go zone's
        // 106 MB of modules went at 180 kB/s and was stopped at 4.
        assert_eq!(send_within(0), Duration::from_mins(15));
        assert_eq!(send_within(106_400_000), Duration::from_mins(15));
        assert_eq!(send_within(1_080_000_000), Duration::from_mins(2 + 18));
        // The 1.4 GB that crawled on 9 November gets 25 minutes, not two hours.
        assert_eq!(send_within(1_400_000_000), Duration::from_secs(120 + 1400));
    }

    /// A turn locked in `folder`, and a second handle on its file to ask
    /// whether it's still held.
    fn a_turn(folder: &Path) -> (Turn, fs::File) {
        let at = folder.join("uplink-1");
        let turn = fs::File::create(&at).expect("a turn");
        turn.try_lock().expect("the turn, free");
        let turn = Turn {
            _held: turn,
            folder: folder.to_owned(),
        };
        (turn, fs::File::open(&at).expect("the turn, again"))
    }

    /// A lease of `seconds` for a copy's turn, as [`turn_within`] gives.
    fn lease(seconds: u64) -> (Duration, &'static str) {
        (Duration::from_secs(seconds), "as a test gives")
    }

    /// Every turn of `kind` in `folder`, held.
    fn all_held(folder: &Path, kind: &str) -> Vec<fs::File> {
        (1..=UPLOADS)
            .map(|turn| {
                let file = fs::File::create(folder.join(format!("{kind}-{turn}"))).expect("a turn");
                file.try_lock().expect("the turn, free");
                file
            })
            .collect()
    }

    /// How long `other`'s turn stays held, looked at every 50 ms, as the
    /// copy's waiter looks every 100 ms, up to 10 s.
    fn held_for(other: &fs::File, began: std::time::Instant) -> Duration {
        while other.try_lock().is_err() && began.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(50));
        }
        let held = began.elapsed();
        other.unlock().expect("the turn, let go");
        held
    }

    /// [`waited_giving_back`] on `copy`, in a thread of its own, with what
    /// it said.
    fn waited_on(
        mut copy: std::process::Child,
        turn: Turn,
        within: (Duration, &'static str),
    ) -> std::thread::JoinHandle<(std::io::Result<std::process::ExitStatus>, Vec<String>)> {
        std::thread::spawn(move || {
            let mut said = Vec::new();
            let mut say = |line: String| said.push(line);
            let ended =
                waited_giving_back(&mut copy, &mut Some(turn), Some(within), "to b", &mut say);
            (ended, said)
        })
    }

    /// How many of the given turns in `folder` are held elsewhere now.
    fn given_held(folder: &Path) -> usize {
        (1..=UPLOADS)
            .filter(|turn| {
                fs::File::open(folder.join(format!("uplink-given-{turn}")))
                    .is_ok_and(|file| matches!(file.try_lock(), Err(fs::TryLockError::WouldBlock)))
            })
            .count()
    }

    #[test]
    fn a_copy_slower_than_its_share_gives_its_turn_back_and_goes_on() {
        let folder = tempfile::tempdir().expect("a folder");
        let (turn, other) = a_turn(folder.path());
        let began = std::time::Instant::now();
        let copy = Command::new("sh")
            .args(["-c", "sleep 3; exit 1"])
            .spawn()
            .expect("a copy");
        let waiting = waited_on(copy, turn, lease(1));
        let held = held_for(&other, began);
        assert!(
            held >= Duration::from_millis(900) && held < Duration::from_millis(2500),
            "the turn of a copy given 1 s of a 3 s copy went after {held:?}"
        );
        // The given turn it took is what bounds the copies going up, so it
        // stays held while the copy goes on, and goes once it ends.
        assert_eq!(
            given_held(folder.path()),
            1,
            "a copy that gave its turn back holds no given turn while it goes on"
        );
        let (ended, said) = waiting.join().expect("the waiter");
        let ended = ended.expect("how the copy ended");
        // It fails past its turn, which must still read as a failure.
        assert!(
            began.elapsed() >= Duration::from_secs(3) && !ended.success(),
            "a copy that went on past its turn didn't end as it would have"
        );
        assert_eq!(
            given_held(folder.path()),
            0,
            "a copy that ended still holds its given turn"
        );
        let holder = fs::read_to_string(folder.path().join("uplink-given-1")).unwrap_or_default();
        assert!(
            holder.contains(&format!("pid {}, copying up to b", std::process::id())),
            "a given turn didn't name who took it: {holder:?}"
        );
        assert_eq!(
            said.len(),
            1,
            "giving a turn back wasn't said once: {said:?}"
        );
    }

    #[test]
    fn a_copy_keeps_its_turn_while_every_given_turn_is_taken() {
        let folder = tempfile::tempdir().expect("a folder");
        let (turn, other) = a_turn(folder.path());
        let given = all_held(folder.path(), "uplink-given");
        let began = std::time::Instant::now();
        let copy = Command::new("sleep").arg("2").spawn().expect("a copy");
        let waiting = waited_on(copy, turn, lease(1));
        let held = held_for(&other, began);
        // Held past its lease while none is free, and let go at its end.
        assert!(
            held >= Duration::from_millis(1900) && held < Duration::from_secs(3),
            "a copy of 2 s with every given turn taken held its turn for {held:?}"
        );
        let (ended, said) = waiting.join().expect("the waiter");
        assert!(
            ended.expect("how it ended").success(),
            "a copy that succeeded read as one that failed"
        );
        assert!(
            said.len() == 1 && said[0].contains("so it keeps it until one ends"),
            "keeping a turn wasn't said once: {said:?}"
        );
        drop(given);
    }

    #[test]
    fn a_copy_that_kept_its_turn_gives_it_back_once_a_given_turn_frees() {
        let folder = tempfile::tempdir().expect("a folder");
        let (turn, other) = a_turn(folder.path());
        let mut given = all_held(folder.path(), "uplink-given");
        let began = std::time::Instant::now();
        let copy = Command::new("sleep").arg("4").spawn().expect("a copy");
        let waiting = waited_on(copy, turn, lease(1));
        // Past its lease, kept, and then one of the given turns frees,
        // late enough that what it held reads apart from its lease.
        std::thread::sleep(Duration::from_millis(2500));
        drop(given.remove(1));
        let held = held_for(&other, began);
        assert!(
            held >= Duration::from_millis(2400) && held < Duration::from_millis(3900),
            "a kept turn went {held:?} into a 4 s copy whose given turn freed at 2.5 s"
        );
        let (ended, said) = waiting.join().expect("the waiter");
        assert!(
            ended.expect("how it ended").success(),
            "a copy that succeeded read as one that failed"
        );
        // How long it held its turn, not its lease: whole seconds, so what
        // the bound on `held` above allows reads as 2 or 3.
        assert!(
            said.len() == 2
                && said[0].contains("so it keeps it until one ends")
                && said[0].contains("has held its turn for 1 second,")
                && ["2 seconds,", "3 seconds,"]
                    .iter()
                    .any(|held| said[1].contains(&format!("has held its turn for {held}"))),
            "keeping and then giving back weren't said once each: {said:?}"
        );
        drop(given);
    }

    /// A systemd-run in `folder` that says `says` on stderr and fails, as
    /// one whose user manager's bus is gone does, never running its command.
    fn a_scope_refused(folder: &Path, says: &str) -> String {
        use std::os::unix::fs::PermissionsExt;
        let systemd_run = folder.join("systemd-run");
        fs::write(
            &systemd_run,
            format!("#!/bin/sh\necho '{says}' >&2\nexit 1\n"),
        )
        .expect("a systemd-run");
        fs::set_permissions(&systemd_run, fs::Permissions::from_mode(0o755))
            .expect("a systemd-run, runnable");
        systemd_run.to_string_lossy().into_owned()
    }

    /// [`copied_up`] through `systemd_run`, with `sh` standing in for Nix
    /// and copying for a second, each run it makes noted in `folder`; how
    /// it ended, how many copies ran and how long `turn` stayed held.
    fn copied_through(folder: &Path, systemd_run: &str) -> (bool, usize, Duration) {
        let (turn, other) = a_turn(folder);
        let ran = folder.join("ran");
        let args = [
            "-c".to_owned(),
            format!("echo once >> '{}'; sleep 1", ran.display()),
        ];
        let began = std::time::Instant::now();
        let systemd_run = systemd_run.to_owned();
        let copying = std::thread::spawn(move || {
            copied_up(
                ["sh", COPY_ALIVE],
                Some(&systemd_run),
                &args,
                Some(turn),
                None,
                "to b",
            )
        });
        let held = held_for(&other, began);
        let (ended, _) = copying.join().expect("the copier").expect("a copy run");
        let ran = fs::read_to_string(&ran).unwrap_or_default().lines().count();
        (ended.is_ok_and(|status| status.success()), ran, held)
    }

    #[test]
    fn a_copy_refused_a_scope_by_the_bus_goes_again_outside_one_keeping_its_turn() {
        let folder = tempfile::tempdir().expect("a folder");
        // What systemd-run said on 23 November, once the user manager's
        // bus had gone with its reload.
        let systemd_run = a_scope_refused(
            folder.path(),
            "Failed to connect to user scope bus via local transport: Connection refused",
        );
        let (copied, ran, held) = copied_through(folder.path(), &systemd_run);
        assert!(copied, "a copy refused only its scope didn't go up");
        assert_eq!(
            ran, 1,
            "a copy refused its scope didn't run once, outside one"
        );
        assert!(
            held >= Duration::from_millis(900),
            "the turn went {held:?} into a copy sent again, before it ran"
        );
    }

    #[test]
    fn a_copy_whose_scope_failed_otherwise_isnt_sent_again() {
        let folder = tempfile::tempdir().expect("a folder");
        let systemd_run = a_scope_refused(folder.path(), "Unknown assignment: MemoryHigh=1G");
        let (copied, ran, _) = copied_through(folder.path(), &systemd_run);
        assert!(
            !copied && ran == 0,
            "a scope refused for another cause sent the copy again: copied {copied}, ran {ran}"
        );
    }

    #[test]
    fn a_turns_file_that_is_a_link_is_refused() {
        let folder = tempfile::tempdir().expect("a folder");
        let elsewhere = folder.path().join("elsewhere");
        std::os::unix::fs::symlink(&elsewhere, folder.path().join("uplink-given-2"))
            .expect("a link");
        let (opened, unopened) = turn_files(folder.path(), "uplink-given");
        assert!(
            opened.len() == 2 && unopened.len() == 1 && unopened[0].contains("uplink-given-2"),
            "a turn's file that links elsewhere was opened, or the rest weren't: {unopened:?}"
        );
        assert!(
            !elsewhere.exists(),
            "opening the turns made the file a link points at"
        );
    }

    #[test]
    fn only_turns_held_now_are_named() {
        let folder = tempfile::tempdir().expect("a folder");
        let mut held = all_held(folder.path(), "uplink");
        for (turn, about) in held.iter().zip(["to b1", "to b2", "to b3"]) {
            holding(turn, about);
        }
        drop(held.remove(1));
        let named = holders(folder.path(), "uplink");
        let pid = std::process::id();
        assert_eq!(
            named,
            format!("pid {pid}, copying up to b1; pid {pid}, copying up to b3"),
            "the turns' holders weren't named as held"
        );
    }

    // Words written again over longer ones leave one line alone, not the
    // first's tail or a run of NULs before the second.
    #[test]
    fn a_holder_written_again_reads_as_one_line() {
        let folder = tempfile::tempdir().expect("a folder");
        let held = all_held(folder.path(), "uplink");
        holding(&held[0], "to a-builder-with-a-long-name");
        holding(&held[0], "to b2");
        holding(&held[0], "to b3");
        assert_eq!(
            fs::read_to_string(folder.path().join("uplink-1")).expect("its words"),
            format!("pid {}, copying up to b3\n", std::process::id()),
            "a turn's holder written thrice didn't read as the last alone"
        );
    }

    // A turn only looked at, with a shared lock as a waiter looks, isn't
    // held, so its last holder's words aren't named, nor are a link's to a
    // turn held elsewhere.
    #[test]
    fn a_turn_looked_at_or_linked_names_no_holder() {
        let folder = tempfile::tempdir().expect("a folder");
        let held = all_held(folder.path(), "uplink");
        for turn in &held {
            holding(turn, "to b1");
        }
        drop(held);
        let looking = fs::File::open(folder.path().join("uplink-1")).expect("a turn");
        looking.try_lock_shared().expect("a look at a free turn");
        let elsewhere = tempfile::tempdir().expect("a folder elsewhere");
        let other = fs::File::create(elsewhere.path().join("held")).expect("a file");
        other.try_lock().expect("the file, held");
        holding(&other, "to elsewhere");
        fs::remove_file(folder.path().join("uplink-2")).expect("a turn gone");
        std::os::unix::fs::symlink(
            elsewhere.path().join("held"),
            folder.path().join("uplink-2"),
        )
        .expect("a link");
        assert_eq!(
            holders(folder.path(), "uplink"),
            "none of them named",
            "a turn only looked at, or a link to one held elsewhere, was named"
        );
        drop((looking, other));
    }

    #[test]
    fn only_words_naming_a_run_that_still_runs_name_a_holder() {
        let mut ended = Command::new("true").spawn().expect("a run");
        ended.wait().expect("the run, ended");
        let pid = std::process::id();
        for (holder, running) in [
            (format!("pid {pid}, copying up to b1"), true),
            (format!("pid {}, copying up to b1", ended.id()), false),
            ("pid 0, copying up to b1".to_owned(), false),
            ("pid -1, copying up to b1".to_owned(), false),
            (String::new(), false),
            ("a run of another kind".to_owned(), true),
        ] {
            assert_eq!(
                still_running(&holder),
                running,
                "{holder:?} read as a run that still runs: {}",
                !running
            );
        }
    }

    // A FIFO among the turns is refused at once rather than hanging its
    // open, as is a turn's file with a second name elsewhere.
    #[test]
    fn a_turns_file_that_is_a_fifo_or_another_name_is_refused() {
        use std::os::unix::ffi::OsStrExt;
        let folder = tempfile::tempdir().expect("a folder");
        let fifo = folder.path().join("uplink-given-1");
        let name = std::ffi::CString::new(fifo.as_os_str().as_bytes()).expect("its name");
        // SAFETY: the name is a valid C string that outlives the call.
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0, "a FIFO");
        let elsewhere = folder.path().join("elsewhere");
        fs::write(&elsewhere, "kept").expect("a file elsewhere");
        fs::hard_link(&elsewhere, folder.path().join("uplink-given-3")).expect("a second name");
        let (opened, unopened) = turn_files(folder.path(), "uplink-given");
        assert!(
            opened.len() == 1
                && unopened.len() == 2
                && unopened[0].contains("uplink-given-1")
                && unopened[1].contains("uplink-given-3"),
            "a FIFO or a second name was opened as a turn: {unopened:?}"
        );
        assert_eq!(
            fs::read_to_string(&elsewhere).expect("the file elsewhere"),
            "kept",
            "a file with a turn's second name was changed"
        );
        assert_eq!(
            holders(folder.path(), "uplink-given"),
            "none of them named",
            "a FIFO or a second name was read for a holder"
        );
    }

    // A turn's file that can't be locked at all (here one opened only as a
    // path, which flock refuses) is taken out and said, and the free one
    // beside it is still taken. Linux alone opens a file only as a path,
    // and a Mac's libc has no O_PATH, so its tests wouldn't build.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_turn_that_cant_be_locked_is_left_out_for_the_one_beside_it() {
        use std::os::unix::fs::OpenOptionsExt;
        let folder = tempfile::tempdir().expect("a folder");
        let (pathed, plain) = (folder.path().join("pathed"), folder.path().join("plain"));
        fs::write(&pathed, "").expect("a turn");
        let pathed = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_PATH)
            .open(&pathed)
            .expect("a turn opened as a path");
        let plain = open_turn(&plain, true).expect("a turn");
        let mut files = vec![pathed, plain];
        let mut unlocked = Vec::new();
        let taken = free_turn(&mut files, &mut unlocked);
        assert!(
            taken.is_some() && files.is_empty() && unlocked.len() == 1,
            "a turn that can't be locked kept the free one from its copy: {unlocked:?}"
        );
    }

    #[test]
    fn copies_are_counted_in_words() {
        assert_eq!(
            (copies(1), copies(3)),
            ("1 copy".to_owned(), "3 copies".to_owned()),
            "the copies counted were said wrong"
        );
    }

    // Opened only to read, as a holder's words are, a FIFO, a folder and
    // a second name are each refused at once, never waited on or read.
    #[test]
    fn a_turn_opened_to_read_refuses_a_fifo_a_folder_or_a_second_name() {
        use std::os::unix::ffi::OsStrExt;
        let folder = tempfile::tempdir().expect("a folder");
        let fifo = folder.path().join("fifo");
        let name = std::ffi::CString::new(fifo.as_os_str().as_bytes()).expect("its name");
        // SAFETY: the name is a valid C string that outlives the call.
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0, "a FIFO");
        let inner = folder.path().join("inner");
        fs::create_dir(&inner).expect("a folder within");
        let elsewhere = folder.path().join("elsewhere");
        fs::write(&elsewhere, "kept").expect("a file elsewhere");
        let second = folder.path().join("second");
        fs::hard_link(&elsewhere, &second).expect("a second name");
        let (told, heard) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let refused = [fifo, inner, second]
                .iter()
                .map(|at| open_turn(at, false).is_err())
                .collect::<Vec<_>>();
            let _ = told.send(refused);
        });
        assert_eq!(
            heard.recv_timeout(Duration::from_secs(5)),
            Ok(vec![true, true, true]),
            "a FIFO, a folder or a second name was opened to read, or its open hung"
        );
    }

    // A turn just taken may still hold the words of a run that has ended,
    // which aren't named as its holder.
    #[test]
    fn a_holder_whose_run_has_ended_is_not_named() {
        let folder = tempfile::tempdir().expect("a folder");
        let held = all_held(folder.path(), "uplink");
        let mut ended = Command::new("true").spawn().expect("a run");
        ended.wait().expect("the run, ended");
        let words = format!("pid {}, copying up to b1\n", ended.id());
        fs::write(folder.path().join("uplink-1"), words).expect("its words");
        holding(&held[1], "to b2");
        fs::write(folder.path().join("uplink-3"), "").expect("no words");
        assert_eq!(
            holders(folder.path(), "uplink"),
            format!("pid {}, copying up to b2", std::process::id()),
            "an ended run, or no words, was named as a holder"
        );
    }

    #[test]
    fn a_knob_set_wrong_is_said_once() {
        let warned = std::sync::Mutex::new(BTreeSet::new());
        let (seconds, first) = knob_seconds("NIX_OFFLOAD_TURN_WITHIN", "90s", &warned);
        let (_, again) = knob_seconds("NIX_OFFLOAD_TURN_WITHIN", "90s", &warned);
        let (_, other) = knob_seconds("NIX_OFFLOAD_SEND_WITHIN", "x", &warned);
        assert!(
            seconds.is_none() && first.is_some() && again.is_none() && other.is_some(),
            "a knob set wrong wasn't said once each: {first:?} {again:?} {other:?}"
        );
        assert_eq!(
            knob_seconds("NIX_OFFLOAD_TURN_WITHIN", "7", &warned),
            (Some(7), None),
            "a knob set right wasn't read, or was said"
        );
    }

    // The turns folder is the account's own: a link there, or another
    // account's folder, is refused, and one open to others is closed.
    #[test]
    fn a_turns_folder_is_the_accounts_own_and_closed_to_others() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let base = tempfile::tempdir().expect("a folder");
        // SAFETY: geteuid has no preconditions and can't fail.
        let uid = unsafe { libc::geteuid() };
        let runtime = base.path().join("runtime");
        fs::create_dir(&runtime).expect("a runtime folder");
        let temp = tempfile::tempdir().expect("a temp folder");
        let in_temp = Ok(temp.path().join(format!("nix-offload-there-{uid}")));
        for mode in [0o775, 0o757] {
            fs::set_permissions(&runtime, fs::Permissions::from_mode(mode)).expect("opened");
            assert_eq!(
                turns_folder_in(Some(runtime.clone()), temp.path(), uid),
                in_temp,
                "a runtime folder others may write ({mode:o}) was used, not the temp folder"
            );
        }
        fs::set_permissions(&runtime, fs::Permissions::from_mode(0o755)).expect("closed");
        let linked = base.path().join("linked");
        std::os::unix::fs::symlink(&runtime, &linked).expect("a link to it");
        let relative = PathBuf::from("runtime");
        for (unfit, what) in [
            (linked, "a link to one of its own"),
            (PathBuf::new(), "empty"),
            (relative, "relative"),
        ] {
            assert_eq!(
                turns_folder_in(Some(unfit), temp.path(), uid),
                in_temp,
                "a runtime folder that's {what} was used, not the temp folder"
            );
        }
        assert!(
            turns_folder_in(
                Some(runtime.clone()),
                base.path().join("gone").as_path(),
                uid + 1
            )
            .is_err_and(|why| why.contains("isn't a runtime folder of this account's alone")),
            "another account's runtime folder was taken, or not said to be another's"
        );
        let made = turns_folder_in(Some(runtime.clone()), base.path(), uid).expect("its folder");
        assert_eq!(
            made,
            runtime.join("nix-offload-there"),
            "it isn't the runtime's"
        );
        fs::set_permissions(&made, fs::Permissions::from_mode(0o755)).expect("opened");
        turns_folder_in(Some(runtime), base.path(), uid).expect("its folder again");
        assert_eq!(
            fs::metadata(&made).expect("the folder").mode() & 0o777,
            0o700,
            "a turns folder open to others wasn't closed"
        );
        let elsewhere = base.path().join("elsewhere");
        fs::create_dir(&elsewhere).expect("a folder elsewhere");
        std::os::unix::fs::symlink(
            &elsewhere,
            base.path().join(format!("nix-offload-there-{uid}")),
        )
        .expect("a link");
        assert!(
            turns_folder_in(None, base.path(), uid)
                .is_err_and(|why| why.contains("isn't a folder")),
            "a turns folder that links elsewhere was taken"
        );
        assert!(
            turns_folder_in(None, base.path(), uid + 1).is_err(),
            "another account's turns folder was taken"
        );
        let fallen = tempfile::tempdir().expect("a folder");
        let gone = fallen.path().join("gone");
        assert_eq!(
            turns_folder_in(Some(gone), fallen.path(), uid),
            Ok(fallen.path().join(format!("nix-offload-there-{uid}"))),
            "a runtime folder that can't be used didn't fall back to the temp folder"
        );
        let neither = turns_folder_in(Some(base.path().join("gone")), base.path(), uid)
            .expect_err("a linked temp folder, with no runtime folder to use");
        assert!(
            neither.contains("isn't a runtime folder of this account's alone")
                && neither.contains("isn't a folder"),
            "neither folder's cause was said: {neither}"
        );
    }

    // A waiter whose turns can't all be opened says so once and waits on
    // the rest; with none it can open, it takes none, saying so.
    #[test]
    fn a_waiter_waits_on_the_turns_that_open_and_says_of_the_rest() {
        let folder = tempfile::tempdir().expect("a folder");
        let elsewhere = folder.path().join("elsewhere");
        std::os::unix::fs::symlink(&elsewhere, folder.path().join("uplink-2")).expect("a link");
        let mut held: Vec<fs::File> = [1, 3]
            .iter()
            .map(|turn| {
                let file =
                    fs::File::create(folder.path().join(format!("uplink-{turn}"))).expect("a turn");
                file.try_lock().expect("the turn, free");
                file
            })
            .collect();
        let at = folder.path().to_owned();
        let waiting = std::thread::spawn(move || {
            let mut said = Vec::new();
            let had = turn_in(&at, "to b", TURN_SAID_EVERY, &mut |line| said.push(line));
            (had.is_some(), said)
        });
        std::thread::sleep(Duration::from_millis(300));
        assert!(
            !waiting.is_finished(),
            "it didn't wait on the turns that open"
        );
        drop(held.remove(1));
        let (had, said) = waiting.join().expect("the waiter");
        assert!(
            had && said.len() == 2
                && said[0].contains("uplink-2")
                && said[0].contains("waits on the other 2")
                && said[1].contains("2 copies already go up")
                && said[1].contains("this waits its turn to send"),
            "the unusable turn wasn't said once before the wait: {said:?}"
        );
        drop(held);
        let none = tempfile::tempdir().expect("a folder");
        for turn in 1..=UPLOADS {
            std::os::unix::fs::symlink(&elsewhere, none.path().join(format!("uplink-{turn}")))
                .expect("a link");
        }
        let mut said = Vec::new();
        let had = turn_in(none.path(), "to b", TURN_SAID_EVERY, &mut |line| {
            said.push(line);
        });
        assert!(
            had.is_none() && said.len() == 1 && said[0].contains("takes no turn at sending"),
            "a waiter with no usable turn didn't say it takes none: {said:?}"
        );
        assert!(!elsewhere.exists(), "a turn's link was followed");
    }

    // A waiter says again, every while it's given and no oftener, how
    // long it has waited, naming who holds the turns each time. A turn is
    // freed only once it has said its third line, so the test waits on
    // what it says rather than on the clock.
    #[test]
    fn a_waiter_says_again_how_long_it_has_waited() {
        let folder = tempfile::tempdir().expect("a folder");
        let mut held = all_held(folder.path(), "uplink");
        for turn in &held {
            holding(turn, "to b1");
        }
        let at = folder.path().to_owned();
        let (thrice, said_thrice) = std::sync::mpsc::channel();
        let every = Duration::from_millis(200);
        let waiting = std::thread::spawn(move || {
            let mut said = Vec::new();
            let had = turn_in(&at, "to b", every, &mut |line| {
                said.push((std::time::Instant::now(), line));
                if said.len() == 3 {
                    let _ = thrice.send(());
                }
            });
            (had.is_some(), said)
        });
        said_thrice
            .recv_timeout(Duration::from_secs(20))
            .expect("three lines said while it waited");
        drop(held.remove(0));
        let (had, said) = waiting.join().expect("the waiter");
        let pid = std::process::id();
        let named = format!("pid {pid}, copying up to b1");
        let lines: Vec<&String> = said.iter().map(|(_, line)| line).collect();
        assert!(
            had && lines.len() >= 3
                && lines[0].contains("this waits its turn to send")
                && lines[1..]
                    .iter()
                    .all(|line| line.contains("this has waited"))
                && lines.iter().all(|line| line.contains(&named)),
            "the wait wasn't said again, naming its holders: {lines:?}"
        );
        // A look comes every tenth of a second, so lines said on every
        // look, rather than every while, come closer than this.
        let apart: Vec<Duration> = said.windows(2).map(|two| two[1].0 - two[0].0).collect();
        assert!(
            apart.iter().all(|gap| *gap >= Duration::from_millis(150)),
            "the wait was said oftener than every {every:?}: {apart:?}"
        );
    }

    // A copy started holding a turn ends once what started it does: here
    // a thread, which Linux counts as its parent, as it counts a run.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_copy_ends_with_what_started_it() {
        use std::os::unix::process::ExitStatusExt;
        let starting = std::thread::spawn(|| {
            dies_with_run(Command::new("sleep").arg("30"))
                .spawn()
                .expect("a copy")
        });
        let mut copy = starting.join().expect("the thread that started it");
        let began = std::time::Instant::now();
        let ended = loop {
            if let Some(ended) = copy.try_wait().expect("how it ended") {
                break Some(ended);
            }
            if began.elapsed() > Duration::from_secs(5) {
                let _ = copy.kill();
                break None;
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        assert_eq!(
            ended.and_then(|ended| ended.signal()),
            Some(libc::SIGTERM),
            "a copy outlived what started it"
        );
    }

    // With no given turn's file to open, a copy keeps its turn to its end,
    // saying so once; with one it can't open, it gives its turn back for
    // one of the others, saying which it couldn't.
    #[test]
    fn a_copy_keeps_its_turn_where_no_given_turn_opens() {
        let folder = tempfile::tempdir().expect("a folder");
        let elsewhere = folder.path().join("elsewhere");
        for turn in 1..=UPLOADS {
            let at = folder.path().join(format!("uplink-given-{turn}"));
            std::os::unix::fs::symlink(&elsewhere, at).expect("a link");
        }
        let (turn, other) = a_turn(folder.path());
        let began = std::time::Instant::now();
        let copy = Command::new("sleep").arg("2").spawn().expect("a copy");
        let waiting = waited_on(copy, turn, lease(1));
        let held = held_for(&other, began);
        assert!(
            held >= Duration::from_millis(1900) && held < Duration::from_secs(3),
            "a copy of 2 s with no given turn to open held its turn for {held:?}"
        );
        let (ended, said) = waiting.join().expect("the waiter");
        assert!(
            ended.expect("how it ended").success(),
            "a copy that succeeded read as one that failed"
        );
        assert!(
            said.len() == 1 && said[0].contains("can't be opened"),
            "keeping a turn to the end wasn't said once: {said:?}"
        );

        fs::remove_file(folder.path().join("uplink-given-1")).expect("a link gone");
        fs::remove_file(folder.path().join("uplink-given-3")).expect("a link gone");
        let (turn, other) = a_turn(folder.path());
        let began = std::time::Instant::now();
        let copy = Command::new("sleep").arg("3").spawn().expect("a copy");
        let waiting = waited_on(copy, turn, lease(1));
        let held = held_for(&other, began);
        let (_, said) = waiting.join().expect("the waiter");
        assert!(
            held >= Duration::from_millis(900)
                && held < Duration::from_millis(2500)
                && said.len() == 2
                && said[0].contains("uplink-given-2")
                && said[1].contains("the turn is free for another copy"),
            "a copy with one given turn it can't open held its turn {held:?}: {said:?}"
        );
        assert!(!elsewhere.exists(), "a given turn's link was followed");
    }

    #[test]
    fn a_knob_takes_whole_seconds_alone() {
        assert_eq!(
            seconds_in(" 60"),
            Some(60),
            "spaces round seconds were refused"
        );
        for set in ["90s", "1.5", "-1", ""] {
            assert_eq!(seconds_in(set), None, "{set:?} read as seconds");
        }
    }

    #[test]
    fn a_copy_within_its_share_holds_its_turn_to_the_end() {
        let folder = tempfile::tempdir().expect("a folder");
        for within in [Some(lease(60)), None] {
            let (turn, other) = a_turn(folder.path());
            let began = std::time::Instant::now();
            let mut copy = Command::new("sleep").arg("2").spawn().expect("a copy");
            let waiting = std::thread::spawn(move || {
                waited_giving_back(&mut copy, &mut Some(turn), within, "to b", &mut |_| {})
            });
            let held = held_for(&other, began);
            // Past its end by what spawning and two polls take under a
            // busy parallel test run, and far short of the 60 s lease.
            assert!(
                held >= Duration::from_millis(1900) && held < Duration::from_secs(3),
                "a copy of 2 s leased {within:?} held its turn for {held:?}"
            );
            let ended = waiting.join().expect("the waiter").expect("how it ended");
            assert!(
                ended.success(),
                "a copy that succeeded read as one that failed"
            );
        }
    }

    #[test]
    fn a_copy_with_no_turn_ends_as_it_ended() {
        let mut failed = Command::new("false").spawn().expect("a copy");
        let ended =
            waited_giving_back(&mut failed, &mut None, Some(lease(60)), "to b", &mut |_| {})
                .expect("how it ended");
        assert!(
            !ended.success(),
            "a copy that failed read as one that succeeded"
        );
    }

    #[test]
    fn a_waiter_takes_whichever_turn_frees_first() {
        let folder = tempfile::tempdir().expect("a folder");
        let mut held = all_held(folder.path(), "uplink");
        let at = folder.path().to_owned();
        let waiting = std::thread::spawn(move || {
            turn_in(&at, "to b", TURN_SAID_EVERY, &mut |_| {}).is_some()
        });
        // Long enough for the waiter to find every turn taken and wait.
        std::thread::sleep(Duration::from_millis(300));
        // Not the one the old code waited on, pid % UPLOADS + 1.
        let other = usize::try_from((std::process::id() + 1) % UPLOADS).expect("a turn");
        drop(held.remove(other));
        let began = std::time::Instant::now();
        while !waiting.is_finished() && began.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(
            waiting.is_finished(),
            "a waiter didn't take turn {} once it was free",
            other + 1
        );
        assert!(waiting.join().expect("the waiter"), "it took no turn");
    }

    // A copy come while another waits takes no turn, even a free one,
    // until the one before it has gone: the freed turn is the earlier's.
    #[test]
    fn a_waiter_takes_no_turn_while_one_that_came_first_waits() {
        let folder = tempfile::tempdir().expect("a folder");
        let first = Ticket::take(folder.path()).expect("a place in line");
        let at = folder.path().to_owned();
        let waiting = std::thread::spawn(move || {
            let mut said = Vec::new();
            let had = turn_in(&at, "to b", TURN_SAID_EVERY, &mut |line| said.push(line));
            (had.is_some(), said)
        });
        // Three looks' worth, for the waiter to pass every free turn by.
        std::thread::sleep(Duration::from_millis(300));
        assert!(
            !waiting.is_finished(),
            "a copy took a free turn while one that came first still waited"
        );
        drop(first);
        let (had, said) = waiting.join().expect("the waiter");
        assert!(
            had && said.len() == 1
                && said[0].contains("1 copy waiting came first")
                && said[0].contains("this waits its turn to send"),
            "the copy didn't take its turn once the first had gone, saying why it \
             waited: {said:?}"
        );
        let left: Vec<_> = fs::read_dir(folder.path())
            .expect("the folder")
            .flatten()
            .filter(|entry| entry.file_name().to_string_lossy().starts_with(TICKET))
            .collect();
        assert!(left.is_empty(), "a ticket outlived its wait: {left:?}");
    }

    // Neither a waiter that died, its ticket left unlocked, nor one stopped,
    // its ticket untouched past the stale time, keeps a later copy waiting;
    // the dead one's ticket is taken away, the stopped one's left.
    #[test]
    fn a_dead_or_stopped_waiter_keeps_no_copy_waiting() {
        let folder = tempfile::tempdir().expect("a folder");
        let dead = folder.path().join(format!("{TICKET}{:020}-1", 1));
        fs::File::create(&dead).expect("a dead waiter's ticket");
        let stopped = folder.path().join(format!("{TICKET}{:020}-2", 2));
        let held = fs::File::create(&stopped).expect("a stopped waiter's ticket");
        held.try_lock().expect("the stopped waiter's lock");
        let old = std::time::SystemTime::now()
            .checked_sub(TICKET_STALE * 2)
            .expect("a time before the stale time");
        held.set_modified(old).expect("an old touch");
        let at = folder.path().to_owned();
        let waiting =
            std::thread::spawn(move || turn_in(&at, "to b", TURN_SAID_EVERY, &mut |_| {}));
        let began = std::time::Instant::now();
        while !waiting.is_finished() && began.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(
            waiting.is_finished(),
            "a dead or stopped waiter kept a later copy from a free turn"
        );
        assert!(
            waiting.join().expect("the waiter").is_some(),
            "it took no turn"
        );
        assert!(!dead.exists(), "a dead waiter's ticket was left");
        assert!(stopped.exists(), "a stopped waiter's ticket was taken away");
    }

    #[test]
    fn a_copy_holds_its_turn_for_its_bytes_at_a_share_of_the_uplink() {
        // A size that couldn't be read is held to the end.
        assert_eq!(turn_within_of(Some(0), None), None);
        assert_eq!(
            turn_within_of(Some(1), None).map(|(within, _)| within),
            Some(Duration::from_mins(15))
        );
        // The 1.1 GB of 10 November, at a third of 1 MB/s.
        assert_eq!(
            turn_within_of(Some(1_100_000_000), None).map(|(within, _)| within),
            // Two minutes to start, and 3,300 s for its bytes.
            Some(Duration::from_mins(2 + 55))
        );
        // One whose size wasn't asked is leased for the least.
        assert_eq!(
            turn_within_of(None, None).map(|(within, _)| within),
            Some(Duration::from_mins(15))
        );
        // The knob gives any copy its lease, a size it couldn't read too.
        assert_eq!(
            turn_within_of(Some(0), Some(7)).map(|(within, _)| within),
            Some(Duration::from_secs(7))
        );
    }

    #[test]
    fn a_while_reads_in_seconds_then_minutes() {
        assert_eq!(lasting(Duration::from_secs(1)), "1 second");
        assert_eq!(lasting(Duration::from_secs(90)), "90 seconds");
        assert_eq!(lasting(Duration::from_secs(1520)), "26 minutes");
    }

    const AARCH64: &str = concat!(
        r#"Derive([("out","/nix/store/a-boot.bin","","")],"#,
        r#"[("/nix/store/b-m1n1.drv",["out"])],["/nix/store/c-builder.sh"],"#,
        r#""aarch64-linux","/bin/sh",["-c","echo \"],\" > $out"],"#,
        r#"[("system","x86_64-linux")])"#,
    );

    #[test]
    fn a_derivation_says_what_it_builds_for() {
        assert_eq!(derivation_system(AARCH64).as_deref(), Some("aarch64-linux"));
        assert_eq!(derivation_system("not a derivation"), None);
    }

    // On 9 November an aarch64 build of a boot chain was sent to a lender
    // that builds x86_64 alone, and began copying its kernel up.
    #[test]
    fn a_derivation_no_lent_builder_builds_for_is_named() {
        let paths = [
            "/nix/store/x-native.drv".to_owned(),
            "/nix/store/a-boot.bin.drv^*".to_owned(),
        ];
        let read = |path: &str| {
            Some(if path.ends_with("a-boot.bin.drv") {
                AARCH64.to_owned()
            } else {
                AARCH64.replace("\"aarch64-linux\",\"/bin", "\"x86_64-linux\",\"/bin")
            })
        };
        assert_eq!(
            foreign(&paths, &["x86_64-linux"], read),
            Some((
                "/nix/store/a-boot.bin.drv".to_owned(),
                "aarch64-linux".to_owned()
            ))
        );
        assert_eq!(
            foreign(&paths, &["x86_64-linux", "aarch64-linux"], read),
            None
        );
        assert_eq!(foreign(&paths, &[], read), None, "systems not known");
    }

    // The store names the account `there` reaches the builder as, and its
    // key; root's line names hook, and neither has the other's options.
    #[test]
    fn what_the_lent_builder_builds_for_is_read_from_the_line_naming_its_host() {
        let folder = tempfile::tempdir().expect("a folder");
        fs::write(
            folder.path().join("store"),
            "ssh-ng://nix-offload?compress=true&ssh-key=/k&base64-ssh-public-host-key=K\n",
        )
        .expect("the store");
        fs::write(
            folder.path().join("machines"),
            "ssh-ng://home x86_64-linux - 4\n\
             ssh-ng://hook@nix-offload?compress=true aarch64-linux,x86_64-linux /k 8 1 - - K\n",
        )
        .expect("the machines");
        assert_eq!(
            lent_systems(folder.path()),
            ["aarch64-linux", "x86_64-linux"]
        );
    }

    #[test]
    fn only_what_the_builder_says_it_fetched_counts_as_fetched() {
        // One of three couldn't be fetched, so it alone goes up, and an
        // answer naming something not asked for counts for nothing.
        let fetching =
            ["/nix/store/a.drv", "/nix/store/b.drv", "/nix/store/c.drv"].map(str::to_owned);
        let said = "/nix/store/c.drv\n/nix/store/a.drv\n/nix/store/z.drv\n";
        assert_eq!(
            fetched_of(said, &fetching),
            ["/nix/store/a.drv", "/nix/store/c.drv"].map(str::to_owned)
        );
        assert_eq!(fetched_of("", &fetching), [] as [std::string::String; 0]);
    }

    #[test]
    fn a_run_ends_with_all_it_started_once_its_caller_is_gone() {
        // The caller stands in for an ssh session. It starts the run in a
        // process group of its own, as sshd does, and ends once told. The
        // run holds the caller's output open, so the output closes only
        // when the run and everything it started have ended.
        let folder = tempfile::tempdir().unwrap();
        let cleaned = folder.path().join("cleaned");
        let run = far(&format!(
            "trap 'touch {}' EXIT; echo started; sleep 120",
            cleaned.display()
        ));
        let mut caller = calling(&run, folder.path());
        let mut said = BufReader::new(caller.stdout.take().unwrap());
        let mut line = String::new();
        said.read_line(&mut line).unwrap();
        assert_eq!(line, "started\n");
        drop(caller.stdin.take());
        caller.wait().unwrap();
        let (ended, closed) = mpsc::channel();
        std::thread::spawn(move || {
            let mut rest = String::new();
            let _ = std::io::Read::read_to_string(&mut said, &mut rest);
            let _ = ended.send(());
        });
        assert!(
            closed.recv_timeout(Duration::from_secs(30)).is_ok(),
            "the run outlived its caller"
        );
        assert!(cleaned.exists(), "it cleaned up as it ended");
    }

    /// A caller standing in for an ssh session: it starts `run` in a
    /// process group of its own, as sshd does, and ends once its input
    /// closes. A run cut off waits a second for another to claim it.
    fn calling(run: &str, scratch: &Path) -> std::process::Child {
        calling_within(run, scratch, 1)
    }

    /// `calling`, with a run cut off going on `grace` seconds for the
    /// same run sent again.
    fn calling_within(run: &str, scratch: &Path, grace: u32) -> std::process::Child {
        Command::new("sh")
            .args(["-c", "sh -c \"$1\" & read -r line", "caller", run])
            .env("TMPDIR", scratch)
            .env("NIX_OFFLOAD_GRACE", grace.to_string())
            .env("NIX_OFFLOAD_OUTLIVE", "60")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .process_group(0)
            .spawn()
            .unwrap()
    }

    #[test]
    fn a_run_cut_off_goes_on_while_the_same_run_is_sent_again() {
        // A lender restarting cuts every ssh to its builder, and the run
        // sent again waits on what the first was building. Each run writes
        // its shell's pid once its work is done, so two lines say the
        // first went on past its caller. Its grace is 5 s: at 1 s, a
        // builder busy with a suite's other tests on 8 November started
        // the second caller too late, and the first was ended.
        let folder = tempfile::tempdir().unwrap();
        let done = folder.path().join("done");
        let run = far(&format!(
            "echo started; sleep 8; echo $$ >> {}",
            done.display()
        ));
        let mut first = calling_within(&run, folder.path(), 5);
        let mut said = BufReader::new(first.stdout.take().unwrap());
        let mut line = String::new();
        said.read_line(&mut line).unwrap();
        assert_eq!(line, "started\n");
        drop(first.stdin.take());
        first.wait().unwrap();
        let mut again = calling_within(&run, folder.path(), 5);
        let mut said = BufReader::new(again.stdout.take().unwrap());
        let mut line = String::new();
        said.read_line(&mut line).unwrap();
        let (ended, closed) = mpsc::channel();
        let watched = done.clone();
        std::thread::spawn(move || {
            while fs::read_to_string(&watched).map_or(0, |done| done.lines().count()) < 2 {
                std::thread::sleep(Duration::from_millis(100));
            }
            let _ = ended.send(());
        });
        let both = closed.recv_timeout(Duration::from_secs(20)).is_ok();
        drop(again.stdin.take());
        again.wait().unwrap();
        assert!(both, "the first run ended with its caller");
    }

    #[test]
    fn a_run_asked_to_outlive_its_caller_finishes_its_work_alone() {
        // A prebuild's caller can hit its own time limit, as one did at
        // 05:14 on 3 November, and the kernel it asked for was dropped
        // mid-compile. Its work lasts past the grace a run cut off has,
        // and ends well before the outliving one's.
        let folder = tempfile::tempdir().unwrap();
        let done = folder.path().join("done");
        let run = outliving(&format!("echo started; sleep 4; touch {}", done.display()));
        let mut caller = calling(&run, folder.path());
        let mut said = BufReader::new(caller.stdout.take().unwrap());
        let mut line = String::new();
        said.read_line(&mut line).unwrap();
        assert_eq!(line, "started\n");
        drop(caller.stdin.take());
        caller.wait().unwrap();
        let (ended, closed) = mpsc::channel();
        std::thread::spawn(move || {
            let mut rest = String::new();
            let _ = std::io::Read::read_to_string(&mut said, &mut rest);
            let _ = ended.send(());
        });
        // Once its work ends, nothing of it stays to wait out the 60 s.
        let gone = closed.recv_timeout(Duration::from_secs(20)).is_ok();
        assert!(done.exists(), "its work ended with its caller");
        assert!(gone, "it stayed on after its work ended");
    }

    #[test]
    fn a_run_outliving_its_caller_says_so_there_until_it_ends() {
        // With its caller gone no ssh here reaches the builder, so the
        // lender asks there before giving it back idle, mid-build.
        let folder = tempfile::tempdir().unwrap();
        let asked = || {
            let printed = Command::new("sh")
                .args(["-c", OUTLIVING_ASKED])
                .env("TMPDIR", folder.path())
                .output()
                .unwrap()
                .stdout;
            String::from_utf8(printed).unwrap()
        };
        // One killed outright leaves its file, naming a shell long gone.
        fs::write(folder.path().join("nix-offload-outliving-999999999"), "").unwrap();
        assert_eq!(asked(), "");
        let mut caller = calling(&outliving("echo started; sleep 3"), folder.path());
        let mut said = BufReader::new(caller.stdout.take().unwrap());
        let mut line = String::new();
        said.read_line(&mut line).unwrap();
        drop(caller.stdin.take());
        caller.wait().unwrap();
        assert_eq!(asked(), "outliving\n");
        let mut rest = String::new();
        let _ = std::io::Read::read_to_string(&mut said, &mut rest);
        // Its watcher sees it gone within a second, and takes its file.
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !asked().is_empty() && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(100));
        }
        assert_eq!(asked(), "", "it still says it runs once it's ended");
    }

    #[test]
    fn a_run_whose_caller_stays_ends_with_its_own_status() {
        // In a group of its own, as sshd starts it, so its watcher's end
        // reaches nothing else once this test's process ends.
        let ran = Command::new("sh")
            .args(["-c", &far("echo ran; false")])
            .process_group(0)
            .output()
            .unwrap();
        assert_eq!(ran.status.code(), Some(1), "{ran:?}");
        assert_eq!(String::from_utf8_lossy(&ran.stdout), "ran\n");
    }

    #[test]
    fn a_run_that_ends_takes_its_claim_away_in_dash_and_zsh() {
        // On 18 November a Mac held 130 claim folders, one with 516
        // claims of runs long ended, three of them pids reused since by
        // daemons, which a watcher reads as callers still there: a run
        // killed watcher and all as its command ended never took its own.
        // A command that returns, exits itself or fails under `set -e`
        // ends each way, and keeps its status.
        let endings = [
            ("true", 0),
            ("exit 3", 3),
            ("set -e; false; echo unreached", 1),
        ];
        for shell in ["dash", "zsh"] {
            let folder = tempfile::tempdir().unwrap();
            for (command, status) in endings {
                for sent in [far(command), outliving(command)] {
                    let ran = Command::new(shell)
                        .args(["-c", &sent])
                        .env("TMPDIR", folder.path())
                        .process_group(0)
                        .output()
                        .unwrap();
                    assert_eq!(
                        ran.status.code(),
                        Some(status),
                        "{shell} {command}: {ran:?}"
                    );
                    assert_eq!(
                        String::from_utf8_lossy(&ran.stdout),
                        "",
                        "{shell} {command}"
                    );
                }
            }
            let left: Vec<_> = fs::read_dir(folder.path())
                .unwrap()
                .map(|entry| entry.unwrap().file_name())
                .collect();
            assert!(left.is_empty(), "{shell} left {left:?}");
        }
    }

    #[test]
    fn a_run_hung_up_takes_its_claim_away() {
        // sshd hangs up the whole group when the connection drops, the
        // command with it.
        for shell in ["dash", "zsh"] {
            let folder = tempfile::tempdir().unwrap();
            let mut ran = Command::new(shell)
                .args(["-c", &outliving("echo started; sleep 30")])
                .env("TMPDIR", folder.path())
                .process_group(0)
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            let mut said = BufReader::new(ran.stdout.take().unwrap());
            let mut line = String::new();
            said.read_line(&mut line).unwrap();
            let group = i32::try_from(ran.id()).unwrap();
            Command::new("kill")
                .args(["-HUP", "--", &format!("-{group}")])
                .status()
                .unwrap();
            let status = ran.wait().unwrap();
            let left: Vec<_> = fs::read_dir(folder.path())
                .unwrap()
                .map(|entry| entry.unwrap().file_name())
                .collect();
            assert_eq!(status.code(), Some(143), "{shell}: {status:?}");
            assert!(left.is_empty(), "{shell} left {left:?}");
        }
    }

    #[test]
    fn a_run_that_ends_leaves_another_callers_claim() {
        // The same command sent by another run still running there.
        let mut other = Command::new("sleep").arg("30").spawn().unwrap();
        let folder = tempfile::tempdir().unwrap();
        let digest = crate::running::stable_digest(&[b"true"]);
        let claims = folder.path().join(format!("nix-offload-run-{digest:016x}"));
        fs::create_dir(&claims).unwrap();
        fs::write(claims.join(other.id().to_string()), "").unwrap();
        let ran = Command::new("dash")
            .args(["-c", &far("true")])
            .env("TMPDIR", folder.path())
            .process_group(0)
            .output()
            .unwrap();
        let left: Vec<_> = fs::read_dir(&claims)
            .map(|entries| entries.map(|entry| entry.unwrap().file_name()).collect())
            .unwrap_or_default();
        other.kill().unwrap();
        other.wait().unwrap();
        assert_eq!(ran.status.code(), Some(0), "{ran:?}");
        assert_eq!(left, [std::ffi::OsString::from(other.id().to_string())]);
    }

    #[test]
    fn a_run_sent_to_a_mac_runs_in_its_login_shell_zsh() {
        // ssh runs the command in the account's login shell, which is
        // zsh on a Mac, where `status` is read-only: on 15 November every
        // fetch asked of the work Mac failed on it, so nothing built there.
        // What it says goes to files, and only zsh is waited on, as sshd
        // waits: a pipe held by the shell's loops would keep it unreaped,
        // which those loops read as a shell still alive.
        let folder = tempfile::tempdir().unwrap();
        let (out, err) = (folder.path().join("out"), folder.path().join("err"));
        let ran = Command::new("zsh")
            .args(["-c", &far("echo ran")])
            .process_group(0)
            .stdout(fs::File::create(&out).unwrap())
            .stderr(fs::File::create(&err).unwrap())
            .status()
            .unwrap();
        let said = fs::read_to_string(&err).unwrap_or_default();
        assert_eq!(ran.code(), Some(0), "zsh said: {said}");
        assert_eq!(
            fs::read_to_string(&out).unwrap(),
            "ran\n",
            "zsh said: {said}"
        );
    }

    // On 23 November a lent builder's alias was taken over by a new
    // instance mid-trial, and no log could say which instance a run was
    // on. A build says its machine's own name as it starts and as it ends,
    // with how it ended, in sh and zsh alike, and leaves what it prints
    // alone; an ask says neither, since a lender makes one every minute.
    #[test]
    fn a_build_says_which_machine_it_runs_on_and_an_ask_says_nothing() {
        let named = Command::new("uname").arg("-n").output().unwrap();
        let machine = String::from_utf8_lossy(&named.stdout).trim().to_owned();
        for shell in ["dash", "zsh"] {
            for (sent, says) in [
                (far_saying_where("echo ran; exit 3"), true),
                (far_brief("echo ran; exit 3"), true),
                (far("echo ran; exit 3"), false),
            ] {
                let folder = tempfile::tempdir().unwrap();
                let (out, err) = (folder.path().join("out"), folder.path().join("err"));
                let ran = Command::new(shell)
                    .args(["-c", &sent])
                    .process_group(0)
                    .stdout(fs::File::create(&out).unwrap())
                    .stderr(fs::File::create(&err).unwrap())
                    .status()
                    .unwrap();
                let said = fs::read_to_string(&err).unwrap_or_default();
                assert_eq!(ran.code(), Some(3), "{shell} said: {said}");
                assert_eq!(
                    fs::read_to_string(&out).unwrap(),
                    "ran\n",
                    "{shell} changed what the run printed: {said}"
                );
                let started = said.contains(&format!("nix-offload: running on {machine}\n"));
                let ended = said.contains(&format!("nix-offload: ran on {machine}, ending 3\n"));
                assert_eq!(
                    (started, ended),
                    (says, says),
                    "{shell} said where it ran, or didn't, against what was asked: {said}"
                );
            }
        }
    }

    /// The boot this machine's shells name a turn's holder in.
    fn this_boot() -> String {
        let read = Command::new("sh").args(["-c", BOOT_READ]).output().unwrap();
        String::from_utf8_lossy(&read.stdout).trim_end().to_owned()
    }

    /// A pid under any system's highest that no process holds: a shell's
    /// that has ended, as one killed outright leaves in its turn.
    fn gone_pid() -> u32 {
        let mut gone = Command::new("true").spawn().unwrap();
        gone.wait().unwrap();
        gone.id()
    }

    /// The name this machine's shells say for it.
    fn this_machine() -> String {
        let read = Command::new("uname").arg("-n").output().unwrap();
        String::from_utf8_lossy(&read.stdout).trim_end().to_owned()
    }

    /// What a run says when it can't read its boot to name a turn by.
    fn unbooted() -> String {
        format!(
            "nix-offload: the boot of {} can't be read from its boot_id or \
             kern.bootsessionuuid, so this builds without a turn\n",
            this_machine()
        )
    }

    /// What a run says when no turn can be made in `turns`.
    fn unheld(turns: &Path) -> String {
        format!(
            "nix-offload: {} on {} can't hold its turns, so this builds without one\n",
            turns.display(),
            this_machine()
        )
    }

    /// A turn on a machine nearby, `n`, held by `holder`: a shell and its
    /// boot.
    fn held_turn(turns: &Path, n: u32, holder: &str) {
        std::os::unix::fs::symlink(holder, turns.join(n.to_string())).unwrap();
    }

    /// Who holds turn `n` in `turns`, if anyone does.
    fn turn_holder(turns: &Path, n: u32) -> Option<String> {
        fs::read_link(turns.join(n.to_string()))
            .ok()
            .map(|holder| holder.to_string_lossy().into_owned())
    }

    /// `sent` started in `shell` in a group of its own, its turns in
    /// `turns`.
    fn sent_with_turns(shell: &str, sent: &str, turns: &Path) -> std::process::Child {
        Command::new(shell)
            .args(["-c", sent])
            .env("NIX_OFFLOAD_TURNS", turns)
            // Where far keeps its callers' claims, kept out of the real one
            // when `turns` is a test folder's own child.
            .env("TMPDIR", turns.parent().unwrap())
            .process_group(0)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    }

    /// `in_turn_nearby(command)` started in `shell`, its turns in `turns`.
    fn taking_turn(shell: &str, command: &str, turns: &Path) -> std::process::Child {
        sent_with_turns(shell, &in_turn_nearby(command), turns)
    }

    /// How `child` ended, failing rather than hanging once 30 seconds
    /// pass.
    fn ended(mut child: std::process::Child) -> std::process::Output {
        let deadline = Instant::now() + Duration::from_secs(30);
        while child.try_wait().unwrap().is_none() {
            if Instant::now() > deadline {
                let _ = child.kill();
                panic!(
                    "still waiting after 30 seconds: {:?}",
                    child.wait_with_output()
                );
            }
            thread::sleep(Duration::from_millis(50));
        }
        child.wait_with_output().unwrap()
    }

    #[test]
    fn a_build_sent_nearby_past_its_turns_waits_there_for_one_in_dash_and_zsh() {
        // On 16 November five builds ran at once on a Mac on 12
        // processors, four sent by different runs, at load 75 to 94.
        for shell in ["dash", "zsh"] {
            waits_for_a_turn(shell);
        }
    }

    /// A run in `shell` past every turn held waits, saying why and who
    /// holds them every second, then takes the one given back.
    fn waits_for_a_turn(shell: &str) {
        let turns = tempfile::tempdir().unwrap();
        // This test's own process holds every turn, and lives.
        let boot = this_boot();
        let live = format!("{} {boot}", std::process::id());
        for n in 1..=NEARBY_AT_ONCE {
            held_turn(turns.path(), n, &live);
        }
        let mut waiting = Command::new(shell)
            .args([
                "-c",
                &in_turn_nearby(&format!("readlink \"$NIX_OFFLOAD_TURNS/{NEARBY_AT_ONCE}\"")),
            ])
            .env("NIX_OFFLOAD_TURNS", turns.path())
            .env("NIX_OFFLOAD_TURN_EVERY", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let said = BufReader::new(waiting.stderr.take().unwrap());
        let (tell, told) = mpsc::channel();
        let reading = thread::spawn(move || {
            for line in said.lines().map_while(Result::ok) {
                if tell.send(line).is_err() {
                    break;
                }
            }
        });
        let why = format!(
            "nix-offload: {} already runs as many builds sent to it as it takes at once, so \
             this waits there for its turn, held in {} by shells {pid}, {pid}",
            this_machine(),
            turns.path().display(),
            pid = std::process::id()
        );
        for said in ["at once", "again a second on"] {
            let heard = told
                .recv_timeout(Duration::from_secs(30))
                .unwrap_or_else(|_| panic!("{shell} didn't say why it waited {said}"));
            assert_eq!(heard, why, "what {shell} said {said}");
        }
        assert_eq!(
            waiting.try_wait().unwrap(),
            None,
            "{shell} ended past {NEARBY_AT_ONCE} turns held"
        );
        for n in 1..=NEARBY_AT_ONCE {
            assert_eq!(
                turn_holder(turns.path(), n),
                Some(live.clone()),
                "{shell}, turn {n}"
            );
        }
        // The last turn, so a run looking at fewer than every turn fails.
        fs::remove_file(turns.path().join(NEARBY_AT_ONCE.to_string())).unwrap();
        let taker = waiting.id();
        let ran = ended(waiting);
        assert_eq!(ran.status.code(), Some(0), "{shell}: {ran:?}");
        assert_eq!(
            String::from_utf8_lossy(&ran.stdout),
            format!("{taker} {boot}\n"),
            "{shell} didn't hold the freed turn while its command ran"
        );
        reading.join().unwrap();
        assert_eq!(
            turn_holder(turns.path(), NEARBY_AT_ONCE),
            None,
            "{shell} kept its turn once it ended"
        );
        assert_eq!(
            turn_holder(turns.path(), 1),
            Some(live),
            "{shell} gave back a turn it never held"
        );
    }

    #[test]
    fn a_turn_whose_shell_is_gone_or_from_an_earlier_boot_is_taken_in_dash_and_zsh() {
        // A shell killed outright gives nothing back, and a reboot ends
        // every shell while its links stay, where a pid can since name
        // another process. ssh runs a Mac's command in zsh.
        let boot = this_boot();
        let live = format!("{} {boot}", std::process::id());
        let gone = [
            (format!("{} {boot}", gone_pid()), "killed outright"),
            (
                format!("{} an earlier boot", std::process::id()),
                "of an earlier boot",
            ),
            // kill -0 0 asks of the whole group, so always answers.
            (format!("0 {boot}"), "naming no shell"),
            (format!("00 {boot}"), "naming no shell in two zeros"),
            // No system's pids reach 8 digits, and zsh reads 4294967295
            // as -1, which kill -0 answers for as live.
            (format!("4294967295 {boot}"), "naming a pid zsh reads as -1"),
            (
                format!("99999999999999999999 {boot}"),
                "naming a pid past any system's",
            ),
        ];
        for (holder, why) in gone {
            for shell in ["dash", "zsh"] {
                let turns = tempfile::tempdir().unwrap();
                for n in 1..NEARBY_AT_ONCE {
                    held_turn(turns.path(), n, &live);
                }
                held_turn(turns.path(), NEARBY_AT_ONCE, &holder);
                let taking = taking_turn(
                    shell,
                    &format!("echo ran; readlink \"$NIX_OFFLOAD_TURNS/{NEARBY_AT_ONCE}\"; exit 3"),
                    turns.path(),
                );
                let taker = taking.id();
                let ran = ended(taking);
                assert_eq!(ran.status.code(), Some(3), "{shell}, {why}: {ran:?}");
                assert_eq!(
                    String::from_utf8_lossy(&ran.stdout),
                    format!("ran\n{taker} {boot}\n"),
                    "{shell} didn't hold the turn {why} while its command ran"
                );
                assert_eq!(String::from_utf8_lossy(&ran.stderr), "", "{shell}, {why}");
                assert_eq!(
                    turn_holder(turns.path(), NEARBY_AT_ONCE),
                    None,
                    "{shell} kept the turn {why} once it ended"
                );
                assert_eq!(turn_holder(turns.path(), 1), Some(live.clone()), "{shell}");
            }
        }
    }

    #[test]
    fn a_run_gives_back_only_a_turn_it_still_holds() {
        // Two runs taking back a dead shell's turn at once can both think
        // it theirs; the one ending first leaves the other's in place.
        let other = format!("{} {}", std::process::id(), this_boot());
        for shell in ["dash", "zsh"] {
            let turns = tempfile::tempdir().unwrap();
            let ran = ended(taking_turn(
                shell,
                &format!("ln -sfn '{other}' \"$NIX_OFFLOAD_TURNS/1\"; exit 3"),
                turns.path(),
            ));
            assert_eq!(ran.status.code(), Some(3), "{shell}: {ran:?}");
            assert_eq!(
                turn_holder(turns.path(), 1),
                Some(other.clone()),
                "{shell} gave back another run's turn"
            );
        }
    }

    #[test]
    fn a_run_waiting_as_its_turns_folder_is_removed_makes_it_again_and_takes_a_turn() {
        // Someone clearing ~/.cache would otherwise send every waiting run
        // to build with no turn at all.
        let boot = this_boot();
        let live = format!("{} {boot}", std::process::id());
        for shell in ["dash", "zsh"] {
            let folder = tempfile::tempdir().unwrap();
            let turns = folder.path().join("turns");
            fs::create_dir(&turns).unwrap();
            for n in 1..=NEARBY_AT_ONCE {
                held_turn(&turns, n, &live);
            }
            let mut waiting = Command::new(shell)
                .args(["-c", &in_turn_nearby("readlink \"$NIX_OFFLOAD_TURNS/1\"")])
                .env("NIX_OFFLOAD_TURNS", &turns)
                .env("NIX_OFFLOAD_TURN_EVERY", "1")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let said = BufReader::new(waiting.stderr.take().unwrap());
            let (tell, told) = mpsc::channel();
            let reading = thread::spawn(move || {
                let mut lines = Vec::new();
                for line in said.lines().map_while(Result::ok) {
                    let _ = tell.send(());
                    lines.push(line);
                }
                lines
            });
            told.recv_timeout(Duration::from_secs(30))
                .unwrap_or_else(|_| panic!("{shell} never said it waited"));
            // Moved aside whole first, so the run can't make a turn in it
            // as it's removed.
            let aside = folder.path().join("aside");
            fs::rename(&turns, &aside).unwrap();
            fs::remove_dir_all(&aside).unwrap();
            let taker = waiting.id();
            let ran = ended(waiting);
            let said = reading.join().unwrap();
            assert_eq!(ran.status.code(), Some(0), "{shell}: {ran:?}");
            assert_eq!(
                String::from_utf8_lossy(&ran.stdout),
                format!("{taker} {boot}\n"),
                "{shell} didn't take a turn in the folder made again: {said:?}"
            );
            assert!(
                !said.iter().any(|line| line.contains("without one")),
                "{shell}: {said:?}"
            );
        }
    }

    #[test]
    fn a_mac_names_its_turns_by_its_boot_session() {
        // A Mac has no /proc, so its boot is read through sysctl.
        let bin = tempfile::tempdir().unwrap();
        crate::testing::executable(&bin.path().join("cat"), "#!/bin/sh\nexit 1\n");
        crate::testing::executable(
            &bin.path().join("sysctl"),
            "#!/bin/sh\n[ \"$*\" = '-n kern.bootsessionuuid' ] || exit 1\n\
             echo 0A1B2C3D-0000-4000-8000-00000000F00D\n",
        );
        for shell in ["dash", "zsh"] {
            let turns = tempfile::tempdir().unwrap();
            let taking = Command::new(shell)
                .args([
                    "-c",
                    &first_on_path(
                        bin.path(),
                        &in_turn_nearby("readlink \"$NIX_OFFLOAD_TURNS/1\""),
                    ),
                ])
                .env("NIX_OFFLOAD_TURNS", turns.path())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let taker = taking.id();
            let ran = ended(taking);
            assert_eq!(ran.status.code(), Some(0), "{shell}: {ran:?}");
            assert_eq!(
                String::from_utf8_lossy(&ran.stdout),
                format!("{taker} 0A1B2C3D-0000-4000-8000-00000000F00D\n"),
                "{shell}: {ran:?}"
            );
        }
    }

    #[test]
    fn a_run_gives_back_its_turn_even_once_its_command_clears_turn() {
        // A build's command runs in the same shell, so may set anything.
        for shell in ["dash", "zsh"] {
            let turns = tempfile::tempdir().unwrap();
            let ran = ended(taking_turn(shell, "turn=; exit 3", turns.path()));
            assert_eq!(ran.status.code(), Some(3), "{shell}: {ran:?}");
            assert_eq!(
                turn_holder(turns.path(), 1),
                None,
                "{shell} kept its turn once its command forgot it"
            );
        }
    }

    #[test]
    fn a_run_whose_new_turn_is_taken_from_it_in_its_first_second_waits_again() {
        // Another run, having found the turn's last holder gone, removes
        // the link this one just made and takes it.
        let boot = this_boot();
        let live = format!("{} {boot}", std::process::id());
        for shell in ["dash", "zsh"] {
            let turns = tempfile::tempdir().unwrap();
            held_turn(turns.path(), NEARBY_AT_ONCE, &live);
            let mut taking = Command::new(shell)
                .args([
                    "-c",
                    &in_turn_nearby(&format!("readlink \"$NIX_OFFLOAD_TURNS/{NEARBY_AT_ONCE}\"")),
                ])
                .env("NIX_OFFLOAD_TURNS", turns.path())
                .env("NIX_OFFLOAD_TURN_EVERY", "1")
                // Long enough to swap its turn on a loaded builder.
                .env("NIX_OFFLOAD_TURN_LOOK", "5")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let taker = format!("{} {boot}", taking.id());
            let deadline = Instant::now() + Duration::from_secs(30);
            while turn_holder(turns.path(), 1) != Some(taker.clone()) {
                assert!(Instant::now() < deadline, "{shell} never took turn 1");
                thread::sleep(Duration::from_millis(10));
            }
            // Swapped whole, so the run never reads the place empty, and
            // held by another live shell, so the wait line names both.
            let mut other = Command::new("sleep").arg("60").spawn().unwrap();
            let parent = other.id();
            held_turn(turns.path(), 0, &format!("{parent} {boot}"));
            fs::rename(turns.path().join("0"), turns.path().join("1")).unwrap();
            let said = BufReader::new(taking.stderr.take().unwrap());
            let (tell, told) = mpsc::channel();
            let reading = thread::spawn(move || {
                for line in said.lines().map_while(Result::ok) {
                    if tell.send(line).is_err() {
                        break;
                    }
                }
            });
            let heard = told
                .recv_timeout(Duration::from_secs(30))
                .unwrap_or_else(|_| panic!("{shell} didn't wait again"));
            let pid = std::process::id();
            assert!(
                heard.contains("waits there for its turn")
                    && heard.ends_with(&format!("by shells {parent}, {pid}")),
                "{shell}: {heard}"
            );
            assert_eq!(
                taking.try_wait().unwrap(),
                None,
                "{shell} built on a lost turn"
            );
            fs::remove_file(turns.path().join(NEARBY_AT_ONCE.to_string())).unwrap();
            let ran = ended(taking);
            reading.join().unwrap();
            assert_eq!(ran.status.code(), Some(0), "{shell}: {ran:?}");
            assert_eq!(
                String::from_utf8_lossy(&ran.stdout),
                format!("{taker}\n"),
                "{shell} didn't hold the turn freed later"
            );
            other.kill().unwrap();
            other.wait().unwrap();
            assert_eq!(
                turn_holder(turns.path(), 1),
                Some(format!("{parent} {boot}")),
                "{shell}"
            );
        }
    }

    #[test]
    fn a_run_whose_look_sleep_refuses_still_looks_a_second_later() {
        // A look at once after taking a turn would miss its being taken.
        let bin = tempfile::tempdir().unwrap();
        crate::testing::executable(
            &bin.path().join("sleep"),
            &format!(
                "#!/bin/sh\necho \"$1\" >> \"$NIX_OFFLOAD_TURNS/../slept\"\n\
                 case $1 in never) echo \"sleep: invalid time interval\" >&2; exit 1 ;; \
                 esac\nexec {sleep} \"$@\"\n",
                sleep = which("sleep")
            ),
        );
        for shell in ["dash", "zsh"] {
            let folder = tempfile::tempdir().unwrap();
            let turns = folder.path().join("turns");
            let ran = ended(
                Command::new(shell)
                    .args(["-c", &first_on_path(bin.path(), &in_turn_nearby("true"))])
                    .env("NIX_OFFLOAD_TURNS", &turns)
                    .env("NIX_OFFLOAD_TURN_LOOK", "never")
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .unwrap(),
            );
            assert_eq!(ran.status.code(), Some(0), "{shell}: {ran:?}");
            assert_eq!(String::from_utf8_lossy(&ran.stderr), "", "{shell}");
            assert_eq!(
                fs::read_to_string(folder.path().join("slept")).unwrap(),
                "never\n1\n",
                "{shell} didn't look a second after a refused look"
            );
        }
    }

    #[test]
    fn a_run_whose_turn_a_run_in_another_boot_takes_builds_without_one_and_says_so() {
        // Each reads the other's turn as an earlier boot's and takes it.
        let bin = booted_as_told();
        let started = |shell: &str, turns: &Path, boot: &str| {
            Command::new(shell)
                .args([
                    "-c",
                    &first_on_path(bin.path(), &in_turn_nearby("echo built")),
                ])
                .env("NIX_OFFLOAD_TURNS", turns)
                .env("NIX_OFFLOAD_TURN_LOOK", "5")
                .env("TURN_BOOT", boot)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap()
        };
        for shell in ["dash", "zsh"] {
            let folder = tempfile::tempdir().unwrap();
            let turns = folder.path().join("turns");
            let first = started(shell, &turns, "one");
            // Well inside the first's look, whichever takes turn 1 first.
            thread::sleep(Duration::from_secs(2));
            let second = started(shell, &turns, "two");
            let said: Vec<String> = [ended(first), ended(second)]
                .into_iter()
                .map(|run| {
                    assert_eq!(run.status.code(), Some(0), "{shell}: {run:?}");
                    assert_eq!(
                        String::from_utf8_lossy(&run.stdout),
                        "built\n",
                        "{shell}: a run never built: {run:?}"
                    );
                    String::from_utf8_lossy(&run.stderr).into_owned()
                })
                .collect();
            let shared = format!(
                "nix-offload: {} on {} is shared with another boot, which took this run's \
                 turn, so the turns can't bound its builds and this builds without one; \
                 NIX_OFFLOAD_TURNS naming a folder of each machine's own bounds them\n",
                turns.display(),
                this_machine()
            );
            assert!(
                said == [shared.clone(), String::new()] || said == [String::new(), shared],
                "{shell}: one run, and only one, loses its turn to the other boot: {said:?}"
            );
        }
    }

    #[test]
    fn runs_in_two_boots_sharing_the_turns_all_end_however_they_interleave() {
        // Four runs in two boots, each holding its turn a while, took turns
        // from each other for ever before a turn another boot took ended it.
        let bin = booted_as_told();
        for shell in ["dash", "zsh"] {
            let folder = tempfile::tempdir().unwrap();
            let turns = folder.path().join("turns");
            let runs: Vec<_> = ["one", "two", "one", "two"]
                .into_iter()
                .map(|boot| {
                    let run = Command::new(shell)
                        .args([
                            "-c",
                            &first_on_path(bin.path(), &in_turn_nearby("sleep 2; echo built")),
                        ])
                        .env("NIX_OFFLOAD_TURNS", &turns)
                        .env("NIX_OFFLOAD_TURN_LOOK", "2")
                        .env("TURN_BOOT", boot)
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .spawn()
                        .unwrap();
                    thread::sleep(Duration::from_millis(700));
                    run
                })
                .collect();
            for run in runs {
                let run = ended(run);
                assert_eq!(run.status.code(), Some(0), "{shell}: {run:?}");
                assert_eq!(
                    String::from_utf8_lossy(&run.stdout),
                    "built\n",
                    "{shell}: a run never built: {run:?}"
                );
            }
        }
    }

    #[test]
    fn a_run_whose_turn_is_freed_during_its_look_takes_one_again_in_its_own_boot() {
        // An empty link read is no other boot's: the turn was given back.
        let bin = tempfile::tempdir().unwrap();
        crate::testing::executable(
            &bin.path().join("sleep"),
            &format!(
                "#!/bin/sh\ncase $1 in 4) [ -e \"$FREED\" ] || {{ : > \"$FREED\"; \
                 rm -f \"$NIX_OFFLOAD_TURNS/1\"; }} ;; esac\nexec {sleep} \"$@\"\n",
                sleep = which("sleep")
            ),
        );
        for shell in ["dash", "zsh"] {
            let folder = tempfile::tempdir().unwrap();
            let turns = folder.path().join("turns");
            let freed = folder.path().join("freed");
            let run = Command::new(shell)
                .args([
                    "-c",
                    &first_on_path(bin.path(), &in_turn_nearby("echo built")),
                ])
                .env("NIX_OFFLOAD_TURNS", &turns)
                .env("NIX_OFFLOAD_TURN_LOOK", "4")
                .env("FREED", &freed)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let run = ended(run);
            assert!(
                freed.exists(),
                "{shell}: the turn was never freed during a look"
            );
            assert_eq!(run.status.code(), Some(0), "{shell}: {run:?}");
            assert_eq!(
                String::from_utf8_lossy(&run.stdout),
                "built\n",
                "{shell}: {run:?}"
            );
            assert_eq!(
                String::from_utf8_lossy(&run.stderr),
                "",
                "{shell}: a freed turn read as another boot's: {run:?}"
            );
        }
    }

    /// A folder holding a `cat` that gives `$TURN_BOOT` as the boot id, so
    /// runs on one machine can stand for runs in different boots.
    fn booted_as_told() -> tempfile::TempDir {
        let bin = tempfile::tempdir().unwrap();
        crate::testing::executable(
            &bin.path().join("cat"),
            &format!(
                "#!/bin/sh\ncase $1 in /proc/sys/kernel/random/boot_id) \
                 echo \"$TURN_BOOT\"; exit ;; esac\nexec {cat} \"$@\"\n",
                cat = which("cat")
            ),
        );
        bin
    }

    #[test]
    fn a_shell_whose_pid_a_dead_turn_names_takes_it_rather_than_wait_on_itself() {
        // A shell killed outright earlier this boot, whose pid this one
        // now has: kill -0 finds it alive, since it's this shell.
        let boot = this_boot();
        for shell in ["dash", "zsh"] {
            let turns = tempfile::tempdir().unwrap();
            for n in 1..NEARBY_AT_ONCE {
                held_turn(turns.path(), n, &format!("{} {boot}", std::process::id()));
            }
            // A shell's pid is known only once it runs, so it names itself.
            let sent = format!(
                "ln -s \"$$ {boot}\" \"$NIX_OFFLOAD_TURNS/{NEARBY_AT_ONCE}\" && {}",
                in_turn_nearby("echo ran")
            );
            let ran = ended(sent_with_turns(shell, &sent, turns.path()));
            assert_eq!(ran.status.code(), Some(0), "{shell}: {ran:?}");
            assert_eq!(String::from_utf8_lossy(&ran.stdout), "ran\n", "{shell}");
            assert_eq!(
                String::from_utf8_lossy(&ran.stderr),
                "",
                "{shell} waited on itself"
            );
        }
    }

    #[test]
    fn a_stray_folder_in_one_turns_place_leaves_the_others_to_take() {
        // A bound left whole by one turn that can't be made, rather than
        // switched off for every run until someone removes it.
        for shell in ["dash", "zsh"] {
            let turns = tempfile::tempdir().unwrap();
            fs::create_dir(turns.path().join("1")).unwrap();
            let ran = ended(taking_turn(
                shell,
                &format!("readlink \"$NIX_OFFLOAD_TURNS/{NEARBY_AT_ONCE}\""),
                turns.path(),
            ));
            assert_eq!(ran.status.code(), Some(0), "{shell}: {ran:?}");
            assert!(
                String::from_utf8_lossy(&ran.stdout).ends_with(&format!(" {}\n", this_boot())),
                "{shell} didn't hold the other turn: {ran:?}"
            );
            assert_eq!(String::from_utf8_lossy(&ran.stderr), "", "{shell}");
            assert!(turns.path().join("1").is_dir(), "{shell} took the folder");
        }
    }

    #[test]
    fn a_run_finding_no_turn_it_can_make_looks_again_before_building_without_one() {
        // A dead turn another run is taking back at that moment reads as
        // one that can't be made, so one look alone would send a run in a
        // burst to build with no turn. Here the stray folder in turn 1's
        // place is cleared as the run makes its turns' folder a second time.
        let bin = tempfile::tempdir().unwrap();
        crate::testing::executable(
            &bin.path().join("mkdir"),
            &format!(
                "#!/bin/sh\nif [ -e \"$NIX_OFFLOAD_TURNS/looked\" ]; then \
                 rmdir \"$NIX_OFFLOAD_TURNS/1\" 2>/dev/null; \
                 else : > \"$NIX_OFFLOAD_TURNS/looked\"; fi\nexec {} \"$@\"\n",
                which("mkdir")
            ),
        );
        for shell in ["dash", "zsh"] {
            let turns = tempfile::tempdir().unwrap();
            for n in 1..=NEARBY_AT_ONCE {
                fs::create_dir(turns.path().join(n.to_string())).unwrap();
            }
            let taking = Command::new(shell)
                .args([
                    "-c",
                    &first_on_path(
                        bin.path(),
                        &in_turn_nearby("readlink \"$NIX_OFFLOAD_TURNS/1\""),
                    ),
                ])
                .env("NIX_OFFLOAD_TURNS", turns.path())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let taker = taking.id();
            let ran = ended(taking);
            assert_eq!(ran.status.code(), Some(0), "{shell}: {ran:?}");
            assert_eq!(
                String::from_utf8_lossy(&ran.stdout),
                format!("{taker} {}\n", this_boot()),
                "{shell} built without the turn freed on its second look: {ran:?}"
            );
            assert_eq!(String::from_utf8_lossy(&ran.stderr), "", "{shell}");
        }
    }

    #[test]
    fn a_run_that_waits_between_two_looks_finding_no_turn_looks_twice_again() {
        // Only two looks in a row finding no turn it can make send a run to
        // build with none, so one such look, a wait on live turns, then
        // another, is still waited out. Its looks find stray folders, live
        // turns, stray folders again, then none.
        let bin = tempfile::tempdir().unwrap();
        crate::testing::executable(
            &bin.path().join("mkdir"),
            &format!(
                "#!/bin/sh\nt=\"$NIX_OFFLOAD_TURNS\"; looks=$(cat \"$t/looks\" 2>/dev/null); \
                 looks=$((${{looks:-0}} + 1)); echo \"$looks\" > \"$t/looks\"\n\
                 case $looks in\n\
                 2) rmdir \"$t/1\" \"$t/2\"; \
                 ln -s \"$LIVE\" \"$t/1\"; ln -s \"$LIVE\" \"$t/2\" ;;\n\
                 3) rm \"$t/1\" \"$t/2\"; {mkdir} \"$t/1\" \"$t/2\" ;;\n\
                 4) rmdir \"$t/1\" \"$t/2\" ;;\n\
                 esac\nexec {mkdir} \"$@\"\n",
                mkdir = which("mkdir")
            ),
        );
        assert_eq!(NEARBY_AT_ONCE, 2, "the stand-in mkdir makes two turns");
        let live = format!("{} {}", std::process::id(), this_boot());
        for shell in ["dash", "zsh"] {
            let turns = tempfile::tempdir().unwrap();
            for n in 1..=NEARBY_AT_ONCE {
                fs::create_dir(turns.path().join(n.to_string())).unwrap();
            }
            let taking = Command::new(shell)
                .args([
                    "-c",
                    &first_on_path(
                        bin.path(),
                        &in_turn_nearby("readlink \"$NIX_OFFLOAD_TURNS/1\""),
                    ),
                ])
                .env("NIX_OFFLOAD_TURNS", turns.path())
                .env("LIVE", &live)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let taker = taking.id();
            let ran = ended(taking);
            assert_eq!(ran.status.code(), Some(0), "{shell}: {ran:?}");
            assert_eq!(
                String::from_utf8_lossy(&ran.stdout),
                format!("{taker} {}\n", this_boot()),
                "{shell} built without the turn freed on its fourth look: {ran:?}"
            );
            let said = String::from_utf8_lossy(&ran.stderr);
            assert!(
                said.contains("waits there for its turn") && !said.contains("without"),
                "{shell} didn't wait on its second look, or built without a turn: {said}"
            );
        }
    }

    #[test]
    fn a_run_waiting_for_a_turn_says_so_again_only_every_turn_every_seconds() {
        let turns = tempfile::tempdir().unwrap();
        let live = format!("{} {}", std::process::id(), this_boot());
        for n in 1..=NEARBY_AT_ONCE {
            held_turn(turns.path(), n, &live);
        }
        for shell in ["dash", "zsh"] {
            let mut waiting = Command::new(shell)
                .args(["-c", &in_turn_nearby("true")])
                .env("NIX_OFFLOAD_TURNS", turns.path())
                .env("NIX_OFFLOAD_TURN_EVERY", "6")
                .process_group(0)
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let said = BufReader::new(waiting.stderr.take().unwrap());
            let (tell, told) = mpsc::channel();
            let reading = thread::spawn(move || {
                for line in said.lines().map_while(Result::ok) {
                    if tell.send(line).is_err() {
                        break;
                    }
                }
            });
            told.recv_timeout(Duration::from_secs(30))
                .unwrap_or_else(|_| panic!("{shell} never said it waited"));
            let early = told.recv_timeout(Duration::from_secs(3));
            let later = told.recv_timeout(Duration::from_secs(30));
            let killed = Command::new("kill")
                .args(["-KILL", "--", &format!("-{}", waiting.id())])
                .status()
                .unwrap();
            assert!(killed.success(), "{shell}'s run couldn't be ended");
            waiting.wait().unwrap();
            reading.join().unwrap();
            assert!(
                early.is_err(),
                "{shell} said it waited again within 3 s: {early:?}"
            );
            assert!(
                later.is_ok_and(|said| said.contains("waits there for its turn")),
                "{shell} never said it waited again"
            );
        }
    }

    #[test]
    fn a_burst_of_runs_sent_nearby_never_builds_more_than_its_turns_at_once() {
        // On 16 November five builds sent by different runs ran at once on
        // one Mac. A turn left by a shell killed outright is in the way.
        for shell in ["dash", "zsh"] {
            let scratch = tempfile::tempdir().unwrap();
            let (turns, running) = (scratch.path().join("turns"), scratch.path().join("running"));
            let seen = scratch.path().join("seen");
            fs::create_dir(&turns).unwrap();
            fs::create_dir(&running).unwrap();
            held_turn(&turns, 1, &format!("{} {}", gone_pid(), this_boot()));
            let build = format!(
                "mkdir '{r}/'$$; ls '{r}' | wc -l >> '{s}'; sleep 1; rmdir '{r}/'$$",
                r = running.display(),
                s = seen.display()
            );
            // Its look after taking a turn is widened, as a stall past it
            // on a loaded builder can let an extra build in.
            let sent = format!(
                "NIX_OFFLOAD_TURN_LOOK=3; export NIX_OFFLOAD_TURN_LOOK; {}",
                in_turn_nearby(&build)
            );
            let runs: Vec<_> = (0..6)
                .map(|_| sent_with_turns(shell, &sent, &turns))
                .collect();
            for run in runs {
                let ran = ended(run);
                assert_eq!(ran.status.code(), Some(0), "{shell}: {ran:?}");
                let said = String::from_utf8_lossy(&ran.stderr);
                assert!(!said.contains("without"), "{shell}: {said}");
            }
            let seen = fs::read_to_string(&seen).unwrap();
            let at_once: Vec<u32> = seen.lines().map(|n| n.trim().parse().unwrap()).collect();
            assert_eq!(at_once.len(), 6, "{shell}: {seen}");
            assert!(
                at_once.iter().all(|n| *n <= NEARBY_AT_ONCE),
                "{shell} built {at_once:?} at once"
            );
            for n in 1..=NEARBY_AT_ONCE {
                assert_eq!(turn_holder(&turns, n), None, "{shell} left turn {n} held");
            }
        }
    }

    #[test]
    fn a_run_waiting_beside_a_turn_it_cant_make_counts_it_apart_from_the_shells() {
        let pid = std::process::id();
        for shell in ["dash", "zsh"] {
            let turns = tempfile::tempdir().unwrap();
            fs::create_dir(turns.path().join("1")).unwrap();
            held_turn(turns.path(), 2, &format!("{pid} {}", this_boot()));
            let mut waiting = Command::new(shell)
                .args(["-c", &in_turn_nearby("true")])
                .env("NIX_OFFLOAD_TURNS", turns.path())
                .process_group(0)
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let said = BufReader::new(waiting.stderr.take().unwrap());
            let (tell, told) = mpsc::channel();
            let reading = thread::spawn(move || {
                for line in said.lines().map_while(Result::ok) {
                    if tell.send(line).is_err() {
                        break;
                    }
                }
            });
            let heard = told.recv_timeout(Duration::from_secs(30));
            let killed = Command::new("kill")
                .args(["-KILL", "--", &format!("-{}", waiting.id())])
                .status()
                .unwrap();
            assert!(killed.success(), "{shell}'s run couldn't be ended");
            waiting.wait().unwrap();
            reading.join().unwrap();
            assert!(
                heard
                    .as_ref()
                    .is_ok_and(|said| said
                        .ends_with(&format!("by shells {pid}, with 1 more it can't make"))),
                "{shell}: {heard:?}"
            );
        }
    }

    #[test]
    fn a_run_that_loses_a_turn_between_two_looks_finding_none_looks_twice_again() {
        // A look that took a turn, even one taken from it after, breaks a
        // run of looks finding no turn it can make. Its looks find stray
        // folders, a free turn taken from it within its look, stray folders
        // again, then none.
        let bin = tempfile::tempdir().unwrap();
        crate::testing::executable(
            &bin.path().join("mkdir"),
            &format!(
                "#!/bin/sh\nt=\"$NIX_OFFLOAD_TURNS\"; looks=$(cat \"$t/looks\" 2>/dev/null); \
                 looks=$((${{looks:-0}} + 1)); echo \"$looks\" > \"$t/looks\"\n\
                 case $looks in\n\
                 2) rmdir \"$t/1\" \"$t/2\"; ln -s \"$LIVE\" \"$t/2\"; \
                 (w=0; until [ -L \"$t/1\" ] || [ \"$w\" -ge 400 ]; do \
                 sleep 0.05; w=$((w + 1)); done; \
                 rm -f \"$t/1\"; ln -s \"$LIVE\" \"$t/1\") & ;;\n\
                 3) rm \"$t/1\" \"$t/2\"; {mkdir} \"$t/1\" \"$t/2\" ;;\n\
                 4) rmdir \"$t/1\" \"$t/2\" ;;\n\
                 esac\nexec {mkdir} \"$@\"\n",
                mkdir = which("mkdir")
            ),
        );
        assert_eq!(NEARBY_AT_ONCE, 2, "the stand-in mkdir makes two turns");
        let live = format!("{} {}", std::process::id(), this_boot());
        for shell in ["dash", "zsh"] {
            let turns = tempfile::tempdir().unwrap();
            for n in 1..=NEARBY_AT_ONCE {
                fs::create_dir(turns.path().join(n.to_string())).unwrap();
            }
            let taking = Command::new(shell)
                .args([
                    "-c",
                    &first_on_path(
                        bin.path(),
                        &in_turn_nearby("readlink \"$NIX_OFFLOAD_TURNS/1\""),
                    ),
                ])
                .env("NIX_OFFLOAD_TURNS", turns.path())
                .env("LIVE", &live)
                // Long enough for the stand-in to take the turn within it.
                .env("NIX_OFFLOAD_TURN_LOOK", "3")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let taker = taking.id();
            let ran = ended(taking);
            assert_eq!(ran.status.code(), Some(0), "{shell}: {ran:?}");
            assert_eq!(
                String::from_utf8_lossy(&ran.stdout),
                format!("{taker} {}\n", this_boot()),
                "{shell} built without the turn freed on its fourth look: {ran:?}"
            );
            // A lost turn is looked for again past the wait line, so a run
            // that waited instead never lost it.
            assert_eq!(
                String::from_utf8_lossy(&ran.stderr),
                "",
                "{shell} waited rather than lost its turn, or built without one"
            );
            assert_eq!(
                fs::read_to_string(turns.path().join("looks")).unwrap(),
                "4\n",
                "{shell}"
            );
        }
    }

    /// `script`, with `bin`'s programs first on this test's PATH. Set by
    /// the script, not the environment, since nix-darwin's /etc/zshenv sets
    /// PATH again in every zsh started without its own marker, as a package
    /// build's is, though not one from a nix-darwin terminal.
    fn first_on_path(bin: &Path, script: &str) -> String {
        format!(
            "PATH='{}:{}'; export PATH; {script}",
            bin.display(),
            std::env::var("PATH").unwrap()
        )
    }

    /// Where `program` is on this PATH.
    fn which(program: &str) -> String {
        std::env::var("PATH")
            .unwrap()
            .split(':')
            .map(|folder| Path::new(folder).join(program))
            .find(|path| path.is_file())
            .unwrap_or_else(|| panic!("no {program} on PATH"))
            .display()
            .to_string()
    }

    #[test]
    fn a_run_that_cant_read_its_boot_builds_without_a_turn_and_leaves_the_others() {
        // With no boot to name, every live turn would read as an earlier
        // boot's and be taken from the run holding it.
        let bin = tempfile::tempdir().unwrap();
        crate::testing::executable(&bin.path().join("cat"), "#!/bin/sh\nexit 1\n");
        crate::testing::executable(&bin.path().join("sysctl"), "#!/bin/sh\nexit 1\n");
        let live = format!("{} {}", std::process::id(), this_boot());
        for shell in ["dash", "zsh"] {
            let turns = tempfile::tempdir().unwrap();
            for n in 1..=NEARBY_AT_ONCE {
                held_turn(turns.path(), n, &live);
            }
            let ran = ended(
                Command::new(shell)
                    .args([
                        "-c",
                        &first_on_path(bin.path(), &in_turn_nearby("echo ran")),
                    ])
                    .env("NIX_OFFLOAD_TURNS", turns.path())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .unwrap(),
            );
            assert_eq!(ran.status.code(), Some(0), "{shell}: {ran:?}");
            assert_eq!(String::from_utf8_lossy(&ran.stdout), "ran\n", "{shell}");
            assert_eq!(String::from_utf8_lossy(&ran.stderr), unbooted(), "{shell}");
            for n in 1..=NEARBY_AT_ONCE {
                assert_eq!(
                    turn_holder(turns.path(), n),
                    Some(live.clone()),
                    "{shell} took live turn {n}"
                );
            }
        }
    }

    #[test]
    fn a_build_sent_where_no_turn_can_be_made_builds_without_one_and_says_so() {
        // A full disk, or a turns folder that can't be written, would
        // otherwise leave every run waiting for a turn none can have.
        use std::os::unix::fs::PermissionsExt;
        // How each way makes its turns' folder.
        type Made = fn(&Path);
        let ways: [(&str, Made); 3] = [
            ("a file where the folder should be", |turns| {
                fs::write(turns, "not a folder").unwrap();
            }),
            // ln -s would make its link inside each, and take nothing.
            ("a folder where every turn should be", |turns| {
                for n in 1..=NEARBY_AT_ONCE {
                    fs::create_dir_all(turns.join(n.to_string())).unwrap();
                }
            }),
            ("dead turns in a folder that can't be written", |turns| {
                fs::create_dir(turns).unwrap();
                for n in 1..=NEARBY_AT_ONCE {
                    held_turn(turns, n, &format!("{} {}", gone_pid(), this_boot()));
                }
                fs::set_permissions(turns, fs::Permissions::from_mode(0o555)).unwrap();
            }),
        ];
        for (way, made) in ways {
            for shell in ["dash", "zsh"] {
                let folder = tempfile::tempdir().unwrap();
                let turns = folder.path().join("turns");
                made(&turns);
                let ran = ended(taking_turn(shell, "echo ran", &turns));
                let _ = fs::set_permissions(&turns, fs::Permissions::from_mode(0o755));
                assert_eq!(ran.status.code(), Some(0), "{shell}, {way}: {ran:?}");
                assert_eq!(
                    String::from_utf8_lossy(&ran.stdout),
                    "ran\n",
                    "{shell}, {way}"
                );
                assert_eq!(
                    String::from_utf8_lossy(&ran.stderr),
                    unheld(&turns),
                    "{shell}, {way}"
                );
            }
        }
    }

    #[test]
    fn a_run_sent_far_ended_as_its_watcher_ends_it_gives_its_turn_back_in_dash_and_zsh() {
        // far's watcher ends a run with TERM to its whole group once its
        // callers are gone, as this does.
        for shell in ["dash", "zsh"] {
            let scratch = tempfile::tempdir().unwrap();
            let turns = scratch.path().join("turns");
            let running = sent_with_turns(shell, &far(&in_turn_nearby("sleep 30")), &turns);
            let deadline = Instant::now() + Duration::from_secs(30);
            while turn_holder(&turns, 1).is_none() {
                assert!(Instant::now() < deadline, "{shell} never took its turn");
                thread::sleep(Duration::from_millis(50));
            }
            Command::new("kill")
                .args(["-TERM", "--", &format!("-{}", running.id())])
                .status()
                .unwrap();
            let ran = ended(running);
            assert_eq!(ran.status.code(), Some(143), "{shell}: {ran:?}");
            assert_eq!(
                turn_holder(&turns, 1),
                None,
                "{shell} kept its turn once its watcher ended it"
            );
        }
    }

    #[test]
    fn a_build_sent_nearby_is_sent_to_wait_for_a_turn() {
        let folder = tempfile::tempdir().unwrap();
        let (ssh, sent) = (folder.path().join("ssh"), folder.path().join("sent"));
        crate::testing::executable(
            &ssh,
            &format!(
                "#!/bin/sh\nfor word; do last=$word; done\nprintf %s \"$last\" > '{}'\n",
                sent.display()
            ),
        );
        let asked = asked(&words("github:o/r checks.aarch64-darwin.b")).unwrap();
        let ran = nearby_run(
            &mac(),
            &asked,
            &folder.path().join("scratch"),
            [ssh.to_str().unwrap(), "tar", "nix"],
        );
        assert_eq!(ran.map(|ran| ran.status), Some(0));
        assert_eq!(
            fs::read_to_string(&sent).unwrap(),
            far_saying_where(&in_turn_nearby(&asked.command())),
            "a build sent nearby waits for a turn and says which machine it runs on"
        );
    }

    /// What a run sent far says while it runs for `seconds`, and how long
    /// it took, on a builder whose `ps` names `uids` as running and whose
    /// Nix runs `most` builds at once, asked about every second.
    #[cfg(target_os = "linux")]
    fn said_far(seconds: u32, uids: &str, most: &str) -> (String, Duration) {
        let folder = tempfile::tempdir().unwrap();
        let bin = folder.path().join("bin");
        fs::create_dir(&bin).unwrap();
        for (name, says) in [("ps", uids), ("nix", most)] {
            let fake = bin.join(name);
            crate::testing::executable(&fake, &format!("#!/bin/sh\nprintf '%s\\n' {says}\n"));
        }
        let began = std::time::Instant::now();
        let ran = Command::new("sh")
            .args(["-c", &far(&format!("sleep {seconds}"))])
            .env(
                "PATH",
                format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
            )
            .env("TMPDIR", folder.path())
            .env("NIX_OFFLOAD_ROOM_EVERY", "1")
            .process_group(0)
            .output()
            .unwrap();
        assert_eq!(ran.status.code(), Some(0), "{ran:?}");
        (
            String::from_utf8_lossy(&ran.stderr).into_owned(),
            began.elapsed(),
        )
    }

    // What's said there runs on a lent builder, which is Linux.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_run_sent_to_a_full_builder_says_it_may_wait_there_for_room() {
        // On 8 November the net's VM test said "building on the lent
        // builder nix-offload-vm-4" and then nothing for 30 minutes, which
        // the net read as a stall and held main. A builder running as
        // many builds as it runs at once, two allocated and one build
        // user's here, says so; one with room says nothing, so a hung
        // build stays silent.
        let full = "872415232 872415233 872480768 30001 0 1000";
        let (said, took) = said_far(3, full, "3");
        assert!(
            said.contains(
                "nix-offload: the builder runs 3 builds, as many as it runs at once, \
                 so this may wait there for room"
            ),
            "{said}"
        );
        assert!(took < Duration::from_secs(6), "it ended late: {took:?}");
        let (said, _) = said_far(3, full, "4");
        assert_eq!(said, "");
    }

    #[test]
    fn a_size_is_said_to_a_tenth_of_its_unit() {
        assert_eq!(sized(500), "500 bytes");
        assert_eq!(sized(11_964), "11.9 KB");
        assert_eq!(sized(3_913_640), "3.9 MB");
        assert_eq!(sized(793_000_000), "793.0 MB");
        assert_eq!(sized(2_500_000_000_000), "2500.0 GB");
    }

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn a_flake_and_its_attributes_are_asked_with_their_flags_anywhere() {
        let asked = asked(&words(
            "-L github:o/r/abc checks.x.a --keep-going checks.x.b",
        ));
        assert_eq!(
            asked,
            Some(Asked {
                flake: "github:o/r/abc".to_owned(),
                attributes: words("checks.x.a checks.x.b"),
                flags: words("-L --keep-going"),
                apply: None,
                out: None,
                print: false,
                here: Here::default(),
                alone: Alone::Ends,
                bulk: false,
                derivations: false,
                pinned: None,
            })
        );
        let outliving = super::asked(&words("--outlive github:o/r/abc checks.x.a"));
        assert!(outliving.is_some_and(|asked| asked.alone == Alone::GoesOn));
    }

    #[test]
    fn installables_of_one_flake_are_asked_as_its_attributes() {
        let asked_of =
            |line: &str| asked(&words(line)).map(|asked| (asked.flake, asked.attributes));
        let wanted = |flake: &str, attributes: &str| Some((flake.to_owned(), words(attributes)));
        assert_eq!(
            asked_of("-L .#checks.x.a .#checks.x.b"),
            wanted(".", "checks.x.a checks.x.b")
        );
        assert_eq!(asked_of("#checks.x.a"), wanted(".", "checks.x.a"));
        assert_eq!(
            asked_of("github:o/r#checks.x.a"),
            wanted("github:o/r", "checks.x.a")
        );
        assert_eq!(asked_of(". checks.x.a"), wanted(".", "checks.x.a"));
        assert_eq!(
            asked_of(".#checks.x.a checks.x.b"),
            None,
            "some installables, some not"
        );
        assert_eq!(asked_of(".#checks.x.a ../o#checks.x.b"), None, "two flakes");
        assert_eq!(asked_of(".#"), None, "no attribute");
    }

    #[test]
    fn this_machine_is_idle_only_while_its_cpu_pressure_has_stayed_low() {
        let pressure = |ten: &str, sixty: &str| {
            format!(
                "some avg10={ten} avg60={sixty} avg300=40.00 total=1\n\
                 full avg10=0.00 avg60=0.00 avg300=0.00 total=0\n"
            )
        };
        assert!(idle(&pressure("2.00", "9.99")));
        assert!(!idle(&pressure("2.00", "10.00")), "busy this last minute");
        assert!(!idle(&pressure("43.38", "4.50")), "busy now");
        assert!(!idle(""), "unread, it's taken as busy");
        assert!(!idle("some avg10=1.00\n"), "half a line is taken as busy");
    }

    #[test]
    fn a_build_begun_idle_gives_way_once_the_last_ten_seconds_are_busy() {
        let pressure = |ten: &str| {
            format!(
                "some avg10={ten} avg60=60.00 avg300=40.00 total=1\n\
                 full avg10=0.00 avg60=0.00 avg300=0.00 total=0\n"
            )
        };
        assert!(
            !busy(&pressure("24.99")),
            "a busy minute behind it gave a build its way"
        );
        assert!(
            busy(&pressure("25.00")),
            "a busy machine kept building here"
        );
        assert!(busy(""), "an unread pressure kept building here");
    }

    // A Mac has no pressure to read, so it never builds here while idle.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_build_here_ends_once_this_machine_grows_busy() {
        let folder = tempfile::tempdir().expect("a folder");
        let nix = folder.path().join("nix");
        let ended = folder.path().join("ended");
        crate::testing::executable(
            &nix,
            &format!(
                "#!/bin/sh\ntrap 'touch {}; exit 143' TERM\nsleep 30 & wait\n",
                ended.display()
            ),
        );
        let nix = nix.display().to_string();
        let asked = asked(&words("--here-when-idle github:o/r/abc checks.x.a")).expect("asked");
        let began = std::time::Instant::now();
        assert!(
            here_until(&asked, &nix, &|| true).is_none(),
            "a build here outlasted a busy machine"
        );
        assert!(
            began.elapsed() < std::time::Duration::from_secs(15),
            "a busy machine waited {:?} for its build here to end",
            began.elapsed()
        );
        assert!(ended.exists(), "the build here wasn't told to end");
        let ran = here_until(&asked, "true", &|| true).expect("a quick build's end");
        assert_eq!(
            ran.status, 0,
            "a build that ended before busy was read was lost"
        );
    }

    #[test]
    fn derivations_are_asked_for_by_their_paths_whole_unless_they_name_outputs() {
        // A build tool's own targets come evaluated, as derivations.
        let both = asked(&words("-L /nix/store/aaaa-x.drv /nix/store/bbbb-y.drv^out")).unwrap();
        assert_eq!(
            both.build(),
            words("build --no-link -L /nix/store/aaaa-x.drv^* /nix/store/bbbb-y.drv^out")
        );
        assert_eq!(asked(&words("/nix/store/aaaa-x.drv checks.x.a")), None);
        assert_eq!(asked(&words("--apply f /nix/store/aaaa-x.drv")), None);
        assert_eq!(asked(&words("--out /o /nix/store/aaaa-x.drv")), None);
        assert_eq!(asked(&words("/nix/store/aaaa-x")), None);
    }

    #[test]
    fn an_applied_function_or_an_output_copied_back_takes_one_attribute() {
        let one = asked(&words("--apply f --out /o github:o/r/abc checks.x.a")).unwrap();
        assert_eq!(one.apply.as_deref(), Some("f"));
        assert_eq!(one.out, Some(PathBuf::from("/o")));
        assert_eq!(
            asked(&words("--apply f github:o/r/abc checks.x.a checks.x.b")),
            None
        );
        assert_eq!(
            asked(&words("--out /o github:o/r/abc checks.x.a checks.x.b")),
            None
        );
        assert_eq!(asked(&words("github:o/r/abc checks.x.a --apply")), None);
    }

    #[test]
    fn an_applied_function_is_built_as_an_expression_impure_only_here() {
        let asked = asked(&[
            "--apply".to_owned(),
            "c: c.extend {}".to_owned(),
            "/t".to_owned(),
            "checks.x.a".to_owned(),
        ])
        .unwrap();
        assert_eq!(
            asked.build(),
            [
                "build",
                "--no-link",
                "--impure",
                "--expr",
                r#"(c: c.extend {}) (builtins.getFlake "/t").checks.x.a"#,
            ]
        );
        assert_eq!(
            asked.command_of("github:o/r/abc"),
            r#"nix build --no-link --expr '(c: c.extend {}) (builtins.getFlake "github:o/r/abc").checks.x.a'"#
        );
    }

    #[test]
    fn an_output_copied_back_comes_as_a_tarball_on_the_builders_output() {
        let asked = asked(&words("--out /o github:o/r/abc checks.x.a")).unwrap();
        assert_eq!(
            asked.command(),
            "out=$(nix build --no-link --print-out-paths 'github:o/r/abc#checks.x.a') \
             && tar -czhf - -C \"$out\" ."
        );
    }

    const QUICK: Waiting = Waiting {
        asks: true,
        answered: std::time::Duration::from_millis(300),
        restarting: std::time::Duration::from_secs(1),
        longest: std::time::Duration::from_secs(5),
        // Wide of what a stand-in ssh takes to fail at once on a loaded
        // machine, past 200 ms at load 55 on 4 November.
        worked: std::time::Duration::from_secs(1),
    };

    #[test]
    fn a_build_lost_again_after_working_waits_afresh_but_not_after_only_trying() {
        // Lost once, sent again and lost again after building a while, as
        // two switches 32 minutes apart cut one at 01:55 on 3 November, then
        // lost at once, then built.
        let folder = tempfile::tempdir().unwrap();
        let ssh = folder.path().join("ssh");
        fs::write(
            folder.path().join("tries"),
            "sleep 1.5; exit 255\nsleep 1.5; exit 255\nexit 255\nexit 0\n",
        )
        .unwrap();
        crate::testing::executable(
            &ssh,
            "#!/bin/sh\nhere=$(dirname \"$0\")\necho x >> \"$here/count\"\n\
             eval \"$(sed -n \"$(wc -l < \"$here/count\")p\" \"$here/tries\")\"\n",
        );
        let asked = asked(&["github:o/r/abc".to_owned(), "checks.x.a".to_owned()]).unwrap();
        let sinces = std::cell::RefCell::new(Vec::new());
        let again = |since| {
            sinces.borrow_mut().push(since);
            Some("ssh-ng://far\n".to_owned())
        };
        let ran = run(
            &asked,
            Some("ssh-ng://far\n"),
            &again,
            QUICK,
            folder.path(),
            [ssh.to_str().unwrap(), "false"],
            Signatures::Unchecked,
        );
        assert_eq!(ran.status, 0);
        let sinces = sinces.into_inner();
        assert_eq!(sinces.len(), 3, "lost three times");
        assert!(sinces[1] > sinces[0], "a loss after building waits afresh");
        assert_eq!(sinces[2], sinces[1], "a loss at once is more of the last");
    }

    #[test]
    fn a_build_whose_lender_is_restarting_waits_for_it_to_lend() {
        let dir = tempfile::tempdir().unwrap();
        // The lender that just stopped, as it does when it's superseded.
        fs::write(dir.path().join("lender"), "999999999\n").unwrap();
        let lender = dir.path().to_owned();
        let restarting = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));
            fs::write(lender.join("lender"), format!("{}\n", std::process::id())).unwrap();
            fs::write(lender.join("store"), "ssh-ng://far\n").unwrap();
        });
        assert_eq!(waited(dir.path(), QUICK).as_deref(), Some("ssh-ng://far\n"));
        restarting.join().unwrap();
    }

    #[test]
    fn a_run_waits_past_its_answer_for_root_to_vouch_for_a_builder_that_stands() {
        // On 22 November a run gave up 30 s into waiting on a builder
        // that stood while root, which can take minutes, hadn't vouched.
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("lender"),
            format!("{}\n", std::process::id()),
        )
        .unwrap();
        // "c3NoLWVkMjU1MTkgQUFBQQ==" is "ssh-ed25519 AAAA".
        let store = concat!(
            "ssh-ng://nix-offload-vm?",
            "base64-ssh-public-host-key=c3NoLWVkMjU1MTkgQUFBQQ==\n"
        );
        fs::write(dir.path().join("store"), store).unwrap();
        let lent = dir.path().join("lent");
        fs::write(&lent, "nix-offload other ssh-ed25519 AAAA\n").unwrap();
        let vouching = lent.clone();
        let began = std::time::Instant::now();
        let root = std::thread::spawn(move || {
            // Past QUICK's 300 ms answer, which once ended the wait.
            std::thread::sleep(std::time::Duration::from_secs(1));
            fs::write(&vouching, "nix-offload-vm vm-1 ssh-ed25519 AAAA\n").unwrap();
        });
        assert_eq!(
            waited_vouching(dir.path(), QUICK, &lent).as_deref(),
            Some(store),
            "it gave up before root vouched"
        );
        // The store stood from the start, so a wait that took it without
        // asking root would have answered at once with the same text.
        assert!(
            began.elapsed() >= std::time::Duration::from_secs(1),
            "it took the store before root vouched for it"
        );
        root.join().unwrap();
    }

    #[test]
    fn a_builder_root_never_vouches_for_is_never_taken() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("lender"),
            format!("{}\n", std::process::id()),
        )
        .unwrap();
        let store = concat!(
            "ssh-ng://nix-offload-vm?",
            "base64-ssh-public-host-key=c3NoLWVkMjU1MTkgQUFBQQ==\n"
        );
        fs::write(dir.path().join("store"), store).unwrap();
        // Root names the lender, but with another host key.
        let lent = dir.path().join("lent");
        fs::write(&lent, "nix-offload-vm vm-1 ssh-ed25519 BBBB\n").unwrap();
        let began = std::time::Instant::now();
        assert_eq!(waited_vouching(dir.path(), QUICK, &lent), None);
        // A standing builder is an answer, so it waits the whole wait.
        assert!(
            began.elapsed() >= QUICK.longest,
            "it gave up at the answer, not the longest wait"
        );
    }

    #[test]
    fn a_lender_a_fence_may_not_signal_still_runs() {
        assert!(
            stands(-1, &std::io::Error::from_raw_os_error(libc::EPERM)),
            "a lender the fence refused signal 0 to was taken for stopped"
        );
        assert!(
            !stands(-1, &std::io::Error::from_raw_os_error(libc::ESRCH)),
            "a pid no process holds was taken for a running lender"
        );
        assert!(
            runs(i32::try_from(std::process::id()).unwrap()),
            "this test's own process was taken for stopped"
        );
    }

    #[test]
    fn a_run_that_never_asks_asks_for_no_builder_though_a_lender_runs() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("lender"),
            format!("{}\n", std::process::id()),
        )
        .unwrap();
        let never = Waiting {
            asks: false,
            ..QUICK
        };
        let began = std::time::Instant::now();
        assert_eq!(waited(dir.path(), never), None);
        assert!(began.elapsed() < QUICK.answered, "it waits for nothing");
        assert!(
            !wants_of(dir.path()).exists(),
            "the lender is never told one is wanted"
        );
    }

    #[test]
    fn a_run_that_never_asks_asks_for_none_again_once_its_stale_builder_went() {
        // At 00:11 on 6 November the keeper's store named a builder that
        // had gone: ssh failed, and the run waited for one to stand again,
        // which asked the lender for one. The lender has since emptied it.
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("lender"),
            format!("{}\n", std::process::id()),
        )
        .unwrap();
        fs::write(dir.path().join("store"), "").unwrap();
        let never = Waiting {
            asks: false,
            ..QUICK
        };
        assert_eq!(
            stood_again(dir.path(), never, std::time::Instant::now()),
            None
        );
        assert!(
            !wants_of(dir.path()).exists(),
            "the lender is never told one is wanted"
        );
    }

    #[test]
    fn with_no_lender_running_nothing_is_waited_for() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(waited(dir.path(), QUICK), None);
        assert!(!wants_of(dir.path()).exists());
        fs::write(dir.path().join("lender"), "999999999\n").unwrap();
        let began = std::time::Instant::now();
        assert_eq!(
            waited(dir.path(), QUICK),
            None,
            "one that ended and never came back lends nothing"
        );
        assert!(
            began.elapsed() >= QUICK.restarting,
            "it waited as long as a restart takes"
        );
    }

    #[test]
    fn a_build_waits_wanting_a_builder_until_one_stands() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("lender"),
            format!("{}\n", std::process::id()),
        )
        .unwrap();
        let lender = dir.path().to_owned();
        let making = std::thread::spawn(move || {
            while !wanted(&lender) {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            fs::write(lender.join("lending"), "a-builder\n").unwrap();
            std::thread::sleep(std::time::Duration::from_millis(500));
            fs::write(lender.join("store"), "ssh-ng://far\n").unwrap();
        });
        assert_eq!(waited(dir.path(), QUICK).as_deref(), Some("ssh-ng://far\n"));
        making.join().unwrap();
        assert!(!wanted(dir.path()), "nothing is wanted once it stands");
    }

    #[test]
    fn a_build_stops_waiting_once_the_lender_makes_none() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("lender"),
            format!("{}\n", std::process::id()),
        )
        .unwrap();
        let began = std::time::Instant::now();
        assert_eq!(waited(dir.path(), QUICK), None);
        assert!(
            began.elapsed() < QUICK.longest,
            "it gave up once nothing was being made"
        );
    }

    #[test]
    fn a_build_lost_as_its_lender_restarts_waits_for_the_new_one() {
        let dir = tempfile::tempdir().unwrap();
        // The lender that just stopped, as a switch stops it.
        fs::write(dir.path().join("lender"), "999999999\n").unwrap();
        let lender = dir.path().to_owned();
        let restarting = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));
            fs::write(lender.join("store"), "ssh-ng://far\n").unwrap();
            fs::write(lender.join("lender"), format!("{}\n", std::process::id())).unwrap();
        });
        assert_eq!(
            stood_again(dir.path(), QUICK, std::time::Instant::now()).as_deref(),
            Some("ssh-ng://far\n")
        );
        restarting.join().unwrap();
    }

    #[test]
    fn a_build_lost_as_its_lender_restarts_slowly_still_waits_for_it() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("lender"), "999999999\n").unwrap();
        let lender = dir.path().to_owned();
        // Back after longer than a lender takes to answer, as a switch
        // that does much else between stopping and starting it is.
        let restarting = std::thread::spawn(move || {
            std::thread::sleep(QUICK.answered * 2);
            fs::write(lender.join("store"), "ssh-ng://far\n").unwrap();
            fs::write(lender.join("lender"), format!("{}\n", std::process::id())).unwrap();
        });
        assert_eq!(
            stood_again(dir.path(), QUICK, std::time::Instant::now()).as_deref(),
            Some("ssh-ng://far\n")
        );
        restarting.join().unwrap();
    }

    #[test]
    fn a_build_lost_where_no_lender_ever_ran_waits_only_for_an_answer() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("store"), "ssh-ng://far\n").unwrap();
        let began = std::time::Instant::now();
        assert_eq!(stood_again(dir.path(), QUICK, began), None);
        assert!(
            began.elapsed() < QUICK.restarting,
            "it waited as for a lender restarting"
        );
    }

    #[test]
    fn a_build_lost_with_no_lender_coming_back_stops_waiting() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("lender"), "999999999\n").unwrap();
        fs::write(dir.path().join("store"), "ssh-ng://far\n").unwrap();
        let began = std::time::Instant::now();
        assert_eq!(stood_again(dir.path(), QUICK, began), None);
        assert!(
            began.elapsed() < QUICK.longest,
            "it gave up once no lender came back"
        );
    }

    /// VM builder folders `names` under a folder of their own, with `waits`
    /// VM tests waiting for room in the first, by the place it gives.
    fn lanes_waiting(names: &[&str], waits: usize) -> (tempfile::TempDir, Vec<PathBuf>, Claim) {
        let folder = tempfile::tempdir().unwrap();
        let dirs: Vec<PathBuf> = names.iter().map(|name| folder.path().join(name)).collect();
        for dir in &dirs {
            fs::create_dir(dir).unwrap();
        }
        let place = queued(&dirs[0], waits, Rank::Plain).expect("a place");
        (folder, dirs, place)
    }

    #[test]
    fn a_run_waiting_past_the_coming_room_asks_a_lane_nobody_asked() {
        let (_folder, dirs, _place) = lanes_waiting(&["vm-3", "vm-8", "vm-12"], 4);
        let others = [dirs[1].as_path(), dirs[2].as_path()];
        assert_eq!(
            to_ask(&dirs[0], &others, VM_ROOM),
            Some(dirs[1].as_path()),
            "4 VM tests waiting with no lane coming asked none, or not the first"
        );
        fs::write(dirs[1].join("lending"), "a-builder\n").unwrap();
        assert_eq!(
            to_ask(&dirs[0], &others, VM_ROOM),
            None,
            "a lane standing up for 4 waiting wasn't room enough"
        );
    }

    #[test]
    fn a_run_asks_past_lanes_coming_or_asked_for() {
        let (_folder, dirs, _place) = lanes_waiting(&["vm-3", "vm-8", "vm-12", "vm-14"], 12);
        fs::write(dirs[1].join("lending"), "a-builder\n").unwrap();
        let asked = wanting(&dirs[2]).expect("a want");
        let others = [dirs[1].as_path(), dirs[2].as_path(), dirs[3].as_path()];
        assert_eq!(
            to_ask(&dirs[0], &others, VM_ROOM),
            Some(dirs[3].as_path()),
            "12 waiting past two lanes coming didn't ask the third, the one nobody had"
        );
        let more = wanting(&dirs[3]).expect("a want");
        assert_eq!(
            to_ask(&dirs[0], &others, VM_ROOM),
            None,
            "a lane was asked for with room coming for all 12"
        );
        drop((asked, more));
    }

    #[test]
    fn a_run_asks_nothing_while_none_waits() {
        let folder = tempfile::tempdir().unwrap();
        let (dir, other) = (folder.path().join("vm-3"), folder.path().join("vm-8"));
        fs::create_dir(&dir).unwrap();
        fs::create_dir(&other).unwrap();
        assert_eq!(
            to_ask(&dir, &[other.as_path()], VM_ROOM),
            None,
            "a lane was asked for with nothing waiting"
        );
    }

    #[test]
    fn a_want_whose_build_ended_is_taken_away() {
        let folder = tempfile::tempdir().unwrap();
        let dir = folder.path().join("nix-offload-vm-5");
        fs::create_dir(&dir).unwrap();
        let wants = wants_of(&dir);
        assert_eq!(
            wants,
            folder.path().join("offload-wants/nix-offload-vm-5"),
            "wants are beside the lender's folder, by its name"
        );
        fs::create_dir_all(&wants).unwrap();
        // Named for this run's pid, but nobody holds it: a fenced run's
        // pid means nothing here.
        let left = wants.join(std::process::id().to_string());
        fs::write(&left, "").unwrap();
        assert!(!wanted(&dir), "a want nobody holds was counted");
        assert!(!left.exists(), "a want nobody holds was left");
        let held = wanting(&dir).expect("a want");
        assert!(wanted(&dir), "a held want wasn't counted");
        assert!(wanted(&dir), "a held want was taken away");
        drop(held);
        // As a lender's file is, below, a want's lock can outlive it
        // briefly in a child another test forked.
        let since = Instant::now();
        while wanted(&dir) && since.elapsed() < Duration::from_secs(5) {
            thread::sleep(Duration::from_millis(20));
        }
        assert!(!wanted(&dir), "a want let go was still counted");
        assert_eq!(
            fs::read_dir(&wants).unwrap().count(),
            0,
            "one let go is gone"
        );
    }

    #[test]
    fn a_lender_whose_pid_names_nothing_here_runs_while_it_holds_its_file() {
        let dir = tempfile::tempdir().unwrap();
        let lender = dir.path().join("lender");
        // As a fenced run reads it: the lender's pid is in another pid
        // namespace, so here it names no process.
        fs::write(&lender, "999999999\n").unwrap();
        assert!(
            !lends(dir.path()),
            "a lender holding nothing was taken to run"
        );
        let holding = fs::File::open(&lender).unwrap();
        holding.lock_shared().unwrap();
        assert!(
            lends(dir.path()),
            "a lender holding its file was taken for stopped"
        );
        drop(holding);
        // A lock is the open file's, so a child another test forks in the
        // moment before it execs holds it a little longer: under load on
        // 16 November that read as a lender still running.
        let since = Instant::now();
        while lends(dir.path()) && since.elapsed() < Duration::from_secs(5) {
            thread::sleep(Duration::from_millis(20));
        }
        assert!(!lends(dir.path()), "a lender that let go was taken to run");
    }

    #[test]
    fn a_github_remote_names_its_repository() {
        for url in [
            "https://github.com/o/r",
            "https://github.com/o/r.git",
            "git@github.com:o/r.git",
            "ssh://git@github.com/o/r",
        ] {
            assert_eq!(github_of(url).as_deref(), Some("o/r"), "{url}");
        }
        assert_eq!(github_of("https://delta.example/o/r"), None);
        assert_eq!(github_of("https://github.com/o"), None);
    }

    #[test]
    fn a_build_with_no_attribute_or_an_unknown_flag_is_refused() {
        assert_eq!(asked(&words("github:o/r/abc")), None);
        assert_eq!(asked(&words("-L")), None);
        assert_eq!(asked(&words("--impure github:o/r/abc checks.x.a")), None);
        // A folder's copy and the store's paths are two answers to one ask.
        assert_eq!(
            asked(&words(
                "--out /o --print-out-paths github:o/r/abc checks.x.a"
            )),
            None
        );
    }

    #[test]
    fn the_builder_is_given_one_quoted_command() {
        let asked = asked(&words("-L github:o/r/abc checks.x.it's")).unwrap();
        assert_eq!(
            asked.command(),
            r"nix build --no-link -L 'github:o/r/abc#checks.x.it'\''s'"
        );
    }

    #[test]
    fn a_store_names_its_host_its_key_its_host_key_and_itself() {
        let store = "ssh-ng://nix-offload?compress=true&ssh-key=/k&\
                     base64-ssh-public-host-key=c3NoLWVkMjU1MTkgQUFBQQ==";
        assert_eq!(
            builder(store),
            Some(Builder {
                host: "nix-offload".to_owned(),
                key: Some("/k".to_owned()),
                host_key: Some("ssh-ed25519 AAAA".to_owned()),
                store: store.to_owned(),
            })
        );
        assert_eq!(
            builder("ssh://someone@far").map(|lent| lent.host),
            Some("someone@far".to_owned())
        );
        assert_eq!(builder("daemon"), None);
        assert_eq!(builder("ssh-ng://?ssh-key=/k"), None);
    }

    /// The arguments `reach` gives ssh for a builder whose key is `key`.
    fn reached_with(key: &Path) -> Vec<String> {
        let scratch = tempfile::tempdir().unwrap();
        let store = format!("ssh-ng://nix-offload?ssh-key={}", key.display());
        let lent = builder(&store).unwrap();
        let (command, _, _) = reach(&lent, scratch.path(), "ssh").unwrap();
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn a_builder_with_a_key_of_its_own_is_reached_without_the_agent() {
        let keys = tempfile::tempdir().unwrap();
        let key = keys.path().join("store_ed25519");
        fs::write(&key, "a private key").unwrap();
        let missing = keys.path().join("missing_ed25519");

        for key in [key, missing] {
            let args = reached_with(&key);

            assert!(
                args.windows(2)
                    .any(|pair| pair == ["-o", "IdentityAgent=none"]),
                "a key that reads, or isn't there, keeps the agent out: {args:?}"
            );
            assert!(
                args.windows(2)
                    .any(|pair| pair == ["-o", "IdentitiesOnly=yes"]),
                "{args:?}"
            );
        }
    }

    #[test]
    fn a_builder_root_vouches_for_is_waited_on_no_longer() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join("lent");
        fs::write(
            &file,
            "# vouched for now\nnix-offload-vm offload-vm-abc ssh-ed25519 AAAA\n",
        )
        .unwrap();

        assert_eq!(
            vouched(&file, "nix-offload-vm", "ssh-ed25519 AAAA", Duration::ZERO),
            Ok(())
        );
        let refused = vouched(&file, "nix-offload", "ssh-ed25519 AAAA", Duration::ZERO);
        assert!(
            refused
                .as_ref()
                .is_err_and(|why| why.contains("nix-offload's builder")),
            "another lender's builder isn't vouched for by this one's line: {refused:?}"
        );
        let other = vouched(&file, "nix-offload-vm", "ssh-ed25519 BBBB", Duration::ZERO);
        assert!(other.is_err(), "nor is another host key: {other:?}");
        let missing = vouched(
            &folder.path().join("none"),
            "nix-offload-vm",
            "ssh-ed25519 AAAA",
            Duration::ZERO,
        );
        assert!(
            missing.is_err(),
            "nor is anything without the file: {missing:?}"
        );
    }

    #[test]
    fn only_a_builder_root_names_with_its_host_key_is_vouched_for() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join("lent");
        fs::write(&file, "nix-offload-vm vm-1 ssh-ed25519 AAAA\n").unwrap();
        // "c3NoLWVkMjU1MTkgQUFBQQ==" is "ssh-ed25519 AAAA", and the
        // other "ssh-ed25519 BBBB".
        let store = |host: &str, key: &str| {
            format!("ssh-ng://{host}?ssh-key=/key&base64-ssh-public-host-key={key}\n")
        };
        let aaaa = "c3NoLWVkMjU1MTkgQUFBQQ==";
        let bbbb = "c3NoLWVkMjU1MTkgQkJCQg==";

        assert!(
            vouched_in(&file, &store("nix-offload-vm", aaaa)),
            "a builder lent names with the same key is vouched for"
        );
        assert!(
            vouched_in(&file, &store("hook@nix-offload-vm", aaaa)),
            "the account isn't the lender's name"
        );
        assert!(
            !vouched_in(&file, &store("nix-offload-vm-4", aaaa)),
            "another lender's builder isn't"
        );
        assert!(
            !vouched_in(&file, &store("nix-offload-vm", bbbb)),
            "nor a builder with another host key"
        );
        assert!(
            !vouched_in(&file, "ssh-ng://nix-offload-vm\n"),
            "nor one pinning no host key"
        );
        assert!(
            vouched_in(&folder.path().join("none"), "ssh-ng://far\n"),
            "with no file of root's, every builder is"
        );
        assert!(
            !vouched_in(folder.path(), &store("nix-offload-vm", aaaa)),
            "a file that can't be read vouches for none"
        );
    }

    #[test]
    fn a_builder_whose_key_is_hidden_is_reached_through_the_agent() {
        let keys = tempfile::tempdir().unwrap();
        let key = keys.path().join("hidden_ed25519");
        fs::write(&key, "").unwrap();

        let args = reached_with(&key);

        assert!(
            !args.iter().any(|arg| arg == "IdentityAgent=none"),
            "a hidden key leaves the agent to sign: {args:?}"
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["-o", "IdentitiesOnly=yes"]),
            "only the builders' key is offered still: {args:?}"
        );
        assert!(
            args.windows(2)
                .any(|pair| pair[0] == "-i" && pair[1] == key.to_string_lossy()),
            "ssh finds the key's public half beside it: {args:?}"
        );
    }

    #[test]
    fn base64_is_decoded_whatever_its_padding() {
        assert_eq!(decoded("YQ==").as_deref(), Some("a"));
        assert_eq!(decoded("YWI=").as_deref(), Some("ab"));
        assert_eq!(decoded("YWJj").as_deref(), Some("abc"));
        assert_eq!(decoded("not base64!"), None);
    }

    fn mac() -> Nearby {
        Nearby {
            address: "mac.local:22".to_owned(),
            host: "mac".to_owned(),
            systems: words("aarch64-darwin aarch64-linux"),
            attributes: Vec::new(),
        }
    }

    #[test]
    fn nearby_machines_are_read_a_line_each() {
        let text = "# at home\n\nmac.local:22 mac aarch64-darwin,aarch64-linux\nshort line\n";
        assert_eq!(nearby(text), vec![mac()]);
        let named = "mac.local:22 mac aarch64-darwin,aarch64-linux \
                     checks.x86_64-linux.formatting,checks.x86_64-linux.*-evaluates\n";
        assert_eq!(
            nearby(named)[0].attributes,
            words("checks.x86_64-linux.formatting checks.x86_64-linux.*-evaluates")
        );
    }

    #[test]
    fn a_star_in_a_pattern_stands_for_any_run_of_characters() {
        assert!(matches("checks.x.formatting", "checks.x.formatting"));
        assert!(!matches("checks.x.formatting", "checks.x.formatting-too"));
        assert!(matches(
            "checks.x.*-evaluates",
            "checks.x.linux-dev-evaluates"
        ));
        assert!(matches("checks.x.*-evaluates", "checks.x.-evaluates"));
        assert!(!matches("checks.x.*-evaluates", "checks.y.dev-evaluates"));
        assert!(matches("checks.*.a*b", "checks.x.aXbYb"));
        assert!(!matches("checks.*.a*b", "checks.x.aXbY"));
    }

    #[test]
    fn a_nearby_machine_is_asked_whether_it_answers_only_when_it_takes_something_asked() {
        // An away machine's name can take seconds to fail to resolve, which
        // every build it would never take shouldn't wait out.
        let machines = [mac()];
        let probed = std::cell::Cell::new(0);
        let answers = |_: &crate::there::Nearby| {
            probed.set(probed.get() + 1);
            true
        };
        let linux = asked(&words("path:/z checks.x86_64-linux.a")).unwrap();
        let (near, _) = shared(&linux, &machines, answers);
        assert_eq!(
            near,
            [] as [(&crate::there::Nearby, crate::there::Asked); 0]
        );
        assert_eq!(probed.get(), 0, "nothing asked is the machine's");
        let darwin = asked(&words("path:/z checks.aarch64-darwin.b")).unwrap();
        let (near, _) = shared(&darwin, &machines, answers);
        assert_eq!(near.len(), 1);
        assert_eq!(probed.get(), 1);
    }

    #[test]
    fn an_attribute_a_nearby_machine_names_goes_to_it_whatever_its_system() {
        let machines = [Nearby {
            attributes: words("checks.x86_64-linux.formatting checks.x86_64-linux.*-evaluates"),
            ..mac()
        }];
        let asked = asked(&words(
            "path:/z checks.x86_64-linux.formatting checks.x86_64-linux.dev-evaluates \
             checks.x86_64-linux.vm-boots checks.aarch64-darwin.b",
        ))
        .unwrap();
        let (near, left) = shared(&asked, &machines, |_| true);
        assert_eq!(
            near[0].1.attributes,
            words(
                "checks.x86_64-linux.formatting checks.x86_64-linux.dev-evaluates \
                 checks.aarch64-darwin.b"
            )
        );
        assert_eq!(
            left.map(|left| left.attributes),
            Some(words("checks.x86_64-linux.vm-boots"))
        );
    }

    #[test]
    fn each_attribute_goes_to_a_nearby_machine_taking_its_system_while_it_answers() {
        let machines = [mac()];
        let asked = asked(&words(
            "-L path:/z checks.x86_64-linux.a checks.aarch64-darwin.b \
             checks.aarch64-linux.c equilibrium",
        ))
        .unwrap();
        let (near, left) = shared(&asked, &machines, |_| true);
        assert_eq!(near.len(), 1);
        assert_eq!(near[0].0, &machines[0]);
        assert_eq!(
            near[0].1.command(),
            "nix build --no-link -L 'path:/z#checks.aarch64-darwin.b' \
             'path:/z#checks.aarch64-linux.c'"
        );
        assert_eq!(
            left.map(|left| left.attributes),
            Some(words("checks.x86_64-linux.a equilibrium"))
        );

        let (near, left) = shared(&asked, &machines, |_| false);
        assert!(near.is_empty(), "one away takes nothing");
        assert_eq!(left.map(|left| left.attributes.len()), Some(4));

        let darwin = super::asked(&words("path:/z checks.aarch64-darwin.b")).unwrap();
        assert_eq!(shared(&darwin, &machines, |_| true).1, None);
    }

    #[test]
    fn what_only_an_away_machine_builds_for_is_owed_rather_than_sent_the_lent_way() {
        let machines = [mac()];
        let asked = asked(&words(
            "-L path:/z checks.x86_64-linux.a checks.aarch64-darwin.b \
             checks.aarch64-linux.c checks.riscv64-linux.d equilibrium",
        ))
        .unwrap();
        let (_, left) = shared(&asked, &machines, |_| false);
        let (left, unbuilt) = owed(left, &machines, "x86_64-linux");
        assert_eq!(
            unbuilt,
            words("checks.aarch64-darwin.b checks.aarch64-linux.c"),
            "only the away machine builds for its systems"
        );
        assert_eq!(
            left.map(|left| left.attributes),
            Some(words(
                "checks.x86_64-linux.a checks.riscv64-linux.d equilibrium"
            )),
            "what no machine nearby names still goes the lent way"
        );

        let darwin = super::asked(&words("path:/z checks.aarch64-darwin.b")).unwrap();
        let (_, left) = shared(&darwin, &machines, |_| false);
        let (left, unbuilt) = owed(left, &machines, "x86_64-linux");
        assert_eq!(left, None, "nothing of it is the lent builder's");
        assert_eq!(unbuilt, words("checks.aarch64-darwin.b"));

        let (left, unbuilt) = owed(Some(darwin), &machines, "aarch64-darwin");
        assert!(
            unbuilt.is_empty(),
            "a Mac's own system is built where it is"
        );
        assert!(left.is_some(), "a Mac's own system was left unasked");
    }

    #[test]
    fn a_builders_cpu_pressure_is_its_last_minute_of_some_waiting() {
        let read = "some avg10=53.71 avg60=48.10 avg300=55.00 total=1299145659\n\
                    full avg10=0.00 avg60=0.00 avg300=0.00 total=0\n";
        assert_eq!(pressure_of(read), Some(48.10));
        assert_eq!(pressure_of("full avg60=3.00\n"), None);
        assert_eq!(pressure_of(""), None);
    }

    #[test]
    fn a_second_builder_takes_only_what_overflows_a_full_first() {
        let room = FULL - 1.0;
        assert!(
            to_second(Some(FULL), &Second::Unlent),
            "full, so one is asked for"
        );
        assert!(
            !to_second(Some(room), &Second::Unlent),
            "room left on the first"
        );
        assert!(
            to_second(Some(60.0), &Second::Stands(Some(5.0))),
            "one less busy"
        );
        assert!(
            !to_second(Some(room), &Second::Stands(Some(5.0))),
            "room on the first"
        );
        assert!(
            !to_second(Some(60.0), &Second::Stands(Some(70.0))),
            "one busier"
        );
        assert!(!to_second(Some(90.0), &Second::Stands(None)), "one unread");
        assert!(
            !to_second(None, &Second::Unlent),
            "a first that can't be read keeps it with no second"
        );
        assert!(
            !to_second(None, &Second::Stands(None)),
            "nor to a second that can't be read either"
        );
        assert!(
            to_second(None, &Second::Stands(Some(90.0))),
            "a first that can't be read goes to a second that answers, however busy"
        );
    }

    #[test]
    fn cold_vm_builders_are_asked_for_as_the_warm_ones_room_runs_out() {
        assert_eq!(to_warm(6, &[0], 6, 0), 0, "all fit the warm one");
        assert_eq!(to_warm(7, &[0], 6, 0), 1, "one past it");
        assert_eq!(to_warm(3, &[5, 6], 6, 0), 1, "two past both");
        assert_eq!(to_warm(13, &[0], 6, 1), 1, "seven past, one on its way");
        assert_eq!(to_warm(7, &[0], 6, 2), 0, "more on their way than needed");
    }

    #[test]
    fn a_vm_builder_is_warm_once_it_stands_and_isnt_warming() {
        let folder = tempfile::tempdir().unwrap();
        let dir = folder.path();
        assert!(!warm(dir), "none stands");
        fs::write(dir.join("store"), "ssh-ng://b\n").unwrap();
        assert!(warm(dir));
        fs::write(dir.join(WARMING), format!("{} b-1\n", std::process::id())).unwrap();
        assert!(!warm(dir), "its lender warms it");
        fs::write(dir.join(WARMING), "999999999 b-1\n").unwrap();
        assert!(warm(dir), "a lender that stopped warms nothing");
    }

    #[test]
    fn a_builder_warms_by_building_what_its_lender_names() {
        let folder = tempfile::tempdir().unwrap();
        let ssh = folder.path().join("ssh");
        crate::testing::executable(
            &ssh,
            "#!/bin/sh\nhere=$(dirname \"$0\")\nfor word; do last=$word; done\n\
             printf '%s\\n' \"$last\" > \"$here/told\"\n[ ! -f \"$here/fails\" ]\n",
        );
        let lent = builder("ssh-ng://b").unwrap();
        let installables = ["github:o/r#checks.x.vm-a.driver".to_owned()];
        let scratch = folder.path().join("scratch");
        let ssh = ssh.to_str().unwrap();
        assert!(warmed(&lent, &installables, &scratch, ssh));
        assert_eq!(
            fs::read_to_string(folder.path().join("told")).unwrap(),
            "nix build --no-link --keep-going 'github:o/r#checks.x.vm-a.driver'\n"
        );
        fs::write(folder.path().join("fails"), "").unwrap();
        assert!(
            !warmed(&lent, &installables, &scratch, ssh),
            "one it couldn't build"
        );
    }

    #[test]
    fn vm_tests_fill_the_first_vm_builder_then_the_second_then_the_less_busy() {
        assert_eq!(
            spread(8, &[0, 0], 6),
            [6, 2],
            "the first's room, the rest on"
        );
        assert_eq!(spread(3, &[0, 0], 6), [3, 0], "all fit the first");
        assert_eq!(spread(3, &[5, 0], 6), [1, 2], "another run's five there");
        assert_eq!(spread(2, &[6, 0], 6), [0, 2], "the first full");
        assert_eq!(spread(4, &[6, 6], 6), [2, 2], "both full, shared evenly");
        assert_eq!(spread(3, &[9, 7], 6), [1, 2], "the less busy first");
        assert_eq!(spread(0, &[0, 0], 6), [0, 0]);
        assert_eq!(spread(3, &[3], 4), [3], "one alone takes them all");
    }

    #[test]
    fn vm_tests_past_every_vm_builders_room_go_to_the_next_then_the_least_busy() {
        assert_eq!(spread(9, &[0, 0, 0], 4), [4, 4, 1], "each's room in turn");
        assert_eq!(
            spread(3, &[4, 4, 0, 0], 4),
            [0, 0, 3, 0],
            "the first with room"
        );
        assert_eq!(
            spread(3, &[4, 2, 4], 4),
            [1, 2, 0],
            "the second's room, then the least busy"
        );
        assert_eq!(spread(4, &[5, 6, 5], 4), [2, 1, 1], "none has room");
    }

    #[test]
    fn a_vm_part_overflowing_its_builder_is_split_keeping_its_order() {
        let asked = asked(&words(
            "-L path:/z checks.x.vm-a checks.x.vm-b checks.x.vm-c",
        ))
        .unwrap();
        let parts = overflow(&asked, &[4, 0], 6);
        assert_eq!(
            parts[0].as_ref().map(|first| first.attributes.clone()),
            Some(words("checks.x.vm-a checks.x.vm-b"))
        );
        let second = parts[1].as_ref().unwrap();
        assert_eq!(second.attributes, words("checks.x.vm-c"));
        assert_eq!(second.flags, words("-L"));
        assert_eq!(overflow(&asked, &[0, 0], 6)[1], None, "all fit the first");
        assert_eq!(overflow(&asked, &[6, 0], 6)[0], None, "none fit the first");
        let parts = overflow(&asked, &[6, 5, 0], 6);
        assert_eq!(
            parts
                .iter()
                .map(|part| part.as_ref().map(count))
                .collect::<Vec<_>>(),
            [None, Some(1), Some(2)],
            "on to a third"
        );
    }

    #[test]
    fn a_vm_builders_claims_count_while_their_runs_live() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(claimed(dir.path()), 0, "none yet");
        let held = claim(dir.path(), 4);
        let other = claim(dir.path(), 2);
        assert_eq!(claimed(dir.path()), 6);
        fs::write(dir.path().join("sending/999999999.0"), "5\n").unwrap();
        assert_eq!(claimed(dir.path()), 6, "a gone run's counts for nothing");
        drop(held);
        assert_eq!(claimed(dir.path()), 2, "taken away as its run ends");
        drop(other);
        assert_eq!(fs::read_dir(dir.path().join("sending")).unwrap().count(), 0);
    }

    #[test]
    fn vm_tests_are_split_off_for_their_own_builder() {
        let patterns = words("checks.*.vm-* *-boots");
        let asked = asked(&words(
            "-L path:/z checks.x86_64-linux.vm-desktop checks.x86_64-linux.formatting \
             usb-boots checks.x86_64-linux.vm-greeter",
        ))
        .unwrap();
        let (vm, rest) = vm_tests(&asked, &patterns, &BTreeSet::new());
        let vm = vm.unwrap();
        assert_eq!(
            vm.command(),
            "nix build --no-link -L 'path:/z#checks.x86_64-linux.vm-desktop' \
             'path:/z#usb-boots' 'path:/z#checks.x86_64-linux.vm-greeter'"
        );
        assert_eq!(
            rest.map(|rest| rest.attributes),
            Some(words("checks.x86_64-linux.formatting"))
        );
    }

    #[test]
    fn with_no_vm_test_asked_everything_stays_as_it_was() {
        let patterns = words("checks.*.vm-*");
        let asked = asked(&words("path:/z checks.x86_64-linux.formatting")).unwrap();
        let (vm, rest) = vm_tests(&asked, &patterns, &BTreeSet::new());
        assert_eq!(vm, None);
        assert_eq!(
            rest.map(|rest| rest.attributes),
            Some(words("checks.x86_64-linux.formatting"))
        );
        let only = super::asked(&words("path:/z checks.x86_64-linux.vm-a")).unwrap();
        assert_eq!(vm_tests(&only, &patterns, &BTreeSet::new()).1, None);
    }

    #[test]
    fn derivations_are_split_by_what_they_need_of_a_builder() {
        // A VM test given as its derivation went the lent way, whatever its
        // name: on 3 November one built 2407 s on the third builder.
        let patterns = words("checks.*.vm-*");
        let featured = BTreeSet::from(["/nix/store/aaaa-vm-test-run-x.drv".to_owned()]);
        let drvs = asked(&words(
            "-L /nix/store/bbbb-kernel.drv /nix/store/aaaa-vm-test-run-x.drv^out",
        ))
        .unwrap();
        let (vm, rest) = vm_tests(&drvs, &patterns, &featured);
        assert_eq!(
            vm.unwrap().command(),
            "nix build --no-link -L '/nix/store/aaaa-vm-test-run-x.drv^out'"
        );
        assert_eq!(
            rest.unwrap().command(),
            "nix build --no-link -L '/nix/store/bbbb-kernel.drv^*'"
        );
        // Named like one, a derivation that needs no VM isn't one.
        let named = asked(&words("/nix/store/cccc-vm-test-run-y.drv")).unwrap();
        assert_eq!(vm_tests(&named, &patterns, &featured).0, None);
    }

    #[test]
    fn an_applied_or_copied_build_of_a_vm_test_goes_whole_to_its_builder() {
        // run-subtests and shots-changed build one VM test so, and theirs
        // went the lent way, up to 2505 s on 3 November.
        let patterns = words("checks.*.vm-* legacyPackages.*.subtests");
        let none = BTreeSet::new();
        for words_given in [
            "--apply f --out /o path:/z legacyPackages.x86_64-linux.subtests",
            "--out /o path:/z checks.x86_64-linux.vm-a",
        ] {
            let applied = asked(&words(words_given)).unwrap();
            let (vm, rest) = vm_tests(&applied, &patterns, &none);
            assert_eq!(
                vm,
                Some(applied.with(&applied.flake, applied.attributes.clone()))
            );
            assert_eq!(rest, None, "{words_given}");
        }
        let other = asked(&words("--apply f path:/z packages.x86_64-linux.a")).unwrap();
        assert_eq!(vm_tests(&other, &patterns, &none).0, None);
    }

    #[test]
    fn a_derivation_needs_a_vm_when_its_features_ask_for_kvm_or_nixos_test() {
        let shown = serde_json::json!({"version": 4, "derivations": {
            "aaaa-vm-test-run-x.drv": {"env": {"requiredSystemFeatures": "kvm nixos-test"}},
            "bbbb-kernel.drv": {"env": {"requiredSystemFeatures": "big-parallel"}},
            "cccc-plain.drv": {"env": {}},
            "dddd-structured.drv": {"structuredAttrs": {"requiredSystemFeatures": ["kvm"]}},
            "eeee-json.drv": {"env": {"__json": "{\"requiredSystemFeatures\":[\"nixos-test\"]}"}}
        }});
        assert_eq!(
            needing_vm(&shown),
            BTreeSet::from([
                "/nix/store/aaaa-vm-test-run-x.drv".to_owned(),
                "/nix/store/dddd-structured.drv".to_owned(),
                "/nix/store/eeee-json.drv".to_owned(),
            ])
        );
        let before = serde_json::json!({
            "/nix/store/aaaa-x.drv": {"env": {"requiredSystemFeatures": "nixos-test"}}
        });
        assert_eq!(
            needing_vm(&before),
            BTreeSet::from(["/nix/store/aaaa-x.drv".to_owned()])
        );
    }

    #[test]
    fn only_a_flake_that_is_a_folder_here_is_sent_as_one() {
        assert_eq!(folder_of("path:/z?dir=x"), Some(PathBuf::from("/z")));
        assert_eq!(folder_of("./z"), Some(PathBuf::from("./z")));
        assert_eq!(folder_of("github:o/r/abc"), None);
        assert_eq!(folder_of("git+https://h/r"), None);
    }

    #[test]
    fn the_inputs_a_lock_pins_to_the_store_are_named() {
        let lock = r#"{"nodes": {
            "mint": {"locked": {"type": "path", "path": "/nix/store/abc-mint"}},
            "near": {"locked": {"type": "path", "path": "/home/me/near"}},
            "core": {"locked": {"type": "github", "owner": "o"}},
            "root": {"inputs": {}}
        }}"#;
        assert_eq!(pinned(lock), words("/nix/store/abc-mint"));
        assert_eq!(pinned("not a lock"), [] as [std::string::String; 0]);
    }

    #[test]
    fn what_ssh_sent_is_read_from_its_last_count() {
        let log = "Authenticated to far\n\
                   Transferred: sent 4321, received 98765 bytes, in 3.2 seconds\n";
        assert_eq!(sent(log), Some(4321));
        assert_eq!(sent("Authenticated to far\n"), None);
    }
}
