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
//! at each binding from root's file of the builders it vouches for now
//! (`--lent`), which only nix-offload's lines account writes, once the
//! cloud vouches for each builder's host key; a command can't write it.
//! Each of its lines names one builder:
//!
//! ```text
//! <lender> <instance> <key-type> <base64-key>
//! ```

use std::fs;
use std::path::PathBuf;

use ssh_key::public::KeyData;
use ssh_key::{PrivateKey, PublicKey};

/// The account a lent builder takes the builders' key for, and the only
/// one the proxy signs a login with that key for.
pub const ACCOUNT: &[u8] = b"nix-offload";

/// What every name nix-offload reaches a lent builder by starts with.
const LENDER: &str = "nix-offload";

/// The only kind of host key a lent builder is taken with.
const ED25519: &str = "ssh-ed25519";

/// Root's files of the builders it vouches for, read afresh at each
/// binding.
#[derive(Default)]
pub struct Lent(Vec<PathBuf>);

impl Lent {
    /// The files at `files`.
    pub fn new(files: Vec<PathBuf>) -> Self {
        Lent(files)
    }

    /// Whether `host` is the host key of a builder lent now: a line of
    /// one of the files naming a nix-offload lender with exactly that
    /// ed25519 key. A file that can't be read, and a line of any other
    /// shape, or naming another kind of key, lend nothing.
    pub fn lends(&self, host: &KeyData) -> bool {
        self.0.iter().any(|file| {
            fs::read_to_string(file)
                .is_ok_and(|text| text.lines().filter_map(lent_host).any(|key| &key == host))
        })
    }
}

/// The host key a line lends, when it's `<lender> <instance> ssh-ed25519
/// <key>` with a nix-offload lender and a key that reads.
fn lent_host(line: &str) -> Option<KeyData> {
    if line.starts_with('#') {
        return None;
    }
    let fields: Vec<&str> = line.split(' ').collect();
    let [lender, instance, kind, key] = fields[..] else {
        return None;
    };
    if !lender.starts_with(LENDER) || instance.is_empty() || kind != ED25519 {
        return None;
    }
    let read = PublicKey::from_openssh(&format!("{kind} {key}")).ok()?;
    match read.key_data() {
        KeyData::Ed25519(_) => Some(read.key_data().clone()),
        _ => None,
    }
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

    const KEY: &str = "AAAAC3NzaC1lZDI1NTE5AAAAIJBWcxb/Blaqt1auOtE+F8QUWrUotiC5qBJ+UuEWdVCb";

    fn key() -> KeyData {
        PublicKey::from_openssh(&format!("{ED25519} {KEY}"))
            .expect("reads")
            .key_data()
            .clone()
    }

    #[test]
    fn a_line_naming_a_nix_offload_lender_lends_its_host_key() {
        let hook = format!("nix-offload offload-abc123 ssh-ed25519 {KEY}");
        assert_eq!(
            lent_host(&hook),
            Some(key()),
            "the first lender's builder is lent"
        );
        let vm = format!("nix-offload-vm-4 offload-vm-4-def456 ssh-ed25519 {KEY}");
        assert_eq!(lent_host(&vm), Some(key()), "a VM lender's builder is lent");
    }

    #[test]
    fn another_lender_lends_nothing() {
        let line = format!("standing-builder offload-abc123 ssh-ed25519 {KEY}");
        assert_eq!(
            lent_host(&line),
            None,
            "a machine nix-offload doesn't lend is never one"
        );
    }

    #[test]
    fn a_line_of_another_shape_lends_nothing() {
        for line in [
            format!("# nix-offload offload-abc123 ssh-ed25519 {KEY}"),
            format!("nix-offload  offload-abc123 ssh-ed25519 {KEY}"),
            format!("nix-offload offload-abc123 ssh-ed25519 {KEY} a comment"),
            format!("nix-offload ssh-ed25519 {KEY}"),
            "nix-offload offload-abc123 ssh-ed25519 broken".to_owned(),
            format!("ssh-ng://nix-offload x86_64-linux /k 4 1 - - {KEY}"),
            String::new(),
        ] {
            assert_eq!(lent_host(&line), None, "{line:?} lends nothing");
        }
    }

    #[test]
    fn another_kind_of_host_key_lends_nothing() {
        let rsa = "nix-offload offload-abc123 ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAAAgQC7";
        assert_eq!(lent_host(rsa), None, "only an ed25519 host key is lent");
    }

    #[test]
    fn a_file_lends_each_of_its_builders_and_none_once_gone() {
        let folder =
            std::env::temp_dir().join(format!("pi-agent-proxy-lent-{}", std::process::id()));
        fs::create_dir_all(&folder).expect("a folder for the file");
        let file = folder.join("lent");
        fs::write(
            &file,
            format!("# vouched for now\nnix-offload offload-abc123 ssh-ed25519 {KEY}\n"),
        )
        .expect("the file is written");
        let lent = Lent::new(vec![file.clone()]);
        assert!(lent.lends(&key()), "a builder the file names is lent");

        fs::write(&file, "").expect("the file is emptied");
        assert!(!lent.lends(&key()), "an empty file lends none");

        fs::remove_file(&file).expect("the file is removed");
        assert!(!lent.lends(&key()), "a missing file lends none");
        let _ = fs::remove_dir_all(&folder);
    }
}
