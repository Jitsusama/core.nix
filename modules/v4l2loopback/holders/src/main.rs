//! Says which programs have a camera open, in a file a shell can watch.
//!
//! ```text
//! loopback-holders [--card NAME] [--ignore-unit UNIT]... SOURCE OUTPUT
//! loopback-holders --usb [--ignore-unit UNIT]... NODE FOLDER
//! loopback-holders --usb --forget NODE FOLDER
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
//! With --forget, it removes the files in FOLDER that say NODE, and ends:
//! a USB camera's unit runs it as it stops, so a run killed before it
//! could remove its own leaves nothing to say the camera is still there.
//! A loopback that has NODE by then keeps its files, which say it rightly.
//!
//! A file is written beside the one it replaces under a dotted name,
//! `.<its name>.<pid>`, which every reader of the folder passes over, so
//! no camera's name, as `Webcam 2.0`, is ever taken for one half-written.
//!
//! Beside the file, `.devices/<its name>` says which node the camera is,
//! as its real path, for as long as there's one: a reader that sees
//! `PipeWire` holding it finds the camera in `PipeWire`'s registry by it, and
//! names who reads it there (modules/machine's camera.rs). A node a link
//! names, as the relay's is, changes with each plug-in, so the link is
//! followed first.
//!
//! Between those it sleeps on inotify and a signal, so a camera nobody
//! holds costs nothing. A file it couldn't write, its own or the one in
//! `.devices/`, or a read of the holders that failed, is looked at again
//! until it's right, whoever holds the camera: each second, or every two
//! while a loopback's camera is watched. What went wrong is said once as
//! it begins and once as it clears, for each file. The first time its
//! files say who holds the camera as a read that worked found, and which
//! node it is, or nobody where there's no camera, its log says
//! `ready in N ms`, once a run (src/ready.rs says why). Its help, its
//! failures and its exits take the terminal kit's shape (`help()`).
//!
//! Long because a quarter of it is the tests beside it, of how it's asked
//! and of what its watches say, which reach its private parts.

mod camera;
mod count;
mod holders;
mod ready;
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

use ready::Ready;
use record::Record;
use terminal_kit::{Exit, Failure, Help};
use wait::{Events, Woken};

struct Options {
    card: Option<String>,
    units: Vec<String>,
    usb: bool,
    forget: bool,
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
        .option(
            "--forget",
            "with --usb, removes what says the node, and ends",
        )
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
    let mut forget = false;
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
            "--forget" => forget = true,
            option if option.starts_with('-') => {
                return Err(wrongly(format!("{option} isn't a flag it takes")));
            }
            _ => places.push(PathBuf::from(argument)),
        }
    }
    if forget && !usb {
        return Err(wrongly("--forget goes with --usb".to_owned()));
    }
    if forget && (card.is_some() || !units.is_empty()) {
        return Err(wrongly(
            "--forget takes no --card or --ignore-unit".to_owned(),
        ));
    }
    let [source, output] = <[PathBuf; 2]>::try_from(places).map_err(|places| {
        wrongly("loopback-holders takes a SOURCE and an OUTPUT".to_owned())
            .because(format!("it was given {} of them", places.len()))
    })?;
    Ok(Options {
        card,
        units,
        usb,
        forget,
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
/// no longer is; answering what went wrong, for its caller to say.
fn say_device(output: &Path, node: Option<&Path>) -> Result<(), String> {
    let Some(file) = device_file(output) else {
        return Ok(());
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
        // A folder that isn't there, or isn't a folder, holds no node file.
        None => match fs::remove_file(&file) {
            Err(error)
                if !matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                ) =>
            {
                Err(error)
            }
            _ => Ok(()),
        },
    };
    said.map_err(|error| format!("{}: {error}", file.display()))
}

/// Removes each file in the folder but this run's own that says this node,
/// and the holders file beside the folder that it's named for, as that
/// camera is gone, saying so in the log. One being written, under a dotted
/// name, is left to its writer, or for a killed writer's, to its camera's
/// next run to tidy: no reader takes it for a camera, so it says nothing.
fn forget_node(folder: &Path, own: Option<&Path>, said: &str) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let being_written = entry.file_name().as_encoded_bytes().starts_with(b".");
        if Some(path.as_path()) == own
            || being_written
            || !entry.file_type().is_ok_and(|kind| kind.is_file())
        {
            continue;
        }
        if !fs::read_to_string(&path).is_ok_and(|text| text == said) {
            continue;
        }
        // Its holders file first, so a camera is never left said held with
        // no node file to say who reads it. Read and removed apart, so a
        // camera whose own run rewrites them between, as two cameras
        // swapping nodes as a hub comes back might, reads unplugged until
        // it streams: a run killed and a swap at once, which is accepted.
        let holders = folder.parent().map(|above| above.join(entry.file_name()));
        // A holders file that can't go keeps its node file too, so a later
        // forget can still find it.
        let mut removed = true;
        for gone in holders.iter().chain([&path]) {
            if fs::symlink_metadata(gone).is_ok_and(|kind| kind.is_file())
                && let Err(error) = fs::remove_file(gone)
                && error.kind() != io::ErrorKind::NotFound
            {
                complain(format_args!("{}: {error}", gone.display()));
                removed = false;
                break;
            }
        }
        // Quoted, as the record quotes a holder, since a USB camera names
        // itself.
        if removed {
            say(&format!(
                "forgot {:?}, whose node file said {}",
                entry.file_name().to_string_lossy(),
                said.trim_end()
            ));
        }
    }
}

/// Where the file is written before it replaces the output: beside it,
/// under a dotted name with this run's pid, which no reader takes for a
/// camera's.
fn temporary(output: &Path) -> Option<PathBuf> {
    let mut name = std::ffi::OsString::from(".");
    name.push(output.file_name()?);
    name.push(format!(".{}", std::process::id()));
    Some(output.with_file_name(name))
}

/// Replaces the file whole with the body, readable by all.
fn replace(output: &Path, body: &str) -> io::Result<()> {
    let temporary = temporary(output).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "a file to write has no name")
    })?;
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
    let prefix = format!(".{}.", base.to_string_lossy());
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

/// What went wrong on a watch's last look at its file, said once as it
/// begins or changes and once as it clears, since a look comes each second
/// while it lasts.
#[derive(Default)]
struct Trouble(Option<String>);

impl Trouble {
    /// Takes this look's troubles, none when all went right, saying them
    /// only when they differ from the last look's.
    fn now(&mut self, output: &Path, troubles: &[String]) {
        let now = (!troubles.is_empty()).then(|| troubles.join("; "));
        if now != self.0 {
            match &now {
                Some(what) => complain(format_args!("{what}")),
                None => complain(format_args!("{}: right again", output.display())),
            }
            self.0 = now;
        }
    }
}

/// What a run has said of its files: whether it's ready, the trouble it
/// last complained of in each, and the node `.devices/` is still owed.
/// It's one for the whole run, so a trouble a loopback's nobody met is
/// cleared by the watch that next writes the file right, and the other way
/// round.
struct Run {
    ready: Ready,
    trouble: Trouble,
    owed: Option<PathBuf>,
    device: Trouble,
}

impl Run {
    fn begun() -> Self {
        Self {
            ready: Ready::begun(),
            trouble: Trouble::default(),
            owed: None,
            device: Trouble::default(),
        }
    }

    /// Owes `.devices/` the node, said on the watch's next look and each
    /// after until it's written.
    fn owe(&mut self, node: &Path) {
        self.owed = Some(node.to_owned());
    }

    /// Says the node owed, answering whether `.devices/` says it now.
    fn name(&mut self, output: &Path) -> bool {
        let (Some(node), Some(file)) = (self.owed.take(), device_file(output)) else {
            return true;
        };
        match say_device(output, Some(&node)) {
            Ok(()) => {
                self.device.now(&file, &[]);
                true
            }
            Err(trouble) => {
                self.device.now(&file, &[trouble]);
                self.owed = Some(node);
                false
            }
        }
    }

    /// Removes what says the node, as the camera goes or the run ends, so
    /// nothing owed is said after, and a trouble the file had clears.
    /// Says whether it's gone, so a removal that failed is tried again.
    fn unname(&mut self, output: &Path) -> bool {
        self.owed = None;
        let Some(file) = device_file(output) else {
            return true;
        };
        match say_device(output, None) {
            Ok(()) => {
                self.device.now(&file, &[]);
                true
            }
            Err(trouble) => {
                self.device.now(&file, &[trouble]);
                false
            }
        }
    }

    /// Says the run is ready, the first time its file says who holds the
    /// camera as a read that worked found.
    fn written(&mut self) {
        if let Some(line) = self.ready.line() {
            say(&line);
        }
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
    run: &mut Run,
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
    // What the file holds, as last written.
    let mut said = None;
    let mut pids = HashSet::new();
    loop {
        let mut troubles = Vec::new();
        // A read that fails writes nobody, which is a guess, so only a
        // read that worked counts toward the watch being ready.
        let (names, seen) = match holders::holders(node, units, &pids) {
            Ok((names, now)) => {
                pids = now;
                (names, true)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ended::Gone,
            Err(error) => {
                troubles.push(format!("{}: {error}", node.display()));
                (BTreeSet::new(), false)
            }
        };
        let anyone = !names.is_empty();
        let shown = listed(names, sysfs, node);
        // The node first, so a reader that finds a holder can find the
        // camera by it, and a run killed between the two leaves the node
        // file that finds the holders file, as the removals keep it.
        let found = run.name(output);
        let right = said.as_ref() == Some(&shown)
            || match write(output, &shown) {
                Ok(()) => true,
                Err(error) => {
                    troubles.push(format!("{}: {error}", output.display()));
                    false
                }
            };
        run.trouble.now(output, &troubles);
        let settled = seen && right && found;
        if settled {
            run.written();
        }
        // The record asks the camera itself, which a stalling camera can
        // keep waiting, so it comes after the file is right.
        if let Some(record) = record.as_deref_mut() {
            let streaming: Vec<String> = shown.iter().cloned().collect();
            if let Some(line) = record.now(node, &streaming) {
                say(&line);
            }
        }
        // A file that couldn't be written, either of them, or a read that
        // failed, is tried again on the next look, which comes on a timer
        // even while nobody holds the camera.
        said = right.then_some(shown);
        match events.wait((anyone || !settled).then_some(every)) {
            Woken::Timeout | Woken::Changed => {}
            Woken::NodeGone => return Ended::Gone,
            Woken::Stopped => return Ended::Stopped,
        }
    }
}

/// Says who streams from a USB camera while it's there, in a file of its
/// own that goes with it.
fn usb(events: &mut Events, options: &Options, run: &mut Run) -> Result<(), Failure> {
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
        // camera's files saying it were left by a run killed before it
        // could remove them. A loopback's unit removes its own as it stops,
        // and one with no card may say a node another camera has since been
        // given.
        if let Some(folder) = file.parent() {
            forget_node(folder, Some(&file), &said_as(&node));
        }
    }
    run.owe(&node);
    let mut record = Record::new(&options.sysfs, &options.debugfs);
    let ended = watch(
        events,
        &node,
        &output,
        &options.units,
        Some(&options.sysfs),
        Some(&mut record),
        run,
    );
    // A stream the watch ends during has its last seconds said too, since
    // a camera dropping off mid-call is what the record is for.
    if let Some(line) = record.ended(then(&ended, node.exists())) {
        say(&line);
    }
    // Its holders file first, as forget_node removes them, so a run killed
    // between the two, or one that can't remove it, leaves the node file
    // that finds the holders file.
    match fs::remove_file(&output) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(Failure::new(
            Exit::Failed,
            "loopback-holders can't remove the camera's file",
        )
        .because(format!("{}: {error}", output.display()))),
        _ => {
            run.unname(&output);
            // A watch that ended with the camera still there, as one that
            // couldn't be made does, fails, so the unit starts it again
            // rather than leaving the camera unwatched.
            if matches!(ended, Ended::Gone) && node.exists() {
                Err(Failure::new(
                    Exit::Failed,
                    "loopback-holders stopped watching a camera that's still there",
                )
                .because(node.display().to_string()))
            } else {
                Ok(())
            }
        }
    }
}

/// How a node file says its node.
fn said_as(node: &Path) -> String {
    format!("{}\n", node.display())
}

/// Removes what says a USB camera's node, as its unit stops: its own run
/// removes its files as it ends, so any left were a killed run's. A node a
/// loopback has by then is left alone, since a loopback added as a hub
/// comes back takes the lowest free number, and its files say it rightly;
/// the gone camera's own come back right when it does. A USB camera there
/// is this one, its run crashed while it stayed plugged in, as the unit's
/// next run at the node waits for this; with no run left to rewrite its
/// files they're forgotten, so it reads unplugged rather than held for
/// good. A loopback taking the number between the look and the removal
/// loses its files until its holders change, which is accepted. A
/// pre-dotted file of the old `<name>.<pid>` shape isn't sought: a run's
/// stop is read from a signalfd between writes, so only a run killed
/// mid-write left one.
fn forget(options: &Options) {
    if camera::loopback_at(&options.sysfs, &options.source) {
        return;
    }
    forget_node(
        &options.output.join(".devices"),
        None,
        &said_as(&options.source),
    );
}

/// Writes that nobody holds a loopback, answering whether the file took it,
/// so one it didn't is written again. It says nothing of ready: that's for
/// its caller, since nobody is so only with no camera and its node file
/// gone, and a guess for one it couldn't watch, until it's tried again.
fn say_nobody(output: &Path, run: &mut Run) -> bool {
    match write(output, &BTreeSet::new()) {
        Ok(()) => {
            run.trouble.now(output, &[]);
            true
        }
        Err(error) => {
            run.trouble
                .now(output, &[format!("{}: {error}", output.display())]);
            false
        }
    }
}

/// Says who holds a loopback, following the source to each node it names.
fn loopback(events: &mut Events, options: &Options, run: &mut Run) {
    tidy(&options.output);
    if let Some(file) = device_file(&options.output) {
        tidy(&file);
    }
    // Whether the camera's going was seen, and whether nobody was written
    // and its node unnamed since, each tried each second until it is.
    let mut gone = false;
    let mut told = false;
    let mut unnamed = false;
    loop {
        let Some(node) = camera::node_of(&options.sysfs, &options.source, options.card.as_deref())
        else {
            if !gone {
                gone = true;
                told = false;
                unnamed = false;
            }
            // Its holders file first, as the removals keep it, so no
            // holder is ever said with no node file to find the camera by.
            if !told {
                told = say_nobody(&options.output, run);
            }
            if told && !unnamed {
                unnamed = run.unname(&options.output);
            }
            if told && unnamed {
                run.written();
            }
            if let Woken::Stopped = events.sleep(Duration::from_secs(1)) {
                return;
            }
            continue;
        };
        gone = false;
        run.owe(&node);
        let ended = watch(
            events,
            &node,
            &options.output,
            &options.units,
            None,
            None,
            run,
        );
        // A camera gone is said no more, so a node that comes next, which
        // may not be sayable yet, never leaves the last one said. A gone
        // node is unnamed by the pass with no camera, which tries until it
        // can; a stop unnames it here, only once nobody is written.
        let told = say_nobody(&options.output, run);
        if let Ended::Stopped = ended {
            if told {
                run.unname(&options.output);
            }
            return;
        }
        // So a node it can't watch is tried again, not spun on.
        if let Woken::Stopped = events.sleep(Duration::from_millis(500)) {
            if told {
                run.unname(&options.output);
            }
            return;
        }
    }
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let mut run = Run::begun();
        let options = options(context.args.iter().cloned())?;
        // Forgetting watches nothing, so a watch it can't have mustn't
        // leave what it's there to remove.
        if options.forget {
            forget(&options);
            return Ok(());
        }
        let mut events = Events::new().map_err(|error| {
            Failure::new(Exit::Failed, "loopback-holders can't watch for its camera")
                .because(format!("inotify: {error}"))
        })?;
        if options.usb {
            usb(&mut events, &options, &mut run)
        } else {
            loopback(&mut events, &options, &mut run);
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
        let real = fs::canonicalize(&node).unwrap();
        // Only a USB camera's run forgets another's file saying its node.
        let another = root.join(".devices/Another");
        fs::write(&another, format!("{}\n", real.display())).unwrap();
        say_device(&output, Some(&link)).unwrap();
        assert!(
            another.exists(),
            "another camera's file, said by a loopback"
        );
        fs::remove_file(&another).unwrap();
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
        say_device(&output, None).unwrap();
        assert!(!file.exists(), "the camera gone, its node is said no more");
    }

    #[test]
    fn tidying_takes_only_what_a_killed_run_left() {
        let root = scratch("tidy");
        for name in [
            "camera",
            ".camera.123",
            ".camera.",
            ".camera.1a",
            ".cameras.4",
            ".other.5",
            // A camera named for a version, beside one that isn't.
            "camera.0",
            ".camera.0.7",
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
            [
                ".camera.",
                ".camera.0.7",
                ".camera.1a",
                ".cameras.4",
                ".other.5",
                "camera",
                "camera.0"
            ]
        );
    }

    #[test]
    fn a_file_is_written_under_a_dotted_name_beside_it() {
        let written = temporary(Path::new("/run/camera-holders/Webcam 2.0")).unwrap();
        assert_eq!(
            written,
            Path::new(&format!(
                "/run/camera-holders/.Webcam 2.0.{}",
                std::process::id()
            ))
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
        let forgetting = given("--usb --forget /dev/video0 /run/camera-holders").unwrap();
        assert!(
            forgetting.usb && forgetting.forget,
            "--usb --forget wasn't read as both"
        );
    }

    #[test]
    fn each_wrong_start_is_refused_as_asked_wrongly() {
        for line in [
            "/dev/video0",
            "--card",
            "--what /a /b",
            "-x /a /b",
            "/a /b /c",
            "--forget /a /b",
            "--usb --forget --card Camera /a /b",
            "--usb --forget --ignore-unit a.service /a /b",
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
