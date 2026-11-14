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
//! has.
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
            Alone::Ends => far,
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
    /// as a tarball on its standard output, the log staying on its error.
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
            format!("out=$({build}) && tar -czf - -C \"$out\" .")
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
    let lent = store.and_then(|store| builder(store.lines().next()?.trim()));
    let Some(mut lent) = lent else {
        return unsent(asked, nix, "no builder is lent");
    };
    // What the flake's lock takes from this machine alone: store paths go
    // ahead of it, and a failure there for want of the rest comes back to
    // be made here (`kept.rs`).
    let inputs = folder_of(&asked.flake)
        .filter(|_| !asked.derivations)
        .and_then(|folder| fs::read_to_string(folder.join("flake.lock")).ok())
        .map(|lock| kept::inputs_of(&lock))
        .unwrap_or_default();
    let reached = sending_of(asked, scratch).map(|mut sending| {
        sending.command = kept::wrapped(&sending.command, &inputs.kept);
        sending
    });
    let reached = reached.and_then(|sending| {
        let ran = loop {
            let mut tried = std::time::Instant::now();
            // Copied to each builder that stands, since one stood again
            // after a loss holds none of them.
            let copied = match all_up(asked, &inputs, &lent, scratch, [ssh, nix]) {
                Err(untaken) => break Err(untaken),
                Ok(copied) => copied,
            };
            let building = copied.is_ok();
            let ran = match copied {
                Ok(()) => {
                    let bring = match (&asked.out, asked.print) {
                        (Some(out), _) => Bring::Folder(out),
                        (None, true) => Bring::Store(nix, signatures),
                        (None, false) => Bring::Nothing,
                    };
                    tried = std::time::Instant::now();
                    let ran = on_with(&lent, &sending, bring, scratch, ssh, asked.alone.wrap());
                    if !matches!(&ran, Ok(ran) if ran.status == UNREACHED) {
                        break ran.map_err(Untaken::Waits);
                    }
                    ran
                }
                Err(uncopied) => {
                    eprintln!("nix-offload: {}", uncopied.why);
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
            eprintln!(
                "nix-offload: ssh lost {}, so this waits for a builder to stand again, for up \
                 to {} minutes from {}",
                lent.host,
                waiting.longest.as_secs().div_ceil(60),
                if since == lost {
                    "now"
                } else {
                    "its last loss"
                }
            );
            // A builder that stayed up goes on with what it was building for
            // `far`'s grace period, and the build sent again waits on that, so
            // it says how it ends but shows none of its log: a VM test cut off
            // mid-test on 2 November passed there, and only its exit said so.
            // One lost while copying up had nothing building.
            if building {
                eprintln!(
                    "nix-offload: if it stands again within {} minutes, what it was building \
                     goes on there, and this shows only how it ends; `nix log` on the builder \
                     has the rest",
                    asked.alone.lasts() / 60
                );
            }
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
        Ok(_) => unsent(asked, nix, &format!("ssh couldn't reach {}", lent.host)),
        Err(Untaken::Waits(why)) => unsent(asked, nix, &why),
        Err(Untaken::Stays(why)) => stays(asked, nix, &why),
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
) -> Result<Result<(), Uncopied>, Untaken> {
    match asked
        .derivations
        .then(|| up(asked, lent, scratch, [ssh, nix]))
    {
        Some(Err(uncopied)) if !uncopied.lost => Err(Untaken::Waits(uncopied.why)),
        Some(Err(uncopied)) => Ok(Err(uncopied)),
        _ => match inputs_up(inputs, lent, nix) {
            Err(uncopied) if !uncopied.lost => Err(Untaken::Stays(uncopied.why)),
            copied => Ok(copied),
        },
    }
}

/// Sends `lent` the store paths the flake takes from this machine alone,
/// when it takes any.
fn inputs_up(inputs: &kept::Inputs, lent: &Builder, nix: &str) -> Result<(), Uncopied> {
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
    sent_up(&inputs.sent, lent, nix, send_within(0))
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
/// user manager to ask, it runs as it is.
fn upload(nix: &str) -> Command {
    let bus = std::env::var_os("XDG_RUNTIME_DIR")
        .is_some_and(|runtime| Path::new(&runtime).join("bus").exists());
    uploading(nix, SYSTEMD_RUN.filter(|_| bus))
}

/// [`upload`], through `systemd_run` where one may be asked.
fn uploading(nix: &str, systemd_run: Option<&str>) -> Command {
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
    command.env("NIX_SSHOPTS", copy_sshopts());
    command
}

/// What a copy up tells its ssh before anything `NIX_SSHOPTS` already
/// says, so it wins: give the builder up once nothing at all has come back
/// from it for 15 s. A builder taking what it's sent says so as it goes,
/// ssh's window opening as its side reads, so one nothing reaches is cut.
/// ssh's keepalive waits in the stream behind what it already wrote, up to
/// its window of about 2 MB, so a copy moving faster than about 1 Mbit/s
/// (2 MB in 15 s) is never cut, and a slower one is cut and sent again
/// like one nothing reaches. On 13 November copies up
/// to both lent builders stalled behind a full uplink and waited out the
/// 60 s of keepalives the builder's ssh entry gives, while ssh's window
/// and the relay's buffers held what Nix said it had sent.
pub const COPY_ALIVE: &str = "-o ServerAliveInterval=5 -o ServerAliveCountMax=3";

/// [`COPY_ALIVE`] and then what `NIX_SSHOPTS` says here.
fn copy_sshopts() -> String {
    match std::env::var("NIX_SSHOPTS") {
        Ok(given) if !given.trim().is_empty() => format!("{COPY_ALIVE} {given}"),
        _ => COPY_ALIVE.to_owned(),
    }
}

/// What Nix, or the ssh under it, says when ssh lost the builder midway
/// through a copy, rather than the copy failing on its own account.
const LOST_SAID: [&str; 5] = [
    "not responding",
    "Nix daemon disconnected",
    "cannot open connection to remote store",
    "Connection closed",
    "Connection reset",
];

/// Why a copy up failed, and whether ssh lost the builder under it, which
/// a builder standing again, or the one beside, may take instead.
struct Uncopied {
    why: String,
    lost: bool,
}

impl Uncopied {
    fn failed(why: String) -> Self {
        Self { why, lost: false }
    }

    /// `why`, or why and that ssh lost the builder, as `copying` heard.
    fn of(why: String, copying: &Copying) -> Self {
        if copying.lost() {
            Self {
                why: format!("{why}, since ssh lost it"),
                lost: true,
            }
        } else {
            Self::failed(why)
        }
    }
}

/// A copy running, whose standard error is passed on as it comes, heard
/// for whether it says ssh lost the builder.
struct Copying {
    child: std::process::Child,
    heard: std::sync::mpsc::Receiver<bool>,
}

impl Copying {
    fn start(command: &mut Command) -> std::io::Result<Self> {
        use std::io::BufRead;
        let mut child = command.stderr(Stdio::piped()).spawn()?;
        let (said, heard) = std::sync::mpsc::channel();
        if let Some(errors) = child.stderr.take() {
            std::thread::spawn(move || {
                let mut lost = false;
                for line in std::io::BufReader::new(errors)
                    .split(b'\n')
                    .map_while(Result::ok)
                {
                    let line = String::from_utf8_lossy(&line);
                    lost |= LOST_SAID.iter().any(|said| line.contains(said));
                    eprintln!("{line}");
                }
                let _ = said.send(lost);
            });
        }
        Ok(Self { child, heard })
    }

    /// Whether it said ssh lost the builder, once it has ended.
    fn lost(&self) -> bool {
        self.heard
            .recv_timeout(std::time::Duration::from_secs(1))
            .unwrap_or(false)
    }
}

/// Whether `lent` takes the store path `path` within `within`, copied up
/// as every copy is but alone and quiet, for a proof: nothing fetches it
/// elsewhere, since no cache holds it.
pub fn carried_up(path: &str, lent: &Builder, nix: &str, within: std::time::Duration) -> bool {
    // ssh's control socket goes in TMPDIR, as when what's built comes back.
    uploading(nix, None)
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
pub fn paths_up(
    sent: &[String],
    lent: &Builder,
    nix: &str,
    within: std::time::Duration,
) -> Result<(), String> {
    sent_up(sent, lent, nix, within).map_err(|uncopied| uncopied.why)
}

/// [`paths_up`], saying whether ssh lost the builder when it fails.
fn sent_up(
    sent: &[String],
    lent: &Builder,
    nix: &str,
    within: std::time::Duration,
) -> Result<(), Uncopied> {
    if sent.is_empty() {
        return Ok(());
    }
    // ssh's control socket goes in TMPDIR, as when what's built comes back.
    let mut copying = Copying::start(
        upload(nix)
            .args(["copy", "--to", &lent.store, "--substitute-on-destination"])
            .args(sent)
            .env("TMPDIR", "/tmp"),
    )
    .map_err(|error| Uncopied::failed(format!("couldn't run {nix}: {error}")))?;
    match finished_within(&mut copying.child, within) {
        Some(status) if status.success() => Ok(()),
        _ => Err(Uncopied::of(
            format!("{} couldn't be sent to the builder", sent.join(", ")),
            &copying,
        )),
    }
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
) -> Result<(), Uncopied> {
    let paths: Vec<&str> = asked
        .attributes
        .iter()
        .map(|path| path.split('^').next().unwrap_or(path))
        .collect();
    derivations_up(&paths, lent, scratch, rank(asked), [ssh, nix])?;
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
) -> Result<(), Uncopied> {
    let mut copying = upload(nix);
    match closure(paths, nix).and_then(|closure| lacking(lent, &closure, scratch, ssh)) {
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
            copying
                .args(["copy", "--no-recursive", "--substitute-on-destination"])
                .args(["--to", &lent.store])
                .args(lacking)
        }
        None => {
            eprintln!(
                "nix-offload: copying the derivations up to the lent builder {} with all \
                 they're built from, since what it lacks is unknown",
                lent.host
            );
            copying
                .args(["copy", "--derivation", "--substitute-on-destination"])
                .args(["--to", &lent.store])
                .args(paths)
        }
    };
    let _turn = uplink_turn(rank);
    // ssh's control socket goes in TMPDIR, as when what's built comes back.
    let mut copied = Copying::start(copying.env("TMPDIR", "/tmp"))
        .map_err(|error| Uncopied::failed(format!("couldn't run {nix}: {error}")))?;
    match copied.child.wait() {
        Ok(status) if status.success() => Ok(()),
        _ => Err(Uncopied::of(
            "the derivations couldn't be copied up to the builder".to_owned(),
            &copied,
        )),
    }
}

/// How many copies go up from this machine at once. On 10 November 30
/// runs sent at once shared an uplink of 11 Mbit/s, and one run's 1.1 GB
/// took 77 minutes. A few at once keep the link full, and the rest wait
/// their turn and then go at its whole speed, rather than all crawling.
const UPLOADS: u32 = 3;

/// A turn at sending up from this machine, had while what this gives is
/// held, so one that dies frees it. A run sent first takes none, as the
/// net's go ahead of sessions'. When none can be had, the copy goes on
/// without one.
fn uplink_turn(rank: Rank) -> Option<fs::File> {
    if rank == Rank::First {
        return None;
    }
    let turns = std::env::var_os("XDG_RUNTIME_DIR")
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join("nix-offload-there");
    fs::create_dir_all(&turns).ok()?;
    let open = |turn: u32| {
        fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(turns.join(format!("uplink-{turn}")))
            .ok()
    };
    for turn in 1..=UPLOADS {
        let held = open(turn)?;
        if held.try_lock().is_ok() {
            return Some(held);
        }
    }
    eprintln!(
        "nix-offload: {UPLOADS} copies already go up from this machine, so this waits its turn \
         to send"
    );
    let held = open(std::process::id() % UPLOADS + 1)?;
    held.lock().ok()?;
    Some(held)
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
    let _turn = if sending.is_empty() {
        None
    } else {
        uplink_turn(rank(asked))
    };
    while !sending.is_empty()
        && !sent_held(
            &sending,
            &planned,
            &fixed,
            &holds,
            lent,
            scratch,
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
/// send failed or fell behind [`send_within`].
fn sent_held(
    sending: &[String],
    planned: &BTreeSet<String>,
    fixed: &std::collections::BTreeMap<String, String>,
    holds: &std::collections::BTreeMap<String, u64>,
    lent: &Builder,
    scratch: &Path,
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
    let sent = upload(nix)
        .args(["copy", "--to", &lent.store, "--substitute-on-destination"])
        .args(&sending)
        .env("TMPDIR", "/tmp")
        .spawn()
        .map(|mut send| finished_within(&mut send, within));
    match sent {
        Ok(Some(sent)) if sent.success() => return true,
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
    std::env::var("NIX_OFFLOAD_SEND_WITHIN")
        .ok()
        .and_then(|seconds| seconds.parse().ok())
        .map_or_else(
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
/// caller's alternates.
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
/// reach it.
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
        (None, Sends::Changes) => committed(&top, scratch).map_err(not)?,
        (None, Sends::Committed) => head_alone(&top).map_err(not)?,
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
const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

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
/// diff that finds changes reads no attribute and no submodule, and a tree
/// with alternates, whose objects could be any repository's, is refused.
fn head_alone(top: &Path) -> Result<String, &'static str> {
    let borrows = ["objects/info/alternates", "objects/info/http-alternates"]
        .iter()
        .any(|one| {
            git(top, &["rev-parse", "--git-path", one], None)
                .is_none_or(|path| top.join(path).exists())
        });
    if borrows {
        return Err("borrows objects from another repository, so --run sends nothing of it");
    }
    let head =
        git(top, &["rev-parse", "--verify", "HEAD^{commit}"], None).ok_or("has no commit")?;
    git_with(
        top,
        &[
            "-c",
            "diff.ignoreSubmodules=all",
            "diff",
            "--quiet",
            "--no-ext-diff",
            "--no-textconv",
            "HEAD",
        ],
        None,
        &[("GIT_ATTR_SOURCE", EMPTY_TREE)],
    )
    .map(|_| head)
    .ok_or("has changes not committed, and --run sends only what is, so commit them first")
}

/// The commit the git tree `top` is built at: its HEAD, or, with changes to
/// the files its index holds, staged new ones included, a commit of them
/// made without touching the branch or the index, as `nix build` of the
/// tree here would read them.
fn committed(top: &Path, scratch: &Path) -> Result<String, &'static str> {
    if lent_only() {
        return head_alone(top);
    }
    let head = git(top, &["rev-parse", "HEAD"], None).ok_or("has no commit")?;
    if git(top, &["diff", "--quiet", "HEAD"], None).is_some() {
        Ok(head)
    } else {
        fs::create_dir_all(scratch).map_err(|_| "has no scratch to commit its changes in")?;
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
        made.ok_or("has changes git couldn't commit")
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
/// which would leave nothing to run that trap.
///
/// Ending at once lost work: on 3 November a lender's restart cut a VM
/// test mid-compile, and the run sent again built linux from the start.
/// So each caller claims the run in a folder named for the command, which
/// is the same each time one run sends it and differs between runs, since
/// it names this machine's pid. The watcher ends the group only once no
/// claimer has lived for the grace period. The run sent again waits on
/// what the first is building through Nix's lock on each derivation.
/// Once the command itself has ended, the watcher ends too, as there's
/// nothing left to wait for.
pub fn far(command: &str) -> String {
    far_within(command, &format!("${{NIX_OFFLOAD_GRACE:-{GRACE}}}"), false)
}

/// `command` as `far` sends it, ending once its caller has been gone 15
/// seconds, or `NIX_OFFLOAD_RUN_GRACE`'s: a command run with `--run` builds
/// nothing another run could wait on, so there's nothing to keep it for,
/// and a suite left running holds a builder others wait for.
pub fn far_brief(command: &str) -> String {
    far_within(command, "${NIX_OFFLOAD_RUN_GRACE:-15}", false)
}

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
        true,
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
/// seconds, as the far shell reads them. One `outlives` names itself for
/// `OUTLIVING_ASKED` while it runs.
fn far_within(command: &str, grace: &str, outlives: bool) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::hash::DefaultHasher::new();
    command.hash(&mut hasher);
    let claims = format!("${{TMPDIR:-/tmp}}/nix-offload-run-{:016x}", hasher.finish());
    let (named, unnamed) = if outlives {
        (
            "outliving=\"${TMPDIR:-/tmp}/nix-offload-outliving-$$\"; : > \"$outliving\"; ",
            "rm -f \"$outliving\"; ",
        )
    } else {
        ("", "")
    };
    format!(
        "{named}claims=\"{claims}\"; mkdir -p \"$claims\" && : > \"$claims/$PPID\"; \
         (gone=0; while [ \"$gone\" -lt \"{grace}\" ]; do sleep 1; \
         kill -0 $$ 2>/dev/null || break; \
         held=; for caller in $(ls \"$claims\" 2>/dev/null); do \
         if kill -0 \"$caller\" 2>/dev/null; then held=1; else rm -f \"$claims/$caller\"; fi; \
         done; if [ -n \"$held\" ]; then gone=0; else gone=$((gone + 1)); fi; done; \
         {unnamed}rmdir \"$claims\" 2>/dev/null; kill -TERM 0) \
         </dev/null >/dev/null 2>&1 & \
         {ROOM_SAID} watching=$!; \
         trap 'kill $watching 2>/dev/null; exit 143' HUP TERM; {command}; \
         status=$?; kill $watching 2>/dev/null; exit $status"
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
/// connection.
pub fn reaching(ssh: &str, said: &Path) -> Command {
    let mut command = Command::new(ssh);
    command
        .args(["-o", "BatchMode=yes", "-o", "LogLevel=VERBOSE"])
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
fn sized(bytes: u64) -> String {
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
    on_with(lent, sending, bring, scratch, ssh, far)
}

/// Builds as `on` does, with the command sent as `wrap` makes it: `far`,
/// or `outliving` for a run asked to outlive its caller.
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
        .arg(far(&sending.command))
        .stdout(Stdio::piped());
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
    // A builder with a key of its own is never reached through the agent:
    // ssh asks the agent first even then, and on 8 November an agent
    // waiting on its person stalled every lender's ssh at login for 18
    // minutes, so each builder failed its proofs and was given back.
    if let Some(key) = &lent.key {
        command.args([
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "IdentityAgent=none",
            "-i",
            key,
        ]);
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

/// Waits for a builder when none stands, rather than building here: it
/// says it wants one by a file in `dir`'s `wanted` named for its pid,
/// which the lender counts as queued work, for as long as it waits. It
/// waits only while the lender whose pid `dir`'s `lender` holds runs, and
/// gives up once that lender hasn't begun making one (`lending` is empty)
/// within `answered`, or none stands within `longest`. A lender that
/// stopped, its pid left there, is waited for up to `restarting` first,
/// since a switch starts one again within seconds, and one superseded
/// ends just as a build that would have used its builder asks. It gives
/// the store's text once one stands.
pub fn waited(dir: &Path, waiting: Waiting) -> Option<String> {
    if !waiting.asks {
        eprintln!("nix-offload: no builder stands, and --never-ask leaves the lender unasked");
        return None;
    }
    if !lender_runs(dir, waiting) {
        return None;
    }
    let wanted = dir.join("wanted");
    let want = wanted.join(std::process::id().to_string());
    fs::create_dir_all(&wanted).ok()?;
    fs::write(&want, "").ok()?;
    eprintln!(
        "nix-offload: no builder stands, so this asks for one and waits for it, for up to {} \
         minutes",
        waiting.longest.as_secs().div_ceil(60)
    );
    let began = std::time::Instant::now();
    let read = |name: &str| {
        fs::read_to_string(dir.join(name))
            .ok()
            .filter(|text| !text.trim().is_empty())
    };
    let stood = loop {
        if let Some(store) = read("store") {
            break Some(store);
        }
        let so_far = began.elapsed();
        if so_far >= waiting.longest || (so_far >= waiting.answered && read("lending").is_none()) {
            break None;
        }
        std::thread::sleep(std::time::Duration::from_millis(250).min(waiting.answered));
    };
    let _ = fs::remove_file(&want);
    stood
}

/// Whether `dir`'s lender runs, waiting up to `waiting.restarting` for one
/// that stopped, its pid left behind, to start again; with none ever
/// started here, it says so at once.
fn lender_runs(dir: &Path, waiting: Waiting) -> bool {
    let lender = dir.join("lender");
    if pid_in(&lender).is_some_and(runs) {
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
        if pid_in(&lender).is_some_and(runs) {
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
        if pid_in(&dir.join("lender")).is_some_and(runs) {
            return fs::read_to_string(dir.join("store"))
                .ok()
                .filter(|store| !store.trim().is_empty())
                .or_else(|| waited(dir, waiting));
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

/// Whether a build in `dir`'s `wanted` is waiting for a builder: a file
/// there is named for a pid that still runs. One whose build ended
/// without taking it away is taken away here.
pub fn wanted(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(dir.join("wanted")) else {
        return false;
    };
    let mut waiting = false;
    for entry in entries.filter_map(Result::ok) {
        let pid = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse().ok());
        if pid.is_some_and(runs) {
            waiting = true;
        } else {
            let _ = fs::remove_file(entry.path());
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
    answers: impl Fn(&str) -> bool,
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
        .filter(|machine| answers(&machine.address))
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
/// 51 minutes on the first while the second sat idle.
///
/// A `bulk` run lets every run that isn't go before it, those waiting
/// and those coming while it waits, and they wait for no bulk run: the
/// after-merge net sends dozens of VM tests a round, and on 5 November a
/// session's few waited behind the whole round. A run sent first goes
/// ahead of every run that isn't, bulk or not.
pub fn admitted<'a>(
    dir: &'a Path,
    others: &[&'a Path],
    count: usize,
    room: usize,
    rank: Rank,
) -> (&'a Path, Option<Claim>) {
    use std::os::fd::AsRawFd;
    let wants = count.clamp(1, room.max(1));
    let mut said = false;
    let mut place: Option<Claim> = None;
    loop {
        let Ok(lock) = fs::File::create(dir.join("sending.lock")) else {
            return (dir, claim(dir, wants));
        };
        // SAFETY: flock only locks the file it's given, open until dropped.
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX) } != 0 {
            return (dir, claim(dir, wants));
        }
        let busy = claimed(dir);
        let before = ahead(dir, place.as_ref(), rank, busy, room);
        if before == 0 && (busy == 0 || busy + wants <= room) {
            // Claimed, and its place given up, before the lock is let go,
            // as dropping it does.
            let claimed = claim(dir, wants);
            drop(place);
            return (dir, claimed);
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
            .filter(|other| !here_warm || warm(other))
            .find_map(|other| Some((*other, free(other, wants, room, rank)?)))
        {
            eprintln!(
                "nix-offload: another VM builder has room now and nothing waiting, so this goes \
                 there"
            );
            drop(place);
            return (other, Some(claimed));
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
    pid_in(&dir.join("lender")).is_some_and(runs)
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
    let stands = fs::read_to_string(dir.join("store")).is_ok_and(|store| !store.trim().is_empty());
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
    let wanted = dir.join("wanted");
    if fs::create_dir_all(&wanted).is_ok() {
        let _ = fs::write(wanted.join(std::process::id().to_string()), "");
    }
}

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
    reached.arg(&machine.host).arg(far(&command));
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
    if !packing.success() {
        return Err(format!("{tar} couldn't pack {}", folder.display()));
    }
    Ok(status.code().unwrap_or(UNREACHED))
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
    let copied = upload(nix)
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
fn decoded(encoded: &str) -> Option<String> {
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
    fn an_upload_runs_in_a_scope_of_its_own_where_systemd_can_make_one() {
        let args = |command: &Command| -> Vec<String> {
            command
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect()
        };
        let scoped = uploading("/bin/nix", Some("/bin/systemd-run"));
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
        let alone = uploading("/bin/nix", None);
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

    /// What a run sent far says while it runs for `seconds`, and how long
    /// it took, on a builder whose `ps` names `uids` as running and whose
    /// Nix runs `most` builds at once, asked about every second.
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
             && tar -czf - -C \"$out\" ."
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
            !dir.path().join("wanted").exists(),
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
            !dir.path().join("wanted").exists(),
            "the lender is never told one is wanted"
        );
    }

    #[test]
    fn with_no_lender_running_nothing_is_waited_for() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(waited(dir.path(), QUICK), None);
        assert!(!dir.path().join("wanted").exists());
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

    #[test]
    fn a_want_whose_build_ended_is_taken_away() {
        let dir = tempfile::tempdir().unwrap();
        let wanting = dir.path().join("wanted");
        fs::create_dir(&wanting).unwrap();
        fs::write(wanting.join("999999999"), "").unwrap();
        assert!(!wanted(dir.path()));
        assert!(!wanting.join("999999999").exists());
        fs::write(wanting.join(std::process::id().to_string()), "").unwrap();
        assert!(wanted(dir.path()));
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

    #[test]
    fn a_builder_with_a_key_of_its_own_is_reached_without_the_agent() {
        let scratch = tempfile::tempdir().unwrap();
        let lent = builder("ssh-ng://nix-offload?ssh-key=/k").unwrap();
        let (command, _, _) = reach(&lent, scratch.path(), "ssh").unwrap();
        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_string_lossy())
            .collect();
        assert!(
            args.windows(2)
                .any(|pair| pair == ["-o", "IdentityAgent=none"]),
            "{args:?}"
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
        let answers = |_: &str| {
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
