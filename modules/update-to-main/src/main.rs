//! update-to-main's command (src/lib.rs says what it's for).
//!
//! - `update-to-main serve --config FILE`: root's daemon, on the config's
//!   socket.
//! - `update-to-main ask [--at INPUT=COMMIT] [--socket FILE]`: asks for an
//!   update and prints each line as it comes, as JSON. With `--at`, the
//!   followed input INPUT is built at COMMIT, by its full id, rather than
//!   at its main's head; the commit must be on its main, signed, and past
//!   what the machine last followed, and holds still judge it.
//!   Otherwise it's as `ask`. It exits 0 when the machine runs main
//!   or already did, 1 when the update failed, 77 when it was refused and
//!   69 when the daemon couldn't be asked or stopped answering.
//! - `update-to-main stop [--socket FILE]`: stops the update the daemon is
//!   fetching or building, one a hold was given for too, and prints what
//!   it did. A switch runs on.
//! - `update-to-main pause|resume [--socket FILE]`: refuses the
//!   follower's updates, stopping one it alone asked for, until resumed,
//!   while a person's ask still updates; and `update-to-main
//!   hold|lift COMMIT [--socket FILE]` refuses any main holding the
//!   commit, by its full id, until lifted (src/held.rs). Each prints what
//!   holds now, and exits 0, or 77 when the daemon refused the ask.
//! - `update-to-main follow [--config FILE] [--every SECONDS]`: an
//!   account's service that asks whenever main moves (src/follow.rs), and
//!   at least every SECONDS, 600 unless given. Once the system's path
//!   holds another update-to-main, it starts that one in its place, in the
//!   same process. A config it can't read it waits out, looking again
//!   every half minute, since a switch writes a newer one's config first.
//! - `update-to-main job FOLDER`: a job the daemon starts in an account's
//!   login session on a Mac, running the command written in FOLDER
//!   (src/session.rs). Root reads how the command ended from the folder;
//!   the job itself ends 0, or 1 when its command didn't.
//!
//! Each takes the terminal kit's shape, its help its own (`help()`).

use std::fs;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use terminal_kit::{Exit, Failure, Help};
use update_to_main::config::Config;
use update_to_main::follow::{self, Follower};
use update_to_main::follows::Followed;
use update_to_main::line::Line;
use update_to_main::{ask, serve, session};

/// Where the daemon listens unless the config says otherwise, so an asker
/// needn't be told.
const SOCKET: &str = if cfg!(target_os = "macos") {
    "/var/run/update-to-main.sock"
} else {
    "/run/update-to-main/ask.sock"
};

/// Where the module writes the config, which a follower reads too.
const CONFIG: &str = "/etc/update-to-main/config.json";

/// How often a follower asks at least, for a main it can't read cheaply.
const FOLLOW_EVERY: u64 = 600;

/// Where `--socket` says the daemon is, as each asking command takes it.
fn socketed(help: Help) -> Help {
    help.option("--socket FILE", "the daemon's, unless the usual")
        .exit(Exit::Unavailable, "the daemon couldn't be asked")
}

/// A command that asks the daemon to hold or let go, as `--help` says it.
fn holding(name: &str, about: &str, example: &str) -> Help {
    socketed(
        Help::new(name, about)
            .example(
                &format!("update-to-main {example}"),
                "and says what holds now",
            )
            .exit(Exit::Refused, "the daemon refused the ask"),
    )
}

/// What `update-to-main --help` says, and each command's own.
fn help() -> Help {
    Help::new("update-to-main", "updates this machine to its main")
        .example("update-to-main ask", "updates now, saying each step")
        .example("update-to-main stop", "stops the update under way")
        .subcommand(socketed(
            Help::new("ask", "asks for an update, saying each step")
                .example("update-to-main ask", "at main's head")
                .example(
                    "update-to-main ask --at core=COMMIT",
                    "one input at a commit on its main",
                )
                .option("--at INPUT=COMMIT", "builds INPUT at COMMIT")
                .exit(Exit::Failed, "the update failed")
                .exit(Exit::Refused, "the update was refused"),
        ))
        .subcommand(socketed(
            Help::new("stop", "stops the update being fetched or built")
                .example("update-to-main stop", "a switch under way runs on"),
        ))
        .subcommand(holding(
            "pause",
            "refuses the follower's updates until resumed",
            "pause",
        ))
        .subcommand(holding(
            "resume",
            "lets the follower update again",
            "resume",
        ))
        .subcommand(
            holding("hold", "refuses any main holding a commit", "hold COMMIT")
                .argument("<commit>", "its full id"),
        )
        .subcommand(
            holding("lift", "lets a held commit through again", "lift COMMIT")
                .argument("<commit>", "its full id"),
        )
        .subcommand(
            Help::new("serve", "root's daemon, on the config's socket")
                .example("update-to-main serve --config FILE", "as its unit runs it")
                .option("--config FILE", "what it serves by")
                .exit(Exit::Failed, "it couldn't serve")
                .exit(Exit::Config, "its config couldn't be read"),
        )
        .subcommand(
            Help::new("follow", "asks for an update whenever main moves")
                .example("update-to-main follow", "as an account's service")
                .option("--config FILE", "the system's, unless given")
                .option("--every SECONDS", "asks at least this often, 600")
                .exit(
                    Exit::Failed,
                    "a newer update-to-main couldn't start in its place",
                ),
        )
        .subcommand(
            Help::new("job", "runs a command in a Mac's login session")
                .example("update-to-main job FOLDER", "as launchd starts it")
                .argument("<folder>", "the command, and where it ends")
                .exit(Exit::Failed, "its command didn't succeed")
                .unlisted(),
        )
}

/// The words a command was given that it doesn't take.
fn unasked(command: &str) -> Failure {
    Failure::new(
        Exit::Usage,
        format!("update-to-main {command} doesn't take those words"),
    )
    .try_this(format!("update-to-main {command} --help"))
}

/// Why the daemon couldn't be asked, as the kit says a failure.
fn unanswered(socket: &Path, e: &std::io::Error) -> Failure {
    let failure = Failure::new(Exit::Unavailable, "the daemon couldn't be asked");
    match ask::unanswered(socket, e) {
        Line::Refused { why, .. } => failure.because(why),
        _ => failure.because(e.to_string()),
    }
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        run(&context.args)
    })
}

/// The command these words name, run.
fn run(args: &[String]) -> Result<(), Failure> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["serve", "--config", file] => {
            let config = Config::read(&PathBuf::from(file)).map_err(|e| {
                Failure::new(Exit::Config, "its config couldn't be read").because(e)
            })?;
            serve::serve(config)
                .map_err(|e| Failure::new(Exit::Failed, "it couldn't serve").because(e))
        }
        ["ask"] => asked(Path::new(SOCKET), None),
        ["ask", "--socket", file] => asked(Path::new(file), None),
        ["ask", "--at", at] => asked(Path::new(SOCKET), Some(at_args(at)?)),
        ["ask", "--at", at, "--socket", file] | ["ask", "--socket", file, "--at", at] => {
            asked(Path::new(file), Some(at_args(at)?))
        }
        ["stop"] => stopped(Path::new(SOCKET)),
        ["stop", "--socket", file] => stopped(Path::new(file)),
        [how @ ("pause" | "resume")] => held(Path::new(SOCKET), how),
        [how @ ("pause" | "resume"), "--socket", file] => held(Path::new(file), how),
        [how @ ("hold" | "lift")] | [how @ ("hold" | "lift"), "--socket", _] => Err(Failure::new(
            Exit::Usage,
            format!("update-to-main {how} needs a commit"),
        )
        .try_this(format!("update-to-main {how} --help"))),
        [how @ ("hold" | "lift"), commit] => held(Path::new(SOCKET), &format!("{how} {commit}")),
        [how @ ("hold" | "lift"), commit, "--socket", file] => {
            held(Path::new(file), &format!("{how} {commit}"))
        }
        ["follow", rest @ ..] => {
            let (config, every) = follow_args(rest).ok_or_else(|| unasked("follow"))?;
            followed(&config, Duration::from_secs(every))
        }
        ["job", folder] => match session::job(Path::new(folder)) {
            0 => Ok(()),
            status => Err(Failure::new(
                Exit::Failed,
                format!("the job's command ended {status}"),
            )),
        },
        [command, ..] => Err(unasked(command)),
        [] => Err(Failure::new(Exit::Usage, "update-to-main needs a command")
            .try_this("update-to-main --help")),
    }
}

fn follow_args(mut rest: &[&str]) -> Option<(PathBuf, u64)> {
    let (mut config, mut every) = (PathBuf::from(CONFIG), FOLLOW_EVERY);
    loop {
        match rest {
            [] => return Some((config, every)),
            ["--config", file, more @ ..] => (config, rest) = (PathBuf::from(file), more),
            ["--every", seconds, more @ ..] => (every, rest) = (seconds.parse().ok()?, more),
            _ => return None,
        }
    }
}

/// Follows main until the system's path holds another update-to-main,
/// which it then starts in its place. The config is read again each time,
/// since a switch may change it, and one it can't read is waited out
/// rather than ended on: a switch writes a newer update-to-main's config
/// before the system's path holds it, and a follower that ended there
/// waited 30 seconds to start again, long enough for the switch to stop
/// it for good (switch 1036 on 18 November).
fn followed(file: &Path, every: Duration) -> Result<(), Failure> {
    let started = std::env::current_exe().and_then(fs::canonicalize).ok();
    let mut follower = Follower::default();
    let mut unreadable = None;
    let mut unread = None;
    let mut unanswered = None;
    let mut seen = None;
    loop {
        let config = match Config::read(file) {
            Ok(config) => {
                unreadable = None;
                config
            }
            Err(e) => {
                if let Some(program) =
                    follow::newer(Config::program_in(file).as_deref(), started.as_deref())
                {
                    return started_in_place(&program);
                }
                // Said once until it changes, since a switch may take a
                // while to give the system its newer update-to-main.
                if unreadable.as_ref() != Some(&e) {
                    eprintln!("update-to-main: its config couldn't be read, so it waits: {e}");
                    unreadable = Some(e);
                }
                thread::sleep(follow::LOOK_EVERY);
                continue;
            }
        };
        if let Some(program) = follow::newer(config.program.as_deref(), started.as_deref()) {
            return started_in_place(&program);
        }
        match follow::heads(&config, seen.as_ref()) {
            Ok(heads) => {
                unread = None;
                seen = Some(heads.clone());
                let ended = fs::read_to_string(&config.ended).unwrap_or_default();
                if let Some(at) = follow::profile_changed(&config.profile, Instant::now()) {
                    follower.saw_switch(at);
                }
                follower.saw(follow::someone_here(), Instant::now());
                if follower.should_ask(&heads, &ended, Instant::now(), every) {
                    let at: Vec<String> = heads
                        .iter()
                        .map(|(name, commit)| format!("{name}@{}", &commit[..commit.len().min(12)]))
                        .collect();
                    // Stamped, since a Mac's log has no time of its own.
                    println!(
                        "{}",
                        update_to_main::stamped(&format!("asking at {}", at.join(" ")))
                    );
                    // Said like an answer's own ending, since otherwise an
                    // ask that never reached the daemon leaves no trace;
                    // once until it changes, since it's asked every minute.
                    let ending = match heard(&config, &heads) {
                        Ok(ending) => {
                            unanswered = None;
                            ending
                        }
                        Err(e) => {
                            let line = ask::unanswered(&config.socket, &e);
                            if unanswered.as_ref() != Some(&line) {
                                println!("{}", line.json());
                                unanswered = Some(line.clone());
                            }
                            line
                        }
                    };
                    let ended = fs::read_to_string(&config.ended).unwrap_or_default();
                    if let Some(why) = follower.asked(heads, &ending, &ended, Instant::now()) {
                        println!("stopped until someone asks by hand: {why}");
                    }
                    if let Some(why) = follower.to_tell() {
                        told(&config, &why);
                    }
                }
            }
            // Said once until it changes, since the host may be away a while.
            Err(why) if unread.as_ref() != Some(&why) => {
                eprintln!("update-to-main: {why}");
                unread = Some(why);
            }
            Err(_) => {}
        }
        thread::sleep(follow::LOOK_EVERY);
    }
}

/// Starts the system's newer update-to-main in this one's place, with the
/// same arguments and in the same process, so its service never stands
/// stopped between them for a switch to leave stopped. It returns only
/// when that couldn't be done, as a failure its service starts the newest
/// again after.
fn started_in_place(program: &Path) -> Result<(), Failure> {
    println!("the system holds another update-to-main; starting it in this one's place");
    let e = Command::new(program)
        .args(std::env::args_os().skip(1))
        .exec();
    Err(Failure::new(
        Exit::Failed,
        "the newer update-to-main couldn't start in its place",
    )
    .because(format!("{}: {e}", program.display())))
}

/// Asks for an update at `heads` and hears it to its ending, looking at
/// main meanwhile and stopping the update once it has long fetched heads
/// main has moved past (`follow::stale`), so the next ask fetches the new.
fn heard(config: &Config, heads: &Followed) -> std::io::Result<Line> {
    let (told, lines) = mpsc::channel::<Line>();
    let socket = config.socket.clone();
    let asking = thread::spawn(move || {
        ask::follow(&socket, &mut |line| {
            println!("{}", line.json());
            let _ = told.send(line.clone());
        })
    });
    let (mut latest, mut since, mut stopped) = (None, Instant::now(), false);
    loop {
        match lines.recv_timeout(follow::LOOK_EVERY) {
            Ok(line) => {
                if latest.as_ref() != Some(&line) {
                    since = Instant::now();
                }
                latest = Some(line);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if stopped {
                    continue;
                }
                let Ok(now) = follow::heads(config, Some(heads)) else {
                    continue;
                };
                if follow::stale(heads, &now, latest.as_ref(), since.elapsed()) {
                    println!("main moved past the heads this update builds; stopping it");
                    match ask::stop_for_newer(&config.socket) {
                        Ok(line) => println!("{}", line.json()),
                        Err(e) => eprintln!("update-to-main: stopping the update: {e}"),
                    }
                    stopped = true;
                }
            }
        }
    }
    asking
        .join()
        .unwrap_or_else(|_| Err(std::io::Error::other("the ask's thread panicked")))
}

/// Tells the person why the machine can't update, with the command the
/// module gives, and says in the log when that couldn't be done.
fn told(config: &Config, why: &str) {
    let Some([program, args @ ..]) = config.follower_tells.as_deref() else {
        return;
    };
    match Command::new(program)
        .args(args)
        .arg(why)
        .stdin(Stdio::null())
        .status()
    {
        Ok(status) if status.success() => println!("told: {why}"),
        Ok(status) => eprintln!("update-to-main: telling that it can't update ended {status}"),
        Err(e) => eprintln!("update-to-main: telling that it can't update: {e}"),
    }
}

fn stopped(socket: &Path) -> Result<(), Failure> {
    let line = ask::stop(socket).map_err(|e| unanswered(socket, &e))?;
    println!("{}", line.json());
    Ok(())
}

fn held(socket: &Path, how: &str) -> Result<(), Failure> {
    match ask::once(socket, how) {
        Ok(Line::Refused { why, .. }) if update_to_main::held::says(&why) => {
            println!("{why}");
            Ok(())
        }
        Ok(line) => {
            Err(Failure::new(Exit::Refused, "the daemon refused the ask").because(line.json()))
        }
        Err(e) => Err(unanswered(socket, &e)),
    }
}

/// The input and commit `--at` names, as INPUT=COMMIT.
fn at_args(at: &str) -> Result<(&str, &str), Failure> {
    at.split_once('=')
        .filter(|(input, commit)| !input.is_empty() && !commit.is_empty())
        .ok_or_else(|| {
            Failure::new(Exit::Usage, format!("--at takes INPUT=COMMIT, not {at}"))
                .try_this("update-to-main ask --help")
        })
}

fn asked(socket: &Path, at: Option<(&str, &str)>) -> Result<(), Failure> {
    let hear = &mut |line: &Line| println!("{}", line.json());
    let ending = match at {
        Some((input, commit)) => ask::ask_at(socket, input, commit, hear),
        None => ask::ask(socket, hear),
    };
    // Each line, the ending's too, is already said on stdout.
    match ending {
        Ok(Line::Done { .. } | Line::Unchanged { .. }) => Ok(()),
        Ok(Line::Failed { .. }) => Err(Failure::new(Exit::Failed, "the update failed")),
        Ok(Line::Refused { .. }) => Err(Failure::new(Exit::Refused, "the update was refused")),
        Ok(_) => Err(Failure::new(
            Exit::Unavailable,
            "the daemon stopped answering before the update ended",
        )),
        Err(e) => Err(unanswered(socket, &e)),
    }
}
