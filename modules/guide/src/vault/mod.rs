//! The vault: what a reinstall stick carries for one machine, every byte
//! of it age-encrypted under one vault key. The key is sealed to the
//! machine's YubiKeys in `key.age` and, since age keeps a passphrase from
//! sharing a file with other recipients, to the recovery passphrase alone
//! in `key-passphrase.age`, as quest-backup seals a chain's key. Files are
//! split into numbered objects of at most a gibibyte, under FAT32's limit
//! of four, and an encrypted index says which objects make which file, so
//! nothing but `manifest.json` shows a name.

pub mod store;

use std::io::{self, BufRead, BufReader, Read, Write};

use age::secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::{Fault, Secret};
use store::Store;

pub const MANIFEST: &str = "manifest.json";
pub const KEY: &str = "key.age";
pub const KEY_PASSPHRASE: &str = "key-passphrase.age";
const INDEX: &str = "index.age";
/// The age plugin a YubiKey seals and opens through, which age finds as
/// `age-plugin-fido2-hmac` on the guide's own PATH (main.rs sets it).
const FIDO2: &str = "fido2-hmac";

/// What a plugin is told when it asks something while sealing: nothing,
/// since sealing to a YubiKey's recipient asks for no key.
#[derive(Clone)]
struct Quiet;

impl age::Callbacks for Quiet {
    fn display_message(&self, _: &str) {}
    fn confirm(&self, _: &str, _: &str, _: Option<&str>) -> Option<bool> {
        None
    }
    fn request_public_string(&self, _: &str) -> Option<String> {
        None
    }
    fn request_passphrase(&self, _: &str) -> Option<SecretString> {
        None
    }
}

/// The PIN a YubiKey asks for when it opens the vault's key, given the
/// once the person typed it; anything else it asks gets no answer, so the
/// plugin stops rather than waiting on a prompt nobody sees.
#[derive(Clone)]
struct Pin(std::sync::Arc<Zeroizing<String>>);

impl age::Callbacks for Pin {
    fn display_message(&self, _: &str) {}
    fn confirm(&self, _: &str, _: &str, _: Option<&str>) -> Option<bool> {
        None
    }
    fn request_public_string(&self, _: &str) -> Option<String> {
        None
    }
    fn request_passphrase(&self, _: &str) -> Option<SecretString> {
        Some(SecretString::from(self.0.as_str().to_string()))
    }
}
/// The most an object holds.
const OBJECT: u64 = 1 << 30;

/// What anyone reading the stick may know: whose vault it is and what
/// opens it, so the guide can name the key to plug in before asking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: u32,
    pub machine: String,
    /// The YubiKeys `key.age` is sealed to, by the names the person knows
    /// them by.
    pub opened_by: Vec<String>,
}

/// The key every file in a vault is encrypted to, made in memory and only
/// ever written down sealed. age wipes each copy's secret when it drops.
#[derive(Clone)]
pub struct VaultKey(age::x25519::Identity);

impl VaultKey {
    pub fn generate() -> VaultKey {
        VaultKey(age::x25519::Identity::generate())
    }

    fn sealed(&self, encryptor: age::Encryptor) -> Result<Vec<u8>, Fault> {
        let mut sealed = vec![];
        let mut writer = encryptor
            .wrap_output(&mut sealed)
            .map_err(|why| unsealable("the vault's key", why))?;
        writer
            .write_all(self.0.to_string().expose_secret().as_bytes())
            .and_then(|()| writer.finish().map(drop))
            .map_err(|why| unsealable("the vault's key", why))?;
        Ok(sealed)
    }

    /// The key, sealed so any one of these opens it.
    pub fn seal(&self, recipients: &[&dyn age::Recipient]) -> Result<Vec<u8>, Fault> {
        self.sealed(
            age::Encryptor::with_recipients(recipients.iter().copied())
                .map_err(|why| unsealable("the vault's key", why))?,
        )
    }

    /// The key, sealed to a passphrase. `work` is scrypt's log₂ N, left to
    /// age (a second's work on this machine) but for tests.
    pub fn seal_with_passphrase(
        &self,
        passphrase: &Secret,
        work: Option<u8>,
    ) -> Result<Vec<u8>, Fault> {
        let mut recipient =
            age::scrypt::Recipient::new(SecretString::from(passphrase.expose().to_string()));
        if let Some(work) = work {
            recipient.set_work_factor(work);
        }
        self.sealed(
            age::Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))
                .map_err(|why| unsealable("the vault's key", why))?,
        )
    }

    /// The key from its sealed copy, with whatever opens it.
    pub fn open(sealed: &[u8], identity: &dyn age::Identity) -> Result<VaultKey, Fault> {
        let mut text = Zeroizing::new(String::new());
        decrypt(sealed, identity, "the vault's key")?
            .read_to_string(&mut text)
            .map_err(|why| unopened("the vault's key", why))?;
        let key = text
            .trim()
            .parse()
            .map_err(|why: &str| unopened("the vault's key", why))?;
        Ok(VaultKey(key))
    }

    /// The key, sealed to each of the machine's YubiKeys by its age
    /// recipient, beginning `age1fido2-hmac1`, any one of which opens it. Sealing
    /// needs no key plugged in, since such a recipient carries the public
    /// half of what the key's secret makes.
    pub fn seal_to_keys(&self, recipients: &[String]) -> Result<Vec<u8>, Fault> {
        let parsed = recipients
            .iter()
            .map(|recipient| recipient.parse::<age::plugin::Recipient>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|why| unsealable("the vault's key", why))?;
        if parsed.is_empty() || parsed.iter().any(|recipient| recipient.plugin() != FIDO2) {
            return Err(unsealable(
                "the vault's key",
                "a YubiKey's recipient begins age1fido2-hmac1",
            ));
        }
        let plugin = age::plugin::RecipientPluginV1::new(FIDO2, &parsed, &[], Quiet)
            .map_err(|why| unsealable("the vault's key", why))?;
        self.sealed(
            age::Encryptor::with_recipients(std::iter::once(&plugin as &dyn age::Recipient))
                .map_err(|why| unsealable("the vault's key", why))?,
        )
    }

    /// The key from its copy sealed to the YubiKeys, with whichever one is
    /// plugged in and its PIN, which the key asks for along with a touch.
    pub fn open_with_key(sealed: &[u8], pin: &Secret) -> Result<VaultKey, Fault> {
        let identity = age::plugin::Identity::default_for_plugin(FIDO2)
            .map_err(|why| unopened("the vault's key", why))?;
        let plugin = age::plugin::IdentityPluginV1::new(
            FIDO2,
            &[identity],
            Pin(std::sync::Arc::new(Zeroizing::new(
                pin.expose().to_string(),
            ))),
        )
        .map_err(|why| unopened("the vault's key", why))?;
        VaultKey::open(sealed, &plugin)
    }

    /// The key as text, to hold in memory once it's open, so each step
    /// after opens the vault without asking again.
    pub fn to_secret(&self) -> Secret {
        Secret::new(self.0.to_string().expose_secret().to_string())
    }

    /// The key from the text `to_secret` gave.
    pub fn from_secret(text: &Secret) -> Result<VaultKey, Fault> {
        text.expose()
            .trim()
            .parse()
            .map(VaultKey)
            .map_err(|why: &str| unopened("the vault's key", why))
    }

    pub fn open_with_passphrase(sealed: &[u8], passphrase: &Secret) -> Result<VaultKey, Fault> {
        let identity =
            age::scrypt::Identity::new(SecretString::from(passphrase.expose().to_string()));
        VaultKey::open(sealed, &identity)
    }

    fn recipient(&self) -> age::x25519::Recipient {
        self.0.to_public()
    }

    /// Bytes only this key opens, for what's small enough to hold whole,
    /// such as a rescue bundle's keys.
    pub fn encrypt(&self, bytes: &[u8]) -> Result<Vec<u8>, Fault> {
        let recipient = self.recipient();
        let encryptor =
            age::Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))
                .map_err(|why| unsealable("what the key seals", why))?;
        let mut sealed = vec![];
        let mut writer = encryptor
            .wrap_output(&mut sealed)
            .map_err(|why| unsealable("what the key seals", why))?;
        writer
            .write_all(bytes)
            .and_then(|()| writer.finish().map(drop))
            .map_err(|why| unsealable("what the key seals", why))?;
        Ok(sealed)
    }

    /// What `encrypt` sealed.
    pub fn decrypt(&self, sealed: &[u8]) -> Result<Zeroizing<Vec<u8>>, Fault> {
        let mut bytes = Zeroizing::new(vec![]);
        decrypt(sealed, &self.0, "what the key seals")?
            .read_to_end(&mut bytes)
            .map_err(|why| unopened("what the key seals", why))?;
        Ok(bytes)
    }
}

fn unsealable(what: &str, why: impl std::fmt::Display) -> Fault {
    Fault::Unkept {
        path: what.into(),
        why: why.to_string(),
    }
}

fn unopened(what: &str, why: impl std::fmt::Display) -> Fault {
    Fault::Unopened {
        what: what.into(),
        why: why.to_string(),
    }
}

fn decrypt<R: Read>(
    sealed: R,
    identity: &dyn age::Identity,
    what: &str,
) -> Result<age::stream::StreamReader<R>, Fault> {
    age::Decryptor::new(sealed)
        .and_then(|decryptor| decryptor.decrypt(std::iter::once(identity)))
        .map_err(|why| unopened(what, why))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Index {
    files: Vec<File>,
}

/// A file the vault holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct File {
    pub path: String,
    pub bytes: u64,
    /// The SHA-256 of its bytes, hex.
    pub sha256: String,
    objects: Vec<Object>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Object {
    name: String,
    bytes: u64,
}

/// A path a vault may hold: relative, in parts that are neither empty, `.`
/// nor `..`, so nothing restored from a vault lands outside where it's put.
fn checked(path: &str) -> Result<&str, Fault> {
    let fine = !path.is_empty()
        && !path.starts_with('/')
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..");
    if fine {
        Ok(path)
    } else {
        Err(Fault::unreadable(
            format!("the path {path:?}"),
            "a vault holds only relative paths",
        ))
    }
}

fn hex(digest: &[u8]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Counts and hashes what passes through, read or written.
struct Hashing<T> {
    inner: T,
    hash: Sha256,
    bytes: u64,
}

impl<T> Hashing<T> {
    fn new(inner: T) -> Hashing<T> {
        Hashing {
            inner,
            hash: Sha256::new(),
            bytes: 0,
        }
    }
}

impl<R: Read> Read for Hashing<R> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        let count = self.inner.read(into)?;
        self.hash.update(&into[..count]);
        self.bytes += count as u64;
        Ok(count)
    }
}

impl<W: Write> Write for Hashing<W> {
    fn write(&mut self, from: &[u8]) -> io::Result<usize> {
        let count = self.inner.write(from)?;
        self.hash.update(&from[..count]);
        self.bytes += count as u64;
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Fills a vault, file by file. Nothing it packed can be opened until it's
/// finished, since only then is the index written.
pub struct Packer<'s, S: Store + ?Sized> {
    store: &'s mut S,
    recipient: age::x25519::Recipient,
    files: Vec<File>,
    objects: u64,
    object_bytes: u64,
}

impl<'s, S: Store + ?Sized> Packer<'s, S> {
    pub fn new(store: &'s mut S, key: &VaultKey) -> Packer<'s, S> {
        Packer {
            store,
            recipient: key.recipient(),
            files: vec![],
            objects: 0,
            object_bytes: OBJECT,
        }
    }

    fn encrypt(
        &mut self,
        name: &str,
        what: &str,
    ) -> Result<age::stream::StreamWriter<Box<dyn Write + '_>>, Fault> {
        let encryptor = age::Encryptor::with_recipients(std::iter::once(
            &self.recipient as &dyn age::Recipient,
        ))
        .map_err(|why| unsealable(what, why))?;
        encryptor
            .wrap_output(self.store.create(name)?)
            .map_err(|why| unsealable(what, why))
    }

    /// Packs what the reader gives as `path`.
    pub fn add(&mut self, path: &str, reader: impl Read) -> Result<(), Fault> {
        let path = checked(path)?.to_string();
        if self.files.iter().any(|file| file.path == path) {
            return Err(Fault::unreadable(
                format!("the path {path:?}"),
                "the vault already holds it",
            ));
        }
        // Everything read is packed, so hashing as it's read hashes the file.
        let mut reader = BufReader::with_capacity(1 << 20, Hashing::new(reader));
        let mut objects = vec![];
        let unread = |why: io::Error| Fault::unreadable(format!("{path} to pack"), why);
        while !reader.fill_buf().map_err(unread)?.is_empty() {
            let name = format!("{:08}.age", self.objects);
            self.objects += 1;
            let limit = self.object_bytes;
            let mut writer = self.encrypt(&name, &path)?;
            let bytes = io::copy(&mut (&mut reader).take(limit), &mut writer)
                .map_err(|why| unsealable(&path, why))?;
            writer.finish().map_err(|why| unsealable(&path, why))?;
            objects.push(Object { name, bytes });
        }
        let read = reader.into_inner();
        self.files.push(File {
            path,
            bytes: read.bytes,
            sha256: hex(&read.hash.finalize()),
            objects,
        });
        Ok(())
    }

    /// Writes the index, then the manifest, which says the vault is whole.
    pub fn finish(mut self, manifest: &Manifest) -> Result<(), Fault> {
        let index = serde_json::to_vec(&Index {
            files: std::mem::take(&mut self.files),
        })
        .map_err(|why| unsealable("the vault's index", why))?;
        let mut writer = self.encrypt(INDEX, "the vault's index")?;
        writer
            .write_all(&index)
            .and_then(|()| writer.finish().map(drop))
            .map_err(|why| unsealable("the vault's index", why))?;
        let manifest =
            serde_json::to_vec_pretty(manifest).map_err(|why| unsealable(MANIFEST, why))?;
        self.store
            .create(MANIFEST)?
            .write_all(&manifest)
            .map_err(|why| unsealable(MANIFEST, why))
    }
}

/// What a vault's manifest says, which needs no key.
pub fn manifest(store: &mut dyn Store) -> Result<Manifest, Fault> {
    serde_json::from_reader(store.open(MANIFEST)?).map_err(|why| Fault::unreadable(MANIFEST, why))
}

/// A vault opened with its key.
pub struct Vault<'s, S: Store + ?Sized> {
    store: &'s mut S,
    key: VaultKey,
    files: Vec<File>,
}

impl<'s, S: Store + ?Sized> Vault<'s, S> {
    pub fn open(store: &'s mut S, key: VaultKey) -> Result<Vault<'s, S>, Fault> {
        if !store.has(MANIFEST)? {
            return Err(unopened("the vault", "it was never finished"));
        }
        let index: Index =
            serde_json::from_reader(decrypt(store.open(INDEX)?, &key.0, "the vault's index")?)
                .map_err(|why| unopened("the vault's index", why))?;
        for file in &index.files {
            checked(&file.path)?;
        }
        Ok(Vault {
            store,
            key,
            files: index.files,
        })
    }

    pub fn files(&self) -> &[File] {
        &self.files
    }

    /// Writes a file's bytes out, refusing it if any of them changed since
    /// it was packed. What was written before a refusal mustn't be used.
    pub fn extract(&mut self, path: &str, out: impl Write) -> Result<(), Fault> {
        let file = self
            .files
            .iter()
            .find(|file| file.path == path)
            .ok_or_else(|| unopened(path, "the vault doesn't hold it"))?
            .clone();
        let mut out = Hashing::new(out);
        for object in &file.objects {
            let before = out.bytes;
            let mut reader = decrypt(self.store.open(&object.name)?, &self.key.0, path)?;
            io::copy(&mut reader, &mut out).map_err(|why| unopened(path, why))?;
            if out.bytes - before != object.bytes {
                return Err(unopened(path, "it isn't what was packed"));
            }
        }
        if out.bytes != file.bytes || hex(&out.hash.clone().finalize()) != file.sha256 {
            return Err(unopened(path, "it isn't what was packed"));
        }
        out.flush().map_err(|why| unopened(path, why))
    }
}

#[cfg(test)]
mod tests {
    use super::store::Directory;
    use super::*;

    fn manifest() -> Manifest {
        Manifest {
            format: 1,
            machine: "a laptop".into(),
            opened_by: vec!["the Nano".into(), "the 5Ci".into()],
        }
    }

    #[test]
    fn what_was_packed_comes_back_across_objects() {
        let directory = crate::Scratch::new("vault-back");
        let mut store = Directory::new(&directory);
        let key = VaultKey::generate();
        let big: Vec<u8> = (0..10_000u32).flat_map(u32::to_le_bytes).collect();
        let mut packer = Packer::new(&mut store, &key);
        packer.object_bytes = 4096;
        packer.add("cache/nar/big.nar.zst", &big[..]).unwrap();
        packer.add("empty", &b""[..]).unwrap();
        packer.finish(&manifest()).unwrap();

        let mut vault = Vault::open(&mut store, key).unwrap();
        let paths: Vec<&str> = vault
            .files()
            .iter()
            .map(|file| file.path.as_str())
            .collect();
        assert_eq!(paths, ["cache/nar/big.nar.zst", "empty"]);
        let mut back = vec![];
        vault.extract("cache/nar/big.nar.zst", &mut back).unwrap();
        assert_eq!(back, big);
        let mut empty = vec![];
        vault.extract("empty", &mut empty).unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn no_name_shows_outside_the_manifest() {
        let directory = crate::Scratch::new("vault-names");
        let mut store = Directory::new(&directory);
        let key = VaultKey::generate();
        let mut packer = Packer::new(&mut store, &key);
        packer
            .add("secure-boot/db.key", &b"a private key"[..])
            .unwrap();
        packer.finish(&manifest()).unwrap();
        let mut shown = String::new();
        for entry in walk(&directory) {
            if entry.ends_with(MANIFEST) {
                continue;
            }
            shown += &entry;
            shown += &String::from_utf8_lossy(&std::fs::read(&entry).unwrap());
        }
        assert!(!shown.contains("secure-boot"), "{shown}");
        assert!(!shown.contains("a private key"));
    }

    fn walk(directory: &std::path::Path) -> Vec<String> {
        let mut found = vec![];
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                found.extend(walk(&path));
            } else {
                found.push(path.display().to_string());
            }
        }
        found
    }

    #[test]
    fn a_changed_object_is_refused() {
        let directory = crate::Scratch::new("vault-changed");
        let mut store = Directory::new(&directory);
        let key = VaultKey::generate();
        let mut packer = Packer::new(&mut store, &key);
        packer.add("state", &b"what was carried"[..]).unwrap();
        packer.finish(&manifest()).unwrap();
        let object = directory.join("00000000.age");
        let mut bytes = std::fs::read(&object).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        std::fs::write(&object, bytes).unwrap();

        let mut vault = Vault::open(&mut store, key).unwrap();
        assert!(matches!(
            vault.extract("state", &mut vec![]),
            Err(Fault::Unopened { .. })
        ));
    }

    #[test]
    fn objects_swapped_between_files_are_refused() {
        let directory = crate::Scratch::new("vault-swapped");
        let mut store = Directory::new(&directory);
        let key = VaultKey::generate();
        let mut packer = Packer::new(&mut store, &key);
        packer.add("first", &b"one thing"[..]).unwrap();
        packer.add("second", &b"other one"[..]).unwrap();
        packer.finish(&manifest()).unwrap();
        let first = directory.join("00000000.age");
        let second = directory.join("00000001.age");
        let (a, b) = (
            std::fs::read(&first).unwrap(),
            std::fs::read(&second).unwrap(),
        );
        std::fs::write(&first, b).unwrap();
        std::fs::write(&second, a).unwrap();

        let mut vault = Vault::open(&mut store, key).unwrap();
        assert!(matches!(
            vault.extract("first", &mut vec![]),
            Err(Fault::Unopened { why, .. }) if why == "it isn't what was packed"
        ));
    }

    #[test]
    fn the_key_opens_with_either_key_or_the_passphrase_and_nothing_else() {
        let key = VaultKey::generate();
        let nano = age::x25519::Identity::generate();
        let five = age::x25519::Identity::generate();
        let sealed = key
            .seal(&[&nano.to_public() as &dyn age::Recipient, &five.to_public()])
            .unwrap();
        let passphrase = Secret::new("correct horse battery staple".into());
        let sealed_with_passphrase = key.seal_with_passphrase(&passphrase, Some(10)).unwrap();
        let same = |opened: VaultKey| opened.recipient().to_string() == key.recipient().to_string();

        assert!(same(VaultKey::open(&sealed, &nano).unwrap()));
        assert!(same(VaultKey::open(&sealed, &five).unwrap()));
        assert!(same(
            VaultKey::open_with_passphrase(&sealed_with_passphrase, &passphrase).unwrap()
        ));
        let stranger = age::x25519::Identity::generate();
        assert!(VaultKey::open(&sealed, &stranger).is_err());
        let wrong = Secret::new("incorrect horse".into());
        assert!(VaultKey::open_with_passphrase(&sealed_with_passphrase, &wrong).is_err());
    }

    #[test]
    fn a_vault_never_finished_wont_open() {
        let directory = crate::Scratch::new("vault-unfinished");
        let mut store = Directory::new(&directory);
        let key = VaultKey::generate();
        let mut packer = Packer::new(&mut store, &key);
        packer.add("state", &b"half"[..]).unwrap();
        drop(packer);
        assert!(matches!(
            Vault::open(&mut store, key),
            Err(Fault::Unopened { why, .. }) if why == "it was never finished"
        ));
    }

    #[test]
    fn only_relative_paths_are_packed() {
        for path in ["/etc/shadow", "../outside", "a//b", "a/./b", ""] {
            assert!(checked(path).is_err(), "{path:?}");
        }
        assert_eq!(
            checked("home/.ssh/id_ed25519_sk").unwrap(),
            "home/.ssh/id_ed25519_sk"
        );
    }
}
