//! Writing a stick on the machine the person writes it from, as the
//! person rather than root: `guide host` onto a stick it finds and has
//! them confirm, and `guide write-stick` into a file the size of one.

use std::cell::RefCell;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;

use guide::rescue::{Bundle, Keys, OpenRescueBundle};
use guide::screen::Theme;
use guide::screen::run::{Ended, Walk};
use guide::screen::terminal::Terminal;
use guide::sign::Signer;
use guide::system::System;
use guide::vault::store::Directory;
use guide::writer::carried;
use guide::writer::{self, GatherBundles, Payload, WriteStick};
use guide::{Fault, Journal, Journey, Machine, Place, Step};

use super::{own_state, passphrase, reported, writer_log};

/// `guide host IMAGE`, or nothing when its options aren't ones it takes.
pub fn host(image: &str, options: &[&str]) -> Option<ExitCode> {
    let written = Written::parse(options)?;
    let _awake = guide::awake::hold("writing a stick");
    // Rooted before anything is read from them, and held until the writer
    // is done, whichever way it ends.
    Some(reported(
        hold_what_is_written(image, written.install).and_then(|held| {
            let wrote = written.open().and_then(|(signers, payload, bundle)| {
                writer(
                    image,
                    signers,
                    payload,
                    bundle,
                    written.uses_kept(),
                    written.every_machine,
                )
            });
            drop(held);
            wrote
        }),
    ))
}

/// `guide write-stick IMAGE FILE`, or nothing when its options aren't
/// ones it takes.
pub fn write_stick(image: &str, target: &str, options: &[&str]) -> Option<ExitCode> {
    // It gathers no bundles, so it has none to sign for.
    let written = Written::parse(options).filter(|written| !written.every_machine)?;
    let _awake = guide::awake::hold("writing a stick");
    Some(reported(written.open().and_then(
        |(signers, payload, bundle)| into_file(image, target, signers, payload, bundle),
    )))
}

/// What a stick is written with: the db key directories its boot program
/// is signed with, one for each machine it's to boot on, and, each given
/// at most once, the machine's payload and the machine's rescue bundle,
/// whose keys the stick carries for a reinstall and signs with when no
/// other key is named.
#[derive(Default)]
struct Written<'a> {
    sign_with: Vec<&'a str>,
    install: Option<&'a str>,
    rescue_bundle: Option<&'a str>,
    /// Whether no bundle the writer keeps may be used.
    no_rescue_bundle: bool,
    /// Whether the stick is signed for every machine whose bundle the
    /// writer keeps, besides any it's signed for already.
    every_machine: bool,
}

impl<'a> Written<'a> {
    fn parse(mut options: &[&'a str]) -> Option<Written<'a>> {
        let mut written = Written::default();
        while !options.is_empty() {
            if let [flag @ ("--no-rescue-bundle" | "--every-machine"), rest @ ..] = options {
                let set = if *flag == "--every-machine" {
                    &mut written.every_machine
                } else {
                    &mut written.no_rescue_bundle
                };
                if std::mem::replace(set, true) {
                    return None;
                }
                options = rest;
                continue;
            }
            let [option, value, rest @ ..] = options else {
                return None;
            };
            if *option == "--sign-with" {
                written.sign_with.push(value);
                options = rest;
                continue;
            }
            let slot = match *option {
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

    /// Whether the bundle the writer keeps for the machine signs its stick:
    /// only when no bundle or key is named, nor none asked for.
    fn uses_kept(&self) -> bool {
        !self.no_rescue_bundle && self.sign_with.is_empty() && self.rescue_bundle.is_none()
    }

    /// Each as it was given, the bundle read but not yet opened, since
    /// opening it asks for a YubiKey or the passphrase.
    fn open(&self) -> Result<Opened, Fault> {
        let signers = self
            .sign_with
            .iter()
            .map(|directory| Signer::open(Path::new(directory)))
            .collect::<Result<Vec<_>, _>>()?;
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
        Ok((signers, payload, bundle))
    }
}

type Opened = (Vec<Signer>, Option<Payload>, Option<Bundle>);

/// The keys a rescue bundle opened to: they sign the stick unless other
/// keys were named, and the payload carries them.
fn carrying(
    mut signers: Vec<Signer>,
    payload: Option<Payload>,
    machine: &str,
    keys: Keys,
) -> Result<(Vec<Signer>, Option<Payload>), Fault> {
    if signers.is_empty() {
        signers.push(keys.signer()?);
    }
    let payload = payload
        .map(|payload| payload.with_secure_boot_keys(machine, keys))
        .transpose()?;
    Ok((signers, payload))
}

/// A stick, into a file already the size of the stick it stands for. A
/// disk is written through `guide host`, which finds the stick and has
/// the person confirm it.
fn into_file(
    image: &str,
    target: &str,
    signers: Vec<Signer>,
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
    let (signers, payload) = match bundle {
        Some(bundle) => {
            let keys = bundle.open_with_passphrase(&self::passphrase()?)?;
            carrying(signers, payload, bundle.machine(), keys)?
        }
        None => (signers, payload),
    };
    let vault = writer::write(
        Path::new(image),
        &mut disk,
        &signers,
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

/// The stick's image and the payload, kept in the Nix store for as long as
/// the writer runs, by roots in the person's own state for the guide.
fn hold_what_is_written(image: &str, payload: Option<&str>) -> Result<writer::roots::Held, Fault> {
    let Some(state) = own_state() else {
        return Ok(writer::roots::Held::default());
    };
    let folder = state.join("roots");
    std::os::unix::fs::DirBuilderExt::mode(std::fs::DirBuilder::new().recursive(true), 0o700)
        .create(&folder)
        .map_err(|why| Fault::Unkept {
            path: folder.display().to_string(),
            why: why.to_string(),
        })?;
    let mut machine = System::new(Journal::in_memory(), writer_log());
    let paths: Vec<&str> = std::iter::once(image).chain(payload).collect();
    writer::roots::hold(&mut machine, &folder, &paths)
}

/// The writer's steps as a journey, or why they aren't one.
fn writer_journey(steps: Vec<Box<dyn Step>>) -> Result<Journey, Fault> {
    Journey::new(steps).map_err(|why| Fault::unreadable("the writer's steps", why))
}

/// The writer's journey: a stick written for a machine, or a repair stick,
/// on the screen of whatever machine writes it, as the person, not root.
fn writer(
    image: &str,
    signers: Vec<Signer>,
    payload: Option<Payload>,
    bundle: Option<Bundle>,
    uses_kept: bool,
    every_machine: bool,
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
    let mut machine = System::new(Journal::in_memory(), writer_log());
    let theme = Theme::built(Terminal::is_console());
    let mut terminal = Terminal::open()?;

    // The rescue bundles the stick carries kept first, before anything
    // writes over them, where the person's own state is.
    let kept = own_state().map(|state| state.join("rescue"));
    let gather = kept.clone().map(GatherBundles::new);
    let phrase = gather.as_ref().map(|gather| Rc::clone(&gather.phrase));
    let gathering = writer_journey(match gather {
        Some(gather) => vec![Box::new(gather) as Box<dyn Step>],
        None => vec![],
    })?;
    let mut walk = Walk {
        journey: &gathering,
        here: Place::Writer,
        name: &title,
        theme: &theme,
        log: None,
        whole: false,
        done: RefCell::default(),
    };
    if kept.is_some() && walk.walk(&mut machine, &mut terminal)? != Ended::Finished {
        return Ok(());
    }
    let bundles = kept
        .as_ref()
        .map(|kept| carried::kept(&mut Directory::new(kept)))
        .unwrap_or_default();
    // A machine whose bundle the writer keeps has its stick signed with it,
    // unless another, a key, or none was given.
    let given = bundle.is_some();
    let bundle = if uses_kept {
        carried::bundle_for(bundle, machine_name.as_deref(), &bundles)?
    } else {
        bundle
    };
    let unopened = |machine: &str| unopened(given, kept.as_deref(), machine);

    // The rescue bundle opened next, its keys held only in memory.
    let opening = bundle.map(opening).transpose()?;
    let own = match &opening {
        Some((_, _, machine_of)) => Some(machine_of.as_str()),
        None => machine_name.as_deref(),
    };
    let (mut signers, payload) = match &opening {
        Some((journey, opened, machine_of)) => {
            walk.journey = journey;
            // The terminal is put back as it was before this is said.
            if walk.walk(&mut machine, &mut terminal)? != Ended::Finished {
                return unopened(machine_of);
            }
            let Some(keys) = opened.take() else {
                return unopened(machine_of);
            };
            carrying(signers, payload, machine_of, keys)?
        }
        None => (signers, payload),
    };

    // Every other machine whose bundle the writer keeps, when asked, signing
    // after the machine's own, so that one's signature stays the first.
    let others = if every_machine {
        others(&bundles, own)?
    } else {
        vec![]
    };
    signers.extend(signed_by_others(
        &mut walk,
        &others,
        &mut machine,
        &mut terminal,
    )?);

    // Named as an installed machine names itself; a Mac has no
    // /etc/hostname, and isn't a machine a stick is written for.
    let here = machine
        .read("/etc/hostname")?
        .map(|name| String::from_utf8_lossy(&name).into_owned());
    let next = match &payload {
        Some(payload) => payload.next(here.as_deref()),
        None => "Start the machine to repair from the stick.".into(),
    };
    // Only a writer that gathered bundles carries them, and then writes over
    // only the stick it gathered them from.
    let write = WriteStick::new(PathBuf::from(image), payload, signers);
    let write = match &kept {
        Some(_) => write.carrying(bundles),
        None => write,
    };
    let sealed = phrase.and_then(|phrase| phrase.take());
    let journey = writer_journey(writer::writing(write, sealed))?;
    walk.journey = &journey;
    walk.whole = true;
    if walk.walk(&mut machine, &mut terminal)? != Ended::Finished {
        return Ok(());
    }
    walk.card(&mut terminal, "The stick is written", &next, "finish")?;
    Ok(())
}

/// What a bundle not opened leaves: one the person didn't name says where
/// it is, and how to write the stick without it; one they named, nothing.
fn unopened(given: bool, kept: Option<&Path>, machine: &str) -> Result<(), Fault> {
    match (given, kept) {
        (false, Some(kept)) => Err(Fault::Blocked {
            step: "write-stick",
            why: format!(
                "the rescue bundle this writer keeps for {machine}, {}, wasn't opened; to write the stick without it, run this again with --no-rescue-bundle",
                kept.join(guide::rescue::carried_as(machine)).display()
            ),
        }),
        _ => Ok(()),
    }
}

/// A walk opening a machine's rescue bundle, the keys it opens to, and
/// the machine's name.
type Opening = (Journey, Rc<RefCell<Option<Keys>>>, String);

fn opening(bundle: Bundle) -> Result<Opening, Fault> {
    let machine = bundle.machine().to_string();
    let opened = Rc::new(RefCell::new(None));
    let step = OpenRescueBundle {
        bundle,
        opened: Rc::clone(&opened),
    };
    let journey = writer_journey(vec![Box::new(step) as Box<dyn Step>])?;
    Ok((journey, opened, machine))
}

/// A walk opening each bundle kept for a machine other than `own`.
fn others(bundles: &[carried::Carried], own: Option<&str>) -> Result<Vec<Opening>, Fault> {
    bundles
        .iter()
        .filter(|carried| own != Some(carried.machine.as_str()))
        .map(|carried| opening(Bundle::read(&carried.bytes)?))
        .collect()
}

/// The keys of each other machine's bundle, opened one after another.
/// One not opened stops the write, and says how to write it without
/// that machine.
fn signed_by_others<'a>(
    walk: &mut Walk<'a>,
    others: &'a [Opening],
    machine: &mut dyn Machine,
    terminal: &mut Terminal,
) -> Result<Vec<Signer>, Fault> {
    let mut signers = vec![];
    for (opening, opened, other) in others {
        walk.journey = opening;
        let finished = walk.walk(machine, terminal)? == Ended::Finished;
        let Some(keys) = opened.take().filter(|_| finished) else {
            return Err(Fault::Blocked {
                step: "write-stick",
                why: format!(
                    "the rescue bundle this writer keeps for {other} wasn't opened; to write the stick without signing it for {other}, run this again without --every-machine"
                ),
            });
        };
        signers.push(keys.signer()?);
    }
    Ok(signers)
}

#[cfg(test)]
mod tests {
    use super::Written;

    fn uses_kept(options: &[&str]) -> Option<bool> {
        Written::parse(options).map(|written| written.uses_kept())
    }

    #[test]
    fn the_bundle_the_writer_keeps_is_used_only_when_nothing_else_is_named() {
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
    fn every_machine_is_a_flag_given_once_that_leaves_the_kept_bundle_in_use() {
        let every = |options: &[&str]| Written::parse(options).map(|written| written.every_machine);
        assert_eq!(every(&["--every-machine"]), Some(true));
        assert_eq!(every(&["--install", "p", "--every-machine"]), Some(true));
        assert_eq!(every(&["--install", "p"]), Some(false));
        assert_eq!(every(&["--every-machine", "--every-machine"]), None);
        assert_eq!(
            uses_kept(&["--every-machine", "--install", "p"]),
            Some(true)
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
