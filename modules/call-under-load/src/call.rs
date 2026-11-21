//! One test call, shaped as Meet's, between two Chromes on this machine
//! while it carries whatever load it carries: the near end is a Chrome
//! with a profile like Joel's, drawing on a headless Weston, never his
//! screen, and the far end a headless Chrome. Both use Chrome's fake
//! camera; no real one is ever opened. A call of Joel's stops it within
//! five seconds.

use crate::cdp::Browser;
use crate::gate::{self, Call};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// How the near end sends, as Meet sends a camera: 1080p at 30 fps from
/// the camera, scaled to 720p and sent as VP9 in three spatial layers at
/// up to 2.5 Mb/s.
const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const FPS: u32 = 30;
const SCALE_DOWN: f64 = 1.5;
const MAX_BITRATE: u32 = 2_500_000;

/// What the hour's figures say the call was.
pub const STAND_IN: &str = "Chrome's fake camera at 1080p30, sent as Meet sends, VP9 \
                            720p in three layers at up to 2.5 Mb/s, to a second Chrome here";

/// How often the call is checked for one of Joel's, and his click stood in for.
const TICK: Duration = Duration::from_secs(5);

/// How long Chrome is given to answer on its pipe: a loaded machine has
/// taken 20 s to start one.
const WAIT: Duration = Duration::from_secs(60);

/// The folders Chrome maps writable and keeps writing, kept in memory and
/// linked from the profile on disk, as Joel's own Chrome has them.
const MAPPED: [&str; 4] = [
    "BrowserMetrics",
    "Default/GPUCache",
    "Default/DawnWebGPUCache",
    "Default/DawnGraphiteCache",
];

/// How a call ended.
pub enum Ended {
    /// It ran its whole length: each end's samples and the clicks' times
    /// to their next frame, in ms.
    Ran {
        mine: Vec<Value>,
        far: Vec<Value>,
        clicks: Vec<f64>,
    },
    /// Joel's call began, or whether one had couldn't be told, so it
    /// stopped and counts for nothing.
    Stopped(String),
}

/// The display and folders a call starts with, ended and removed as it
/// drops, however the call ends. Each Chrome ends as its `Browser` drops,
/// which is first, since each is made after this.
struct Started {
    folders: Vec<PathBuf>,
    children: Vec<Child>,
}

impl Drop for Started {
    fn drop(&mut self) {
        for child in &mut self.children {
            let _ = child.kill();
            let _ = child.wait();
        }
        for folder in &self.folders {
            let _ = fs::remove_dir_all(folder);
        }
    }
}

/// What the near end sends, as `page.html`'s offer takes it.
fn sends() -> Value {
    json!({
        "width": WIDTH,
        "height": HEIGHT,
        "fps": FPS,
        "maxBitrate": MAX_BITRATE,
        "scaleDown": SCALE_DOWN,
    })
}

/// Runs a call of `seconds` with `chrome`, on the page at `page`; `runtime`
/// is Joel's runtime folder and `cache` the folder the profile goes in.
pub fn run(
    chrome: &Path,
    page: &Path,
    seconds: u64,
    runtime: &Path,
    cache: &Path,
) -> Result<Ended, String> {
    let id = std::process::id();
    // A Wayland socket's path must fit in 108 bytes, so it's kept short.
    let memory = runtime.join(format!("call-under-load-{id}"));
    let profile = cache.join(format!("profile-{id}"));
    let mut started = Started {
        folders: vec![memory.clone(), profile.clone()],
        children: Vec::new(),
    };
    let config = memory.join("config");
    let far_profile = memory.join("far");
    fs::create_dir_all(config.join("pkcs11/modules")).map_err(|e| e.to_string())?;
    fs::write(
        config.join("pkcs11/modules/tpm2_pkcs11.module"),
        "enable-in: none-of-these\n",
    )
    .map_err(|e| e.to_string())?;
    for folder in MAPPED {
        let (kept, linked) = (memory.join("maps").join(folder), profile.join(folder));
        fs::create_dir_all(&kept).map_err(|e| e.to_string())?;
        fs::create_dir_all(linked.parent().unwrap_or(&profile)).map_err(|e| e.to_string())?;
        std::os::unix::fs::symlink(&kept, &linked).map_err(|e| e.to_string())?;
    }

    started.children.push(
        Command::new("weston")
            .args([
                "--backend=headless",
                "--renderer=gl",
                "--socket=wl",
                "--no-config",
            ])
            .args([format!("--width={WIDTH}"), format!("--height={HEIGHT}")])
            .env("XDG_RUNTIME_DIR", &memory)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("weston couldn't start: {e}"))?,
    );
    let socket = memory.join("wl");
    let until = Instant::now() + WAIT;
    while !socket.exists() {
        if Instant::now() > until {
            return Err("weston opened no display".into());
        }
        // Weston makes its socket a moment after it starts; nothing says when.
        thread::sleep(Duration::from_millis(100));
    }

    let mut near = chrome_command(chrome, &profile, &memory, &config);
    near.arg("--ozone-platform=wayland")
        .env("WAYLAND_DISPLAY", "wl");
    let mut near = Browser::start(near).map_err(|e| format!("Chrome couldn't start: {e}"))?;
    let mut far = chrome_command(chrome, &far_profile, &memory, &config);
    far.arg("--headless=new");
    let mut far = Browser::start(far).map_err(|e| format!("the far end's Chrome: {e}"))?;

    let url = format!("file://{}", page.display());
    for browser in [&mut near, &mut far] {
        browser.attach(WAIT)?;
        browser.call("Page.navigate", json!({"url": url}), WAIT)?;
        loaded(browser)?;
    }
    let offered = near.eval(&format!("offer({})", sends()), WAIT)?;
    let label = offered["label"].as_str().unwrap_or("");
    // Never a real camera: one whose label isn't Chrome's fake stops it here.
    if !label.contains("fake") {
        return Err(format!(
            "the camera opened was {label:?}, not Chrome's fake one"
        ));
    }
    let answer = far.eval(
        &format!("answer({}, {MAX_BITRATE})", json!(offered["offer"])),
        WAIT,
    )?;
    near.eval(&format!("accept({})", json!(answer)), WAIT)?;

    let ends = Instant::now() + Duration::from_secs(seconds);
    let mut next = Instant::now();
    while Instant::now() < ends {
        match gate::a_call() {
            Call::Off => {}
            Call::On => return Ok(Ended::Stopped("Joel's call began".into())),
            Call::Unknown(why) => return Ok(Ended::Stopped(why)),
        }
        near.click(40.0, 40.0, WAIT)?;
        next += TICK;
        // Each check and click comes on the call's own 5 s beat.
        thread::sleep(next.saturating_duration_since(Instant::now()));
    }
    let mine = list(near.eval("window.samples", WAIT)?);
    let far_samples = list(far.eval("window.samples", WAIT)?);
    let clicks = list(near.eval("window.taps.map((t) => t.frame)", WAIT)?)
        .iter()
        .filter_map(Value::as_f64)
        .collect();
    Ok(Ended::Ran {
        mine,
        far: far_samples,
        clicks,
    })
}

/// A Chrome with `profile`, Chrome's fake camera, and the TPM kept out of
/// reach: agents never touch it, and Chrome's NSS loads every PKCS#11
/// module p11-kit knows, the TPM's among them. A user config turning it
/// off isn't honoured on its own, so FAPI is given no config and the
/// module's TPM link a closed local port, and neither opens the device.
fn chrome_command(chrome: &Path, profile: &Path, memory: &Path, config: &Path) -> Command {
    let mut command = Command::new(chrome);
    command
        .arg(format!("--user-data-dir={}", profile.display()))
        .args([
            "--no-first-run",
            "--no-default-browser-check",
            "--use-fake-device-for-media-stream=fps=30",
            "--use-fake-ui-for-media-stream",
            "--autoplay-policy=no-user-gesture-required",
        ])
        .env("XDG_RUNTIME_DIR", memory)
        .env("XDG_CONFIG_HOME", config)
        .env("TSS2_FAPICONF", "/nonexistent/fapi-config.json")
        .env("TPM2_PKCS11_TCTI", "mssim:host=127.0.0.1,port=9")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

/// Waits for the page's script to have run.
fn loaded(browser: &mut Browser) -> Result<(), String> {
    let until = Instant::now() + WAIT;
    loop {
        if browser.eval("typeof window.offer", WAIT)? == "function" {
            return Ok(());
        }
        if Instant::now() > until {
            return Err("the call's page never loaded".into());
        }
        // A navigation says nothing when its page's script has run.
        thread::sleep(Duration::from_millis(200));
    }
}

fn list(value: Value) -> Vec<Value> {
    match value {
        Value::Array(items) => items,
        _ => Vec::new(),
    }
}
