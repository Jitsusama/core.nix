//! What processes are doing, which decides what's busy and what's occupied.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::ops::Bound;
use std::path::{Path, PathBuf};

/// How a tree is used, each naming the process that decided it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Use {
    Free,
    Occupied(u32),
    Unknown(u32),
    Busy(u32),
}

impl Use {
    /// Which use wins when several processes disagree: anything busy, then
    /// anything the guard couldn't read, then anything sitting idle.
    fn rank(self) -> u8 {
        match self {
            Use::Free => 0,
            Use::Occupied(_) => 1,
            Use::Unknown(_) => 2,
            Use::Busy(_) => 3,
        }
    }
}

/// The programs that write a target, or read it as they build.
const BUILD_TOOLS: &[&str] = &[
    "as",
    "c++",
    "cargo",
    "cargo-clippy",
    "cargo-miri",
    "cargo-nextest",
    "cc",
    "cc1",
    "cc1plus",
    "clang",
    "clang++",
    "clippy-driver",
    "collect2",
    "g++",
    "gcc",
    "ld",
    "ld.bfd",
    "ld.gold",
    "ld.lld",
    "ld.mold",
    "lld",
    "mold",
    "rust-lld",
    "rustc",
    "rustdoc",
    "sccache",
];

/// The variables that point a build at what it writes and reads: its
/// target, and the cargo home its downloads are kept in. What a process
/// is read for and vouched for by, as well as judged by.
pub(crate) const BUILD_VARIABLES: [&str; 3] =
    ["CARGO_TARGET_DIR", "CARGO_BUILD_BUILD_DIR", "CARGO_HOME"];

/// Whether the guard keeps a variable of a process's environment: those
/// that point a build somewhere, and the temporary directory it uses, which
/// are all it keeps.
pub(crate) fn kept(name: &[u8]) -> bool {
    name == b"TMPDIR"
        || BUILD_VARIABLES
            .iter()
            .any(|variable| name == variable.as_bytes())
}

/// A program's name, through Nix's wrappers: a binary wrapped as
/// `.name-wrapped`, and a script, known only by the name it was called by.
pub(crate) fn names(process: &Process) -> impl Iterator<Item = String> + '_ {
    let exe = process.exe.as_deref().and_then(Path::file_name);
    let called = process
        .cmdline
        .first()
        .and_then(|word| Path::new(word).file_name());
    exe.into_iter().chain(called).map(|name| {
        let name = name.to_string_lossy();
        let name = name.strip_prefix('.').unwrap_or(&name);
        name.strip_suffix("-wrapped").unwrap_or(name).to_owned()
    })
}

/// Whether a process runs the program named `command`, by its executable
/// or the name it was called by.
pub fn runs(process: &Process, command: &str) -> bool {
    names(process).any(|name| name == command)
}

fn builds(process: &Process) -> bool {
    names(process).any(|name| BUILD_TOOLS.contains(&name.as_str()))
}

/// rust-analyzer's proc-macro server, standalone or as `rust-analyzer
/// proc-macro`. It maps the macros a target holds and writes nothing there.
fn serves_macros(process: &Process) -> bool {
    let mode = process
        .cmdline
        .get(1)
        .is_some_and(|word| word == "proc-macro");
    names(process)
        .any(|name| name == "rust-analyzer-proc-macro-srv" || (name == "rust-analyzer" && mode))
}

/// A path with `.` and `..` worked out by its words alone, since the
/// directory a variable names may not exist yet.
fn normalized(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// How one process uses a target. Of a target kept `within` another, only
/// a variable naming it, or something in it, says a build goes there: one
/// naming the target around it, as a zone's shell does, or a process
/// sitting in the checkout, builds into the outer one.
fn used(process: &Process, target: &Path, checkout: Option<&Path>, within: Option<&Path>) -> Use {
    let pid = process.pid;
    if !process.readable {
        return Use::Unknown(pid);
    }
    let under = |path: &PathBuf| path.starts_with(target);
    if process
        .cwd
        .iter()
        .chain(&process.exe)
        .chain(&process.files)
        .any(under)
    {
        return Use::Busy(pid);
    }
    let builds = builds(process);
    if process.maps.iter().any(under) {
        return if serves_macros(process) {
            Use::Occupied(pid)
        } else {
            Use::Busy(pid)
        };
    }
    let sits = within.is_none()
        && checkout.is_some_and(|checkout| {
            process
                .cwd
                .as_ref()
                .is_some_and(|cwd| cwd.starts_with(checkout))
        });
    let points = process.env.iter().any(|(name, value)| {
        if !BUILD_VARIABLES.iter().any(|variable| name == variable) {
            return false;
        }
        let value = Path::new(value);
        let value = match &process.cwd {
            Some(cwd) if value.is_relative() => normalized(&cwd.join(value)),
            _ => normalized(value),
        };
        // The target, somewhere in it, or the directory it sits in.
        value.starts_with(target) || (within.is_none() && target.starts_with(&value))
    });
    match (sits || points, builds) {
        (false, _) => Use::Free,
        (true, true) => Use::Busy(pid),
        (true, false) => Use::Occupied(pid),
    }
}

/// How a target is used: busy when any process works in it or builds its
/// checkout, occupied when they only sit there, and unknown when only a
/// process the guard can't read stands in the way. The guard (`me`) holds
/// its own files there while it works, and is left out.
pub fn judge(processes: &[Process], me: u32, target: &Path, checkout: Option<&Path>) -> Use {
    judge_within(processes, me, target, checkout, None)
}

/// How a target is used, as `judge` says, when it may be kept `within`
/// another target.
pub fn judge_within(
    processes: &[Process],
    me: u32,
    target: &Path,
    checkout: Option<&Path>,
    within: Option<&Path>,
) -> Use {
    processes
        .iter()
        .filter(|process| process.pid != me)
        .map(|process| used(process, target, checkout, within))
        .fold(Use::Free, |worst, next| {
            if next.rank() > worst.rank() {
                next
            } else {
                worst
            }
        })
}

/// Where processes work, read once so that asking it of many paths costs
/// a lookup each rather than a pass over every process's files and maps.
/// On one laptop a survey asked it of tens of thousands of quest entries,
/// each against half a million open and mapped files, which took minutes
/// of CPU a pass. It answers as `judge` calling a path busy does.
pub struct Workplaces<'a> {
    /// What a process sits in, runs, holds or maps, or a build variable
    /// names: a path is worked in when one of these lies in it.
    held: BTreeSet<PathBuf>,
    /// What a build variable names, which also makes all inside it busy.
    around: Vec<PathBuf>,
    /// A process both serving macros and building, which `used` judges by
    /// its maps before its variables, so it's asked the slow way.
    odd: Vec<&'a Process>,
}

impl<'a> Workplaces<'a> {
    /// Where every process but the guard (`me`) works. One it can't read
    /// works nowhere here, as `judge` only calls it unknown.
    pub fn new(processes: &'a [Process], me: u32) -> Self {
        let mut places = Workplaces {
            held: BTreeSet::new(),
            around: Vec::new(),
            odd: Vec::new(),
        };
        for process in processes {
            if process.pid == me || !process.readable {
                continue;
            }
            let (serves, builds) = (serves_macros(process), builds(process));
            if serves && builds {
                places.odd.push(process);
                continue;
            }
            let mapped = if serves { &[][..] } else { &process.maps[..] };
            let opened = process.cwd.iter().chain(&process.exe).chain(&process.files);
            places.held.extend(opened.chain(mapped).cloned());
            if builds {
                for (name, value) in &process.env {
                    if !BUILD_VARIABLES.iter().any(|variable| name == variable) {
                        continue;
                    }
                    let value = Path::new(value);
                    let value = match &process.cwd {
                        Some(cwd) if value.is_relative() => normalized(&cwd.join(value)),
                        _ => normalized(value),
                    };
                    places.held.insert(value.clone());
                    places.around.push(value);
                }
            }
        }
        places
    }

    /// Whether a process works in `path`, so `judge` would call it busy.
    /// Paths order by their components, so all that lie in `path` follow
    /// it at once, and the first after it says whether any do.
    pub fn work_in(&self, path: &Path) -> bool {
        let inside = self
            .held
            .range::<Path, _>((Bound::Included(path), Bound::Unbounded))
            .next()
            .is_some_and(|held| held.starts_with(path));
        inside
            || self.around.iter().any(|named| path.starts_with(named))
            || self
                .odd
                .iter()
                .any(|process| matches!(used(process, path, None, None), Use::Busy(_)))
    }
}

/// One process as the platform reads it. It crosses the system daemon's
/// socket, where a path that isn't UTF-8 won't go, and the asker then
/// keeps the process unknown.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Process {
    pub pid: u32,
    pub uid: u32,
    /// When it started, in clock ticks since boot.
    pub start_ticks: u64,
    pub name: String,
    pub exe: Option<PathBuf>,
    pub cwd: Option<PathBuf>,
    pub cmdline: Vec<OsString>,
    /// The build variables in its environment.
    pub env: Vec<(OsString, OsString)>,
    /// Files it holds open.
    pub files: Vec<PathBuf>,
    /// Files it has mapped, executables and libraries among them.
    pub maps: Vec<PathBuf>,
    /// Whether everything above could be read; a process that hides its
    /// files, cwd or maps can't be judged.
    pub readable: bool,
    pub write_bytes: Option<u64>,
    pub cgroup: Option<String>,
}
