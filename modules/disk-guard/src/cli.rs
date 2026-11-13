//! The command line: the two daemons, and the commands that answer and
//! exit. It takes the shape every command of ours does (docs/commands.md)
//! from the terminal kit, which reads the shared flags, answers help and
//! says a failure; the guard's own options are read here by hand, since
//! there are few of them and a parser crate would be most of the binary's
//! dependencies.
//!
//! What a command answers is in `reads` (status, journal, plan, explain),
//! what it changes in `acts` (keep, unkeep, clear, footprint, thaw), and
//! the daemons in `daemons`, each changing for its own reason.

mod acts;
mod daemons;
mod reads;

use crate::band::Band;
use crate::config::{Config, Scope};
use crate::units::{Size, Span};
use serde_json::{Value, json};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use terminal_kit::{Context, Exit, Failure, Help, flags};

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
    Status,
    Plan {
        config: Option<PathBuf>,
        band: Option<Band>,
        pretend_free: Option<Size>,
        /// Survey every source again rather than read the daemon's last.
        fresh: bool,
    },
    Journal {
        since: Option<Span>,
    },
    CheckConfig {
        file: PathBuf,
    },
    Thaw,
    Explain {
        path: PathBuf,
        config: Option<PathBuf>,
    },
    /// Never offer what's in a folder, for a reason of Joel's.
    Keep {
        path: PathBuf,
        reason: String,
    },
    Unkeep {
        path: PathBuf,
    },
    /// Clear what the guard would take of a folder now, without waiting.
    Clear {
        path: PathBuf,
    },
    /// What the files under each path take on the disk, which the
    /// machine's guard reads.
    Footprint {
        paths: Vec<PathBuf>,
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
    /// The system daemon's memory and journal, in a folder any account may
    /// pass through to read the journal alone.
    pub system_state: PathBuf,
    pub system_status: PathBuf,
    pub system_socket: PathBuf,
    /// Where processes are read from: `/proc`.
    pub processes: PathBuf,
}

/// The commands, as they're typed.
const COMMANDS: [&str; 12] = [
    "status",
    "plan",
    "journal",
    "explain",
    "keep",
    "unkeep",
    "clear",
    "footprint",
    "thaw",
    "check-config",
    "user",
    "system",
];

/// The bands `--band` takes, as they're typed, said when it's given another.
const BANDS: [&str; 4] = ["healthy", "filling", "nearly-full", "critically-full"];

/// The command line's help, which the kit answers `--help` with.
pub fn help() -> Help {
    Help::new(
        "disk-guard",
        "keeps a disk from filling without breaking a build",
    )
    .example(
        "disk-guard",
        "how full each disk is, and what each guard did",
    )
    .example(
        "disk-guard plan",
        "what a pass would clear now, clearing nothing",
    )
    .example(
        "disk-guard keep ~/vms mine",
        "never offer ~/vms, because it's yours",
    )
    .example(
        "disk-guard status --json",
        "the status for a program, as disk-guard.status/1",
    )
    .command(
        "status",
        "each guard's disks, last pass and biggest holders",
    )
    .command("plan", "what a pass would take now, taking nothing")
    .command("journal", "what the guards did, oldest first")
    .command("explain", "why a path stays or would go, band by band")
    .command("keep", "never offer what's in a folder, for your reason")
    .command("unkeep", "let a kept folder go again")
    .command("clear", "clear what the guard would take of a folder now")
    .command("footprint", "what files really take on the disk")
    .command("thaw", "let every writer the guard froze carry on")
    .command("check-config", "whether a file is a config the guard reads")
    .command("user", "the account's guard, kept running")
    .command("system", "the machine's guard, kept running as root")
    .option("--config FILE", "read this config, not the daemon's own")
    .option(
        "--state DIR",
        "user, system: where the daemon keeps its state",
    )
    .option("--band BAND", "plan: as if the disk were in this band")
    .option("--pretend-free SIZE", "plan: as if this much were free")
    .option("--fresh", "plan: survey every source again")
    .option(
        "--since SPAN",
        "journal: only what's newer, such as 1h or 2d",
    )
    .exit(
        Exit::Failed,
        "it couldn't be done, or a footprint couldn't be read",
    )
    .exit(
        Exit::BadInput,
        "check-config: the file isn't a config the guard reads",
    )
    .exit(Exit::NotFound, "the file or folder named isn't there")
    .exit(Exit::Unavailable, "no guard is running to ask")
    .exit(Exit::TryAgain, "a daemon already runs with that state")
    .exit(
        Exit::Config,
        "the config can't be read, or is the other daemon's",
    )
}

impl Env {
    /// The places the XDG base directories, the account and this platform
    /// give.
    pub fn from_environment() -> Env {
        let var = |name: &str| {
            std::env::var_os(name)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        };
        // SAFETY: geteuid only reads the process's own credentials.
        let uid = unsafe { libc::geteuid() };
        Env::for_os(std::env::consts::OS, var, uid)
    }

    /// The places on `os`, as Rust names it, given the environment `var`
    /// reads and the account `uid`. A Mac keeps a daemon's state in
    /// /var/db and what lasts a boot in /var/run, and has no runtime
    /// directory for an account: its own temporary directory, which
    /// launchd gives an agent, lasts until the next boot too.
    pub fn for_os(os: &str, var: impl Fn(&str) -> Option<PathBuf>, uid: u32) -> Env {
        let home = var("HOME").unwrap_or_else(|| PathBuf::from("/"));
        let config = var("XDG_CONFIG_HOME").unwrap_or_else(|| home.join(".config"));
        let state = var("XDG_STATE_HOME").unwrap_or_else(|| home.join(".local/state"));
        let mac = os == "macos";
        let runtime = var("XDG_RUNTIME_DIR").unwrap_or_else(|| {
            if mac {
                var("TMPDIR").unwrap_or_else(|| home.join("Library/Caches"))
            } else {
                PathBuf::from(format!("/run/user/{uid}"))
            }
        });
        let (system_state, system_run) = if mac {
            ("/var/db/disk-guard", "/var/run/disk-guard")
        } else {
            ("/var/lib/disk-guard", "/run/disk-guard")
        };
        Env {
            config: config.join("disk-guard/config.json"),
            state: state.join("disk-guard"),
            status: runtime.join("disk-guard/status.json"),
            system_config: PathBuf::from("/etc/disk-guard/config.json"),
            system_state: PathBuf::from(system_state),
            system_status: Path::new(system_run).join("status.json"),
            system_socket: Path::new(system_run).join("socket"),
            processes: PathBuf::from("/proc"),
        }
    }
}

/// The command line asked wrongly: what was wrong, and where to read how.
fn asked(what: impl Into<String>) -> Failure {
    Failure::new(Exit::Usage, what).try_this("disk-guard --help")
}

/// The arguments the kit left after the shared flags, read into a
/// command. Nothing at all asks for the status, the common read.
pub fn parse(args: &[String]) -> Result<Command, Failure> {
    let Some((name, rest)) = args.split_first() else {
        return Ok(Command::Status);
    };
    let mut options = Options::read(rest)?;
    let command = match name.as_str() {
        "user" => Command::User {
            config: options.path("--config"),
            state: options.path("--state"),
        },
        "system" => Command::System {
            config: options.path("--config"),
            state: options.path("--state"),
        },
        "status" => Command::Status,
        "plan" => Command::Plan {
            config: options.path("--config"),
            band: options
                .value("--band")
                .map(|band| {
                    serde_json::from_value(Value::String(band.clone())).map_err(|_| {
                        asked(format!("{band:?} is not a band"))
                            .because(format!("the bands are {}", flags::or(&BANDS)))
                    })
                })
                .transpose()?,
            pretend_free: options
                .value("--pretend-free")
                .map(|size| Size::parse(&size).map_err(asked))
                .transpose()?,
            fresh: options.flag("--fresh"),
        },
        "journal" => Command::Journal {
            since: options
                .value("--since")
                .map(|span| Span::parse(&span).map_err(asked))
                .transpose()?,
        },
        "check-config" => Command::CheckConfig {
            file: options.one("check-config takes one file")?,
        },
        "thaw" => Command::Thaw,
        "keep" => {
            let (path, reason) = kept(std::mem::take(&mut options.positional))?;
            Command::Keep { path, reason }
        }
        "unkeep" => Command::Unkeep {
            path: options.one("unkeep takes one path")?,
        },
        "clear" => Command::Clear {
            path: options.one("clear takes one folder")?,
        },
        "footprint" => {
            if options.positional.is_empty() {
                return Err(asked("footprint takes one path or more"));
            }
            let paths = std::mem::take(&mut options.positional)
                .into_iter()
                .map(PathBuf::from)
                .collect();
            Command::Footprint { paths }
        }
        "explain" => {
            let config = options.path("--config");
            Command::Explain {
                path: options.one("explain takes one path")?,
                config,
            }
        }
        _ => return Err(flags::unknown("command", name, &COMMANDS).try_this("disk-guard --help")),
    };
    options.finished()?;
    Ok(command)
}

/// A keep's folder and its reason, the words after the folder.
fn kept(words: Vec<String>) -> Result<(PathBuf, String), Failure> {
    let mut words = words.into_iter();
    let path = words
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| asked("keep takes a folder and a reason"))?;
    let reason = words.collect::<Vec<_>>().join(" ");
    if reason.trim().is_empty() {
        return Err(asked("keep needs a reason, after the folder"));
    }
    Ok((path, reason.trim().to_owned()))
}

/// Options as `--name value` and `--flag`, and anything else. After `--`
/// every word is the command's own, as a folder or a reason may begin
/// with a dash.
struct Options {
    named: Vec<(String, Option<String>)>,
    positional: Vec<String>,
}

/// The options that take no value.
const FLAGS: [&str; 1] = ["--fresh"];

impl Options {
    fn read(args: &[String]) -> Result<Options, Failure> {
        let mut options = Options {
            named: Vec::new(),
            positional: Vec::new(),
        };
        let mut args = args.iter();
        while let Some(arg) = args.next() {
            if arg == "--" {
                options.positional.extend(args.cloned());
                break;
            }
            if !arg.starts_with("--") {
                options.positional.push(arg.clone());
                continue;
            }
            if options.named.iter().any(|(seen, _)| seen == arg) {
                return Err(asked(format!("{arg} is given twice")));
            }
            let value = if FLAGS.contains(&arg.as_str()) {
                None
            } else {
                Some(
                    args.next()
                        .ok_or_else(|| asked(format!("{arg} needs a value")))?
                        .clone(),
                )
            };
            options.named.push((arg.clone(), value));
        }
        Ok(options)
    }

    /// The option named `name` and its value, if it was given.
    fn take(&mut self, name: &str) -> Option<(String, Option<String>)> {
        let at = self.named.iter().position(|(seen, _)| seen == name)?;
        Some(self.named.remove(at))
    }

    /// The value given for `name`, if it was given one.
    fn value(&mut self, name: &str) -> Option<String> {
        self.take(name).and_then(|(_, value)| value)
    }

    fn flag(&mut self, name: &str) -> bool {
        self.take(name).is_some()
    }

    fn path(&mut self, name: &str) -> Option<PathBuf> {
        self.value(name).map(PathBuf::from)
    }

    /// The one word a command takes, refused in `words` otherwise.
    fn one(&mut self, words: &str) -> Result<PathBuf, Failure> {
        match self.positional.as_slice() {
            [one] => {
                let one = PathBuf::from(one);
                self.positional.clear();
                Ok(one)
            }
            _ => Err(asked(words)),
        }
    }

    /// Refuses whatever the command didn't ask for.
    fn finished(self) -> Result<(), Failure> {
        if let Some((name, _)) = self.named.first() {
            return Err(asked(format!("{name} is not an option here")));
        }
        if let Some(arg) = self.positional.first() {
            return Err(asked(format!("{arg:?} is not expected here")));
        }
        Ok(())
    }
}

/// Runs a command that answers and exits, its answer on `out` in the form
/// `context` asks for and its notes on `err`, or the failure it ends with.
pub fn run(
    command: Command,
    env: &Env,
    context: &Context,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), Failure> {
    match command {
        Command::CheckConfig { file } => check_config(&file, context, out),
        Command::Status => reads::status(env, context, out),
        Command::Journal { since } => reads::journal(env, since, context, out, err),
        Command::Plan {
            config,
            band,
            pretend_free,
            fresh,
        } => reads::plan(
            env,
            config.as_deref(),
            (band, pretend_free),
            fresh,
            context,
            out,
        ),
        Command::Explain { path, config } => {
            reads::explain(env, &path, config.as_deref(), context, out)
        }
        Command::User { config, state } => daemons::daemon(env, Scope::User, config, state, err),
        Command::System { config, state } => {
            daemons::daemon(env, Scope::System, config, state, err)
        }
        Command::Thaw => acts::thaw(context, out),
        Command::Keep { path, reason } => acts::keep(env, &path, &reason, context, out),
        Command::Unkeep { path } => acts::unkeep(env, &path, context, out),
        Command::Clear { path } => acts::clear(env, &path, context, out),
        Command::Footprint { paths } => acts::footprint(env, &paths, context, out),
    }
}

/// Reads a config, saying nothing where it's one the guard reads; a file
/// that isn't there, or isn't one, is the input at fault, not the guard's
/// own config.
fn check_config(file: &Path, context: &Context, out: &mut dyn Write) -> Result<(), Failure> {
    let config = read_config(file).map_err(|failure| {
        let exit = if file.exists() {
            Exit::BadInput
        } else {
            Exit::NotFound
        };
        Failure { exit, ..failure }
    })?;
    if context.format().is_json() {
        let scope = match config.scope {
            Scope::User => "user",
            Scope::System => "system",
        };
        let answer = json!({
            "schema": "disk-guard.check-config/1",
            "file": file.to_string_lossy(),
            "scope": scope,
        });
        return context.answer_to(out, &answer).map_err(failed);
    }
    Ok(())
}

/// The config at `path`, or why it can't be the guard's.
fn read_config(path: &Path) -> Result<Config, Failure> {
    let text = fs::read_to_string(path).map_err(|error| {
        Failure::new(Exit::Config, format!("{} can't be read", path.display()))
            .because(error.to_string())
    })?;
    Config::parse(&text).map_err(|why| {
        Failure::new(
            Exit::Config,
            format!("{} isn't a config the guard reads", path.display()),
        )
        .because(why)
    })
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "it takes the error as `map_err` hands it over, so callers write `map_err(failed)`"
)]
fn failed(error: io::Error) -> Failure {
    Failure::new(Exit::Failed, error.to_string())
}

fn wall() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |since| since.as_secs_f64())
}

/// A path as JSON says it: as text, which the command line's own always
/// are.
fn said_path(path: &Path) -> Value {
    Value::from(path.to_string_lossy().into_owned())
}
