//! Keeps the machine awake while the guide works on it. A laptop whose lid
//! closes, or which idles, partway through writing a stick or copying an
//! install would sleep with the step half done, and a USB stick or a
//! YubiKey may come back from that as another device, or not at all.
//!
//! On Linux the guide asks logind for a lock that blocks sleep, idling and
//! the lid's switch, which lasts as long as the guide holds its file
//! descriptor: until it exits, however it exits. Root holds one without
//! asking, and polkit lets the person at the machine take one too. On a
//! Mac, caffeinate keeps it from sleeping while the guide runs, though a
//! Mac with its lid closed still sleeps.

/// What keeps the machine awake, for as long as it's held.
pub struct Awake {
    #[cfg(target_os = "linux")]
    _lock: Option<zbus::zvariant::OwnedFd>,
    #[cfg(target_os = "macos")]
    caffeinate: Option<std::process::Child>,
}

/// What logind's lock blocks.
#[cfg(target_os = "linux")]
pub const BLOCKS: &str = "sleep:idle:handle-lid-switch";

/// Keeps the machine awake until what it gives back is dropped, saying
/// `why` to whoever lists what holds it so. Should the machine refuse,
/// the guide works on, since a person at it keeps it awake as well.
#[cfg(target_os = "linux")]
pub fn hold(why: &str) -> Awake {
    let lock = (|| -> zbus::Result<zbus::zvariant::OwnedFd> {
        let connection = zbus::blocking::Connection::system()?;
        let reply = connection.call_method(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1",
            Some("org.freedesktop.login1.Manager"),
            "Inhibit",
            &(BLOCKS, "guide", why, "block"),
        )?;
        reply.body().deserialize()
    })();
    Awake { _lock: lock.ok() }
}

#[cfg(target_os = "macos")]
pub fn hold(_why: &str) -> Awake {
    let caffeinate = std::process::Command::new("/usr/bin/caffeinate")
        .args(["-i", "-w", &std::process::id().to_string()])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
    Awake {
        caffeinate: caffeinate.ok(),
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn hold(_why: &str) -> Awake {
    Awake {}
}

#[cfg(target_os = "macos")]
impl Drop for Awake {
    fn drop(&mut self) {
        if let Some(caffeinate) = self.caffeinate.as_mut() {
            let _ = caffeinate.kill();
            let _ = caffeinate.wait();
        }
    }
}
