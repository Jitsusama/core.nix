//! Runs an agent's heavy job on another machine of the realm while this
//! one is crowded, so one laptop doesn't carry the realm's work alone.
//!
//! On 15 November one machine's processor stayed over for hours while
//! another in its realm had room: agents' commands, test suites and model
//! checks, took 5.3 of its cores, and its builds a quarter of one. Builds
//! already go elsewhere through nix-offload; this is the same for the
//! commands that aren't builds.
//!
//! A job is sent only by its kind, opted in by name in the settings
//! (src/settings.rs): a kind names the programs its jobs may start, a
//! setup that puts them on the far machine's path, and what's copied
//! there and back. `elsewhere run KIND -- COMMAND` asks mycelium where the
//! room is (src/room.rs) and either becomes the command here or copies its
//! tree to the far machine and runs it there (src/far.rs), passing its
//! output and its code through untouched, so a caller can't tell where it
//! ran but by the line it says on stderr. When to run a job is its
//! caller's business, such as night-shift's turn; where is this one's.
//!
//! A job ssh can't reach the machine for ends 69, saying how to run it
//! here; one that exits 255 itself reads the same, since that's ssh's own
//! code. Interrupted, the far job is left to end when it next writes to
//! the link ssh closed.

mod commands;
mod far;
mod room;
mod settings;

use room::{Place, Why};
use serde_json::json;
use settings::{Kind, Settings};
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use terminal_kit::layout::{self, Row};
use terminal_kit::{Context, Exit, Failure};

fn main() -> ExitCode {
    terminal_kit::main(&commands::help(), env!("CARGO_PKG_VERSION"), |context| {
        let args = &context.args;
        match args.first().map(String::as_str) {
            None | Some("kinds") if args.len() <= 1 => kinds(context, &settings()?),
            Some("where") if args.len() == 2 => {
                let settings = settings()?;
                let kind = named(&settings, &args[1])?;
                place_view(context, &args[1], kind, &place(&settings, false, false))
            }
            Some("run") => run(context, &settings()?),
            _ => Err(commands::wrongly(
                args,
                "elsewhere takes run, where or kinds",
            )),
        }
    })
}

/// The settings, or none where no file is written yet.
fn settings() -> Result<Settings, Failure> {
    let var = |name| std::env::var(name).ok();
    let Some(file) = settings::file(var("XDG_CONFIG_HOME").as_deref(), var("HOME").as_deref())
    else {
        return Ok(Settings::default());
    };
    match fs::read_to_string(&file) {
        Ok(text) => settings::read(&text).map_err(|problem| {
            Failure::new(Exit::Config, format!("can't read {}", file.display()))
                .because(problem)
                .try_this("jitsusama.elsewhere in the machine's home-manager settings")
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
        Err(error) => Err(
            Failure::new(Exit::Config, format!("can't read {}", file.display()))
                .because(error.to_string()),
        ),
    }
}

fn named<'a>(settings: &'a Settings, name: &str) -> Result<&'a Kind, Failure> {
    settings.kinds.get(name).ok_or_else(|| {
        Failure::new(Exit::NotFound, format!("no kind of job is called {name}"))
            .because(if settings.kinds.is_empty() {
                "the settings name none".to_owned()
            } else {
                format!("the settings name {}", names(settings))
            })
            .try_this("elsewhere kinds")
    })
}

fn names(settings: &Settings) -> String {
    settings
        .kinds
        .keys()
        .cloned()
        .collect::<Vec<_>>()
        .join(", ")
}

/// This machine's name, as mycelium calls it: the host's, short.
fn here() -> String {
    // std links libc, so this comes from it.
    unsafe extern "C" {
        fn gethostname(name: *mut u8, len: usize) -> i32;
    }
    let mut buffer = [0u8; 256];
    // SAFETY: the buffer is as long as the length given, and gethostname
    // writes no further.
    if unsafe { gethostname(buffer.as_mut_ptr(), buffer.len()) } != 0 {
        return String::new();
    }
    let end = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
    let name = String::from_utf8_lossy(&buffer[..end]);
    name.split('.').next().unwrap_or_default().to_owned()
}

/// Where a job goes now: here when told to `stay`, the first machine when
/// told to `send`, otherwise wherever mycelium's machines say.
fn place(settings: &Settings, stay: bool, send: bool) -> Place {
    let told = |machine: Option<&String>| Place {
        machine: machine.cloned(),
        why: Why::Told,
    };
    if stay {
        return told(None);
    }
    if send && let Some(first) = settings.machines.first() {
        return told(Some(first));
    }
    let mut ask = Command::new(&settings.mycelium);
    ask.arg("machines");
    if let Some(realm) = &settings.realm {
        ask.args(["--realm", realm]);
    }
    let heard = ask
        .output()
        .map_err(|error| format!("{} didn't start: {error}", settings.mycelium))
        .and_then(|output| room::machines(&String::from_utf8_lossy(&output.stdout)));
    match heard {
        Ok(machines) => room::decide(&here(), &machines, &settings.machines),
        Err(problem) => Place {
            machine: None,
            why: Why::NotHeard(problem),
        },
    }
}

/// Why a job goes where it goes, in words.
fn because(place: &Place) -> String {
    let here = here();
    match &place.why {
        Why::Room => format!("{here} has room"),
        Why::Crowded => format!("{here} is crowded"),
        Why::NoneHasRoom => format!("{here} is crowded, and so is every machine it may send to"),
        Why::NoMachines => "it may send to no machine".to_owned(),
        Why::NotHeard(problem) => format!("mycelium couldn't say: {problem}"),
        Why::Told => "as told".to_owned(),
    }
}

fn kinds(context: &Context, settings: &Settings) -> Result<(), Failure> {
    let answer = json!({
        "schema": "elsewhere.kinds/1",
        "machines": settings.machines,
        "kinds": settings.kinds.iter().map(|(name, kind)| json!({
            "name": name,
            "description": kind.description,
            "programs": kind.programs,
        })).collect::<Vec<_>>(),
    });
    if context.format().is_json() {
        return context.answer(&answer).map_err(|error| written(&error));
    }
    if settings.kinds.is_empty() {
        println!("{}", context.out.voice("no kind of job is opted in"));
        return Ok(());
    }
    let rows: Vec<Row> = settings
        .kinds
        .iter()
        .map(|(name, kind)| {
            Row::named(name)
                .holding(&kind.description)
                .side(kind.programs.join(" · "))
        })
        .collect();
    let mut lines = vec![context.out.title(&format!(
        "sent to {}",
        if settings.machines.is_empty() {
            "no machine".to_owned()
        } else {
            settings.machines.join(" · ")
        }
    ))];
    lines.push(String::new());
    lines.extend(layout::list(&rows, &context.out, context.room));
    println!("{}", lines.join("\n"));
    Ok(())
}

fn place_view(context: &Context, name: &str, kind: &Kind, place: &Place) -> Result<(), Failure> {
    let answer = json!({
        "schema": "elsewhere.where/1",
        "kind": name,
        "machine": place.machine,
        "here": place.machine.is_none(),
        "because": place.why.word(),
        "programs": kind.programs,
    });
    if context.format().is_json() {
        return context.answer(&answer).map_err(|error| written(&error));
    }
    let goes = place.machine.clone().unwrap_or_else(|| "here".to_owned());
    let row = Row::named(name).holding(because(place)).side(goes);
    println!(
        "{}",
        layout::list(&[row], &context.out, context.room).join("\n")
    );
    Ok(())
}

fn written(error: &std::io::Error) -> Failure {
    Failure::new(Exit::Failed, "couldn't write the answer").because(error.to_string())
}

/// What `run` was asked.
#[derive(Debug, PartialEq)]
struct Asked {
    kind: String,
    here: bool,
    there: bool,
    command: Vec<String>,
}

fn parse(args: &[String]) -> Result<Asked, String> {
    let split = args
        .iter()
        .position(|arg| arg == "--")
        .ok_or("elsewhere run takes the job's command after --")?;
    let command = args[split + 1..].to_vec();
    if command.is_empty() {
        return Err("the job's command is empty".into());
    }
    let mut asked = Asked {
        kind: String::new(),
        here: false,
        there: false,
        command,
    };
    for word in &args[1..split] {
        match word.as_str() {
            "--here" => asked.here = true,
            "--there" => asked.there = true,
            option if option.starts_with('-') => return Err(format!("run takes no {option}")),
            kind if asked.kind.is_empty() => kind.clone_into(&mut asked.kind),
            extra => {
                return Err(format!(
                    "run takes one kind, not {} and {extra}",
                    asked.kind
                ));
            }
        }
    }
    if asked.kind.is_empty() {
        return Err("run takes the job's kind before --".into());
    }
    if asked.here && asked.there {
        return Err("run takes --here or --there, not both".into());
    }
    Ok(asked)
}

fn run(context: &Context, settings: &Settings) -> Result<(), Failure> {
    let args = &context.args;
    let asked = parse(args).map_err(|problem| commands::wrongly(args, problem))?;
    let kind = named(settings, &asked.kind)?;
    let program = Path::new(&asked.command[0])
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    if !kind.programs.contains(&program) {
        return Err(Failure::new(
            Exit::Refused,
            format!("{} jobs don't start {program}", asked.kind),
        )
        .because(format!(
            "its settings let it start {}",
            kind.programs.join(", ")
        )));
    }
    if asked.there && settings.machines.is_empty() {
        return Err(Failure::new(Exit::Config, "there's no machine to send to")
            .try_this("jitsusama.elsewhere.machines in the machine's home-manager settings"));
    }
    let place = place(settings, asked.here, asked.there);
    let Some(machine) = &place.machine else {
        if place.why != Why::Told {
            eprintln!("{} runs here: {}", asked.kind, because(&place));
        }
        let error = Command::new(&asked.command[0])
            .args(&asked.command[1..])
            .exec();
        return Err(
            Failure::new(Exit::Failed, format!("can't start {}", asked.command[0]))
                .because(error.to_string()),
        );
    };
    eprintln!("{} runs on {machine}: {}", asked.kind, because(&place));
    there(settings, kind, &asked, machine)
}

fn there(settings: &Settings, kind: &Kind, asked: &Asked, machine: &str) -> Result<(), Failure> {
    let folder = std::env::current_dir().map_err(|error| {
        Failure::new(Exit::Failed, "can't tell which folder it's in").because(error.to_string())
    })?;
    let root = far::root(kind, &folder).map_err(|problem| {
        Failure::new(Exit::Failed, "can't tell which tree to copy").because(problem)
    })?;
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let copy = far::copy(&root, &folder, &home, &settings.scratch);
    let unreached = |what: &str| {
        Failure::new(
            Exit::Unavailable,
            format!("couldn't reach {machine} to {what}"),
        )
        .because("ssh couldn't connect or log in")
        .try_this(format!(
            "elsewhere run {} --here -- {}",
            asked.kind,
            words(&asked.command)
        ))
    };
    let failed = |problem: String| {
        Failure::new(Exit::Failed, format!("couldn't copy to {machine}")).because(problem)
    };
    match far::step(&settings.rsync, &far::sending(&copy, kind, machine)).map_err(failed)? {
        far::Ended::Done => {}
        far::Ended::Unreached => return Err(unreached("copy the job's tree")),
        far::Ended::Code(code) => return Err(failed(format!("rsync ended {code}"))),
    }
    let line = far::line(&copy, kind, &asked.command);
    let ended = far::step(&settings.ssh, &far::running(machine, &line))
        .map_err(|problem| Failure::new(Exit::Failed, "the job didn't start").because(problem))?;
    if ended == far::Ended::Unreached {
        return Err(unreached("run the job"));
    }
    for path in &kind.brings_back {
        let (args, into) = far::bringing(&copy, path, machine);
        let brought = fs::create_dir_all(&into)
            .map_err(|error| error.to_string())
            .and_then(|()| far::step(&settings.rsync, &args));
        if brought != Ok(far::Ended::Done) {
            eprintln!("couldn't bring {path} back from {machine}");
        }
    }
    match ended {
        far::Ended::Code(code) => Err(Failure::passed(
            code,
            format!("{} ended {code} on {machine}", asked.command[0]),
        )),
        _ => Ok(()),
    }
}

fn words(command: &[String]) -> String {
    command
        .iter()
        .map(|word| far::quoted(word))
        .collect::<Vec<_>>()
        .join(" ")
}
