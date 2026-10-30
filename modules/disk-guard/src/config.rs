//! What Nix tells the guard: its scope, its bands and its sources.
//!
//! Nix renders one JSON file per daemon, and the build runs `check-config`
//! over it, so a field this doesn't know, a path that isn't absolute or a
//! source the scope doesn't own fails the build rather than the daemon.

use crate::band::Band;
use crate::units::{Size, Span};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    User,
    System,
}

/// Where each band starts, as free space under which it applies, and the free
/// space a pass reclaims towards.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bands {
    #[serde(default = "Bands::pressure")]
    pub pressure: Size,
    #[serde(default = "Bands::urgent")]
    pub urgent: Size,
    #[serde(default = "Bands::critical")]
    pub critical: Size,
    #[serde(default = "Bands::target")]
    pub target: Size,
}

impl Bands {
    // Routine reclaiming starts at a quarter free and reaches for over a
    // third, so a workstation keeps room for its next builds, and the
    // urgent bands begin well before the disk is full.
    fn pressure() -> Size {
        Size::Percent(25.0)
    }
    fn urgent() -> Size {
        Size::Percent(10.0)
    }
    fn critical() -> Size {
        Size::Percent(4.0)
    }
    /// Refuses bands out of order where their units let it show: each
    /// band under the one before, and the target over pressure. Bytes
    /// against a share are ordered on each filesystem, where the band
    /// thresholds fold them.
    fn check(&self) -> Result<(), String> {
        // Whether `a` is surely more than `b`.
        let over = |a: Size, b: Size| match (a, b) {
            (Size::Bytes(a), Size::Bytes(b)) => a > b,
            (Size::Percent(a), Size::Percent(b)) => a > b,
            _ => false,
        };
        let written = |size: Size| match size {
            Size::Bytes(bytes) => bytes.to_string(),
            Size::Percent(share) => format!("{share}%"),
        };
        if over(self.urgent, self.pressure) {
            return Err(format!(
                "urgent {} is over pressure {}",
                written(self.urgent),
                written(self.pressure)
            ));
        }
        if over(self.critical, self.urgent) {
            return Err(format!(
                "critical {} is over urgent {}",
                written(self.critical),
                written(self.urgent)
            ));
        }
        if over(self.pressure, self.target) {
            return Err(format!(
                "target {} is under pressure {}",
                written(self.target),
                written(self.pressure)
            ));
        }
        Ok(())
    }

    fn target() -> Size {
        Size::Percent(35.0)
    }
}

/// How a pass that can wait paces its deletes: a rest after every `files`
/// unlinks of at least `rest`. The default rest outlasts Btrfs's commit
/// interval of thirty seconds, so each batch goes in a commit of its own.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pacing {
    #[serde(default = "Pacing::files")]
    pub files: usize,
    #[serde(default = "Pacing::rest")]
    pub rest: Span,
}

impl Pacing {
    fn files() -> usize {
        2000
    }

    fn rest() -> Span {
        Span(Duration::from_secs(30))
    }
}

impl Default for Pacing {
    fn default() -> Pacing {
        Pacing {
            files: Pacing::files(),
            rest: Pacing::rest(),
        }
    }
}

impl Default for Bands {
    fn default() -> Bands {
        Bands {
            pressure: Bands::pressure(),
            urgent: Bands::urgent(),
            critical: Bands::critical(),
            target: Bands::target(),
        }
    }
}

/// How a cache is trimmed short of emptying it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CacheProtocol {
    /// Go's build cache, whose entries' modification times say when each
    /// was last used, so the oldest go first, as Go's own trim takes them.
    GoBuild,
}

/// Something a source names as regenerable, and how to judge it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Source {
    /// Roots holding checkouts and the cargo targets built from them.
    Cargo {
        roots: Vec<PathBuf>,
        #[serde(default)]
        depth: Option<u32>,
    },
    /// Roots holding `node_modules` and `.venv` installs.
    Installs {
        roots: Vec<PathBuf>,
        #[serde(default)]
        depth: Option<u32>,
    },
    /// A directory that refills on demand, emptied from a band on.
    Cache {
        path: PathBuf,
        #[serde(default = "urgent")]
        from: Band,
        #[serde(default)]
        protocol: Option<CacheProtocol>,
        /// Where it stands among caches, cheapest to refill first.
        #[serde(default = "middle_rank")]
        rank: u8,
    },
    /// Entries that go once nothing has written to them for the band's
    /// window, while healthy only with a window of their own.
    Scratch {
        path: PathBuf,
        #[serde(default)]
        healthy: Option<Span>,
    },
    /// Entries named for the process that made them, kept while it lives.
    Snapshots {
        path: PathBuf,
        pattern: String,
        command: String,
    },
    /// Quest workspaces, judged by their quest's README. `trees`, when
    /// given, is a tree broker's records, one JSON file per tree naming
    /// its `path` and the `owners` holding it: a tree none of them still
    /// holds, and no quest going lists, has its builds spent. `sessions`,
    /// when given, is the harness's record of each coding session and the
    /// quest it works on: a quest none of them still works on has its
    /// trees' builds spent.
    Workspaces {
        path: PathBuf,
        quests: PathBuf,
        #[serde(default)]
        trees: Option<PathBuf>,
        #[serde(default)]
        sessions: Option<PathBuf>,
    },
    /// A temporary directory's per-run leftovers: the one named, or the
    /// account's own, which on a Mac is a folder of its own under
    /// /var/folders rather than /tmp.
    Temp {
        #[serde(default = "account_temp")]
        path: PathBuf,
    },
    /// A desktop trash, expired by its deletion dates.
    Trash { path: PathBuf },
    /// Logs, each kept to its last part once past a limit.
    Logs { path: PathBuf },
    /// podman's own prune commands, for the storage at `path`.
    Podman { command: PathBuf, path: PathBuf },
    /// A producer's own collection, run from a band or on a schedule.
    Producer {
        command: Vec<String>,
        /// Where it frees space, whose filesystem's passes run it.
        path: PathBuf,
        #[serde(default)]
        every: Option<Span>,
        #[serde(default)]
        from: Option<Band>,
        /// Files or folders whose saying something new since it last ran
        /// makes it due in the next upkeep, looked at once a minute: what
        /// its own rule would let go is spent once what it watches says
        /// something new arrived, not when a schedule comes round. What
        /// they say is a link's target, a file's length and start, and a
        /// folder's names three levels down, so rewriting one unchanged is
        /// no news.
        #[serde(default)]
        watch: Vec<PathBuf>,
        timeout: Span,
    },
    /// A command that says whether a checkout's work is finished, for
    /// checkouts whose remote matches.
    Finished {
        remote: String,
        command: Vec<String>,
        timeout: Span,
    },
    /// Git repositories, by their git directories, whose temporaries go
    /// once no git works on them.
    Git { repos: Vec<PathBuf> },
    /// systemd-coredump's cores.
    Coredumps { path: PathBuf },
    /// Build directories Nix left behind.
    NixBuilds { path: PathBuf },
}

fn urgent() -> Band {
    Band::Urgent
}

fn middle_rank() -> u8 {
    50
}

/// The running account's own temporary folder: on a Mac the one the system
/// keeps for it, whatever TMPDIR says, so a plan read under a harness names
/// the folder the daemon sweeps; elsewhere TMPDIR's, or /tmp.
#[must_use]
pub fn account_temp() -> PathBuf {
    #[cfg(target_os = "macos")]
    if let Some(folder) = crate::macos::account_temp() {
        return folder;
    }
    std::env::temp_dir()
}

impl Source {
    /// The paths it lies under, which say what filesystems it's on and
    /// what folders it can offer from.
    pub fn under(&self) -> Vec<&Path> {
        match self {
            Source::Cargo { roots, .. }
            | Source::Installs { roots, .. }
            | Source::Git { repos: roots } => roots.iter().map(PathBuf::as_path).collect(),
            Source::Cache { path, .. }
            | Source::Scratch { path, .. }
            | Source::Snapshots { path, .. }
            | Source::Workspaces { path, .. }
            | Source::Temp { path }
            | Source::Trash { path }
            | Source::Logs { path }
            | Source::Coredumps { path }
            | Source::NixBuilds { path }
            | Source::Producer { path, .. }
            | Source::Podman { path, .. } => vec![path.as_path()],
            Source::Finished { .. } => Vec::new(),
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Source::Cargo { .. } => "cargo",
            Source::Installs { .. } => "installs",
            Source::Cache { .. } => "cache",
            Source::Scratch { .. } => "scratch",
            Source::Snapshots { .. } => "snapshots",
            Source::Workspaces { .. } => "workspaces",
            Source::Temp { .. } => "temp",
            Source::Trash { .. } => "trash",
            Source::Logs { .. } => "logs",
            Source::Podman { .. } => "podman",
            Source::Producer { .. } => "producer",
            Source::Finished { .. } => "finished",
            Source::Git { .. } => "git",
            Source::Coredumps { .. } => "coredumps",
            Source::NixBuilds { .. } => "nix-builds",
        }
    }

    /// Whether a daemon of this scope may act on the source. Root never
    /// deletes inside a home, so the system daemon owns only what lies
    /// outside one, and an account's daemon only what its account owns.
    fn owned_by(&self, scope: Scope) -> bool {
        match self {
            Source::Cache { .. }
            | Source::Temp { .. }
            | Source::Logs { .. }
            | Source::Podman { .. }
            | Source::Producer { .. } => true,
            Source::Coredumps { .. } | Source::NixBuilds { .. } => scope == Scope::System,
            _ => scope == Scope::User,
        }
    }

    fn paths(&self) -> Vec<&Path> {
        match self {
            Source::Cargo { roots, .. }
            | Source::Installs { roots, .. }
            | Source::Git { repos: roots } => roots.iter().map(PathBuf::as_path).collect(),
            Source::Workspaces {
                path,
                quests,
                trees,
                sessions,
            } => [path, quests]
                .into_iter()
                .chain(trees)
                .chain(sessions)
                .map(PathBuf::as_path)
                .collect(),
            Source::Cache { path, .. }
            | Source::Scratch { path, .. }
            | Source::Snapshots { path, .. }
            | Source::Temp { path }
            | Source::Trash { path }
            | Source::Logs { path }
            | Source::Coredumps { path }
            | Source::NixBuilds { path }
            | Source::Producer { path, .. } => vec![path],
            Source::Podman { command, path } => vec![command, path],
            Source::Finished { .. } => vec![],
        }
    }

    fn check(&self) -> Result<(), String> {
        for path in self.paths() {
            if !plain_absolute(path) {
                return Err(format!(
                    "{} is not an absolute path without . or ..",
                    path.display()
                ));
            }
        }
        match self {
            Source::Snapshots { pattern, .. } => {
                let pattern = regex_lite::Regex::new(pattern)
                    .map_err(|error| format!("its pattern doesn't compile: {error}"))?;
                if !pattern.capture_names().any(|name| name == Some("pid")) {
                    return Err("its pattern has no pid group".into());
                }
            }
            Source::Finished {
                remote, command, ..
            } => {
                regex_lite::Regex::new(remote)
                    .map_err(|error| format!("its remote doesn't compile: {error}"))?;
                check_command(command)?;
            }
            Source::Producer {
                command,
                every,
                from,
                watch,
                ..
            } => {
                check_command(command)?;
                if every.is_none() && from.is_none() {
                    return Err("it says neither when to run (every) nor from which band".into());
                }
                if let Some(path) = watch.iter().find(|path| !plain_absolute(path)) {
                    return Err(format!(
                        "it watches {}, which is not an absolute path",
                        path.display()
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// A command runs by absolute path, so what runs is what Nix rendered.
fn check_command(command: &[String]) -> Result<(), String> {
    match command.first() {
        None => Err("its command is empty".into()),
        Some(program) if !plain_absolute(Path::new(program)) => {
            Err(format!("its command {program:?} is not an absolute path"))
        }
        Some(_) => Ok(()),
    }
}

fn plain_absolute(path: &Path) -> bool {
    path.is_absolute()
        && path
            .components()
            .all(|part| matches!(part, Component::RootDir | Component::Normal(_)))
}

/// Nix's garbage, which a machine may keep as a rebuild cache with a
/// budget. Kept for three days, up to a tenth of the disk, it held 34 GiB
/// of the 43 nothing held on a laptop whose owner expected the disk far
/// emptier. Kept not at all, a stick writer's payload went ten minutes
/// after it was built, while the program that built it was still writing
/// it. So by default what was built in the last hour stays while the disk
/// is healthy, up to a twentieth of it, and everything older goes.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Garbage {
    /// Garbage younger than this stays while healthy, within the budget.
    #[serde(default = "Garbage::keep")]
    pub keep: Span,
    /// How much young garbage may stay before the largest goes.
    #[serde(default = "Garbage::budget")]
    pub budget: Size,
}

impl Garbage {
    fn keep() -> Span {
        Span(Duration::from_hours(1))
    }
    fn budget() -> Size {
        Size::Percent(5.0)
    }
}

impl Default for Garbage {
    fn default() -> Garbage {
        Garbage {
            keep: Garbage::keep(),
            budget: Garbage::budget(),
        }
    }
}

/// Where Nix is, for either daemon: what collects garbage and expires
/// generations, the store that frees, and the profiles whose generations
/// this daemon looks after.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Nix {
    /// The `nix` that collects; `nix-store` and `nix-env` are beside it.
    #[serde(default = "Nix::bin")]
    pub bin: PathBuf,
    /// Where the store lies, whose filesystem collecting frees.
    #[serde(default = "Nix::store")]
    pub store: PathBuf,
    /// Where Nix keeps the temporary roots of each process using the
    /// store, which hold what a build or an evaluation has in hand.
    #[serde(default = "Nix::temp_roots")]
    pub temp_roots: PathBuf,
    /// Where Nix keeps the roots it was told of, whose `auto` folder links
    /// to each build's result: one going is garbage made, listed at once.
    #[serde(default = "Nix::roots")]
    pub roots: PathBuf,
    /// The directory of profiles whose generations expire: the machine's
    /// for the system daemon unless told, and an account's only if told.
    #[serde(default)]
    pub profiles: Option<PathBuf>,
    #[serde(default)]
    pub generations: Generations,
}

impl Nix {
    fn bin() -> PathBuf {
        PathBuf::from("/run/current-system/sw/bin/nix")
    }
    fn store() -> PathBuf {
        PathBuf::from("/nix/store")
    }
    fn temp_roots() -> PathBuf {
        PathBuf::from("/nix/var/nix/temproots")
    }
    fn roots() -> PathBuf {
        PathBuf::from("/nix/var/nix/gcroots")
    }
}

impl Default for Nix {
    fn default() -> Nix {
        Nix {
            bin: Nix::bin(),
            store: Nix::store(),
            temp_roots: Nix::temp_roots(),
            roots: Nix::roots(),
            profiles: None,
            generations: Generations::default(),
        }
    }
}

/// Which of a profile's generations stay.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Generations {
    /// The newest this many always stay.
    #[serde(default = "Generations::keep")]
    pub keep: u32,
    /// Generations younger than this stay while healthy.
    #[serde(default = "Generations::young")]
    pub young: Span,
}

impl Generations {
    fn keep() -> u32 {
        5
    }
    fn young() -> Span {
        Span(Duration::from_hours(7 * 24))
    }
}

impl Default for Generations {
    fn default() -> Generations {
        Generations {
            keep: Generations::keep(),
            young: Generations::young(),
        }
    }
}

/// How often the store's identical files are linked together.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dedupe {
    #[serde(default = "Dedupe::every")]
    pub every: Span,
    /// What confines a run to a scope of its own.
    #[serde(default = "Dedupe::systemd_run")]
    pub systemd_run: Option<PathBuf>,
}

impl Dedupe {
    fn every() -> Span {
        Span(Duration::from_hours(7 * 24))
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "serde's default for an Option field must return the Option"
    )]
    fn systemd_run() -> Option<PathBuf> {
        Some(PathBuf::from("/run/current-system/systemd/bin/systemd-run"))
    }
}

impl Default for Dedupe {
    fn default() -> Dedupe {
        Dedupe {
            every: Dedupe::every(),
            systemd_run: Dedupe::systemd_run(),
        }
    }
}

/// What only the system daemon does.
#[derive(Clone, Debug, PartialEq)]
pub struct System {
    pub garbage: Garbage,
    pub dedupe: Dedupe,
    /// Where the boot partition is mounted, whose entries keep generations.
    pub boot: PathBuf,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub scope: Scope,
    pub bands: Bands,
    pub pacing: Pacing,
    pub notify: bool,
    pub status_file: bool,
    /// Whether a runaway writer may be frozen at critical.
    pub freeze: bool,
    pub sources: BTreeMap<String, Source>,
    pub nix: Nix,
    pub system: Option<System>,
    /// The space used past which the guard says what holds each disk, as
    /// it does below healthy, though it takes nothing more for it: a disk
    /// can be far from filling and still hold more than anything on it has
    /// reason to, and a budget is a signal and a backstop, never a reason
    /// to take what an owner still holds.
    pub budget: Option<Size>,
    pub holders: Holders,
}

/// Where the guard looks for what holds a disk, beside its sources.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Holders {
    /// Folders whose every folder is a holder, so what no source names,
    /// such as a program's own data, is still sized and named. The guard
    /// clears nothing for being here.
    #[serde(default)]
    pub roots: Vec<PathBuf>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rendered {
    version: u32,
    scope: Scope,
    #[serde(default)]
    bands: Bands,
    #[serde(default)]
    pacing: Pacing,
    #[serde(default = "yes")]
    notify: bool,
    #[serde(default)]
    status_file: bool,
    #[serde(default = "yes")]
    freeze: bool,
    sources: BTreeMap<String, Source>,
    #[serde(default)]
    nix: Nix,
    garbage: Option<Garbage>,
    dedupe: Option<Dedupe>,
    boot: Option<PathBuf>,
    budget: Option<Size>,
    #[serde(default)]
    holders: Holders,
}

fn yes() -> bool {
    true
}

impl Config {
    pub fn parse(text: &str) -> Result<Config, String> {
        let rendered: Rendered = serde_json::from_str(text).map_err(|error| error.to_string())?;
        if rendered.version != 1 {
            return Err(format!(
                "version {} is not one this guard reads",
                rendered.version
            ));
        }
        rendered.bands.check()?;
        if rendered.pacing.files == 0 {
            return Err("pacing needs at least one file a batch".to_owned());
        }
        for (name, source) in &rendered.sources {
            if !source.owned_by(rendered.scope) {
                return Err(format!(
                    "source {name}: a {:?} daemon doesn't own {} sources",
                    rendered.scope,
                    source.kind()
                ));
            }
            source
                .check()
                .map_err(|error| format!("source {name}: {error}"))?;
        }
        let system = match rendered.scope {
            Scope::User => {
                let sections = [
                    ("garbage", rendered.garbage.is_some()),
                    ("dedupe", rendered.dedupe.is_some()),
                    ("boot", rendered.boot.is_some()),
                ];
                if let Some((section, _)) = sections.iter().find(|(_, set)| *set) {
                    return Err(format!("{section} belongs to the system daemon"));
                }
                None
            }
            Scope::System => {
                let boot = rendered.boot.ok_or("the system daemon needs boot")?;
                let dedupe = rendered.dedupe.unwrap_or_default();
                for path in std::iter::once(&boot).chain(&dedupe.systemd_run) {
                    if !plain_absolute(path) {
                        return Err(format!("{} is not a plain absolute path", path.display()));
                    }
                }
                Some(System {
                    garbage: rendered.garbage.unwrap_or_default(),
                    dedupe,
                    boot,
                })
            }
        };
        let mut nix = rendered.nix;
        if rendered.scope == Scope::System && nix.profiles.is_none() {
            nix.profiles = Some(PathBuf::from("/nix/var/nix/profiles"));
        }
        let roots = &rendered.holders.roots;
        for path in [&nix.bin, &nix.store]
            .into_iter()
            .chain(&nix.profiles)
            .chain(roots)
        {
            if !plain_absolute(path) {
                return Err(format!("{} is not a plain absolute path", path.display()));
            }
        }
        Ok(Config {
            scope: rendered.scope,
            bands: rendered.bands,
            pacing: rendered.pacing,
            notify: rendered.notify,
            status_file: rendered.status_file,
            freeze: rendered.freeze,
            sources: rendered.sources,
            nix,
            system,
            budget: rendered.budget,
            holders: rendered.holders,
        })
    }
}
