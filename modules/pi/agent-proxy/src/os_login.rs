//! The key an agent's ssh logs in to OS Login hosts with, made by the
//! proxy as it starts and held only in its memory, and the hosts it
//! signs those logins to.
//!
//! A cloud's OS Login hosts take the keys in the person's login profile.
//! The person's own key there is sealed in the TPM, whose agent refuses
//! any process an agent started, so on 24 November an agent's `gcloud
//! compute ssh` failed, and an agent made a 12-hour key of its own on
//! disk to get its work done. So the proxy makes a key at each start,
//! from the kernel's randomness, and writes out only its public half
//! (`--os-login`, the folder its socket is in), where the account's own
//! service puts it in the person's profile for a few hours at a time
//! (agent-proxy/os-login.sh) and ssh finds it to offer (modules/pi
//! nixos.nix). The private half never leaves the proxy, and goes with it.
//!
//! It signs with that key only a login to a session with a host the
//! known-hosts file gcloud keeps names as an instance, `compute.<id>`
//! (`--os-login-hosts`), read afresh at each binding, since gcloud adds
//! each instance's key as it first reaches it, taken from the cloud's
//! own record of that instance where the cloud has one. Other keys never
//! sign for such a host, nor that key for any other. A host `--hosts`
//! names, or a builder lent, stays what it was though gcloud knows it
//! too: the person named it, so the agents' keys and the builders' still
//! sign for it as before, and this key doesn't.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write as _};
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use signature::Signer as _;
use ssh_key::private::{Ed25519Keypair, KeypairData};
use ssh_key::public::KeyData;
use ssh_key::{PrivateKey, PublicKey, Signature};

/// The file the key's public half is written to, in the `--os-login`
/// folder, for ssh to offer and the account's service to register.
pub const PUBLIC_FILE: &str = "os-login.pub";

/// The comment the key is listed and written with.
pub const COMMENT: &str = "pi-agent-proxy OS Login";

/// What every name gcloud gives an instance in its known hosts starts
/// with, before the instance's number.
const INSTANCE: &str = "compute.";

/// The most of the known-hosts file read at a binding. gcloud's grows by
/// a line of about a hundred bytes an instance, so this is tens of
/// thousands of instances, and a file grown past it can't stall the
/// binding every agent waits on.
const MOST_KNOWN: u64 = 8 * 1024 * 1024;

/// The key the proxy made, and the known-hosts file naming the hosts it
/// signs logins to with it.
pub struct OsLogin {
    key: PrivateKey,
    blob: Vec<u8>,
    known: PathBuf,
}

impl OsLogin {
    /// A key made afresh, its public half written into `folder`, for the
    /// hosts `known` names as instances.
    ///
    /// # Errors
    ///
    /// When the kernel gives no randomness, or the folder can't be
    /// written: the proxy then signs no OS Login logins, and goes on
    /// with the rest.
    pub fn start(folder: &Path, known: PathBuf) -> io::Result<Self> {
        let mut seed = [0; 32];
        File::open("/dev/urandom")?.read_exact(&mut seed)?;
        let keypair = Ed25519Keypair::from_seed(&seed);
        seed.fill(0);
        let made = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        let key = PrivateKey::new(KeypairData::from(keypair), format!("{COMMENT} {made}"))
            .map_err(io::Error::other)?;
        let public = key.public_key().to_openssh().map_err(io::Error::other)?;
        let blob = key.public_key().to_bytes().map_err(io::Error::other)?;
        // Written whole beside it and moved into place, so ssh and the
        // service never read half a key; never through a link left there.
        let written = folder.join(format!("{PUBLIC_FILE}.new"));
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&written)?
            .write_all(format!("{public}\n").as_bytes())?;
        fs::rename(&written, folder.join(PUBLIC_FILE))?;
        Ok(OsLogin { key, blob, known })
    }

    /// The key's wire blob.
    pub fn blob(&self) -> &[u8] {
        &self.blob
    }

    /// Whether `host` is an OS Login host: the key of an instance the
    /// known-hosts file names. A file that can't be read, as on a machine
    /// whose gcloud has reached none, names none, and so does anything
    /// but a plain file, which could hold the binding open; past
    /// [`MOST_KNOWN`], only what comes before it is read.
    pub fn knows(&self, host: &KeyData) -> bool {
        let Ok(wanted) = PublicKey::from(host.clone()).to_openssh() else {
            return false;
        };
        let mut text = Vec::new();
        let read = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&self.known)
            .and_then(|file| {
                if !file.metadata()?.is_file() {
                    return Err(io::Error::other("not a plain file"));
                }
                file.take(MOST_KNOWN).read_to_end(&mut text)
            });
        read.is_ok()
            && String::from_utf8_lossy(&text)
                .lines()
                .any(|line| names_instance(line, &wanted))
    }

    /// The key's signature over `data`, in the agent protocol's shape.
    pub fn sign(&self, data: &[u8]) -> Option<Vec<u8>> {
        let signed: Signature = self.key.try_sign(data).ok()?;
        Vec::try_from(signed).ok()
    }
}

/// Whether a known-hosts `line` names an instance, `compute.<id>` among
/// its names, with the host key `wanted` (`<kind> <base64>`). A marked
/// line (`@revoked`, `@cert-authority`), a hashed one and a comment name
/// none.
fn names_instance(line: &str, wanted: &str) -> bool {
    if line.trim_start().starts_with(['@', '#']) {
        return false;
    }
    let mut fields = line.split_whitespace();
    let (Some(names), Some(kind), Some(key)) = (fields.next(), fields.next(), fields.next()) else {
        return false;
    };
    let instance = names.split(',').any(|name| {
        name.strip_prefix(INSTANCE)
            .is_some_and(|id| !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()))
    });
    instance && wanted.split_once(' ') == Some((kind, key))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJBWcxb/Blaqt1auOtE+F8QUWrUotiC5qBJ+UuEWdVCb";

    #[test]
    fn a_line_naming_an_instance_names_it() {
        for line in [
            format!("compute.8034457558281374540 {KEY}"),
            format!("10.0.0.4,compute.1 {KEY} a comment"),
        ] {
            assert!(names_instance(&line, KEY), "{line:?} names an instance");
        }
    }

    #[test]
    fn a_line_of_another_shape_names_none() {
        for line in [
            format!("basin-1 {KEY}"),
            format!("compute. {KEY}"),
            format!("compute.12a {KEY}"),
            format!("@revoked compute.1 {KEY}"),
            format!("# compute.1 {KEY}"),
            format!("|1|abc=|def= {KEY}"),
            "compute.1 ssh-ed25519".to_owned(),
            String::new(),
        ] {
            assert!(!names_instance(&line, KEY), "{line:?} names no instance");
        }
    }

    #[test]
    fn a_line_with_another_key_names_none() {
        let other =
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        assert!(
            !names_instance(&format!("compute.1 {other}"), KEY),
            "an instance with another host key is named"
        );
    }
}
