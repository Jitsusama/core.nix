//! The load proof's run: an idle reading, then the machine loaded in
//! steps as Joel's work loads it, a reading in each, each judged against
//! idle.
//!
//! The load is its own, so a run needs nothing but this program: each
//! build is a release build of load-proof's own source, shipped beside it
//! with its crates, rebuilt from clean until its step ends; and the line
//! is filled both ways, a large download and an upload to a public iperf3
//! server, and each step says what the line carried, so one whose upload
//! found every server busy says so. All of it runs in jobs.slice, where
//! Joel's work runs, and every reading in app.slice, where his programs
//! do. A run refuses to start with too little memory free and stops the
//! load if free memory falls too far, so it can't push Joel's own
//! programs out.

use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Where the build's source and crates are, and the tools it builds with:
/// the package names them as it builds.
const SOURCE: Option<&str> = option_env!("LOAD_PROOF_SOURCE");
const VENDOR: Option<&str> = option_env!("LOAD_PROOF_VENDOR");
const TOOLS: Option<&str> = option_env!("LOAD_PROOF_TOOLS");

/// What a download fetches: a large file a public server keeps for
/// testing a line.
const DOWNLOAD: &str = "http://ipv4.download.thinkbroadband.com/1GB.zip";

/// Where an upload goes: public iperf3 servers, each port taking one test
/// at a time and most often busy, so an upload moves on to the next until
/// one takes it.
const UPLOADS: [(&str, u16); 11] = [
    ("iperf.he.net", 5201),
    ("ping.online.net", 5200),
    ("ping.online.net", 5201),
    ("ping.online.net", 5202),
    ("ping.online.net", 5203),
    ("ping.online.net", 5204),
    ("ping.online.net", 5205),
    ("ping.online.net", 5206),
    ("ping.online.net", 5207),
    ("ping.online.net", 5208),
    ("ping.online.net", 5209),
];

/// How many streams an upload sends at once: one alone doesn't fill a line
/// a hundred milliseconds from its server.
const STREAMS: &str = "4";

const GIB: u64 = 1 << 30;

/// The subcommands a run starts its load with, this program run again in
/// jobs.slice: one build, and the line filled both ways.
pub const BUILD: &str = "load-build";
pub const LINE: &str = "load-line";

/// How a run goes.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    /// How many builds run in each step, in order.
    pub steps: Vec<usize>,
    /// How long each reading lasts.
    pub reading: Duration,
    /// How long a step's builds run before its reading starts.
    pub settle: Duration,
    /// Free memory a run needs to start.
    pub start_free: u64,
    /// Free memory under which the load is stopped.
    pub stop_free: u64,
}

impl Default for Plan {
    /// Joel's work at its heaviest is twenty builds at once, so the steps
    /// climb to it; each gives its builds long enough to be compiling.
    fn default() -> Plan {
        Plan {
            steps: vec![4, 8, 12, 20],
            reading: Duration::from_secs(30),
            settle: Duration::from_secs(15),
            start_free: 16 * GIB,
            stop_free: 8 * GIB,
        }
    }
}

impl Plan {
    /// How long a run takes: the idle reading, then each step's settling
    /// and reading.
    pub fn length(&self) -> Duration {
        let steps = u32::try_from(self.steps.len()).unwrap_or(u32::MAX);
        self.reading + (self.settle + self.reading) * steps
    }
}

/// What one step found: each reading is said as it's taken, so only its
/// failures are kept.
#[derive(Debug)]
pub struct Step {
    pub builds: usize,
    pub failed: Vec<String>,
}

/// What a run found.
#[derive(Debug)]
pub struct Run {
    pub steps: Vec<Step>,
    /// Why the load was stopped early, if it was.
    pub stopped: Option<String>,
}

impl Run {
    /// Whether every step felt as idle did, and the load ran to the end.
    pub fn passed(&self) -> bool {
        self.stopped.is_none() && self.first_failed().is_none()
    }

    /// How many builds the first step that failed ran.
    pub fn first_failed(&self) -> Option<usize> {
        self.steps
            .iter()
            .find(|step| !step.failed.is_empty())
            .map(|step| step.builds)
    }
}

/// Free memory, in bytes, from what `/proc/meminfo` says is available.
pub fn available(meminfo: &str) -> Option<u64> {
    let line = meminfo
        .lines()
        .find(|line| line.starts_with("MemAvailable:"))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kib * 1024)
}

/// Cargo's settings for a build that finds its crates in `vendor` alone,
/// so it never reaches the network.
pub fn offline(vendor: &str) -> String {
    format!(
        "[source.crates-io]\nreplace-with = \"vendored\"\n\n\
         [source.vendored]\ndirectory = \"{vendor}\"\n"
    )
}

/// Copies the folder `from` into `to`, as plain files Cargo may write
/// beside: what's copied from the store is read-only.
pub fn copy(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let there = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy(&entry.path(), &there)?;
        } else {
            fs::write(&there, fs::read(entry.path())?)?;
        }
    }
    Ok(())
}

/// Runs `plan`, reading every signal with `program`, and says each step's
/// lines as they come on `out`.
pub fn run(plan: &Plan, program: &Path, folder: &Path, out: &mut dyn Write) -> Result<Run, String> {
    let now = free()?;
    if now < plan.start_free {
        return Err(format!(
            "only {} GiB free, and a run needs {}",
            now / GIB,
            plan.start_free / GIB
        ));
    }
    let idle = reading(program, plan.reading)?;
    say(out, "idle", &idle);
    let load = Load::new(program, folder);
    let stopped = Arc::new(Mutex::new(None));
    let done = Arc::new(AtomicBool::new(false));
    let guard = {
        let (units, stopped, done) = (load.units.clone(), stopped.clone(), done.clone());
        let stop_free = plan.stop_free;
        thread::spawn(move || {
            while !done.load(Ordering::Relaxed) {
                if let Ok(free) = free()
                    && free < stop_free
                {
                    stop(
                        &units
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner),
                    );
                    *stopped
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) =
                        Some(format!("free memory fell under {} GiB", stop_free / GIB));
                    return;
                }
                thread::sleep(Duration::from_secs(1));
            }
        })
    };
    let result = steps(plan, program, &load, &idle, &stopped, out);
    done.store(true, Ordering::Relaxed);
    let _ = guard.join();
    drop(load);
    let stopped = stopped
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take();
    Ok(Run {
        steps: result?,
        stopped,
    })
}

fn steps(
    plan: &Plan,
    program: &Path,
    load: &Load,
    idle: &str,
    stopped: &Mutex<Option<String>>,
    out: &mut dyn Write,
) -> Result<Vec<Step>, String> {
    let left = plan.length().saturating_sub(plan.reading);
    load.line(left)?;
    let mut steps = Vec::new();
    let mut running = 0;
    let started = Instant::now();
    for &builds in &plan.steps {
        let ends = left.saturating_sub(started.elapsed());
        while running < builds {
            running += 1;
            load.build(running, ends)?;
        }
        thread::sleep(plan.settle);
        if stopped.lock().is_ok_and(|stopped| stopped.is_some()) {
            break;
        }
        let link = fs::read_to_string("/proc/net/route")
            .ok()
            .and_then(|route| default_link(&route));
        let before = link.as_deref().and_then(carried);
        let started_reading = Instant::now();
        let mut reading = reading(program, plan.reading)?;
        if stopped.lock().is_ok_and(|stopped| stopped.is_some()) {
            break;
        }
        if let (Some(link), Some(before), Some(after)) =
            (&link, before, link.as_deref().and_then(carried))
        {
            let (up, down) = rates(before, after, started_reading.elapsed());
            let _ = writeln!(
                reading,
                "line {link} up {up:.1} Mbit/s down {down:.1} Mbit/s"
            );
        }
        let failed = crate::judge::judge(&crate::judge::read(idle), &crate::judge::read(&reading));
        say(out, &format!("{builds} builds"), &reading);
        for failure in &failed {
            let _ = writeln!(out, "  fails: {failure}");
        }
        steps.push(Step { builds, failed });
    }
    Ok(steps)
}

fn say(out: &mut dyn Write, what: &str, reading: &str) {
    let _ = writeln!(out, "{what}:");
    for line in reading.lines() {
        let _ = writeln!(out, "  {line}");
    }
}

/// The link the default route leaves by, from `/proc/net/route`: the one
/// with the lowest metric, as the kernel would choose.
pub fn default_link(route: &str) -> Option<String> {
    route
        .lines()
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let (link, destination, metric) = (fields.first()?, fields.get(1)?, fields.get(6)?);
            (*destination == "00000000").then_some((metric.parse::<u64>().ok()?, *link))
        })
        .min()
        .map(|(_, link)| link.to_owned())
}

/// The bytes `link` has sent and received, as the kernel counts them.
fn carried(link: &str) -> Option<(u64, u64)> {
    let count = |which| {
        fs::read_to_string(format!("/sys/class/net/{link}/statistics/{which}_bytes"))
            .ok()?
            .trim()
            .parse()
            .ok()
    };
    Some((count("tx")?, count("rx")?))
}

/// Megabits a second up and down between two counts `over` apart.
pub fn rates(before: (u64, u64), after: (u64, u64), over: Duration) -> (f64, f64) {
    let seconds = over.as_secs_f64().max(f64::EPSILON);
    let rate = |from: u64, to: u64| {
        f64::from(u32::try_from(to.saturating_sub(from) / 1000).unwrap_or(u32::MAX)) * 8.0
            / 1000.0
            / seconds
    };
    (rate(before.0, after.0), rate(before.1, after.1))
}

fn free() -> Result<u64, String> {
    let meminfo =
        fs::read_to_string("/proc/meminfo").map_err(|e| format!("reading /proc/meminfo: {e}"))?;
    available(&meminfo).ok_or_else(|| "no MemAvailable in /proc/meminfo".to_owned())
}

/// Every signal read at once for `length`, from app.slice, as the lines
/// `load-proof all` prints.
fn reading(program: &Path, length: Duration) -> Result<String, String> {
    let out = Command::new("systemd-run")
        .args(["--user", "--slice=app.slice", "--wait", "--pipe", "--quiet"])
        .args(["-p", "Type=exec"])
        .arg(program)
        .arg("all")
        .arg(length.as_secs().to_string())
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("starting a reading: {e}"))?;
    String::from_utf8(out.stdout).map_err(|e| format!("reading a reading: {e}"))
}

/// The load a run started: its units, and the folder its builds write in,
/// all stopped and removed once it's dropped.
struct Load {
    program: PathBuf,
    folder: PathBuf,
    units: Arc<Mutex<Vec<String>>>,
}

impl Load {
    fn new(program: &Path, folder: &Path) -> Load {
        Load {
            program: program.to_owned(),
            folder: folder.to_owned(),
            units: Arc::default(),
        }
    }

    /// Starts build `n`, until `length` from now.
    fn build(&self, n: usize, length: Duration) -> Result<(), String> {
        let folder = self.folder.join(format!("build-{n}"));
        self.start(
            &format!("build-{n}"),
            &[
                BUILD.as_ref(),
                folder.as_os_str(),
                length.as_secs().to_string().as_ref(),
            ],
        )
    }

    /// Fills the line both ways, until `length` from now.
    fn line(&self, length: Duration) -> Result<(), String> {
        self.start(
            "line",
            &[LINE.as_ref(), length.as_secs().to_string().as_ref()],
        )
    }

    fn start(&self, name: &str, args: &[&std::ffi::OsStr]) -> Result<(), String> {
        let unit = format!("load-proof-{name}-{}", std::process::id());
        let status = Command::new("systemd-run")
            .args(["--user", "--slice=jobs.slice", "--quiet", "--collect"])
            .arg(format!("--unit={unit}"))
            .arg(&self.program)
            .args(args)
            .stdin(Stdio::null())
            .status()
            .map_err(|e| format!("starting {unit}: {e}"))?;
        if !status.success() {
            return Err(format!("starting {unit}: {status}"));
        }
        self.units
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(format!("{unit}.service"));
        Ok(())
    }
}

impl Drop for Load {
    fn drop(&mut self) {
        stop(
            &self
                .units
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        );
        let _ = fs::remove_dir_all(&self.folder);
    }
}

fn stop(units: &[String]) {
    if units.is_empty() {
        return;
    }
    let _ = Command::new("systemctl")
        .args(["--user", "stop"])
        .args(units)
        .stdin(Stdio::null())
        .status();
}

/// One build of the load: load-proof's own source copied into `folder`
/// and built for release from clean, again and again until `length` has
/// passed.
pub fn build(folder: &Path, length: Duration) -> Result<(), String> {
    let (Some(source), Some(vendor), Some(tools)) = (SOURCE, VENDOR, TOOLS) else {
        return Err("this load-proof wasn't built knowing its source".to_owned());
    };
    let crate_folder = folder.join("src");
    copy(Path::new(source), &crate_folder).map_err(|e| format!("copying the source: {e}"))?;
    fs::create_dir_all(crate_folder.join(".cargo"))
        .and_then(|()| fs::write(crate_folder.join(".cargo/config.toml"), offline(vendor)))
        .map_err(|e| format!("writing Cargo's settings: {e}"))?;
    let target = folder.join("target");
    let end = Instant::now() + length;
    while Instant::now() < end {
        let status = Command::new(format!("{tools}/cargo"))
            .args(["build", "--release", "--offline", "--quiet"])
            .current_dir(&crate_folder)
            .env("PATH", tools)
            .env("CARGO_TARGET_DIR", &target)
            .stdin(Stdio::null())
            .status()
            .map_err(|e| format!("running cargo: {e}"))?;
        if !status.success() {
            return Err(format!("cargo failed: {status}"));
        }
        let _ = fs::remove_dir_all(&target);
    }
    Ok(())
}

/// Fills the line both ways until `length` has passed.
pub fn line(length: Duration) -> Result<(), String> {
    let Some(tools) = TOOLS else {
        return Err("this load-proof wasn't built knowing curl and iperf3".to_owned());
    };
    let end = Instant::now() + length;
    let down = thread::spawn(move || {
        again_until(end, || {
            Command::new(format!("{tools}/curl"))
                .args(["-4", "-s", "-o", "/dev/null", "--max-time"])
                .arg(left(end))
                .arg(DOWNLOAD)
                .stdin(Stdio::null())
                .status()
                .map(|_| ())
        });
    });
    let up = thread::spawn(move || {
        let mut servers = UPLOADS.iter().cycle();
        again_until(end, || {
            let Some((host, port)) = servers.next() else {
                return Ok(());
            };
            let status = Command::new(format!("{tools}/iperf3"))
                .args([
                    "-4",
                    "-c",
                    host,
                    "-p",
                    &port.to_string(),
                    "-P",
                    STREAMS,
                    "-t",
                ])
                .arg(left(end))
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()?;
            if status.success() {
                Ok(())
            } else {
                Err(io::Error::other(format!("{host}:{port} is busy")))
            }
        });
    });
    let _ = down.join();
    let _ = up.join();
    Ok(())
}

fn left(end: Instant) -> String {
    end.saturating_duration_since(Instant::now())
        .as_secs()
        .max(1)
        .to_string()
}

/// Runs `once` until `end`, again whenever it finishes early, with a
/// second's pause after a failure so a server that refuses isn't
/// hammered: the uploads' next server is tried after it.
fn again_until(end: Instant, mut once: impl FnMut() -> io::Result<()>) {
    while Instant::now() < end {
        if once().is_err() {
            thread::sleep(Duration::from_secs(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_fits_in_joels_five_minutes() {
        assert!(
            Plan::default().length() <= Duration::from_mins(5),
            "{:?}",
            Plan::default().length()
        );
    }

    #[test]
    fn the_steps_climb_to_twenty_builds() {
        let steps = Plan::default().steps;
        assert!(steps.windows(2).all(|pair| pair[0] < pair[1]), "{steps:?}");
        assert_eq!(steps.last(), Some(&20));
    }

    #[test]
    fn free_memory_is_what_the_kernel_says_is_available() {
        let meminfo = "MemTotal:       65333888 kB\nMemFree:         1310720 kB\n\
                       MemAvailable:   13107200 kB\n";
        assert_eq!(available(meminfo), Some(13_107_200 * 1024));
        assert_eq!(available("MemTotal: 1 kB\n"), None);
    }

    #[test]
    fn a_build_finds_its_crates_only_in_the_vendored_folder() {
        let settings = offline("/nix/store/abc-vendor");
        assert!(
            settings.contains("replace-with = \"vendored\""),
            "{settings}"
        );
        assert!(
            settings.contains("directory = \"/nix/store/abc-vendor\""),
            "{settings}"
        );
    }

    #[test]
    fn the_source_is_copied_whole_and_writable() {
        let scratch = std::env::temp_dir().join(format!("load-proof-copy-{}", std::process::id()));
        let (from, to) = (scratch.join("from"), scratch.join("to"));
        fs::create_dir_all(from.join("src")).expect("a source");
        fs::write(from.join("Cargo.toml"), "[package]\n").expect("a manifest");
        fs::write(from.join("src/main.rs"), "fn main() {}\n").expect("a program");
        let mut read_only = fs::metadata(from.join("src/main.rs"))
            .expect("its mode")
            .permissions();
        read_only.set_readonly(true);
        fs::set_permissions(from.join("src/main.rs"), read_only).expect("read-only");
        copy(&from, &to).expect("copying");
        assert_eq!(
            fs::read_to_string(to.join("src/main.rs")).expect("the copy"),
            "fn main() {}\n"
        );
        assert!(
            !fs::metadata(to.join("src/main.rs"))
                .expect("its mode")
                .permissions()
                .readonly()
        );
        let _ = fs::remove_dir_all(&scratch);
    }

    #[test]
    fn the_line_is_the_default_routes_with_the_lowest_metric() {
        let route = "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\n\
                     wlp0s20f3\t00000000\t0104A8C0\t0003\t0\t0\t600\t00000000\n\
                     wlp0s20f3\t0004A8C0\t00000000\t0001\t0\t0\t600\t00FCFFFF\n\
                     enp0s1\t00000000\t0101A8C0\t0003\t0\t0\t100\t00000000\n";
        assert_eq!(default_link(route).as_deref(), Some("enp0s1"));
        assert_eq!(default_link("Iface\tDestination\n"), None);
    }

    #[test]
    fn what_the_line_carried_is_read_in_megabits_a_second() {
        let (up, down) = rates((0, 0), (1_250_000, 12_500_000), Duration::from_secs(10));
        assert!((up - 1.0).abs() < 1e-9, "{up}");
        assert!((down - 10.0).abs() < 1e-9, "{down}");
    }

    #[test]
    fn a_run_passes_only_when_every_step_did_and_nothing_stopped_it() {
        let step = |builds, failed: &[&str]| Step {
            builds,
            failed: failed.iter().map(|&f| f.to_owned()).collect(),
        };
        let run = |steps, stopped: Option<&str>| Run {
            steps,
            stopped: stopped.map(str::to_owned),
        };
        assert!(run(vec![step(4, &[]), step(8, &[])], None).passed());
        let failed = run(vec![step(4, &[]), step(8, &["save: p99"])], None);
        assert!(!failed.passed());
        assert_eq!(failed.first_failed(), Some(8));
        assert!(!run(vec![step(4, &[])], Some("free memory fell")).passed());
    }
}
