//! The launcher's passwords, sealed under one key.
//!
//! Opening a secret through the TPM takes about two seconds, and the TPM
//! does one thing at a time, so sealing each network's password there made
//! every boot, join and save wait two seconds a network. Instead, one key is
//! sealed there, under the name wifid-vault, with the TPM where there is one
//! and the install's own key, as each password was before. wifid opens it
//! once as it starts and keeps it on a page of its own, locked in memory
//! and left out of core dumps. Each password is sealed under it with
//! XChaCha20-Poly1305 and a random nonce, which at 24 bytes never repeats
//! by chance, and with the name it's kept under as associated data, so one
//! network's password can't stand in for another's.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;
use std::ptr::{self, NonNull};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chacha20poly1305::aead::{Aead, Generate, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use zeroize::{Zeroize, Zeroizing};

use crate::secret::Credentials;

/// The name the key is sealed under, which systemd checks on opening it.
const SEALED_AS: &str = "wifid-vault";

/// The first byte of every sealed password, so a later way of sealing can
/// be told from this one.
const SEALED_THIS_WAY: u8 = 1;

const NONCE: usize = 24;

/// The key every launcher password is sealed under.
pub struct Vault {
    key: Page,
}

impl Vault {
    /// The vault whose key is sealed in this file, making a key the first
    /// time. A file that won't open to a key is refused rather than
    /// replaced, since a new key would lose every password sealed under the
    /// old one.
    pub async fn kept_in(file: &Path, credentials: &Credentials) -> io::Result<Vault> {
        let mut key = Page::new()?;
        match fs::read_to_string(file) {
            Ok(sealed) => {
                let text = credentials.unseal(SEALED_AS, sealed.trim_end()).await?;
                let bytes = STANDARD.decode(text.as_bytes()).map(Zeroizing::new);
                match bytes {
                    Ok(bytes) if bytes.len() == key.bytes().len() => {
                        key.bytes_mut().copy_from_slice(&bytes);
                    }
                    _ => {
                        let reason = format!("{} doesn't hold wifid's key", file.display());
                        return Err(io::Error::new(io::ErrorKind::InvalidData, reason));
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                File::open("/dev/urandom")?.read_exact(key.bytes_mut())?;
                let text = Zeroizing::new(STANDARD.encode(key.bytes()));
                let sealed = credentials.seal(SEALED_AS, &text).await?;
                keep(file, &sealed)?;
            }
            Err(error) => return Err(error),
        }
        Ok(Vault { key })
    }

    /// A vault with this key, kept nowhere.
    #[cfg(test)]
    pub fn with_key(bytes: [u8; 32]) -> io::Result<Vault> {
        let mut key = Page::new()?;
        key.bytes_mut().copy_from_slice(&bytes);
        Ok(Vault { key })
    }

    /// The password sealed under this name, as text to keep in a file.
    pub fn seal(&self, name: &str, password: &str) -> io::Result<String> {
        let nonce = XNonce::try_generate().map_err(|error| io::Error::other(error.to_string()))?;
        let payload = Payload {
            msg: password.as_bytes(),
            aad: name.as_bytes(),
        };
        let sealed = self
            .cipher()
            .encrypt(&nonce, payload)
            .map_err(|_| io::Error::other("the password is too long to seal"))?;
        let mut kept = vec![SEALED_THIS_WAY];
        kept.extend_from_slice(&nonce);
        kept.extend_from_slice(&sealed);
        Ok(STANDARD.encode(kept))
    }

    /// Opens a password sealed under this name, refusing one sealed under
    /// another, changed since, or sealed by another vault.
    pub fn open(&self, name: &str, sealed: &str) -> io::Result<Zeroizing<String>> {
        let unopenable = || {
            let reason = format!(
                "the password kept for {name} won't open: it was sealed for another network \
                 or on another machine, or it has changed since"
            );
            io::Error::new(io::ErrorKind::InvalidData, reason)
        };
        let kept = STANDARD
            .decode(sealed.trim_end())
            .map_err(|_| unopenable())?;
        let Some((&SEALED_THIS_WAY, kept)) = kept.split_first() else {
            return Err(unopenable());
        };
        if kept.len() < NONCE {
            return Err(unopenable());
        }
        let (nonce, sealed) = kept.split_at(NONCE);
        let nonce = XNonce::try_from(nonce).map_err(|_| unopenable())?;
        let payload = Payload {
            msg: sealed,
            aad: name.as_bytes(),
        };
        let opened = self
            .cipher()
            .decrypt(&nonce, payload)
            .map_err(|_| unopenable())?;
        let opened = Zeroizing::new(opened);
        let password = std::str::from_utf8(&opened).map_err(|_| unopenable())?;
        Ok(Zeroizing::new(password.to_string()))
    }

    /// The cipher under the key, which zeroes its own copy of the key once
    /// it's done with.
    fn cipher(&self) -> XChaCha20Poly1305 {
        let key = <&Key>::try_from(self.key.bytes()).expect("the page holds a whole key");
        XChaCha20Poly1305::new(key)
    }
}

/// Keeps the sealed key, whole or not at all, and on the disk before it's
/// used: a key lost to a power cut would take every password with it.
fn keep(file: &Path, sealed: &str) -> io::Result<()> {
    let partial = file.with_extension("partial");
    let mut written = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&partial)?;
    written.write_all(sealed.as_bytes())?;
    // A partial file left by a wifid that stopped part-way keeps the mode it
    // was made with, which mode() above doesn't change.
    fs::set_permissions(&partial, fs::Permissions::from_mode(0o600))?;
    written.sync_all()?;
    fs::rename(&partial, file)?;
    let folder = file
        .parent()
        .filter(|folder| !folder.as_os_str().is_empty());
    File::open(folder.unwrap_or(Path::new(".")))?.sync_all()
}

/// A page of memory of the key's own, mapped apart from the heap so nothing
/// else shares it. It's locked so it's never written to swap, left out of
/// core dumps, and zeroed before it's given back.
struct Page {
    start: NonNull<u8>,
    length: usize,
}

// The page is wifid's alone, written only while the vault is made.
unsafe impl Send for Page {}
unsafe impl Sync for Page {}

impl Page {
    const KEY: usize = 32;

    fn new() -> io::Result<Page> {
        // SAFETY: sysconf only reads a setting.
        let length = usize::try_from(unsafe { libc::sysconf(libc::_SC_PAGESIZE) })
            .map_err(|_| io::Error::last_os_error())?;
        // SAFETY: an anonymous mapping of fresh pages touches no memory
        // already in use.
        let start = unsafe {
            libc::mmap(
                ptr::null_mut(),
                length,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        if start == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }
        let page = Page {
            start: NonNull::new(start.cast()).ok_or_else(io::Error::last_os_error)?,
            length,
        };
        // Neither is worth stopping Wi-Fi for: the key is still kept
        // nowhere but in wifid's memory.
        // SAFETY: both act on the mapping just made, and on nothing else.
        if unsafe { libc::mlock(start, length) } != 0 {
            let error = io::Error::last_os_error();
            eprintln!("<4>can't keep wifid's key out of swap: {error}");
        }
        // SAFETY: as above.
        if unsafe { libc::madvise(start, length, libc::MADV_DONTDUMP) } != 0 {
            let error = io::Error::last_os_error();
            eprintln!("<4>can't keep wifid's key out of core dumps: {error}");
        }
        Ok(page)
    }

    fn bytes(&self) -> &[u8] {
        // SAFETY: the key's bytes are the start of a page that lives as
        // long as this does, and longer than a page is no key.
        unsafe { std::slice::from_raw_parts(self.start.as_ptr(), Self::KEY) }
    }

    fn bytes_mut(&mut self) -> &mut [u8] {
        // SAFETY: as in bytes(), borrowed once as this is.
        unsafe { std::slice::from_raw_parts_mut(self.start.as_ptr(), Self::KEY) }
    }
}

impl Drop for Page {
    fn drop(&mut self) {
        self.bytes_mut().zeroize();
        // SAFETY: the page is this one's own mapping, used by nothing once
        // this is dropped.
        unsafe {
            libc::munlock(self.start.as_ptr().cast(), self.length);
            libc::munmap(self.start.as_ptr().cast(), self.length);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use serde_json::{Value, json};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixListener;

    use super::*;

    fn vault() -> Vault {
        Vault::with_key([7; 32]).unwrap()
    }

    /// A stand-in for systemd's credential service that answers one call
    /// with this reply, and hands back the call it was asked.
    fn service(socket: &Path, reply: Value) -> tokio::task::JoinHandle<Value> {
        let listener = UnixListener::bind(socket).unwrap();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = BufReader::new(stream);
            let mut call = Vec::new();
            stream.read_until(0, &mut call).await.unwrap();
            call.pop();
            let mut answer = serde_json::to_vec(&reply).unwrap();
            answer.push(0);
            stream.get_mut().write_all(&answer).await.unwrap();
            serde_json::from_slice(&call).unwrap()
        })
    }

    /// The call the stand-in was asked, or none when nothing asked it
    /// within a few seconds.
    async fn call_to(service: tokio::task::JoinHandle<Value>) -> Value {
        let waited = tokio::time::timeout(std::time::Duration::from_secs(3), service).await;
        waited.map_or(Value::Null, Result::unwrap)
    }

    #[test]
    fn a_sealed_password_opens_under_its_name() {
        let vault = vault();

        let sealed = vault.seal("lantern", "glowing in the dark").unwrap();

        assert_eq!(
            *vault.open("lantern", &sealed).unwrap(),
            "glowing in the dark"
        );
        assert!(!sealed.contains("glowing"), "{sealed}");
    }

    #[test]
    fn sealing_the_same_password_twice_reads_differently() {
        let vault = vault();

        let once = vault.seal("lantern", "glowing in the dark").unwrap();
        let twice = vault.seal("lantern", "glowing in the dark").unwrap();

        assert_ne!(once, twice);
    }

    #[test]
    fn a_vault_kept_in_a_file_seals_its_key_through_systemd_once() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("vault.cred");
        let socket = dir.path().join("credentials");
        let credentials = Credentials::at(&socket);

        let (made, asked) = runtime.block_on(async {
            let asked = service(&socket, json!({"parameters": {"blob": "c2VhbGVk"}}));
            let made = Vault::kept_in(&file, &credentials).await;
            (made, call_to(asked).await)
        });

        let made = made.unwrap();
        assert_eq!(asked["method"], "io.systemd.Credentials.Encrypt");
        assert_eq!(asked["parameters"]["name"], "wifid-vault");
        assert_eq!(fs::read_to_string(&file).unwrap(), "c2VhbGVk");
        assert_eq!(
            fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let key = asked["parameters"]["text"].as_str().unwrap().to_string();
        let sealed = made.seal("lantern", "glowing in the dark").unwrap();
        fs::remove_file(&socket).unwrap();
        let (opened, asked) = runtime.block_on(async {
            let asked = service(&socket, json!({"parameters": {"data": base64(&key)}}));
            let opened = Vault::kept_in(&file, &credentials).await;
            (opened, call_to(asked).await)
        });
        assert_eq!(
            asked,
            json!({
                "method": "io.systemd.Credentials.Decrypt",
                "parameters": {"name": "wifid-vault", "blob": "c2VhbGVk"},
            })
        );
        let opened = opened.unwrap();
        assert_eq!(
            *opened.open("lantern", &sealed).unwrap(),
            "glowing in the dark"
        );
    }

    fn base64(text: &str) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(text)
    }

    #[test]
    fn a_password_sealed_under_one_name_wont_open_under_another() {
        let vault = vault();
        let sealed = vault.seal("lantern", "glowing in the dark").unwrap();

        let refused = vault.open("roadside-inn", &sealed);

        assert_eq!(refused.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn a_sealed_password_changed_since_wont_open() {
        let vault = vault();
        let sealed = vault.seal("lantern", "glowing in the dark").unwrap();
        let mut changed = sealed.into_bytes();
        let last = changed.len() - 3;
        changed[last] = if changed[last] == b'A' { b'B' } else { b'A' };

        let refused = vault.open("lantern", &String::from_utf8(changed).unwrap());

        assert_eq!(refused.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn another_vault_cant_open_it() {
        let sealed = vault().seal("lantern", "glowing in the dark").unwrap();

        let refused = Vault::with_key([8; 32]).unwrap().open("lantern", &sealed);

        assert_eq!(refused.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn text_that_was_never_sealed_wont_open() {
        let refused = vault().open("lantern", "glowing in the dark");

        assert_eq!(refused.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn a_kept_key_that_isnt_one_is_refused_not_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("vault.cred");
        fs::write(&file, "c2VhbGVk").unwrap();
        let socket = dir.path().join("credentials");
        let asked = service(&socket, json!({"parameters": {"data": base64("short")}}));

        let refused = Vault::kept_in(&file, &Credentials::at(&socket)).await;

        call_to(asked).await;
        assert_eq!(refused.err().unwrap().kind(), io::ErrorKind::InvalidData);
        assert_eq!(fs::read_to_string(&file).unwrap(), "c2VhbGVk");
    }
}
