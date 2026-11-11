//! What the stick runs on its console, as root: the install, recovering
//! a machine with its data left in place, and reviving a Mac. Each keeps
//! its journal and log in the stick's memory.

use std::cell::RefCell;
use std::fs::OpenOptions;
use std::path::Path;

use guide::install;
use guide::recover::AlreadyThere;
use guide::screen::Theme;
use guide::screen::run::{Ended, Walk};
use guide::screen::terminal::Terminal;
use guide::system::System;
use guide::vault;
use guide::vault::store::{Fat, Store};
use guide::{Fault, Journal, Journey, Machine, Place, Step, Tool};

/// macOS put back on a Mac with Apple silicon from the stick, as root, in a
/// walk of its own (src/revive.rs).
pub fn revive() -> Result<(), Fault> {
    // SAFETY: getuid can't fail and touches no memory.
    if unsafe { libc::getuid() } != 0 {
        return Err(Fault::Blocked {
            step: "revive",
            why: "reviving a Mac runs as root, on the stick".into(),
        });
    }
    let journal = Journal::open(Path::new(JOURNAL))?;
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG)
        .map_err(|why| Fault::Unkept {
            path: LOG.into(),
            why: why.to_string(),
        })?;
    let mut machine = System::new(journal, Some(log));
    let theme = Theme::built(Terminal::is_console());
    let journey = Journey::new(guide::revive::steps())
        .map_err(|why| Fault::unreadable("the revive's steps", why))?;
    let mut terminal = Terminal::open()?;
    let walk = Walk {
        journey: &journey,
        here: Place::Stick,
        name: "Revive a Mac",
        theme: &theme,
        log: Some(Path::new(LOG)),
        whole: true,
        done: RefCell::default(),
    };
    walk.walk(&mut machine, &mut terminal)?;
    Ok(())
}

/// Recovering the machine this stick boots on, its data left in place:
/// its disk opened and mounted, then a root shell inside it.
pub fn recover() -> Result<(), Fault> {
    // SAFETY: getuid can't fail and touches no memory.
    if unsafe { libc::getuid() } != 0 {
        return Err(Fault::Blocked {
            step: "recover",
            why: "recovering runs as root, on the stick".into(),
        });
    }
    let journal = Journal::open(Path::new(JOURNAL))?;
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG)
        .map_err(|why| Fault::Unkept {
            path: LOG.into(),
            why: why.to_string(),
        })?;
    let mut machine = System::new(journal, Some(log));
    let theme = Theme::built(Terminal::is_console());
    let journey = Journey::new(guide::recover::steps())
        .map_err(|why| Fault::unreadable("the recovery's steps", why))?;
    let shell = {
        let mut terminal = Terminal::open()?;
        let walk = Walk {
            journey: &journey,
            here: Place::Stick,
            name: "Recover a machine",
            theme: &theme,
            log: Some(Path::new(LOG)),
            whole: true,
            done: RefCell::default(),
        };
        if walk.walk(&mut machine, &mut terminal)? != Ended::Finished {
            return Ok(());
        }
        // A repair at a time, back here after each, until the shell or a
        // restart.
        loop {
            let question = "What should be done? A root shell inside it has its own tools: nixos-rebuild, sbctl, systemd-cryptenroll.";
            match walk.choose(
                &mut terminal,
                "The machine's disk is open",
                question,
                guide::recover::REPAIRS,
            )? {
                Some(0) => break true,
                Some(at @ 1..=4) => {
                    let steps: Vec<Box<dyn Step>> = match at {
                        1 => {
                            // Chosen again, it copies again, onto the
                            // same drive or another.
                            machine.remove(guide::recover::copy::COPIED)?;
                            vec![Box::new(guide::recover::copy::CopyHome)]
                        }
                        2 => vec![Box::new(guide::recover::boot::BootGeneration)],
                        3 => vec![
                            Box::new(guide::recover::keys::IdentifyKeptKey),
                            Box::new(guide::recover::keys::ReplaceLostKey),
                        ],
                        _ => vec![Box::new(guide::recover::boot::InstallBootLoader)],
                    };
                    let repair = Journey::new(steps)
                        .map_err(|why| Fault::unreadable("the repair's steps", why))?;
                    let chosen = Walk {
                        journey: &repair,
                        here: Place::Stick,
                        name: "Recover a machine",
                        theme: &theme,
                        log: Some(Path::new(LOG)),
                        whole: false,
                        done: RefCell::default(),
                    };
                    if chosen.walk(&mut machine, &mut terminal)? != Ended::Finished {
                        return Ok(());
                    }
                    *walk.done.borrow_mut() = chosen.done.take();
                }
                Some(_) => {
                    machine
                        .run(Tool::Umount, &["--recursive", install::ROOT])?
                        .ok()?;
                    machine.run(Tool::Systemctl, &["reboot"])?.ok()?;
                    return Ok(());
                }
                None => return Ok(()),
            }
        }
    };
    // The terminal is given back before the shell takes it.
    if shell {
        use std::os::unix::process::CommandExt;
        let enter = Tool::NixosEnter.program().ok_or(Fault::Unbuilt {
            tool: Tool::NixosEnter.name(),
        })?;
        let why = std::process::Command::new(enter)
            .args(["--root", install::ROOT])
            .env("PATH", guide::tool::Path::Install.get()?)
            .exec();
        return Err(Fault::Unstarted {
            program: enter.into(),
            why: why.to_string(),
        });
    }
    Ok(())
}

/// Where the stick keeps its journal and log: memory, gone when it stops.
const JOURNAL: &str = "/run/guide/journal";
const LOG: &str = "/run/guide/log";

/// What the person chose to do with what the disk already holds.
enum Chosen {
    Leave,
    Recover,
    Install(install::Start),
}

/// One stick for everything: on a disk that already holds a machine, it
/// asks first whether to recover it or install over it, and on one holding
/// this machine's install stopped part way, whether to carry it on or
/// start again. With nothing there, it installs afresh without asking.
fn what_is_there(
    machine: &mut System,
    machine_name: &str,
    title: &str,
    theme: &Theme,
) -> Result<Chosen, Fault> {
    let Some(found) = guide::recover::already_there(machine, machine_name)? else {
        return Ok(Chosen::Install(install::Start::Afresh));
    };
    let (heading, question, choices) = match &found {
        AlreadyThere::Machine { question } => (
            "A machine is already here",
            question,
            guide::recover::ALREADY_THERE,
        ),
        AlreadyThere::Stopped { question, .. } => (
            "An install stopped part way",
            question,
            guide::recover::STOPPED,
        ),
        AlreadyThere::StoppedEarly { question } => (
            "An install stopped part way",
            question,
            guide::recover::STOPPED_EARLY,
        ),
    };
    let chosen = {
        let mut terminal = Terminal::open()?;
        let ask =
            Journey::new(vec![]).map_err(|why| Fault::unreadable("the install's steps", why))?;
        Walk {
            journey: &ask,
            here: Place::Stick,
            name: title,
            theme,
            log: Some(Path::new(LOG)),
            whole: false,
            done: RefCell::default(),
        }
        .choose(&mut terminal, heading, question, choices)?
    };
    Ok(match (chosen, found) {
        (None, _) => Chosen::Leave,
        (Some(0), AlreadyThere::Machine { .. }) => Chosen::Recover,
        // What the stopped start knew, but for the steps true only of it,
        // such as its system running from memory; the disk taken as
        // confirmed, since carrying on wipes nothing.
        (Some(0), AlreadyThere::Stopped { disk, facts, .. }) => {
            let opening: Vec<&str> = install::opening().iter().map(|step| step.id()).collect();
            machine.journal().learn(&facts, &opening)?;
            machine.journal().record("confirm-disk", &disk)?;
            Chosen::Install(install::Start::CarryingOn)
        }
        (Some(_), AlreadyThere::Machine { .. }) => Chosen::Install(install::Start::Again),
        (Some(_), _) => Chosen::Install(install::Start::Afresh),
    })
}

/// The install, on the stick's console: the vault opened, then every step
/// it holds for the machine, then the stick taken out and the machine
/// restarted.
pub fn install() -> Result<(), Fault> {
    // SAFETY: getuid can't fail and touches no memory.
    if unsafe { libc::getuid() } != 0 {
        return Err(Fault::Blocked {
            step: "install",
            why: "the install runs as root, on the stick".into(),
        });
    }
    // Only read through this handle: what the install changes in the vault
    // goes through one of its own, written back before the step ends.
    let partition = OpenOptions::new()
        .read(true)
        .open(install::STICK_VAULT)
        .map_err(|why| Fault::unreadable("the stick's vault", why))?;
    let mut vault: Box<dyn Store> = Box::new(Fat::open(partition)?);
    let machine_name = vault::manifest(vault.as_mut())?.machine;
    // A repair stick is no one machine's and installs nothing.
    if machine_name.is_empty() {
        return recover();
    }
    let journal = Journal::open(Path::new(JOURNAL))?;
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG)
        .map_err(|why| Fault::Unkept {
            path: LOG.into(),
            why: why.to_string(),
        })?;
    let mut machine = System::new(journal, Some(log))
        .with_vault(vault)
        .with_vault_partition(install::STICK_VAULT);
    let theme = Theme::built(Terminal::is_console());
    let title = format!("Install {machine_name}");

    let start = match what_is_there(&mut machine, &machine_name, &title, &theme)? {
        Chosen::Leave => return Ok(()),
        Chosen::Recover => {
            drop(machine);
            return recover();
        }
        Chosen::Install(start) => start,
    };
    let mut terminal = Terminal::open()?;

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
        done: RefCell::default(),
    };
    if walk.walk(&mut machine, &mut terminal)? != Ended::Finished {
        return Ok(());
    }
    let target = install::target(&mut machine)?;
    let journey = Journey::new(install::steps(&target, start))
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
    let next = first_start(&target.machine, &target.yubikeys, target.secure_boot);
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

/// What the first start from the disk asks, in the order it asks it: on
/// that start only a YubiKey opens the disk, since the TPM is sealed after.
fn first_start(machine: &str, yubikeys: &[String], secure_boot: bool) -> String {
    let plug = format!(
        "Press Return to shut down. Take the stick out, plug in {}, then turn {machine} on. When its disk asks for a security token PIN, type that key's PIN, and touch the key when it blinks.",
        guide::rescue::either(yubikeys)
    );
    // The TPM's seal counts on Secure Boot, so a machine without it opens
    // its disk with a YubiKey or the recovery words at every start.
    if secure_boot {
        format!(
            "{plug} The guide then finishes {machine} before you sign in, and asks for a PIN of the TPM's own, which opens the disk from the next start with no key plugged in."
        )
    } else {
        format!(
            "{plug} It asks the same at every start, or for the recovery words when no key is plugged in."
        )
    }
}

#[cfg(test)]
mod tests {
    use super::first_start;

    #[test]
    fn the_first_start_says_to_plug_in_a_key_and_that_after_it_the_pin_alone_will_do() {
        // A card that said only "asks for a security token PIN" left which
        // key, and when, for the person to guess.
        let said = first_start("laptop", &["the Nano".into(), "the 5Ci".into()], true);
        let out = said.find("Take the stick out").unwrap();
        let plug = said.find("plug in the Nano or the 5Ci").unwrap();
        let on = said.find("then turn laptop on").unwrap();
        assert!(out < plug && plug < on, "{said}");
        assert!(
            said.ends_with(
                "asks for a PIN of the TPM's own, which opens the disk from the next start with no key plugged in."
            ),
            "{said}"
        );
    }

    #[test]
    fn a_machine_without_secure_boot_is_told_its_disk_asks_every_start() {
        // It has nothing to seal to, so a card promising the TPM's PIN
        // would leave the person waiting for a prompt that never comes.
        let said = first_start("laptop", &["the Nano".into()], false);
        assert!(said.contains("plug in the Nano"), "{said}");
        assert!(!said.contains("TPM"), "{said}");
        assert!(
            said.ends_with(
                "It asks the same at every start, or for the recovery words when no key is plugged in."
            ),
            "{said}"
        );
    }
}
