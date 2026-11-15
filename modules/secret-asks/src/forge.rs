//! What a forge itself says a repository's default branch holds, so a
//! standing yes covers only code the forge took (ISXEG8, 15 November).
//! A tree's own refs say what origin held when last fetched, and any
//! command of Joel's account can write them, so they're never asked:
//! the forge is, over the network, and its answer only names a commit,
//! whose objects must then be the tree's own, since objects can't be
//! forged as refs can.
//!
//! Only a forge the setting names is asked, at a URL whose rest holds
//! nothing but a path's plain letters, so a tree's origin can't send git
//! anywhere else or by any other means. git runs outside the tree, with
//! none of its settings or Joel's, so no `insteadOf` turns the URL into
//! another, and nothing of the tree's runs.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::tree;

/// How long a forge may take to answer before it's taken as saying
/// nothing, which covers no ask, since a row is still asked of Joel.
const ANSWERING: Duration = Duration::from_secs(10);
/// How long a forge's answer is kept, so a burst of asks from one
/// repository asks it once.
const KEPT: Duration = Duration::from_secs(30);
/// How often an unanswered forge is looked at again.
const POLL: Duration = Duration::from_millis(20);

/// The forges a standing yes may be checked against, and what each
/// repository's default branch was last said to hold.
pub struct Forge {
    git: PathBuf,
    /// Each forge's prefix, as `https://github.com/`; none covers no ask.
    prefixes: Vec<String>,
    said: Mutex<HashMap<String, (Instant, Option<String>)>>,
}

impl Forge {
    #[must_use]
    pub fn new(git: PathBuf, prefixes: Vec<String>) -> Forge {
        Forge {
            git,
            prefixes,
            said: Mutex::new(HashMap::new()),
        }
    }

    /// The URL of the tree at `path`'s origin, as a forge this names
    /// serves it; none where its origin is on none of them.
    #[must_use]
    pub fn origin(&self, path: &Path) -> Option<String> {
        let url = tree::origin(&self.git, path)?;
        let url = plain(&url);
        self.prefix_of(&url).is_some().then_some(url)
    }

    fn prefix_of(&self, url: &str) -> Option<&str> {
        self.prefixes
            .iter()
            .map(String::as_str)
            .find(|prefix| url.strip_prefix(prefix).is_some_and(plain_path))
    }

    /// The commit the forge says the repository at `url` has on its
    /// default branch, kept a while; none where it's on no forge named,
    /// or the forge didn't say in time.
    pub fn head(&self, url: &str) -> Option<String> {
        let prefix = self.prefix_of(url)?;
        let mut said = self.said.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((when, head)) = said.get(url)
            && when.elapsed() < KEPT
        {
            return head.clone();
        }
        let head = asked(&self.git, prefix, url);
        said.insert(url.to_owned(), (Instant::now(), head.clone()));
        head
    }
}

/// A URL as a forge's https form says it: `git@host:path` and
/// `ssh://git@host/path` as `https://host/path`, less one trailing `/`.
fn plain(url: &str) -> String {
    let url = url.trim();
    let url = url.strip_suffix('/').unwrap_or(url);
    if let Some(rest) = url.strip_prefix("ssh://") {
        let rest = rest.split_once('@').map_or(rest, |(_, rest)| rest);
        return format!("https://{rest}");
    }
    if !url.contains("://")
        && let Some((host, path)) = url.split_once(':')
    {
        let host = host.split_once('@').map_or(host, |(_, host)| host);
        return format!("https://{host}/{path}");
    }
    url.to_owned()
}

/// Whether a repository's path on a forge holds only a path's plain
/// letters, and never climbs out of where the forge's prefix says.
fn plain_path(path: &str) -> bool {
    !path.is_empty()
        && !path.split('/').any(|part| part.is_empty() || part == "..")
        && path
            .chars()
            .all(|letter| letter.is_ascii_alphanumeric() || "._/-".contains(letter))
}

/// What the forge at `url` says HEAD is, asked by the git at `git`
/// outside any repository, speaking only the prefix's own protocol.
fn asked(git: &Path, prefix: &str, url: &str) -> Option<String> {
    let scheme = prefix.split_once("://")?.0;
    let mut command = Command::new(git);
    command
        .env_clear()
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ALLOW_PROTOCOL", scheme)
        .env("HOME", "/var/empty")
        .current_dir("/")
        .args(["ls-remote", "--", url, "HEAD"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    // The certificates a machine names, for a forge over https.
    for name in ["SSL_CERT_FILE", "NIX_SSL_CERT_FILE"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    let mut child = command.spawn().ok()?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() < ANSWERING => std::thread::sleep(POLL),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let ran = child.wait_with_output().ok()?;
    let told = String::from_utf8_lossy(&ran.stdout);
    let head = told.lines().next()?.split_whitespace().next()?;
    (ran.status.success() && head.len() >= 40 && head.chars().all(|c| c.is_ascii_hexdigit()))
        .then(|| head.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ssh_origin_is_said_as_its_https_form() {
        assert_eq!(
            plain("git@github.com:Jitsusama/core.nix.git"),
            "https://github.com/Jitsusama/core.nix.git"
        );
        assert_eq!(
            plain("ssh://git@github.com/Jitsusama/core.nix/"),
            "https://github.com/Jitsusama/core.nix"
        );
        assert_eq!(
            plain("https://github.com/Jitsusama/core.nix"),
            "https://github.com/Jitsusama/core.nix"
        );
    }

    #[test]
    fn only_a_named_forges_plain_paths_are_asked() {
        let forge = Forge::new(PathBuf::from("git"), vec!["https://github.com/".to_owned()]);
        assert!(
            forge
                .prefix_of("https://github.com/Jitsusama/core.nix.git")
                .is_some(),
            "a repository on the named forge wasn't asked"
        );
        for elsewhere in [
            "https://github.com.evil/x",
            "https://github.com/../x",
            "https://github.com/a//b",
            "https://github.com/a?b",
            "https://github.com/",
            "file:///home/x",
        ] {
            assert!(
                forge.prefix_of(elsewhere).is_none(),
                "{elsewhere} was asked"
            );
        }
        let none = Forge::new(PathBuf::from("git"), Vec::new());
        assert!(
            none.prefix_of("https://github.com/a/b").is_none(),
            "a forge was asked though none is named"
        );
    }
}
