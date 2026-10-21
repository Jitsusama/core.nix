//! `guide observe` says what the guide sees on this machine and changes
//! nothing; `guide tools` says where this build finds each program it
//! runs, and fails when one is missing.

use std::path::Path;
use std::process::ExitCode;

use guide::observe::{disks, efi, entries, fido2, sbctl, tpm};
use guide::system::System;
use guide::{Fault, Journal, Tool};

const USAGE: &str = "usage: guide observe | guide tools";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["observe"] => observe(),
        ["tools"] => tools(),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn tools() -> ExitCode {
    let mut missing = 0;
    for tool in Tool::ALL {
        match tool.program() {
            Some(program) if Path::new(program).is_file() => println!("{tool}: {program}"),
            Some(program) => {
                missing += 1;
                println!("{tool}: {program} isn't there");
            }
            None => {
                missing += 1;
                println!("{tool}: this build has no path for it");
            }
        }
    }
    if missing == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Each observation on its own, so one a person can't make (sbctl's keys
/// need root) still leaves the rest to read.
fn observe() -> ExitCode {
    let mut machine = System::new(Journal::in_memory(), None);
    let mut faults = 0;
    let mut show = |what: &str, said: Result<String, Fault>| match said {
        Ok(said) => println!("{what}: {said}"),
        Err(fault) => {
            faults += 1;
            println!("{what}: {fault}");
        }
    };

    show(
        "Secure Boot",
        efi::SecureBoot::observe(&mut machine).map(|seen| match seen {
            Some(seen) => format!(
                "{}, {} mode",
                if seen.enforcing {
                    "enforced"
                } else {
                    "not enforced"
                },
                seen.mode
            ),
            None => "this machine didn't boot through UEFI".into(),
        }),
    );
    show(
        "Owner keys",
        sbctl::Status::observe(&mut machine).map(|status| match status.guid {
            Some(guid) if status.installed => format!("the machine's own, owner {guid}"),
            _ => "none of the machine's own".into(),
        }),
    );
    show(
        "Booted",
        entries::status(&mut machine).map(|fields| {
            match fields.measured_uki() {
                Some(true) => "a measured UKI",
                Some(false) => "a kernel that didn't measure itself",
                None => "bootctl didn't say how",
            }
            .into()
        }),
    );
    show(
        "TPM",
        tpm::Lockout::observe(&mut machine).map(|lockout| {
            format!(
                "{} of {} wrong PINs left{}{}",
                lockout.left(),
                lockout.allowed,
                if lockout.locked { ", locked out" } else { "" },
                if lockout.guarded {
                    ", lockout guarded by a password"
                } else {
                    ""
                }
            )
        }),
    );
    show(
        "FIDO2 keys",
        fido2::list(&mut machine).map(|tokens| {
            if tokens.is_empty() {
                return "none plugged in".into();
            }
            tokens
                .iter()
                .map(|token| {
                    let maker = if token.vendor == fido2::YUBICO {
                        "a YubiKey"
                    } else {
                        "a key"
                    };
                    format!("{maker} at {} ({})", token.path, token.name)
                })
                .collect::<Vec<_>>()
                .join("; ")
        }),
    );
    match disks::observe(&mut machine) {
        Ok(found) => {
            for disk in found {
                show(
                    &disk.path,
                    Ok(format!(
                        "{} GB {}, {}{}",
                        disk.bytes / 1_000_000_000,
                        disk.model.as_deref().unwrap_or("disk"),
                        match disk.kind {
                            disks::Kind::Internal => "built in",
                            disks::Kind::Removable => "removable",
                        },
                        if disk.installable() {
                            ", free to install on"
                        } else if disk.in_use() {
                            ", in use"
                        } else {
                            ""
                        }
                    )),
                );
            }
        }
        Err(fault) => show("Disks", Err(fault)),
    }

    if faults == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
