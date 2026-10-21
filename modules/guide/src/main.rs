//! `guide observe` says what the guide sees on this machine and changes
//! nothing; `guide tools` says where this build finds each program it
//! runs, and fails when one is missing. `guide host IMAGE` writes a stick
//! on whatever machine runs it, as the person: given `--sign-with` and
//! sbctl's db key directory, its boot program signed for that machine;
//! given `--install` and a machine's payload (payload.nix), its vault
//! holding everything to install it, sealed to a recovery passphrase.
//! `guide write-stick IMAGE FILE` does the same into a file the size of a
//! stick, the passphrase read from standard input. `guide install` is
//! what the stick runs on its console, as root, and `guide installed`
//! finishes the machine once it boots its own disk, as the person.

use std::fs::{File, OpenOptions};
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use guide::host::{self, Payload, WriteStick};
use guide::install;
use guide::observe::{disks, efi, entries, fido2, sbctl, tpm};
use guide::screen::Theme;
use guide::screen::run::{Ended, Walk};
use guide::screen::terminal::Terminal;
use guide::sign::Signer;
use guide::system::System;
use guide::vault;
use guide::vault::store::{Fat, Store};
use guide::{Fault, Journal, Journey, Machine, Place, Secret, Tool};

const USAGE: &str = "usage: guide observe | guide tools | guide install | guide installed
       guide host IMAGE [--sign-with DIRECTORY] [--install PAYLOAD]
       guide write-stick IMAGE FILE [--sign-with DIRECTORY] [--install PAYLOAD]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["observe"] => observe(),
        ["tools"] => tools(),
        ["install"] => reported(install()),
        ["installed"] => reported(installed()),
        ["host", image, ref options @ ..] => match written(options) {
            Some((keys, payload)) => reported(
                keys.map(|keys| Signer::open(Path::new(keys)))
                    .transpose()
                    .and_then(|signer| host(image, signer, payload)),
            ),
            None => {
                eprintln!("{USAGE}");
                ExitCode::from(2)
            }
        },
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
/// disk is written through `guide host`, which finds the stick and has
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
            why: format!("{target} isn't a file; a stick is written by guide host"),
        });
    }
    let mut disk = OpenOptions::new()
        .read(true)
        .write(true)
        .open(target)
        .map_err(kept)?;
    let payload = payload
        .map(|payload| Payload::open(Path::new(payload)))
        .transpose()?;
    let passphrase = payload.as_ref().map(|_| passphrase()).transpose()?;
    let vault = host::write(
        Path::new(image),
        &mut disk,
        signer,
        payload.as_ref().zip(passphrase.as_ref()),
    )?;
    match &payload {
        Some(payload) => println!(
            "{target}: {}'s install in a {} MB vault, read back as written",
            payload.target.machine,
            vault / 1_000_000
        ),
        None => println!(
            "{target}: the image and a {} MB vault, read back as written",
            vault / 1_000_000
        ),
    }
    Ok(())
}

/// The person's own state for the guide, made readable only by them.
fn own_state() -> Option<PathBuf> {
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| Path::new(&home).join(".local/state")))?;
    Some(state.join("guide"))
}

/// Where the host keeps its log: the person's own state, readable only by
/// them.
fn host_log() -> Option<File> {
    let directory = own_state()?;
    std::os::unix::fs::DirBuilderExt::mode(std::fs::DirBuilder::new().recursive(true), 0o700)
        .create(&directory)
        .ok()?;
    std::os::unix::fs::OpenOptionsExt::mode(OpenOptions::new().create(true).append(true), 0o600)
        .open(directory.join("log"))
        .ok()
}

/// The host's journey: a stick written for a machine, or a repair stick,
/// on the screen of whatever machine writes it, as the person, not root.
fn host(image: &str, signer: Option<Signer>, payload: Option<&str>) -> Result<(), Fault> {
    // SAFETY: getuid can't fail and touches no memory.
    if unsafe { libc::getuid() } == 0 {
        return Err(Fault::Blocked {
            step: "write-stick",
            why: "a stick is written as yourself, not root: the system asks for what it needs"
                .into(),
        });
    }
    let payload = payload
        .map(|payload| Payload::open(Path::new(payload)))
        .transpose()?;
    let machine_name = payload
        .as_ref()
        .map(|payload| payload.target.machine.clone());
    let title = match &machine_name {
        Some(name) => format!("Write {name}'s stick"),
        None => "Write a repair stick".into(),
    };
    let journey = Journey::new(vec![Box::new(WriteStick {
        image: PathBuf::from(image),
        payload,
        signer,
    })])
    .map_err(|why| Fault::unreadable("the host's steps", why))?;
    let mut machine = System::new(Journal::in_memory(), host_log());
    let theme = Theme::built(Terminal::is_console());
    let mut terminal = Terminal::open()?;
    let walk = Walk {
        journey: &journey,
        here: Place::Host,
        name: &title,
        theme: &theme,
        log: None,
        whole: true,
    };
    if walk.walk(&mut machine, &mut terminal)? != Ended::Finished {
        return Ok(());
    }
    let next = match &machine_name {
        Some(name) => format!(
            "Take the stick to {name} and start it from the stick. The guide carries on there, and asks for the recovery passphrase first."
        ),
        None => "Start the machine to repair from the stick.".into(),
    };
    walk.card(&mut terminal, "The stick is written", &next, "finish")?;
    Ok(())
}

/// The installed machine's journey, as the person who signs in there:
/// Secure Boot checked, then the disk sealed to the TPM, then a restart to
/// meet the TPM's PIN at the disk's prompt.
fn installed() -> Result<(), Fault> {
    // SAFETY: getuid can't fail and touches no memory.
    if unsafe { libc::getuid() } == 0 {
        return Err(Fault::Blocked {
            step: "finish",
            why: "the guide runs as yourself here, and asks for root where it needs it".into(),
        });
    }
    // What's proven here has to outlive the restarts in between.
    let journal = match own_state() {
        Some(state) => Journal::open(&state.join("journal"))?,
        None => Journal::in_memory(),
    };
    let mut machine = System::new(journal, host_log());
    let name = machine
        .read("/etc/hostname")?
        .map(|bytes| String::from_utf8_lossy(&bytes).trim().to_string())
        .unwrap_or_else(|| "this machine".into());
    let title = format!("Finish {name}");
    let journey = Journey::new(guide::installed::steps())
        .map_err(|why| Fault::unreadable("the installed machine's steps", why))?;
    let theme = Theme::built(Terminal::is_console());
    let mut terminal = Terminal::open()?;
    let walk = Walk {
        journey: &journey,
        here: Place::Installed,
        name: &title,
        theme: &theme,
        log: None,
        whole: true,
    };
    if walk.walk(&mut machine, &mut terminal)? != Ended::Finished {
        return Ok(());
    }
    let next = "Restart, and the disk asks for the TPM's PIN from now on. A YubiKey or the recovery key still opens it whenever the TPM won't.";
    if walk.card(
        &mut terminal,
        &format!("{name} is finished"),
        next,
        "restart",
    )? {
        machine.run(Tool::Systemctl, &["reboot"])?.ok()?;
    }
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
        println!("the theme: {absent} isn't in this build");
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
