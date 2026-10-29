//! Building the checks on the builder nix-offload lends, by sending it a
//! command rather than paths.
//!
//! The build hook builds a check's inputs here and copies each up, and a
//! VM test took 1025 s that way against 60 s here, nearly all of it the
//! upload. Sending the build whole with `--store` crosses only derivations
//! and sources, 3.9 MB for that test, but copies them one path at a time,
//! 5070 round trips, and a builder that wants signed paths refuses them.
//! So the builder is given a command naming a commit origin has, and it
//! fetches the commit and every input, evaluates and builds there. Only
//! the command goes up, and the log comes down; evaluation, the heaviest
//! part of a check here, leaves this machine too.

use std::fs;
use std::path::Path;
use std::process::Command;

/// The builder nix-offload lends, as it names the builder's store: where
/// ssh reaches it, the key it takes and its host key.
#[derive(Debug, PartialEq, Eq)]
pub struct Builder {
    /// The host, as ssh is given it, with its user where the store names one.
    host: String,
    /// The key the builder takes, where the store names one.
    key: Option<String>,
    /// The builder's host key, as a line of a known hosts file has it.
    host_key: Option<String>,
}

/// How a build there ended, and what it sent the builder.
pub struct Sent {
    pub passed: bool,
    /// What ssh sent, as its own count says, or none when it didn't say.
    pub bytes: Option<u64>,
}

/// The builder lent while one stands: the first line of `store` in
/// nix-offload's folder, when it names an ssh store.
pub fn lent(offload: &Path) -> Option<Builder> {
    let named = fs::read_to_string(offload.join("store")).ok()?;
    builder(named.lines().next()?.trim())
}

/// The builder an ssh store's URI names, as Nix writes one:
/// `ssh-ng://host?ssh-key=...&base64-ssh-public-host-key=...`.
fn builder(store: &str) -> Option<Builder> {
    let rest = store
        .strip_prefix("ssh-ng://")
        .or_else(|| store.strip_prefix("ssh://"))?;
    let (host, settings) = rest.split_once('?').unwrap_or((rest, ""));
    if host.is_empty() {
        return None;
    }
    let setting = |name: &str| {
        settings
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.to_owned())
    };
    Some(Builder {
        host: host.to_owned(),
        key: setting("ssh-key"),
        host_key: setting("base64-ssh-public-host-key").and_then(|key| decoded(&key)),
    })
}

/// The flake a builder fetches a commit from origin's URL by: GitHub's
/// archive where origin is there, since it needs no git and no API, and
/// otherwise git over the URL.
pub fn fetched(url: &str, commit: &str) -> String {
    let on_github = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("git@github.com:"))
        .or_else(|| url.strip_prefix("ssh://git@github.com/"));
    if let Some(path) = on_github {
        let path = path.trim_end_matches('/');
        let path = path.strip_suffix(".git").unwrap_or(path);
        return format!("github:{path}/{commit}");
    }
    if url.contains("://") {
        format!("git+{url}?rev={commit}")
    } else {
        // git's short form, `user@host:path`, is ssh.
        let (host, path) = url.split_once(':').unwrap_or((url, ""));
        format!("git+ssh://{host}/{path}?rev={commit}")
    }
}

impl Builder {
    /// Build `checks` of `flake` for `system` on the builder, its log
    /// coming here as it goes. `cache` holds, for the length of the build,
    /// the builder's host key as ssh reads it and ssh's own log.
    pub fn build(
        &self,
        flake: &str,
        system: &str,
        checks: &[&str],
        cache: &Path,
    ) -> Result<Sent, String> {
        let folder = cache.join("changed-checks");
        fs::create_dir_all(&folder).map_err(|error| format!("{}: {error}", folder.display()))?;
        let mine = |name: &str| folder.join(format!("{name}-{}", std::process::id()));
        let known = mine("known");
        let said = mine("ssh.log");
        let alias = self.host.rsplit('@').next().unwrap_or(&self.host);
        let mut ssh = Command::new("ssh");
        // ssh's verbose log ends with what it sent, which is what this run
        // put on the upload.
        ssh.args(["-o", "BatchMode=yes", "-o", "LogLevel=VERBOSE", "-E"])
            .arg(&said);
        if let Some(key) = &self.key {
            ssh.args(["-o", "IdentitiesOnly=yes", "-i", key]);
        }
        if let Some(host_key) = &self.host_key {
            fs::write(&known, format!("{alias} {host_key}\n"))
                .map_err(|error| format!("{}: {error}", known.display()))?;
            ssh.args(["-o", "StrictHostKeyChecking=yes", "-o"])
                .arg(format!("UserKnownHostsFile={}", known.display()))
                .args(["-o", &format!("HostKeyAlias={alias}")]);
        }
        let command = std::iter::once("nix build --no-link --keep-going -L".to_owned())
            .chain(
                checks
                    .iter()
                    .map(|check| quoted(&format!("{flake}#checks.{system}.{check}"))),
            )
            .collect::<Vec<_>>()
            .join(" ");
        eprintln!("changed-checks: building on the lent builder, {flake}");
        let ran = ssh.arg(&self.host).arg(command).status();
        let bytes = fs::read_to_string(&said).ok().and_then(|log| sent(&log));
        let _ = fs::remove_file(&known);
        let _ = fs::remove_file(&said);
        let ran = ran.map_err(|error| format!("running ssh: {error}"))?;
        Ok(Sent {
            passed: ran.success(),
            bytes,
        })
    }
}

/// What ssh's verbose log says it sent, from its closing count:
/// `Transferred: sent 4321, received 98765 bytes, in 3.2 seconds`.
fn sent(log: &str) -> Option<u64> {
    log.lines().rev().find_map(|line| {
        line.split_once("Transferred: sent ")?
            .1
            .split_once(',')?
            .0
            .parse()
            .ok()
    })
}

/// A word the builder's shell reads as itself.
fn quoted(word: &str) -> String {
    format!("'{}'", word.replace('\'', r"'\''"))
}

/// Base64 as text, or none when it isn't both. Nix's store URIs carry a
/// host key this way, and it's the only base64 this reads, so it's
/// decoded here rather than with a crate.
fn decoded(encoded: &str) -> Option<String> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut bits: u32 = 0;
    let mut held = 0;
    let mut bytes = Vec::new();
    for byte in encoded.trim_end_matches('=').bytes() {
        let value = ALPHABET.iter().position(|letter| *letter == byte)?;
        bits = (bits << 6) | u32::try_from(value).ok()?;
        held += 6;
        if held >= 8 {
            held -= 8;
            bytes.push(u8::try_from(bits >> held).ok()?);
            bits &= (1 << held) - 1;
        }
    }
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_store_names_its_host_its_key_and_its_host_key() {
        let store = "ssh-ng://nix-offload?compress=true&ssh-key=/k&\
                     base64-ssh-public-host-key=c3NoLWVkMjU1MTkgQUFBQQ==";
        assert_eq!(
            builder(store),
            Some(Builder {
                host: "nix-offload".to_owned(),
                key: Some("/k".to_owned()),
                host_key: Some("ssh-ed25519 AAAA".to_owned()),
            })
        );
        assert_eq!(
            builder("ssh://someone@far"),
            Some(Builder {
                host: "someone@far".to_owned(),
                key: None,
                host_key: None,
            })
        );
        assert_eq!(builder("daemon"), None);
        assert_eq!(builder("ssh-ng://?ssh-key=/k"), None);
    }

    #[test]
    fn base64_is_decoded_whatever_its_padding() {
        assert_eq!(decoded("YQ==").as_deref(), Some("a"));
        assert_eq!(decoded("YWI=").as_deref(), Some("ab"));
        assert_eq!(decoded("YWJj").as_deref(), Some("abc"));
        assert_eq!(decoded("not base64!"), None);
    }

    #[test]
    fn a_commit_is_fetched_from_githubs_archive_or_else_by_git() {
        let commit = "fedcba98";
        for url in [
            "https://github.com/someone/thing.git",
            "https://github.com/someone/thing",
            "git@github.com:someone/thing.git",
            "ssh://git@github.com/someone/thing.git",
        ] {
            assert_eq!(
                fetched(url, commit),
                "github:someone/thing/fedcba98",
                "{url}"
            );
        }
        assert_eq!(
            fetched("https://git.example/thing.git", commit),
            "git+https://git.example/thing.git?rev=fedcba98"
        );
        assert_eq!(
            fetched("me@git.example:thing.git", commit),
            "git+ssh://me@git.example/thing.git?rev=fedcba98"
        );
    }

    #[test]
    fn what_ssh_sent_is_read_from_its_last_count() {
        let log = "Authenticated to far\n\
                   Transferred: sent 4321, received 98765 bytes, in 3.2 seconds\n\
                   Bytes per second: sent 1350.3, received 30864.1\n";
        assert_eq!(sent(log), Some(4321));
        assert_eq!(sent("Authenticated to far\n"), None);
    }

    #[test]
    fn a_word_with_a_quote_is_quoted_for_the_shell() {
        assert_eq!(quoted("a#b?c&d"), "'a#b?c&d'");
        assert_eq!(quoted("it's"), r"'it'\''s'");
    }
}
