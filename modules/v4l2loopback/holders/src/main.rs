//! Says which programs have a camera open, in a file a shell can watch.
//!
//! ```text
//! loopback-holders [--card NAME] [--ignore-unit UNIT]... SOURCE OUTPUT
//! loopback-holders --usb [--ignore-unit UNIT]... NODE FOLDER
//! ```
//!
//! SOURCE is the camera's V4L2 node, or a file naming it, as the relay
//! writes one. OUTPUT gets one program name per line, sorted, and nothing
//! while no program has it open; it's replaced whole, so a reader never
//! sees half of it. A process in an ignored unit, such as the relay that
//! fills the camera, is left out; a unit is a cgroup no program can put
//! itself in, as it can take any name. With --card, only a loopback of
//! that name counts, so a node number another camera has since been given
//! isn't taken for this one.
//!
//! It looks when something opens or closes the node, and every two seconds
//! while anyone holds it, since a holder can change with neither: by exec,
//! by handing its descriptor to another process, or by exiting and leaving
//! it to a child. While there's no node, as when the camera's module is
//! reloaded, it says nobody and waits for one, looking once a second.
//!
//! With --usb, NODE is a USB camera's, and the file is FOLDER/<the camera's
//! name>, made and removed with the camera: it's written while the camera
//! is there and removed when its node goes or this is stopped. A program
//! holding the camera is listed only while the camera streams, since
//! programs that only look at a camera, such as `PipeWire`'s, open it for a
//! moment as it comes; while anyone holds it, it looks once a second, as
//! the stream shows only in sysfs, which says nothing when it changes.
//! While a USB camera streams, it also says in its log, every ten seconds,
//! how: the format and frame rate asked for, the frames the camera gave,
//! the packets it sent in error, and its controls (src/record.rs says
//! how).
//!
//! Beside the file, `.devices/<its name>` says which node the camera is,
//! as its real path, for as long as there's one: a reader that sees
//! `PipeWire` holding it finds the camera in `PipeWire`'s registry by it, and
//! names who reads it there (modules/machine's camera.rs). A node a link
//! names, as the relay's is, changes with each plug-in, so the link is
//! followed first.
//!
//! Between those it sleeps on inotify and a signal, so a camera nobody
//! holds costs nothing. Its help, its failures and its exits take the
//! terminal kit's shape (`help()`).

mod camera;
mod count;
mod holders;
mod record;
mod v4l2;
mod wait;

use std::collections::{BTreeSet, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use record::Record;
use terminal_kit::{Exit, Failure, Help};
use wait::{Events, Woken};

struct Options {
    card: Option<String>,
    units: Vec<String>,
    usb: bool,
    sysfs: PathBuf,
    debugfs: PathBuf,
    source: PathBuf,
    output: PathBuf,
}

/// What it says it is, does and ends with, as `--help` shows it. Its
/// tests' `--sysfs PATH` and `--debugfs PATH`, where those are, go
/// unnamed.
fn help() -> Help {
    Help::new("loopback-holders", "says who has a camera open, in a file")
        .example(
            "loopback-holders /dev/video9 /run/camera-holders/meet",
            "names who holds a loopback",
        )
        .example(
            "loopback-holders --usb /dev/video0 /run/camera-holders",
            "names who streams from a USB camera",
        )
        .argument("SOURCE", "the camera's node, or a file naming it")
        .argument("OUTPUT", "the file to name them in; with --usb, its folder")
        .option("--card NAME", "only a loopback of this name counts")
        .option("--ignore-unit UNIT", "leaves out this unit's processes")
        .option("--usb", "SOURCE is a USB camera's node")
        .exit(Exit::Failed, "its camera or file couldn't be watched")
}

/// A start asked wrongly, pointing at the help.
fn wrongly(what: String) -> Failure {
    Failure::new(Exit::Usage, what).try_this("loopback-holders --help")
}

fn options(mut arguments: impl Iterator<Item = String>) -> Result<Options, Failure> {
    let mut card = None;
    let mut units = Vec::new();
    let mut usb = false;
    // Where sysfs and debugfs are, for the tests.
    let mut sysfs = PathBuf::from("/sys");
    let mut debugfs = PathBuf::from("/sys/kernel/debug");
    let mut places = Vec::new();
    while let Some(argument) = arguments.next() {
        let mut value = || {
            arguments
                .next()
                .ok_or_else(|| wrongly(format!("{argument} wants a value after it")))
        };
        match argument.as_str() {
            "--card" => card = Some(value()?),
            "--ignore-unit" => units.push(value()?),
            "--sysfs" => sysfs = PathBuf::from(value()?),
            "--debugfs" => debugfs = PathBuf::from(value()?),
            "--usb" => usb = true,
            option if option.starts_with('-') => {
                return Err(wrongly(format!("{option} isn't a flag it takes")));
            }
            _ => places.push(PathBuf::from(argument)),
        }
    }
    let [source, output] = <[PathBuf; 2]>::try_from(places).map_err(|places| {
        wrongly("loopback-holders takes a SOURCE and an OUTPUT".to_owned())
            .because(format!("it was given {} of them", places.len()))
    })?;
    Ok(Options {
        card,
        units,
        usb,
        sysfs,
        debugfs,
        source,
        output,
    })
}

/// Who to name: everyone holding the camera, or for a USB camera, everyone
/// only while it streams.
fn listed(names: BTreeSet<String>, sysfs: Option<&Path>, node: &Path) -> BTreeSet<String> {
    match sysfs {
        Some(sysfs) if !camera::streams(sysfs, node) => BTreeSet::new(),
        _ => names,
    }
}

/// Replaces the output whole with the names, one a line, readable by all.
fn write(output: &Path, names: &BTreeSet<String>) -> io::Result<()> {
    let body: String = names
        .iter()
        .flat_map(|name| [name.as_str(), "\n"])
        .collect();
    replace(output, &body)
}

/// Where the node the output's camera is gets said: `.devices/` beside
/// it, under its name, a folder every reader of the cameras' files passes
/// over, as its name begins with a dot.
fn device_file(output: &Path) -> Option<PathBuf> {
    let name = output.file_name()?;
    let folder = output.parent().unwrap_or_else(|| Path::new(""));
    Some(folder.join(".devices").join(name))
}

/// Says which node the output's camera is, as its real path, or with none,
/// removes what said it, so a reader never finds the camera by a node it
/// no longer is.
fn say_device(output: &Path, node: Option<&Path>) {
    let Some(file) = device_file(output) else {
        return;
    };
    let said = match node {
        Some(node) => {
            let node = fs::canonicalize(node).unwrap_or_else(|_| node.to_owned());
            let folder = file.parent().unwrap_or_else(|| Path::new(""));
            let said = format!("{}\n", node.display());
            fs::create_dir_all(folder)
                .and_then(|()| fs::set_permissions(folder, fs::Permissions::from_mode(0o755)))
                .and_then(|()| replace(&file, &said))
        }
        None => match fs::remove_file(&file) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        },
    };
    if let Err(error) = said {
        complain(format_args!("{}: {error}", file.display()));
    }
}

/// Removes each file in the folder but this one that says this node; one
/// being written, its name ending in a dot and a pid, is left to its
/// writer.
fn forget_node(folder: &Path, own: &Path, said: &str) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let being_written = path
            .extension()
            .and_then(|end| end.to_str())
            .is_some_and(|end| !end.is_empty() && end.bytes().all(|byte| byte.is_ascii_digit()));
        if path == own || being_written || !entry.file_type().is_ok_and(|kind| kind.is_file()) {
            continue;
        }
        if fs::read_to_string(&path).is_ok_and(|text| text == said)
            && let Err(error) = fs::remove_file(&path)
            && error.kind() != io::ErrorKind::NotFound
        {
            complain(format_args!("{}: {error}", path.display()));
        }
    }
}

/// Replaces the file whole with the body, readable by all.
fn replace(output: &Path, body: &str) -> io::Result<()> {
    let mut temporary = output.as_os_str().to_owned();
    temporary.push(format!(".{}", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o644)
        .open(&temporary)?;
    file.write_all(body.as_bytes())?;
    file.set_permissions(fs::Permissions::from_mode(0o644))?;
    fs::rename(&temporary, output)
}

/// Removes what a run killed mid-write left beside the output.
fn tidy(output: &Path) {
    let (Some(folder), Some(base)) = (output.parent(), output.file_name()) else {
        return;
    };
    let folder = if folder.as_os_str().is_empty() {
        Path::new(".")
    } else {
        folder
    };
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    let prefix = format!("{}.", base.to_string_lossy());
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let left = name
            .strip_prefix(&prefix)
            .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()));
        if left {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// What happened when a watch ended, for a stream's last line. An
/// unplugged camera's unit is stopped as its device goes, often before the
/// watch has seen the node go, so a stop finding the node gone is the
/// camera going.
fn then(ended: &Ended, there: bool) -> &'static str {
    match ended {
        Ended::Stopped if there => "the watch stopped",
        Ended::Gone | Ended::Stopped => "the camera went",
    }
}

/// How a watch ended.
enum Ended {
    /// The node went, or couldn't be watched.
    Gone,
    /// It was told to stop.
    Stopped,
}

/// Says what went wrong in the log. Under the unit, stderr is the same
/// journal stream as stdout, so when one can't be written to neither can
/// the other, and eprintln! would panic and stop the watch, leaving the
/// file naming whoever held the node; there's nowhere left to say it.
fn complain(what: std::fmt::Arguments) {
    let _ = writeln!(io::stderr().lock(), "{what}");
}

/// Says a line of how the camera streams. A log that can't be written to
/// mustn't stop the watch.
fn say(line: &str) {
    if let Err(error) = writeln!(io::stdout().lock(), "{line}") {
        complain(format_args!("can't say how it streams: {error}"));
    }
}

/// Says who holds the node until it goes. With sysfs, the node is a USB
/// camera's, named only while it streams, and with a record, said in the
/// log as it does.
fn watch(
    events: &mut Events,
    node: &Path,
    output: &Path,
    units: &[String],
    sysfs: Option<&Path>,
    mut record: Option<&mut Record>,
) -> Ended {
    if let Err(error) = events.watch(node) {
        complain(format_args!("can't watch {}: {error}", node.display()));
        return match events.sleep(Duration::from_secs(5)) {
            Woken::Stopped => Ended::Stopped,
            _ => Ended::Gone,
        };
    }
    // Whether a USB camera streams shows only in sysfs, so while anyone
    // holds it, it's looked at every second.
    let every = Duration::from_secs(if sysfs.is_some() { 1 } else { 2 });
    let mut said = None;
    let mut pids = HashSet::new();
    loop {
        let names = match holders::holders(node, units, &pids) {
            Ok((names, now)) => {
                pids = now;
                names
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ended::Gone,
            Err(error) => {
                complain(format_args!("{}: {error}", node.display()));
                BTreeSet::new()
            }
        };
        let anyone = !names.is_empty();
        let shown = listed(names, sysfs, node);
        if said.as_ref() != Some(&shown)
            && let Err(error) = write(output, &shown)
        {
            complain(format_args!("{}: {error}", output.display()));
        }
        // The record asks the camera itself, which a stalling camera can
        // keep waiting, so it comes after the file is right.
        if let Some(record) = record.as_deref_mut() {
            let streaming: Vec<String> = shown.iter().cloned().collect();
            if let Some(line) = record.now(node, &streaming) {
                say(&line);
            }
        }
        said = Some(shown);
        match events.wait(anyone.then_some(every)) {
            Woken::Timeout | Woken::Changed => {}
            Woken::NodeGone => return Ended::Gone,
            Woken::Stopped => return Ended::Stopped,
        }
    }
}

/// Says who streams from a USB camera while it's there, in a file of its
/// own that goes with it.
fn usb(events: &mut Events, options: &Options) -> Result<(), Failure> {
    let Ok(node) = fs::canonicalize(&options.source) else {
        return Ok(());
    };
    let output = camera::camera_file(&options.sysfs, &node, &options.output).map_err(|error| {
        Failure::new(
            Exit::Failed,
            "loopback-holders can't name the camera's file",
        )
        .because(format!("{}: {error}", node.display()))
    })?;
    tidy(&output);
    if let Some(file) = device_file(&output) {
        tidy(&file);
        // A USB camera's unit is the one that watches its node, so another
        // camera's file saying it was left by a run killed before it could
        // remove it. A loopback's unit removes its own as it stops, and one
        // with no card may say a node another camera has since been given.
        if let Some(folder) = file.parent() {
            forget_node(folder, &file, &format!("{}\n", node.display()));
        }
    }
    say_device(&output, Some(&node));
    let mut record = Record::new(&options.sysfs, &options.debugfs);
    let ended = watch(
        events,
        &node,
        &output,
        &options.units,
        Some(&options.sysfs),
        Some(&mut record),
    );
    // A stream the watch ends during has its last seconds said too, since
    // a camera dropping off mid-call is what the record is for.
    if let Some(line) = record.ended(then(&ended, node.exists())) {
        say(&line);
    }
    say_device(&output, None);
    match fs::remove_file(&output) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(Failure::new(
            Exit::Failed,
            "loopback-holders can't remove the camera's file",
        )
        .because(format!("{}: {error}", output.display()))),
        _ => Ok(()),
    }
}

/// Says who holds a loopback, following the source to each node it names.
fn loopback(events: &mut Events, options: &Options) {
    let nobody = BTreeSet::new();
    let say_nobody = || {
        if let Err(error) = write(&options.output, &nobody) {
            complain(format_args!("{}: {error}", options.output.display()));
        }
    };
    tidy(&options.output);
    if let Some(file) = device_file(&options.output) {
        tidy(&file);
    }
    let mut gone = false;
    loop {
        let Some(node) = camera::node_of(&options.sysfs, &options.source, options.card.as_deref())
        else {
            if !gone {
                say_device(&options.output, None);
                say_nobody();
                gone = true;
            }
            if let Woken::Stopped = events.sleep(Duration::from_secs(1)) {
                return;
            }
            continue;
        };
        gone = false;
        say_device(&options.output, Some(&node));
        let ended = watch(events, &node, &options.output, &options.units, None, None);
        say_nobody();
        if let Ended::Stopped = ended {
            say_device(&options.output, None);
            return;
        }
        // So a node it can't watch is tried again, not spun on.
        if let Woken::Stopped = events.sleep(Duration::from_millis(500)) {
            say_device(&options.output, None);
            return;
        }
    }
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let options = options(context.args.iter().cloned())?;
        let mut events = Events::new().map_err(|error| {
            Failure::new(Exit::Failed, "loopback-holders can't watch for its camera")
                .because(format!("inotify: {error}"))
        })?;
        if options.usb {
            usb(&mut events, &options)
        } else {
            loopback(&mut events, &options);
            Ok(())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use camera::tests::{Camera, scratch};

    fn named(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn a_watch_stopped_as_its_camera_goes_says_the_camera_went() {
        let said = [
            then(&Ended::Stopped, true),
            then(&Ended::Stopped, false),
            then(&Ended::Gone, false),
            then(&Ended::Gone, true),
        ];
        assert_eq!(
            said,
            [
                "the watch stopped",
                "the camera went",
                "the camera went",
                "the camera went"
            ]
        );
    }

    #[test]
    fn an_idle_usb_camera_names_nobody() {
        let camera = Camera::new("listed-idle", "MX Brio 705 for Business");
        assert!(listed(named(&["firefox"]), Some(&camera.sysfs), &camera.node).is_empty());
    }

    #[test]
    fn a_streaming_usb_camera_names_its_holders() {
        let camera = Camera::new("listed-streaming", "MX Brio 705 for Business");
        camera.alternate("2-1:1.1", 9);
        assert_eq!(
            listed(named(&["firefox"]), Some(&camera.sysfs), &camera.node),
            named(&["firefox"])
        );
        camera.alternate("2-1:1.1", 0);
        assert!(listed(named(&["firefox"]), Some(&camera.sysfs), &camera.node).is_empty());
    }

    #[test]
    fn a_loopback_names_whoever_holds_it() {
        let camera = Camera::new("listed-loopback", "Stand-in");
        assert_eq!(
            listed(named(&["firefox"]), None, &camera.node),
            named(&["firefox"])
        );
    }

    #[test]
    fn the_output_is_replaced_whole_and_readable_by_all() {
        let root = scratch("write");
        let output = root.join("camera");
        write(&output, &named(&["zoom", "firefox"])).unwrap();
        assert_eq!(fs::read_to_string(&output).unwrap(), "firefox\nzoom\n");
        assert_eq!(
            fs::metadata(&output).unwrap().permissions().mode() & 0o777,
            0o644
        );
        write(&output, &BTreeSet::new()).unwrap();
        assert_eq!(fs::read_to_string(&output).unwrap(), "");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    }

    #[test]
    fn the_node_is_said_by_its_real_path_until_the_camera_goes() {
        let root = scratch("device");
        let node = root.join("video32");
        fs::write(&node, "").unwrap();
        let link = root.join("device");
        std::os::unix::fs::symlink(&node, &link).unwrap();
        let output = root.join("MX Brio 705 for Business");
        let file = root.join(".devices/MX Brio 705 for Business");
        // A folder left narrower, as a umask might make it, is widened.
        fs::create_dir(root.join(".devices")).unwrap();
        fs::set_permissions(root.join(".devices"), fs::Permissions::from_mode(0o700)).unwrap();
        say_device(&output, Some(&link));
        let real = fs::canonicalize(&node).unwrap();
        assert_eq!(
            fs::read_to_string(&file).unwrap(),
            format!("{}\n", real.display())
        );
        assert_eq!(
            fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o644
        );
        assert_eq!(
            fs::metadata(root.join(".devices"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
        assert_eq!(fs::read_dir(root.join(".devices")).unwrap().count(), 1);
        say_device(&output, None);
        assert!(!file.exists(), "the camera gone, its node is said no more");
    }

    #[test]
    fn tidying_takes_only_what_a_killed_run_left() {
        let root = scratch("tidy");
        for name in [
            "camera",
            "camera.123",
            "camera.",
            "camera.1a",
            "cameras.4",
            "other.5",
        ] {
            fs::write(root.join(name), "").unwrap();
        }
        tidy(&root.join("camera"));
        let mut left: Vec<_> = fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        left.sort();
        assert_eq!(
            left,
            ["camera", "camera.", "camera.1a", "cameras.4", "other.5"]
        );
    }

    #[test]
    fn the_arguments_are_read_as_the_units_give_them() {
        let given = |line: &str| options(line.split(' ').map(str::to_owned));
        let read =
            given("--card Stand-in --ignore-unit a.service --ignore-unit b.service /run/d /run/o")
                .unwrap();
        assert_eq!(read.card.as_deref(), Some("Stand-in"));
        assert_eq!(read.units, ["a.service", "b.service"]);
        assert!(!read.usb);
        assert_eq!(read.sysfs, Path::new("/sys"));
        assert_eq!(read.debugfs, Path::new("/sys/kernel/debug"));
        assert_eq!(
            (read.source.as_path(), read.output.as_path()),
            (Path::new("/run/d"), Path::new("/run/o"))
        );
        assert!(given("--usb /dev/video0 /run/camera-holders").unwrap().usb);
    }

    #[test]
    fn each_wrong_start_is_refused_as_asked_wrongly() {
        for line in [
            "/dev/video0",
            "--card",
            "--what /a /b",
            "-x /a /b",
            "/a /b /c",
        ] {
            let ending = terminal_kit::run(
                &help(),
                "0.1.0",
                &terminal_kit::Environment::of([false; 3], &[]),
                line.split(' ').map(str::to_owned).collect(),
                |context| options(context.args.iter().cloned()).map(|_| ()),
            );
            assert_eq!(ending.exit, Exit::Usage, "{line} wasn't refused with 64");
            assert!(
                ending.stderr.contains("loopback-holders --help"),
                "{line}'s refusal didn't point at the help: {}",
                ending.stderr
            );
        }
    }
}
