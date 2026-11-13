//! Writes which memory modules the firmware lists, as JSON, for the
//! launcher's Memory to read.
//!
//! The firmware lists each memory module as an SMBIOS memory device (type
//! 17), which the kernel shows in /sys/firmware/dmi/entries, readable by
//! root alone. nixos.nix runs this as root once at boot with that folder and
//! the file to write, and moves the file into place. Each module is its
//! locator, size, type, speed, maker and part, an empty string where the
//! firmware says nothing (src/smbios.rs reads them), and a machine whose
//! firmware lists none gets an empty list.
//!
//!   memory-modules ENTRIES WRITTEN    writes the modules ENTRIES lists to WRITTEN
//!
//! It takes the shape every command of ours does (docs/commands.md): its
//! help, and a failure that says what to try.

mod smbios;

use serde::Serialize;
use smbios::{MEMORY_DEVICE, Module};
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use terminal_kit::{Exit, Failure, Help};

/// What the launcher reads.
#[derive(Serialize)]
struct Written {
    modules: Vec<Module>,
}

/// Every memory device's entry in the folder, by the number the kernel gives
/// it among those of its type, in that order.
fn entries_in(entries: &Path) -> io::Result<Vec<PathBuf>> {
    let prefix = format!("{MEMORY_DEVICE}-");
    let mut found: Vec<(u32, PathBuf)> = fs::read_dir(entries)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let instance = name.strip_prefix(&prefix)?.parse().ok()?;
            Some((instance, entry.path()))
        })
        .collect();
    found.sort_by_key(|(instance, _)| *instance);
    Ok(found.into_iter().map(|(_, path)| path).collect())
}

/// Every memory module the firmware lists, in its order, leaving out empty
/// slots; none where the folder isn't there.
fn modules_in(entries: &Path) -> io::Result<Vec<Module>> {
    if !entries.is_dir() {
        return Ok(Vec::new());
    }
    let mut modules = Vec::new();
    for entry in entries_in(entries)? {
        if let Some(module) = smbios::module_of(&fs::read(entry.join("raw"))?) {
            modules.push(module);
        }
    }
    Ok(modules)
}

/// Writes the modules found in the entries folder to the file, readable by
/// all.
fn write(entries: &Path, written: &Path) -> io::Result<()> {
    let said = serde_json::to_string(&Written {
        modules: modules_in(entries)?,
    })
    .map_err(io::Error::other)?;
    fs::write(written, said + "\n")?;
    fs::set_permissions(written, fs::Permissions::from_mode(0o644))
}

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "memory-modules",
        "writes which memory modules the firmware lists, as JSON, for the launcher",
    )
    .example(
        "memory-modules /sys/firmware/dmi/entries modules.json",
        "what its unit runs at boot, as root",
    )
    .exit(
        Exit::Failed,
        "it couldn't read the firmware's list or write the file",
    )
    .exit(Exit::Refused, "only root may read the firmware's list")
}

/// Why it couldn't write the modules, and what to try.
fn failed(error: &io::Error, entries: &str, written: &str) -> Failure {
    if error.kind() == io::ErrorKind::PermissionDenied {
        return Failure::new(
            Exit::Refused,
            "it may not read the firmware's list or write the file",
        )
        .because(error.to_string())
        .try_this(format!("run0 memory-modules {entries} {written}"));
    }
    Failure::new(Exit::Failed, format!("it couldn't write {written}"))
        .because(error.to_string())
        .try_this(format!(
            "check that {entries} is there and {written} can be written"
        ))
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let [entries, written] = context.args.as_slice() else {
            return Err(Failure::new(
                Exit::Usage,
                "it takes the firmware's entries and the file to write",
            )
            .try_this("memory-modules /sys/firmware/dmi/entries modules.json"));
        };
        write(Path::new(entries), Path::new(written))
            .map_err(|error| failed(&error, entries, written))
    })
}
