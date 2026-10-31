//! Main on a host that doesn't sign it. Root asks the host itself over TLS
//! which commit main is, with a token the account gives, so the answer is
//! the host's and a bad token is only refused. It reads that commit's
//! objects from the account's checkout through git's alternates, never its
//! configuration, and git checks them against the commit the host named.
//! It unpacks the flake's folder from them and builds it, after the account
//! has fetched the inputs only it can reach, each of which the store holds
//! to the hash its lock names.

use crate::config::{Config, Hosted};
use crate::git;
use crate::run::{self, Account, Failure};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

/// The variable the locked input is given to Nix in, rather than in its
/// arguments.
const LOCKED: &str = "UPDATE_TO_MAIN_LOCKED";

/// Fetches a locked input into the store, by its lock.
const FETCH: &str =
    "(builtins.fetchTree (builtins.fromJSON (builtins.getEnv \"UPDATE_TO_MAIN_LOCKED\"))).outPath";

/// Asks the host for main's head, and makes sure root's copy can read it
/// from the account's checkout, fetching it there as the account when it
/// isn't there yet.
pub fn head(config: &Config, hosted: &Hosted) -> Result<String, Failure> {
    let account = run::account(&hosted.user)?;
    git::make_copy(config)?;
    let token = token(hosted, &account)?;
    let head = ask(config, &token)?;
    borrow(config, hosted, &account)?;
    if !git::has(config, &head) {
        fetch(config, hosted, &account)?;
        if !git::has(config, &head) {
            return Err(Failure::new(format!(
                "{} isn't in {} even after fetching main there",
                git::short(&head),
                hosted.checkout.display()
            )));
        }
    }
    Ok(head)
}

/// Unpacks the flake's folder at `commit`, fetches its private inputs as
/// the account, and gives the flake reference to build.
pub fn prepare(config: &Config, hosted: &Hosted, commit: &str) -> Result<String, Failure> {
    let account = run::account(&hosted.user)?;
    let folder = unpack(config, hosted, commit)?;
    if !hosted.inputs.is_empty() {
        let lock = folder.join("flake.lock");
        let text = fs::read_to_string(&lock)
            .map_err(|e| Failure::new(format!("{}: {e}", lock.display())))?;
        let lock: Value = serde_json::from_str(&text)
            .map_err(|e| Failure::new(format!("reading the flake's lock: {e}")))?;
        for name in &hosted.inputs {
            fetch_input(config, hosted, &account, name, locked(&lock, name)?)?;
        }
    }
    Ok(format!("path:{}#{}", folder.display(), config.output))
}

fn token(hosted: &Hosted, account: &Account) -> Result<String, Failure> {
    let (program, args) = hosted
        .token
        .split_first()
        .ok_or_else(|| Failure::new("no command gives a token"))?;
    let mut command = run::command_as(program.as_ref(), account, &hosted.path, &hosted.checkout);
    command.args(args);
    let said = run::output(command, "asking for a token", std::iter::empty::<&str>())?;
    let token = said.lines().last().unwrap_or_default().trim().to_owned();
    if token.is_empty() {
        return Err(Failure::new("asking for a token gave none"));
    }
    Ok(token)
}

/// Main's head as the host names it. The token goes to git in its
/// environment, which only root reads, rather than in its arguments.
fn ask(config: &Config, token: &str) -> Result<String, Failure> {
    let mut command = git::git(config);
    command
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "http.extraHeader")
        .env(
            "GIT_CONFIG_VALUE_0",
            format!("Authorization: Bearer {token}"),
        );
    let main = format!("refs/heads/{}", config.branch);
    let said = run::output(
        command,
        "asking the host for main",
        ["ls-remote", "--refs", &config.repository, &main],
    )?;
    said.lines()
        .find_map(|line| {
            let (commit, name) = line.split_once('\t')?;
            (name == main && commit.len() == 40 && commit.bytes().all(|b| b.is_ascii_hexdigit()))
                .then(|| commit.to_owned())
        })
        .ok_or_else(|| Failure::new(format!("the host names no {main}")))
}

/// Points root's copy at the checkout's objects, as the account finds them.
fn borrow(config: &Config, hosted: &Hosted, account: &Account) -> Result<(), Failure> {
    let mut command = run::command_as(
        config.tools.git.as_os_str(),
        account,
        &hosted.path,
        &hosted.checkout,
    );
    command.env("GIT_CONFIG_NOSYSTEM", "1");
    let common = run::output(
        command,
        "finding the checkout's objects",
        ["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    let objects = Path::new(&common).join("objects");
    if !objects.is_absolute() || !objects.is_dir() {
        return Err(Failure::new(format!(
            "{} holds no objects",
            hosted.checkout.display()
        )));
    }
    let alternates = config.repository_copy().join("objects/info/alternates");
    fs::write(&alternates, format!("{}\n", objects.display()))
        .map_err(|e| Failure::new(format!("{}: {e}", alternates.display())))
}

/// Fetches main into the checkout as the account, with its own git and
/// logins, moving none of its branches.
fn fetch(config: &Config, hosted: &Hosted, account: &Account) -> Result<(), Failure> {
    let command = run::command_as("git".as_ref(), account, &hosted.path, &hosted.checkout);
    run::output(
        command,
        "fetching main into the checkout",
        [
            "fetch",
            "--quiet",
            "--no-tags",
            &config.repository,
            &format!("refs/heads/{}", config.branch),
        ],
    )?;
    Ok(())
}

/// Unpacks the flake's folder at `commit` into a folder of its own, the
/// last one's taken away first.
fn unpack(config: &Config, hosted: &Hosted, commit: &str) -> Result<PathBuf, Failure> {
    let source = config.source();
    let _ = fs::remove_dir_all(&source);
    let into = source.join(commit);
    fs::create_dir_all(&into).map_err(|e| Failure::new(format!("{}: {e}", into.display())))?;
    let mut archive = git::git(config);
    archive.args(["archive", "--format=tar", commit]);
    if !hosted.directory.is_empty() {
        archive.args(["--", &hosted.directory]);
    }
    let mut archive = archive
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Failure::new(format!("unpacking main: {e}")))?;
    let tar = archive
        .stdout
        .take()
        .ok_or_else(|| Failure::new("unpacking main: git gave nothing"))?;
    let mut unpack = run::command(&config.tools.tar, &config.state);
    unpack.arg("-x").arg("-C").arg(&into).stdin(tar);
    let unpacked = run::output(unpack, "unpacking main", std::iter::empty::<&str>());
    let archived = archive
        .wait_with_output()
        .map_err(|e| Failure::new(format!("unpacking main: {e}")))?;
    if !archived.status.success() {
        return Err(Failure {
            why: format!("reading main's files failed ({})", archived.status),
            lines: run::tail(&String::from_utf8_lossy(&archived.stderr)),
        });
    }
    unpacked?;
    let folder = into.join(&hosted.directory);
    if !folder.join("flake.nix").is_file() {
        return Err(Failure::new(format!(
            "main at {} has no flake in {}",
            git::short(commit),
            if hosted.directory.is_empty() {
                "its top"
            } else {
                &hosted.directory
            }
        )));
    }
    Ok(folder)
}

/// An input's lock, by its name among the flake's own inputs.
fn locked<'a>(lock: &'a Value, name: &str) -> Result<&'a Value, Failure> {
    let missing = || Failure::new(format!("the flake's lock has no input {name}"));
    let root = lock["root"].as_str().ok_or_else(missing)?;
    let node = lock["nodes"][root]["inputs"][name]
        .as_str()
        .ok_or_else(missing)?;
    let locked = &lock["nodes"][node]["locked"];
    if locked.is_object() {
        Ok(locked)
    } else {
        Err(missing())
    }
}

/// Puts an input in the store as the account, by its lock: a path already
/// in the store is substituted, and anything else fetched to its hash.
fn fetch_input(
    config: &Config,
    hosted: &Hosted,
    account: &Account,
    name: &str,
    locked: &Value,
) -> Result<(), Failure> {
    let nix = config.tools.nix.as_os_str();
    let mut command = run::command_as(nix, account, &hosted.path, &hosted.checkout);
    let what = format!("fetching the input {name}");
    let in_store = locked["type"] == "path"
        && locked["path"]
            .as_str()
            .is_some_and(|p| p.starts_with("/nix/store/"));
    if in_store {
        let path = locked["path"].as_str().unwrap_or_default();
        run::output(
            command,
            &what,
            [
                "--extra-experimental-features",
                "nix-command",
                "build",
                "--no-link",
                path,
            ],
        )?;
        return Ok(());
    }
    command.env(LOCKED, locked.to_string());
    run::output(
        command,
        &what,
        [
            "--extra-experimental-features",
            "nix-command flakes",
            "eval",
            "--raw",
            "--impure",
            "--expr",
            FETCH,
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lock() -> Value {
        serde_json::json!({
            "root": "root",
            "nodes": {
                "root": { "inputs": { "private": "private_2", "followed": ["a", "b"] } },
                "private_2": { "locked": { "type": "git", "url": "https://example.com/p", "rev": "abc" } },
            },
            "version": 7,
        })
    }

    #[test]
    fn an_input_is_found_by_the_name_the_flake_gives_it() {
        let lock = lock();
        assert_eq!(
            locked(&lock, "private").unwrap()["url"],
            "https://example.com/p"
        );
    }

    #[test]
    fn an_input_that_follows_another_or_isnt_there_isnt_found() {
        let lock = lock();
        assert!(locked(&lock, "followed").is_err());
        assert!(locked(&lock, "nowhere").is_err());
    }

    #[test]
    fn the_fetch_reads_the_variable_the_lock_is_given_in() {
        assert!(FETCH.contains(LOCKED));
    }
}
