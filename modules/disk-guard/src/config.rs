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
    fn pressure() -> Size {
        Size::Percent(15.0)
    }
    fn urgent() -> Size {
        Size::Percent(8.0)
    }
    fn critical() -> Size {
        Size::Percent(3.0)
    }
    fn target() -> Size {
        Size::Percent(20.0)
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
    /// Quest workspaces, judged by their quest's README.
    Workspaces { path: PathBuf, quests: PathBuf },
    /// A temporary directory's per-run leftovers.
    Temp { path: PathBuf },
    /// A desktop trash, expired by its deletion dates.
    Trash { path: PathBuf },
    /// Logs, each kept to its last part once past a limit.
    Logs { path: PathBuf },
    /// podman's own prune commands.
    Podman { command: PathBuf },
    /// A producer's own collection, run from a band or on a schedule.
    Producer {
        command: Vec<String>,
        #[serde(default)]
        every: Option<Span>,
        #[serde(default)]
        from: Option<Band>,
        timeout: Span,
    },
    /// A command that says whether a checkout's work is finished, for
    /// checkouts whose remote matches.
    Finished {
        remote: String,
        command: Vec<String>,
        timeout: Span,
    },
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

impl Source {
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
            | Source::Producer { .. } => true,
            Source::Coredumps { .. } | Source::NixBuilds { .. } => scope == Scope::System,
            _ => scope == Scope::User,
        }
    }

    fn paths(&self) -> Vec<&Path> {
        match self {
            Source::Cargo { roots, .. } | Source::Installs { roots, .. } => {
                roots.iter().map(PathBuf::as_path).collect()
            }
            Source::Workspaces { path, quests } => vec![path, quests],
            Source::Cache { path, .. }
            | Source::Scratch { path, .. }
            | Source::Snapshots { path, .. }
            | Source::Temp { path }
            | Source::Trash { path }
            | Source::Logs { path }
            | Source::Coredumps { path }
            | Source::NixBuilds { path } => vec![path],
            Source::Podman { command } => vec![command],
            Source::Producer { .. } | Source::Finished { .. } => vec![],
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
                ..
            } => {
                check_command(command)?;
                if every.is_none() && from.is_none() {
                    return Err("it says neither when to run (every) nor from which band".into());
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

/// Nix's garbage, kept as a rebuild cache with a budget.
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
        Span(Duration::from_secs(3 * 86400))
    }
    fn budget() -> Size {
        Size::Percent(10.0)
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
        Span(Duration::from_secs(7 * 86400))
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
}

impl Dedupe {
    fn every() -> Span {
        Span(Duration::from_secs(7 * 86400))
    }
}

impl Default for Dedupe {
    fn default() -> Dedupe {
        Dedupe {
            every: Dedupe::every(),
        }
    }
}

/// What only the system daemon does.
#[derive(Clone, Debug, PartialEq)]
pub struct System {
    pub garbage: Garbage,
    pub generations: Generations,
    pub dedupe: Dedupe,
    /// Where the boot partition is mounted, whose entries keep generations.
    pub boot: PathBuf,
    /// Where the machine's Nix profiles live.
    pub profiles: PathBuf,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub scope: Scope,
    pub bands: Bands,
    pub notify: bool,
    pub status_file: bool,
    /// Whether a runaway writer may be frozen at critical.
    pub freeze: bool,
    pub sources: BTreeMap<String, Source>,
    pub system: Option<System>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rendered {
    version: u32,
    scope: Scope,
    #[serde(default)]
    bands: Bands,
    #[serde(default = "yes")]
    notify: bool,
    #[serde(default)]
    status_file: bool,
    #[serde(default = "yes")]
    freeze: bool,
    sources: BTreeMap<String, Source>,
    garbage: Option<Garbage>,
    generations: Option<Generations>,
    dedupe: Option<Dedupe>,
    boot: Option<PathBuf>,
    profiles: Option<PathBuf>,
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
                    ("generations", rendered.generations.is_some()),
                    ("dedupe", rendered.dedupe.is_some()),
                    ("boot", rendered.boot.is_some()),
                    ("profiles", rendered.profiles.is_some()),
                ];
                if let Some((section, _)) = sections.iter().find(|(_, set)| *set) {
                    return Err(format!("{section} belongs to the system daemon"));
                }
                None
            }
            Scope::System => {
                let boot = rendered.boot.ok_or("the system daemon needs boot")?;
                let profiles = rendered
                    .profiles
                    .unwrap_or_else(|| PathBuf::from("/nix/var/nix/profiles"));
                for path in [&boot, &profiles] {
                    if !plain_absolute(path) {
                        return Err(format!("{} is not a plain absolute path", path.display()));
                    }
                }
                Some(System {
                    garbage: rendered.garbage.unwrap_or_default(),
                    generations: rendered.generations.unwrap_or_default(),
                    dedupe: rendered.dedupe.unwrap_or_default(),
                    boot,
                    profiles,
                })
            }
        };
        Ok(Config {
            scope: rendered.scope,
            bands: rendered.bands,
            notify: rendered.notify,
            status_file: rendered.status_file,
            freeze: rendered.freeze,
            sources: rendered.sources,
            system,
        })
    }
}
