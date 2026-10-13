//! The command line: the two daemons, and the commands that answer and
//! exit. Arguments are read by hand, since there are few of them and a
//! parser crate would be most of the binary's dependencies.

use crate::band::Band;
use crate::config::{Config, Scope};
use crate::daemon::{Explained, Guard, Places};
use crate::serve::{answering, hold, steps, stopping};
use crate::units::{Size, Span, shown as size};
use serde_json::Value;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    User {
        config: Option<PathBuf>,
        state: Option<PathBuf>,
    },
    System {
        config: Option<PathBuf>,
        state: Option<PathBuf>,
    },
    Status {
        json: bool,
    },
    Plan {
        config: Option<PathBuf>,
        band: Option<Band>,
        pretend_free: Option<Size>,
    },
    Journal {
        since: Option<Span>,
        json: bool,
    },
    CheckConfig {
        file: PathBuf,
    },
    Thaw,
    Explain {
        path: PathBuf,
        config: Option<PathBuf>,
    },
}

/// Where the commands find things, worked out once from the environment.
#[derive(Clone, Debug, PartialEq)]
pub struct Env {
    /// The user daemon's config.
    pub config: PathBuf,
    /// The user daemon's memory and journal.
    pub state: PathBuf,
    /// The user daemon's status file.
    pub status: PathBuf,
    pub system_config: PathBuf,
    pub system_state: PathBuf,
    pub system_status: PathBuf,
    pub system_socket: PathBuf,
    /// Where processes are read from: `/proc`.
    pub processes: PathBuf,
}

/// Fine.
pub const FINE: i32 = 0;
/// The command line or a config was refused.
pub const REFUSED: i32 = 1;
/// A daemon already runs for this scope.
pub const RUNNING: i32 = 2;
/// No daemon has anything to say.
pub const NO_DAEMON: i32 = 3;

const USAGE: &str = "usage: disk-guard user|system [--config FILE] [--state DIR]
       disk-guard status [--json]
       disk-guard plan [--config FILE] [--band BAND] [--pretend-free SIZE]
       disk-guard journal [--since SPAN] [--json]
       disk-guard check-config FILE
       disk-guard explain PATH [--config FILE]
       disk-guard thaw";

impl Env {
    /// The places the XDG base directories and the account give.
    pub fn from_environment() -> Env {
        let var = |name: &str| {
            std::env::var_os(name)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        };
        let home = var("HOME").unwrap_or_else(|| PathBuf::from("/"));
        let config = var("XDG_CONFIG_HOME").unwrap_or_else(|| home.join(".config"));
        let state = var("XDG_STATE_HOME").unwrap_or_else(|| home.join(".local/state"));
        // SAFETY: geteuid only reads the process's own credentials.
        let uid = unsafe { libc::geteuid() };
        let runtime =
            var("XDG_RUNTIME_DIR").unwrap_or_else(|| PathBuf::from(format!("/run/user/{uid}")));
        Env {
            config: config.join("disk-guard/config.json"),
            state: state.join("disk-guard"),
            status: runtime.join("disk-guard/status.json"),
            system_config: PathBuf::from("/etc/disk-guard/config.json"),
            system_state: PathBuf::from("/var/lib/disk-guard"),
            system_status: PathBuf::from("/run/disk-guard/status.json"),
            system_socket: PathBuf::from("/run/disk-guard/socket"),
            processes: PathBuf::from("/proc"),
        }
    }
}

/// The arguments after the program's name, read into a command.
pub fn parse(args: &[OsString]) -> Result<Command, String> {
    let Some((name, rest)) = args.split_first() else {
        return Err(USAGE.to_owned());
    };
    let mut options = Options::read(rest)?;
    let command = match name.to_str() {
        Some("user") => Command::User {
            config: options.path("--config")?,
            state: options.path("--state")?,
        },
        Some("system") => Command::System {
            config: options.path("--config")?,
            state: options.path("--state")?,
        },
        Some("status") => Command::Status {
            json: options.flag("--json"),
        },
        Some("plan") => Command::Plan {
            config: options.path("--config")?,
            band: options
                .text("--band")?
                .map(|band| {
                    serde_json::from_value(Value::String(band.clone())).map_err(|_| {
                        format!("{band:?} is not a band: healthy, pressure, urgent or critical")
                    })
                })
                .transpose()?,
            pretend_free: options
                .text("--pretend-free")?
                .map(|size| Size::parse(&size))
                .transpose()?,
        },
        Some("journal") => Command::Journal {
            since: options
                .text("--since")?
                .map(|span| Span::parse(&span))
                .transpose()?,
            json: options.flag("--json"),
        },
        Some("check-config") => match options.positional.as_slice() {
            [file] => {
                let file = PathBuf::from(file);
                options.positional.clear();
                Command::CheckConfig { file }
            }
            _ => return Err(format!("check-config takes one file\n{USAGE}")),
        },
        Some("thaw") => Command::Thaw,
        Some("explain") => {
            let config = options.path("--config")?;
            match options.positional.as_slice() {
                [path] => {
                    let path = PathBuf::from(path);
                    options.positional.clear();
                    Command::Explain { path, config }
                }
                _ => return Err(format!("explain takes one path\n{USAGE}")),
            }
        }
        _ => return Err(format!("{name:?} is not a command\n{USAGE}")),
    };
    options.finished()?;
    Ok(command)
}

/// Options as `--name value` and `--flag`, and anything else.
struct Options {
    named: Vec<(String, Option<OsString>)>,
    positional: Vec<OsString>,
}

/// The options that take no value.
const FLAGS: [&str; 1] = ["--json"];

impl Options {
    fn read(args: &[OsString]) -> Result<Options, String> {
        let mut options = Options {
            named: Vec::new(),
            positional: Vec::new(),
        };
        let mut args = args.iter();
        while let Some(arg) = args.next() {
            let Some(name) = arg.to_str().filter(|arg| arg.starts_with("--")) else {
                options.positional.push(arg.clone());
                continue;
            };
            if options.named.iter().any(|(seen, _)| seen == name) {
                return Err(format!("{name} is given twice"));
            }
            let value = if FLAGS.contains(&name) {
                None
            } else {
                Some(
                    args.next()
                        .ok_or_else(|| format!("{name} needs a value"))?
                        .clone(),
                )
            };
            options.named.push((name.to_owned(), value));
        }
        Ok(options)
    }

    fn take(&mut self, name: &str) -> Option<Option<OsString>> {
        let at = self.named.iter().position(|(seen, _)| seen == name)?;
        Some(self.named.remove(at).1)
    }

    fn flag(&mut self, name: &str) -> bool {
        self.take(name).is_some()
    }

    fn path(&mut self, name: &str) -> Result<Option<PathBuf>, String> {
        Ok(self.take(name).flatten().map(PathBuf::from))
    }

    fn text(&mut self, name: &str) -> Result<Option<String>, String> {
        self.take(name)
            .flatten()
            .map(|value| {
                value
                    .into_string()
                    .map_err(|value| format!("{name} {value:?} is not text"))
            })
            .transpose()
    }

    /// Refuses whatever the command didn't ask for.
    fn finished(self) -> Result<(), String> {
        if let Some((name, _)) = self.named.first() {
            return Err(format!("{name} is not an option here\n{USAGE}"));
        }
        if let Some(arg) = self.positional.first() {
            return Err(format!("{arg:?} is not expected here\n{USAGE}"));
        }
        Ok(())
    }
}

/// Runs a command that answers and exits, saying how in its exit code.
pub fn run(command: Command, env: &Env, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let done = match command {
        Command::CheckConfig { file } => read_config(&file).map(|_| FINE),
        Command::Status { json } => status(env, json, out),
        Command::Journal { since, json } => journal(env, since, json, out),
        Command::Plan {
            config,
            band,
            pretend_free,
        } => plan(env, config, band, pretend_free, out),
        Command::User { config, state } => daemon(env, Scope::User, config, state, err),
        Command::System { config, state } => daemon(env, Scope::System, config, state, err),
        Command::Thaw => thaw(out),
        Command::Explain { path, config } => explain(env, &path, config, out),
    };
    match done {
        Ok(code) => code,
        Err((code, why)) => {
            let _ = writeln!(err, "disk-guard: {why}");
            code
        }
    }
}

type Done = Result<i32, (i32, String)>;

/// How often a daemon steps.
const STEP_EVERY: Duration = Duration::from_secs(5);

/// How often each account may ask the system daemon.
const ASK_EVERY: Duration = Duration::from_millis(500);

/// Runs a daemon until it's told to stop.
fn daemon(
    env: &Env,
    scope: Scope,
    config: Option<PathBuf>,
    state: Option<PathBuf>,
    err: &mut dyn Write,
) -> Done {
    // Before any thread starts, so every thread blocks them too.
    let mut wait = stopping(&[libc::SIGTERM, libc::SIGINT, libc::SIGHUP]).map_err(failed)?;
    let (default_config, default_state, name) = match scope {
        Scope::User => (&env.config, &env.state, "user"),
        Scope::System => (&env.system_config, &env.system_state, "system"),
    };
    let config_path = config.unwrap_or_else(|| default_config.clone());
    let config = read_config(&config_path)?;
    if config.scope != scope {
        return Err((
            REFUSED,
            format!(
                "{} is a config for the other daemon, not the {name} daemon",
                config_path.display()
            ),
        ));
    }
    let state = state.unwrap_or_else(|| default_state.clone());
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&state)
        .map_err(|error| (REFUSED, format!("{}: {error}", state.display())))?;
    let Some(_held) = hold(&state).map_err(failed)? else {
        return Err((
            RUNNING,
            format!("a {name} daemon already runs with {}", state.display()),
        ));
    };
    let mut hearing = None;
    let places = match scope {
        Scope::User => Places {
            state: state.clone(),
            status: Some(env.status.clone()),
            processes: env.processes.clone(),
            system: Some(env.system_socket.clone()),
            running: running(),
        },
        Scope::System => {
            if let Some(dir) = env.system_socket.parent() {
                fs::create_dir_all(dir).map_err(failed)?;
            }
            let store = crate::pool::pool(&config.nix.store).ok();
            hearing = Some(
                answering(&env.system_socket, &env.processes, ASK_EVERY, store).map_err(failed)?,
            );
            Places {
                state: state.clone(),
                status: Some(env.system_status.clone()),
                processes: env.processes.clone(),
                system: None,
                running: running(),
            }
        }
    };
    // Only the user daemon has a session to show a notice in.
    let mut desk = crate::desktop::Buses::new(config.notify && scope == Scope::User);
    let builds = config
        .sources
        .values()
        .any(|source| matches!(source, crate::config::Source::Cargo { .. }));
    let freezes = config.freeze;
    let mut guard = Guard::new(config, &places).map_err(failed)?;
    if let Some(hearing) = hearing {
        guard.hear(hearing);
    }
    // Only the account's guard freezes, in its own manager, unless its
    // config says never.
    if scope == Scope::User && freezes {
        guard.freeze_with(Box::new(crate::freezer::Session::default()));
    }
    // Without a watch nothing dates an older build, so it stays.
    if builds {
        match crate::opens::Watch::new() {
            Ok(watch) => guard.listen(std::sync::Arc::new(watch)),
            Err(error) => {
                let _ = writeln!(
                    err,
                    "disk-guard: no watch on builds, so older ones stay: {error}"
                );
            }
        }
    }
    steps(&mut guard, STEP_EVERY, &mut wait, &mut desk, err);
    Ok(FINE)
}

/// Says why `path` stays or would go: the candidate a source offers there
/// or holding it, how it's in use now, and what each band would do.
fn explain(env: &Env, path: &Path, config: Option<PathBuf>, out: &mut dyn Write) -> Done {
    let config = read_config(config.as_deref().unwrap_or(&env.config))?;
    let places = Places {
        state: env.state.clone(),
        status: None,
        processes: env.processes.clone(),
        system: Some(env.system_socket.clone()),
        running: running(),
    };
    let mut guard = Guard::new(config, &places).map_err(failed)?;
    let tick = crate::linux::tick().map_err(failed)?;
    let used_as = |used: crate::activity::Use| match used {
        crate::activity::Use::Free => "by nothing".to_owned(),
        crate::activity::Use::Busy(pid) => format!("busy: process {pid} works in it"),
        crate::activity::Use::Occupied(pid) => format!("process {pid} sits in its tree"),
        crate::activity::Use::Unknown(pid) => format!("maybe: process {pid} can't be read"),
    };
    let (candidate, used) = match guard.explain(&tick, path).map_err(failed)? {
        Explained::Offered { candidate, used } => (candidate, used),
        Explained::Kept { path, used } => {
            writeln!(out, "{}: kept while it's used", path.display()).map_err(failed)?;
            writeln!(out, "  in use: {}", used_as(used)).map_err(failed)?;
            return Ok(FINE);
        }
        Explained::Nothing => {
            writeln!(
                out,
                "No source offers {} or anything holding it, so the guard leaves it be.",
                path.display()
            )
            .map_err(failed)?;
            return Ok(FINE);
        }
    };
    writeln!(
        out,
        "{}: {}, {}, idle {}",
        candidate.path.display(),
        candidate.class.kind(),
        size(candidate.size),
        span(candidate.idle)
    )
    .map_err(failed)?;
    writeln!(out, "  in use: {}", used_as(used)).map_err(failed)?;
    let (_, windows) = crate::plan::windows(&candidate);
    for (band, window) in ["healthy", "pressure", "urgent", "critical"]
        .iter()
        .zip(windows)
    {
        let when = match window {
            None => "never".to_owned(),
            Some(window) if window <= 0.0 => "at once".to_owned(),
            Some(window) => format!("after {} idle", span(window)),
        };
        writeln!(out, "  {band}: {when}").map_err(failed)?;
    }
    Ok(FINE)
}

/// Thaws whatever the account's guard froze, through the account's own
/// service manager. The guard sees it thawed and never freezes it again.
fn thaw(out: &mut dyn Write) -> Done {
    let failed = |error: zbus::Error| (NO_DAEMON, format!("asking the service manager: {error}"));
    let builder = zbus::blocking::connection::Builder::session().map_err(failed)?;
    let freezer = crate::freezer::Freezer::new(crate::desktop::patient(builder).map_err(failed)?);
    let thawed = crate::freezer::thaw_all(&freezer).map_err(failed)?;
    if thawed.is_empty() {
        let _ = writeln!(out, "Nothing is frozen.");
    }
    for unit in thawed {
        let _ = writeln!(out, "Thawed {unit}.");
    }
    Ok(FINE)
}

/// The links to the systems running and booted, NixOS's own, whose
/// generations never expire.
fn running() -> Vec<PathBuf> {
    ["/run/current-system", "/run/booted-system"]
        .map(PathBuf::from)
        .to_vec()
}

fn read_config(path: &Path) -> Result<Config, (i32, String)> {
    let text = fs::read_to_string(path)
        .map_err(|error| (REFUSED, format!("{}: {error}", path.display())))?;
    Config::parse(&text).map_err(|why| (REFUSED, format!("{}: {why}", path.display())))
}

fn failed(error: io::Error) -> (i32, String) {
    (REFUSED, error.to_string())
}

fn wall() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |since| since.as_secs_f64())
}

/// A length of time as people read one.
fn span(seconds: f64) -> String {
    let seconds = seconds.max(0.0);
    if seconds < 90.0 {
        format!("{seconds:.0} s")
    } else if seconds < 90.0 * 60.0 {
        format!("{:.0} min", seconds / 60.0)
    } else if seconds < 48.0 * 3600.0 {
        format!("{:.1} h", seconds / 3600.0)
    } else if seconds < 730.0 * 86400.0 {
        format!("{:.1} d", seconds / 86400.0)
    } else {
        format!("{:.0} y", seconds / (365.25 * 86400.0))
    }
}

/// How old a heartbeat may be before its guard is taken to have stopped.
const STOPPED_AFTER: f64 = 120.0;

fn status(env: &Env, json: bool, out: &mut dyn Write) -> Done {
    let read = |path: &Path| -> Option<Value> {
        serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
    };
    let (user, system) = (read(&env.status), read(&env.system_status));
    if user.is_none() && system.is_none() {
        return Err((
            NO_DAEMON,
            "no guard has written its status; is one running?".to_owned(),
        ));
    }
    if json {
        let both = serde_json::json!({ "user": user, "system": system });
        writeln!(out, "{both}").map_err(failed)?;
        return Ok(FINE);
    }
    let now = wall();
    for (name, status) in [("user", user), ("system", system)] {
        let Some(status) = status else { continue };
        let age = now - status["heartbeat"].as_f64().unwrap_or(0.0);
        if age > STOPPED_AFTER {
            writeln!(out, "{name} guard, not heard from for {}", span(age)).map_err(failed)?;
        } else {
            writeln!(out, "{name} guard, heard from {} ago", span(age)).map_err(failed)?;
        }
        for filesystem in status["filesystems"].as_array().into_iter().flatten() {
            let band = filesystem["band"].as_str().unwrap_or("unknown");
            let mut line = format!(
                "  {}: {band}, {} free of {}",
                filesystem["path"].as_str().unwrap_or("?"),
                size(filesystem["free"].as_u64().unwrap_or(0)),
                size(filesystem["total"].as_u64().unwrap_or(0)),
            );
            let next = match band {
                "healthy" => Some("pressure"),
                "pressure" => Some("urgent"),
                "urgent" => Some("critical"),
                _ => None,
            };
            if let (Some(next), Some(forecast)) = (next, filesystem["forecast"].as_f64()) {
                line.push_str(&format!(", {next} in {}", span(forecast)));
            }
            writeln!(out, "{line}").map_err(failed)?;
        }
        let last = &status["last"];
        if last.is_object() {
            writeln!(
                out,
                "  last pass: {} ago on {}, {}, {}, freed {}",
                span(now - last["at"].as_f64().unwrap_or(0.0)),
                last["path"].as_str().unwrap_or("?"),
                last["band"].as_str().unwrap_or("?"),
                last["end"].as_str().unwrap_or("?"),
                size(last["freed"].as_u64().unwrap_or(0)),
            )
            .map_err(failed)?;
        }
    }
    Ok(FINE)
}

fn journal(env: &Env, since: Option<Span>, json: bool, out: &mut dyn Write) -> Done {
    let now = wall();
    let cut = since.map_or(f64::NEG_INFINITY, |since| now - since.0.as_secs_f64());
    for name in ["journal.jsonl.1", "journal.jsonl"] {
        let text = match fs::read_to_string(env.state.join(name)) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(failed(error)),
        };
        for line in text.lines() {
            // A line cut short by a crash says nothing.
            let Ok(entry) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            let at = entry["at"].as_f64().unwrap_or(0.0);
            if at < cut {
                continue;
            }
            if json {
                writeln!(out, "{line}").map_err(failed)?;
                continue;
            }
            let when = format!("{} ago", span(now - at));
            let band = entry["band"].as_str().unwrap_or("?");
            let said = match entry["event"].as_str() {
                Some("action") => {
                    let mut said = format!(
                        "{:<9} {:<10} {:>10}  {}",
                        entry["outcome"].as_str().unwrap_or("?"),
                        entry["kind"].as_str().unwrap_or("?"),
                        size(entry["size"].as_u64().unwrap_or(0)),
                        entry["path"].as_str().unwrap_or("?"),
                    );
                    if let Some(reason) = entry["reason"].as_str() {
                        said.push_str(&format!(" ({reason})"));
                    }
                    said
                }
                Some("pass") => format!(
                    "pass {}, freed {}{}",
                    entry["end"].as_str().unwrap_or("?"),
                    size(entry["freed"].as_u64().unwrap_or(0)),
                    // A journal from before passes were timed says nothing.
                    entry["seconds"]
                        .as_f64()
                        .map_or_else(String::new, |seconds| format!(" in {}", span(seconds))),
                ),
                _ => line.to_owned(),
            };
            writeln!(out, "{when:>12}  {band:<8}  {said}").map_err(failed)?;
        }
    }
    Ok(FINE)
}

fn plan(
    env: &Env,
    config: Option<PathBuf>,
    band: Option<Band>,
    free: Option<Size>,
    out: &mut dyn Write,
) -> Done {
    let config = read_config(config.as_deref().unwrap_or(&env.config))?;
    let places = Places {
        state: env.state.clone(),
        status: None,
        processes: env.processes.clone(),
        system: Some(env.system_socket.clone()),
        running: running(),
    };
    let mut guard = Guard::new(config, &places).map_err(failed)?;
    let tick = crate::linux::tick().map_err(failed)?;
    let previews = guard.preview(&tick, band, free).map_err(failed)?;
    for preview in previews {
        let band = serde_json::to_value(preview.band).ok();
        writeln!(
            out,
            "{}: {}, {} free, target {}; the pass would end {}",
            preview.path.display(),
            band.as_ref().and_then(Value::as_str).unwrap_or("?"),
            size(preview.free),
            size(preview.target),
            preview.end,
        )
        .map_err(failed)?;
        for candidate in &preview.taken {
            writeln!(
                out,
                "  {:<10} {:>10}  idle {:>7}  {}",
                candidate.class.kind(),
                size(candidate.size),
                span(candidate.idle),
                candidate.path.display(),
            )
            .map_err(failed)?;
        }
    }
    let (unread, why) = guard.unread();
    if unread > 0 {
        let why = why.map_or_else(String::new, |why| {
            format!(" and the system daemon couldn't be asked about them ({why})")
        });
        writeln!(
            out,
            "{unread} processes couldn't be read{why}, so a tree one of them might use isn't \
             shown."
        )
        .map_err(failed)?;
    }
    writeln!(
        out,
        "Nothing was taken. Whether each is in use is judged when a pass tries it."
    )
    .map_err(failed)?;
    Ok(FINE)
}
