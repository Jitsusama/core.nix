//! Joel's own command for the safe word: which stops stand, and resuming
//! them.
//!
//! `pineapple`, or `pineapple stops`, says each stop standing: its id,
//! whether it is still in shadow, what it reaches, who said it and where.
//! `pineapple resume <scope>` resumes the stops a scope names: `all`, a
//! stop's id, or one of its reaches as `stops` shows it, such as
//! `pr:owner/name#12` or `quest:ID`. Each record is marked resumed, never
//! removed, and the harness stops acting on it within a minute.
//!
//! Only Joel resumes, so `resume` refuses to run under pi: in an agent's
//! shell, under the host, or from a window's own `!`. Joel runs it at a
//! terminal of his own; the launcher's Resume asks `pineapple serve`,
//! which takes it only from his shell (src/serve.rs).
//!
//! `pineapple stop all` is Joel's own stop on everything, for a time no
//! agent may run, as through a demo, and is refused under pi too. Beside
//! the stop's record it pauses what the machine's pause plan names
//! (src/pause.rs), here and on the machines named, says so in one line,
//! then names each session as it parks, for a minute, and any still
//! mid-turn after it. Resuming it, by `pineapple resume all` or the
//! launcher's Resume, undoes what it paused.
//!
//! `pineapple serve` is that daemon, as its service runs it: it serves
//! the stops, alerts and resume requests as Needs You's rows on its
//! socket, and knocks as each first appears. The harness
//! also refuses an agent's call to it before it starts, saying why; this
//! is the refusal a model can't argue with, since it is the program's.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use pineapple::pause;
use pineapple::resume::{self, Standing};
use pineapple::serve::{self as serving, Serving};
use pineapple::vocabulary::Words;
use serde_json::{Value, json};
use terminal_kit::{Exit, Failure, Format, Help, flags};

/// How many parents up a process looks for pi, far past any real tree.
const DEEPEST: usize = 64;

/// How long each step of a stop is given, so one machine that doesn't
/// answer can't hold the rest up.
const STEP: Duration = Duration::from_secs(6);

/// How long a stop watches its sessions park.
const PARKING: Duration = Duration::from_secs(60);

/// How often it looks for one parked.
const LOOKED: Duration = Duration::from_millis(500);

/// The variables pi sets in every shell it starts.
const PI_SAYS: [&str; 3] = ["PI_SESSION_ID", "PI_BASH_CALL", "PI_HOST_CONVERSATION"];

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "pineapple",
        "lists the safe word's stops, and resumes them, for you alone, never an agent",
    )
    .example("pineapple", "the stops standing, the same as stops")
    .example("pineapple resume all", "resumes every stop")
    .example(
        "pineapple stop all",
        "stops every agent here and on the machines named",
    )
    .subcommand(
        Help::new("stops", "the stops standing, with what each reaches")
            .example("pineapple stops --json", "the same as pineapple.stops/1"),
    )
    .subcommand(
        Help::new(
            "resume",
            "resumes the stops a scope names; refused under pi",
        )
        .argument(
            "<scope>",
            "all, a stop's id, or a reach such as repo:owner/name",
        )
        .example(
            "pineapple resume pr:example/shop#12",
            "resumes the stop on that pull request",
        ),
    )
    .subcommand(
        Help::new(
            "stop",
            "stops every agent, as before a demo, until you resume; refused under pi",
        )
        .argument(
            "all",
            "every agent, here and on the machines the pause plan names",
        )
        .example(
            "pineapple stop all",
            "stops them, naming each session as it parks",
        ),
    )
    .subcommand(
        Help::new(
            "serve",
            "the daemon showing stops in the launcher, as its service runs it",
        )
        .option(
            "--socket PATH",
            "where it listens, unless systemd hands it one",
        )
        .option(
            "--vocabulary PATH",
            "the launcher's words.json, for its rows' words",
        )
        .option(
            "--notify-send PATH",
            "the program it knocks with; none, no notice",
        )
        .option(
            "--busctl PATH",
            "closes a notice once its stop is resumed or left",
        )
        .option(
            "--every MS",
            "how often it looks for a change; 2 s unless given",
        )
        .example(
            "pineapple serve --socket /run/user/1000/daemons/pineapple.sock",
            "serves until stopped",
        ),
    )
    .exit(
        Exit::Refused,
        "run under pi, where only Joel may stop all or resume",
    )
    .exit(Exit::Config, "stop all finds no pause plan on this machine")
    .exit(Exit::NotFound, "no stop standing is named by the scope")
    .exit(Exit::BadInput, "serve's vocabulary can't be read")
}

fn main() -> ExitCode {
    let help = help();
    let commands = help.command_names();
    terminal_kit::main(&help, env!("CARGO_PKG_VERSION"), |context| {
        let args: Vec<&str> = context
            .args
            .iter()
            .map(String::as_str)
            .filter(|arg| !matches!(*arg, "--verbose" | "-v"))
            .collect();
        match args.as_slice() {
            [] | ["stops"] => stops(context),
            ["resume", scope] => resume(context, scope),
            ["stop", "all"] => stop_all(context),
            ["stop", ..] => Err(Failure::new(Exit::Usage, "stop takes all")
                .because("Joel's own stop is on everything")
                .try_this("pineapple stop all")),
            ["serve", rest @ ..] => serve(rest),
            ["resume"] => Err(Failure::new(Exit::Usage, "resume takes a scope")
                .because("it resumes only the stops the scope names")
                .try_this("pineapple stops")),
            ["stops" | "resume", extra, ..] => Err(Failure::new(
                Exit::Usage,
                format!("{extra} isn't taken here"),
            )
            .try_this("pineapple --help")),
            [word, ..] => Err(flags::unknown("command", word, &commands)),
        }
    })
}

fn state() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map_or_else(
            || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state"),
            PathBuf::from,
        )
        .join("pineapple")
}

fn folder() -> PathBuf {
    state().join("stops")
}

/// The machine's pause plan, which its config writes, or why there's none.
fn plan() -> Result<pause::Plan, Failure> {
    let path = std::env::var_os("PINEAPPLE_PAUSE").map_or_else(
        || {
            std::env::var_os("XDG_CONFIG_HOME")
                .map_or_else(
                    || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config"),
                    PathBuf::from,
                )
                .join("pineapple/pause.json")
        },
        PathBuf::from,
    );
    std::fs::read_to_string(&path)
        .map_err(|error| error.to_string())
        .and_then(|text| pause::Plan::from_json(&text))
        .map(|mut plan| {
            // A mycelium the config names but the machine lacks is none,
            // so a stop says plainly that it reached no other machine.
            plan.mycelium = plan.mycelium.filter(|path| path.exists());
            plan
        })
        .map_err(|why| {
            Failure::new(Exit::Config, "this machine has no pause plan")
                .because(format!("{}: {why}", path.to_string_lossy()))
        })
}

/// This machine's name, as its kernel says it.
fn host() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .map(|name| name.trim().to_owned())
        .ok()
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "this machine".to_owned())
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

/// Joel's stop on everything: its record and what it pauses, said in one
/// line, then each session named as it parks, for a minute at most.
fn stop_all(context: &terminal_kit::Context) -> Result<(), Failure> {
    if let Some(why) = under_pi() {
        return Err(Failure::new(Exit::Refused, "only Joel stops every agent")
            .because(why)
            .try_this("run it at a terminal of your own"));
    }
    if let Some(id) = pause::stands(&state()) {
        return context
            .say(&format!(
                "agents are already stopped, by {id}; pineapple resume all ends it\n"
            ))
            .map_err(|error| couldnt_write(&error));
    }
    let plan = plan()?;
    let at = now();
    let id = pause::id(at);
    let mut runs = pause::System { limit: STEP };
    let took = pause::stop(&plan, &state(), &id, &host(), at, &mut runs).map_err(|error| {
        Failure::new(Exit::Failed, "couldn't write the stop").because(error.to_string())
    })?;
    let sessions = pause::sessions(&plan, &pause::recorded(&took), &mut runs);
    context
        .say(&format!("{}\n", pause::said("stopping", &took, sessions)))
        .map_err(|error| couldnt_write(&error))?;
    // Only this machine's parking is written here; another's parks there.
    let host = host();
    let here = pause::sessions(&plan, &[host.as_str()], &mut runs);
    let parked = watch_parking(context, &id, here);
    let left = here.map_or(0, |count| count.saturating_sub(parked));
    let there = if took.iter().any(|one| one.recorded && one.machine != host) {
        ", the other machines' park there"
    } else {
        ""
    };
    let last = if left == 0 {
        format!("{parked} parked here{there}; nothing starts until pineapple resume all\n")
    } else {
        format!(
            "{parked} parked here, {left} still mid-turn after a minute, its writes refused\
             {there}; nothing starts until pineapple resume all\n"
        )
    };
    context.say(&last).map_err(|error| couldnt_write(&error))
}

/// Names each session as the harness writes it parked for the stop `id`,
/// until every one of `sessions` is or a minute has passed, and answers
/// how many parked.
fn watch_parking(context: &terminal_kit::Context, id: &str, sessions: Option<usize>) -> usize {
    let began = std::time::Instant::now();
    let mut named: Vec<String> = Vec::new();
    loop {
        let parked = std::fs::read_to_string(state().join("parked.jsonl")).unwrap_or_default();
        for line in parked.lines() {
            let Ok(line) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            let Some(session) = line["session"].as_str() else {
                continue;
            };
            if line["stop"] == id && !named.iter().any(|one| one == session) {
                named.push(session.to_owned());
                // A session's name is whatever it was given, so what would
                // move Joel's terminal is left out of it.
                let shown: String = session.chars().filter(|c| !c.is_control()).collect();
                // A line not written to a closed terminal is no reason to
                // stop watching; the stop stands either way.
                let _ = context.say(&format!("  parked {shown}\n"));
            }
        }
        if sessions.is_some_and(|count| named.len() >= count) || began.elapsed() >= PARKING {
            return named.len();
        }
        std::thread::sleep(LOOKED);
    }
}

/// The daemon, from its flags, which serves until it's stopped.
fn serve(rest: &[&str]) -> Result<(), Failure> {
    const FLAGS: [&str; 5] = [
        "--socket",
        "--vocabulary",
        "--notify-send",
        "--busctl",
        "--every",
    ];
    let mut socket = None;
    let mut vocabulary = None;
    let mut notify = None;
    let mut busctl = None;
    let mut every = serving::EVERY;
    let mut args = rest.iter();
    while let Some(flag) = args.next() {
        let needs = || {
            Failure::new(Exit::Usage, format!("{flag} takes a value"))
                .try_this("pineapple serve --help")
        };
        match *flag {
            "--socket" => socket = Some(PathBuf::from(args.next().ok_or_else(needs)?)),
            "--vocabulary" => vocabulary = Some(PathBuf::from(args.next().ok_or_else(needs)?)),
            "--notify-send" => notify = Some(PathBuf::from(args.next().ok_or_else(needs)?)),
            "--busctl" => busctl = Some(PathBuf::from(args.next().ok_or_else(needs)?)),
            "--every" => {
                every = args
                    .next()
                    .and_then(|given| given.parse().ok())
                    .map(Duration::from_millis)
                    .ok_or_else(needs)?;
            }
            other => return Err(flags::unknown("option", other, &FLAGS)),
        }
    }
    let socket = socket
        .or_else(|| {
            std::env::var_os("XDG_RUNTIME_DIR")
                .map(|runtime| PathBuf::from(runtime).join("daemons/pineapple.sock"))
        })
        .ok_or_else(|| {
            Failure::new(
                Exit::Usage,
                "no --socket and no XDG_RUNTIME_DIR to listen in",
            )
        })?;
    let words = match vocabulary {
        None => Words::default(),
        Some(path) => std::fs::read_to_string(&path)
            .map_err(|error| error.to_string())
            .and_then(|text| Words::from_vocabulary(&text).map_err(|error| error.to_string()))
            .map_err(|why| {
                Failure::new(Exit::BadInput, "couldn't read the vocabulary")
                    .because(format!("{}: {why}", path.display()))
            })?,
    };
    serving::serve(Serving {
        socket,
        state: state(),
        words,
        notify,
        busctl,
        every,
        // A machine with no plan serves on, refusing only Pause All Agents.
        pause: plan().ok(),
        host: host(),
    })
    .map_err(|error| Failure::new(Exit::Failed, "couldn't serve").because(error.to_string()))
}

fn stops(context: &terminal_kit::Context) -> Result<(), Failure> {
    let standing = resume::standing(&folder());
    let written = match context.format() {
        Format::Text if standing.is_empty() => context.say("no stop stands\n"),
        Format::Text => context.say(&listed(&standing)),
        _ => context.answer(&json!({
            "schema": "pineapple.stops/1",
            "stops": standing.iter().map(|stop| &stop.record).collect::<Vec<_>>(),
        })),
    };
    written.map_err(|error| couldnt_write(&error))
}

fn listed(standing: &[Standing]) -> String {
    let field = |record: &Value, key: &str| {
        record
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let mut out = String::new();
    for stop in standing {
        let shadow = stop.record.get("shadow").and_then(Value::as_bool) != Some(false);
        out.push_str(&stop.id);
        if shadow {
            out.push_str("  (noted only: nothing was stopped)");
        }
        out.push_str("\n  stops ");
        out.push_str(&resume::reaches(&stop.record).join(", "));
        out.push_str("\n  said by ");
        out.push_str(&field(&stop.record, "said_by"));
        out.push_str(", ");
        out.push_str(&field(&stop.record, "where"));
        out.push('\n');
    }
    out
}

fn resume(context: &terminal_kit::Context, scope: &str) -> Result<(), Failure> {
    if let Some(why) = under_pi() {
        return Err(Failure::new(Exit::Refused, "only Joel resumes a stop")
            .because(why)
            .try_this("run it at a terminal of your own"));
    }
    // The plan is read first: a stop on everything whose pause couldn't be
    // undone is left standing whole, rather than resumed with its agents'
    // work still frozen.
    let plan = match plan() {
        Ok(plan) => Some(plan),
        Err(failure) if pause::stands(&state()).is_some() => return Err(failure),
        Err(_) => None,
    };
    let at = now();
    let resumed = resume::resume(&folder(), scope, at, "joel").map_err(|error| {
        Failure::new(Exit::Failed, "couldn't mark a stop resumed").because(error.to_string())
    })?;
    // Undone after the records, so what a stop paused is undone even when
    // its record was resumed some other way and its pause left standing.
    let undone = match plan {
        Some(plan) => {
            let mut runs = pause::System { limit: STEP };
            pause::settle(&plan, &state(), &host(), at, &mut runs).map_err(|error| {
                Failure::new(Exit::Failed, "couldn't undo what a stop paused")
                    .because(error.to_string())
            })?
        }
        None => Vec::new(),
    };
    if resumed.is_empty() && undone.is_empty() {
        return Err(Failure::new(
            Exit::NotFound,
            format!("no stop standing is named by {scope}"),
        )
        .try_this("pineapple stops"));
    }
    let written = match context.format() {
        Format::Text if resumed.is_empty() => context.say(&format!(
            "{}; agents carry on within a minute\n",
            pause::said("resuming", &undone, None),
        )),
        Format::Text if !undone.is_empty() => context.say(&format!(
            "resumed {}; {}; agents carry on within a minute\n",
            resumed.join(", "),
            pause::said("resuming", &undone, None),
        )),
        Format::Text => context.say(&format!(
            "resumed {}; agents carry on within a minute\n",
            resumed.join(", ")
        )),
        _ => context.answer(&json!({ "schema": "pineapple.resumed/1", "resumed": resumed })),
    };
    written.map_err(|error| couldnt_write(&error))
}

fn couldnt_write(error: &std::io::Error) -> Failure {
    Failure::new(Exit::Failed, "couldn't write the answer").because(error.to_string())
}

/// Why this runs under pi, or none when it doesn't: what pi sets in the
/// shells it starts, the agents' own part of the user's manager, or pi
/// itself among this process's parents, which a shell can't hide.
fn under_pi() -> Option<String> {
    if let Some(var) = PI_SAYS.iter().find(|var| std::env::var_os(var).is_some()) {
        return Some(format!("{var} is set, as pi sets it in its shells"));
    }
    if let Some(why) = std::fs::read_to_string("/proc/self/cgroup")
        .ok()
        .and_then(|cgroup| agents_cgroup(&cgroup))
    {
        return Some(why);
    }
    let mut pid = parent_of("self")?;
    for _ in 0..DEEPEST {
        if pid <= 1 {
            return None;
        }
        if is_pi(pid) {
            return Some(format!("process {pid}, a parent of this one, is pi"));
        }
        pid = parent_of(&pid.to_string())?;
    }
    None
}

/// Why the cgroup `cgroup` names is agents', or none when it may be
/// Joel's: a service the user's manager runs is started for something,
/// not typed, and agents' work and their shells run in slices of their
/// own; only the awaited part of the jobs slice holds Joel's own terminal
/// work. Outside the user's manager, as in a build's sandbox, it says
/// nothing.
fn agents_cgroup(cgroup: &str) -> Option<String> {
    let path = cgroup
        .lines()
        .find_map(|line| line.strip_prefix("0::"))?
        .trim();
    let (_, inside) = path.split_once(".service/").filter(|(manager, _)| {
        manager
            .rsplit('/')
            .next()
            .is_some_and(|unit| unit.starts_with("user@"))
    })?;
    if inside.starts_with("jobs.slice/jobs-awaited.slice/") {
        return None;
    }
    let agents = ["agents.slice/", "jobs.slice/", "offload.slice/"];
    if agents.iter().any(|slice| inside.starts_with(slice))
        || inside
            .rsplit('/')
            .find(|part| !part.is_empty())
            .is_some_and(|leaf| leaf.ends_with(".service"))
    {
        return Some(format!(
            "it runs in {inside}, which the user's manager started for agents or a service"
        ));
    }
    None
}

fn parent_of(pid: &str) -> Option<u32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // The name in parentheses may hold spaces, so read after its end.
    let after = &stat[stat.rfind(')')? + 1..];
    after.split_whitespace().nth(1)?.parse().ok()
}

fn is_pi(pid: u32) -> bool {
    let Ok(cmdline) = std::fs::read(format!("/proc/{pid}/cmdline")) else {
        return false;
    };
    let words: Vec<String> = cmdline
        .split(|byte| *byte == 0)
        .map(|word| String::from_utf8_lossy(word).into_owned())
        .collect();
    let first = words.first().map(String::as_str).unwrap_or_default();
    first.rsplit('/').next() == Some("pi")
        || words
            .iter()
            .take(3)
            .any(|word| word.contains("pi-host") || word.contains("pi-coding-agent"))
}
