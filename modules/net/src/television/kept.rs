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

use serde_json::{Value, json};

/// A certificate's SHA-256.
pub type Pin = [u8; 32];

/// The name each TV's file is sealed under, which systemd checks on
/// opening it, so no other sealed file can stand in for one.
const SEALED_AS: &str = "net-television";

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
    let name: String = mac
        .chars()
        .filter(char::is_ascii_hexdigit)
        .map(|c| c.to_ascii_lowercase())
        .collect();
    folder.join(format!("{name}.cred"))
}

/// What was kept of the TV with hardware address `mac`: none where it has
/// never allowed this account, and an error where its file won't open.
pub fn open(mac: &str) -> io::Result<Option<Kept>> {
    let file = file_in(&folder()?, mac);
    if !file.exists() {
        return Ok(None);
    }
    let output = Command::new("systemd-creds")
        .args(["--user", "decrypt", &format!("--name={SEALED_AS}")])
        .arg(&file)
        .arg("-")
        .stderr(Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "systemd-creds couldn't open {}",
            file.display()
        )));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Kept::read(text.trim())
        .map(Some)
        .ok_or_else(|| io::Error::other(format!("{} doesn't hold a TV", file.display())))
}

/// Seals what the TV with hardware address `mac` allowed, replacing what
/// was kept only once the new file is whole on the disk.
pub fn keep(mac: &str, kept: &Kept) -> io::Result<()> {
    let folder = folder()?;
    own_folder(&folder)?;
    let file = file_in(&folder, mac);
    let fresh = file.with_extension(format!("cred.{}.new", std::process::id()));
    let _ = fs::remove_file(&fresh);
    let sealed = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&fresh)?;
    let whole = sealed.try_clone()?;
    let mut seal = Command::new("systemd-creds")
        .args([
            "--user",
            "encrypt",
            &format!("--name={SEALED_AS}"),
            "-",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(sealed)
        .stderr(Stdio::inherit())
        .spawn()?;
    seal.stdin
        .take()
        .ok_or_else(|| io::Error::other("no stdin"))?
        .write_all(kept.text().as_bytes())?;
    if !seal.wait()?.success() {
        let _ = fs::remove_file(&fresh);
        return Err(io::Error::other(
            "systemd-creds couldn't seal the TV's token",
        ));
    }
    whole.sync_all()?;
    fs::rename(&fresh, &file)
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
    fn no_hardware_address_can_name_a_file_outside_the_folder() {
        let folder = Path::new("/state/net/televisions");
        assert_eq!(
            file_in(folder, "../../../etc/passwd"),
            folder.join("ecad.cred")
        );
    }
}
