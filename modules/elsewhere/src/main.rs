//! Runs an agent's heavy job on whichever machine of the realm mycelium
//! places it, so one laptop doesn't carry the realm's work alone.
//!
//! On 15 November one machine's processor stayed over for hours while
//! another in its realm had room: agents' commands, test suites and model
//! checks, took 5.3 of its cores, and its builds a quarter of one. Builds
//! already go elsewhere through nix-offload; this is the same for the
//! commands that aren't builds.
//!
//! A job is sent only by its kind, opted in by name in the settings
//! (src/settings.rs): a kind names the programs its jobs may start, a
//! setup that puts them on the far machine's path, and what's brought
//! back. `elsewhere run KIND -- COMMAND` asks `mycelium place` where the
//! kind's work fits (src/room.rs), so one score weighs every machine's
//! load, and either becomes the command here or sends it by mycelium
//! alone (src/far.rs): its tracked files through the quest's handover,
//! the job through `mycelium run`, which fences an agent's run where it
//! lands. Its output and its code pass through untouched, so a caller
//! can't tell where it ran but by the line it says on stderr. When to run
//! a job is its caller's business, such as night-shift's turn; where is
//! this one's.
//!
//! A job that can't go runs here, saying why: with no quest to carry it,
//! outside a repository, past what a shared folder carries, or where
//! mycelium doesn't run it far, as it refuses an agent's run on a machine
//! that declares no way to fence one. Interrupted, `mycelium run` stops
//! the far job too.

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
use terminal_kit::{Context, Exit, Failure, flags};

fn main() -> ExitCode {
    terminal_kit::main(&commands::help(), env!("CARGO_PKG_VERSION"), |context| {
        let args = &context.args;
        match args.first().map(String::as_str) {
            None | Some("kinds" | "kind") if args.len() <= 1 => kinds(context, &settings()?),
            None | Some("kinds" | "kind") => {
                Err(commands::wrongly(args, "kinds takes no more words"))
            }
            Some("where") if args.len() == 2 => {
                let settings = settings()?;
                let kind = named(&settings, &args[1])?;
                place_view(
                    context,
                    &args[1],
                    kind,
                    &place(&settings, kind, false, false),
                )
            }
            Some("where") => Err(commands::wrongly(args, "where takes one kind")),
            Some("run") => run(context, &settings()?),
            Some(other) => Err(flags::unknown("command", other, &["run", "where", "kinds"])
                .try_this("elsewhere --help")),
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

/// Where a job of `kind` goes now: here when told to `stay`, the first
/// machine when told to `send`, otherwise wherever mycelium places it.
fn place(settings: &Settings, kind: &Kind, stay: bool, send: bool) -> Place {
    let told = |machine: Option<&String>| Place {
        machine: machine.cloned(),
        why: Why::Told,
    };
    if stay {
        return told(None);
    }
    // What it made couldn't come home, so it's made here, where it lands
    // as it's made, rather than refused.
    if let Some(odd) = kind.odd_brings_back() {
        return Place {
            machine: None,
            why: Why::CantComeHome(odd.to_owned()),
        };
    }
    if send && let Some(first) = settings.machines.first() {
        return told(Some(first));
    }
    let mut ask = Command::new(&settings.mycelium);
    ask.args(room::asking(&kind.needs, kind.lasts.as_deref()));
    if let Some(realm) = &settings.realm {
        ask.args(["--realm", realm]);
    }
    let heard = ask
        .output()
        .map_err(|error| format!("{} didn't start: {error}", settings.mycelium))
        .and_then(|output| room::placement(&String::from_utf8_lossy(&output.stdout)));
    match heard {
        Ok(placement) => room::decide(&here(), &placement, &settings.machines),
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
        Why::PlacedHere => format!("mycelium placed it on {here}"),
        Why::Placed => match &place.machine {
            Some(machine) => format!("mycelium placed it on {machine}"),
            None => "mycelium placed it".to_owned(),
        },
        Why::NotAllowed(machine) => {
            format!("mycelium placed it on {machine}, which it may not send to")
        }
        Why::Refused(misses) => format!("mycelium fits it nowhere: {misses}"),
        Why::NoMachines => "it may send to no machine".to_owned(),
        Why::NotHeard(problem) => format!("mycelium couldn't say: {problem}"),
        Why::Told => "as told".to_owned(),
        Why::CantComeHome(path) => {
            format!("its brings_back names {path}, which isn't a plain place in the job's folder")
        }
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

fn parse(args: &[String]) -> Result<Asked, Failure> {
    let wrongly = |problem: &str| commands::wrongly(args, problem);
    let split = args
        .iter()
        .position(|arg| arg == "--")
        .ok_or_else(|| wrongly("elsewhere run takes the job's command after --"))?;
    let command = args[split + 1..].to_vec();
    if command.is_empty() {
        return Err(wrongly("no command after --"));
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
            option if option.starts_with('-') => {
                return Err(flags::unknown("option", option, &["--here", "--there"])
                    .try_this("elsewhere run --help"));
            }
            kind if asked.kind.is_empty() => kind.clone_into(&mut asked.kind),
            extra => {
                return Err(wrongly(&format!(
                    "run takes one kind, not {} and {extra}",
                    asked.kind
                )));
            }
        }
    }
    if asked.kind.is_empty() {
        return Err(wrongly("run takes the job's kind before --"));
    }
    if asked.here && asked.there {
        return Err(wrongly("run takes --here or --there, not both"));
    }
    Ok(asked)
}

fn run(context: &Context, settings: &Settings) -> Result<(), Failure> {
    let args = &context.args;
    let asked = parse(args)?;
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
    // Told to keep it off this machine, it isn't made here instead.
    if asked.there
        && let Some(odd) = kind.odd_brings_back()
    {
        return Err(Failure::new(
            Exit::Config,
            format!(
                "{} can't be sent: what it made couldn't come home",
                asked.kind
            ),
        )
        .because(format!(
            "its brings_back names {odd}, which isn't a plain place in the job's folder"
        ))
        .try_this("the kind's brings_back, or --here"));
    }
    let place = place(settings, kind, asked.here, asked.there);
    let Some(machine) = &place.machine else {
        if place.why != Why::Told {
            eprintln!("{} runs here: {}", asked.kind, because(&place));
        }
        return Err(here_now(&asked.command));
    };
    eprintln!("{} runs on {machine}: {}", asked.kind, because(&place));
    match there(settings, kind, &asked, machine)? {
        Sent::Ended(0) => Ok(()),
        Sent::Ended(code) => Err(Failure::passed(
            code,
            format!("{} ended {code} on {machine}", asked.command[0]),
        )),
        // Told to send it, it isn't made here instead.
        Sent::Stayed(exit, why) if asked.there => Err(Failure::new(
            exit,
            format!("{} couldn't be sent to {machine}", asked.kind),
        )
        .because(why)
        .try_this("--here, to make it here")),
        Sent::Stayed(_, why) => {
            eprintln!("{} runs here: {why}", asked.kind);
            Err(here_now(&asked.command))
        }
    }
}

/// Becomes the job here, or says why it couldn't.
fn here_now(command: &[String]) -> Failure {
    let error = Command::new(&command[0]).args(&command[1..]).exec();
    Failure::new(Exit::Failed, format!("can't start {}", command[0])).because(error.to_string())
}

/// How a job sent away ended: with its code there, or kept here, why, and
/// how a job told to go ends for it: unavailable only when mycelium
/// didn't carry it, since another try meets anything else again.
enum Sent {
    Ended(u8),
    Stayed(Exit, String),
}

/// Sends the job to `machine` by the quest's handover and `mycelium run`,
/// or says why it stays.
fn there(settings: &Settings, kind: &Kind, asked: &Asked, machine: &str) -> Result<Sent, Failure> {
    let Some(quest) = std::env::var("PI_QUEST")
        .ok()
        .filter(|quest| far::plain_quest(quest))
    else {
        return Ok(Sent::Stayed(
            Exit::Config,
            "no quest's handover is here to carry it".to_owned(),
        ));
    };
    let folder = std::env::current_dir().map_err(|error| {
        Failure::new(Exit::Failed, "can't tell which folder it's in").because(error.to_string())
    })?;
    let gathered = far::root(kind, &folder)
        .and_then(|root| far::gathered(&root, kind).map(|files| (root, files)));
    let (root, files) = match gathered {
        Ok(gathered) => gathered,
        Err(why) => return Ok(Sent::Stayed(Exit::Refused, why)),
    };
    let within = folder
        .strip_prefix(&root)
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let id = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos())
    );
    let job = Job {
        handover: far::scratches().join(&quest).join(far::HANDOVER).join(&id),
        quest,
        id,
        root,
        files,
        within,
        folder,
    };
    let sent = send(settings, kind, asked, machine, &job);
    // Whatever's left is spent once the job ends here; one that won't go
    // is no reason to fail a job that ran.
    let _ = fs::remove_dir_all(&job.handover);
    sent
}

/// A job on its way: the quest carrying it, its id there, the tree its
/// files are from and its folder within it, and where in the quest's
/// handover it's left.
struct Job {
    quest: String,
    id: String,
    root: PathBuf,
    files: Vec<(String, u64)>,
    within: PathBuf,
    folder: PathBuf,
    handover: PathBuf,
}

/// Leaves the job in the handover, shares the quest, runs the job by
/// mycelium and brings back what it made.
fn send(
    settings: &Settings,
    kind: &Kind,
    asked: &Asked,
    machine: &str,
    job: &Job,
) -> Result<Sent, Failure> {
    let (quest, id) = (job.quest.as_str(), job.id.as_str());
    let paths = job.files.iter().map(|(path, _)| path.as_str());
    far::left(&job.root, paths, &job.handover).map_err(|error| {
        Failure::new(
            Exit::Failed,
            "couldn't leave the job in the quest's handover",
        )
        .because(error.to_string())
    })?;
    // Shared to the machine and waited for where both ends' mycelium can;
    // otherwise shared, and the far line waits for the files to cross.
    let shared = mycelium(settings, &["share", quest, "--to", machine, "--wait"])
        || mycelium(settings, &["share", quest]);
    if !shared {
        return Ok(Sent::Stayed(
            Exit::Unavailable,
            format!("mycelium couldn't share {quest}"),
        ));
    }
    let script = far::script(
        quest,
        id,
        &settings.scratch,
        &job.within,
        kind,
        &asked.command,
    );
    let need = format!("machine={machine}");
    let mut run = Command::new(&settings.mycelium);
    run.args(["run", "--need", &need]);
    if let Some(realm) = &settings.realm {
        run.args(["--realm", realm]);
    }
    let child = run
        .args(["--", "sh", "-c", &script])
        .stdout(std::process::Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(error) => {
            return Ok(Sent::Stayed(
                Exit::Unavailable,
                format!("{} didn't start: {error}", settings.mycelium),
            ));
        }
    };
    let started = child.stdout.take().is_some_and(|out| far::relay(out, id));
    let status = child.wait().map_err(|error| {
        Failure::new(Exit::Failed, "lost the far job").because(error.to_string())
    })?;
    let code = ended(status);
    if !started {
        let (exit, why) = far::unstarted(code);
        return Ok(Sent::Stayed(
            exit,
            format!("{machine} {why} before it started, ending {code}"),
        ));
    }
    let brought = far::bring_back(
        &job.handover,
        &job.folder,
        &kind.brings_back,
        far::CROSSES_WITHIN,
    );
    let Err(missed) = brought else {
        return Ok(Sent::Ended(code));
    };
    if code != 0 {
        eprintln!(
            "couldn't bring back what it made on {machine}: {}",
            missed.why()
        );
        return Ok(Sent::Ended(code));
    }
    // The job passed but its work never reached the folder, so it isn't
    // done here; made here, it would be. A refusal would refuse it again,
    // and what couldn't land would meet the same folder, so only a loss
    // is worth another try.
    let failure = match missed {
        far::Missed::Refused(why) => Failure::new(
            Exit::Refused,
            format!("what it made on {machine} was refused as it came home"),
        )
        .because(why),
        // Not 1, which a job's own failure passes through as, so a
        // passing job whose work couldn't land isn't read as failing.
        far::Missed::Unlanded(why) => Failure::new(
            Exit::Refused,
            format!("what it made on {machine} couldn't land here"),
        )
        .because(why),
        far::Missed::Lost(why) => Failure::new(
            Exit::TryAgain,
            format!("what it made on {machine} didn't come home"),
        )
        .because(why),
    };
    Err(failure.try_this("--here, to make it here"))
}

/// Whether mycelium, asked `words`, says yes.
fn mycelium(settings: &Settings, words: &[&str]) -> bool {
    let mut ask = Command::new(&settings.mycelium);
    ask.args(words);
    if let Some(realm) = &settings.realm {
        ask.args(["--realm", realm]);
    }
    ask.stdout(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// The code a process ended with, or 128 and the signal that ended it.
fn ended(status: std::process::ExitStatus) -> u8 {
    use std::os::unix::process::ExitStatusExt;
    match (status.code(), status.signal()) {
        (Some(code), _) => u8::try_from(code).unwrap_or(1),
        (None, signal) => 128u8.saturating_add(
            signal
                .and_then(|signal| u8::try_from(signal).ok())
                .unwrap_or(0),
        ),
    }
}
