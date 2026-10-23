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
//!
//! Between those it sleeps on inotify and a signal, so a camera nobody
//! holds costs nothing.

mod camera;
mod holders;
mod wait;

use std::collections::{BTreeSet, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use wait::{Events, Woken};

struct Options {
    card: Option<String>,
    units: Vec<String>,
    usb: bool,
    sysfs: PathBuf,
    source: PathBuf,
    output: PathBuf,
}

const USAGE: &str = "usage: loopback-holders [--card NAME] [--ignore-unit UNIT]... SOURCE OUTPUT\n       \
    loopback-holders --usb [--ignore-unit UNIT]... NODE FOLDER";

fn options(mut arguments: impl Iterator<Item = String>) -> Option<Options> {
    let mut card = None;
    let mut units = Vec::new();
    let mut usb = false;
    // Where sysfs is, for the tests.
    let mut sysfs = PathBuf::from("/sys");
    let mut places = Vec::new();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--card" => card = Some(arguments.next()?),
            "--ignore-unit" => units.push(arguments.next()?),
            "--usb" => usb = true,
            "--sysfs" => sysfs = PathBuf::from(arguments.next()?),
            option if option.starts_with("--") => return None,
            _ => places.push(PathBuf::from(argument)),
        }
    }
    let [source, output] = <[PathBuf; 2]>::try_from(places).ok()?;
    Some(Options {
        card,
        units,
        usb,
        sysfs,
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

/// How a watch ended.
enum Ended {
    /// The node went, or couldn't be watched.
    Gone,
    /// It was told to stop.
    Stopped,
}

/// Says who holds the node until it goes. With sysfs, the node is a USB
/// camera's, named only while it streams.
fn watch(
    events: &mut Events,
    node: &Path,
    output: &Path,
    units: &[String],
    sysfs: Option<&Path>,
) -> Ended {
    if let Err(error) = events.watch(node) {
        eprintln!("can't watch {}: {error}", node.display());
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
                eprintln!("{}: {error}", node.display());
                BTreeSet::new()
            }
        };
        let anyone = !names.is_empty();
        let shown = listed(names, sysfs, node);
        if said.as_ref() != Some(&shown) {
            if let Err(error) = write(output, &shown) {
                eprintln!("{}: {error}", output.display());
            }
            said = Some(shown);
        }
        match events.wait(anyone.then_some(every)) {
            Woken::Timeout | Woken::Changed => {}
            Woken::NodeGone => return Ended::Gone,
            Woken::Stopped => return Ended::Stopped,
        }
    }
}

/// Says who streams from a USB camera while it's there, in a file of its
/// own that goes with it.
fn usb(events: &mut Events, options: &Options) -> ExitCode {
    let Ok(node) = fs::canonicalize(&options.source) else {
        return ExitCode::SUCCESS;
    };
    let output = match camera::camera_file(&options.sysfs, &node, &options.output) {
        Ok(output) => output,
        Err(error) => {
            eprintln!("{}: {error}", node.display());
            return ExitCode::FAILURE;
        }
    };
    tidy(&output);
    watch(events, &node, &output, &options.units, Some(&options.sysfs));
    match fs::remove_file(&output) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => {
            eprintln!("{}: {error}", output.display());
            ExitCode::FAILURE
        }
        _ => ExitCode::SUCCESS,
    }
}

/// Says who holds a loopback, following the source to each node it names.
fn loopback(events: &mut Events, options: &Options) -> ExitCode {
    let nobody = BTreeSet::new();
    let say_nobody = || {
        if let Err(error) = write(&options.output, &nobody) {
            eprintln!("{}: {error}", options.output.display());
        }
    };
    tidy(&options.output);
    let mut gone = false;
    loop {
        let Some(node) = camera::node_of(&options.sysfs, &options.source, options.card.as_deref())
        else {
            if !gone {
                say_nobody();
                gone = true;
            }
            if let Woken::Stopped = events.sleep(Duration::from_secs(1)) {
                return ExitCode::SUCCESS;
            }
            continue;
        };
        gone = false;
        let ended = watch(events, &node, &options.output, &options.units, None);
        say_nobody();
        if let Ended::Stopped = ended {
            return ExitCode::SUCCESS;
        }
        // So a node it can't watch is tried again, not spun on.
        if let Woken::Stopped = events.sleep(Duration::from_millis(500)) {
            return ExitCode::SUCCESS;
        }
    }
}

fn main() -> ExitCode {
    let Some(options) = options(std::env::args().skip(1)) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let mut events = match Events::new() {
        Ok(events) => events,
        Err(error) => {
            eprintln!("inotify: {error}");
            return ExitCode::FAILURE;
        }
    };
    if options.usb {
        usb(&mut events, &options)
    } else {
        loopback(&mut events, &options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use camera::tests::{Camera, scratch};

    fn named(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
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
        assert_eq!(
            (read.source.as_path(), read.output.as_path()),
            (Path::new("/run/d"), Path::new("/run/o"))
        );
        assert!(given("--usb /dev/video0 /run/camera-holders").unwrap().usb);
        assert!(given("/dev/video0").is_none());
        assert!(given("--card").is_none());
        assert!(given("--what /a /b").is_none());
    }
}
