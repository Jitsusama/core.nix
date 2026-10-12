//! What processes are doing, which decides what's busy and what's occupied.

use std::ffi::OsString;
use std::path::PathBuf;

/// One process as the platform reads it.
#[derive(Clone, Debug, Default, PartialEq)]
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
