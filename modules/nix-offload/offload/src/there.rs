//! `nix-offload there [-L] [--keep-going] FLAKE ATTR...`: builds a flake's
//! attributes on the lent builder by sending it a command, or here when
//! none stands.
//!
//! The build hook builds a build's inputs here and copies each up, and a
//! VM test took 1025 s that way against 60 s here, nearly all of it the
//! upload. So a flake the builder can fetch itself, such as a pushed
//! commit of a public repository, goes as one command: the builder
//! fetches it and every input, evaluates and builds, and only the command
//! goes up while the log comes down. Evaluation leaves this machine too.
//!
//! A caller may rewrite any `nix build --no-link` into this without ever
//! failing for the routing: with no builder, or none ssh can reach, the
//! same build runs here, saying so. It exits with the build's own status,
//! and journals where each ran and what it sent, in `there.jsonl` beside
//! the lender's journal.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

/// What a `there` was asked to build, and how.
#[derive(Debug, PartialEq, Eq)]
pub struct Asked {
    flake: String,
    attributes: Vec<String>,
    /// The flags passed through to `nix build`: `-L` and `--keep-going`.
    flags: Vec<String>,
}

/// Reads `there`'s arguments: its flags, in any place, then the flake and
/// at least one attribute. Anything else is refused, since only a build
/// whose every word is known can be sent whole.
pub fn asked(arguments: &[String]) -> Option<Asked> {
    let mut flags = Vec::new();
    let mut words = Vec::new();
    for argument in arguments {
        match argument.as_str() {
            "-L" | "--print-build-logs" | "--keep-going" => {
                if !flags.contains(argument) {
                    flags.push(argument.clone());
                }
            }
            flag if flag.starts_with('-') => return None,
            word => words.push(word.to_owned()),
        }
    }
    let (flake, attributes) = words.split_first()?;
    (!attributes.is_empty()).then(|| Asked {
        flake: flake.clone(),
        attributes: attributes.to_vec(),
        flags,
    })
}

impl Asked {
    /// `nix build`'s arguments for this, wherever it runs.
    fn build(&self) -> Vec<String> {
        ["build", "--no-link"]
            .iter()
            .map(|word| (*word).to_owned())
            .chain(self.flags.iter().cloned())
            .chain(
                self.attributes
                    .iter()
                    .map(|attribute| format!("{}#{attribute}", self.flake)),
            )
            .collect()
    }

    /// The command the builder's shell is given.
    fn command(&self) -> String {
        std::iter::once("nix".to_owned())
            .chain(self.build().iter().map(|word| quoted(word)))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// The lent builder, as nix-offload names its store: where ssh reaches it,
/// the key it takes and its host key.
#[derive(Debug, PartialEq, Eq)]
struct Builder {
    /// The host, as ssh is given it, with its user where the store names one.
    host: String,
    /// The key the builder takes, where the store names one.
    key: Option<String>,
    /// The builder's host key, as a line of a known hosts file has it.
    host_key: Option<String>,
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

/// ssh's own status when it couldn't reach the builder or lost it, which
/// no build gives.
const UNREACHED: i32 = 255;

/// Where a build ran and how it ended.
#[derive(Debug, PartialEq, Eq)]
pub struct Ran {
    /// The builder's host, or none when it ran here.
    pub on: Option<String>,
    /// The build's status.
    pub status: i32,
    /// What ssh sent the builder, as its own count says.
    pub sent: Option<u64>,
}

/// Builds what was asked on the builder `store` names (the store file's
/// first line), or here when there's none or ssh can't reach it. `scratch`
/// holds, for the length of the build, the builder's host key as ssh
/// reads it and ssh's own log; `ssh` and `nix` are the programs run.
pub fn run(asked: &Asked, store: Option<&str>, scratch: &Path, ssh: &str, nix: &str) -> Ran {
    let lent = store.and_then(|store| builder(store.lines().next()?.trim()));
    let Some(lent) = lent else {
        eprintln!("nix-offload: no builder is lent, so this builds here");
        return here(asked, nix);
    };
    match on(&lent, asked, scratch, ssh) {
        Ok(ran) if ran.status != UNREACHED => ran,
        Ok(_) => {
            eprintln!(
                "nix-offload: ssh couldn't reach {}, so this builds here",
                lent.host
            );
            here(asked, nix)
        }
        Err(error) => {
            eprintln!("nix-offload: {error}, so this builds here");
            here(asked, nix)
        }
    }
}

fn here(asked: &Asked, nix: &str) -> Ran {
    let status = Command::new(nix)
        .args(asked.build())
        .status()
        .map_or(1, |status| status.code().unwrap_or(1));
    Ran {
        on: None,
        status,
        sent: None,
    }
}

fn on(lent: &Builder, asked: &Asked, scratch: &Path, ssh: &str) -> Result<Ran, String> {
    fs::create_dir_all(scratch).map_err(|error| format!("{}: {error}", scratch.display()))?;
    let mine = |name: &str| scratch.join(format!("{name}-{}", std::process::id()));
    let known = mine("known");
    let said = mine("ssh.log");
    let alias = lent.host.rsplit('@').next().unwrap_or(&lent.host);
    let mut command = Command::new(ssh);
    // ssh's verbose log ends with what it sent, which is what this build
    // put on the upload.
    command
        .args(["-o", "BatchMode=yes", "-o", "LogLevel=VERBOSE", "-E"])
        .arg(&said);
    if let Some(key) = &lent.key {
        command.args(["-o", "IdentitiesOnly=yes", "-i", key]);
    }
    if let Some(host_key) = &lent.host_key {
        fs::write(&known, format!("{alias} {host_key}\n"))
            .map_err(|error| format!("{}: {error}", known.display()))?;
        command
            .args(["-o", "StrictHostKeyChecking=yes", "-o"])
            .arg(format!("UserKnownHostsFile={}", known.display()))
            .args(["-o", &format!("HostKeyAlias={alias}")]);
    }
    eprintln!("nix-offload: building on the lent builder, {}", asked.flake);
    let status = command.arg(&lent.host).arg(asked.command()).status();
    let sent = fs::read_to_string(&said).ok().and_then(|log| sent(&log));
    let _ = fs::remove_file(&known);
    let _ = fs::remove_file(&said);
    let status = status.map_err(|error| format!("couldn't run {ssh}: {error}"))?;
    Ok(Ran {
        on: Some(lent.host.clone()),
        // A build killed by a signal there comes back as ssh's status.
        status: status.code().unwrap_or(UNREACHED),
        sent,
    })
}

/// Adds a line for a build to `journal`: when it ended, how long it took,
/// what it built, where, its status and what it sent.
pub fn note(journal: &Path, at: u64, seconds: u64, asked: &Asked, ran: &Ran) {
    let line = serde_json::json!({
        "at": at,
        "seconds": seconds,
        "flake": asked.flake,
        "attributes": asked.attributes,
        "on": ran.on,
        "status": ran.status,
        "sent": ran.sent,
    });
    let written = journal
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| {
            fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(journal)
        })
        .and_then(|mut file| writeln!(file, "{line}"));
    if let Err(error) = written {
        eprintln!(
            "nix-offload: couldn't journal the build in {}: {error}",
            journal.display()
        );
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
    if word
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b"-_./:".contains(&byte))
    {
        return word.to_owned();
    }
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

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn a_flake_and_its_attributes_are_asked_with_their_flags_anywhere() {
        let asked = asked(&words(
            "-L github:o/r/abc checks.x.a --keep-going checks.x.b",
        ));
        assert_eq!(
            asked,
            Some(Asked {
                flake: "github:o/r/abc".to_owned(),
                attributes: words("checks.x.a checks.x.b"),
                flags: words("-L --keep-going"),
            })
        );
    }

    #[test]
    fn a_build_with_no_attribute_or_an_unknown_flag_is_refused() {
        assert_eq!(asked(&words("github:o/r/abc")), None);
        assert_eq!(asked(&words("-L")), None);
        assert_eq!(asked(&words("--impure github:o/r/abc checks.x.a")), None);
    }

    #[test]
    fn the_builder_is_given_one_quoted_command() {
        let asked = asked(&words("-L github:o/r/abc checks.x.it's")).unwrap();
        assert_eq!(
            asked.command(),
            r"nix build --no-link -L 'github:o/r/abc#checks.x.it'\''s'"
        );
    }

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
            builder("ssh://someone@far").map(|lent| lent.host),
            Some("someone@far".to_owned())
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
    fn what_ssh_sent_is_read_from_its_last_count() {
        let log = "Authenticated to far\n\
                   Transferred: sent 4321, received 98765 bytes, in 3.2 seconds\n";
        assert_eq!(sent(log), Some(4321));
        assert_eq!(sent("Authenticated to far\n"), None);
    }
}
