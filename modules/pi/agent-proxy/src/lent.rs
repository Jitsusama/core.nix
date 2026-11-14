//! The builders lent to the machine, whose logins the proxy signs with
//! the builders' key, and that key itself.
//!
//! nix-offload reaches each lent builder as the account `nix-offload`
//! with a key of the machine's own (`--key`), which the fence hides from
//! every command: a command holding it could log in to a builder as
//! anything it liked. So the proxy holds it, and signs with it only a
//! login as `nix-offload` to a lent builder, proved by the builder's own
//! signature over the session (src/bind.rs). Which builders are lent
//! changes as builders come and go, so their host keys are read afresh
//! at each binding from root's machines files (`--lent`), which only
//! nix-offload's lines account writes, once the cloud vouches for each
//! builder's host key; a command can't write them.

use std::fs;
use std::path::PathBuf;

use ssh_key::public::KeyData;
use ssh_key::{PrivateKey, PublicKey};

/// The account a lent builder takes the builders' key for, and the only
/// one the proxy signs a login with that key for.
pub const ACCOUNT: &[u8] = b"nix-offload";

/// What every alias nix-offload names a lent builder by starts with.
const ALIAS: &str = "nix-offload";

/// The field of a machines line carrying its host key: base64 of
/// `ssh-ed25519 KEY`, or `-`.
const HOST_KEY_FIELD: usize = 7;

/// Root's machines files, read afresh at each binding.
#[derive(Default)]
pub struct Lent(Vec<PathBuf>);

impl Lent {
    /// The machines files at `files`.
    pub fn new(files: Vec<PathBuf>) -> Self {
        Lent(files)
    }

    /// Whether `host` is the host key of a builder lent now: a line of a
    /// machines file naming a nix-offload alias with exactly that ed25519
    /// key. A file that can't be read, and a line that can't be, or that
    /// names another kind of key, lend nothing.
    pub fn lends(&self, host: &KeyData) -> bool {
        self.0.iter().any(|file| {
            fs::read_to_string(file)
                .is_ok_and(|text| text.lines().filter_map(lent_host).any(|key| &key == host))
        })
    }
}

/// The host key a machines line lends, when it names a nix-offload alias
/// and an ed25519 host key that reads.
fn lent_host(line: &str) -> Option<KeyData> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    let uri = fields.first()?;
    let rest = uri
        .strip_prefix("ssh-ng://")
        .or_else(|| uri.strip_prefix("ssh://"))?;
    let host = rest.split('?').next()?;
    let alias = host.rsplit('@').next()?;
    if !alias.starts_with(ALIAS) {
        return None;
    }
    let decoded = base64(fields.get(HOST_KEY_FIELD)?)?;
    let text = String::from_utf8(decoded).ok()?;
    let key = PublicKey::from_openssh(text.trim()).ok()?;
    match key.key_data() {
        KeyData::Ed25519(_) => Some(key.key_data().clone()),
        _ => None,
    }
}

/// `text` decoded from standard base64, padded or not; nothing when any
/// character isn't base64's.
fn base64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let mut held = 0u32;
    let mut bits = 0;
    for byte in text.trim_end_matches('=').bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        held = (held << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(u8::try_from((held >> bits) & 0xff).ok()?);
        }
    }
    Some(out)
}

/// The builders' key, read from its file at each use, since the machine
/// makes it only once it first lends a builder.
pub struct Key(Option<PathBuf>);

impl Key {
    /// The key in `file`, or none when the proxy holds no key.
    pub fn new(file: Option<PathBuf>) -> Self {
        Key(file)
    }

    /// The key, when there's a file holding one unencrypted. One that
    /// can't be read is said, since a builder's login then fails.
    pub fn read(&self) -> Option<PrivateKey> {
        let file = self.0.as_ref()?;
        match PrivateKey::read_openssh_file(file) {
            Ok(key) if !key.is_encrypted() => Some(key),
            Ok(_) => {
                eprintln!(
                    "pi-agent-proxy: {} is encrypted, so it signs no builder's login",
                    file.display()
                );
                None
            }
            Err(error) => {
                eprintln!(
                    "pi-agent-proxy: {} can't be read, so it signs no builder's login: {error}",
                    file.display()
                );
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOST: &str =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJBWcxb/Blaqt1auOtE+F8QUWrUotiC5qBJ+UuEWdVCb";

    fn encoded(text: &str) -> String {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in text.as_bytes().chunks(3) {
            let mut block = [0u8; 3];
            block[..chunk.len()].copy_from_slice(chunk);
            let value =
                (u32::from(block[0]) << 16) | (u32::from(block[1]) << 8) | u32::from(block[2]);
            for at in 0..=chunk.len() {
                let six = usize::try_from((value >> (18 - 6 * at)) & 63).expect("six bits fit");
                out.push(char::from(ALPHABET[six]));
            }
            for _ in chunk.len()..3 {
                out.push('=');
            }
        }
        out
    }

    fn key() -> KeyData {
        PublicKey::from_openssh(HOST)
            .expect("reads")
            .key_data()
            .clone()
    }

    #[test]
    fn a_line_naming_a_nix_offload_alias_lends_its_host_key() {
        let line = format!(
            "ssh-ng://hook@nix-offload?compress=true x86_64-linux /k 16 1 big-parallel - {}",
            encoded(HOST)
        );
        assert_eq!(lent_host(&line), Some(key()), "the hook's builder is lent");
        let more = format!(
            "ssh-ng://nix-offload-vm-4 x86_64-linux /k 4 1 - - {}",
            encoded(HOST)
        );
        assert_eq!(lent_host(&more), Some(key()), "a VM builder is lent");
    }

    #[test]
    fn a_standing_builder_lends_nothing() {
        let line = format!(
            "ssh-ng://equilibrius-linux x86_64-linux /k 4 0.5 big-parallel - {}",
            encoded(HOST)
        );
        assert_eq!(
            lent_host(&line),
            None,
            "a machine that isn't lent is never one"
        );
    }

    #[test]
    fn a_line_without_a_key_that_reads_lends_nothing() {
        for line in [
            "ssh-ng://nix-offload x86_64-linux /k 4 1 - - -".to_owned(),
            "ssh-ng://nix-offload x86_64-linux /k 4 1 - - not*base64".to_owned(),
            format!(
                "ssh-ng://nix-offload x86_64-linux /k 4 1 - - {}",
                encoded("ssh-ed25519 broken")
            ),
            "ssh-ng://nix-offload x86_64-linux".to_owned(),
            String::new(),
        ] {
            assert_eq!(lent_host(&line), None, "{line:?} lends nothing");
        }
    }

    #[test]
    fn another_kind_of_host_key_lends_nothing() {
        let rsa = "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAAAgQC7";
        let line = format!(
            "ssh-ng://nix-offload x86_64-linux /k 4 1 - - {}",
            encoded(rsa)
        );
        assert_eq!(lent_host(&line), None, "only an ed25519 host key is lent");
    }

    #[test]
    fn base64_reads_padded_and_bare_and_refuses_strangers() {
        assert_eq!(base64("aGk=").as_deref(), Some(&b"hi"[..]), "padded");
        assert_eq!(base64("aGk").as_deref(), Some(&b"hi"[..]), "bare");
        assert_eq!(base64("a-k"), None, "a character outside base64");
    }
}
