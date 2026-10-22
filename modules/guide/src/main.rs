//! `guide observe` says what the guide sees on this machine and changes
//! nothing; `guide tools` says where this build finds each program it
//! runs, and fails when one is missing. `guide host IMAGE` writes a stick
//! on whatever machine runs it, as the person: given `--sign-with` and
//! sbctl's db key directory, its boot program signed for that machine;
//! given `--install` and a machine's payload (payload.nix), its vault
//! holding everything to install it, sealed to a recovery passphrase;
//! given `--rescue-bundle` too, the machine's Secure Boot keys, opened
//! from its bundle, sign the boot program and go into the vault, as the
//! bundle the host keeps for the machine does when neither that nor
//! `--sign-with` is given, unless `--no-rescue-bundle` is.
//! `guide write-stick IMAGE FILE` does the same into a file the size of a
//! stick, the passphrases read from standard input. `guide seal-keys
//! TARGET BUNDLE` makes a machine's rescue bundle, on that machine, from
//! its payload's target.json. `guide install` is what the stick runs on
//! its console, as root, and `guide installed` finishes the machine once
//! it boots its own disk, as root on its first console at every boot.

use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;

use guide::host::carried;
use guide::host::{self, GatherBundles, Payload, WriteStick};
use guide::install;
use guide::observe::{disks, efi, entries, fido2, sbctl, tpm};
use guide::rescue::{Bundle, Keys, OpenRescueBundle, SealKeys};
use guide::screen::Theme;
use guide::screen::run::{Ended, Walk};
use guide::screen::terminal::Terminal;
use guide::sign::Signer;
use guide::system::System;
use guide::vault;
use guide::vault::store::{Directory, Fat, Store};
use guide::{Fault, Journal, Journey, Machine, Place, Secret, Step, Tool};

const USAGE: &str = "usage: guide observe | guide tools | guide install | guide installed | guide recover
       guide host IMAGE [--sign-with DIRECTORY] [--install PAYLOAD [--rescue-bundle BUNDLE | --no-rescue-bundle]]
       guide write-stick IMAGE FILE [--sign-with DIRECTORY] [--install PAYLOAD [--rescue-bundle BUNDLE]]
       guide seal-keys TARGET BUNDLE";

fn main() -> ExitCode {
    // age finds its plugins on PATH and takes no other, so the guide's own
    // PATH is the one this build was given, and nothing a shell set can
    // put another plugin in its place. The programs the guide runs get
    // empty environments of their own (system.rs).
    if let Some(plugins) = guide::tool::Path::AgePlugins.value() {
        // SAFETY: the first thing main does, while no other thread is
        // there to read the environment.
        unsafe { std::env::set_var("PATH", plugins) };
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["observe"] => observe(),
        ["tools"] => tools(),
        ["install"] => reported(install()),
        ["installed"] => reported(installed()),
        ["recover"] => reported(recover()),
        ["seal-keys", target, bundle] => reported(seal_keys(target, bundle)),
        ["host", image, ref options @ ..] => match Written::parse(options) {
            Some(written) => reported(written.open().and_then(|(signer, payload, bundle)| {
                host(image, signer, payload, bundle, written.uses_kept())
            })),
            None => {
                eprintln!("{USAGE}");
                ExitCode::from(2)
            }
        },
        ["write-stick", image, target, ref options @ ..] => match Written::parse(options) {
            Some(written) => reported(written.open().and_then(|(signer, payload, bundle)| {
                write_stick(image, target, signer, payload, bundle)
            })),
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

/// What a stick is written with, each given at most once: the db key
/// directory its boot program is signed with, the machine's payload, and
/// the machine's rescue bundle, whose keys the stick carries for a
/// reinstall and signs with when no other key is named.
#[derive(Default)]
struct Written<'a> {
    sign_with: Option<&'a str>,
    install: Option<&'a str>,
    rescue_bundle: Option<&'a str>,
    /// Whether no bundle the host keeps may be used.
    no_rescue_bundle: bool,
}

impl<'a> Written<'a> {
    fn parse(mut options: &[&'a str]) -> Option<Written<'a>> {
        let mut written = Written::default();
        while !options.is_empty() {
            if let ["--no-rescue-bundle", rest @ ..] = options {
                if std::mem::replace(&mut written.no_rescue_bundle, true) {
                    return None;
                }
                options = rest;
                continue;
            }
            let [option, value, rest @ ..] = options else {
                return None;
            };
            let slot = match *option {
                "--sign-with" => &mut written.sign_with,
                "--install" => &mut written.install,
                "--rescue-bundle" => &mut written.rescue_bundle,
                _ => return None,
            };
            if slot.replace(*value).is_some() {
                return None;
            }
            options = rest;
        }
        let bundled = written.rescue_bundle.is_some() || written.no_rescue_bundle;
        (!bundled || written.install.is_some())
            .then_some(written)
            .filter(|written| !(written.rescue_bundle.is_some() && written.no_rescue_bundle))
    }

    /// Whether the bundle the host keeps for the machine signs its stick:
    /// only when no bundle or key is named, nor none asked for.
    fn uses_kept(&self) -> bool {
        !self.no_rescue_bundle && self.sign_with.is_none() && self.rescue_bundle.is_none()
    }

    /// Each as it was given, the bundle read but not yet opened, since
    /// opening it asks for a YubiKey or the passphrase.
    fn open(&self) -> Result<Opened, Fault> {
        let signer = self
            .sign_with
            .map(|directory| Signer::open(Path::new(directory)))
            .transpose()?;
        let payload = self
            .install
            .map(|payload| Payload::open(Path::new(payload)))
            .transpose()?;
        let bundle = self
            .rescue_bundle
            .map(|path| {
                std::fs::read(path)
                    .map_err(|why| Fault::unreadable(path, why))
                    .and_then(|bytes| Bundle::read(&bytes))
            })
            .transpose()?;
        Ok((signer, payload, bundle))
    }
}

type Opened = (Option<Signer>, Option<Payload>, Option<Bundle>);

/// The keys a rescue bundle opened to: they sign the stick unless another
/// key was named, and the payload carries them.
fn carrying(
    signer: Option<Signer>,
    payload: Option<Payload>,
    machine: &str,
    keys: Keys,
) -> Result<(Signer, Option<Payload>), Fault> {
    let signer = match signer {
        Some(signer) => signer,
        None => keys.signer()?,
    };
    let payload = payload
        .map(|payload| payload.with_secure_boot_keys(machine, keys))
        .transpose()?;
    Ok((signer, payload))
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

/// A recovery passphrase, as the next line of standard input.
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
    signer: Option<Signer>,
    payload: Option<Payload>,
    bundle: Option<Bundle>,
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
    let passphrase = payload.as_ref().map(|_| passphrase()).transpose()?;
    // The bundle's passphrase follows the vault's, on the next line.
    let (signer, payload) = match bundle {
        Some(bundle) => {
            let keys = bundle.open_with_passphrase(&self::passphrase()?)?;
            let (signer, payload) = carrying(signer, payload, bundle.machine(), keys)?;
            (Some(signer), payload)
        }
        None => (signer, payload),
    };
    let vault = host::write(
        Path::new(image),
        &mut disk,
        signer.as_ref(),
        payload.as_ref().zip(passphrase.as_ref()),
        &[],
        &guide::progress::Meter::new(),
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
    // A service's own, which systemd names for it.
    if let Some(state) = std::env::var_os("STATE_DIRECTORY") {
        return Some(PathBuf::from(state));
    }
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
fn host(
    image: &str,
    signer: Option<Signer>,
    payload: Option<Payload>,
    bundle: Option<Bundle>,
    uses_kept: bool,
) -> Result<(), Fault> {
    // SAFETY: getuid can't fail and touches no memory.
    if unsafe { libc::getuid() } == 0 {
        return Err(Fault::Blocked {
            step: "write-stick",
            why: "a stick is written as yourself, not root: the system asks for what it needs"
                .into(),
        });
    }
    let machine_name = payload
        .as_ref()
        .map(|payload| payload.target.machine.clone());
    let title = match &machine_name {
        Some(name) => format!("Write {name}'s stick"),
        None => "Write a repair stick".into(),
    };
    let mut machine = System::new(Journal::in_memory(), host_log());
    let theme = Theme::built(Terminal::is_console());
    let mut terminal = Terminal::open()?;

    // The rescue bundles the stick carries kept first, before anything
    // writes over them, where the person's own state is.
    let kept = own_state().map(|state| state.join("rescue"));
    let gathering = Journey::new(match &kept {
        Some(kept) => vec![Box::new(GatherBundles::new(kept.clone())) as Box<dyn Step>],
        None => vec![],
    })
    .map_err(|why| Fault::unreadable("the host's steps", why))?;
    let mut walk = Walk {
        journey: &gathering,
        here: Place::Host,
        name: &title,
        theme: &theme,
        log: None,
        whole: false,
        done: Default::default(),
    };
    if kept.is_some() && walk.walk(&mut machine, &mut terminal)? != Ended::Finished {
        return Ok(());
    }
    let bundles = kept
        .as_ref()
        .map(|kept| carried::kept(&mut Directory::new(kept)))
        .unwrap_or_default();
    // A machine whose bundle the host keeps has its stick signed with it,
    // unless another, a key, or none was given.
    let given = bundle.is_some();
    let bundle = if uses_kept {
        carried::bundle_for(bundle, machine_name.as_deref(), &bundles)?
    } else {
        bundle
    };
    // One the person didn't name that isn't opened says where it is, and
    // how to write the stick without it.
    let unopened = |machine: &str| match (given, &kept) {
        (false, Some(kept)) => Err(Fault::Blocked {
            step: "write-stick",
            why: format!(
                "the rescue bundle this host keeps for {machine}, {}, wasn't opened; to write the stick without it, run this again with --no-rescue-bundle",
                kept.join(guide::rescue::carried_as(machine)).display()
            ),
        }),
        _ => Ok(()),
    };

    // The rescue bundle opened next, its keys held only in memory.
    let opened = Rc::new(RefCell::new(None));
    let bundle_of = bundle.as_ref().map(|bundle| bundle.machine().to_string());
    let steps: Vec<Box<dyn Step>> = match bundle {
        Some(bundle) => vec![Box::new(OpenRescueBundle {
            bundle,
            opened: Rc::clone(&opened),
        })],
        None => vec![],
    };
    let opening = Journey::new(steps).map_err(|why| Fault::unreadable("the host's steps", why))?;
    walk.journey = &opening;
    let (signer, payload) = match bundle_of {
        Some(machine_of) => {
            // The terminal is put back as it was before this is said.
            if walk.walk(&mut machine, &mut terminal)? != Ended::Finished {
                return unopened(&machine_of);
            }
            let Some(keys) = opened.take() else {
                return unopened(&machine_of);
            };
            let (signer, payload) = carrying(signer, payload, &machine_of, keys)?;
            (Some(signer), payload)
        }
        None => (signer, payload),
    };

    let next = match &payload {
        Some(payload) => payload.next(),
        None => "Start the machine to repair from the stick.".into(),
    };
    // Only a host that gathered bundles carries them, and then writes over
    // only the stick it gathered them from.
    let write = WriteStick::new(PathBuf::from(image), payload, signer);
    let write = match &kept {
        Some(_) => write.carrying(bundles),
        None => write,
    };
    let journey = Journey::new(vec![Box::new(write) as Box<dyn Step>])
        .map_err(|why| Fault::unreadable("the host's steps", why))?;
    walk.journey = &journey;
    walk.whole = true;
    if walk.walk(&mut machine, &mut terminal)? != Ended::Finished {
        return Ok(());
    }
    walk.card(&mut terminal, "The stick is written", &next, "finish")?;
    Ok(())
}

/// A machine's rescue bundle, made on that machine from its payload's
/// target.json: a walk on a terminal, or, when standard input is no
/// terminal, the passphrase read from its first line, for a test.
fn seal_keys(target: &str, bundle: &str) -> Result<(), Fault> {
    let target: install::Target = serde_json::from_slice(
        &std::fs::read(target).map_err(|why| Fault::unreadable(target, why))?,
    )
    .map_err(|why| Fault::unreadable("the payload's target.json", why))?;
    let mut machine = System::new(Journal::in_memory(), host_log());
    if !std::io::stdin().is_terminal() {
        let passphrase = passphrase()?;
        let keys = Keys::read(&mut machine)?;
        let bytes = Bundle::seal(
            &target,
            &keys,
            &passphrase,
            guide::rescue::RECOVERY_PASSPHRASE,
            None,
        )?;
        machine.write(bundle, &bytes)?;
        println!(
            "{bundle}: {}'s rescue bundle, owned by {}",
            target.machine,
            keys.owner()
        );
        return Ok(());
    }
    let title = format!("Make {}'s rescue bundle", target.machine);
    let journey = Journey::new(vec![Box::new(SealKeys::new(target, PathBuf::from(bundle)))])
        .map_err(|why| Fault::unreadable("the rescue bundle's steps", why))?;
    let theme = Theme::built(Terminal::is_console());
    let mut terminal = Terminal::open()?;
    let walk = Walk {
        journey: &journey,
        here: Place::Installed,
        name: &title,
        theme: &theme,
        log: None,
        whole: true,
        done: Default::default(),
    };
    if walk.walk(&mut machine, &mut terminal)? != Ended::Finished {
        return Ok(());
    }
    walk.card(
        &mut terminal,
        "The rescue bundle is made",
        &format!(
            "Write this machine's stick with {bundle}, and the stick is signed for its firmware and carries its keys, so a reinstall never visits the firmware's setup. The stick carries the bundle on from there."
        ),
        "finish",
    )?;
    Ok(())
}

/// The installed machine's journey, before anyone signs in there: Secure
/// Boot checked, then the disk sealed to the TPM, whose PIN the disk's
/// prompt asks for from the next start.
fn installed() -> Result<(), Fault> {
    // Everything here needs root. Core's guide module runs it as root at
    // boot; run by hand, run0 asks for it once, on this terminal, before
    // the guide takes the screen. Asked for each program instead, run0
    // couldn't ask at all: the guide holds their input and output.
    // SAFETY: geteuid can't fail and touches no memory.
    if unsafe { libc::geteuid() } != 0 {
        use std::os::unix::process::CommandExt;
        let run0 = Tool::Run0.program().ok_or(Fault::Unbuilt {
            tool: Tool::Run0.name(),
        })?;
        let unstarted = |why: std::io::Error| Fault::Unstarted {
            program: run0.into(),
            why: why.to_string(),
        };
        let guide = std::env::current_exe().map_err(unstarted)?;
        // An empty background keeps run0 from tinting the guide's colours.
        let why = std::process::Command::new(run0)
            .arg("--background=")
            .arg(guide)
            .arg("installed")
            .exec();
        return Err(unstarted(why));
    }
    // What's proven here has to outlive the restarts in between.
    let journal = match own_state() {
        Some(state) => Journal::open(&state.join("journal"))?,
        None => Journal::in_memory(),
    };
    let mut machine = System::new(journal, host_log());
    // At boot it's looked at every time, so a machine with nothing to do
    // shows nothing, and the sign-in follows at once.
    let journey = Journey::new(guide::installed::steps())
        .map_err(|why| Fault::unreadable("the installed machine's steps", why))?;
    if matches!(
        journey.next(Place::Installed, &mut machine),
        Ok(guide::Next::Finished)
    ) {
        return Ok(());
    }
    let name = machine
        .read("/etc/hostname")?
        .map(|bytes| String::from_utf8_lossy(&bytes).trim().to_string())
        .unwrap_or_else(|| "this machine".into());
    let title = format!("Finish {name}");
    let theme = Theme::built(Terminal::is_console());
    let mut terminal = Terminal::open()?;
    let walk = Walk {
        journey: &journey,
        here: Place::Installed,
        name: &title,
        theme: &theme,
        log: None,
        whole: true,
        done: Default::default(),
    };
    if walk.walk(&mut machine, &mut terminal)? != Ended::Finished {
        return Ok(());
    }
    // No restart: at boot the guide holds the boot back from counting as
    // good, so restarting from here would spend one of a new generation's
    // tries.
    let next = "From the next start, the disk asks for the TPM's PIN. A YubiKey or the recovery key still opens it whenever the TPM won't.";
    walk.card(
        &mut terminal,
        &format!("{name} is finished"),
        next,
        "sign in",
    )?;
    Ok(())
}

/// Recovering the machine this stick boots on, its data left in place:
/// its disk opened and mounted, then a root shell inside it.
fn recover() -> Result<(), Fault> {
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
            done: Default::default(),
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
                Some(at @ 1..=3) => {
                    let steps: Vec<Box<dyn guide::Step>> = match at {
                        1 => vec![Box::new(guide::recover::BootGeneration)],
                        2 => vec![
                            Box::new(guide::recover::IdentifyKeptKey),
                            Box::new(guide::recover::ReplaceLostKey),
                        ],
                        _ => vec![Box::new(guide::recover::InstallBootLoader)],
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
                        done: Default::default(),
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

    // One stick for everything: on a disk that already holds a machine, it
    // asks first whether to recover it or install over it.
    if let Some(question) = guide::recover::already_there(&mut machine, &machine_name)? {
        let chosen = {
            let mut terminal = Terminal::open()?;
            let ask = Journey::new(vec![])
                .map_err(|why| Fault::unreadable("the install's steps", why))?;
            Walk {
                journey: &ask,
                here: Place::Stick,
                name: &title,
                theme: &theme,
                log: Some(Path::new(LOG)),
                whole: false,
                done: Default::default(),
            }
            .choose(
                &mut terminal,
                "A machine is already here",
                &question,
                guide::recover::ALREADY_THERE,
            )?
        };
        match chosen {
            Some(0) => {
                drop(machine);
                return recover();
            }
            Some(_) => {}
            None => return Ok(()),
        }
    }
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
        done: Default::default(),
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
        "Press Return to shut down. Take the stick out, then turn {} on: its disk asks for a security token PIN, so type a YubiKey's PIN and touch the key when it blinks. The guide then finishes {} before you sign in.",
        target.machine, target.machine
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

#[cfg(test)]
mod tests {
    use super::Written;

    fn uses_kept(options: &[&str]) -> Option<bool> {
        Written::parse(options).map(|written| written.uses_kept())
    }

    #[test]
    fn the_bundle_the_host_keeps_is_used_only_when_nothing_else_is_named() {
        assert_eq!(uses_kept(&["--install", "p"]), Some(true));
        assert_eq!(uses_kept(&[]), Some(true));
        assert_eq!(
            uses_kept(&["--install", "p", "--no-rescue-bundle"]),
            Some(false)
        );
        assert_eq!(
            uses_kept(&["--install", "p", "--rescue-bundle", "b"]),
            Some(false)
        );
        assert_eq!(
            uses_kept(&["--sign-with", "d", "--install", "p"]),
            Some(false)
        );
    }

    #[test]
    fn a_bundle_and_none_together_or_either_without_a_payload_is_refused() {
        for options in [
            &[
                "--install",
                "p",
                "--rescue-bundle",
                "b",
                "--no-rescue-bundle",
            ][..],
            &["--no-rescue-bundle"],
            &["--rescue-bundle", "b"],
            &["--install", "p", "--no-rescue-bundle", "--no-rescue-bundle"],
            &["--install"],
        ] {
            assert_eq!(uses_kept(options), None, "{options:?}");
        }
    }
}
