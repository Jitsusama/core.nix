//! Says, once, each firmware update fwupd offers the machine.
//!
//! fwupd refreshes what LVFS offers on its own timer, and the launcher's
//! Machine › Firmware lists it, but only for someone who looks. This
//! program is run as that list may have changed, after a refresh, a device
//! coming or a login, and says each release it hasn't said before as one
//! notice from Firmware: the device by name, and "1.12.0 to 1.13.0, restart
//! to install", or "ready to install" for a device that needs no restart.
//! Nothing here installs anything.
//!
//! What it has said is kept under `$STATE_DIRECTORY` (as systemd sets it)
//! or `$XDG_STATE_HOME/firmware-notice`, one key per device and version, so a
//! release is said once however often it's run, and only releases still
//! offered are kept. A notice that couldn't be sent, the shell being away,
//! isn't kept, so the next run says it.
//!
//! Usage: firmware-notice FWUPDMGR NOTIFY-SEND

use serde::Deserialize;
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// What fwupdmgr exits with when there's nothing to do: here, no update
/// offered to any device, which it says in words rather than JSON.
const NOTHING_OFFERED: i32 = 2;

/// A device fwupd's update finishes only at a restart for, as the BIOS's.
const AT_RESTART: &str = "needs-reboot";

/// The file the keys of what's been said are kept in.
const SAID: &str = "said.json";

/// What `fwupdmgr get-updates --json` says: the devices with an update.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Offered {
    #[serde(default)]
    devices: Vec<Device>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Device {
    #[serde(rename = "DeviceId")]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    flags: Vec<String>,
    #[serde(default)]
    releases: Vec<Release>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Release {
    version: String,
}

/// One release as a notice says it, keyed by its device and version.
#[derive(Debug, PartialEq, Eq)]
struct Notice {
    key: String,
    summary: String,
    body: String,
}

/// The newest release offered each device, as a notice would say it.
fn notices(offered: &Offered) -> Vec<Notice> {
    offered
        .devices
        .iter()
        .filter_map(|device| {
            let release = device.releases.first()?;
            let name = if device.name.is_empty() {
                "Firmware"
            } else {
                &device.name
            };
            let then = if device.flags.iter().any(|flag| flag == AT_RESTART) {
                "restart to install"
            } else {
                "ready to install"
            };
            Some(Notice {
                key: format!("{}@{}", device.id, release.version),
                summary: name.to_owned(),
                body: format!("{} to {}, {then}", device.version, release.version),
            })
        })
        .collect()
}

/// What fwupdmgr said, as the releases offered: nothing where it exits
/// saying there's nothing to do, and an error where it couldn't say, so
/// what's been said is kept rather than forgotten.
fn offered(code: Option<i32>, stdout: &[u8]) -> Result<Offered, String> {
    match code {
        Some(0) => {
            serde_json::from_slice(stdout).map_err(|error| format!("fwupd's updates: {error}"))
        }
        Some(NOTHING_OFFERED) => Ok(Offered::default()),
        Some(code) => Err(format!("fwupdmgr get-updates exited {code}")),
        None => Err("fwupdmgr get-updates was killed".to_owned()),
    }
}

/// What's kept after a run: what was said before and is still offered,
/// and what this run said.
fn kept(notices: &[Notice], before: &BTreeSet<String>, said: &[&Notice]) -> BTreeSet<String> {
    notices
        .iter()
        .filter(|notice| before.contains(&notice.key))
        .chain(said.iter().copied())
        .map(|notice| notice.key.clone())
        .collect()
}

/// Where what's been said is kept.
fn state() -> Result<PathBuf, String> {
    if let Some(dir) = env::var_os("STATE_DIRECTORY") {
        return Ok(PathBuf::from(dir));
    }
    let home = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| Path::new(&home).join(".local/state")))
        .ok_or("neither STATE_DIRECTORY, XDG_STATE_HOME nor HOME is set")?;
    Ok(home.join("firmware-notice"))
}

/// The keys said before, or none where nothing has been said yet.
fn read(file: &Path) -> Result<BTreeSet<String>, String> {
    match fs::read(file) {
        Ok(text) => {
            serde_json::from_slice(&text).map_err(|error| format!("{}: {error}", file.display()))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(BTreeSet::new()),
        Err(error) => Err(format!("{}: {error}", file.display())),
    }
}

/// Keeps the keys whole, by a rename, so a run cut short leaves the last.
fn write(file: &Path, keys: &BTreeSet<String>) -> Result<(), String> {
    let dir = file.parent().ok_or("the state file has no folder")?;
    fs::create_dir_all(dir).map_err(|error| format!("{}: {error}", dir.display()))?;
    let part = file.with_extension("json.part");
    let text = serde_json::to_vec(keys).map_err(|error| error.to_string())?;
    fs::write(&part, text).map_err(|error| format!("{}: {error}", part.display()))?;
    fs::rename(&part, file).map_err(|error| format!("{}: {error}", file.display()))
}

/// Says one notice from Firmware, never urgent, giving whether it went.
fn say(notify_send: &str, notice: &Notice) -> bool {
    Command::new(notify_send)
        .args([
            "--app-name=Firmware",
            "--urgency=normal",
            &notice.summary,
            &notice.body,
        ])
        .status()
        .is_ok_and(|status| status.success())
}

fn run(fwupdmgr: &str, notify_send: &str) -> Result<(), String> {
    let asked = Command::new(fwupdmgr)
        .args(["get-updates", "--json"])
        .output()
        .map_err(|error| format!("{fwupdmgr}: {error}"))?;
    let notices = notices(&offered(asked.status.code(), &asked.stdout)?);
    let file = state()?.join(SAID);
    let before = read(&file)?;
    let said: Vec<&Notice> = notices
        .iter()
        .filter(|notice| !before.contains(&notice.key))
        .filter(|notice| say(notify_send, notice))
        .collect();
    write(&file, &kept(&notices, &before, &said))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let [fwupdmgr, notify_send] = args.as_slice() else {
        eprintln!("usage: firmware-notice FWUPDMGR NOTIFY-SEND");
        return ExitCode::from(2);
    };
    match run(fwupdmgr, notify_send) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("firmware-notice: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offered_from(json: &str) -> Offered {
        offered(Some(0), json.as_bytes()).expect("fwupd's answer parses")
    }

    const BIOS: &str = r#"{"Devices": [
        {"DeviceId": "bios", "Name": "System Firmware", "Version": "1.12.0",
         "Flags": ["internal", "needs-reboot"],
         "Releases": [{"Version": "1.13.0"}, {"Version": "1.12.5"}]},
        {"DeviceId": "dock", "Name": "USB-C Controller", "Version": "69.52",
         "Releases": [{"Version": "69.64"}]},
        {"DeviceId": "ssd", "Name": "PVC10 SK hynix 1024GB", "Version": "61001171",
         "Releases": []}
    ]}"#;

    #[test]
    fn each_device_offered_says_its_newest_release_and_whether_it_restarts() {
        assert_eq!(
            notices(&offered_from(BIOS)),
            vec![
                Notice {
                    key: "bios@1.13.0".to_owned(),
                    summary: "System Firmware".to_owned(),
                    body: "1.12.0 to 1.13.0, restart to install".to_owned(),
                },
                Notice {
                    key: "dock@69.64".to_owned(),
                    summary: "USB-C Controller".to_owned(),
                    body: "69.52 to 69.64, ready to install".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn nothing_offered_is_said_in_words_and_means_no_notice() {
        let none = offered(Some(NOTHING_OFFERED), b"No updatable devices").expect("nothing to do");
        assert!(notices(&none).is_empty());
    }

    #[test]
    fn fwupd_failing_to_answer_is_an_error_rather_than_nothing() {
        assert!(offered(Some(1), b"").is_err());
        assert!(offered(None, b"").is_err());
        assert!(offered(Some(0), b"not json").is_err());
    }

    #[test]
    fn a_release_said_before_is_kept_and_one_no_longer_offered_is_dropped() {
        let notices = notices(&offered_from(BIOS));
        let before: BTreeSet<String> = ["bios@1.13.0", "bios@1.12.0"].map(str::to_owned).into();
        let said = vec![&notices[1]];
        let after: Vec<String> = kept(&notices, &before, &said).into_iter().collect();
        assert_eq!(after, ["bios@1.13.0", "dock@69.64"]);
    }

    #[test]
    fn a_notice_that_could_not_be_sent_is_not_kept() {
        let notices = notices(&offered_from(BIOS));
        let after = kept(&notices, &BTreeSet::new(), &[]);
        assert!(after.is_empty());
    }

    #[test]
    fn what_was_said_is_kept_whole_and_read_back() {
        let dir = env::temp_dir().join(format!("firmware-notice-{}", std::process::id()));
        let file = dir.join(SAID);
        assert!(read(&file).expect("nothing said yet").is_empty());
        let keys: BTreeSet<String> = ["bios@1.13.0".to_owned()].into();
        write(&file, &keys).expect("kept");
        assert_eq!(read(&file).expect("read back"), keys);
        assert!(!file.with_extension("json.part").exists());
        fs::remove_dir_all(dir).expect("cleaned up");
    }
}
