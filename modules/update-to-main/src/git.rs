//! Root's own copies of repositories, and what it checks of a main's head
//! before anything is built from it: that it descends from what the machine
//! last had, and that a key root trusts signed it. The machine's own
//! repository is one; each input that follows its own main is another.

use crate::config::{Config, Follow, Signers};
use crate::run::{self, Failure};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The ref main is kept at in root's copy.
const MAIN: &str = "refs/heads/main";

/// The one signature state that counts: good, by a key trusted. For SSH
/// that's a signer in the allowed-signers file, and `U` is a good signature
/// by anyone else. For PGP it's a key in root's keyring, each of which
/// is trusted ultimately as it's imported.
const GOOD: &str = "G";

/// A repository root keeps a bare copy of, and whose signatures count.
pub struct Repo<'a> {
    config: &'a Config,
    url: &'a str,
    branch: &'a str,
    signers: &'a Signers,
    copy: PathBuf,
    keyring: PathBuf,
    /// What answers call its main: main, or an input's.
    pub called: String,
}

impl<'a> Repo<'a> {
    /// The machine's own repository.
    pub fn main(config: &'a Config) -> Repo<'a> {
        Repo {
            config,
            url: &config.repository,
            branch: &config.branch,
            signers: &config.signers,
            copy: config.repository_copy(),
            keyring: config.keyring(),
            called: "main".to_owned(),
        }
    }

    /// The repository of the input the flake calls `name`.
    pub fn followed(config: &'a Config, name: &str, follow: &'a Follow) -> Repo<'a> {
        let folder = config.follows_folder();
        Repo {
            config,
            url: &follow.repository,
            branch: &follow.branch,
            signers: &follow.signers,
            copy: folder.join(format!("{name}.git")),
            keyring: folder.join(format!("{name}.gnupg")),
            called: format!("{name}'s main"),
        }
    }
}

/// Root's git, in root's copy, with nothing of anyone's configuration.
pub fn git(repo: &Repo) -> Command {
    let mut command = run::command(&repo.config.tools.git, &repo.config.state);
    command
        .env("GNUPGHOME", &repo.keyring)
        .arg("-C")
        .arg(&repo.copy);
    command
}

/// Makes root's copy, empty, when there isn't one.
pub fn make_copy(repo: &Repo) -> Result<(), Failure> {
    if repo.copy.join("HEAD").exists() {
        return Ok(());
    }
    fs::create_dir_all(&repo.copy)
        .map_err(|e| Failure::new(format!("{}: {e}", repo.copy.display())))?;
    run::output(
        git(repo),
        "making root's copy",
        ["init", "--quiet", "--bare"],
    )?;
    Ok(())
}

/// Whether root's copy can read `commit`.
pub fn has(repo: &Repo, commit: &str) -> bool {
    git(repo)
        .args(["cat-file", "-e", &format!("{commit}^{{commit}}")])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Fetches main into root's copy, whole, and gives its head.
pub fn fetch(repo: &Repo) -> Result<String, Failure> {
    make_copy(repo)?;
    let refspec = format!("+refs/heads/{}:{MAIN}", repo.branch);
    run::output(
        git(repo),
        &format!("fetching {}", repo.called),
        ["fetch", "--quiet", "--no-tags", repo.url, &refspec],
    )?;
    run::output(
        git(repo),
        &format!("reading {}", repo.called),
        ["rev-parse", "--verify", MAIN],
    )
}

/// Whether `newer` descends from `older`, so main wasn't rewritten or
/// taken back past what the machine runs. An `older` root's copy hasn't
/// got isn't in `newer`'s history, which it holds whole.
pub fn descends(repo: &Repo, older: &str, newer: &str) -> Result<bool, Failure> {
    if !has(repo, older) {
        return Ok(false);
    }
    let status = git(repo)
        .args(["merge-base", "--is-ancestor", older, newer])
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| Failure::new(format!("checking {}'s history: {e}", repo.called)))?;
    match status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(Failure::new(format!(
            "checking {}'s history failed ({status}): is {} in it?",
            repo.called,
            short(older)
        ))),
    }
}

/// The file at `path` in `commit`, or none when the commit hasn't one.
pub fn show(repo: &Repo, commit: &str, path: &str) -> Option<String> {
    let out = git(repo)
        .args(["show", &format!("{commit}:{path}")])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Puts the PGP keys that count in root's keyring, made afresh each
/// time so a key taken out of the config stops counting.
pub fn keyring(repo: &Repo) -> Result<(), Failure> {
    let config = repo.config;
    let keyring = &repo.keyring;
    let _ = fs::remove_dir_all(keyring);
    fs::create_dir_all(keyring).map_err(|e| Failure::new(format!("{}: {e}", keyring.display())))?;
    set_private(keyring)?;
    for key in &repo.signers.openpgp {
        let mut gpg = run::command(&config.tools.gpg, &config.state);
        gpg.env("GNUPGHOME", keyring);
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
    if repo.signers.openpgp.is_empty() {
        return Ok(());
    }
    // Every key here is one the config trusts, so each is trusted
    // ultimately, which is what makes its signatures good rather than of
    // unknown validity.
    let mut gpg = run::command(&config.tools.gpg, &config.state);
    gpg.env("GNUPGHOME", keyring);
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
    gpg.env("GNUPGHOME", keyring);
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

fn set_private(dir: &Path) -> Result<(), Failure> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
        .map_err(|e| Failure::new(format!("{}: {e}", dir.display())))
}

/// Whether a key that counts signed `commit`, by what git says of it.
pub fn signed(repo: &Repo, commit: &str) -> Result<Result<(), String>, Failure> {
    let tools = &repo.config.tools;
    let mut command = git(repo);
    command
        .arg("-c")
        .arg(format!("gpg.program={}", tools.gpg.display()))
        .arg("-c")
        .arg(format!("gpg.ssh.program={}", tools.ssh_keygen.display()));
    let signers = repo
        .signers
        .ssh
        .clone()
        .unwrap_or_else(|| "/dev/null".into());
    command
        .arg("-c")
        .arg(format!("gpg.ssh.allowedSignersFile={}", signers.display()));
    let said = run::output(
        command,
        &format!("checking {}'s signature", repo.called),
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

/// The flake reference of `commit` in root's copy, without an output.
pub fn reference(repo: &Repo, commit: &str) -> String {
    format!("git+file://{}?ref={MAIN}&rev={commit}", repo.copy.display())
}

/// The flake reference of the machine's system at `commit` in root's copy.
pub fn flake(config: &Config, commit: &str) -> String {
    format!(
        "{}#{}",
        reference(&Repo::main(config), commit),
        config.output
    )
}

/// A commit's first twelve characters, as answers name one to a person.
pub fn short(commit: &str) -> &str {
    &commit[..commit.len().min(12)]
}
