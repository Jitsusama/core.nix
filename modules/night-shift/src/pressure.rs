//! Memory pressure as the kernel reports it, for the machine and for a
//! cgroup: the share of the last ten seconds, and of the last minute, in
//! which every task that wanted memory waited for it.

use std::fs;
use std::path::{Path, PathBuf};

/// A pressure file's `full` line: the share of time, in percent, in which
/// every task that wanted memory was waiting for it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Full {
    pub avg10: f64,
    pub avg60: f64,
}

/// The `full` line of a pressure file's text, or None when it has none.
pub fn full(text: &str) -> Option<Full> {
    let line = text.lines().find(|line| line.starts_with("full "))?;
    let field = |name: &str| {
        line.split_whitespace()
            .find_map(|word| word.strip_prefix(name)?.strip_prefix('='))
            .and_then(|value| value.parse().ok())
    };
    Some(Full {
        avg10: field("avg10")?,
        avg60: field("avg60")?,
    })
}

/// Where the pressure files are: the machine's under `proc`, and every
/// cgroup's under `cgroups`.
pub struct Places {
    pub proc: PathBuf,
    pub cgroups: PathBuf,
}

impl Places {
    /// The machine's own files, or a test's stand-ins named by
    /// `NIGHT_SHIFT_PROC` and `NIGHT_SHIFT_CGROUPS`.
    pub fn of_this_machine() -> Self {
        let from = |var: &str, fallback: &str| {
            std::env::var_os(var).map_or_else(|| PathBuf::from(fallback), PathBuf::from)
        };
        Places {
            proc: from("NIGHT_SHIFT_PROC", "/proc"),
            cgroups: from("NIGHT_SHIFT_CGROUPS", "/sys/fs/cgroup"),
        }
    }

    /// The machine's memory pressure file.
    pub fn machine(&self) -> PathBuf {
        self.proc.join("pressure").join("memory")
    }

    /// This process's cgroup, as `/proc/self/cgroup`'s unified line names
    /// it, relative to the cgroup tree's root.
    fn own(&self) -> Option<PathBuf> {
        let text = fs::read_to_string(self.proc.join("self").join("cgroup")).ok()?;
        let path = text.lines().find_map(|line| line.strip_prefix("0::"))?;
        Some(PathBuf::from(path.trim_start_matches('/')))
    }

    /// This process's own cgroup's memory pressure file: its unit's, as
    /// systemd runs it, which what it starts shares.
    pub fn own_file(&self) -> Option<PathBuf> {
        Some(self.cgroups.join(self.own()?).join("memory.pressure"))
    }

    /// The memory pressure file of `slice`, a slice of the account's own
    /// manager when this process runs under one (`user@<uid>.service`),
    /// otherwise of the machine's.
    pub fn slice_file(&self, slice: &str) -> PathBuf {
        let manager = self.own().and_then(|own| manager_of(&own));
        self.cgroups
            .join(manager.unwrap_or_default())
            .join(slice)
            .join("memory.pressure")
    }
}

/// The account manager's cgroup that `own` sits under, if any.
fn manager_of(own: &Path) -> Option<PathBuf> {
    let mut manager = PathBuf::new();
    for part in own.components() {
        manager.push(part);
        let name = part.as_os_str().to_string_lossy();
        if name.starts_with("user@") && name.ends_with(".service") {
            return Some(manager);
        }
    }
    None
}

/// The full pressure a file reports now, or None when it can't be read.
pub fn read(file: &Path) -> Option<Full> {
    full(&fs::read_to_string(file).ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "some avg10=4.20 avg60=1.50 avg300=0.30 total=99\n\
                        full avg10=2.75 avg60=0.81 avg300=0.10 total=42\n";

    #[test]
    fn a_pressure_file_gives_its_full_line() {
        assert_eq!(
            full(FILE),
            Some(Full {
                avg10: 2.75,
                avg60: 0.81
            })
        );
    }

    #[test]
    fn a_file_with_no_full_line_gives_nothing() {
        assert_eq!(
            full("some avg10=4.20 avg60=1.50 avg300=0.30 total=99\n"),
            None
        );
    }

    #[test]
    fn a_slice_is_found_under_the_account_manager() {
        let own =
            Path::new("user.slice/user-1000.slice/user@1000.service/background.slice/job.service");
        assert_eq!(
            manager_of(own),
            Some(PathBuf::from(
                "user.slice/user-1000.slice/user@1000.service"
            ))
        );
    }

    #[test]
    fn a_system_unit_has_no_account_manager() {
        assert_eq!(manager_of(Path::new("system.slice/job.service")), None);
    }
}
