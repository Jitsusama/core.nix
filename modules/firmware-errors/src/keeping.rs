//! The tests of keep.sh, run over stand-ins for the firmware's tables, the
//! kernel's files and the serial, and read back by the listing, so what
//! the keeping writes is held to what the command reads.

use crate::boots::{self, BEGAN, BERT_REGION, BERT_TABLE, PHAT_TABLE};
use crate::crash::Crash;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

/// The boot the stand-in kernel is running, as the folders name it.
const BOOT: &str = "6bccd85d80714583afd805c687f0cdb6";
/// When the stand-in kernel says it began, in seconds since 1970.
const BEGAN_AT: i64 = 1000;
/// An error region with no entries: its status block alone, all zero.
const EMPTY_REGION: [u8; 20] = [0; 20];

/// A machine's stand-ins, in a folder of their own.
struct Machine {
    root: PathBuf,
}

impl Machine {
    /// A machine whose firmware gives the tables named, at their paths
    /// under the tables' folder.
    fn giving(name: &str, tables: &[(&str, &[u8])]) -> Self {
        let root = std::env::temp_dir().join(format!(
            "firmware-errors-keep-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("tables")).expect("the tables' folder");
        for (path, bytes) in tables {
            let path = root.join("tables").join(path);
            fs::create_dir_all(path.parent().expect("a table's folder")).expect("a folder");
            fs::write(path, bytes).expect("a table");
        }
        let random = root.join("proc/sys/kernel/random");
        fs::create_dir_all(&random).expect("the kernel's folder");
        fs::write(
            random.join("boot_id"),
            "6bccd85d-8071-4583-afd8-05c687f0cdb6\n",
        )
        .expect("the boot's id");
        fs::create_dir_all(root.join("state")).expect("the state folder");
        fs::write(root.join("serial"), "stand-in-serial\n").expect("the serial");
        let machine = Self { root };
        machine.began_at(BEGAN_AT);
        machine
    }

    /// The tables a firmware such as the XPS 14's gives.
    fn with_every_table(name: &str) -> Self {
        Self::giving(
            name,
            &[
                ("PHAT", b"PHAT stand-in"),
                ("BERT", b"BERT stand-in"),
                ("data/BERT", &EMPTY_REGION),
            ],
        )
    }

    /// The kernel's clock set, so it now says the boot began at `btime`.
    fn began_at(&self, btime: i64) {
        let stat = format!("cpu  1 2 3 4\nbtime {btime}\nprocesses 9\n");
        fs::write(self.root.join("proc/stat"), stat).expect("the kernel's stat");
    }

    /// keep.sh run over the stand-ins: its exit code and what it said.
    fn keep(&self) -> (Option<i32>, String) {
        self.keep_with(Command::new("bash"))
    }

    /// The same, with an `mv` that always fails ahead on its PATH.
    fn keep_with_no_move(&self) -> (Option<i32>, String) {
        let shell = env::split_paths(&env::var_os("PATH").unwrap_or_default())
            .map(|folder| folder.join("bash"))
            .find(|bash| bash.is_file())
            .expect("bash on PATH");
        let tools = self.root.join("tools");
        fs::create_dir_all(&tools).expect("the stand-in tools' folder");
        let mv = tools.join("mv");
        fs::write(&mv, format!("#!{}\nexit 1\n", shell.display())).expect("a stand-in mv");
        fs::set_permissions(&mv, fs::Permissions::from_mode(0o755)).expect("mv made runnable");
        let path = env::join_paths(
            std::iter::once(tools)
                .chain(env::split_paths(&env::var_os("PATH").unwrap_or_default())),
        )
        .expect("a PATH");
        let mut command = Command::new(shell);
        command.env("PATH", path);
        self.keep_with(command)
    }

    fn keep_with(&self, mut command: Command) -> (Option<i32>, String) {
        let ran = command
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("keep.sh"))
            .env("STATE_DIRECTORY", self.root.join("state"))
            .env("FIRMWARE_ERRORS_TABLES", self.root.join("tables"))
            .env("FIRMWARE_ERRORS_SERIAL", self.root.join("serial"))
            .env("FIRMWARE_ERRORS_PROC", self.root.join("proc"))
            .output()
            .expect("keep.sh ran");
        (
            ran.status.code(),
            String::from_utf8_lossy(&ran.stderr).into_owned(),
        )
    }

    /// keep.sh run, holding it to ending done and saying nothing.
    fn kept_whole(&self) {
        assert_eq!(self.keep(), (Some(0), String::new()), "keep.sh failed");
    }

    /// Where it keeps a file for the running boot.
    fn kept(&self, name: &str) -> PathBuf {
        self.root.join("state").join(BOOT).join(name)
    }

    fn read(&self, name: &str) -> Vec<u8> {
        fs::read(self.kept(name)).unwrap_or_else(|error| panic!("{name} wasn't kept: {error}"))
    }
}

impl Drop for Machine {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn nothing_is_kept_where_the_firmware_gives_no_table() {
    let machine = Machine::giving("none", &[]);
    machine.kept_whole();
    let state = fs::read_dir(machine.root.join("state")).expect("the state folder");
    assert_eq!(state.count(), 0, "a boot was kept with no table");
}

#[test]
fn what_is_kept_is_what_the_listing_reads() {
    let machine = Machine::with_every_table("listed");
    machine.kept_whole();
    let kept = boots::kept_in(&machine.root.join("state")).expect("the boots");
    let found: Vec<_> = kept
        .iter()
        .map(|boot| (boot.boot.as_str(), boot.began, boot.crash.clone()))
        .collect();
    assert_eq!(found, [(BOOT, Some(BEGAN_AT), Some(Crash::None))]);
    assert_eq!(machine.read(PHAT_TABLE), b"PHAT stand-in");
    assert_eq!(machine.read("product-serial"), b"stand-in-serial\n");
    for name in [PHAT_TABLE, BERT_TABLE, BERT_REGION, "product-serial"] {
        let mode = fs::metadata(machine.kept(name))
            .expect("a copy")
            .permissions()
            .mode();
        assert_eq!(
            mode & 0o777,
            0o640,
            "{name} is kept {mode:o}, not for its group alone"
        );
    }
    // keep.sh names a copy it hasn't yet placed NAME.new, in that case.
    let names: Vec<String> = fs::read_dir(machine.kept(""))
        .expect("the boot's folder")
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| Path::new(name).extension() == Some(OsStr::new("new")))
        .collect();
    assert_eq!(names, Vec::<String>::new(), "a copy was left half placed");
}

#[test]
fn a_later_run_rewrites_no_table_kept_whole() {
    let machine = Machine::with_every_table("again");
    machine.kept_whole();
    let long_ago = SystemTime::UNIX_EPOCH + Duration::from_secs(5);
    fs::File::options()
        .write(true)
        .open(machine.kept(PHAT_TABLE))
        .and_then(|file| file.set_modified(long_ago))
        .expect("the copy's time");
    machine.kept_whole();
    let written = fs::metadata(machine.kept(PHAT_TABLE))
        .and_then(|copy| copy.modified())
        .expect("the copy's time");
    assert_eq!(written, long_ago, "a table kept whole was written again");
}

#[test]
fn a_copy_cut_short_is_written_again_whole() {
    let machine = Machine::with_every_table("short");
    machine.kept_whole();
    fs::write(machine.kept(PHAT_TABLE), b"PH").expect("a short copy");
    machine.kept_whole();
    assert_eq!(machine.read(PHAT_TABLE), b"PHAT stand-in");
}

#[test]
fn a_copy_is_placed_whole_or_not_at_all() {
    let machine = Machine::with_every_table("placed");
    machine.kept_whole();
    fs::write(machine.kept(PHAT_TABLE), b"PH").expect("a short copy");
    // With its move refused, a copy written in place would still land;
    // one written aside and moved in leaves the old copy as it was.
    let (exit, said) = machine.keep_with_no_move();
    assert_eq!(
        exit,
        Some(1),
        "a copy it couldn't place was passed over: {said}"
    );
    assert_eq!(
        machine.read(PHAT_TABLE),
        b"PH",
        "a copy was written in place"
    );
}

#[test]
fn when_the_boot_began_follows_the_clock_and_an_empty_one_is_written() {
    let machine = Machine::with_every_table("began");
    machine.kept_whole();
    assert_eq!(machine.read(BEGAN), b"1000\n");
    machine.began_at(BEGAN_AT + 2);
    machine.kept_whole();
    assert_eq!(
        machine.read(BEGAN),
        b"1002\n",
        "a later run left a start the clock moved"
    );
    fs::write(machine.kept(BEGAN), b"").expect("an empty start");
    machine.kept_whole();
    assert_eq!(machine.read(BEGAN), b"1002\n", "an empty start was left");
}

#[test]
fn a_table_that_cant_be_copied_fails_once_the_others_are_kept() {
    let machine = Machine::with_every_table("lost");
    let region = machine.root.join("tables/data/BERT");
    fs::set_permissions(&region, fs::Permissions::from_mode(0o000)).expect("the region closed");
    let (exit, said) = machine.keep();
    assert_eq!(
        exit,
        Some(1),
        "a table it couldn't copy was passed over: {said}"
    );
    assert_eq!(machine.read(PHAT_TABLE), b"PHAT stand-in");
    assert!(
        !machine.kept(BERT_REGION).exists(),
        "a region it couldn't read was kept"
    );
}
