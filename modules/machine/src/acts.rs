//! What Firmware asks the machine to do, done here so the shell spawns
//! nothing: `firmware-install` puts in the update fwupd offers a device,
//! through fwupdmgr, which fetches and checks the update as fwupd's own
//! client does; `firmware-fix` and `firmware-undo` have fwupd fix one of
//! the host's security attributes, or put the fix back, over the system
//! bus; and `firmware-setup` has logind restart into the firmware's own
//! setup, as `systemctl reboot --firmware-setup` does. Each is waited for
//! as long as the shell says, so a polkit prompt left unanswered or a
//! stuck fwupdmgr stops being waited for, and each says why it didn't go
//! in its service's own words.

use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use zbus::proxy::MethodFlags;

/// fwupdmgr, by its path, as the shell's home configuration hands it to
/// the daemon; a machine without fwupd has none, and installs nothing.
pub const FWUPDMGR: &str = "MACHINE_FWUPDMGR";

/// Why an act didn't go: what was asked is no act this machine takes
/// (`bad-act`), it was refused (`refused`), with the service's words for
/// why, it was waited for past the shell's patience (`stopped`), or
/// another firmware act is still going (`busy`).
#[derive(Debug, PartialEq, Eq)]
pub struct Refused {
    pub code: &'static str,
    pub message: String,
}

pub fn refused(code: &'static str, message: impl Into<String>) -> Refused {
    Refused {
        code,
        message: message.into(),
    }
}

/// Puts in the update offered to this device alone with fwupdmgr, saying
/// yes to what it asks and nothing about a restart, which stays Joel's,
/// waiting at most `patience`. fwupdmgr stopped stops only the waiting:
/// fwupd carries on with what it was asked, and says what changed.
pub fn install(program: &str, device: &str, patience: Duration) -> Result<(), Refused> {
    if program.is_empty() {
        return Err(refused("bad-act", "no fwupdmgr on this machine"));
    }
    if device.is_empty() || device.starts_with('-') {
        return Err(refused("bad-act", format!("no device {device:?}")));
    }
    let mut child = Command::new(program)
        .args(["update", device, "--assume-yes", "--no-reboot-check"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|unrun| refused("refused", unrun.to_string()))?;
    let mut stderr = child.stderr.take();
    let (said, heard) = mpsc::channel();
    // Its stderr closes as it ends, so what it said is the sign it's done.
    std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(stderr) = stderr.as_mut() {
            let _ = stderr.read_to_string(&mut text);
        }
        let _ = said.send(text);
    });
    let Ok(text) = heard.recv_timeout(patience) else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(refused("stopped", ""));
    };
    match child.wait() {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => Err(refused("refused", text)),
        Err(unwaited) => Err(refused("refused", unwaited.to_string())),
    }
}

/// Has fwupd fix the attribute fwupd names by this id, or put its fix
/// back, as fwupdmgr's security-fix and security-undo do, letting polkit
/// ask Joel where its rules say to.
pub fn change(attribute: &str, undo: bool, patience: Duration) -> Result<(), Refused> {
    if attribute.is_empty() {
        return Err(refused("bad-act", "no attribute named"));
    }
    let method = if undo {
        "UndoHostSecurityAttr"
    } else {
        "FixHostSecurityAttr"
    };
    asked(
        patience,
        "org.freedesktop.fwupd",
        "/",
        "org.freedesktop.fwupd",
        &[(method, Some(attribute))],
    )
}

/// Has logind restart into the firmware's setup: first saying the
/// firmware should open it, which logind refuses where it can't be asked
/// to, then the restart.
pub fn setup(patience: Duration) -> Result<(), Refused> {
    asked(
        patience,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
        &[("SetRebootToFirmwareSetup", None), ("Reboot", None)],
    )
}

/// Calls each method in turn on a connection of its own that waits at
/// most `patience`, each given its string, or `true` where it has none,
/// which is what logind's two take; the first refusal ends it.
fn asked(
    patience: Duration,
    service: &str,
    path: &str,
    interface: &str,
    calls: &[(&str, Option<&str>)],
) -> Result<(), Refused> {
    let unreached = |error: zbus::Error| said(&error);
    let bus = zbus::blocking::connection::Builder::system()
        .and_then(|builder| builder.method_timeout(patience).build())
        .map_err(unreached)?;
    let proxy = zbus::blocking::Proxy::new(&bus, service, path, interface).map_err(unreached)?;
    for (method, given) in calls {
        let flags = MethodFlags::AllowInteractiveAuth.into();
        let answer = match given {
            Some(text) => proxy.call_with_flags::<_, _, ()>(*method, flags, &(*text,)),
            None => proxy.call_with_flags::<_, _, ()>(*method, flags, &(true,)),
        };
        answer.map_err(|error| said(&error))?;
    }
    Ok(())
}

/// A bus call's failure as an act says it: one past its wait is
/// `stopped`, and any other `refused`, in the service's own words where
/// it gave any.
fn said(error: &zbus::Error) -> Refused {
    match error {
        zbus::Error::InputOutput(io) if io.kind() == std::io::ErrorKind::TimedOut => {
            refused("stopped", "")
        }
        zbus::Error::MethodError(_, Some(words), _) => refused("refused", words.clone()),
        other => refused("refused", other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    /// Held by each test that writes a program and runs it: a program run
    /// while another thread's fork still holds it open to be written fails
    /// as a busy file, so one test at a time writes and runs.
    static RUNNING: Mutex<()> = Mutex::new(());

    fn alone() -> MutexGuard<'static, ()> {
        RUNNING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The shell the test's programs run in, found on PATH as the build's
    /// own is, since a sandbox has no /bin/sh.
    fn shell() -> String {
        let path = std::env::var_os("PATH").expect("the test has a PATH");
        std::env::split_paths(&path)
            .map(|folder| folder.join("sh"))
            .find(|sh| sh.exists())
            .expect("a shell is on PATH")
            .to_string_lossy()
            .into_owned()
    }

    /// A program written for the test, as fwupdmgr would be found.
    fn program(name: &str, body: &str) -> String {
        use std::os::unix::fs::PermissionsExt;
        let folder = std::env::temp_dir().join(format!("machine-acts-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("the test's folder is made");
        let path = folder.join(name);
        std::fs::write(&path, format!("#!{}\n{body}\n", shell())).expect("the program is written");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .expect("the program can run");
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn an_install_that_goes_in_is_done_and_says_what_was_asked() {
        let _alone = alone();
        let said = std::env::temp_dir().join(format!("machine-acts-said-{}", std::process::id()));
        let fwupdmgr = program("goes-in", &format!("echo \"$*\" > {}", said.display()));
        assert_eq!(install(&fwupdmgr, "bios", Duration::from_secs(10)), Ok(()));
        let asked = std::fs::read_to_string(&said).expect("fwupdmgr was run");
        assert_eq!(asked.trim(), "update bios --assume-yes --no-reboot-check");
    }

    #[test]
    fn an_install_refused_says_why_in_fwupdmgrs_words() {
        let _alone = alone();
        let fwupdmgr = program("refuses", "echo 'Device is locked' >&2; exit 1");
        assert_eq!(
            install(&fwupdmgr, "bios", Duration::from_secs(10)),
            Err(refused("refused", "Device is locked\n"))
        );
    }

    #[test]
    fn an_install_waited_for_past_its_patience_is_stopped() {
        let _alone = alone();
        let fwupdmgr = program("hangs", "exec sleep 30");
        assert_eq!(
            install(&fwupdmgr, "bios", Duration::from_millis(200)),
            Err(refused("stopped", ""))
        );
    }

    #[test]
    fn nothing_is_run_for_a_device_that_could_be_an_option_or_with_no_fwupdmgr() {
        let _alone = alone();
        let fwupdmgr = program("never", "exit 0");
        let patience = Duration::from_secs(1);
        assert_eq!(
            install(&fwupdmgr, "--force", patience).map_err(|r| r.code),
            Err("bad-act")
        );
        assert_eq!(
            install(&fwupdmgr, "", patience).map_err(|r| r.code),
            Err("bad-act")
        );
        assert_eq!(
            install("", "bios", patience).map_err(|r| r.code),
            Err("bad-act")
        );
    }
}
