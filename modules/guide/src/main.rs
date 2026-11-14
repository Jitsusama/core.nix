//! `guide observe` says what the guide sees on this machine and changes
//! nothing; `guide tools` says where this build finds each program it
//! runs, and fails when one is missing. `guide host IMAGE` writes a stick
//! on whatever machine runs it, as the person: given `--sign-with` and
//! sbctl's db key directory, its boot program signed for that machine, and
//! for each machine whose directory follows another `--sign-with`; given
//! `--every-machine`, signed too for every other machine whose rescue
//! bundle the writer keeps, each bundle opened in turn;
//! given `--install` and a machine's payload (payload.nix), its vault
//! holding everything to install it, sealed to the stick's passphrase;
//! given `--rescue-bundle` too, the machine's Secure Boot keys, opened
//! from its bundle, sign the boot program and go into the vault, as the
//! bundle the writer keeps for the machine does when neither that nor
//! `--sign-with` is given, unless `--no-rescue-bundle` is.
//! `guide write-stick IMAGE FILE` does the same into a file the size of a
//! stick, the passphrases read from standard input. `guide seal-keys
//! TARGET BUNDLE` makes a machine's rescue bundle, on that machine, from
//! its payload's target.json. `guide install` is what the stick runs on
//! its console, as root, and `guide installed` finishes the machine once
//! it boots its own disk, as root on its first console at every boot.
//! `guide revive` puts macOS back on a Mac with Apple silicon, from a
//! stick carrying Apple's restore image, and `guide vault-list DIRECTORY`
//! reads back every file of a vault copied there, its passphrase on
//! standard input, and prints each one's SHA-256 as sha256sum does.
//!
//! Each command that runs in one place lives in commands/, in the file for
//! that place; those that run anywhere are here. Each takes the terminal
//! kit's shape, its help its own (`help()`), and a fault ends 1.

mod commands;

use std::path::Path;
use std::process::ExitCode;

use guide::observe::{disks, efi, entries, fido2, sbctl, tpm};
use guide::screen::Theme;
use guide::system::System;
use guide::vault;
use guide::vault::store::{Directory, Store};
use guide::{Fault, Journal, Tool};

use commands::{passphrase, reported};
use terminal_kit::{Exit, Failure, Help};

/// A command that needs only itself, as `guide --help` lists it.
fn plain(name: &str, about: &str, example: &str, failed: &str) -> Help {
    Help::new(name, about)
        .example(&format!("guide {name}"), example)
        .exit(Exit::Failed, failed)
}

/// The options a stick is written with, as host and write-stick take them.
fn written(help: Help) -> Help {
    help.option("--sign-with DIRECTORY", "sbctl's db keys, one per machine")
        .option("--install PAYLOAD", "a machine's payload, for its vault")
        .option("--rescue-bundle BUNDLE", "its keys, from its rescue bundle")
}

/// What `guide --help` says, and each command's own.
fn help() -> Help {
    Help::new("guide", "walks a machine from blank to sealed")
        .example("guide observe", "what the guide sees here")
        .example("guide host nixos.img", "writes a stick it finds")
        .subcommand(plain(
            "observe",
            "says what the guide sees, changing nothing",
            "Secure Boot, keys, the TPM and disks",
            "something couldn't be seen",
        ))
        .subcommand(plain(
            "tools",
            "says where this build finds each program",
            "each program's path",
            "a program isn't there",
        ))
        .subcommand(plain(
            "install",
            "installs this machine, from the stick",
            "as root, on the stick's console",
            "a step couldn't finish",
        ))
        .subcommand(plain(
            "installed",
            "finishes the machine on its own disk",
            "as root, at every boot",
            "a step couldn't finish",
        ))
        .subcommand(plain(
            "recover",
            "repairs this machine, from the stick",
            "as root, on the stick's console",
            "a step couldn't finish",
        ))
        .subcommand(plain(
            "revive",
            "puts macOS back on a Mac with Apple silicon",
            "from a stick with Apple's restore image",
            "macOS couldn't be put back",
        ))
        .subcommand(plain(
            "finish",
            "adds the account's keys, and the last steps",
            "once the machine is installed",
            "a step couldn't finish",
        ))
        .subcommand(
            Help::new("vault-list", "proves a copied vault, file by file")
                .example("guide vault-list /mnt/vault", "each file's SHA-256")
                .argument("<directory>", "the vault, its passphrase on stdin")
                .exit(Exit::Failed, "the vault couldn't be read back"),
        )
        .subcommand(
            Help::new("seal-keys", "makes this machine's rescue bundle")
                .example(
                    "guide seal-keys target.json rescue.json",
                    "its passphrase on stdin",
                )
                .argument("<target>", "the payload's target.json")
                .argument("<bundle>", "where the bundle is written")
                .exit(Exit::Failed, "the bundle couldn't be made"),
        )
        .subcommand(written(
            Help::new("host", "writes a stick, as the person")
                .example("guide host nixos.img", "onto a stick it finds")
                .example(
                    "guide host nixos.img --install payload",
                    "with a vault to install from",
                )
                .argument("<image>", "the stick's system image")
                .option("--every-machine", "signs for every machine kept")
                .option("--no-rescue-bundle", "leaves the machine's keys out")
                .exit(Exit::Failed, "the stick couldn't be written"),
        ))
        .subcommand(written(
            Help::new("write-stick", "writes a stick into a file")
                .example(
                    "guide write-stick nixos.img stick.raw",
                    "passphrases on stdin",
                )
                .argument("<image>", "the stick's system image")
                .argument("<file>", "the file, a stick's size")
                .exit(Exit::Failed, "the file couldn't be written"),
        ))
}

/// The words a command was given that it doesn't take.
fn unasked(command: &str) -> Failure {
    Failure::new(
        Exit::Usage,
        format!("guide {command} doesn't take those words"),
    )
    .try_this(format!("guide {command} --help"))
}

fn main() -> ExitCode {
    // Run by ssh-tpm-keygen as its SSH_ASKPASS, for the PIN of a key the
    // guide is making (src/installed/keys.rs), it answers that and only
    // that, whatever it's asked.
    if let Some(pin) = std::env::var_os(guide::installed::keys::ASKPASS) {
        use std::io::Write as _;
        let mut answer = pin.into_encoded_bytes();
        answer.push(b'\n');
        return match std::io::stdout().write_all(&answer) {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        };
    }
    // age finds its plugins on PATH and takes no other, so the guide's own
    // PATH is the one this build was given, and nothing a shell set can
    // put another plugin in its place. The programs the guide runs get
    // empty environments of their own (system.rs).
    if let Some(plugins) = guide::tool::Path::AgePlugins.value() {
        // SAFETY: the first thing main does, while no other thread is
        // there to read the environment.
        unsafe { std::env::set_var("PATH", plugins) };
    }
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        run(&context.args)
    })
}

/// The command these words name, run.
fn run(args: &[String]) -> Result<(), Failure> {
    match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["observe"] => observe(),
        ["tools"] => tools(),
        ["install"] => {
            let _awake = guide::awake::hold("installing this machine");
            reported(commands::stick::install())
        }
        ["installed"] => {
            let _awake = guide::awake::hold("finishing this machine");
            reported(commands::installed::installed())
        }
        ["recover"] => {
            let _awake = guide::awake::hold("recovering this machine");
            reported(commands::stick::recover())
        }
        ["revive"] => {
            let _awake = guide::awake::hold("reviving a Mac");
            reported(commands::stick::revive())
        }
        ["vault-list", directory] => reported(vault_list(directory)),
        ["seal-keys", target, bundle] => reported(commands::installed::seal_keys(target, bundle)),
        ["finish"] => reported(commands::installed::finish()),
        ["host", image, ref options @ ..] => commands::host::host(image, options),
        ["write-stick", image, target, ref options @ ..] => {
            commands::host::write_stick(image, target, options)
        }
        [command, ..] => Err(unasked(command)),
        [] => Err(Failure::new(Exit::Usage, "guide needs a command").try_this("guide --help")),
    }
}

/// Every file of a vault copied into a directory, read back whole and
/// proven, with its SHA-256, so a stick's vault can be shown to hold the
/// same files before it's written over and after.
fn vault_list(directory: &str) -> Result<(), Fault> {
    let passphrase = passphrase()?;
    let mut store = Directory::new(directory);
    let sealed = {
        let mut sealed = vec![];
        std::io::Read::read_to_end(&mut store.open(vault::KEY_PASSPHRASE)?, &mut sealed)
            .map_err(|why| Fault::unreadable(vault::KEY_PASSPHRASE, why))?;
        sealed
    };
    let key = vault::VaultKey::open_with_passphrase(&sealed, &passphrase)?;
    for file in vault::Vault::open(&mut store, key)?.read_back()? {
        println!("{}  {}", file.sha256, file.path);
    }
    Ok(())
}

fn tools() -> Result<(), Failure> {
    let mut missing = 0;
    for tool in Tool::built() {
        match tool.program() {
            Some(program) if Path::new(program).is_file() => println!("{tool}: {program}"),
            Some(program) => {
                missing += 1;
                println!("{tool}: {program} isn't there");
            }
            None if tool.optional() => println!("{tool}: not in this build, which needs none"),
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
    for path in guide::tool::Path::built() {
        let name = path.name();
        if let Some(value) = path.value() {
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
        } else {
            missing += 1;
            println!("{name}: this build has no path for them");
        }
    }
    if missing == 0 {
        Ok(())
    } else {
        Err(Failure::new(
            Exit::Failed,
            format!("{missing} of what this build needs isn't there"),
        ))
    }
}

/// Each observation on its own, so one a person can't make (sbctl's keys
/// need root) still leaves the rest to read.
fn observe() -> Result<(), Failure> {
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
        efi::SecureBoot::observe(&mut machine).map(|seen| secure_boot_said(seen.as_ref())),
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
        fido2::list(&mut machine).map(|tokens| fido2_said(&tokens)),
    );
    match disks::observe(&mut machine) {
        Ok(found) => {
            for disk in found {
                show(&disk.path, Ok(disk_said(&disk)));
            }
        }
        Err(fault) => show("Disks", Err(fault)),
    }

    if faults == 0 {
        Ok(())
    } else {
        Err(Failure::new(
            Exit::Failed,
            format!("{faults} of the observations couldn't be made"),
        ))
    }
}

/// Secure Boot as `guide observe` says it.
fn secure_boot_said(seen: Option<&efi::SecureBoot>) -> String {
    match seen {
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
    }
}

/// The FIDO2 keys plugged in, as `guide observe` says them.
fn fido2_said(tokens: &[fido2::Token]) -> String {
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
}

/// A disk as `guide observe` says it.
fn disk_said(disk: &disks::Disk) -> String {
    format!(
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
    )
}
