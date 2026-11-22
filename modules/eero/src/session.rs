//! The session eero gives once Joel types the code it sends him, kept sealed.
//!
//! The token is the account's whole say over the network, so it's never
//! written in the clear. systemd's credential service seals it for this
//! account, with the TPM where there is one and the machine's own key, so
//! the file opens only here and only for Joel. Opening it takes the TPM
//! about two seconds, which a command pays once and the daemon once a start.

use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// The name the token is sealed under, which systemd checks on opening it.
const SEALED_AS: &str = "eero-session";

/// Where the sealed token is kept: the account's state, since it's neither
/// config nor cache, and losing it means signing in again.
pub fn file() -> io::Result<PathBuf> {
    Ok(folder()?.join("session.cred"))
}

/// eero's folder in the account's state, which the session and the
/// history share.
pub fn folder() -> io::Result<PathBuf> {
    Ok(state()?.join("eero"))
}

/// The account's state folder, which eero's folder and For You's share.
pub fn state() -> io::Result<PathBuf> {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .ok_or_else(|| io::Error::other("neither XDG_STATE_HOME nor HOME is set"))
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
            "couldn't open {}: run `eero sign-in <email or phone>` to seal a new one",
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
    // The folder and the sealed file are the account's alone from the
    // moment they exist, never opened up and narrowed after, so no other
    // account ever has a moment to read the session.
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(folder)?;
    fs::set_permissions(folder, fs::Permissions::from_mode(0o700))?;
    let fresh = file.with_extension("cred.new");
    let _ = fs::remove_file(&fresh);
    let sealed = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&fresh)?;
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
        .write_all(token.as_bytes())?;
    if !seal.wait()?.success() {
        let _ = fs::remove_file(&fresh);
        return Err(io::Error::other("systemd-creds couldn't seal the session"));
    }
    fs::rename(&fresh, &file)
}
