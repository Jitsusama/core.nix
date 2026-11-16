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
//!
//! GitHub alone, over https alone, is asked with the account's own
//! credential, as gh keeps it, so a private repository's main can be
//! asked too (ISXEG8, 15 November). gh runs from its store path with
//! none of the account's environment, so no helper the account's git
//! settings name runs; and a token gh were made to say would only ask
//! GitHub as someone else, whose answer over TLS is still GitHub's.
//! What this leans on: gh runs here, outside the fence, and reads its
//! config.yml and hosts.yml, which the account can write. No setting
//! there makes `gh auth token` run a program today, since it uses no
//! pager, editor or browser; a gh that changed that would bite here.

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
/// The one forge given the account's credential, and only over https.
const CREDITED: &str = "https://github.com/";
/// The longest token GitHub gives, which is far shorter.
const LONGEST_TOKEN: usize = 255;

/// The forges a standing yes may be checked against, and what each
/// repository's default branch was last said to hold.
pub struct Forge {
    git: PathBuf,
    /// Each forge's prefix, as `https://github.com/`; none covers no ask.
    prefixes: Vec<String>,
    /// The gh whose token GitHub is asked with, where there's one.
    gh: Option<PathBuf>,
    said: Mutex<HashMap<String, (Instant, Option<String>)>>,
}

impl Forge {
    #[must_use]
    pub fn new(git: PathBuf, prefixes: Vec<String>, gh: Option<PathBuf>) -> Forge {
        Forge {
            git,
            prefixes,
            gh,
            said: Mutex::new(HashMap::new()),
        }
    }

    /// The header the forge of `prefix` is asked with: the account's
    /// GitHub credential, for GitHub over https alone, where gh says one.
    fn credential(&self, prefix: &str) -> Option<String> {
        if prefix != CREDITED {
            return None;
        }
        let token = token(self.gh.as_deref()?)?;
        Some(format!(
            "Authorization: Basic {}",
            base64(format!("x-access-token:{token}").as_bytes())
        ))
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
        let head = asked(&self.git, prefix, url, self.credential(prefix).as_deref());
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

/// What a forge's name is said as: GitHub's as its own, any other's as
/// its host, and one with none, as a file URL, as "its forge".
#[must_use]
pub fn called(url: &str) -> String {
    let host = url
        .split_once("://")
        .map_or(url, |(_, rest)| rest)
        .split('/')
        .next()
        .unwrap_or_default();
    match host {
        "github.com" => "GitHub".to_owned(),
        "" => "its forge".to_owned(),
        host => host.to_owned(),
    }
}

/// The token gh says is the account's for GitHub, asked of the gh at
/// `gh` with only what it needs to reach the keyring it keeps it in;
/// none where it says none in time, or anything but a token's letters.
fn token(gh: &Path) -> Option<String> {
    let mut command = Command::new(gh);
    command
        .env_clear()
        .current_dir("/")
        .args(["auth", "token", "--hostname", "github.com"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for name in [
        "HOME",
        "XDG_CONFIG_HOME",
        "XDG_RUNTIME_DIR",
        "DBUS_SESSION_BUS_ADDRESS",
    ] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    let ran = within(&mut command)?;
    let token = String::from_utf8(ran.stdout).ok()?;
    let token = token.trim_end_matches('\n');
    let plain = (1..=LONGEST_TOKEN).contains(&token.len())
        && token.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    (ran.status.success() && plain).then(|| token.to_owned())
}

/// What a command said, once it's ended within the forge's time; none
/// where it didn't start or end in time, when it's ended.
fn within(command: &mut Command) -> Option<std::process::Output> {
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
    child.wait_with_output().ok()
}

/// Bytes in base64, as a Basic header says them.
fn base64(bytes: &[u8]) -> String {
    const LETTERS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut said = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for three in bytes.chunks(3) {
        let whole = three.iter().enumerate().fold(0_u32, |all, (at, byte)| {
            all | u32::from(*byte) << (16 - 8 * at)
        });
        for at in 0..4 {
            if at <= three.len() {
                said.push(char::from(LETTERS[(whole >> (18 - 6 * at) & 63) as usize]));
            } else {
                said.push('=');
            }
        }
    }
    said
}

/// The git that asks the forge of `prefix` where the repository at
/// `url` has HEAD: outside any repository, with none of the account's
/// settings, speaking only the prefix's own protocol, and sending
/// `credential`, a header, to that forge alone.
fn asking(git: &Path, prefix: &str, url: &str, credential: Option<&str>) -> Option<Command> {
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
    // In git's environment rather than its arguments, which any process
    // may read, and scoped to the forge's own URL, with no redirect
    // followed, which would carry it off the forge (ISXEG8, 15 November).
    if let Some(credential) = credential {
        command
            .env("GIT_CONFIG_COUNT", "2")
            .env("GIT_CONFIG_KEY_0", format!("http.{prefix}.extraHeader"))
            .env("GIT_CONFIG_VALUE_0", credential)
            .env("GIT_CONFIG_KEY_1", "http.followRedirects")
            .env("GIT_CONFIG_VALUE_1", "false");
    }
    Some(command)
}

/// What the forge of `prefix` says the repository at `url` has as HEAD,
/// asked with `credential` where it's given one.
fn asked(git: &Path, prefix: &str, url: &str, credential: Option<&str>) -> Option<String> {
    let ran = within(&mut asking(git, prefix, url, credential)?)?;
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
        let forge = Forge::new(
            PathBuf::from("git"),
            vec!["https://github.com/".to_owned()],
            None,
        );
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
        let none = Forge::new(PathBuf::from("git"), Vec::new(), None);
        assert!(
            none.prefix_of("https://github.com/a/b").is_none(),
            "a forge was asked though none is named"
        );
    }

    /// A stand-in gh saying `said` for a token, in a folder of its own.
    fn gh_saying(name: &str, said: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let folder =
            std::env::temp_dir().join(format!("secret-asks-gh-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("the stand-in's folder is made");
        let gh = folder.join("gh");
        // The script is copied into place by cp rather than written here,
        // so no thread of this process holds it open for writing: a test
        // forking meanwhile keeps that handle until its child execs, and
        // gh run then is refused as busy (ETXTBSY), which failed the merge
        // train's run of this module on 16 November.
        let draft = folder.join("gh.draft");
        std::fs::write(&draft, format!("#!/bin/sh\nprintf '{said}'\n")).expect("gh is written");
        let copied = std::process::Command::new("cp")
            .arg(&draft)
            .arg(&gh)
            .status()
            .expect("cp runs");
        assert!(copied.success(), "gh wasn't copied into place");
        std::fs::set_permissions(&gh, std::fs::Permissions::from_mode(0o755))
            .expect("the stand-in runs");
        gh
    }

    #[test]
    fn only_github_over_https_is_asked_with_the_accounts_credential() {
        let named = vec![
            CREDITED.to_owned(),
            "https://forge.example/".to_owned(),
            "http://github.com/".to_owned(),
        ];
        let gh = gh_saying("credited", "gho_ab\\n");
        let forge = Forge::new(PathBuf::from("git"), named.clone(), Some(gh));
        assert_eq!(
            forge.credential(CREDITED).as_deref(),
            Some("Authorization: Basic eC1hY2Nlc3MtdG9rZW46Z2hvX2Fi"),
            "GitHub wasn't asked with gh's token"
        );
        for elsewhere in ["https://forge.example/", "http://github.com/"] {
            assert_eq!(
                forge.credential(elsewhere),
                None,
                "{elsewhere} was given GitHub's token"
            );
        }
        let unsigned = Forge::new(PathBuf::from("git"), named, None);
        assert_eq!(
            unsigned.credential(CREDITED),
            None,
            "a credential came from no gh"
        );
    }

    #[test]
    fn gh_saying_anything_but_a_token_gives_no_credential() {
        for (name, said) in [
            ("lines", "gho_a\\nX-Other: b"),
            ("spaced", "gho a"),
            ("none", ""),
        ] {
            let forge = Forge::new(
                PathBuf::from("git"),
                vec![CREDITED.to_owned()],
                Some(gh_saying(name, said)),
            );
            assert_eq!(forge.credential(CREDITED), None, "gh's {said:?} was sent");
        }
    }

    #[test]
    fn the_credential_is_in_gits_environment_for_its_forge_alone() {
        let header = "Authorization: Basic eA==";
        let command = asking(
            Path::new("git"),
            CREDITED,
            "https://github.com/a/b",
            Some(header),
        )
        .expect("git asks the forge");
        let envs: HashMap<_, _> = command
            .get_envs()
            .filter_map(|(name, value)| Some((name.to_str()?, value?.to_str()?)))
            .collect();
        assert_eq!(
            (envs.get("GIT_CONFIG_KEY_0"), envs.get("GIT_CONFIG_VALUE_0")),
            (Some(&"http.https://github.com/.extraHeader"), Some(&header)),
            "git wasn't given the header for GitHub alone: {envs:?}"
        );
        // A redirect would carry the header off GitHub (ISXEG8, 15 November).
        assert_eq!(
            (envs.get("GIT_CONFIG_KEY_1"), envs.get("GIT_CONFIG_VALUE_1")),
            (Some(&"http.followRedirects"), Some(&"false")),
            "git may follow a redirect with the header: {envs:?}"
        );
        assert!(
            command.get_args().all(|arg| arg.to_str() != Some(header)),
            "the credential is in git's arguments"
        );
    }

    #[test]
    fn base64_says_bytes_as_a_header_does() {
        for (bytes, said) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(bytes.as_bytes()), said, "{bytes:?}");
        }
    }

    #[test]
    fn a_forge_is_called_by_its_name() {
        assert_eq!(called("https://github.com/someone/tree"), "GitHub");
        assert_eq!(called("https://forge.example/a/b"), "forge.example");
        assert_eq!(called("file:///srv/forge/a.git"), "its forge");
    }
}
