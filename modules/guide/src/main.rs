//! `guide observe` says what the guide sees on this machine and changes
//! nothing; `guide tools` says where this build finds each program it
//! runs, and fails when one is missing. `guide write-stick IMAGE FILE`
//! writes a stick into a file the size of one: given `--sign-with` and
//! sbctl's db key directory, its boot program signed for that machine;
//! given `--install` and a machine's payload (payload.nix), the vault
//! holding everything to install it, sealed to the recovery passphrase
//! read from standard input. `guide install` is what the stick runs on
//! its console, as root.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use guide::install::{self, LAYOUT_SCRIPT, SYSTEM_CACHE, TARGET, Target};
use guide::observe::{disks, efi, entries, fido2, sbctl, tpm};
use guide::screen::Theme;
use guide::screen::run::{Ended, Walk};
use guide::screen::terminal::Terminal;
use guide::sign::Signer;
use guide::stick::{self, Image};
use guide::system::System;
use guide::vault::store::{Fat, Store};
use guide::vault::{self, Manifest, Packer, Vault, VaultKey};
use guide::{Fault, Journal, Journey, Machine, Place, Secret, Tool};

const USAGE: &str = "usage: guide observe | guide tools | guide install
       guide write-stick IMAGE FILE [--sign-with DIRECTORY] [--install PAYLOAD]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["observe"] => observe(),
        ["tools"] => tools(),
        ["install"] => reported(install()),
        ["write-stick", image, target, ref options @ ..] => match written(options) {
            Some((keys, payload)) => reported(
                keys.map(|keys| Signer::open(Path::new(keys)))
                    .transpose()
                    .and_then(|signer| write_stick(image, target, signer.as_ref(), payload)),
            ),
            None => {
                eprintln!("{USAGE}");
                ExitCode::from(2)
            }
        },
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// write-stick's options: the db key directory and the payload, each
/// given at most once.
fn written<'a>(mut options: &[&'a str]) -> Option<(Option<&'a str>, Option<&'a str>)> {
    let (mut keys, mut payload) = (None, None);
    while let [option, value, rest @ ..] = options {
        match *option {
            "--sign-with" if keys.is_none() => keys = Some(*value),
            "--install" if payload.is_none() => payload = Some(*value),
            _ => return None,
        }
        options = rest;
    }
    options.is_empty().then_some((keys, payload))
}

fn reported(result: Result<(), Fault>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(fault) => {
            eprintln!("{fault}");
            ExitCode::FAILURE
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

/// Every file under a directory, links followed, as paths relative to it,
/// in a steady order.
fn files_under(root: &Path) -> Result<Vec<PathBuf>, Fault> {
    let mut found = vec![];
    let mut pending = vec![PathBuf::new()];
    while let Some(relative) = pending.pop() {
        let here = root.join(&relative);
        let entries = std::fs::read_dir(&here)
            .map_err(|why| Fault::unreadable(here.display().to_string(), why))?;
        for entry in entries {
            let entry = entry.map_err(|why| Fault::unreadable(here.display().to_string(), why))?;
            let path = relative.join(entry.file_name());
            if root.join(&path).is_dir() {
                pending.push(path);
            } else {
                found.push(path);
            }
        }
    }
    found.sort();
    Ok(found)
}

/// The recovery passphrase, as the first line of standard input.
fn passphrase() -> Result<Secret, Fault> {
    let mut line = zeroize::Zeroizing::new(String::new());
    std::io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(|why| Fault::unreadable("the recovery passphrase on standard input", why))?;
    let passphrase = line.trim_end_matches(['\n', '\r']);
    if passphrase.chars().count() < 12 {
        return Err(Fault::Blocked {
            step: "write-stick",
            why: "a recovery passphrase has to be at least 12 characters".into(),
        });
    }
    Ok(Secret::new(passphrase.to_string()))
}

/// A stick, into a file already the size of the stick it stands for. A
/// disk is refused: writing one comes with the steps that find it and have
/// the person confirm it.
fn write_stick(
    image: &str,
    target: &str,
    signer: Option<&Signer>,
    payload: Option<&str>,
) -> Result<(), Fault> {
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
    let file = File::open(image).map_err(|why| Fault::unreadable(image, why))?;
    let mut image = Image::open(file)?;
    let mut disk = OpenOptions::new()
        .read(true)
        .write(true)
        .open(target)
        .map_err(kept)?;

    let Some(payload) = payload.map(Path::new) else {
        let manifest = repair_manifest();
        let written = stick::write(&mut image, &mut disk, stick::new_guid()?, signer, |store| {
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
        return Ok(());
    };

    let unread = |path: &Path| {
        let shown = path.display().to_string();
        move |why: std::io::Error| Fault::unreadable(shown.clone(), why)
    };
    let described: Target = serde_json::from_slice(
        &std::fs::read(payload.join(TARGET)).map_err(unread(&payload.join(TARGET)))?,
    )
    .map_err(|why| Fault::unreadable("the payload's target.json", why))?;
    let cache = payload.join("cache");
    let cached = files_under(&cache)?;

    // Each file takes whole clusters, FAT32's largest being 32 KiB, and
    // age adds a little to each; refused before anything is written.
    let cluster = 32 << 10;
    let mut needed: u64 = 16 << 20;
    for from in [payload.join(TARGET), payload.join("layout.sh")]
        .into_iter()
        .chain(cached.iter().map(|relative| cache.join(relative)))
    {
        let bytes = std::fs::metadata(&from).map_err(unread(&from))?.len();
        needed += (bytes + bytes / 1000 + 512).div_ceil(cluster) * cluster;
    }
    let stick_bytes = std::fs::metadata(target).map_err(kept)?.len();
    let room = stick::vault_room(&image, stick_bytes)?;
    if room < needed {
        return Err(Fault::Blocked {
            step: "write-stick",
            why: format!(
                "the stick holds {} MB of vault, and {}'s install needs {} MB",
                room / 1_000_000,
                described.machine,
                needed.div_ceil(1_000_000)
            ),
        });
    }
    let passphrase = passphrase()?;
    let key = VaultKey::generate();
    let sealed = key.seal_with_passphrase(&passphrase, None)?;
    let manifest = Manifest {
        format: 1,
        machine: described.machine.clone(),
        opened_by: vec!["the recovery passphrase".into()],
    };
    let written = stick::write(&mut image, &mut disk, stick::new_guid()?, signer, |store| {
        store
            .create(vault::KEY_PASSPHRASE)?
            .write_all(&sealed)
            .map_err(kept)?;
        let mut packer = Packer::new(store, &key);
        for (name, from) in [
            (TARGET, payload.join(TARGET)),
            (LAYOUT_SCRIPT, payload.join("layout.sh")),
        ] {
            packer.add(name, File::open(&from).map_err(unread(&from))?)?;
        }
        for relative in &cached {
            let from = cache.join(relative);
            packer.add(
                &format!("{SYSTEM_CACHE}/{}", relative.display()),
                File::open(&from).map_err(unread(&from))?,
            )?;
        }
        packer.finish(&manifest)
    })?;
    // Read back as the stick will open it: with the passphrase, every file
    // extracted and checked.
    stick::verify(&written, &mut disk, |store| {
        let mut sealed = vec![];
        store
            .open(vault::KEY_PASSPHRASE)?
            .read_to_end(&mut sealed)
            .map_err(kept)?;
        let mut vault = Vault::open(store, VaultKey::open_with_passphrase(&sealed, &passphrase)?)?;
        let paths: Vec<String> = vault.files().iter().map(|file| file.path.clone()).collect();
        for path in &paths {
            vault.extract(path, std::io::sink())?;
        }
        Ok(())
    })?;
    println!(
        "{target}: {}'s install, {} files in a {} MB vault, read back as written",
        described.machine,
        cached.len() + 2,
        written.vault.bytes() / 1_000_000
    );
    Ok(())
}

/// Where the stick keeps its journal and log: memory, gone when it stops.
const JOURNAL: &str = "/run/guide/journal";
const LOG: &str = "/run/guide/log";
const VAULT: &str = "/dev/disk/by-partlabel/vault";

/// The install, on the stick's console: the vault opened, then every step
/// it holds for the machine, then the stick taken out and the machine
/// restarted.
fn install() -> Result<(), Fault> {
    // SAFETY: getuid can't fail and touches no memory.
    if unsafe { libc::getuid() } != 0 {
        return Err(Fault::Blocked {
            step: "install",
            why: "the install runs as root, on the stick".into(),
        });
    }
    let partition = OpenOptions::new()
        .read(true)
        .write(true)
        .open(VAULT)
        .map_err(|why| Fault::unreadable("the stick's vault", why))?;
    let mut vault: Box<dyn Store> = Box::new(Fat::open(partition)?);
    let machine_name = vault::manifest(vault.as_mut())?.machine;
    let journal = Journal::open(Path::new(JOURNAL))?;
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG)
        .map_err(|why| Fault::Unkept {
            path: LOG.into(),
            why: why.to_string(),
        })?;
    let mut machine = System::new(journal, Some(log)).with_vault(vault);
    let theme = Theme::built(Terminal::is_console());
    let mut terminal = Terminal::open()?;
    let title = format!("Install {machine_name}");

    // The steps before the vault opens, then, from what it holds, all of
    // them, those already done passed over.
    let opening = Journey::new(install::opening())
        .map_err(|why| Fault::unreadable("the install's steps", why))?;
    let mut walk = Walk {
        journey: &opening,
        here: Place::Stick,
        name: &title,
        theme: &theme,
        log: Some(Path::new(LOG)),
        whole: false,
    };
    if walk.walk(&mut machine, &mut terminal)? != Ended::Finished {
        return Ok(());
    }
    let target = install::target(&mut machine)?;
    let journey = Journey::new(install::steps(&target.yubikeys))
        .map_err(|why| Fault::unreadable("the install's steps", why))?;
    walk.journey = &journey;
    walk.whole = true;
    if walk.walk(&mut machine, &mut terminal)? != Ended::Finished {
        return Ok(());
    }

    // The reboot card: what the next boot asks for, said before it. The
    // machine shuts down rather than restarting, so the stick comes out
    // of a machine that's off.
    let installed = format!("{} is installed", target.machine);
    let next = format!(
        "Press Return to shut down. Take the stick out, then turn {} on: its disk asks for a security token PIN, so type a YubiKey's PIN and touch the key when it blinks.",
        target.machine
    );
    if !walk.card(&mut terminal, &installed, &next, "shut down")? {
        return Ok(());
    }
    machine
        .run(Tool::Umount, &["--recursive", install::ROOT])?
        .ok()?;
    machine
        .run(Tool::Cryptsetup, &["close", &target.disk_name])?
        .ok()?;
    machine.run(Tool::Systemctl, &["poweroff"])?.ok()?;
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
    for absent in Theme::built(false).missing() {
        missing += 1;
        println!("the theme: this build has no {absent}");
    }
    for path in guide::tool::Path::ALL {
        let name = path.name();
        match path.value() {
            Some(value) => {
                let absent: Vec<&str> = value
                    .split(':')
                    .filter(|directory| !Path::new(directory).is_dir())
                    .collect();
                if absent.is_empty() {
                    println!("{name}: {value}");
                } else {
                    missing += 1;
                    println!("{name}: {} isn't there", absent.join(", "));
                }
            }
            None => {
                missing += 1;
                println!("{name}: this build has no path for them");
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
