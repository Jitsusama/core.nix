//! What the machine is busy with, as its `refuse_while` commands say: a
//! call, someone at the keys, a Mac past its room. Each command answers as
//! `test` does, 0 busy and 1 not, and anything else, or no answer within
//! 30 seconds, counts as busy, since it couldn't tell. An update asks at
//! three moments, each reading only the commands meant for it: as an ask
//! comes and before a switch (`now`), once the follower's ask has found
//! something to build, before it builds (`at_the_ask`), and while a built switch the
//! follower asked for waits for nobody at the machine (`quiet`).

use crate::config::{Busy, Config};
use crate::run;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// How long a busy command has to answer before it counts as busy.
const BUSY_WITHIN: Duration = Duration::from_secs(30);

/// How often a busy command is looked at while it runs.
const BUSY_EVERY: Duration = Duration::from_millis(100);

/// How long a busy command's last word is waited for once it has ended,
/// should something it started still hold its error stream.
const SAID_WITHIN: Duration = Duration::from_secs(1);

/// The most of a busy command's last word a refusal carries.
const SAID_AT_MOST: usize = 200;

/// How often a built switch the follower asked for asks again whether
/// what it waits on is over, and the longest it waits before it ends with
/// nothing switched, what it built kept in the store for the next ask.
const WAITS_EVERY: Duration = Duration::from_secs(1);
const WAITS_AT_MOST: Duration = Duration::from_hours(1);

/// How long after the last refusal a stretch of them ends, so the next
/// starts its count again: a follower asks again within a minute while
/// busy, and every half hour at most otherwise, so a longer gap means
/// another check refused first, or the machine slept.
const A_STRETCH_ENDS_AFTER: u64 = 30 * 60;

/// What the machine is busy with, when any of its busy commands says so,
/// as an ask comes, once it has built or before a waiting switch. Those a
/// follower waits on refuse nothing, and those that refuse the ask alone
/// are asked apart, refusing no switch.
pub fn now(config: &Config) -> Option<String> {
    config
        .refuse_while
        .iter()
        .filter(|busy| !busy.follower_waits && !busy.refuses_the_ask_alone)
        .find_map(is_busy)
}

/// What the machine is busy with once an ask the follower alone made has
/// found something to build, before it builds, when any of its commands that refuse the
/// ask alone says so and hasn't refused asks for longer than it may. A
/// person's ask builds anyway: such a check spares their work a build
/// they didn't ask for, and one they ask for is theirs to make, as one to
/// mend the check itself may be.
pub fn at_the_ask(config: &Config, following: &dyn Fn() -> bool) -> Option<String> {
    // Who asked is looked at only where it matters, since each look reads
    // whether a person has joined the ask since.
    let mut checks = config
        .refuse_while
        .iter()
        .filter(|busy| !busy.follower_waits && busy.refuses_the_ask_alone)
        .peekable();
    if checks.peek().is_none() || !following() {
        return None;
    }
    // A person who joined while the check ran builds too.
    checks
        .find_map(|busy| refused_for_at_most(config, busy, is_busy(busy)))
        .filter(|_| following())
}

/// `why` the ask is refused, unless `busy` has refused asks for longer
/// than it may, when this one goes ahead, says so, and the count starts
/// again. When the stretch of refusals began, and when it last refused,
/// are kept in the state folder, under a name drawn from what it's busy
/// with; one it can't keep lets the ask go ahead, since the most it may
/// refuse couldn't be held to.
fn refused_for_at_most(config: &Config, busy: &Busy, why: Option<String>) -> Option<String> {
    let Some(most) = busy.refuses_at_most else {
        return why;
    };
    let name: String = busy
        .why
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let since = config.state.join(format!("refused-since-{name}"));
    let Some(why) = why else {
        let _ = fs::remove_file(&since);
        return None;
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    // A time from the future, as a clock set back leaves, starts the
    // stretch again rather than refusing until the clock reaches it.
    let began = fs::read_to_string(&since)
        .ok()
        .and_then(|text| {
            let mut times = text.split_whitespace().map(str::parse::<u64>);
            Some((times.next()?.ok()?, times.next()?.ok()?))
        })
        .filter(|&(began, last)| began <= last && last <= now && now - last <= A_STRETCH_ENDS_AFTER)
        .map_or(now, |(began, _)| began);
    if now - began >= most {
        let _ = fs::remove_file(&since);
        eprintln!(
            "update-to-main: {why} for {} s of asks, so this one goes ahead",
            now - began
        );
        return None;
    }
    if let Err(e) = fs::write(&since, format!("{began} {now}\n")) {
        eprintln!(
            "update-to-main: {why}, but {} couldn't keep when that began ({e}), so this \
             ask goes ahead",
            since.display()
        );
        return None;
    }
    Some(why)
}

/// Waits, for a switch only the follower asked for, while any command a
/// follower waits on says someone is at the machine, each for as long as
/// it may hold the switch, and gives why it switches nothing once it has
/// waited an hour. On 9 November a launcher stopped taking keys after five
/// switches the follower asked for in half an hour, each restarting the
/// desktop's shell under Joel's hands; and that evening no switch came for
/// two hours while he worked, with 26 merges waiting, so someone at the
/// keys holds it for 45 minutes and an open launcher the whole hour. A
/// person asking meanwhile, or a stop, ends the wait at once, but for what
/// every ask waits on: on 12 November a switch Joel's orchestrator asked for
/// restarted the shell while he typed into it, and the line was lost.
pub fn quiet(config: &Config, following: &dyn Fn() -> bool) -> Option<String> {
    let began = Instant::now();
    let mut said = None;
    while !run::stopping() {
        let waited = began.elapsed();
        let follower = following();
        let Some(why) = config
            .refuse_while
            .iter()
            .filter(|busy| {
                busy.follower_waits
                    && (follower || busy.every_ask_waits)
                    && busy
                        .follower_waits_at_most
                        .is_none_or(|most| waited < Duration::from_secs(most))
            })
            .find_map(is_busy)
        else {
            if said.is_some() {
                eprintln!("the switch waited {} s, and goes", waited.as_secs());
            }
            return None;
        };
        if waited >= WAITS_AT_MOST {
            return Some(format!(
                "{why} for an hour, so what was built waits in the store for the next ask"
            ));
        }
        if said.as_ref() != Some(&why) {
            eprintln!("the switch waits while {why}");
            said = Some(why);
        }
        thread::sleep(WAITS_EVERY);
    }
    None
}

fn is_busy(busy: &Busy) -> Option<String> {
    let (program, args) = busy.command.split_first()?;
    let unsure = |why: String| Some(format!("it can't tell whether {}: {why}", busy.why));
    let mut child = match Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => return unsure(format!("{program}: {e}")),
    };
    let said = last_said(child.stderr.take());
    let deadline = Instant::now() + BUSY_WITHIN;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return match status.code() {
                    Some(0) => Some(busy.why.clone()),
                    Some(1) => None,
                    _ => unsure(format!(
                        "`{}` ended {status}{}",
                        busy.command.join(" "),
                        saying(&said)
                    )),
                };
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(BUSY_EVERY),
            Ok(None) => {
                // It's past answering; what it was doing no longer matters.
                let _ = child.kill();
                let _ = child.wait();
                return unsure(format!(
                    "`{}` didn't answer within {} s{}",
                    busy.command.join(" "),
                    BUSY_WITHIN.as_secs(),
                    saying(&said)
                ));
            }
            Err(e) => return unsure(e.to_string()),
        }
    }
}

/// The last line a busy command says on its error stream, read beside it
/// so a long one can't fill the pipe and stall it. A check that can't tell
/// says why there: on 14 November the meeting check refused a switch for
/// one of three reasons and the refusal named none, since the stream went
/// nowhere.
fn last_said(stream: Option<impl Read + Send + 'static>) -> Receiver<String> {
    let (send, said) = mpsc::channel();
    if let Some(stream) = stream {
        thread::spawn(move || {
            let last = BufReader::new(stream)
                .lines()
                .map_while(Result::ok)
                .filter(|line| !line.trim().is_empty())
                .last()
                .unwrap_or_default();
            let _ = send.send(last);
        });
    }
    said
}

/// A busy command's last word, as a refusal ends with it, or nothing when
/// it said none.
fn saying(said: &Receiver<String>) -> String {
    let last = said.recv_timeout(SAID_WITHIN).unwrap_or_default();
    let last = last.trim();
    if last.is_empty() {
        return String::new();
    }
    let cut: String = last.chars().take(SAID_AT_MOST).collect();
    format!(", saying {cut}")
}
