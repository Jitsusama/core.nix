//! One test call, shaped as Meet's, between two Chromiums on this machine
//! while it carries whatever load it carries: the near end is a Chromium
//! with a profile like Joel's, drawing on a headless Weston, never his
//! screen, and the far end a headless Chromium. Both use Chromium's fake
//! camera; no real one is ever opened. Chromium, not Google Chrome, for
//! the reason `managed` gives; the Chromes named below are these. A call of Joel's stops it within
//! fifteen seconds, from its first step to its last: the watch asks every
//! five, an ask has five to answer, and a step still going five more on
//! has its Weston and Chromes ended under it.

use crate::cdp::{self, Browser};
use crate::gate::{self, Call};
use crate::managed;
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
pub const STAND_IN: &str = "Chromium's fake camera at 1080p30, sent as Meet sends, VP9 \
                            720p in three layers at up to 2.5 Mb/s, to a second Chromium \
                            here";

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
/// which is first, since each is made after this. Each of its `profiles`
/// is looked in for an enrolment before its folder goes, since every way
/// a call ends comes through here, not only the beat that looks for one.
struct Started {
    folders: Vec<PathBuf>,
    children: Vec<Child>,
    profiles: Vec<PathBuf>,
    state: PathBuf,
}

impl Drop for Started {
    fn drop(&mut self) {
        for child in &mut self.children {
            let _ = child.kill();
            let _ = child.wait();
        }
        let profiles: Vec<&Path> = self.profiles.iter().map(PathBuf::as_path).collect();
        if let Err(Enrolled { kept: false, .. }) = unenrolled(&profiles, &self.state) {
            // An enrolment that couldn't be kept stays where it is, for
            // `run` and the next run's sweep to find, rather than going.
            return;
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
    /// What the watch said, kept, so every later ask hears it too.
    heard: Mutex<Option<String>>,
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
            heard: Mutex::new(None),
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
        self.hearing(|said| said.try_recv().ok())
    }

    /// Waits until `then`, unless the call must stop first.
    fn wait_until(&self, then: Instant) -> Result<(), Ended> {
        let wait = then.saturating_duration_since(Instant::now());
        self.hearing(|said| said.recv_timeout(wait).ok())
    }

    /// What the watch has said, kept, or else what `listen` hears now.
    fn hearing(
        &self,
        listen: impl FnOnce(&Receiver<String>) -> Option<String>,
    ) -> Result<(), Ended> {
        let mut heard = self
            .heard
            .lock()
            .map_err(|_| Ended::Stopped("the watch broke".into()))?;
        if heard.is_none() {
            *heard = listen(&self.said);
        }
        heard.clone().map_or(Ok(()), |why| Err(Ended::Stopped(why)))
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
/// is Joel's runtime folder, `cache` the folder both profiles go in and
/// `state` where an enrolment is kept.
pub fn run(
    chrome: &Path,
    page: &Path,
    seconds: u64,
    runtime: &Path,
    cache: &Path,
    state: &Path,
) -> Result<Ended, Fault> {
    let watch = Watch::start();
    let folders = Folders {
        runtime,
        cache,
        state,
    };
    let ended = settled(steps(chrome, page, seconds, &folders, &watch), &watch);
    // An enrolment that couldn't be kept was left in its profile by the
    // call's clean-up; looking again keeps it if it now can.
    let id = std::process::id();
    let mine = [
        cache.join(format!("profile-{id}")),
        cache.join(format!("far-{id}")),
    ];
    let left = unenrolled(&[mine[0].as_path(), mine[1].as_path()], state);
    if let Err(Enrolled { why, kept }) = left {
        if kept {
            for profile in &mine {
                let _ = fs::remove_dir_all(profile);
            }
        }
        return Err(Fault::Enrolled(why));
    }
    // The run starts only with nothing kept, so anything kept now was
    // found by this run, perhaps only as its clean-up looked, and this
    // run says so rather than the next hour.
    match managed::stays_off(state) {
        Some(why) => Err(Fault::Enrolled(why)),
        None => ended.map_err(Fault::Failed),
    }
}

/// Why a call gave no figures: a test browser enrolled, which refuses the
/// run as every later hour is refused, or the call couldn't be made.
#[derive(Debug)]
pub enum Fault {
    Enrolled(String),
    Failed(String),
}

/// The folders a call is made in, as `run` names them.
struct Folders<'a> {
    runtime: &'a Path,
    cache: &'a Path,
    state: &'a Path,
}

/// How the call's steps ended, as the run says it.
fn settled(steps: Result<Ended, Step>, watch: &Watch) -> Result<Ended, String> {
    steps.or_else(|step| match step {
        Step::Stopped(ended) => Ok(ended),
        // An alarm is never a skip, whatever the watch has heard since.
        Step::Refused(why) => Err(why),
        // A step the watch broke off by ending its processes fails, but
        // the call stopped for Joel's, and counts for nothing.
        Step::Failed(why) => match watch.stop() {
            Err(ended) => Ok(ended),
            Ok(()) => Err(why),
        },
    })
}

/// Why a call's steps ended early: Joel's call, an alarm (a real camera,
/// or an enrolled browser), or a failure.
enum Step {
    Stopped(Ended),
    Refused(String),
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

/// Removes what earlier runs ended without their cleanup left in
/// `runtime` and `cache`, looking in each profile first (`sweep`). It's
/// run before the hour's gates, so a skipped hour still looks. An error
/// is always an enrolment.
pub fn swept(runtime: &Path, cache: &Path, state: &Path) -> Result<(), String> {
    sweep(runtime, "call-under-load-", state)?;
    sweep(cache, "profile-", state)?;
    sweep(cache, "far-", state)
}

/// Removes what an earlier run ended without its cleanup left in
/// `folder`: each `<prefix><pid>` whose process is gone. A run killed by
/// a signal never looked in its profiles, so each is looked in here, and
/// an enrolment in one is kept in `state` and refuses the run.
fn sweep(folder: &Path, prefix: &str, state: &Path) -> Result<(), String> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name.to_str().and_then(|n| n.strip_prefix(prefix)) else {
            continue;
        };
        if pid.parse::<u32>().is_ok() && !Path::new("/proc").join(pid).exists() {
            let left = entry.path();
            // A run before the far end's profile moved to the cache made
            // it inside the runtime folder.
            let far = left.join("far");
            // What it was given is copied into `state` before the folder
            // goes, and a folder whose enrolment couldn't be kept stays.
            let looked = unenrolled(&[left.as_path(), far.as_path()], state);
            if let Err(Enrolled { why, kept }) = looked {
                if kept {
                    let _ = fs::remove_dir_all(&left);
                }
                return Err(why);
            }
            let _ = fs::remove_dir_all(&left);
        }
    }
    Ok(())
}

/// The call's steps in their order, each followed by its check for Joel's call.
fn steps(
    chrome: &Path,
    page: &Path,
    seconds: u64,
    folders: &Folders,
    watch: &Watch,
) -> Result<Ended, Step> {
    let id = std::process::id();
    // A Wayland socket's path must fit in 108 bytes, so it's kept short.
    let memory = folders.runtime.join(format!("call-under-load-{id}"));
    let profile = folders.cache.join(format!("profile-{id}"));
    // Both profiles are in the cache, not the runtime folder, which a
    // logout or a reboot empties before a sweep could look in it.
    let far_profile = folders.cache.join(format!("far-{id}"));
    let mut started = Started {
        folders: vec![memory.clone(), profile.clone(), far_profile.clone()],
        children: Vec::new(),
        profiles: vec![profile.clone(), far_profile.clone()],
        state: folders.state.to_path_buf(),
    };
    let config = memory.join("config");
    lay_out(&memory, &profile, &config).map_err(|e| e.to_string())?;

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
    let profiles = [profile.as_path(), far_profile.as_path()];
    unenrolled(&profiles, folders.state)?;
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
        unenrolled(&profiles, folders.state)?;
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

/// Makes the call's folders: a home and a config in `memory`, whose
/// pkcs11 module file keeps the Chromes off the TPM, and Chrome's mapped
/// caches in `memory`, linked from `profile`.
fn lay_out(memory: &Path, profile: &Path, config: &Path) -> std::io::Result<()> {
    fs::create_dir_all(config.join("pkcs11/modules"))?;
    fs::create_dir_all(memory.join("home"))?;
    fs::write(
        config.join("pkcs11/modules/tpm2_pkcs11.module"),
        "enable-in: none-of-these\n",
    )?;
    for folder in MAPPED {
        let (kept, linked) = (memory.join("maps").join(folder), profile.join(folder));
        fs::create_dir_all(&kept)?;
        fs::create_dir_all(linked.parent().unwrap_or(profile))?;
        std::os::unix::fs::symlink(&kept, &linked)?;
    }
    Ok(())
}

/// Ends the call the moment either browser has enrolled with a management
/// service, which keeps the token it's given in its profile's
/// `Policy/Enrollment`. Chromium reads no enrolment token, so it never
/// should; this is the alarm behind that, since an enrolled test browser
/// shows up in the machine owner's management as one of Joel's.
/// What each profile was given is kept in `state` first, which keeps the
/// test off until someone has read it (`managed::keep`).
fn unenrolled(profiles: &[&Path], state: &Path) -> Result<(), Enrolled> {
    let mut found: Option<Enrolled> = None;
    for profile in profiles {
        let enrolment = profile.join("Policy/Enrollment");
        if !fs::read_dir(&enrolment).is_ok_and(|mut tokens| tokens.next().is_some()) {
            continue;
        }
        let (why, kept) = match managed::keep(state, profile) {
            Ok(why) => (why, true),
            Err(why) => (why, false),
        };
        found = Some(match found {
            None => Enrolled { why, kept },
            Some(earlier) => Enrolled {
                why: format!("{}; {why}", earlier.why),
                kept: earlier.kept && kept,
            },
        });
    }
    found.map_or(Ok(()), Err)
}

/// What `unenrolled` found: why the call ends, and whether every enrolled
/// profile's evidence was kept, which alone lets the profiles go.
#[derive(Debug)]
struct Enrolled {
    why: String,
    kept: bool,
}

impl From<Enrolled> for Step {
    fn from(enrolled: Enrolled) -> Self {
        Step::Refused(enrolled.why)
    }
}

/// Refuses a camera that isn't Chromium's fake one, as `end`'s page named it.
/// It's an alarm, not the gate: the label is read once the camera's open,
/// and what keeps a real one shut is `--use-fake-device-for-media-stream`,
/// with which Chrome lists only its fake devices.
fn fake(said: &Value, end: &str) -> Result<(), Step> {
    let label = said["label"].as_str().unwrap_or("");
    if label.contains("fake") {
        return Ok(());
    }
    Err(Step::Refused(format!(
        "the {end} end's camera was {label:?}, not Chromium's fake one"
    )))
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
        // Joel's session bus is named by an absolute path, which a runtime
        // folder of its own doesn't hide, so the browser isn't told it.
        .env_remove("DBUS_SESSION_BUS_ADDRESS")
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
#[path = "call_tests.rs"]
mod tests;
