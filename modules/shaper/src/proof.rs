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
//! is dropped, and it waits for his next idle stretch. SIGTERM, SIGINT,
//! SIGHUP and SIGTSTP end it the same way, and so does any SIGCONT, since
//! a proof paused by SIGSTOP counts as gone and its keeper may already
//! have put the shaper back; it writes what it measured to `--out` and
//! exits 130. Each window reads the shaper afresh and owns it only while
//! it runs, so a switch or Joel's own change between windows stands. A
//! proof killed outright can put nothing back itself, but its load and
//! probes end within about two seconds, and a keeper beside it puts the
//! shaper back as its window found it, or, failing that, the next proof
//! does before it waits. One proof runs at a time; another exits 75 at
//! once. It waits `--wait` seconds from its start in all, and exits 75 if
//! they run out before every pair is measured, writing what it measured
//! to `--out`. It exits 69 if it couldn't put the shaper back, saying how
//! to, and leaves the shaper to Joel. When systemd doesn't say whether
//! the shaper is on, no window opens on it, and it exits 75 with the
//! shaper left as it stood. The call's packets are src/call.rs's.
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
mod keeper;
mod load;
mod lock;
mod probes;
mod stop;
mod verdict;
mod windows;

use crate::qdisc;
use gate::{Gate, WAITS_ENV, waits_while};
use load::{end_group, stop_scopes};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use stop::{STOPPED, halted, put_back, systemctl, tell};
use terminal_kit::{Exit, Failure};
use verdict::{Block, answers, judge, percentile};
use windows::{across_windows, opened, pay};

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
    // Anything else would reach curl as an option, not an address.
    if !a.server.starts_with("https://") && !a.server.starts_with("http://") {
        return Err(format!(
            "--server {}: not an http or https address",
            a.server
        ));
    }
    Ok(a)
}

/// Starts `command` in a scope of its own under `slice`, named so the
/// proof can stop every scope it started at once.
fn scoped(slice: &str, name: &str, command: &[&str]) -> Command {
    scoped_as(
        slice,
        &format!("shaper-proof-{name}-{}", std::process::id()),
        command,
    )
}

/// Starts `command` in a scope named `unit` under `slice`. A scope runs
/// its command in this process, which leads a group of its own, so what
/// it starts can end all of it at once, and a terminal's Ctrl-C or
/// Ctrl-Z reaches only the proof, which ends it. The command is passed
/// as it stands, since systemd-run would read a shell's `${...}` itself.
/// Its memory ceiling, far over what curl or a shell needs, keeps the
/// jobs sorter from taking a scope an agent starts back into its work:
/// Joel's probes there would be agents' traffic, and the keeper would
/// end with the agent's command.
fn scoped_as(slice: &str, unit: &str, command: &[&str]) -> Command {
    use std::os::unix::process::CommandExt;
    let mut c = Command::new("systemd-run");
    c.process_group(0);
    c.args([
        "--user",
        "--quiet",
        "--collect",
        "--scope",
        "--expand-environment=no",
    ])
    .arg("--property=MemoryMax=128M")
    .arg(format!("--slice={slice}"))
    .arg(format!("--unit={unit}"))
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

/// Measures one block: Joel's four probes for `seconds`, while the load
/// runs, reading CAKE's queue each second.
fn block(arm: Arm, link: &str, asked: &Asked, stop: &AtomicBool) -> Option<Block> {
    let seconds = asked.seconds;
    let server = &asked.server;
    let me = std::env::current_exe().ok()?;
    let me = me.to_str()?;
    let [up, down, first, call] = probes::scripts(seconds);
    let spawned: Vec<_> = [
        scoped("app.slice", "up", &["sh", "-c", &up, "sh", server]),
        scoped("app.slice", "down", &["sh", "-c", &down, "sh", server]),
        scoped("app.slice", "first", &["sh", "-c", &first, "sh", server]),
        scoped("app.slice", "call", &["sh", "-c", &call, "sh", me]),
    ]
    .into_iter()
    .map(|mut c| c.stdout(Stdio::piped()).spawn())
    .collect();
    // A block missing a probe measures nothing, and those started end.
    if spawned.iter().any(Result::is_err) {
        for mut probe in spawned.into_iter().flatten() {
            end_group(&probe);
            let _ = probe.wait();
        }
        return None;
    }
    let probes: Vec<Child> = spawned.into_iter().flatten().collect();
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
    // A stop ends the probes at once, whether or not systemd has their
    // scopes yet, and one held past its block ends then.
    let by = began + Duration::from_secs(seconds) + probes::GRACE;
    let outs: Vec<String> = probes
        .into_iter()
        .map(|p| probes::finish(p, by, || halted(stop)))
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

/// The call's round trips with nothing loading the link, until `seconds`
/// pass or a stop.
fn idle_call(seconds: u64, stop: &AtomicBool) -> Vec<f64> {
    let Some(me) = std::env::current_exe().ok() else {
        return Vec::new();
    };
    let Some(me) = me.to_str() else {
        return Vec::new();
    };
    let call = probes::call(seconds);
    let Ok(probe) = scoped("app.slice", "idle", &["sh", "-c", &call, "sh", me])
        .stdout(Stdio::piped())
        .spawn()
    else {
        return Vec::new();
    };
    let by = Instant::now() + Duration::from_secs(seconds) + probes::GRACE;
    probes::finish(probe, by, || halted(stop))
        .lines()
        .filter_map(|l| l.parse().ok())
        .collect()
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
    // Gone however this proof ends before its own is written, so an
    // earlier proof's report is never read for this one's, even when
    // this one is refused the lock.
    if let Some(out) = &asked.out {
        let _ = fs::remove_file(out);
    }
    let marker = stop::marker();
    // Held to the end, released however the proof ends.
    let _alone = marker.as_deref().map(alone).transpose()?;
    // Paid before anything else can end the proof, so a killed proof's
    // debt is paid by the next however far that one gets.
    let paid = marker
        .as_deref()
        .map(|marker| owed(marker, asked.out.as_deref()))
        .transpose()?
        .flatten();
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
    // Only a window starts or stops the shaper, so each owns it only while
    // it runs, reading it afresh first: between windows it is whoever's
    // sets it, a switch's or Joel's, and nothing puts it back.
    let mut idle: Vec<f64> = Vec::new();
    let windows = across_windows(
        asked.pairs,
        || opened(|| gate.open(), deadline, &STOPPED),
        stop::shaper_on,
        |from, on| {
            stop::owning(marker.as_deref(), on, || {
                window(&asked, &link, &gate, on, from, &mut idle)
            })
        },
    );
    // Ended here, since a watch left unjoined still holds the gate.
    gate.end();
    let stopped = STOPPED.load(Ordering::Relaxed);
    let pairs = &windows.pairs;
    let verdict = (!pairs.is_empty()).then(|| judge(&idle, pairs));
    if let Some(verdict) = &verdict {
        tell(&verdict.said());
    }
    // Written however the proof ends, its verdict null with no pairs, so
    // a report left by an earlier proof is never read for this one's.
    report(
        asked.out.as_deref(),
        &serde_json::json!({
            "verdict": verdict, "complete": windows.complete, "stopped": stopped,
            "shaper_back": windows.back, "idle_call_ms": idle, "pairs": pairs
        }),
    );
    windows.ending(paid, stopped)?;
    let Some(verdict) = verdict else {
        return Err(Failure::new(Exit::TryAgain, "not loading the link")
            .because("Joel hasn't been idle two minutes, or a call, audio or a meeting is on"));
    };
    if !windows.complete {
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

/// Writes `report` to `out`, when asked for, saying when it can't.
fn report(out: Option<&Path>, report: &serde_json::Value) {
    if let Some(out) = out
        && fs::write(out, report.to_string()).is_err()
    {
        tell(&format!("couldn't write {}", out.display()));
    }
}

/// Pays what an earlier proof killed outright left owing in `marker`,
/// so the shaper isn't left wrong through this one's wait, then starts
/// this proof's own keeper. The word goes either way: once paid, or once
/// Joel is told by the 69, with a report in `out` saying so.
fn owed(marker: &Path, out: Option<&Path>) -> Result<Option<bool>, Failure> {
    let paying = pay(stop::debt(marker), put_back);
    stop::forget(marker);
    if paying.is_err() {
        report(
            out,
            &serde_json::json!({
                "verdict": null, "complete": false, "stopped": false,
                "shaper_back": false, "idle_call_ms": [], "pairs": []
            }),
        );
    }
    let paid = paying?;
    keeper::keep(marker);
    Ok(paid)
}

/// The lock beside `marker` that only one proof holds at a time.
fn alone(marker: &Path) -> Result<fs::File, Failure> {
    lock::alone(marker).map_err(|why| match why {
        lock::NotAlone::Another(who) => Failure::new(
            Exit::TryAgain,
            format!("another proof is running, pid {who}"),
        )
        .because("two at once would load the link twice"),
        lock::NotAlone::Unopened(e) => {
            Failure::new(Exit::Unavailable, "couldn't open the proof's lock").because(e)
        }
    })
}

/// One idle window: the load and the pairs from `from` on, until they're
/// done or the gate closes, the shaper going back to `was_on` after, and
/// whether it went back. The call's quiet is heard first, in the run's
/// first window.
fn window(
    asked: &Asked,
    link: &str,
    gate: &Arc<Gate>,
    was_on: bool,
    from: u32,
    idle: &mut Vec<f64>,
) -> (Vec<[Block; 2]>, bool) {
    // The rest of the machine's traffic runs through the whole proof, and
    // with the router alone it bloats the router's queue: the first run's
    // idle median was 81 ms, where a loaded call through the shaper took
    // 38. So the call's quiet is heard with the shaper on, and the bar
    // asks what the proof's own load adds to it.
    let stop = Arc::new(AtomicBool::new(false));
    // A stop heard as the window opened starts nothing.
    let mut touched = false;
    if idle.is_empty() && !halted(&stop) {
        touched = true;
        systemctl("start");
        // The shaper settles before its quiet counts.
        settle(&stop);
        if !halted(&stop) {
            tell("idle: the call's round trips through the shaper, with the proof loading nothing");
            *idle = idle_call(asked.seconds.min(15), &stop);
        }
    }
    if halted(&stop) {
        return (Vec::new(), !touched || put_back(was_on));
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
    let pairs = run_pairs(asked, link, &stop, from, &mut touched);
    let closed = stop.swap(true, Ordering::Relaxed);
    stop_scopes();
    // Put back before the watch is joined, since it may be stuck asking
    // the gate; once stopped, it isn't waited for, ending with the proof.
    // One stopped before it touched the shaper leaves it as it found it,
    // since a put-back then could only undo a change made since.
    let back = !touched || put_back(was_on);
    if !STOPPED.load(Ordering::Relaxed) {
        let _ = watch.join();
    }
    if closed && !STOPPED.load(Ordering::Relaxed) {
        tell(&format!(
            "kept {} pairs, the one under way dropped; waiting for Joel's next idle",
            from as usize + pairs.len()
        ));
    }
    (pairs, back)
}

/// Measures the pairs, the shaper on for one block of each and off for the
/// other, until they're done or the gate closes, `touched` once an arm
/// has started or stopped it.
fn run_pairs(
    asked: &Asked,
    link: &str,
    stop: &AtomicBool,
    from: u32,
    touched: &mut bool,
) -> Vec<[Block; 2]> {
    let mut pairs: Vec<[Block; 2]> = Vec::new();
    'pairs: for pair in from..asked.pairs {
        tell(&format!("pair {} of {}", pair + 1, asked.pairs));
        let mut done: Vec<Block> = Vec::new();
        for arm in arms(pair) {
            if halted(stop) {
                break 'pairs;
            }
            *touched = true;
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
    use super::*;

    #[test]
    fn a_stop_ends_the_settle_at_once() {
        let began = Instant::now();
        settle(&AtomicBool::new(true));
        assert!(
            began.elapsed() < Duration::from_secs(2),
            "a stop waited {:?} for the settle",
            began.elapsed()
        );
        // And a stop that comes while it waits.
        let stopped = Arc::new(AtomicBool::new(false));
        let later = Arc::clone(&stopped);
        let raiser = thread::spawn(move || {
            thread::sleep(Duration::from_millis(300));
            later.store(true, Ordering::Relaxed);
        });
        let began = Instant::now();
        settle(&stopped);
        let _ = raiser.join();
        assert!(
            began.elapsed() < Duration::from_secs(2),
            "a stop on the way waited {:?}",
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
    fn the_shaper_goes_first_in_every_other_pair() {
        assert_eq!(arms(0), [Arm::On, Arm::Off]);
        assert_eq!(arms(1), [Arm::Off, Arm::On]);
    }

    #[test]
    fn a_scopes_shell_gets_its_own_dollars() {
        // systemd-run would otherwise read ${20} and ${s##*) } itself, as
        // nothing, and every watcher would take a live proof for gone.
        let c = scoped_as("app.slice", "u", &["sh", "-c", "${20}"]);
        let args: Vec<_> = c.get_args().map(|a| a.to_string_lossy()).collect();
        let ends = args.iter().position(|a| a == "--").expect("its command");
        assert!(
            args[..ends].iter().any(|a| a == "--expand-environment=no"),
            "systemd-run is left to expand the command: {args:?}"
        );
        assert_eq!(args[ends + 1..], ["sh", "-c", "${20}"]);
        // The sorter sends a scope with no ceiling, started from an
        // agent's work, back into that work, where Joel's probes would be
        // agents' traffic and the keeper would die with the agent's command.
        assert!(
            args[..ends]
                .iter()
                .any(|a| a == "--property=MemoryMax=128M"),
            "a scope an agent starts is taken back into its work: {args:?}"
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
        assert!(
            asked(&[], Some("-K/etc/passwd".into())).is_err(),
            "took a server curl would read as its option"
        );
    }
}
