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
//! download, each repeated, a fresh HTTPS connection each second, and a DNS
//! round trip five times a second, as a call's packets would go. Each pair
//! of blocks has the shaper on for one and off for the other, in turn
//! first, so a link that drifts through the run favours neither.
//!
//! The load goes on the link only while no call is on, no audio plays,
//! nothing the machine names in `jitsusama.shaper.proofWaitsWhile` says
//! it's busy, such as a meeting within ten minutes, since a call can ring
//! before its microphone opens, and Joel has been idle two minutes, as
//! Wayland's idle notice tells swayidle. The moment any of them changes,
//! every load stops, the shaper goes back as it was, the pair under way
//! is dropped, and it waits for his next idle stretch. SIGTERM, SIGINT
//! and SIGHUP end it the same way, writing what it measured to `--out`
//! and exiting 130. A proof killed outright can put nothing back, but its
//! load ends within about two seconds, and the next proof puts the shaper back
//! as the killed one found it. It waits `--wait` seconds from its start
//! in all, and exits 75 if they run out before every pair is measured,
//! writing what it measured to `--out`. It exits 69 if it couldn't put
//! the shaper back, saying how to. The call's packets are src/call.rs's.
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

mod gate;
mod load;
mod stop;
mod verdict;

use crate::qdisc;
use gate::{Gate, WAITS_ENV, waits_while};
use load::stop_scopes;
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use stop::{STOPPED, halted, put_back, systemctl, tell};
use terminal_kit::{Exit, Failure};
use verdict::{Block, answers, judge, percentile};

/// Where the server is named when `--server` doesn't: the machine's
/// `jitsusama.shaper.speedServer`, which the wrapper sets.
const SERVER_ENV: &str = "SHAPER_SPEED_SERVER";
/// How long an arm settles before its block counts: the shaper takes the
/// queue back within ten seconds of the load, and the router's queue
/// drains as long once it's off.
const SETTLE: Duration = Duration::from_secs(15);

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

/// A shell loop asking `request` again until `seconds` have passed, and
/// waiting `every` after each. A server that refused one, its status 400
/// or more, is given a second before the next: asked again at once, it
/// went on refusing for most of an hour.
fn repeated(seconds: &str, request: &str, every: &str) -> String {
    format!(
        "end=$((SECONDS + {seconds})); while [ $SECONDS -lt $end ]; do a=$({request}); \
         echo \"$a\"; case $a in [45]*) sleep 1;; esac; {every} done"
    )
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
    // end one early. The download comes in pieces of 5 MB, since the
    // server refuses one of 10 MB or more once it has sent many.
    let up = repeated(
        &s,
        &format!(
            "head -c 4000000000 /dev/zero | curl -s -o /dev/null -X POST -T - \
             --max-time $((end - SECONDS + 1)) -w '%{{http_code}} %{{size_upload}}\\n' \
             '{server}/__up'"
        ),
        "",
    );
    let down = repeated(
        &s,
        &format!(
            "curl -s -o /dev/null --max-time $((end - SECONDS + 1)) \
             -w '%{{http_code}} %{{size_download}}\\n' '{server}/__down?bytes=5000000'"
        ),
        "",
    );
    let first = repeated(
        &s,
        &format!(
            "curl -s -o /dev/null -w '%{{http_code}} %{{time_starttransfer}}\\n' \
             '{server}/__down?bytes=0'"
        ),
        // A fresh connection a second, as a page's would be.
        "sleep 1;",
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
    while began.elapsed() < Duration::from_secs(seconds) && !halted(stop) {
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
    if halted(stop) {
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
    tell(&said(block, arm));
}

/// What one block measured, as a line of the run's progress.
fn said(block: &Block, arm: Arm) -> String {
    format!(
        "  {} Joel up {:.0} kbit/s of {:.0} ({:.0}%), down {:.0}, \
         call p50 {:.1} p99 {:.1} ms, {} lost, first byte p50 {:.0} ms, \
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
    )
}

/// `shaper proof`: the run itself.
pub fn command(args: &[String]) -> Result<(), Failure> {
    // Heard from the start, so a stop while swayidle starts still ends it.
    stop::hear_stops();
    let asked =
        asked(args, std::env::var(SERVER_ENV).ok()).map_err(|e| crate::wrongly("proof", e))?;
    let Some(link) = uplink() else {
        return Err(Failure::new(
            Exit::Unavailable,
            "no default route to measure",
        ));
    };
    let waits = waits_while(std::env::var(WAITS_ENV).ok().as_deref())
        .map_err(|e| Failure::new(Exit::Config, e))?;
    let Some(gate) = Gate::new(waits) else {
        return Err(Failure::new(
            Exit::Unavailable,
            "couldn't start swayidle to hear whether Joel is away",
        ));
    };
    let gate = Arc::new(gate);
    let deadline = Instant::now() + Duration::from_secs(asked.wait);
    let marker = stop::marker();
    let was_on = stop::own_state(&marker, systemctl("is-active"));
    let mut idle: Vec<f64> = Vec::new();
    let (pairs, complete) = across_windows(
        asked.pairs,
        || opened(|| gate.open(), deadline, &STOPPED),
        |from| window(&asked, &link, &gate, was_on, from, &mut idle),
    );
    let back = put_back(was_on);
    if back {
        stop::forget(&marker);
    }
    let stopped = STOPPED.load(Ordering::Relaxed);
    let verdict = (!pairs.is_empty()).then(|| judge(&idle, &pairs));
    if let Some(verdict) = &verdict {
        tell(&verdict.said());
        if let Some(out) = &asked.out {
            let report = serde_json::json!({
                "verdict": verdict, "complete": complete, "stopped": stopped,
                "idle_call_ms": idle, "pairs": pairs
            });
            if fs::write(out, report.to_string()).is_err() {
                tell(&format!("couldn't write {}", out.display()));
            }
        }
    }
    let state = if was_on { "on" } else { "off" };
    if !back {
        return Err(
            Failure::new(Exit::Unavailable, "couldn't put the shaper back").because(format!(
                "it should be {state}: systemctl {} shaper.service",
                if was_on { "start" } else { "stop" }
            )),
        );
    }
    if stopped {
        return Err(Failure::new(Exit::Interrupted, "stopped").because(format!(
            "a signal asked it to; the shaper is back {state}, its load ended"
        )));
    }
    let Some(verdict) = verdict else {
        return Err(Failure::new(Exit::TryAgain, "not loading the link")
            .because("Joel hasn't been idle two minutes, or a call, audio or a meeting is on"));
    };
    if !complete {
        return Err(
            Failure::new(Exit::TryAgain, "not enough to judge").because(format!(
                "the wait ran out after {} of {} pairs",
                pairs.len(),
                asked.pairs
            )),
        );
    }
    if verdict.passed() {
        Ok(())
    } else {
        // The verdict has said why on its own lines.
        Err(Failure::new(
            Exit::Failed,
            "the shaper didn't beat the router",
        ))
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

/// Waits for `open` to say the gate is open, until `deadline` or a stop.
fn opened(open: impl Fn() -> bool, deadline: Instant, stopped: &AtomicBool) -> bool {
    while !stopped.load(Ordering::Relaxed) && !open() {
        if Instant::now() >= deadline {
            return false;
        }
        // The gate is asked once a second while it's waited for.
        thread::sleep(Duration::from_secs(1));
    }
    !stopped.load(Ordering::Relaxed)
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
    let stop = Arc::new(AtomicBool::new(false));
    if idle.is_empty() {
        systemctl("start");
        // The shaper settles before its quiet counts.
        settle(&stop);
        if !halted(&stop) {
            tell("idle: the call's round trips through the shaper, with the proof loading nothing");
            *idle = idle_call(asked.seconds.min(15));
        }
    }
    if halted(&stop) {
        put_back(was_on);
        return Vec::new();
    }
    let watch = {
        let (stop, gate) = (Arc::clone(&stop), Arc::clone(gate));
        thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                // The load is stopped before anything is said, since
                // saying it may fail.
                if STOPPED.load(Ordering::Relaxed) {
                    stop.store(true, Ordering::Relaxed);
                    stop_scopes();
                    tell("asked to stop: ending the load");
                } else if !gate.open() {
                    stop.store(true, Ordering::Relaxed);
                    stop_scopes();
                    tell("Joel is back, or a call, audio or a meeting started: stopping the load");
                }
                // The gate is asked once a second while the load runs.
                thread::sleep(Duration::from_secs(1));
            }
        })
    };
    load::start(asked.uploads, &asked.server);
    let pairs = run_pairs(asked, link, &stop, from);
    let closed = stop.swap(true, Ordering::Relaxed);
    stop_scopes();
    // Put back before the watch is joined, since it may be stuck asking
    // the gate.
    put_back(was_on);
    let _ = watch.join();
    if closed && !STOPPED.load(Ordering::Relaxed) {
        tell(&format!(
            "kept {} pairs, the one under way dropped; waiting for Joel's next idle",
            from as usize + pairs.len()
        ));
    }
    pairs
}

/// Measures the pairs, the shaper on for one block of each and off for the
/// other, until they're done or the gate closes.
fn run_pairs(asked: &Asked, link: &str, stop: &AtomicBool, from: u32) -> Vec<[Block; 2]> {
    let mut pairs: Vec<[Block; 2]> = Vec::new();
    'pairs: for pair in from..asked.pairs {
        tell(&format!("pair {} of {}", pair + 1, asked.pairs));
        let mut done: Vec<Block> = Vec::new();
        for arm in arms(pair) {
            if halted(stop) {
                break 'pairs;
            }
            systemctl(if arm == Arm::On { "start" } else { "stop" });
            settle(stop);
            if halted(stop) {
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

/// Lets an arm settle before it counts, as SETTLE says, or until a stop
/// or a signal.
fn settle(stop: &AtomicBool) {
    let began = Instant::now();
    while began.elapsed() < SETTLE && !halted(stop) {
        // A stop is heard within a second of a settle.
        thread::sleep(Duration::from_secs(1));
    }
}

#[cfg(test)]
mod tests {
    use super::verdict::tests::pairs;
    use super::*;

    #[test]
    fn a_stop_ends_the_wait_and_the_settle_at_once() {
        let stopped = AtomicBool::new(true);
        let began = Instant::now();
        let deadline = began + Duration::from_secs(5);
        assert!(
            !opened(|| false, deadline, &stopped),
            "opened while stopped"
        );
        settle(&stopped);
        assert!(
            began.elapsed() < Duration::from_secs(2),
            "a stop waited {:?} for Joel's idle and the settle",
            began.elapsed()
        );
    }

    #[test]
    fn a_block_is_said_in_words_without_padding() {
        let block = Block {
            up_kbit: 812.0,
            sent_kbit: 9_000.0,
            down_kbit: 4_000.0,
            call_ms: vec![21.0; 10],
            first_byte_ms: vec![90.0; 10],
            ..Block::default()
        };
        let line = said(&block, Arm::On);
        assert!(
            line.starts_with("  on Joel up 812 kbit/s of 9000 (9%)"),
            "{line}"
        );
        assert!(!line.trim_start().contains("  "), "{line}");
    }

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
    fn a_refused_request_waits_before_it_is_asked_again() {
        let refused = repeated("2", "echo '429 0'", "");
        let out = Command::new("sh")
            .args(["-c", &refused])
            .output()
            .expect("a shell");
        let asked = String::from_utf8_lossy(&out.stdout).lines().count();
        assert!(
            (1..=3).contains(&asked),
            "asked {asked} times in two seconds a server refused"
        );
        // Answered, a probe waits only what it asks to between requests.
        let answered = repeated("2", "echo '200 5'", "sleep 0.4;");
        let out = Command::new("sh")
            .args(["-c", &answered])
            .output()
            .expect("a shell");
        let asked = String::from_utf8_lossy(&out.stdout).lines().count();
        assert!(
            (2..=6).contains(&asked),
            "asked {asked} times, not every 0.4 s"
        );
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
