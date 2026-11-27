//! The program itself watching a loopback that has no camera behind it, run
//! as its unit runs it: the nobody it says is written once it can be, and
//! only then is the run ready; a trouble is the run's, not the step's, so
//! it's said once whether the nobody or the watch of a camera met it; and
//! the node a camera's watch couldn't say in `.devices/` is said once it
//! can be, its trouble said once, and only then is the run ready; and as
//! its camera goes or its watch stops, the node file goes only once its
//! nobody is written, and one it couldn't remove as its camera went is
//! tried again; and a stop tries once more what's still owed; and a camera
//! it moves to is said held while its node file still says the camera
//! before, its holders file written again when a USB camera's run removes
//! it.

mod common;

use common::{Watch, complaints_of, lock, ready, right_again};
use std::fs::{self, File, Permissions};
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

#[test]
fn a_loopbacks_nobody_it_couldnt_write_is_written_once_it_can_be() {
    let watch = Watch::locked("unwritten");
    watch.until(
        |watch| watch.read("complaints").contains("Permission denied"),
        "a nobody it couldn't write went unsaid",
    );
    // Two more looks' worth, to see it neither ready nor complaining again.
    sleep(Duration::from_millis(2500));
    assert_eq!(
        ready(&watch.read("log")),
        0,
        "a nobody it couldn't write was said ready"
    );
    assert_eq!(
        watch
            .read("complaints")
            .matches("Permission denied")
            .count(),
        1,
        "a write failing each look was complained of each look"
    );
    fs::set_permissions(&watch.folder, Permissions::from_mode(0o755)).unwrap();
    watch.until(
        |watch| fs::read_to_string(watch.file()).is_ok_and(|said| said.is_empty()),
        "a nobody it couldn't write wasn't tried again",
    );
    watch.until(
        |watch| ready(&watch.read("log")) > 0,
        "a nobody written at last wasn't said ready",
    );
    watch.until(
        |watch| right_again(watch, &watch.file()) > 0,
        "a write that worked again went unsaid",
    );
    assert_eq!(
        ready(&watch.read("log")),
        1,
        "the watch didn't say once that it's ready: {}",
        watch.read("log")
    );
}

#[test]
fn a_trouble_the_nobody_met_is_the_cameras_watch_to_clear() {
    let watch = Watch::locked("handed");
    watch.until(
        |watch| complaints_of(watch, &watch.file()) == 1,
        "a nobody it couldn't write went unsaid",
    );
    // A camera nothing on the machine holds comes while the folder is still
    // locked, so its watch meets the trouble the nobody did.
    fs::write(watch.root.join("source"), "/dev/random").unwrap();
    watch.until(
        |watch| watch.read("complaints").contains(".devices/Stand-in"),
        "the camera that came wasn't watched",
    );
    // Past the watch's next look, two seconds on, which writes again.
    sleep(Duration::from_millis(2500));
    assert_eq!(
        complaints_of(&watch, &watch.file()),
        1,
        "the camera's watch complained again of what the nobody had: {:?}",
        watch.read("complaints")
    );
    fs::set_permissions(&watch.folder, Permissions::from_mode(0o755)).unwrap();
    watch.until(
        |watch| right_again(watch, &watch.file()) == 1,
        "the camera's watch didn't say right again what the nobody met",
    );
    watch.until(
        Watch::named,
        "the node the camera's watch couldn't say wasn't said once it could be",
    );
    watch.until(
        |watch| ready(&watch.read("log")) == 1,
        "a camera's file written at last wasn't said ready",
    );
    // A watch with its files right and nobody holding the camera looks only
    // as the camera is opened or closed, so it's made to look, each look
    // finding its files right.
    watch.looked_twice();
    assert_eq!(
        right_again(&watch, &watch.file()),
        1,
        "a file right on each look was said right again each look: {:?}",
        watch.read("complaints")
    );
}

#[test]
fn a_camera_is_ready_only_once_its_node_is_said() {
    let watch = Watch::unnamed("unnamed");
    watch.until(
        |watch| complaints_of(watch, &watch.device()) == 1,
        "a node it couldn't say went unsaid",
    );
    watch.until(
        |watch| fs::read_to_string(watch.file()).is_ok_and(|said| said.is_empty()),
        "a camera nothing holds wasn't said held by nobody",
    );
    // Past the watch's next look, two seconds on, which tries the node again.
    sleep(Duration::from_millis(2500));
    assert_eq!(
        ready(&watch.read("log")),
        0,
        "a camera whose node went unsaid was said ready"
    );
    assert_eq!(
        complaints_of(&watch, &watch.device()),
        1,
        "a node it couldn't say each look was complained of each look: {:?}",
        watch.read("complaints")
    );
    fs::remove_file(watch.folder.join(".devices")).unwrap();
    watch.until(
        Watch::named,
        "a node it couldn't say wasn't said once it could be",
    );
    watch.until(
        |watch| ready(&watch.read("log")) == 1,
        "a camera whose node was said at last wasn't said ready",
    );
    watch.until(
        |watch| right_again(watch, &watch.device()) == 1,
        "a node said at last wasn't said right again",
    );
    watch.looked_twice();
    assert_eq!(
        right_again(&watch, &watch.device()),
        1,
        "a node right on each look was said right again each look: {:?}",
        watch.read("complaints")
    );
}

#[test]
fn a_node_file_it_couldnt_remove_as_its_camera_went_is_tried_again() {
    let watch = Watch::owned("removed");
    let camera = watch.root.join("camera");
    watch.until(
        |watch| watch.names(&camera),
        "the camera's node went unsaid",
    );
    watch.until(
        |watch| ready(&watch.read("log")) == 1,
        "a camera whose files were written wasn't said ready",
    );
    // Held as it goes, so the nobody said after is the watch's doing.
    let held = File::open(&camera).unwrap();
    watch.until(
        |watch| fs::read_to_string(watch.file()).is_ok_and(|said| !said.is_empty()),
        "a camera held wasn't said held",
    );
    // Its folder locked, so removing the node file fails; nothing is owed,
    // so no look writes it again.
    let devices = watch.folder.join(".devices");
    fs::set_permissions(&devices, Permissions::from_mode(0o555)).unwrap();
    assert!(
        File::create(devices.join("probe")).is_err(),
        "a locked folder still took a file, as it does for root"
    );
    // Gone, then let go, so the look its closing wakes finds it gone.
    fs::remove_file(&camera).unwrap();
    drop(held);
    watch.until(
        |watch| complaints_of(watch, &watch.device()) == 1,
        "a node file it couldn't remove went unsaid",
    );
    watch.until(
        |watch| fs::read_to_string(watch.file()).is_ok_and(|said| said.is_empty()),
        "a camera gone wasn't said held by nobody",
    );
    // Past two more of the one-second tries with no camera.
    sleep(Duration::from_millis(2500));
    assert_eq!(
        complaints_of(&watch, &watch.device()),
        1,
        "a node file it couldn't remove each try was complained of each try: {:?}",
        watch.read("complaints")
    );
    fs::set_permissions(&devices, Permissions::from_mode(0o755)).unwrap();
    watch.until(
        |watch| !watch.device().exists(),
        "a node file it couldn't remove wasn't tried again",
    );
    watch.until(
        |watch| right_again(watch, &watch.device()) == 1,
        "a node file removed at last wasn't said right again",
    );
}

#[test]
fn a_loopback_with_no_camera_is_ready_only_once_its_node_file_is_gone() {
    // A node file a killed run left, as a folder with a file in it, so
    // removing it fails.
    let watch = Watch::started("stale", "", |folder| {
        let stale = folder.join(".devices/Stand-in");
        fs::create_dir_all(&stale).unwrap();
        fs::write(stale.join("kept"), "").unwrap();
    });
    watch.until(
        |watch| complaints_of(watch, &watch.device()) == 1,
        "a node file it couldn't remove went unsaid",
    );
    watch.until(
        |watch| fs::read_to_string(watch.file()).is_ok_and(|said| said.is_empty()),
        "a loopback with no camera wasn't said held by nobody",
    );
    // Past two more of the one-second tries with no camera.
    sleep(Duration::from_millis(2500));
    assert_eq!(
        ready(&watch.read("log")),
        0,
        "a loopback whose node file stayed was said ready"
    );
    fs::remove_dir_all(watch.device()).unwrap();
    watch.until(
        |watch| ready(&watch.read("log")) == 1,
        "a loopback whose node file went wasn't said ready",
    );
}

#[test]
fn a_node_file_stays_until_its_loopbacks_nobody_is_written() {
    let watch = Watch::stale("ordered");
    watch.until(
        |watch| complaints_of(watch, &watch.file()) == 1,
        "a nobody it couldn't write went unsaid",
    );
    // Past two more of the one-second tries with no camera.
    sleep(Duration::from_millis(2500));
    assert!(
        watch.device().exists(),
        "a node file was removed before its loopback's nobody was written"
    );
    fs::set_permissions(&watch.folder, Permissions::from_mode(0o755)).unwrap();
    watch.until(
        |watch| !watch.device().exists(),
        "a node file stayed once its loopback's nobody was written",
    );
    watch.until(
        |watch| ready(&watch.read("log")) == 1,
        "a loopback whose files were right wasn't said ready",
    );
}

#[test]
fn a_stopped_watch_keeps_its_node_named_while_its_nobody_cant_be_written() {
    let mut watch = Watch::owned("stopped");
    let camera = watch.root.join("camera");
    watch.until(
        |watch| ready(&watch.read("log")) == 1,
        "a camera whose files were written wasn't said ready",
    );
    // Held as it stops, so its file says a holder a reader looks up by
    // the node.
    let held = File::open(&camera).unwrap();
    watch.until(
        |watch| fs::read_to_string(watch.file()).is_ok_and(|said| !said.is_empty()),
        "a camera held wasn't said held",
    );
    // The holders file's folder locked, the node files' folder not.
    fs::set_permissions(&watch.folder, Permissions::from_mode(0o555)).unwrap();
    assert!(
        File::create(watch.folder.join("probe")).is_err(),
        "a locked folder still took a file, as it does for root"
    );
    watch.stop();
    drop(held);
    assert!(
        fs::read_to_string(watch.file()).is_ok_and(|said| !said.is_empty()),
        "a stopped watch's holder went unsaid with its nobody unwritten"
    );
    assert!(
        watch.names(&camera),
        "a stopped watch unnamed its node while its file still said a holder"
    );
}

#[test]
fn a_stop_with_no_camera_tries_its_nobody_and_node_file_once_more() {
    let mut watch = Watch::stale("lasting");
    watch.until(
        |watch| complaints_of(watch, &watch.file()) == 1,
        "a nobody it couldn't write went unsaid",
    );
    // Unlocked and stopped at once, well inside the second before its next
    // try, so only the stop's own try can write them.
    fs::set_permissions(&watch.folder, Permissions::from_mode(0o755)).unwrap();
    watch.stop();
    assert!(
        fs::read_to_string(watch.file()).is_ok_and(|said| said.is_empty()),
        "a stop with no camera didn't try its nobody once more"
    );
    assert!(
        !watch.device().exists(),
        "a stop with no camera didn't try its node file once more"
    );
    // A try of the run's own would have said it ready, as the stop's
    // doesn't, so on a slow machine this shows which one wrote them.
    assert_eq!(
        ready(&watch.read("log")),
        0,
        "the run's own try wrote them before the stop could"
    );
}

#[test]
fn a_stop_whose_nobody_still_cant_be_written_keeps_its_node_file() {
    let mut watch = Watch::stale("kept");
    watch.until(
        |watch| complaints_of(watch, &watch.file()) == 1,
        "a nobody it couldn't write went unsaid",
    );
    // Stopped still locked, so the stop's own try at its nobody fails too,
    // and its holders file still says the holder the killed run left: the
    // node file a reader finds that holder's camera by must stay.
    watch.stop();
    assert!(
        watch.named(),
        "a stop removed the node file of a holder its holders file still says"
    );
}

#[test]
fn a_stop_as_its_camera_goes_tries_its_nobody_and_node_file_once_more() {
    let mut watch = Watch::owned("leaving");
    let camera = watch.root.join("camera");
    watch.until(
        |watch| ready(&watch.read("log")) == 1,
        "a camera whose files were written wasn't said ready",
    );
    // Held as it goes, so the nobody said after is a write of its own.
    let held = File::open(&camera).unwrap();
    watch.until(
        |watch| fs::read_to_string(watch.file()).is_ok_and(|said| !said.is_empty()),
        "a camera held wasn't said held",
    );
    // The holders file's folder locked, the node files' folder not.
    fs::set_permissions(&watch.folder, Permissions::from_mode(0o555)).unwrap();
    assert!(
        File::create(watch.folder.join("probe")).is_err(),
        "a locked folder still took a file, as it does for root"
    );
    // Gone, then let go, so the look its closing wakes finds it gone.
    fs::remove_file(&camera).unwrap();
    drop(held);
    watch.until(
        |watch| complaints_of(watch, &watch.file()) == 1,
        "a nobody it couldn't write as its camera went went unsaid",
    );
    // Unlocked and stopped at once: in the half second the run waits
    // after a watch, or, on a slow machine, in the second between tries
    // with no camera. Either way it's the stop's own try that writes them.
    fs::set_permissions(&watch.folder, Permissions::from_mode(0o755)).unwrap();
    watch.stop();
    assert!(
        fs::read_to_string(watch.file()).is_ok_and(|said| said.is_empty()),
        "a stop as its camera went didn't try its nobody once more"
    );
    assert!(
        !watch.device().exists(),
        "a stop as its camera went didn't try its node file once more"
    );
}

#[test]
fn a_stopped_watch_unnames_its_node_once_its_nobody_is_written() {
    let mut watch = Watch::owned("stopping");
    let camera = watch.root.join("camera");
    watch.until(
        |watch| ready(&watch.read("log")) == 1,
        "a camera whose files were written wasn't said ready",
    );
    // Held as it stops, so the nobody said after is the stop's doing.
    let held = File::open(&camera).unwrap();
    watch.until(
        |watch| fs::read_to_string(watch.file()).is_ok_and(|said| !said.is_empty()),
        "a camera held wasn't said held",
    );
    watch.stop();
    drop(held);
    assert!(
        fs::read_to_string(watch.file()).is_ok_and(|said| said.is_empty()),
        "a stopped watch wasn't said held by nobody"
    );
    assert!(
        !watch.device().exists(),
        "a stopped watch left its node named with its nobody written"
    );
}

#[test]
fn a_gone_cameras_node_file_stays_until_its_nobody_is_written() {
    let watch = Watch::owned("went");
    let camera = watch.root.join("camera");
    watch.until(
        |watch| ready(&watch.read("log")) == 1,
        "a camera whose files were written wasn't said ready",
    );
    // Held as it goes, so the nobody said after is a write of its own.
    let held = File::open(&camera).unwrap();
    watch.until(
        |watch| fs::read_to_string(watch.file()).is_ok_and(|said| !said.is_empty()),
        "a camera held wasn't said held",
    );
    // The holders file's folder locked, the node files' folder not.
    fs::set_permissions(&watch.folder, Permissions::from_mode(0o555)).unwrap();
    assert!(
        File::create(watch.folder.join("probe")).is_err(),
        "a locked folder still took a file, as it does for root"
    );
    // Gone, then let go, so the look its closing wakes finds it gone.
    fs::remove_file(&camera).unwrap();
    drop(held);
    watch.until(
        |watch| complaints_of(watch, &watch.file()) == 1,
        "a nobody it couldn't write as its camera went went unsaid",
    );
    // Past a look of the watch's, every two seconds, and a try after it.
    sleep(Duration::from_millis(3500));
    assert!(
        watch.device().exists(),
        "a gone camera's node file was removed before its nobody was written"
    );
    fs::set_permissions(&watch.folder, Permissions::from_mode(0o755)).unwrap();
    watch.until(
        |watch| fs::read_to_string(watch.file()).is_ok_and(|said| said.is_empty()),
        "a gone camera's nobody it couldn't write wasn't tried again",
    );
    watch.until(
        |watch| !watch.device().exists(),
        "a gone camera's node file stayed once its nobody was written",
    );
}

/// A watch moved to another camera while its node file still says the one
/// before: the node files' folder, locked, and the camera moved to, held
/// by the test.
struct Moved {
    devices: PathBuf,
    writing: PathBuf,
    other: PathBuf,
    _holding: File,
}

/// Moves the watch, its camera held, to another camera the test holds,
/// so that its node file can't be replaced or removed, and waits for its
/// complaints of both.
fn moved(watch: &Watch) -> Moved {
    let camera = watch.root.join("camera");
    watch.until(
        |watch| ready(&watch.read("log")) == 1,
        "a camera whose files were written wasn't said ready",
    );
    // Held as it goes, so the look its closing wakes finds it gone.
    let held = File::open(&camera).unwrap();
    watch.until(
        |watch| fs::read_to_string(watch.file()).is_ok_and(|said| !said.is_empty()),
        "a camera held wasn't said held",
    );
    // A folder where the node file is written before it replaces the one
    // there, so the node moved to can't be said even once the run widens
    // the node files' folder, as it does before writing; and that folder
    // locked, so the node file saying the camera before can't be removed.
    let devices = watch.folder.join(".devices");
    let writing = devices.join(format!(".Stand-in.{}", watch.program.id()));
    fs::create_dir(&writing).unwrap();
    fs::set_permissions(&devices, Permissions::from_mode(0o555)).unwrap();
    assert!(
        File::create(devices.join("probe")).is_err(),
        "a locked folder still took a file, as it does for root"
    );
    // The camera it moves to, another node nothing else holds, held by the
    // test, so a holder is there to be said.
    let other = watch.root.join("other");
    symlink("/dev/full", &other).unwrap();
    let holding = File::open(&other).unwrap();
    fs::write(watch.root.join("source"), other.to_str().unwrap()).unwrap();
    fs::remove_file(&camera).unwrap();
    drop(held);
    // Once for the node file it couldn't remove, once for the node it
    // couldn't say, each said before the look's holders file is written.
    watch.until(
        |watch| complaints_of(watch, &watch.device()) == 2,
        "a node file it couldn't remove, or a node it couldn't say, went unsaid",
    );
    assert!(
        watch.named(),
        "the node file stopped saying the camera before while it couldn't be replaced"
    );
    Moved {
        devices,
        writing,
        other,
        _holding: holding,
    }
}

#[test]
fn a_camera_moved_to_is_said_held_while_its_node_file_says_the_one_before() {
    let watch = Watch::owned("moved");
    let moved = moved(&watch);
    // Waited for, as the look that complained of the node writes the
    // holders file only after; nobody, said as the camera before went,
    // stays if the camera moved to is said off.
    watch.until(
        |watch| fs::read_to_string(watch.file()).is_ok_and(|said| !said.is_empty()),
        "the camera it moved to was said off while held, as its node file said the one before",
    );
    fs::set_permissions(&moved.devices, Permissions::from_mode(0o755)).unwrap();
    fs::remove_dir(&moved.writing).unwrap();
    watch.until(
        |watch| watch.names(&moved.other),
        "the camera it moved to wasn't named once it could be",
    );
    watch.until(
        |watch| fs::read_to_string(watch.file()).is_ok_and(|said| !said.is_empty()),
        "the camera it moved to wasn't said held once it was named",
    );
}

#[test]
fn a_moved_cameras_holders_file_a_usb_cameras_run_removed_is_written_again() {
    let watch = Watch::owned("forgotten");
    let moved = moved(&watch);
    watch.until(
        |watch| fs::read_to_string(watch.file()).is_ok_and(|said| !said.is_empty()),
        "the camera it moved to was said off while held",
    );
    // A USB camera given the node the camera before had, whose run forgets
    // the files saying it, as it does when its unit stops: both go, since
    // the watch widened the node files' folder before its last write.
    let forgot = Command::new(env!("CARGO_BIN_EXE_loopback-holders"))
        .arg("--sysfs")
        .arg(watch.root.join("sys"))
        .args(["--usb", "--forget", "/dev/random"])
        .arg(&watch.folder)
        .output()
        .unwrap();
    assert!(forgot.status.success(), "the forget failed: {forgot:?}");
    // Said only once both its files went, the holders file first, which
    // the watch may already have written again.
    assert!(
        String::from_utf8_lossy(&forgot.stdout).contains("forgot \"Stand-in\""),
        "the forget didn't find the node file saying the camera before: {forgot:?}"
    );
    // The camera moved to is still held, so its watch looks every two
    // seconds, its holders the same as when it last wrote them.
    watch.until(
        |watch| fs::read_to_string(watch.file()).is_ok_and(|said| !said.is_empty()),
        "a holders file removed under its watch wasn't written again",
    );
    fs::set_permissions(&moved.devices, Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn a_loopback_with_no_camera_and_no_node_files_folder_is_ready() {
    // A file where `.devices/` would be, so no node file can be there.
    let watch = Watch::started("foldless", "", |folder| {
        fs::write(folder.join(".devices"), "").unwrap();
    });
    watch.until(
        |watch| ready(&watch.read("log")) == 1,
        "a loopback with no node files' folder wasn't said ready",
    );
    assert_eq!(
        complaints_of(&watch, &watch.device()),
        0,
        "a node file that can't be there was complained of: {:?}",
        watch.read("complaints")
    );
}
