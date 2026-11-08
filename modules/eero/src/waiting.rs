//! Asking Joel to sign in to eero again, in the launcher's Needs You.
//!
//! eero's session ages out, and then the daemon can only keep the last
//! model, its cloud saying it's signed out. Signing in again takes Joel's
//! phone or email, so the daemon asks him where his other asks wait: a
//! file of its own in the account's `mission/waiting` folder, which the
//! shell's Needs You (WaitingOnYou.qml) lists beside each quest's. It's
//! written once as eero refuses the session and taken away as eero answers
//! again, or as the daemon stops (home.nix), so Needs You never holds an
//! ask nothing keeps current.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::session;
use crate::time;

/// Where the ask waits: named for eero, so no quest's address is it.
pub fn file() -> io::Result<PathBuf> {
    Ok(session::state()?.join("mission/waiting/eero.json"))
}

/// The ask, as Needs You reads it, written at `at`, seconds since 1970.
pub fn sign_in(at: i64) -> Value {
    json!({
        "v": 2,
        "updated": time::from_unix(at),
        "quest": "eero",
        "items": [{
            "id": "eero-sign-in",
            "title": "Sign In to eero",
            "why": "eero's session aged out, so the Home Network place holds eero's last word",
            "minutes": 1,
            "ready": true,
            // The daemon takes the ask away as eero answers, so it's done
            // once the record's cloud no longer says it's signed out.
            "doneWhen": {
                "check": "! grep -q '\"signed-out\"' \"$XDG_RUNTIME_DIR/eero/home-network.json\""
            },
            "steps": [
                "Type eero sign-in and your email or phone in a terminal",
                "Type the code eero sends you"
            ]
        }]
    })
}

/// Asks Joel to sign in, or stops asking, changing `file` only when that
/// changes, so the shell hears it once.
pub fn ask(file: &Path, asks: bool, at: i64) -> io::Result<()> {
    if asks == file.exists() {
        return Ok(());
    }
    if asks {
        whole(file, &sign_in(at).to_string())
    } else {
        fs::remove_file(file)
    }
}

/// Writes `text` in a `.writing` folder beside `file` and then renames it
/// over `file`, so no reader sees half, and a shell watching the folder
/// hears only the rename.
fn whole(file: &Path, text: &str) -> io::Result<()> {
    let aside = file
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".writing");
    fs::create_dir_all(&aside)?;
    let writing = aside.join(file.file_name().unwrap_or_default());
    fs::write(&writing, text)?;
    fs::rename(&writing, file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ask_says_what_joel_does_and_when_it_was_written() {
        let asked = sign_in(1_794_092_400);

        assert_eq!(asked["updated"], "2026-11-07T23:00:00Z");
        assert_eq!(asked["items"][0]["title"], "Sign In to eero");
        assert_eq!(
            asked["items"][0]["steps"][0],
            "Type eero sign-in and your email or phone in a terminal"
        );
    }

    #[test]
    fn the_ask_is_written_once_and_taken_away_as_eero_answers() {
        let folder = std::env::temp_dir().join(format!("eero-waiting-{}", std::process::id()));
        let file = folder.join("eero.json");
        fs::create_dir_all(&folder).unwrap();

        ask(&file, true, 1).unwrap();
        let first = fs::read_to_string(&file).unwrap();
        ask(&file, true, 2).unwrap();

        assert_eq!(fs::read_to_string(&file).unwrap(), first, "asked again");
        ask(&file, false, 3).unwrap();
        assert!(!file.exists());
        ask(&file, false, 4).unwrap();
        fs::remove_dir_all(&folder).unwrap();
    }
}
