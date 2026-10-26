//! The session eero gives once Joel types the code it sends him, kept sealed.
//!
//! The token is the account's whole say over the network, so it's never
//! written in the clear. systemd's credential service seals it for this
//! account, with the TPM where there is one and the machine's own key, so
//! the file opens only here and only for Joel. Opening it takes the TPM
//! about two seconds, which a command pays once and the watch once a start.

use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// The name the token is sealed under, which systemd checks on opening it.
const SEALED_AS: &str = "eero-session";

/// Where the sealed token is kept: the account's state, since it's neither
/// config nor cache, and losing it means signing in again.
pub fn file() -> io::Result<PathBuf> {
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .ok_or_else(|| io::Error::other("neither XDG_STATE_HOME nor HOME is set"))?;
    Ok(state.join("eero/session.cred"))
}

/// The token, opened from its sealed file.
pub fn open() -> io::Result<String> {
    let file = file()?;
    if !file.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "not signed in to eero: run `eero sign-in <email or phone>`",
        ));
    }
    let output = Command::new("systemd-creds")
        .args(["--user", "decrypt", &format!("--name={SEALED_AS}")])
        .arg(&file)
        .arg("-")
        .stderr(Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "couldn't open {}",
            file.display()
        )));
    }
    String::from_utf8(output.stdout)
        .map(|token| token.trim().to_owned())
        .map_err(|_| io::Error::other("the sealed session isn't text"))
}

/// Seals the token in its file, replacing any before it only once the new
/// one is whole.
pub fn keep(token: &str) -> io::Result<()> {
    let file = file()?;
    let folder = file.parent().ok_or_else(|| io::Error::other("no folder"))?;
    fs::create_dir_all(folder)?;
    let fresh = file.with_extension("cred.new");
    let mut seal = Command::new("systemd-creds")
        .args(["--user", "encrypt", &format!("--name={SEALED_AS}"), "-"])
        .arg(&fresh)
        .stdin(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    seal.stdin
        .take()
        .ok_or_else(|| io::Error::other("no stdin"))?
        .write_all(token.as_bytes())?;
    if !seal.wait()?.success() {
        let _ = fs::remove_file(&fresh);
        return Err(io::Error::other("systemd-creds couldn't seal the session"));
    }
    fs::rename(&fresh, &file)
}
