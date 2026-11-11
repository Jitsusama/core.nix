//! What runs on the installed machine: making its rescue bundle,
//! finishing it before anyone signs in, then finishing it online.

use std::cell::RefCell;
use std::io::IsTerminal;
use std::path::PathBuf;

use guide::install;
use guide::rescue::{Bundle, Keys, SealKeys};
use guide::screen::Theme;
use guide::screen::run::{Ended, Walk};
use guide::screen::terminal::Terminal;
use guide::system::System;
use guide::{Fault, Journal, Journey, Machine, Place, Tool};

use super::{own_state, passphrase, writer_log};

/// A machine's rescue bundle, made on that machine from its payload's
/// target.json: a walk on a terminal, or, when standard input is no
/// terminal, the passphrase read from its first line, for a test.
pub fn seal_keys(target: &str, bundle: &str) -> Result<(), Fault> {
    let target: install::Target = serde_json::from_slice(
        &std::fs::read(target).map_err(|why| Fault::unreadable(target, why))?,
    )
    .map_err(|why| Fault::unreadable("the payload's target.json", why))?;
    let mut machine = System::new(Journal::in_memory(), writer_log());
    if !std::io::stdin().is_terminal() {
        let passphrase = passphrase()?;
        let keys = Keys::read(&mut machine)?;
        let bytes = Bundle::seal(
            &target,
            &keys,
            &passphrase,
            &guide::rescue::rescue_passphrase(&target.machine),
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
        done: RefCell::default(),
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
pub fn installed() -> Result<(), Fault> {
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
    let mut machine = System::new(journal, writer_log());
    // At boot it's looked at every time, so a machine with nothing to do
    // shows nothing, and the sign-in follows at once.
    let journey = Journey::new(guide::installed::steps(&mut machine)?)
        .map_err(|why| Fault::unreadable("the installed machine's steps", why))?;
    if !guide::installed::takes_the_screen(&journey, &mut machine) {
        return Ok(());
    }
    let name = machine.read("/etc/hostname")?.map_or_else(
        || "this machine".into(),
        |bytes| String::from_utf8_lossy(&bytes).trim().to_string(),
    );
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
        done: RefCell::default(),
    };
    if walk.walk(&mut machine, &mut terminal)? != Ended::Finished {
        return Ok(());
    }
    // No restart: at boot the guide holds the boot back from counting as
    // good, so restarting from here would spend one of a new generation's
    // tries.
    let next = "From the next start, the disk asks for the TPM's PIN you just chose. A YubiKey with its own PIN, or the recovery key, still opens it whenever the TPM won't. Once you're signed in and online, type guide finish in a terminal, which adds this machine's keys to GitHub and walks you through what's left.";
    walk.card(
        &mut terminal,
        &format!("{name} is finished"),
        next,
        "sign in",
    )?;
    Ok(())
}

/// What's left once the person is signed in and online: the account's keys
/// in the TPM added to GitHub, after a sign-in on their phone, then the
/// steps the machine's repo adds. Run as the account, since it's their
/// GitHub, their gh and their session.
pub fn finish() -> Result<(), Fault> {
    // SAFETY: getuid can't fail and touches no memory.
    if unsafe { libc::getuid() } == 0 {
        return Err(Fault::Blocked {
            step: "finish",
            why: "it adds your own keys to your own GitHub, so run it as yourself, without sudo"
                .into(),
        });
    }
    let from = |name: &str| {
        std::env::var(name).map_err(|_| Fault::Blocked {
            step: "finish",
            why: format!("{name} isn't set, so run it from a terminal in your own session"),
        })
    };
    let account = guide::github::Account {
        name: from("USER")?,
        home: from("HOME")?,
        runtime: from("XDG_RUNTIME_DIR")?,
        bus: std::env::var("DBUS_SESSION_BUS_ADDRESS").ok(),
    };
    let mut machine = System::new(Journal::in_memory(), None);
    let added = guide::installed::facts(&mut machine)?.once_signed_in;
    let theirs = !added.is_empty();
    let journey = Journey::new(guide::finish::steps(&account, added))
        .map_err(|why| Fault::unreadable("what's left to finish", why))?;
    let name = machine.read("/etc/hostname")?.map_or_else(
        || "this machine".into(),
        |bytes| String::from_utf8_lossy(&bytes).trim().to_string(),
    );
    let title = format!("Finish {name} online");
    let theme = Theme::built(Terminal::is_console());
    let mut terminal = Terminal::open()?;
    let walk = Walk {
        journey: &journey,
        here: Place::Installed,
        name: &title,
        theme: &theme,
        log: None,
        whole: true,
        done: RefCell::default(),
    };
    if walk.walk(&mut machine, &mut terminal)? != Ended::Finished {
        return Ok(());
    }
    let said = if theirs {
        "GitHub takes this machine's SSH logins and shows the commits it signs as verified, and every step its repo adds is done."
    } else {
        "GitHub takes this machine's SSH logins, and shows the commits it signs as verified."
    };
    walk.card(
        &mut terminal,
        &format!("{name} is finished online"),
        said,
        "leave",
    )?;
    Ok(())
}
