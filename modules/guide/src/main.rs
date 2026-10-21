//! `guide observe` says what the guide sees on this machine and changes
//! nothing; `guide tools` says where this build finds each program it
//! runs, and fails when one is missing; `guide write-stick IMAGE FILE`
//! writes a repair stick into a file the size of one.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use guide::observe::{disks, efi, entries, fido2, sbctl, tpm};
use guide::stick::{self, Image};
use guide::system::System;
use guide::vault::{self, Manifest};
use guide::{Fault, Journal, Tool};

const USAGE: &str = "usage: guide observe | guide tools | guide write-stick IMAGE FILE";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["observe"] => observe(),
        ["tools"] => tools(),
        ["write-stick", image, target] => match write_stick(image, target) {
            Ok(()) => ExitCode::SUCCESS,
            Err(fault) => {
                eprintln!("{fault}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// What a repair stick's vault says: it's no one machine's, and nothing
/// opens it, since it carries nothing secret.
fn repair_manifest() -> Manifest {
    Manifest {
        format: 1,
        machine: String::new(),
        opened_by: vec![],
    }
}

/// A repair stick, into a file already the size of the stick it stands for.
/// A disk is refused: writing one comes with the steps that find it and
/// have the person confirm it.
fn write_stick(image: &str, target: &str) -> Result<(), Fault> {
    let kept = |why: std::io::Error| Fault::Unkept {
        path: target.into(),
        why: why.to_string(),
    };
    if !std::fs::metadata(target).map_err(kept)?.is_file() {
        return Err(Fault::Blocked {
            step: "write-stick",
            why: format!("{target} isn't a file, and this only writes into files"),
        });
    }
    let file = std::fs::File::open(image).map_err(|why| Fault::unreadable(image, why))?;
    let mut image = Image::open(file)?;
    let mut disk = OpenOptions::new()
        .read(true)
        .write(true)
        .open(target)
        .map_err(kept)?;
    let manifest = repair_manifest();
    let written = stick::write(&mut image, &mut disk, stick::new_guid()?, |store| {
        let bytes = serde_json::to_vec_pretty(&manifest)
            .map_err(|why| Fault::unreadable("the manifest", why))?;
        store
            .create(vault::MANIFEST)?
            .write_all(&bytes)
            .map_err(kept)
    })?;
    stick::verify(&written, &mut disk, |store| {
        if vault::manifest(store)? == repair_manifest() {
            Ok(())
        } else {
            Err(Fault::Unproven {
                step: "write-stick",
                standing: "the vault's manifest isn't what was written".into(),
            })
        }
    })?;
    println!(
        "{target}: the image and a {} MB vault, read back as written",
        written.vault.bytes() / 1_000_000
    );
    Ok(())
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
