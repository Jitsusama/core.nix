//! What a TV allowed this account: the token it handed once Joel pressed
//! Allow, and the pin of the certificate it answered with, kept sealed.
//!
//! The token lets anything holding it press the TV's keys without asking,
//! so it's never written in the clear: systemd's credential service seals
//! it for this account, as eero's session is, with the TPM where there is
//! one and the machine's own key, so the file opens only here and only for
//! Joel, and a fenced agent, kept from that service, can't open it.

use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

use serde_json::{Value, json};

/// A certificate's SHA-256.
pub type Pin = [u8; 32];

/// The name the TV with hardware address `mac` is sealed under, which
/// systemd checks on opening its file, so no other sealed file, another
/// TV's among them, can stand in for it.
fn sealed_as(mac: &str) -> String {
    format!("net-television-{}", plain_mac(mac))
}

/// A hardware address as its hex digits alone, in lowercase.
fn plain_mac(mac: &str) -> String {
    mac.chars()
        .filter(char::is_ascii_hexdigit)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// What a TV allowed: its token, none until it hands one, and its pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kept {
    pub token: Option<String>,
    pub pin: Pin,
}

impl Kept {
    /// What's sealed: one JSON object, the pin as hex.
    pub fn text(&self) -> String {
        json!({"token": self.token, "pin": hex(&self.pin)}).to_string()
    }

    /// What `text` gave, or none for anything else.
    pub fn read(text: &str) -> Option<Kept> {
        let said: Value = serde_json::from_str(text).ok()?;
        let pin = unhex(said.get("pin")?.as_str()?)?;
        let token = match said.get("token")? {
            Value::Null => None,
            Value::String(token) => Some(token.clone()),
            _ => return None,
        };
        Some(Kept { token, pin })
    }
}

/// `bytes` as lowercase hex, two digits each.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut text, byte| {
        // Writing to a String can't fail.
        let _ = write!(text, "{byte:02x}");
        text
    })
}

fn unhex(text: &str) -> Option<Pin> {
    if text.len() != 64 || !text.is_ascii() {
        return None;
    }
    let mut pin = [0; 32];
    for (at, byte) in pin.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[at * 2..at * 2 + 2], 16).ok()?;
    }
    Some(pin)
}

/// The folder the TVs' files are kept in: the account's state, since it's
/// neither config nor cache, and losing it means pressing Allow again.
pub fn folder() -> io::Result<PathBuf> {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .map(|state| state.join("net/televisions"))
        .ok_or_else(|| io::Error::other("neither XDG_STATE_HOME nor HOME is set"))
}

/// The file the TV with hardware address `mac` is kept in, in `folder`.
pub fn file_in(folder: &Path, mac: &str) -> PathBuf {
    folder.join(format!("{}.cred", plain_mac(mac)))
}

/// Why what a TV allowed couldn't be opened, told apart by whether
/// removing its file would help: only a file that won't open as a TV's
/// is worth forgetting.
#[derive(Debug)]
pub enum Unopened {
    /// The file couldn't be looked at, or no folder is known for it.
    Unseen(io::Error),
    /// systemd-creds couldn't be run.
    Unrun(io::Error),
    /// systemd-creds ran and couldn't open the file: it was sealed on
    /// another machine or for another account, or it's damaged.
    Unsealed,
    /// The file opened and doesn't hold a TV.
    Unread,
}

/// What was kept of the TV with hardware address `mac`: none where it has
/// never allowed this account, and why where its file won't open.
pub fn open(mac: &str) -> Result<Option<Kept>, Unopened> {
    let file = file_in(&folder().map_err(Unopened::Unseen)?, mac);
    // Only a file that isn't there is a TV never allowed: one that can't
    // be looked at fails, so its pin is never silently dropped.
    match fs::symlink_metadata(&file) {
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(Unopened::Unseen(error)),
    }
    let output = Command::new("systemd-creds")
        .args(["--user", "decrypt", &format!("--name={}", sealed_as(mac))])
        .arg(&file)
        .arg("-")
        .stderr(Stdio::inherit())
        .output()
        .map_err(Unopened::Unrun)?;
    if !output.status.success() {
        return Err(Unopened::Unsealed);
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Kept::read(text.trim()).map(Some).ok_or(Unopened::Unread)
}

/// Seals what the TV with hardware address `mac` allowed, replacing what
/// was kept only once the new file is whole on the disk. A half-made
/// file goes whatever stops it, and one a stopped net left goes the next
/// time its TV is kept.
pub fn keep(mac: &str, kept: &Kept) -> io::Result<()> {
    let folder = folder()?;
    own_folder(&folder)?;
    let file = file_in(&folder, mac);
    sweep(&folder, &plain_mac(mac));
    let fresh = file.with_extension(format!("cred.{}.new", std::process::id()));
    let _ = fs::remove_file(&fresh);
    let sealed = seal(mac, kept, &fresh).and_then(|()| fs::rename(&fresh, &file));
    if sealed.is_err() {
        let _ = fs::remove_file(&fresh);
    }
    sealed
}

/// Seals `kept` into `fresh`, whole on the disk when it answers.
fn seal(mac: &str, kept: &Kept, fresh: &Path) -> io::Result<()> {
    let sealed = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(fresh)?;
    let whole = sealed.try_clone()?;
    let mut seal = Command::new("systemd-creds")
        .args([
            "--user",
            "encrypt",
            &format!("--name={}", sealed_as(mac)),
            "-",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(sealed)
        .stderr(Stdio::inherit())
        .spawn()?;
    let written = seal
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("no stdin"))
        .and_then(|mut stdin| stdin.write_all(kept.text().as_bytes()));
    // The child is waited on even when writing to it failed, so none is
    // left behind; its stdin closed above, so it ends.
    let ended = seal.wait();
    written?;
    if !ended?.success() {
        return Err(io::Error::other(
            "systemd-creds couldn't seal the TV's token",
        ));
    }
    whole.sync_all()
}

/// How old a half-made file must be before it's swept: longer than any
/// press takes, the wait for Allow and the seal after it together, so a
/// net still sealing is never swept, even one in another pid namespace,
/// whose pid this machine's /proc doesn't show.
const STALE: Duration = Duration::from_secs(120);

/// Removes the TV `name`'s half-made files in `folder` whose net has
/// stopped, by the process each names and its age; one still running, or
/// begun too lately to tell, keeps its own.
fn sweep(folder: &Path, name: &str) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        let file = entry.file_name();
        if let Some(pid) = half_made(&file.to_string_lossy(), name)
            && !Path::new(&format!("/proc/{pid}")).exists()
            && stale(&entry, now)
        {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// Whether `entry` was last written longer than `STALE` before `now`; one
/// whose age can't be read is kept.
fn stale(entry: &fs::DirEntry, now: SystemTime) -> bool {
    entry
        .metadata()
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|written| now.duration_since(written).ok())
        .is_some_and(|age| age > STALE)
}

/// The process that began `file`, where it's a half-made file of the TV
/// `name`: `<name>.cred.<pid>.new`.
fn half_made(file: &str, name: &str) -> Option<u32> {
    file.strip_prefix(name)?
        .strip_prefix(".cred.")?
        .strip_suffix(".new")?
        .parse()
        .ok()
}

/// Makes `folder` the account's alone from the moment it exists.
fn own_folder(folder: &Path) -> io::Result<()> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(folder)?;
    fs::set_permissions(folder, fs::Permissions::from_mode(0o700))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kept(token: Option<&str>) -> Kept {
        let mut pin = [0; 32];
        for (at, byte) in pin.iter_mut().enumerate() {
            *byte = u8::try_from(at * 7).expect("it fits");
        }
        Kept {
            token: token.map(str::to_owned),
            pin,
        }
    }

    #[test]
    fn what_is_kept_reads_back_as_it_was() {
        for one in [kept(Some("12345678")), kept(None)] {
            assert_eq!(Kept::read(&one.text()), Some(one));
        }
    }

    #[test]
    fn a_file_holding_anything_else_is_no_tv() {
        let zeros = "0".repeat(64);
        let unhex = format!("zz{}", "0".repeat(62));
        for text in [
            String::new(),
            "{}".into(),
            r#"{"token": "1", "pin": "00"}"#.into(),
            json!({"token": 1, "pin": zeros}).to_string(),
            json!({"pin": zeros}).to_string(),
            json!({"token": null, "pin": unhex}).to_string(),
        ] {
            let text = text.as_str();
            assert_eq!(Kept::read(text), None, "{text}");
        }
    }

    #[test]
    fn a_tv_is_kept_by_its_hardware_address_in_any_case() {
        let folder = Path::new("/state/net/televisions");
        let file = folder.join("020000000090.cred");
        assert_eq!(file_in(folder, "02:00:00:00:00:90"), file);
        assert_eq!(file_in(folder, "02-00-00-00-00-90"), file);
        assert_eq!(
            file_in(folder, "02:00:00:00:00:90".to_uppercase().as_str()),
            file
        );
    }

    #[test]
    fn each_tv_is_sealed_under_a_name_of_its_own() {
        assert_eq!(
            sealed_as("02:00:00:00:00:90"),
            "net-television-020000000090"
        );
        assert_ne!(
            sealed_as("02:00:00:00:00:90"),
            sealed_as("02:00:00:00:00:91")
        );
    }

    #[test]
    fn only_a_tvs_own_half_made_files_are_read_as_its_own() {
        let name = "020000000090";
        assert_eq!(half_made("020000000090.cred.4242.new", name), Some(4242));
        for file in [
            "020000000090.cred",
            "020000000091.cred.4242.new",
            "020000000090.cred.x.new",
            "020000000090.cred.4242",
        ] {
            assert_eq!(half_made(file, name), None, "{file}");
        }
    }

    /// `file`, written `ago` before now.
    fn written(file: &Path, ago: Duration) {
        fs::write(file, "").expect("written");
        fs::File::options()
            .write(true)
            .open(file)
            .and_then(|opened| opened.set_modified(SystemTime::now() - ago))
            .expect("its time set");
    }

    #[test]
    fn only_an_old_half_made_file_whose_net_stopped_is_swept() {
        let folder = std::env::temp_dir().join(format!("net-kept-{}", std::process::id()));
        fs::create_dir_all(&folder).expect("a folder");
        let name = "020000000090";
        let old = STALE * 2;
        // No process has the largest id a pid may be, or one less.
        let stopped = folder.join(format!("{name}.cred.4194304.new"));
        let sealing = folder.join(format!("{name}.cred.4194303.new"));
        let running = folder.join(format!("{name}.cred.{}.new", std::process::id()));
        let kept = folder.join(format!("{name}.cred"));
        written(&stopped, old);
        written(&sealing, Duration::from_secs(5));
        written(&running, old);
        written(&kept, old);
        sweep(&folder, name);
        let left = [&stopped, &sealing, &running, &kept].map(|file| file.exists());
        let _ = fs::remove_dir_all(&folder);
        assert_eq!(
            left,
            [false, true, true, true],
            "stopped, sealing in another pid namespace, running, kept"
        );
    }

    #[test]
    fn no_hardware_address_can_name_a_file_outside_the_folder() {
        let folder = Path::new("/state/net/televisions");
        assert_eq!(
            file_in(folder, "../../../etc/passwd"),
            folder.join("ecad.cred")
        );
    }
}
