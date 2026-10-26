//! Whether any process on the machine still uses a version.
//!
//! A running pi keeps the version it loaded: its native modules mapped,
//! files open, and the code it imports later read from the same folder.
//! So a version `current` has left is removed only once nothing maps a
//! file in it, holds one open or stands in it. Linux says so through
//! `/proc` and macOS through libproc (inuse/macos.rs); elsewhere nothing
//! is known to use it and its age alone decides.

use std::fs;
use std::path::Path;

#[cfg(target_os = "macos")]
mod macos;

/// Whether a process maps, holds open or stands in a file under `dir`.
#[must_use]
pub fn in_use(dir: &Path) -> bool {
    // Both kernels say every path with its links resolved, so the folder
    // is compared that way too; one that can't be resolved is gone.
    let Ok(dir) = fs::canonicalize(dir) else {
        return false;
    };
    let Some(prefix) = dir
        .to_str()
        .map(|dir| format!("{}/", dir.trim_end_matches('/')))
    else {
        return false;
    };
    let inside = |path: &str| path.starts_with(&prefix) || path == prefix.trim_end_matches('/');
    any_process(&inside)
}

#[cfg(target_os = "macos")]
fn any_process(inside: &dyn Fn(&str) -> bool) -> bool {
    macos::any_process(inside)
}

#[cfg(not(target_os = "macos"))]
fn any_process(inside: &dyn Fn(&str) -> bool) -> bool {
    let Ok(processes) = fs::read_dir("/proc") else {
        return false;
    };
    processes.flatten().any(|process| {
        let proc = process.path();
        let numbered = process
            .file_name()
            .to_str()
            .is_some_and(|name| name.bytes().all(|byte| byte.is_ascii_digit()));
        if !numbered {
            return false;
        }
        let stands = fs::read_link(proc.join("cwd"))
            .ok()
            .and_then(|cwd| cwd.to_str().map(inside))
            .unwrap_or(false);
        let maps = fs::read_to_string(proc.join("maps")).is_ok_and(|maps| {
            maps.lines()
                .filter_map(|line| line.split_whitespace().nth(5))
                .any(inside)
        });
        let holds = fs::read_dir(proc.join("fd")).is_ok_and(|fds| {
            fds.flatten().any(|fd| {
                fs::read_link(fd.path())
                    .ok()
                    .and_then(|target| target.to_str().map(inside))
                    .unwrap_or(false)
            })
        });
        stands || maps || holds
    })
}
