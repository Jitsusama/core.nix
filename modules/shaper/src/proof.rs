//! `shaper proof`: whether the shaper beats the router alone on this
//! link, measured as Joel would feel it while the agents fill the upload.
//!
//! ```text
//! shaper proof [--server URL] [--pairs 12] [--seconds 30] [--uploads 32] [--wait 0]
//!              [--out FILE]
//! ```
//!
//! Its server takes what is posted to `/__up` and answers
//! `/__down?bytes=N` with N bytes; the machine names it in
//! `jitsusama.shaper.speedServer`, or `--server` does.
//!
//! It runs `--uploads` uploads from jobs.slice for the whole run, and in
//! each block, from app.slice as Joel's own programs: an upload and a
//! download, each repeated, a fresh HTTPS connection each second, and a STUN
//! round trip five times a second, as a call's packets would go. Each pair
//! of blocks has the shaper on for one and off for the other, in turn
//! first, so a link that drifts through the run favours neither.
//!
//! The load goes on the link only while no call is on and Joel has been
//! idle two minutes, as Wayland's idle notice tells swayidle; the moment
//! either changes, every load stops, the shaper goes back as it was, the
//! pair under way is dropped, and it waits for his next idle stretch. It
//! waits `--wait` seconds from its start in all, and exits 75 if they run
//! out before every pair is measured, writing what it measured to
//! `--out`. The call's packets are src/call.rs's.
//!
//! It passes when, with the shaper on, the call's 99th percentile is
//! within `CALL_MARGIN_MS` of its median through the shaper before the
//! proof's load starts, Joel's upload gets at least half of what the link
//! sends, and at least `FAVOUR` of every `FAVOUR_OF` pairs favour the
//! shaper; src/proof/verdict.rs judges it. A pair says nothing of the
//! shaper, and is void, when its block with the shaper on found CAKE
//! never held a queue, or when the server refused any of Joel's requests
//! in either block. A request never answered still counts, since a
//! shaper starving Joel would look the same.

mod verdict;

use crate::qdisc;
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, ExitCode, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use verdict::{Block, answers, judge, percentile};

/// Where the server is named when `--server` doesn't: the machine's
/// `jitsusama.shaper.speedServer`, which the wrapper sets.
const SERVER_ENV: &str = "SHAPER_SPEED_SERVER";
/// How long an arm settles before its block counts: the shaper takes the
/// queue back within ten seconds of the load, and the router's queue
/// drains as long once it's off.
const SETTLE: Duration = Duration::from_secs(15);
/// What the gate's idle notice waits for, in seconds.
const IDLE_S: u32 = 120;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Arm {
    On,
    Off,
}

/// The arms of a pair, the shaper's first in every other one.
fn arms(pair: u32) -> [Arm; 2] {
    if pair.is_multiple_of(2) {
        [Arm::On, Arm::Off]
    } else {
        [Arm::Off, Arm::On]
    }
}

const USAGE: &str = "shaper proof [--server URL] [--pairs N] [--seconds S] [--uploads N] \
                     [--wait S] [--out FILE]";

/// What the proof is asked.
struct Asked {
    pairs: u32,
    seconds: u64,
    uploads: u32,
    wait: u64,
    out: Option<PathBuf>,
    /// The speed server, with no slash at its end.
    server: String,
}

/// Reads the flags, the server falling back on `named`, the machine's.
fn asked(args: &[String], named: Option<String>) -> Result<Asked, String> {
    let mut server = named;
    let mut a = Asked {
        server: String::new(),
        pairs: 12,
        seconds: 30,
        uploads: 32,
        wait: 0,
        out: None,
    };
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        let value = it.next().ok_or(format!("{flag} needs a value"))?;
        let number = || {
            value
                .parse::<u64>()
                .map_err(|_| format!("{flag} {value}: not a number"))
        };
        match flag.as_str() {
            "--pairs" => a.pairs = u32::try_from(number()?).map_err(|e| e.to_string())?,
            "--seconds" => a.seconds = number()?,
            "--uploads" => a.uploads = u32::try_from(number()?).map_err(|e| e.to_string())?,
            "--wait" => a.wait = number()?,
            "--out" => a.out = Some(PathBuf::from(value)),
            "--server" => server = Some(value.clone()),
            other => return Err(format!("unknown {other}")),
        }
    }
    a.server = server
        .filter(|s| !s.is_empty())
        .ok_or("no speed server: give --server, or set jitsusama.shaper.speedServer")?
        .trim_end_matches('/')
        .to_string();
    Ok(a)
}

/// Joel's being away and no call on, held as swayidle tells it.
struct Gate {
    mark: PathBuf,
    idle: Child,
}

impl Gate {
    fn new() -> Option<Gate> {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")?;
        let mark = PathBuf::from(runtime).join(format!("shaper-proof-idle-{}", std::process::id()));
        let _ = fs::remove_file(&mark);
        let shown = mark.display().to_string();
        let idle = Command::new("swayidle")
            .args(["timeout", &IDLE_S.to_string(), &format!("touch {shown}")])
            .args(["resume", &format!("rm -f {shown}")])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .ok()?;
        Some(Gate { mark, idle })
    }

    /// Whether the link may carry the load now. A call detector that
    /// can't tell is taken for a call.
    fn open(&self) -> bool {
        self.mark.exists()
            && Command::new("a-call-is-on")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|s| s.code() == Some(1))
    }
}

impl Drop for Gate {
    fn drop(&mut self) {
        let _ = self.idle.kill();
        let _ = self.idle.wait();
        let _ = fs::remove_file(&self.mark);
    }
}

/// Starts `command` in a scope of its own under `slice`, named so the
/// proof can stop every scope it started at once.
fn scoped(slice: &str, name: &str, command: &[&str]) -> Command {
    let mut c = Command::new("systemd-run");
    c.args(["--user", "--quiet", "--collect", "--scope"])
        .arg(format!("--slice={slice}"))
        .arg(format!("--unit=shaper-proof-{name}-{}", std::process::id()))
        .arg("--")
        .args(command)
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    c
}

/// Stops every scope this proof started.
fn stop_scopes() {
    let _ = Command::new("systemctl")
        .args([
            "--user",
            "stop",
            &format!("shaper-proof-*-{}.scope", std::process::id()),
        ])
        .status();
}

fn systemctl(verb: &str) -> bool {
    Command::new("systemctl")
        .args([verb, "shaper.service"])
        .stdout(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// The link the default route leaves by.
fn uplink() -> Option<String> {
    let out = Command::new("ip")
        .args(["-j", "route", "show", "default"])
        .output()
        .ok()?;
    let routes: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).ok()?;
    routes.first()?.get("dev")?.as_str().map(str::to_string)
}

fn sent_bytes(link: &str) -> u64 {
    fs::read_to_string(format!("/sys/class/net/{link}/statistics/tx_bytes"))
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

fn kbit(bytes: f64, seconds: f64) -> f64 {
    bytes * 8.0 / 1_000.0 / seconds
}

/// Measures one block: Joel's four probes for `seconds`, while the load
/// runs, reading CAKE's queue each second.
fn block(arm: Arm, link: &str, asked: &Asked, stop: &AtomicBool) -> Option<Block> {
    let seconds = asked.seconds;
    let server = &asked.server;
    let s = seconds.to_string();
    let me = std::env::current_exe().ok()?;
    let me = me.to_str()?;
    // Each probe says each answer's status beside what it measured, so a
    // server refusing Joel's requests is told from a shaper slowing them,
    // and the upload is repeated as the download is, since a server may
    // end one early.
    let up = format!(
        "end=$((SECONDS + {s})); while [ $SECONDS -lt $end ]; do head -c 4000000000 /dev/zero \
         | curl -s -o /dev/null -X POST -T - --max-time $((end - SECONDS + 1)) \
         -w '%{{http_code}} %{{size_upload}}\\n' '{server}/__up'; done"
    );
    let down = format!(
        "end=$((SECONDS + {s})); while [ $SECONDS -lt $end ]; do curl -s -o /dev/null \
         --max-time $((end - SECONDS + 1)) -w '%{{http_code}} %{{size_download}}\\n' \
         '{server}/__down?bytes=25000000'; done"
    );
    let first = format!(
        "end=$((SECONDS + {s})); while [ $SECONDS -lt $end ]; do curl -s -o /dev/null \
         -w '%{{http_code}} %{{time_starttransfer}}\\n' '{server}/__down?bytes=0'; \
         sleep 1; done"
    );
    let probes: Vec<Child> = [
        scoped("app.slice", "up", &["sh", "-c", &up]),
        scoped("app.slice", "down", &["sh", "-c", &down]),
        scoped("app.slice", "first", &["sh", "-c", &first]),
        scoped("app.slice", "call", &[me, "proof-call", s.as_str()]),
    ]
    .into_iter()
    .map(|mut c| c.stdout(Stdio::piped()).spawn())
    .collect::<Result<_, _>>()
    .ok()?;
    let sent_then = sent_bytes(link);
    let began = Instant::now();
    let mut owned = false;
    while began.elapsed() < Duration::from_secs(seconds) && !stop.load(Ordering::Relaxed) {
        if arm == Arm::On
            && let Ok(out) = Command::new("tc")
                .args(["-s", "-j", "qdisc", "show", "dev", link])
                .output()
            && let Some((backlog, _)) = qdisc::holding(&String::from_utf8_lossy(&out.stdout))
        {
            owned |= backlog >= qdisc::QUEUED_BYTES;
        }
        // CAKE's queue is read once a second through the block.
        thread::sleep(Duration::from_secs(1));
    }
    let elapsed = began.elapsed().as_secs_f64();
    let sent = sent_bytes(link).saturating_sub(sent_then);
    let outs: Vec<String> = probes
        .into_iter()
        .map(|c| {
            c.wait_with_output()
                .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
                .unwrap_or_default()
        })
        .collect();
    if stop.load(Ordering::Relaxed) {
        return None;
    }
    #[expect(clippy::cast_precision_loss, reason = "bytes far under 2^52")]
    let sent_kbit = kbit(sent as f64, elapsed);
    let call = &outs[3];
    let (up, down, first) = (answers(&outs[0]), answers(&outs[1]), answers(&outs[2]));
    Some(Block {
        shaped: arm == Arm::On,
        up_kbit: kbit(up.total, elapsed),
        down_kbit: kbit(down.total, elapsed),
        sent_kbit,
        call_ms: call.lines().filter_map(|l| l.parse().ok()).collect(),
        lost: call.lines().filter(|l| *l == "lost").count(),
        first_byte_ms: first.each.iter().map(|s| s * 1_000.0).collect(),
        owned,
        refused: up.refused + down.refused + first.refused,
        unanswered: up.unanswered + down.unanswered + first.unanswered,
    })
}

/// The call's round trips with nothing loading the link.
fn idle_call(seconds: u64) -> Vec<f64> {
    let Some(me) = std::env::current_exe().ok() else {
        return Vec::new();
    };
    let s = seconds.to_string();
    let Some(me) = me.to_str() else {
        return Vec::new();
    };
    scoped("app.slice", "idle", &[me, "proof-call", s.as_str()])
        .stdout(Stdio::piped())
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|l| l.parse().ok())
                .collect()
        })
        .unwrap_or_default()
}

fn say(block: &Block, arm: Arm) {
    eprintln!(
        "  {:<3} Joel up {:>6.0} kbit/s of {:>6.0} ({:>3.0}%), down {:>7.0}, \
         call p50 {:>5.1} p99 {:>5.1} ms, {} lost, first byte p50 {:>4.0} ms, \
         {} refused, {} unanswered{}",
        if arm == Arm::On { "on" } else { "off" },
        block.up_kbit,
        block.sent_kbit,
        block.share() * 100.0,
        block.down_kbit,
        percentile(&block.call_ms, 0.5),
        percentile(&block.call_ms, 0.99),
        block.lost,
        percentile(&block.first_byte_ms, 0.5),
        block.refused,
        block.unanswered,
        block
            .void()
            .map(|why| format!(", void: {why}"))
            .unwrap_or_default()
    );
}

/// `shaper proof`: the run itself.
pub fn command(args: &[String]) -> ExitCode {
    let asked = match asked(args, std::env::var(SERVER_ENV).ok()) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}\nusage: {USAGE}");
            return ExitCode::from(2);
        }
    };
    let Some(link) = uplink() else {
        eprintln!("no default route to measure");
        return ExitCode::FAILURE;
    };
    let Some(gate) = Gate::new() else {
        eprintln!("couldn't start swayidle to hear whether Joel is away");
        return ExitCode::FAILURE;
    };
    let gate = Arc::new(gate);
    let deadline = Instant::now() + Duration::from_secs(asked.wait);
    let was_on = systemctl("is-active");
    let mut idle: Vec<f64> = Vec::new();
    let (pairs, complete) = across_windows(
        asked.pairs,
        || opened(&gate, deadline),
        |from| window(&asked, &link, &gate, was_on, from, &mut idle),
    );
    if pairs.is_empty() {
        eprintln!("Joel hasn't been idle two minutes, or a call is on: not loading the link");
        return ExitCode::from(75);
    }
    let verdict = judge(&idle, &pairs);
    verdict.say();
    if let Some(out) = &asked.out {
        let report = serde_json::json!({
            "verdict": verdict, "complete": complete, "idle_call_ms": idle, "pairs": pairs
        });
        if fs::write(out, report.to_string()).is_err() {
            eprintln!("couldn't write {}", out.display());
        }
    }
    if !complete {
        eprintln!(
            "the wait ran out after {} of {} pairs: not enough to judge",
            pairs.len(),
            asked.pairs
        );
        return ExitCode::from(75);
    }
    if verdict.passed() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Starts the agents' uploads, from jobs.slice, each going again as it ends.
fn load(uploads: u32, server: &str) {
    let load = format!(
        "while :; do head -c 400000000 /dev/zero | curl -s -o /dev/null -X POST -T - \
         --max-time 120 '{server}/__up'; done"
    );
    for n in 0..uploads {
        let _ = scoped("jobs.slice", &format!("load{n}"), &["sh", "-c", &load])
            .stdout(Stdio::null())
            .spawn();
    }
}

/// Gathers the pairs across Joel's idle windows, `opened` waiting for the
/// next and `window` measuring from the pair it's given until the gate
/// closes. Each window starts after the last pair kept, since a pair's
/// arms run back to back and so compare alike in any window. Returns the
/// pairs and whether all were measured before the wait ran out.
fn across_windows(
    pairs: u32,
    mut opened: impl FnMut() -> bool,
    mut window: impl FnMut(u32) -> Vec<[Block; 2]>,
) -> (Vec<[Block; 2]>, bool) {
    let mut kept: Vec<[Block; 2]> = Vec::new();
    while kept.len() < pairs as usize {
        if !opened() {
            return (kept, false);
        }
        let from = u32::try_from(kept.len()).unwrap_or(pairs);
        kept.extend(window(from));
    }
    (kept, true)
}

/// Waits for the gate to open, until `deadline`.
fn opened(gate: &Gate, deadline: Instant) -> bool {
    while !gate.open() {
        if Instant::now() >= deadline {
            return false;
        }
        // The gate is asked once a second while it's waited for.
        thread::sleep(Duration::from_secs(1));
    }
    true
}

/// One idle window: the load and the pairs from `from` on, until they're
/// done or the gate closes, the shaper going back to `was_on` after. The
/// call's quiet is heard first, in the run's first window.
fn window(
    asked: &Asked,
    link: &str,
    gate: &Arc<Gate>,
    was_on: bool,
    from: u32,
    idle: &mut Vec<f64>,
) -> Vec<[Block; 2]> {
    // The rest of the machine's traffic runs through the whole proof, and
    // with the router alone it bloats the router's queue: the first run's
    // idle median was 81 ms, where a loaded call through the shaper took
    // 38. So the call's quiet is heard with the shaper on, and the bar
    // asks what the proof's own load adds to it.
    if idle.is_empty() {
        systemctl("start");
        // The shaper settles before its quiet counts, as SETTLE says.
        thread::sleep(SETTLE);
        eprintln!(
            "idle: the call's round trips through the shaper, with the proof loading nothing"
        );
        *idle = idle_call(asked.seconds.min(15));
    }
    let stop = Arc::new(AtomicBool::new(false));
    let watch = {
        let (stop, gate) = (Arc::clone(&stop), Arc::clone(gate));
        thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                if !gate.open() {
                    eprintln!("Joel is back, or a call started: stopping the load");
                    stop.store(true, Ordering::Relaxed);
                    stop_scopes();
                }
                // The gate is asked once a second while the load runs.
                thread::sleep(Duration::from_secs(1));
            }
        })
    };
    load(asked.uploads, &asked.server);
    let pairs = run_pairs(asked, link, &stop, from);
    let stopped = stop.swap(true, Ordering::Relaxed);
    stop_scopes();
    let _ = watch.join();
    systemctl(if was_on { "start" } else { "stop" });
    if stopped {
        eprintln!(
            "kept {} pairs, the one under way dropped; waiting for Joel's next idle",
            from as usize + pairs.len()
        );
    }
    pairs
}

/// Measures the pairs, the shaper on for one block of each and off for the
/// other, until they're done or the gate closes.
fn run_pairs(asked: &Asked, link: &str, stop: &AtomicBool, from: u32) -> Vec<[Block; 2]> {
    let mut pairs: Vec<[Block; 2]> = Vec::new();
    'pairs: for pair in from..asked.pairs {
        eprintln!("pair {} of {}", pair + 1, asked.pairs);
        let mut done: Vec<Block> = Vec::new();
        for arm in arms(pair) {
            systemctl(if arm == Arm::On { "start" } else { "stop" });
            // Each arm settles before it counts, as SETTLE says.
            thread::sleep(SETTLE);
            if stop.load(Ordering::Relaxed) {
                break 'pairs;
            }
            let Some(b) = block(arm, link, asked, stop) else {
                break 'pairs;
            };
            say(&b, arm);
            done.push(b);
        }
        if let Ok(both) = <[Block; 2]>::try_from(done) {
            pairs.push(both);
        }
    }
    pairs
}

#[cfg(test)]
mod tests {
    use super::verdict::tests::pairs;
    use super::*;

    #[test]
    fn pairs_measured_in_three_windows_are_kept_as_one_run() {
        let mut froms = Vec::new();
        let mut windows = [5, 0, 7].into_iter();
        let (kept, complete) = across_windows(
            12,
            || true,
            |from| {
                froms.push(from);
                pairs(windows.next().unwrap_or(0), 0.8, 0.05)
            },
        );
        assert_eq!((kept.len(), complete), (12, true));
        assert_eq!(
            froms,
            [0, 5, 5],
            "a window didn't start after the last pair kept"
        );
    }

    #[test]
    fn a_wait_that_runs_out_keeps_the_pairs_measured() {
        let mut opens = [true, false].into_iter();
        let (kept, complete) = across_windows(
            12,
            || opens.next().unwrap_or(false),
            |_| pairs(4, 0.8, 0.05),
        );
        assert_eq!((kept.len(), complete), (4, false));
    }

    #[test]
    fn the_shaper_goes_first_in_every_other_pair() {
        assert_eq!(arms(0), [Arm::On, Arm::Off]);
        assert_eq!(arms(1), [Arm::Off, Arm::On]);
    }

    #[test]
    fn the_flags_are_read() {
        let args: Vec<String> = ["--pairs", "6", "--seconds", "20"]
            .map(String::from)
            .to_vec();
        let a = asked(&args, Some("https://speed.example.net/".into())).expect("flags");
        assert_eq!((a.pairs, a.seconds, a.uploads), (6, 20, 32));
        assert_eq!(a.server, "https://speed.example.net");
        let named = asked(&["--server".into(), "https://b.example".into()], None).expect("server");
        assert_eq!(named.server, "https://b.example");
        assert!(
            asked(&["--pairs".to_string()], Some("https://a.example".into())).is_err(),
            "a flag with no value taken"
        );
        assert!(asked(&[], None).is_err(), "a proof with no server to load");
    }
}
