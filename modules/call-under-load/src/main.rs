//! Whether Joel's calls and apps stay quick while the machine is under
//! everything it carries, measured each hour.
//!
//! Joel asked to run insanely high quality video and audio while the
//! machine is under all its pressures, with Chrome and Slack staying
//! responsive. A check that runs only when someone remembers finds a slide
//! a day late, so each hour `call-under-load run` makes a test call shaped
//! as Meet's between two Chromiums on this machine, on Chromium's fake
//! camera and microphone, never a real one, and clicks the call's tab
//! every five seconds. It writes what
//! the call did, with the hour's Chrome and Slack drawing waits from
//! lag-watch's history and the launcher's openings from the journal, to
//! `$XDG_STATE_HOME/apps-under-load/hour.json`, which cadence judges. The
//! run beside them sets the launcher's openings while the test ran against
//! the same span just before, and says when the test made them worse, or
//! that too few openings were seen to tell.
//!
//! It keeps out of Joel's calls: it asks `a-call-is-on --here`, which hears
//! a call on this machine by its microphone, before it starts and every five seconds after,
//! and the calendar, keeping ten minutes clear of any meeting there. A run
//! stopped or skipped writes nothing.
//!
//! `call-under-load watch` watches Joel's own calls as they happen, and
//! each hour's figures carry the worst of those since the hour before
//! (src/watch.rs says how).
//!
//! `call-under-load camera DEVICE` checks before each of Joel's meetings
//! that his camera gives a picture, and says when it doesn't (src/camera.rs
//! says how).
//!
//! `call-under-load`, or `call-under-load now`, says the last hour's
//! figures, in the form `--format` asks for (docs/commands.md).

mod call;
mod camera;
mod cdp;
mod figures;
mod gate;
mod history;
mod hour;
mod launcher;
mod managed;
mod watch;

use serde_json::{Value, json};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};
use terminal_kit::{Context, Exit, Failure, Format, Help, flags};

/// How long the call runs: 140 samples past its first ten seconds, read
/// as about 135 overlapping five-second windows, and short enough to
/// leave the hour to Joel.
const SECONDS: u64 = 150;

/// The longest a run may take, its unit's start timeout: a meeting must be
/// ten minutes clear of all of it, since starting Chrome under load can
/// take minutes before the call's own 150 s.
const LONGEST: u64 = 10 * 60;

fn env_dir(var: &str, fallback: &str) -> PathBuf {
    std::env::var_os(var).map_or_else(
        || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(fallback),
        PathBuf::from,
    )
}

fn state() -> PathBuf {
    env_dir("XDG_STATE_HOME", ".local/state")
}

fn folder() -> PathBuf {
    state().join("apps-under-load")
}

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new("call-under-load", "calls and apps under load, each hour")
        .example("call-under-load", "the last hour's figures")
        .example("call-under-load --json", "the same, for a script")
        .subcommand(
            Help::new("now", "the last hour's figures, as a bare call-under-load")
                .example("call-under-load now", "the last hour's figures")
                .exit(Exit::NotFound, "no hour has been measured yet")
                .exit(Exit::BadInput, "the hour can't be read"),
        )
        .subcommand(
            Help::new("run", "the hour's test call, as its timer runs it")
                .example(
                    "call-under-load run",
                    "measures the hour, unless Joel is in a call",
                )
                .exit(Exit::Done, "measured, or skipped for a call or a meeting")
                .exit(
                    Exit::Unavailable,
                    "Chromium, the calendar or the call check isn't there",
                )
                .exit(
                    Exit::Refused,
                    "a test browser enrolled, or Chromium's policy could enrol one",
                )
                .exit(Exit::Failed, "the call couldn't be made"),
        )
        .subcommand(
            Help::new("watch", "Joel's own calls, as its service watches them")
                .example(
                    "call-under-load watch",
                    "adds each call's sending waits to the history as it ends",
                )
                .exit(Exit::Unavailable, "the apps' slice can't be found")
                .exit(Exit::Refused, "another watch is running")
                .exit(Exit::Failed, "a call's line couldn't be written"),
        )
        .subcommand(
            Help::new(
                "camera",
                "the camera before a meeting, as its timer checks it",
            )
            .argument(
                "device",
                "the machine's camera, or a file naming it; one plugged in comes first",
            )
            .example(
                "call-under-load camera /run/camera/device",
                "says when a meeting is near and the camera gives no picture",
            )
            .exit(Exit::Done, "checked, or nothing due")
            .exit(Exit::Unavailable, "the calendar can't be read")
            .exit(Exit::Failed, "the notice or the check couldn't be written"),
        )
        .exit(Exit::NotFound, "no hour has been measured yet")
        .exit(Exit::BadInput, "the hour can't be read")
}

fn main() -> ExitCode {
    let help = help();
    let commands = help.command_names();
    terminal_kit::main(&help, env!("CARGO_PKG_VERSION"), |context| {
        let (command, rest) = match context.args.split_first() {
            Some((command, rest)) => (command.as_str(), rest),
            None => ("now", &[][..]),
        };
        match command {
            "now" => none_after(rest).and_then(|()| now(context)),
            "run" => none_after(rest).and_then(|()| run(context)),
            "watch" => none_after(rest).and_then(|()| watch()),
            "camera" => {
                let mut words = rest
                    .iter()
                    .filter(|arg| !matches!(arg.as_str(), "--verbose" | "-v"));
                let device = words.next();
                match words.next() {
                    Some(arg) => none_after(std::slice::from_ref(arg)),
                    None => camera_check(context, device.map(String::as_str)),
                }
            }
            word => Err(flags::unknown("command", word, &commands)),
        }
    })
}

/// Nothing more on the line; `--verbose` is the kit's.
fn none_after(rest: &[String]) -> Result<(), Failure> {
    match rest
        .iter()
        .find(|arg| !matches!(arg.as_str(), "--verbose" | "-v"))
    {
        Some(arg) => Err(Failure::new(Exit::Usage, format!("{arg} isn't taken here"))
            .because("now, run and watch take nothing more, and camera only its device")
            .try_this("call-under-load --help")),
        None => Ok(()),
    }
}

/// Checks the camera before a meeting near enough, once each, when no call
/// is on: a call on has its camera, and the check would only load it.
fn camera_check(context: &Context, device: Option<&str>) -> Result<(), Failure> {
    let said = |what: String| {
        context
            .say(&format!("{what}\n"))
            .map_err(|error| unwritten(&error))
    };
    let runtime = env_dir("XDG_RUNTIME_DIR", ".cache");
    let calendar = gate::ask(&runtime.join("daemons/google.sock"))
        .and_then(|answer| gate::calendar(&answer))
        .map_err(|why| {
            Failure::new(Exit::Unavailable, "the calendar can't be read")
                .because(why)
                .try_this("systemctl --user status google")
        })?;
    let now = seconds_now() * 1000;
    let file = folder().join("camera-checked.json");
    let starts = camera::checked(&file, now);
    let Some(meeting) = camera::due(&calendar, now, &starts) else {
        return Ok(());
    };
    if gate::a_call() != gate::Call::Off {
        return said(format!(
            "the camera waits: a call may be on before {}",
            meeting.title
        ));
    }
    let Some((node, plugged)) = camera::which(Path::new("/"), device) else {
        return said("no camera to check".to_string());
    };
    let gave = camera::gives(&node, plugged);
    camera::remember(&file, starts, meeting.start).map_err(|error| {
        Failure::new(Exit::Failed, "the check couldn't be kept")
            .because(format!("{}: {error}", file.display()))
    })?;
    let Err(why) = gave else {
        return said(format!(
            "{} gives a picture before {}",
            node.display(),
            meeting.title
        ));
    };
    let (summary, body) = camera::told(meeting, seconds_now() * 1000, plugged, &why);
    said(format!("{}: {body}", node.display()))?;
    camera::within(
        "notify-send",
        &[
            "--app-name=Camera",
            "--urgency=critical",
            summary.as_str(),
            body.as_str(),
        ],
        std::time::Duration::from_secs(10),
    )
    .map_err(|why| {
        Failure::new(Exit::Failed, "the notice couldn't be sent")
            .because(why)
            .try_this("systemctl --user status notices")
    })
}

fn unwritten(error: &std::io::Error) -> Failure {
    Failure::new(Exit::Failed, "couldn't write the answer").because(error.to_string())
}

fn seconds_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The last hour's figures.
fn now(context: &Context) -> Result<(), Failure> {
    let path = folder().join("hour.json");
    let text = fs::read_to_string(&path).map_err(|error| {
        Failure::new(Exit::NotFound, "no hour has been measured yet")
            .because(format!("{} can't be read: {error}", path.display()))
            .try_this("systemctl --user start call-under-load")
    })?;
    let hour: Value = serde_json::from_str(&text).map_err(|error| {
        Failure::new(Exit::BadInput, "the hour can't be read")
            .because(format!("{}: {error}", path.display()))
            .try_this("systemctl --user start call-under-load")
    })?;
    let age = seconds_now().saturating_sub(hour["at"].as_u64().unwrap_or(0));
    match context.format() {
        Format::Text => context.say(&said(&hour, age)),
        _ => context.answer(&json!({
            "schema": "call-under-load.hour/1",
            "at": hour["at"],
            "age_s": age,
            "measures": hour["measures"],
            "run": hour["run"],
        })),
    }
    .map_err(|error| unwritten(&error))
}

/// The hour as lines for a person: each figure against its target.
fn said(hour: &Value, age: u64) -> String {
    let mut text = format!("measured {} min ago\n", age / 60);
    for m in hour["measures"].as_array().into_iter().flatten() {
        let now = m["now"]
            .as_f64()
            .map_or_else(|| "unread".to_string(), |x| format!("{x}"));
        let target = m["target"].as_f64().unwrap_or(0.0);
        let way = if m["lower_is_better"] == true {
            "at most"
        } else {
            "at least"
        };
        // Writing to a String can't fail.
        let _ = writeln!(
            text,
            "{}: {now}, {way} {target}",
            m["measure"].as_str().unwrap_or("?")
        );
    }
    if let Some(line) = beside_said(&hour["run"]["launcher"]) {
        text.push_str(&line);
    }
    text
}

/// The launcher's openings during the test against just before it, as a
/// line, or none for an hour written before the run carried them.
fn beside_said(launcher: &Value) -> Option<String> {
    if let Some(why) = launcher["unread"].as_str() {
        return Some(format!("launcher during the test: unread, {why}\n"));
    }
    let span = |side: &str| {
        let opens = launcher[side]["opens"].as_u64()?;
        let ms = |key: &str, what: &str| {
            launcher[side][key]
                .as_f64()
                .map_or_else(String::new, |ms| format!(", {what} {ms} ms"))
        };
        let plural = if opens == 1 { "" } else { "s" };
        Some(format!(
            "{opens} opening{plural}{}{}",
            ms("p50", "median"),
            ms("p99", "slowest")
        ))
    };
    let verdict = match launcher["worse"].as_bool() {
        Some(true) => "worse".to_string(),
        Some(false) => "not worse".to_string(),
        None => format!("can't tell under {} openings a side", launcher::FEWEST),
    };
    Some(format!(
        "launcher during the test: {}; just before: {}; {verdict}\n",
        span("during")?,
        span("before")?
    ))
}

/// Refuses the call where a test browser could join anyone's management
/// (`managed`): once one has enrolled, until someone has read what it was
/// given, or where Chromium's own machine policy could enrol it.
fn unmanaged() -> Result<(), Failure> {
    if let Some(why) = managed::stays_off(&state()) {
        let (look, clear) = managed::look_at(&why, &state());
        return Err(Failure::new(Exit::Refused, "the test call stays off")
            .because(format!("{}. {clear}", why.trim_end()))
            .try_this(look));
    }
    if let Some(why) = managed::could_enrol(Path::new(managed::POLICIES)) {
        return Err(
            Failure::new(Exit::Refused, "the test call could enrol its browsers")
                .because(why)
                .try_this(format!("ls -R {}", managed::POLICIES)),
        );
    }
    Ok(())
}

/// An enrolment this run found: refused, as the next hour would be, in
/// the run's own words, which say where the evidence is, kept or still in
/// its profile.
fn enrolled(why: &str) -> Failure {
    let (look, clear) = managed::look_at(why, &state());
    Failure::new(Exit::Refused, "a test browser enrolled")
        .because(format!("{why}. {clear}"))
        .try_this(look)
}

/// Where the call's profiles are made.
fn cache() -> PathBuf {
    env_dir("XDG_CACHE_HOME", ".cache").join("call-under-load")
}

/// The hour's test call, and the figures written for cadence.
fn run(context: &Context) -> Result<(), Failure> {
    unmanaged()?;
    let runtime = env_dir("XDG_RUNTIME_DIR", ".cache");
    let cache = cache();
    // What a killed run left is looked in before the gates, so an hour
    // skipped for a meeting still finds an enrolment it holds.
    call::swept(&runtime, &cache, &state()).map_err(|why| enrolled(&why))?;
    let skip = |why: String| {
        context
            .say(&format!("no test call this hour: {why}\n"))
            .map_err(|error| unwritten(&error))
    };
    // Asked up to three times, as the watch does during the call, since
    // one slow answer under the heaviest load would drop just that hour.
    let mut asked = gate::a_call();
    for _ in 1..call::UNSURE {
        if !matches!(asked, gate::Call::Unknown(_)) {
            break;
        }
        std::thread::sleep(call::TICK);
        asked = gate::a_call();
    }
    match asked {
        gate::Call::Off => {}
        gate::Call::On => return skip("Joel is in a call".into()),
        gate::Call::Unknown(why) => {
            return Err(
                Failure::new(Exit::Unavailable, "whether Joel is in a call can't be told")
                    .because(why)
                    .try_this("a-call-is-on --here"),
            );
        }
    }
    let socket = runtime.join("daemons/google.sock");
    let calendar = gate::ask(&socket).and_then(|answer| gate::calendar(&answer));
    let calendar = calendar.map_err(|why| {
        Failure::new(Exit::Unavailable, "the calendar can't be read")
            .because(why)
            .try_this("systemctl --user status google")
    })?;
    let started = seconds_now();
    if let Some(why) = gate::kept_clear(&calendar, started * 1000, LONGEST * 1000) {
        return skip(why);
    }
    // Chromium, never Google Chrome, which a managed machine enrols with
    // its owner's management at every fresh profile (package.nix).
    let chrome = on_path("chromium").ok_or_else(|| {
        Failure::new(Exit::Unavailable, "there's no Chromium to call with")
            .because("chromium isn't on the program's PATH")
            .try_this("nix build .#call-under-load")
    })?;
    let page = std::env::current_exe()
        .ok()
        .and_then(|exe| page_beside(&exe))
        .filter(|page| page.exists())
        .ok_or_else(|| {
            Failure::new(Exit::Failed, "the call's page isn't installed")
                .because("share/call-under-load/page.html isn't beside the program")
                .try_this("nix build .#call-under-load")
        })?;
    let ended = call::run(&chrome, &page, SECONDS, &runtime, &cache, &state()).map_err(
        |fault| match fault {
            call::Fault::Enrolled(why) => enrolled(&why),
            call::Fault::Failed(why) => {
                Failure::new(Exit::Failed, "the test call couldn't be made")
                    .because(why)
                    .try_this("journalctl --user -u call-under-load")
            }
        },
    )?;
    let (mine, far, clicks) = match ended {
        call::Ended::Stopped(why) => return skip(why),
        call::Ended::Ran { mine, far, clicks } => (mine, far, clicks),
    };
    let call = figures::call(&mine, &far, &clicks);
    let at = seconds_now();
    let watched = state().join("lag-watch");
    let lines = std::fs::read_to_string(folder().join("history.jsonl")).unwrap_or_default();
    let ran = history::ran(&lines);
    let chrome_waits = history::waits(&watched, "chrome-ui", started, LONGEST, &ran);
    let slack_waits = history::waits(&watched, "slack-ui", started, LONGEST, &ran);
    let read = launcher::read();
    let took: Vec<f64> = read.opens.iter().map(|(_, ms)| *ms).collect();
    let mut measures = hour::measures(
        &call,
        call::STAND_IN,
        &chrome_waits,
        &slack_waits,
        &took,
        read.unread.as_deref(),
    );
    measures.extend(real_calls(&lines, &ran, at));
    // The call's span runs from the run's start, Chromium's own included,
    // to now: the whole of the load the test put beside Joel's programs.
    let launcher = launcher::written(&read, started, at);
    let run = json!({"started": started, "seconds": SECONDS, "call": call, "launcher": launcher});
    hour::write(&folder(), at, &measures, &run).map_err(|error| {
        Failure::new(Exit::Failed, "the hour couldn't be written")
            .because(error.to_string())
            .try_this(format!("ls -ld {}", folder().display()))
    })?;
    context
        .say(&said(&json!({"measures": measures, "run": run}), 0))
        .map_err(|error| unwritten(&error))
}

/// Joel's own calls' figures: those since the last hour written, so calls
/// in hours a meeting or a call skipped still reach cadence, and when the
/// watch last could tell.
fn real_calls(lines: &str, ran: &[(u64, u64)], at: u64) -> Vec<hour::Measure> {
    let since = ran.iter().map(|&(_, ended)| ended).max().unwrap_or(0);
    let clear = watch::clear_at(&folder().join("watch.json"));
    hour::real(&watch::heard(lines, since, at), since, clear, at)
}

/// Joel's own calls, watched until the service stops.
fn watch() -> Result<(), Failure> {
    let cgroup = fs::read_to_string("/proc/self/cgroup").unwrap_or_default();
    let account = account(&cgroup).ok_or_else(|| {
        Failure::new(Exit::Unavailable, "the apps' slice can't be found")
            .because(format!(
                "it runs outside a user's manager: {}",
                cgroup.trim()
            ))
            .try_this("systemctl --user status call-under-load-watch")
    })?;
    // Without XDG_RUNTIME_DIR, the account's own runtime folder, so an
    // agent's systemd-run unit is still told from his.
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").map_or_else(
        || PathBuf::from(format!("/run/user/{}", uid(&account).unwrap_or_default())),
        PathBuf::from,
    );
    let seat = watch::Seat {
        proc: PathBuf::from("/proc"),
        slice: PathBuf::from(format!("/sys/fs/cgroup{account}/app.slice")),
        account,
        transient: runtime.join("systemd/transient"),
    };
    let history = folder().join("history.jsonl");
    let unwritable = |error: std::io::Error| {
        Failure::new(Exit::Failed, "a call's line couldn't be written")
            .because(error.to_string())
            .try_this(format!("ls -ld {}", folder().display()))
    };
    fs::create_dir_all(folder()).map_err(unwritable)?;
    let Some(_held) = watch::alone(&folder().join("watch.lock")).map_err(unwritable)? else {
        return Err(Failure::new(Exit::Refused, "another watch is running")
            .because("two would write each of Joel's calls twice")
            .try_this("systemctl --user status call-under-load-watch"));
    };
    watch::watch(&seat, &history, &folder().join("watch.json")).map_err(|error| {
        Failure::new(Exit::Failed, "a call's line couldn't be written")
            .because(error.to_string())
            .try_this(format!("ls -ld {}", folder().display()))
    })
}

/// The path in the cgroup tree of the user's manager this runs under,
/// from its own line in `/proc/self/cgroup`.
fn account(cgroup: &str) -> Option<String> {
    let path = cgroup.lines().find_map(|line| line.strip_prefix("0::"))?;
    let (manager, _) = path.split_once(".service/")?;
    manager
        .contains("/user@")
        .then(|| format!("{manager}.service"))
}

/// The account's uid, from its manager's path, `.../user@<uid>.service`.
fn uid(account: &str) -> Option<u32> {
    account
        .rsplit_once("/user@")?
        .1
        .strip_suffix(".service")?
        .parse()
        .ok()
}

/// The first program named `name` on PATH.
fn on_path(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(name))
        .find(|path| path.is_file())
}

/// Where the call's page is installed beside the program at `exe`.
fn page_beside(exe: &Path) -> Option<PathBuf> {
    Some(
        exe.parent()?
            .parent()?
            .join("share/call-under-load/page.html"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hour_is_said_figure_by_figure_against_its_target() {
        let hour = json!({"measures": [
            {"measure": "call sent fps p10", "now": 29.8, "target": 30.0, "lower_is_better": false},
            {"measure": "launcher open ms p99", "now": null, "target": 50.0,
             "lower_is_better": true},
        ]});
        assert_eq!(
            said(&hour, 125),
            "measured 2 min ago\ncall sent fps p10: 29.8, at least 30\n\
             launcher open ms p99: unread, at most 50\n"
        );
    }

    #[test]
    fn the_launcher_s_spans_are_said_with_whether_they_tell() {
        let mut opens: Vec<(f64, f64)> = [910.0, 930.0, 950.0].map(|at| (at, 30.0)).to_vec();
        opens.extend([1010.0, 1030.0, 1050.0, 1070.0].map(|at| (at, 46.0)));
        let hour = json!({"measures": [],
                          "run": {"launcher": launcher::beside(&opens, 1000, 1100)}});
        assert_eq!(
            said(&hour, 0),
            "measured 0 min ago\nlauncher during the test: 4 openings, median 46 ms, \
             slowest 46 ms; just before: 3 openings, median 30 ms, slowest 30 ms; worse\n",
            "the run's own spans are said under the figures"
        );
        let fine = json!({"before": {"opens": 3, "p50": 30.0, "p99": 30.0},
                          "during": {"opens": 3, "p50": 31.0, "p99": 31.0}, "worse": false});
        assert!(
            beside_said(&fine).is_some_and(|line| line.ends_with("; not worse\n")),
            "a call that didn't slow the openings says so: {:?}",
            beside_said(&fine)
        );
        let thin = json!(launcher::beside(&[(950.0, 30.0)], 1000, 1100));
        assert_eq!(
            beside_said(&thin).as_deref(),
            Some(
                "launcher during the test: 0 openings; just before: 1 opening, median 30 ms, \
                 slowest 30 ms; can't tell under 3 openings a side\n"
            )
        );
        let unread = "journalctl ended with code 1: No journal files were found.";
        assert_eq!(
            beside_said(&json!({"unread": unread})).as_deref(),
            Some(
                "launcher during the test: unread, journalctl ended with code 1: \
                 No journal files were found.\n"
            )
        );
        assert_eq!(
            beside_said(&Value::Null),
            None,
            "an older hour says nothing"
        );
    }

    #[test]
    fn the_watch_finds_the_manager_it_runs_under() {
        assert_eq!(
            account(
                "0::/user.slice/user-1000.slice/user@1000.service/app.slice/\
                 call-under-load-watch.service\n"
            )
            .as_deref(),
            Some("/user.slice/user-1000.slice/user@1000.service")
        );
        assert_eq!(
            uid("/user.slice/user-1000.slice/user@1000.service"),
            Some(1000),
            "the account's runtime folder couldn't be found without XDG_RUNTIME_DIR"
        );
        assert_eq!(
            account("0::/system.slice/sshd.service\n"),
            None,
            "a system service took a slice no user's apps are in"
        );
    }

    #[test]
    fn the_page_is_looked_for_in_the_package_s_share() {
        assert_eq!(
            page_beside(Path::new("/nix/store/x/bin/call-under-load")),
            Some(PathBuf::from(
                "/nix/store/x/share/call-under-load/page.html"
            ))
        );
    }
}
