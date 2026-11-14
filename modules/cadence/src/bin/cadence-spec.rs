//! The Spec daemon's own name: `cadence-spec ARGS` becomes `cadence serve
//! ARGS`, shown as cadence-spec. Every daemon's socket unit, service and
//! program say one name (docs/daemons.md), and `cadence` is already the
//! five-minute tick's, so serving the axes takes a program of its own.
//! It execs, keeping its pid, so the socket systemd hands it (LISTEN_PID)
//! stays its own.

use std::os::unix::process::CommandExt;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let cadence = std::env::current_exe().map(|own| own.with_file_name("cadence"));
    let Ok(cadence) = cadence else {
        eprintln!("cadence-spec: can't find where it runs from");
        return ExitCode::FAILURE;
    };
    let failed = Command::new(&cadence)
        .arg0("cadence-spec")
        .arg("serve")
        .args(std::env::args_os().skip(1))
        .exec();
    eprintln!("cadence-spec: can't run {}: {failed}", cadence.display());
    ExitCode::FAILURE
}
