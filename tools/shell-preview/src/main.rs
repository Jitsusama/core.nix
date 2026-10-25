//! Shows the desktop's shell from the working copy, in a niri window of its
//! own.
//!
//! Usage: `shell-preview [--type TEXT] [--check NAME] [--account NAME]`
//!
//! It builds the shell's home from a check's machine, the desktop test's by
//! default, so the launcher lists that machine's stand-in programs rather
//! than the session's own. Its Quickshell folder is laid out afresh: each
//! file that is a tracked file of the working copy is linked to it, so an
//! edit shows as Quickshell reloads, and each the build made, the theme
//! among them, is copied. niri then starts nested in the session's
//! compositor, running that Quickshell with a runtime folder, a D-Bus
//! session and state of its own, so nothing reaches the session's own
//! shell. With `--type`, the launcher opens and the text is typed into it,
//! to see the rows a query gives. Closing the window ends it, as do Ctrl-C
//! and a stop, which end all it started. A change to what Nix generates
//! needs a new preview.
//!
//! git, nix, dbus-run-session and wtype are found on the path: the first
//! two are the person's own, with their settings, and package.nix puts the
//! others there.

mod desktop;
mod layout;

use std::env;
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use desktop::{Desktop, Ended};

/// What a run asked for is refused with, as sysexits.h numbers it.
const USAGE: u8 = 64;
const USED: &str = "Usage: shell-preview [--type TEXT] [--check NAME] [--account NAME]";
/// How long the nested shell may take to answer: it starts cold, drawn in
/// software when the GPU is the session's.
const STARTING: Duration = Duration::from_secs(60);
/// How long the nested desktop may take to end once asked, before it's killed.
const ENDING: Duration = Duration::from_secs(5);
/// The nested D-Bus session's config: a bus anyone in the account may use,
/// with no services to start, so no portal starts and mounts its documents
/// in the runtime folder, which the preview then couldn't take away. The
/// shell serves what it needs itself.
const BUS: &str = concat!(
    r#"<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN""#,
    r#"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <listen>unix:dir=@runtime@</listen>
  <policy context="default">
    <allow send_destination="*" eavesdrop="true"/>
    <allow eavesdrop="true"/>
    <allow own="*"/>
  </policy>
</busconfig>
"#
);

/// What a run was asked for.
struct Options {
    /// What to type into the launcher once it's open, if anything.
    typed: Option<String>,
    /// The check whose machine's shell is shown.
    check: String,
    /// The machine's account whose shell it is.
    account: String,
}

impl Options {
    /// Read the command line's arguments, or say what's wrong with them.
    fn parse(mut arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut options = Self {
            typed: None,
            check: "vm-desktop-starts-clean".to_owned(),
            account: "jitsusama".to_owned(),
        };
        while let Some(argument) = arguments.next() {
            let mut value = || {
                arguments
                    .next()
                    .ok_or_else(|| format!("{argument} needs a value"))
            };
            match argument.as_str() {
                "--type" => options.typed = Some(value()?),
                "--check" => options.check = value()?,
                "--account" => options.account = value()?,
                _ => return Err(format!("{argument} isn't something it takes")),
            }
        }
        Ok(options)
    }
}

/// What a preview takes from a check's machine, each a path in the store.
struct Machine {
    home: PathBuf,
    niri: PathBuf,
    quickshell: PathBuf,
    data: Vec<PathBuf>,
}

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if arguments.iter().any(|argument| argument == "--help") {
        println!("{USED}");
        return ExitCode::SUCCESS;
    }
    let options = match Options::parse(arguments.into_iter()) {
        Ok(options) => options,
        Err(why) => {
            eprintln!("shell-preview: {why}\n{USED}");
            return ExitCode::from(USAGE);
        }
    };
    let (ended, heard) = mpsc::channel();
    if let Err(error) = desktop::hear_stops(ended.clone()) {
        eprintln!("shell-preview: can't hear its stop signals: {error}");
        return ExitCode::FAILURE;
    }
    // Named for this run alone, so what a run cut short left can't refuse it.
    let started = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let own = env::temp_dir().join(format!("shell-preview-{}-{started}", std::process::id()));
    let shown = preview(&options, &own, &ended, &heard);
    let _ = fs::remove_dir_all(&own);
    match shown {
        Ok(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        Err(why) => {
            eprintln!("shell-preview: {why}");
            ExitCode::FAILURE
        }
    }
}

/// Show the shell, in the folder of its own given, until something ends it,
/// and give what the preview exits with.
fn preview(
    options: &Options,
    own: &Path,
    ended: &mpsc::Sender<Ended>,
    heard: &Receiver<Ended>,
) -> Result<i32, String> {
    let failed = |doing: &'static str| move |error: std::io::Error| format!("{doing}: {error}");
    let root = PathBuf::from(run("git", &["rev-parse", "--show-toplevel"])?);
    let system = run(
        "nix",
        &[
            "eval",
            "--raw",
            "--impure",
            "--expr",
            "builtins.currentSystem",
        ],
    )?;
    eprintln!("building the shell's home from the working copy");
    let flake = format!("git+file://{}", root.display());
    let built = machine(&flake, &system, &options.check, &options.account)?;

    fs::DirBuilder::new()
        .mode(0o700)
        .create(own)
        .map_err(failed("making the preview's folder"))?;
    let shell = own.join("quickshell");
    let tracked = run(
        "git",
        &[
            "-C",
            &root.to_string_lossy(),
            "ls-files",
            "-z",
            "*.qml",
            "*.js",
        ],
    )?;
    let tracked: Vec<PathBuf> = tracked
        .split('\0')
        .filter(|name| !name.is_empty())
        .map(PathBuf::from)
        .collect();
    let laid = layout::lay_out(
        &built.home.join(".config/quickshell"),
        &layout::known(&root, &tracked),
        &shell,
    )
    .map_err(failed("laying the shell out"))?;
    eprintln!(
        "{} files follow the working copy, {} were built",
        laid.linked, laid.copied
    );
    let config = layout::niri_config(
        &built.home.join(".config/niri"),
        &own.join("niri"),
        &built.quickshell,
        &shell,
    )
    .map_err(failed("copying niri's config"))?;
    let environment = nested(own, &built.data)?;
    let runtime = &environment["XDG_RUNTIME_DIR"];
    let bus = own.join("bus.conf");
    fs::write(&bus, BUS.replace("@runtime@", runtime))
        .map_err(failed("writing the bus's config"))?;

    let desktop = Desktop::start(
        Command::new("dbus-run-session")
            .arg(format!("--config-file={}", bus.display()))
            .arg("--")
            .arg(built.niri.join("bin/niri"))
            .arg("-c")
            .arg(&config)
            .env_clear()
            .envs(&environment),
        ended.clone(),
    )
    .map_err(failed("starting niri"))?;
    let shown = match &options.typed {
        Some(text) => typed_into(&built.quickshell, &shell, &environment, text, heard),
        None => Ok(None),
    };
    let code = match shown {
        Ok(Some(early)) => Ok(early.code()),
        Ok(None) => Ok(heard.recv().map_or(1, |ended| ended.code())),
        Err(why) => Err(why),
    };
    desktop.end(ENDING);
    code
}

/// Open the nested launcher once its shell answers, and type the text into
/// it on the nested niri's keyboard. Gives what ended the preview first, if
/// something did while it waited.
fn typed_into(
    quickshell: &Path,
    shell: &Path,
    environment: &Environment,
    text: &str,
    heard: &Receiver<Ended>,
) -> Result<Option<Ended>, String> {
    let ask = |what: &str| {
        desktop::unblocked(&mut Command::new(quickshell))
            .args([
                "-p",
                &shell.to_string_lossy(),
                "ipc",
                "call",
                "launcher",
                what,
            ])
            .env_clear()
            .envs(inside(environment))
            .output()
            .map_err(|error| format!("asking the shell: {error}"))
    };
    let deadline = Instant::now() + STARTING;
    loop {
        if let Ok(early) = heard.try_recv() {
            return Ok(Some(early));
        }
        let asked = ask("showing")?;
        if asked.status.success() {
            break;
        }
        if Instant::now() >= deadline {
            let said =
                String::from_utf8_lossy(&asked.stderr) + String::from_utf8_lossy(&asked.stdout);
            return Err(format!("the shell never answered:\n{said}"));
        }
        thread::sleep(Duration::from_secs(1));
    }
    if !ask("toggle")?.status.success() {
        return Err("the shell wouldn't open its launcher".to_owned());
    }
    // The launcher takes the keys a frame after it opens.
    thread::sleep(Duration::from_secs(1));
    let typed = desktop::unblocked(&mut Command::new("wtype"))
        .arg(text)
        .env_clear()
        .envs(inside(environment))
        .status()
        .map_err(|error| format!("running wtype: {error}"))?;
    if !typed.success() {
        return Err(format!("wtype couldn't type {text:?}"));
    }
    eprintln!("typed {text:?} into the launcher");
    Ok(None)
}

/// The environment a nested program runs with, by variable.
type Environment = std::collections::BTreeMap<String, String>;

/// Give the nested niri's environment: the session's display, and all else
/// its own, in a runtime folder it makes.
fn nested(own: &Path, data: &[PathBuf]) -> Result<Environment, String> {
    let session = |name: &str| {
        env::var(name).map_err(|_| format!("{name} isn't set, so there's no session to show it in"))
    };
    let parent = Path::new(&session("XDG_RUNTIME_DIR")?).join(session("WAYLAND_DISPLAY")?);
    let runtime = own.join("runtime");
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&runtime)
        .map_err(|error| format!("making the runtime folder: {error}"))?;
    let shown = |path: &Path| path.to_string_lossy().into_owned();
    let mut environment: Environment = env::vars().collect();
    environment.extend([
        ("XDG_RUNTIME_DIR".to_owned(), shown(&runtime)),
        ("WAYLAND_DISPLAY".to_owned(), shown(&parent)),
        ("XDG_STATE_HOME".to_owned(), shown(&own.join("state"))),
        ("XDG_CACHE_HOME".to_owned(), shown(&own.join("cache"))),
        // Where pi's quests and the like are kept, so the preview shows none
        // of the account's own and the same rows each time.
        ("XDG_DATA_HOME".to_owned(), shown(&own.join("data"))),
        (
            "XDG_DATA_DIRS".to_owned(),
            data.iter()
                .map(|path| shown(path))
                .collect::<Vec<_>>()
                .join(":"),
        ),
    ]);
    Ok(environment)
}

/// Give the environment a program inside the nested niri has, on its
/// display. The shell's IPC finds a shell by the display it runs on, so
/// it's asked from there.
fn inside(environment: &Environment) -> Environment {
    let runtime = Path::new(&environment["XDG_RUNTIME_DIR"]);
    let mut displays: Vec<String> = fs::read_dir(runtime)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| {
            name.starts_with("wayland-") && name.ends_with(|last: char| last.is_ascii_digit())
        })
        .collect();
    displays.sort();
    let mut inside = environment.clone();
    inside.insert(
        "WAYLAND_DISPLAY".to_owned(),
        displays.into_iter().next().unwrap_or_default(),
    );
    inside
}

/// Build what a preview takes from the check's machine.
fn machine(flake: &str, system: &str, check: &str, account: &str) -> Result<Machine, String> {
    let attribute = format!("{flake}#checks.{system}.{check}.nodes.machine");
    let taking = format!(
        "m: let home = m.home-manager.users.{account}; in {{ \
         home = home.home-files.outPath; niri = m.programs.niri.package.outPath; \
         quickshell = builtins.head home.systemd.user.services.quickshell.Service.ExecStart; \
         data = [ m.system.path.outPath home.home.path.outPath ]; }}"
    );
    let found: serde_json::Value = serde_json::from_str(&run(
        "nix",
        &["eval", "--json", &attribute, "--apply", &taking],
    )?)
    .map_err(|error| format!("nix described the machine oddly: {error}"))?;
    let path = |name: &str| {
        found[name]
            .as_str()
            .map(PathBuf::from)
            .ok_or_else(|| format!("nix gave no {name} for the machine"))
    };
    let machine = Machine {
        home: path("home")?,
        niri: path("niri")?,
        quickshell: path("quickshell")?,
        data: found["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(|path| Path::new(path).join("share"))
            .collect(),
    };
    run(
        "nix",
        &[
            "build",
            "--no-link",
            &format!("{attribute}.home-manager.users.{account}.home-files"),
            &format!("{attribute}.home-manager.users.{account}.home.path"),
            &format!("{attribute}.programs.niri.package"),
            &format!("{attribute}.system.path"),
        ],
    )?;
    Ok(machine)
}

/// Run a program on the path, and give what it printed, trimmed; what it
/// says of itself goes to this one's standard error.
fn run(program: &str, arguments: &[&str]) -> Result<String, String> {
    let ran = desktop::unblocked(&mut Command::new(program))
        .args(arguments)
        .stderr(std::process::Stdio::inherit())
        .output()
        .map_err(|error| format!("running {program}: {error}"))?;
    if !ran.status.success() {
        return Err(format!(
            "{program} {} failed: {}",
            arguments.join(" "),
            ran.status
        ));
    }
    String::from_utf8(ran.stdout)
        .map(|said| said.trim_end_matches('\n').to_owned())
        .map_err(|_| format!("{program} {} printed what isn't text", arguments.join(" ")))
}
