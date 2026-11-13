//! Keeps what smartd heard from a disk where device-check, which each person
//! at the machine runs, can read it. smartd runs it as root with the fault in
//! its SMARTD_ variables, once for each kind of fault a disk reports, and
//! reads anything it prints as the hook going wrong, so it prints nothing.
//!
//!   device-check-disk    keeps the fault smartd names, one file a kind
//!
//! /run empties at every boot, and smartd names each fault again once it
//! starts, so what's kept is what the disks report this boot.

use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;
use terminal_kit::{Exit, Failure, Help};

/// What smartd reported, without the device it starts with, which
/// device-check names itself, and on one line.
fn report(device: &str, message: &str) -> String {
    let prefix = format!("Device: {device}, ");
    message
        .strip_prefix(&prefix)
        .unwrap_or(message)
        .replace(['\t', '\n', '\r'], " ")
}

/// One of smartd's variables, which it always sets.
fn smartd(name: &str) -> io::Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| io::Error::other(format!("{name} isn't set")))
}

fn keep() -> io::Result<()> {
    let device = smartd("SMARTD_DEVICE")?;
    let kind = smartd("SMARTD_FAILTYPE")?;
    let message = smartd("SMARTD_MESSAGE")?;

    let mut dir = env::var_os("DEVICE_CHECK_ROOT").unwrap_or_default();
    dir.push("/run/device-check/disks");
    let dir = PathBuf::from(dir);
    fs::create_dir_all(&dir)?;
    let name = device.rsplit('/').next().unwrap_or(&device);
    let file = dir.join(format!("{name}-{kind}"));
    let new = dir.join(format!("{name}-{kind}.new"));
    fs::write(&new, format!("{device}\t{}\n", report(&device, &message)))?;
    fs::rename(&new, &file)
}

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "device-check-disk",
        "keeps what smartd heard from a disk, for device-check to name",
    )
    .example(
        "SMARTD_DEVICE=/dev/nvme0 SMARTD_FAILTYPE=Health SMARTD_MESSAGE=... device-check-disk",
        "what smartd runs, its fault in its variables",
    )
    .exit(
        Exit::Failed,
        "smartd's variables weren't set, or it couldn't keep the fault",
    )
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        if let Some(word) = context.args.first() {
            return Err(
                Failure::new(Exit::Usage, format!("{word} isn't taken here"))
                    .because("smartd hands it the fault in SMARTD_ variables, not on the line")
                    .try_this("device-check-disk --help"),
            );
        }
        keep().map_err(|error| {
            Failure::new(Exit::Failed, "it couldn't keep the disk's fault")
                .because(error.to_string())
                .try_this("check that smartd runs it as root, with its SMARTD_ variables")
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_the_device_smartd_starts_with() {
        assert_eq!(
            report(
                "/dev/nvme0",
                "Device: /dev/nvme0, Critical Warning (0x04): Reliability"
            ),
            "Critical Warning (0x04): Reliability"
        );
    }

    #[test]
    fn keeps_a_message_that_names_another_device() {
        assert_eq!(
            report("/dev/nvme0", "Device: /dev/sda, x"),
            "Device: /dev/sda, x"
        );
    }

    #[test]
    fn keeps_a_report_on_one_line() {
        assert_eq!(report("/dev/nvme0", "a\tb\nc\rd"), "a b c d");
    }
}
