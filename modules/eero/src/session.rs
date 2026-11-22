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
use std::path::{Path, PathBuf};
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
/// one is whole and on the disk. The daemon and a command may both seal
/// at once, as when each refreshes an aged session, so each seals into a
/// file named for its own process, and neither moves or removes the
/// other's half-written one.
pub fn keep(token: &str) -> io::Result<()> {
    let file = file()?;
    let fresh = sealing(&file, std::process::id());
    let sealed = private(&fresh)?;
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
        .write_all(token.as_bytes())?;
    if !seal.wait()?.success() {
        let _ = fs::remove_file(&fresh);
        return Err(io::Error::other("systemd-creds couldn't seal the session"));
    }
    whole.sync_all()?;
    fs::rename(&fresh, &file)?;
    let _ = forget_unfinished(&file);
    Ok(())
}

/// The file process `pid` seals a new session into, beside `file`.
fn sealing(file: &Path, pid: u32) -> PathBuf {
    file.with_extension(format!("cred.{pid}.new"))
}

/// Removes what a sealer that died partway left beside `file`: its
/// process is gone, so it will never finish or move it.
fn forget_unfinished(file: &Path) -> io::Result<()> {
    let folder = file.parent().ok_or_else(|| io::Error::other("no folder"))?;
    for entry in fs::read_dir(folder)? {
        let name = entry?.file_name();
        let pid = name
            .to_str()
            .and_then(|name| name.strip_prefix("session.cred."))
            .and_then(|rest| rest.strip_suffix(".new"))
            .and_then(|pid| pid.parse::<u32>().ok());
        if let Some(pid) = pid
            && !Path::new(&format!("/proc/{pid}")).exists()
        {
            let _ = fs::remove_file(folder.join(&name));
        }
    }
    Ok(())
}

/// A fresh file at `path`, and its folder, the account's alone from the
/// moment they exist, never opened up and narrowed after, so no other
/// account ever has a moment to read what's written to it. A folder an
/// older eero left open is narrowed, and a file a failed seal left is
/// replaced rather than written through.
fn private(path: &Path) -> io::Result<fs::File> {
    let folder = path.parent().ok_or_else(|| io::Error::other("no folder"))?;
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(folder)?;
    fs::set_permissions(folder, fs::Permissions::from_mode(0o700))?;
    let _ = fs::remove_file(path);
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode(path: &Path) -> u32 {
        fs::metadata(path).expect("it exists").permissions().mode() & 0o777
    }

    #[test]
    fn the_sealed_file_and_its_folder_are_the_accounts_alone_from_the_start() {
        let root = std::env::temp_dir().join(format!("eero-session-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let new = root.join("new").join("session.cred.new");
        private(&new).expect("a file in a new folder");
        assert_eq!(
            (mode(new.parent().expect("its folder")), mode(&new)),
            (0o700, 0o600),
            "a new folder or file is open to other accounts"
        );
        let open = root.join("open");
        fs::create_dir_all(&open).expect("an open folder");
        fs::set_permissions(&open, fs::Permissions::from_mode(0o755)).expect("it opens");
        let stale = open.join("session.cred.new");
        fs::write(&stale, "a failed seal").expect("a stale file");
        fs::set_permissions(&stale, fs::Permissions::from_mode(0o644)).expect("it opens");
        private(&stale).expect("a file over a stale one");
        assert_eq!(
            (
                mode(&open),
                mode(&stale),
                fs::read(&stale).expect("it reads").len()
            ),
            (0o700, 0o600, 0),
            "an open folder wasn't narrowed, or a stale file was written through"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn each_sealer_seals_apart_and_only_a_dead_ones_leftover_is_removed() {
        let root = std::env::temp_dir().join(format!("eero-session-apart-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("the test's folder");
        let file = root.join("session.cred");
        let ours = std::process::id();
        assert_ne!(
            sealing(&file, ours),
            sealing(&file, ours + 1),
            "two sealers share a file"
        );
        // A pid past the kernel's largest is never running.
        let dead = sealing(&file, 4_194_305);
        for leftover in [&dead, &sealing(&file, ours)] {
            fs::write(leftover, "half a seal").expect("a leftover");
        }
        fs::write(root.join("session.cred"), "the session").expect("the session");
        forget_unfinished(&file).expect("the folder reads");
        assert_eq!(
            (dead.exists(), sealing(&file, ours).exists(), file.exists()),
            (false, true, true),
            "a running sealer's file or the session went, or a dead one's stayed"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
