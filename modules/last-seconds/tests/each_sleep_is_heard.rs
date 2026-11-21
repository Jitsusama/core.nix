//! The recorder run whole over a stand-in machine, and signalled as the
//! sleep's hook signals it: how its loop carries the sleeps and wakes it
//! was told of from one second to the next, which the unit tests, holding
//! the counts themselves, can't reach. A slip there leaves a sleep
//! unrecorded, so a reset asleep would read as a cut.

use last_seconds::record::Record;
use last_seconds::{RING, STATE_VARIABLE, ring};
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

/// How often the stand-in's recorder reads, in milliseconds.
const EVERY: u64 = 50;
/// How long it holds off writing after a sleep that never says it woke,
/// in milliseconds: sixty reads' time, as last-seconds-keep's
/// `ASLEEP_AT_MOST` says.
const HOLD: u64 = 60 * EVERY;
/// The stand-in boot's id, as the kernel gives it and as its folder is
/// named.
const BOOT: &str = "6bccd85d-8071-4583-afd8-05c687f0cdb6";
const FOLDER: &str = "6bccd85d80714583afd805c687f0cdb6";
/// How long a record the recorder owes may take, well past its read, for
/// a builder at full load.
const SOON: Duration = Duration::from_secs(5);

/// A recorder kept over a stand-in machine of its own, stopped and cleared
/// away when dropped.
struct Keeping {
    recorder: Child,
    folder: PathBuf,
}

impl Keeping {
    fn start(name: &str) -> Self {
        let folder =
            std::env::temp_dir().join(format!("last-seconds-{name}-{}", std::process::id()));
        // A run killed before it cleared away leaves its folder, whose
        // records would read as this recorder's before its handlers are set.
        let _ = fs::remove_dir_all(&folder);
        let machine = folder.join("machine");
        let random = machine.join("proc/sys/kernel/random");
        fs::create_dir_all(&random).expect("the stand-in's folders");
        fs::write(random.join("boot_id"), format!("{BOOT}\n")).expect("the boot's id");
        fs::write(machine.join("proc/stat"), "btime 1795241514\n").expect("the boot's start");
        let recorder = Command::new(env!("CARGO_BIN_EXE_last-seconds-keep"))
            .args([
                "--root",
                &machine.to_string_lossy(),
                "--every",
                &EVERY.to_string(),
            ])
            .env(STATE_VARIABLE, folder.join("state"))
            .spawn()
            .expect("the recorder started");
        let mut keeping = Self { recorder, folder };
        // Its handlers are set before its first record, and a signal
        // before them would end it.
        assert!(
            keeping.until(SOON, |records| !records.is_empty()),
            "the recorder wrote nothing"
        );
        keeping
    }

    /// Sends a signal as the hook's `systemctl kill` does, to the process.
    fn signal(&self, signal: libc::c_int) {
        let pid = libc::pid_t::try_from(self.recorder.id()).expect("a pid");
        // SAFETY: a signal to the recorder this started and still holds.
        let sent = unsafe { libc::kill(pid, signal) };
        assert_eq!(sent, 0, "the recorder couldn't be signalled");
    }

    /// The ring's records, oldest first; none while it can't be read.
    fn records(&self) -> Vec<Record> {
        ring::read(&self.folder.join("state").join(FOLDER).join(RING))
            .map(|(_, records)| records)
            .unwrap_or_default()
    }

    fn newest(&self) -> Record {
        self.records().pop().expect("a record")
    }

    /// Whether the records come to hold `what` within `within`. A recorder
    /// that ended is said as such, rather than as what it never wrote.
    fn until(&mut self, within: Duration, what: impl Fn(&[Record]) -> bool) -> bool {
        let given = Instant::now();
        while given.elapsed() < within {
            if what(&self.records()) {
                return true;
            }
            let ended = self.recorder.try_wait().expect("the recorder's state");
            assert!(ended.is_none(), "the recorder ended: {ended:?}");
            // Five looks to each of the recorder's reads, so a record is
            // seen soon after it lands.
            thread::sleep(Duration::from_millis(EVERY / 5));
        }
        false
    }

    /// The first record newer than `sequence` saying it's asleep or not as
    /// asked, if one comes within `within`.
    fn comes(&mut self, sequence: u32, asleep: bool, within: Duration) -> Option<Record> {
        let first = |records: &[Record]| {
            records
                .iter()
                .find(|record| record.sequence > sequence && record.asleep == asleep)
                .cloned()
        };
        self.until(within, |records| first(records).is_some())
            .then(|| first(&self.records()))
            .flatten()
    }
}

impl Drop for Keeping {
    fn drop(&mut self) {
        let _ = self.recorder.kill();
        let _ = self.recorder.wait();
        let _ = fs::remove_dir_all(&self.folder);
    }
}

#[test]
fn a_wake_said_with_no_sleep_before_it_leaves_the_next_sleep_heard() {
    let mut keeping = Keeping::start("stray-wake");
    // As to a recorder started mid-sleep, which hears only its wake.
    keeping.signal(libc::SIGUSR2);
    let at_the_wake = keeping.newest().sequence;
    // Two seconds' starts past it, so the loop has carried it on.
    assert!(
        keeping.until(SOON, |records| records
            .last()
            .is_some_and(|record| record.sequence > at_the_wake + 1)),
        "the recorder stopped writing after a wake with no sleep before it"
    );
    let before = keeping.newest().sequence;
    keeping.signal(libc::SIGUSR1);
    assert!(
        keeping.comes(before, true, SOON).is_some(),
        "a sleep after a wake said with no sleep before it was never recorded"
    );
}

#[test]
fn a_sleep_that_never_says_it_woke_is_let_go_and_the_next_is_heard() {
    let mut keeping = Keeping::start("never-woke");
    let before = keeping.newest().sequence;
    keeping.signal(libc::SIGUSR1);
    let asleep = keeping
        .comes(before, true, SOON)
        .expect("a sleep was never recorded");
    let let_go = keeping
        .comes(asleep.sequence, false, Duration::from_millis(HOLD) + SOON)
        .expect("a sleep that never said it woke held the ring past sixty reads' time");
    // Held to the recorder's own steady clock, which a slow test can't make late and a step of
    // the wall clock can't move.
    let held = let_go.since_boot_ms - asleep.since_boot_ms;
    assert!(
        held >= HOLD,
        "a sleep that never said it woke was let go after {held} ms, before sixty reads' time"
    );
    keeping.signal(libc::SIGUSR1);
    assert!(
        keeping.comes(let_go.sequence, true, SOON).is_some(),
        "after a sleep was let go, the next was never recorded"
    );
}
