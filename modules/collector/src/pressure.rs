//! Waits the kernel counts, kept as spans beside what programs send, so a
//! moment read back says whether the machine made them wait.
//!
//! The kernel adds up, for the whole machine and for each cgroup, how
//! long something there waited for the processor, for memory or for the
//! disk (pressure stall information). Read each second, a second in which
//! something waited at least a tenth of it starts a wait, and three quiet
//! seconds end it.

use std::fmt::Write as _;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

use crate::otlp::Span;
use crate::store::Store;

/// The share of a reading's time spent waiting that makes it a waiting
/// one: a tenth, past which a program that waits shows it.
const WAITING: f64 = 0.1;

/// Quiet readings in a row that end a wait.
const QUIET: u32 = 3;

/// The longest a wait goes before it's kept, in nanoseconds.
const LONGEST: i64 = 60_000_000_000;

const RESOURCES: [&str; 3] = ["cpu", "memory", "io"];

/// The slices whose waits are Joel's: what he runs, and the session that
/// draws it.
const SLICES: [&str; 2] = ["app.slice", "session.slice"];

/// One place whose waits for one resource are read.
#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    /// Where, as a span names it: "the machine", "app.slice".
    pub place: String,
    pub resource: &'static str,
    pub file: PathBuf,
}

/// What a pressure file says: how long, in microseconds since boot, at
/// least one task there waited (`some`), and every task at once (`full`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Totals {
    pub some: u64,
    pub full: u64,
}

/// A wait that ended, from the reading before its first waiting second to
/// the last one.
#[derive(Debug, Clone, PartialEq)]
pub struct Wait {
    /// Unix nanoseconds.
    pub start: i64,
    pub end: i64,
    pub waited_us: u64,
    pub full_us: u64,
    /// The most of any one reading's time spent waiting, from 0 to 1.
    pub worst: f64,
}

/// The places read: the machine's memory and disk, and the processor,
/// memory and disk of the slices the account's own programs run in,
/// found below the account's manager in `cgroup`, this process's line
/// of /proc/self/cgroup. The machine's processor is left out: under
/// builds something always waits for it, which says nothing.
pub fn sources(root: &Path, cgroup: &str) -> Vec<Source> {
    let mut found: Vec<Source> = ["memory", "io"]
        .into_iter()
        .map(|resource| Source {
            place: "the machine".into(),
            resource,
            file: root.join("proc/pressure").join(resource),
        })
        .collect();
    let Some(manager) = manager(cgroup) else {
        return found;
    };
    let manager = root.join("sys/fs/cgroup").join(manager);
    for slice in SLICES {
        for resource in RESOURCES {
            found.push(Source {
                place: slice.into(),
                resource,
                file: manager.join(slice).join(format!("{resource}.pressure")),
            });
        }
    }
    found
}

/// The account's manager's cgroup, without its leading slash, from this
/// process's line: everything up to its `user@` service.
fn manager(cgroup: &str) -> Option<String> {
    let path = cgroup.lines().find_map(|line| line.strip_prefix("0::/"))?;
    let mut kept = Vec::new();
    for part in path.split('/') {
        kept.push(part);
        if part.starts_with("user@") && part.ends_with(".service") {
            return Some(kept.join("/"));
        }
    }
    None
}

/// The totals a pressure file holds, or none when it holds no `some` line.
pub fn totals(text: &str) -> Option<Totals> {
    let total = |kind: &str| {
        text.lines()
            .find(|line| line.starts_with(kind))?
            .split_whitespace()
            .find_map(|field| field.strip_prefix("total="))?
            .parse()
            .ok()
    };
    Some(Totals {
        some: total("some ")?,
        full: total("full ").unwrap_or(0),
    })
}

/// One source's readings, and the wait they're in.
#[derive(Debug, Default)]
pub struct Waits {
    last: Option<(i64, Totals)>,
    open: Option<Wait>,
    quiet: u32,
}

impl Waits {
    /// Takes the reading at `at`, Unix nanoseconds, and gives the wait it
    /// ends, if it ends one.
    pub fn read(&mut self, at: i64, totals: Totals) -> Option<Wait> {
        let (then, before) = self.last.replace((at, totals))?;
        let waited = totals.some.saturating_sub(before.some);
        let full = totals.full.saturating_sub(before.full);
        #[expect(
            clippy::cast_precision_loss,
            reason = "microseconds in a second, far inside f64's precision"
        )]
        let share = waited as f64 * 1000.0 / (at - then).max(1) as f64;
        if share >= WAITING {
            self.quiet = 0;
            let wait = self.open.get_or_insert(Wait {
                start: then,
                end: at,
                waited_us: 0,
                full_us: 0,
                worst: 0.0,
            });
            wait.end = at;
            wait.waited_us += waited;
            wait.full_us += full;
            wait.worst = wait.worst.max(share);
            if at - wait.start >= LONGEST {
                return self.open.take();
            }
            return None;
        }
        self.open.as_ref()?;
        self.quiet += 1;
        if self.quiet >= QUIET {
            self.quiet = 0;
            return self.open.take();
        }
        None
    }
}

/// The sources being read, each with its readings so far.
pub struct Watching(Vec<(Source, Waits)>);

impl Watching {
    /// Takes every source's first reading at once, so what's read later is
    /// measured from now. A file that can't be read is passed over each
    /// time: a cgroup without one of the controllers, or a kernel without
    /// pressure information.
    pub fn start(sources: Vec<Source>) -> Self {
        let mut watching = Watching(sources.into_iter().map(|s| (s, Waits::default())).collect());
        watching.read(now());
        watching
    }

    /// Reads each source once a second, keeping each wait as it ends.
    pub fn run(mut self, store: &Mutex<Store>) -> ! {
        loop {
            thread::sleep(Duration::from_secs(1));
            let waits = self.read(now());
            if waits.is_empty() {
                continue;
            }
            let mut store = store
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Err(e) = store.add(&waits, now()) {
                eprintln!("{e}");
            }
        }
    }

    fn read(&mut self, at: i64) -> Vec<Span> {
        let mut ended = Vec::new();
        for (source, waits) in &mut self.0 {
            let Some(totals) = fs::read_to_string(&source.file)
                .ok()
                .as_deref()
                .and_then(totals)
            else {
                continue;
            };
            if let Some(wait) = waits.read(at, totals) {
                ended.push(span(source, &wait, (random(16), random(8))));
            }
        }
        ended
    }
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|since| i64::try_from(since.as_nanos()).ok())
        .unwrap_or(0)
}

/// `bytes` random bytes as hex, an id no other span has.
fn random(bytes: usize) -> String {
    let mut taken = vec![0; bytes];
    if fs::File::open("/dev/urandom")
        .and_then(|mut source| source.read_exact(&mut taken))
        .is_err()
    {
        // The time is unlike any other wait's start, which is enough to
        // keep two apart.
        taken = now()
            .to_le_bytes()
            .iter()
            .copied()
            .cycle()
            .take(bytes)
            .collect();
    }
    taken.iter().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    })
}

/// The span a wait is kept as, in a trace of its own.
pub fn span(source: &Source, wait: &Wait, ids: (String, String)) -> Span {
    Span {
        trace: ids.0,
        id: ids.1,
        parent: None,
        name: format!("{} waits in {}", source.resource, source.place),
        service: "kernel".into(),
        start: wait.start,
        end: wait.end,
        status: None,
        attributes: Map::<String, Value>::from_iter([
            ("waited_ms".into(), json!(wait.waited_us / 1000)),
            ("full_ms".into(), json!(wait.full_us / 1000)),
            ("worst_percent".into(), json!(percent(wait.worst))),
        ]),
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a share from 0 to 1, as a whole percent"
)]
fn percent(share: f64) -> u64 {
    (share * 100.0).round().clamp(0.0, 100.0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: i64 = 1_000_000_000;

    fn at(totals: &[u64]) -> Vec<Option<Wait>> {
        let mut waits = Waits::default();
        totals
            .iter()
            .zip(0..)
            .map(|(&some, second)| {
                waits.read(
                    second * SECOND,
                    Totals {
                        some,
                        full: some / 2,
                    },
                )
            })
            .collect()
    }

    #[test]
    fn a_pressure_file_gives_both_totals_and_a_cpu_file_without_full_gives_none_full() {
        let memory = "some avg10=0.31 avg60=0.08 avg300=0.02 total=1517003\n\
                      full avg10=0.10 avg60=0.03 avg300=0.00 total=681220\n";
        assert_eq!(
            totals(memory),
            Some(Totals {
                some: 1_517_003,
                full: 681_220
            })
        );
        let old_cpu = "some avg10=12.00 avg60=9.50 avg300=4.00 total=99\n";
        assert_eq!(totals(old_cpu), Some(Totals { some: 99, full: 0 }));
        assert_eq!(totals("nothing here"), None);
    }

    // 300 ms of the second second waited and 500 of the third, then
    // nothing: the wait runs from the first second to the third, and the
    // third quiet reading ends it.
    #[test]
    fn a_wait_runs_from_its_first_waiting_second_until_three_quiet_ones() {
        let waits = at(&[0, 0, 300_000, 800_000, 800_000, 800_000, 800_000]);
        assert!(waits[..6].iter().all(Option::is_none), "{waits:?}");
        assert_eq!(
            waits[6],
            Some(Wait {
                start: SECOND,
                end: 3 * SECOND,
                waited_us: 800_000,
                full_us: 400_000,
                worst: 0.5,
            })
        );
    }

    #[test]
    fn less_than_a_tenth_of_a_second_waited_is_no_wait() {
        let waits = at(&[0, 90_000, 180_000, 270_000, 270_000, 270_000, 270_000]);
        assert!(waits.iter().all(Option::is_none), "{waits:?}");
    }

    // A wait that doesn't end is kept each minute, so a moment read back
    // while it lasts still finds it; the next minute starts another.
    #[test]
    fn a_wait_that_lasts_is_kept_each_minute() {
        let totals: Vec<u64> = (0..=62).map(|second| second * 500_000).collect();
        let waits = at(&totals);
        let kept: Vec<&Wait> = waits.iter().flatten().collect();
        assert_eq!(kept.len(), 1, "{kept:?}");
        assert_eq!((kept[0].start, kept[0].end), (0, 60 * SECOND));
        assert_eq!(kept[0].waited_us, 30_000_000);
    }

    #[test]
    fn the_slices_are_read_below_the_accounts_manager() {
        let line = "0::/user.slice/user-1000.slice/user@1000.service/background.slice/\
                    collector.service\n";
        let found = sources(Path::new("/r"), line);
        let manager = "/r/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service";
        let files: Vec<(String, &str, String)> = found
            .iter()
            .map(|s| (s.place.clone(), s.resource, s.file.display().to_string()))
            .collect();
        assert!(files.contains(&(
            "the machine".into(),
            "memory",
            "/r/proc/pressure/memory".into()
        )));
        assert!(files.contains(&("the machine".into(), "io", "/r/proc/pressure/io".into())));
        assert!(files.contains(&(
            "app.slice".into(),
            "cpu",
            format!("{manager}/app.slice/cpu.pressure")
        )));
        assert!(files.contains(&(
            "session.slice".into(),
            "memory",
            format!("{manager}/session.slice/memory.pressure")
        )));
        assert_eq!(found.len(), 8, "{files:?}");
        // Outside an account's manager there are no slices of its own.
        assert_eq!(sources(Path::new("/r"), "0::/\n").len(), 2);
    }

    #[test]
    fn a_wait_is_kept_as_the_kernels_span_with_what_it_waited() {
        let source = Source {
            place: "app.slice".into(),
            resource: "memory",
            file: PathBuf::new(),
        };
        let wait = Wait {
            start: SECOND,
            end: 3 * SECOND,
            waited_us: 812_400,
            full_us: 401_000,
            worst: 0.504,
        };
        let kept = span(&source, &wait, ("t".into(), "s".into()));
        assert_eq!(kept.name, "memory waits in app.slice");
        assert_eq!(kept.service, "kernel");
        assert_eq!(kept.attributes["waited_ms"], 812);
        assert_eq!(kept.attributes["full_ms"], 401);
        assert_eq!(kept.attributes["worst_percent"], 50);
    }
}
