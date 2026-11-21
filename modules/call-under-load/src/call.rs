//! One test call, shaped as Meet's, between two Chromes on this machine
//! while it carries whatever load it carries: the near end is a Chrome
//! with a profile like Joel's, drawing on a headless Weston, never his
//! screen, and the far end a headless Chrome. Both use Chrome's fake
//! camera; no real one is ever opened. A call of Joel's stops it within
//! fifteen seconds, from its first step to its last: the watch asks every
//! five, an ask has five to answer, and a step still going five more on
//! has its Weston and Chromes ended under it.

use crate::cdp::{self, Browser};
use crate::gate::{self, Call};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
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

/// How long a step that can't be broken off, such as a Chrome slow to
/// answer, is given to end once Joel's call began, before the watch ends
/// the call's Weston and Chromes, which ends the step.
const GRACE: Duration = Duration::from_secs(5);

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

/// Joel's calls, watched for from the test call's first step, since
/// starting Weston and two Chromes under load can take minutes: a thread
/// of its own asks `a-call-is-on` every five seconds, and the call takes
/// what it says between its steps and on its beat. If the call is still
/// in a step `GRACE` after a call of Joel's began, the watch ends each
/// process the call started, so the step fails and the call returns,
/// cleaning up as it goes.
struct Watch {
    said: Receiver<String>,
    over: Arc<AtomicBool>,
    started: Arc<Mutex<Vec<u32>>>,
}

impl Watch {
    fn start() -> Self {
        Watch::asking(gate::a_call, TICK)
    }

    /// A watch that asks `ask` every `tick`.
    fn asking(ask: fn() -> Call, tick: Duration) -> Self {
        let (tell, said) = mpsc::channel();
        let over = Arc::new(AtomicBool::new(false));
        let done = Arc::clone(&over);
        let started = Arc::new(Mutex::new(Vec::new()));
        let ending = Arc::clone(&started);
        thread::spawn(move || {
            let why = loop {
                match ask() {
                    Call::Off => {}
                    Call::On => break "Joel's call began".to_string(),
                    Call::Unknown(why) => break why,
                }
                // Asked on the call's own five-second beat.
                thread::sleep(tick);
                if done.load(Ordering::SeqCst) {
                    return;
                }
            };
            let _ = tell.send(why.clone());
            // The call is given a moment to stop by itself and clean up.
            thread::sleep(GRACE);
            if !done.load(Ordering::SeqCst) {
                let pids = ending.lock().map(|pids| pids.clone()).unwrap_or_default();
                for pid in pids {
                    cdp::end(pid);
                }
            }
        });
        Watch {
            said,
            over,
            started,
        }
    }

    /// Has the process `pid` ended if a step outlasts its grace.
    fn ends(&self, pid: u32) {
        if let Ok(mut pids) = self.started.lock() {
            pids.push(pid);
        }
    }

    /// Why the call must stop, if it must.
    fn stop(&self) -> Result<(), Ended> {
        self.said
            .try_recv()
            .map_or(Ok(()), |why| Err(Ended::Stopped(why)))
    }

    /// Waits until `then`, unless the call must stop first.
    fn wait_until(&self, then: Instant) -> Result<(), Ended> {
        let wait = then.saturating_duration_since(Instant::now());
        self.said
            .recv_timeout(wait)
            .map_or(Ok(()), |why| Err(Ended::Stopped(why)))
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        self.over.store(true, Ordering::SeqCst);
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
    let watch = Watch::start();
    sweep(runtime, "call-under-load-");
    sweep(cache, "profile-");
    settled(steps(chrome, page, seconds, runtime, cache, &watch), &watch)
}

/// How the call's steps ended, as the run says it.
fn settled(steps: Result<Ended, Step>, watch: &Watch) -> Result<Ended, String> {
    steps.or_else(|step| match step {
        Step::Stopped(ended) => Ok(ended),
        // A step the watch broke off by ending its processes fails, but
        // the call stopped for Joel's, and counts for nothing.
        Step::Failed(why) => match watch.stop() {
            Err(ended) => Ok(ended),
            Ok(()) => Err(why),
        },
    })
}

/// Why a call's steps ended early: Joel's call, or a failure.
enum Step {
    Stopped(Ended),
    Failed(String),
}

impl From<String> for Step {
    fn from(why: String) -> Self {
        Step::Failed(why)
    }
}

impl From<Ended> for Step {
    fn from(ended: Ended) -> Self {
        Step::Stopped(ended)
    }
}

/// Removes what an earlier run ended without its cleanup left in
/// `folder`: each `<prefix><pid>` whose process is gone.
fn sweep(folder: &Path, prefix: &str) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name.to_str().and_then(|n| n.strip_prefix(prefix)) else {
            continue;
        };
        if pid.parse::<u32>().is_ok() && !Path::new("/proc").join(pid).exists() {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}

/// The call's steps in their order, each followed by its check for Joel's call.
fn steps(
    chrome: &Path,
    page: &Path,
    seconds: u64,
    runtime: &Path,
    cache: &Path,
    watch: &Watch,
) -> Result<Ended, Step> {
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
    fs::create_dir_all(memory.join("home")).map_err(|e| e.to_string())?;
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

    let mut weston = Command::new("weston");
    weston
        .args([
            "--backend=headless",
            "--renderer=gl",
            "--socket=wl",
            "--no-config",
        ])
        .args([format!("--width={WIDTH}"), format!("--height={HEIGHT}")])
        .env("XDG_RUNTIME_DIR", &memory)
        .stdout(Stdio::null());
    // Its complaints go to the unit's journal, where a failed hour is read.
    cdp::ends_with_us(&mut weston);
    let weston = weston
        .spawn()
        .map_err(|e| format!("weston couldn't start: {e}"))?;
    watch.ends(weston.id());
    started.children.push(weston);
    let socket = memory.join("wl");
    let until = Instant::now() + WAIT;
    while !socket.exists() {
        if let Some(Ok(Some(status))) = started.children.last_mut().map(Child::try_wait) {
            return Err(format!("weston ended before opening a display: {status}").into());
        }
        if Instant::now() > until {
            return Err("weston opened no display".to_string().into());
        }
        watch.stop()?;
        // Weston makes its socket a moment after it starts; nothing says when.
        thread::sleep(Duration::from_millis(100));
    }

    let mut near = chrome_command(chrome, &profile, &memory, &config);
    near.arg("--ozone-platform=wayland")
        .env("WAYLAND_DISPLAY", "wl");
    let mut near = Browser::start(near).map_err(|e| format!("Chrome couldn't start: {e}"))?;
    watch.ends(near.pid());
    watch.stop()?;
    let mut far = chrome_command(chrome, &far_profile, &memory, &config);
    far.arg("--headless=new");
    let mut far = Browser::start(far).map_err(|e| format!("the far end's Chrome: {e}"))?;
    watch.ends(far.pid());
    watch.stop()?;

    let url = format!("file://{}", page.display());
    for browser in [&mut near, &mut far] {
        browser.attach(WAIT)?;
        watch.stop()?;
        browser.call("Page.navigate", json!({"url": url}), WAIT)?;
        loaded(browser, watch)?;
    }
    let offered = near.eval(&format!("offer({})", sends()), WAIT)?;
    fake(&offered, "near")?;
    watch.stop()?;
    let answered = far.eval(
        &format!("answer({}, {MAX_BITRATE})", json!(offered["offer"])),
        WAIT,
    )?;
    fake(&answered, "far")?;
    watch.stop()?;
    near.eval(&format!("accept({})", json!(answered["answer"])), WAIT)?;

    let ends = Instant::now() + Duration::from_secs(seconds);
    let mut next = Instant::now();
    while Instant::now() < ends {
        near.click(40.0, 40.0, WAIT)?;
        next += TICK;
        // Each click comes on the call's own 5 s beat, and Joel's call
        // stops it the moment the watch hears of one.
        watch.wait_until(next)?;
    }
    let mine = list(near.eval("window.samples", WAIT)?);
    connected(&mine)?;
    let far_samples = list(far.eval("window.samples", WAIT)?);
    let clicks = list(near.eval("window.taps.map((t) => t.frame)", WAIT)?)
        .iter()
        .filter_map(Value::as_f64)
        .collect();
    // A call of Joel's heard while the samples were read still counts.
    watch.stop()?;
    Ok(Ended::Ran {
        mine,
        far: far_samples,
        clicks,
    })
}

/// Refuses a call that never connected: it measured nothing, and says so
/// as a failure rather than as an hour of empty figures.
fn connected(samples: &[Value]) -> Result<(), Step> {
    if samples.iter().any(|sample| sample["state"] == "connected") {
        return Ok(());
    }
    Err("the call never connected".to_string().into())
}

/// Refuses a camera that isn't Chrome's fake one, as `end`'s page named it.
/// It's an alarm, not the gate: the label is read once the camera's open,
/// and what keeps a real one shut is `--use-fake-device-for-media-stream`,
/// with which Chrome lists only its fake devices.
fn fake(said: &Value, end: &str) -> Result<(), Step> {
    let label = said["label"].as_str().unwrap_or("");
    if label.contains("fake") {
        return Ok(());
    }
    Err(format!("the {end} end's camera was {label:?}, not Chrome's fake one").into())
}

/// A Chrome with `profile`, Chrome's fake camera, and the TPM kept out of
/// reach: agents never touch it, and Chrome's NSS loads every PKCS#11
/// module p11-kit knows, the TPM's among them. A user config turning it
/// off isn't honoured on its own, so FAPI is given no config and the
/// module's TPM link a closed local port, and neither opens the device.
/// It's kept off Joel's keyring and his NSS database too, with a home of
/// its own in the call's folder, and fetches nothing in the background.
/// Its runtime folder is the call's own, which also keeps it off the
/// sockets in Joel's, such as the one his open-tabs host answers on.
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
            "--password-store=basic",
            "--disable-background-networking",
            "--disable-component-update",
        ])
        .env("HOME", memory.join("home"))
        .env("XDG_RUNTIME_DIR", memory)
        .env("XDG_CONFIG_HOME", config)
        .env("TSS2_FAPICONF", "/nonexistent/fapi-config.json")
        .env("TPM2_PKCS11_TCTI", "mssim:host=127.0.0.1,port=9")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

/// Waits for the page's script to have run.
fn loaded(browser: &mut Browser, watch: &Watch) -> Result<(), Step> {
    let until = Instant::now() + WAIT;
    loop {
        // A page still navigating can refuse the ask; it's asked again.
        if browser
            .eval("typeof window.offer", WAIT)
            .is_ok_and(|kind| kind == "function")
        {
            return Ok(());
        }
        if Instant::now() > until {
            return Err("the call's page never loaded".to_string().into());
        }
        watch.stop()?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_test_chrome_takes_the_fake_camera_and_keeps_off_joel_s_secrets() {
        let memory = Path::new("/run/x/call-under-load-1");
        let command = chrome_command(
            Path::new("chrome"),
            Path::new("/p"),
            memory,
            &memory.join("config"),
        );
        let args: Vec<_> = command.get_args().filter_map(|a| a.to_str()).collect();
        for flag in [
            "--use-fake-device-for-media-stream=fps=30",
            "--use-fake-ui-for-media-stream",
            "--password-store=basic",
            "--disable-background-networking",
        ] {
            assert!(args.contains(&flag), "{flag} is missing from {args:?}");
        }
        let env: Vec<_> = command
            .get_envs()
            .filter_map(|(k, v)| Some((k.to_str()?, v?.to_str()?)))
            .collect();
        for (key, value) in [
            ("HOME", "/run/x/call-under-load-1/home"),
            ("XDG_CONFIG_HOME", "/run/x/call-under-load-1/config"),
            ("TSS2_FAPICONF", "/nonexistent/fapi-config.json"),
            ("TPM2_PKCS11_TCTI", "mssim:host=127.0.0.1,port=9"),
        ] {
            assert!(
                env.contains(&(key, value)),
                "{key}={value} is missing from {env:?}"
            );
        }
    }

    fn on() -> Call {
        Call::On
    }

    fn off() -> Call {
        Call::Off
    }

    #[test]
    fn a_call_of_joel_s_stops_a_step_and_the_call_s_beat() {
        let watch = Watch::asking(on, Duration::from_millis(10));
        let started = Instant::now();
        let beat = watch.wait_until(started + Duration::from_secs(3));
        assert!(
            matches!(beat, Err(Ended::Stopped(ref why)) if why == "Joel's call began"),
            "the call's beat ran on through Joel's call"
        );
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "the beat waited {:?} after Joel's call began",
            started.elapsed()
        );
        // Dropped well within its grace, so the watch never ends the tests.
        drop(watch);
        let watch = Watch::asking(on, Duration::from_millis(10));
        thread::sleep(Duration::from_millis(100));
        assert!(watch.stop().is_err(), "a step went on through Joel's call");
    }

    #[test]
    fn with_no_call_on_the_beat_keeps_its_time() {
        let watch = Watch::asking(off, Duration::from_millis(10));
        let started = Instant::now();
        assert!(
            watch
                .wait_until(started + Duration::from_millis(200))
                .is_ok(),
            "the beat stopped with no call on"
        );
        assert!(
            started.elapsed() >= Duration::from_millis(200),
            "the beat came early, after {:?}",
            started.elapsed()
        );
        assert!(watch.stop().is_ok(), "a step stopped with no call on");
    }

    #[test]
    fn a_step_outlasting_its_grace_has_what_the_call_started_ended() {
        let mut stuck = Command::new("sleep")
            .arg("60")
            .spawn()
            .expect("sleep starts");
        let watch = Watch::asking(on, Duration::from_millis(10));
        watch.ends(stuck.id());
        let started = Instant::now();
        let status = stuck.wait().expect("sleep is waited on");
        assert!(
            !status.success() && started.elapsed() < GRACE + Duration::from_secs(2),
            "the stuck process ended {status} after {:?}",
            started.elapsed()
        );
        assert!(watch.stop().is_err(), "the call didn't hear why it stopped");
    }

    #[test]
    fn a_step_broken_off_for_joel_s_call_counts_as_stopped_and_any_other_as_failed() {
        let broken = || Err(Step::Failed("Chrome's DevTools pipe closed".into()));
        let watch = Watch::asking(on, Duration::from_millis(10));
        thread::sleep(Duration::from_millis(100));
        assert!(
            matches!(settled(broken(), &watch), Ok(Ended::Stopped(ref why)) if why == "Joel's call began"),
            "a step the watch broke off read as a failed hour"
        );
        drop(watch);
        let watch = Watch::asking(off, Duration::from_millis(10));
        assert!(
            matches!(settled(broken(), &watch), Err(ref why) if why.contains("pipe closed")),
            "a step that failed by itself read as stopped"
        );
    }

    #[test]
    fn only_a_child_of_the_call_s_is_ever_ended() {
        // This program isn't its own child, so it's left be.
        cdp::end(std::process::id());
        let mut child = Command::new("sleep")
            .arg("60")
            .spawn()
            .expect("sleep starts");
        cdp::end(child.id());
        let status = child.wait().expect("sleep is waited on");
        assert!(!status.success(), "a child of ours wasn't ended: {status}");
    }

    #[test]
    fn a_call_that_never_connected_fails() {
        let new = [json!({"state": "new"}), json!({"state": "connecting"})];
        assert!(
            matches!(connected(&new), Err(Step::Failed(_))),
            "a call that never connected passed"
        );
        assert!(
            connected(&[json!({"state": "connected"})]).is_ok(),
            "a connected call failed"
        );
    }

    #[test]
    fn only_chrome_s_fake_camera_is_taken_at_either_end() {
        assert!(
            fake(&json!({"label": "fake_device_0"}), "near").is_ok(),
            "Chrome's fake camera was refused"
        );
        let refused = |said: Value| matches!(fake(&said, "far"), Err(Step::Failed(_)));
        assert!(
            refused(json!({"label": "MX Brio 705 for Business"})),
            "a real camera was taken"
        );
        assert!(refused(json!({})), "a camera with no name is refused");
    }

    #[test]
    fn a_sweep_takes_only_what_a_gone_run_left() {
        let folder = std::env::temp_dir().join(format!("cul-sweep-{}", std::process::id()));
        let mine = folder.join(format!("profile-{}", std::process::id()));
        let gone = folder.join("profile-4294967294");
        let other = folder.join("profile-notes");
        for path in [&mine, &gone, &other] {
            fs::create_dir_all(path).unwrap();
        }
        sweep(&folder, "profile-");
        assert!(mine.exists(), "a running call's folder stays");
        assert!(!gone.exists(), "a gone run's folder goes");
        assert!(other.exists(), "a folder not named for a run stays");
        fs::remove_dir_all(&folder).unwrap();
    }
}
