//! Root's own copy of the repository, and what it checks of main's head
//! before anything is built from it: that it descends from what the machine
//! last switched to, and that a key root trusts signed it.

use crate::config::Config;
use crate::run::{self, Failure};
use std::fs;
use std::process::Command;

/// The ref main is kept at in root's copy.
const MAIN: &str = "refs/heads/main";

/// The one signature state that counts: good, by a key trusted. For SSH
/// that's a signer in the allowed-signers file, and `U` is a good signature
/// by anyone else. For PGP it's a key in root's keyring, each of which
/// is trusted ultimately as it's imported.
const GOOD: &str = "G";

/// Root's git, in root's copy, with nothing of anyone's configuration.
pub fn git(config: &Config) -> Command {
    let mut command = run::command(&config.tools.git, &config.state);
    command
        .env("GNUPGHOME", config.keyring())
        .arg("-C")
        .arg(config.repository_copy());
    command
}

/// Makes root's copy, empty, when there isn't one.
pub fn make_copy(config: &Config) -> Result<(), Failure> {
    let copy = config.repository_copy();
    if copy.join("HEAD").exists() {
        return Ok(());
    }
    fs::create_dir_all(&copy).map_err(|e| Failure::new(format!("{}: {e}", copy.display())))?;
    run::output(
        git(config),
        "making root's copy",
        ["init", "--quiet", "--bare"],
    )?;
    Ok(())
}

/// Whether root's copy can read `commit`.
pub fn has(config: &Config, commit: &str) -> bool {
    git(config)
        .args(["cat-file", "-e", &format!("{commit}^{{commit}}")])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Fetches main into root's copy, whole, and gives its head.
pub fn fetch(config: &Config) -> Result<String, Failure> {
    make_copy(config)?;
    let refspec = format!("+refs/heads/{}:{MAIN}", config.branch);
    run::output(
        git(config),
        "fetching main",
        [
            "fetch",
            "--quiet",
            "--no-tags",
            &config.repository,
            &refspec,
        ],
    )?;
    run::output(git(config), "reading main", ["rev-parse", "--verify", MAIN])
}

/// Whether `newer` descends from `older`, so main wasn't rewritten or
/// taken back past what the machine runs.
pub fn descends(config: &Config, older: &str, newer: &str) -> Result<bool, Failure> {
    let status = git(config)
        .args(["merge-base", "--is-ancestor", older, newer])
        .status()
        .map_err(|e| Failure::new(format!("checking main's history: {e}")))?;
    match status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(Failure::new(format!(
            "checking main's history failed ({status})"
        ))),
    }
}

/// Puts the PGP keys that count in root's keyring, made afresh each
/// time so a key taken out of the config stops counting.
pub fn keyring(config: &Config) -> Result<(), Failure> {
    let keyring = config.keyring();
    let _ = fs::remove_dir_all(&keyring);
    fs::create_dir_all(&keyring)
        .map_err(|e| Failure::new(format!("{}: {e}", keyring.display())))?;
    set_private(&keyring)?;
    for key in &config.signers.openpgp {
        let mut gpg = run::command(&config.tools.gpg, &config.state);
        gpg.env("GNUPGHOME", &keyring);
        run::output(
            gpg,
            "reading a signer's key",
            [
                "--batch".as_ref(),
                "--quiet".as_ref(),
                "--import".as_ref(),
                key.as_os_str(),
            ],
        )?;
    }
    if config.signers.openpgp.is_empty() {
        return Ok(());
    }
    // Every key here is one the config trusts, so each is trusted
    // ultimately, which is what makes its signatures good rather than of
    // unknown validity.
    let mut gpg = run::command(&config.tools.gpg, &config.state);
    gpg.env("GNUPGHOME", &keyring);
    let listed = run::output(
        gpg,
        "listing the signers' keys",
        ["--batch", "--with-colons", "--list-keys"],
    )?;
    let trust = listed
        .lines()
        .filter_map(|l| l.strip_prefix("fpr:::::::::"))
        .filter_map(|l| l.split(':').next())
        .fold(String::new(), |mut trust, fpr| {
            trust.push_str(fpr);
            trust.push_str(":6:\n");
            trust
        });
    let trusted = keyring.join("trust");
    fs::write(&trusted, trust).map_err(|e| Failure::new(format!("{}: {e}", trusted.display())))?;
    let mut gpg = run::command(&config.tools.gpg, &config.state);
    gpg.env("GNUPGHOME", &keyring);
    run::output(
        gpg,
        "trusting the signers' keys",
        [
            "--batch".as_ref(),
            "--quiet".as_ref(),
            "--import-ownertrust".as_ref(),
            trusted.as_os_str(),
        ],
    )?;
    Ok(())
}

fn set_private(dir: &std::path::Path) -> Result<(), Failure> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
        .map_err(|e| Failure::new(format!("{}: {e}", dir.display())))
}

/// Whether a key that counts signed `commit`, by what git says of it.
pub fn signed(config: &Config, commit: &str) -> Result<Result<(), String>, Failure> {
    let mut command = git(config);
    command
        .arg("-c")
        .arg(format!("gpg.program={}", config.tools.gpg.display()))
        .arg("-c")
        .arg(format!(
            "gpg.ssh.program={}",
            config.tools.ssh_keygen.display()
        ));
    let signers = config
        .signers
        .ssh
        .clone()
        .unwrap_or_else(|| "/dev/null".into());
    command
        .arg("-c")
        .arg(format!("gpg.ssh.allowedSignersFile={}", signers.display()));
    let said = run::output(
        command,
        "checking main's signature",
        ["log", "-1", "--format=%G?", commit],
    )?;
    if said == GOOD {
        Ok(Ok(()))
    } else {
        Ok(Err(format!(
            "{} isn't signed by a key this machine trusts (git says {said})",
            short(commit)
        )))
    }
}

/// The flake reference of `commit` in root's copy.
pub fn flake(config: &Config, commit: &str) -> String {
    format!(
        "git+file://{}?ref={MAIN}&rev={commit}#{}",
        config.repository_copy().display(),
        config.output
    )
}

/// A commit's first twelve characters, as answers name one to a person.
pub fn short(commit: &str) -> &str {
    &commit[..commit.len().min(12)]
}
