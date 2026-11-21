//! last-seconds-keep: writes what the machine does each second to this
//! boot's ring, from boot to shutdown, as its unit runs it (nixos.nix).
//!
//! Its ring is the boot's own folder's, named for the kernel's boot id as
//! journalctl names boots; a run after a restart goes on after the ring's
//! newest record. When it's stopped, as at a shutdown, it writes one more
//! record saying so, so a ring whose newest record doesn't say it ended
//! with a cut or a reset. When the machine goes to sleep, SIGUSR1 from the
//! sleep's hook has it write a record saying so and write nothing more
//! until SIGUSR2 says it woke, so a boot that never woke says it ended
//! asleep, not at the second the lid closed. Each of those records asks
//! nothing of the firmware, so neither the stop nor the sleep waits on it.
//!
//! The signals are held back while it reads and writes and let in only as
//! it rests, so one that comes mid-second is heard as soon as the second's
//! record is written, never slept through.

use last_seconds::record::Record;
use last_seconds::ring::{self, Header, Ring};
use last_seconds::sample::{self, Machine, Sampler};
use last_seconds::{RING, STATE, STATE_VARIABLE};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};
use terminal_kit::{Exit, Failure, Help, flags};

/// How often it reads the machine, unless asked otherwise.
const EVERY: Duration = Duration::from_secs(1);

/// The longest it holds off writing after the machine said it was going
/// to sleep, awake: a sleep that failed and never said it woke can't leave
/// the ring unwritten for longer. A sleep's own time doesn't count, since
/// the clock it's held to stops with the machine.
const ASLEEP_AT_MOST: Duration = Duration::from_secs(60);

/// Whether it has been asked to stop.
static STOP: AtomicBool = AtomicBool::new(false);
/// How many times the machine said it's going to sleep, and said it woke.
/// They're counted rather than kept as one flag, since a sleep and a wake let
/// in together have their handlers run the lowest-numbered last, which would
/// leave the wake read as a sleep.
static SLEPT: AtomicU32 = AtomicU32::new(0);
static WOKE: AtomicU32 = AtomicU32::new(0);

extern "C" fn stop(_: libc::c_int) {
    STOP.store(true, Ordering::SeqCst);
}

extern "C" fn sleeps(_: libc::c_int) {
    SLEPT.fetch_add(1, Ordering::SeqCst);
}

extern "C" fn wakes(_: libc::c_int) {
    WOKE.fetch_add(1, Ordering::SeqCst);
}

/// Whether the machine said it's going to sleep more often than it said it
/// woke.
fn asleep() -> bool {
    SLEPT.load(Ordering::SeqCst) > WOKE.load(Ordering::SeqCst)
}

/// Takes the machine as awake, with the signals held: a sleep held past
/// `ASLEEP_AT_MOST` is let go, and so is a wake said with no sleep before
/// it, as to a recorder started mid-sleep, so the next sleep still counts.
fn awake() {
    WOKE.store(SLEPT.load(Ordering::SeqCst), Ordering::SeqCst);
}

/// The signals it answers.
const HEARD: [libc::c_int; 4] = [libc::SIGTERM, libc::SIGINT, libc::SIGUSR1, libc::SIGUSR2];

/// A set of signals: those it answers, or none.
fn signals(heard: bool) -> libc::sigset_t {
    // SAFETY: sigemptyset makes the zeroed set a valid one before
    // sigaddset adds to it.
    unsafe {
        let mut set: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&raw mut set);
        if heard {
            for signal in HEARD {
                libc::sigaddset(&raw mut set, signal);
            }
        }
        set
    }
}

/// Rests for `duration`, or until `done` holds, which it asks first. The
/// signals held back come in only inside ppoll, which lets them in and
/// waits in one step, so one sent at any moment ends the rest at once
/// rather than landing just before it and being slept through. A rest with
/// nothing left, after a second that overran, still goes into ppoll for no
/// time, so what was held back through that second is let in.
fn rest(duration: Duration, done: impl Fn() -> bool) {
    let until = Instant::now() + duration;
    let open = signals(false);
    loop {
        if done() {
            return;
        }
        let left = until.saturating_duration_since(Instant::now());
        let timeout = libc::timespec {
            tv_sec: libc::time_t::try_from(left.as_secs()).unwrap_or(libc::time_t::MAX),
            tv_nsec: libc::c_long::from(left.subsec_nanos()),
        };
        // SAFETY: no descriptors, and both pointers are to values this owns.
        let woke =
            unsafe { libc::ppoll(std::ptr::null_mut(), 0, &raw const timeout, &raw const open) };
        if left.is_zero()
            || (woke < 0 && io::Error::last_os_error().raw_os_error() != Some(libc::EINTR))
        {
            return;
        }
    }
}

fn help() -> Help {
    Help::new(
        "last-seconds-keep",
        "writes what the machine does each second to this boot's ring",
    )
    .example(
        "last-seconds-keep",
        "as its unit runs it, until it's stopped",
    )
    .example(
        "LAST_SECONDS_STATE=/tmp/rings last-seconds-keep --root /tmp/machine --seconds 5",
        "five seconds of a stand-in machine, ending as a cut would, kept apart",
    )
    .option(
        "--root FOLDER",
        "where the machine's /sys and /proc are; / by default, and any other \
         only with LAST_SECONDS_STATE naming where its rings go",
    )
    .option(
        "--every MS",
        "how often to read it, in milliseconds; 1000 by default",
    )
    .option(
        "--seconds N",
        "end after N records, without saying it was stopped",
    )
    .exit(Exit::Failed, "the boot's ring couldn't be made or written")
}

/// What it was asked.
struct Asked {
    root: PathBuf,
    every: Duration,
    seconds: Option<u64>,
}

fn usage(what: impl Into<String>) -> Failure {
    Failure::new(Exit::Usage, what).try_this("last-seconds-keep --help")
}

fn asked(args: &[String]) -> Result<Asked, Failure> {
    let mut asked = Asked {
        root: PathBuf::from("/"),
        every: EVERY,
        seconds: None,
    };
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = || {
            args.next()
                .cloned()
                .ok_or_else(|| usage(format!("{arg} needs a value")))
        };
        let count = |value: String| {
            value
                .parse::<u64>()
                .map_err(|_| usage(format!("{arg} takes a whole number, not {value:?}")))
        };
        match arg.as_str() {
            "--root" => asked.root = PathBuf::from(value()?),
            "--every" => asked.every = Duration::from_millis(count(value()?)?.max(1)),
            "--seconds" => match count(value()?)? {
                0 => return Err(usage("--seconds takes 1 or more")),
                seconds => asked.seconds = Some(seconds),
            },
            flag if flag.starts_with("--") => {
                return Err(flags::unknown(
                    "option",
                    flag,
                    &["--root", "--every", "--seconds"],
                ));
            }
            word => return Err(usage(format!("it takes no {word:?}"))),
        }
    }
    Ok(asked)
}

/// Where the rings go: a stand-in's never among the machine's own, where
/// they'd read as boots it had.
/// `named` is what `LAST_SECONDS_STATE` says, if anything.
fn state(asked: &Asked, named: Option<OsString>) -> Result<PathBuf, Failure> {
    match named.filter(|state| !state.is_empty()) {
        Some(state) => Ok(PathBuf::from(state)),
        None if asked.root == Path::new("/") => Ok(PathBuf::from(STATE)),
        None => Err(usage("a stand-in's rings would go among the machine's own")
            .because(format!("--root is {}", asked.root.display()))
            .try_this(format!(
                "{STATE_VARIABLE}=/tmp/rings last-seconds-keep --root {}",
                terminal_kit::failure::shell_word(&asked.root.to_string_lossy())
            ))),
    }
}

/// The boot running now, as journalctl names it.
fn boot(root: &Path) -> Result<String, Failure> {
    let path = root.join("proc/sys/kernel/random/boot_id");
    let id = fs::read_to_string(&path).map_err(|error| {
        Failure::new(Exit::Failed, "it couldn't tell which boot this is")
            .because(format!("{}: {error}", path.display()))
    })?;
    let id = id.trim().replace('-', "");
    // The folder is named for it, so it's held to what the reader takes.
    if id.len() == 32 && id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(id)
    } else {
        Err(
            Failure::new(Exit::Failed, "it couldn't tell which boot this is")
                .because(format!("{} doesn't hold a boot id", path.display())),
        )
    }
}

/// When the boot began, in seconds since 1970, as the kernel says.
fn began(root: &Path) -> i64 {
    fs::read_to_string(root.join("proc/stat"))
        .ok()
        .and_then(|stat| {
            stat.lines()
                .find_map(|line| line.strip_prefix("btime "))?
                .trim()
                .parse()
                .ok()
        })
        .unwrap_or(0)
}

fn unwritten(path: &Path, error: &std::io::Error) -> Failure {
    Failure::new(Exit::Failed, "it couldn't write the boot's ring")
        .because(format!("{}: {error}", path.display()))
}

fn keep(asked: &Asked) -> Result<(), Failure> {
    // Set first, so a stop while the ring is being made still ends in a
    // record saying so.
    let handlers: [extern "C" fn(libc::c_int); 4] = [stop, stop, sleeps, wakes];
    let held = signals(true);
    // SAFETY: each handler only stores to an atomic, which a signal may,
    // and the signals are held back from here but for each rest.
    unsafe {
        for (signal, handler) in HEARD.into_iter().zip(handlers) {
            libc::signal(signal, handler as libc::sighandler_t);
        }
        libc::pthread_sigmask(libc::SIG_BLOCK, &raw const held, std::ptr::null_mut());
    }
    let folder = state(asked, env::var_os(STATE_VARIABLE))?.join(boot(&asked.root)?);
    fs::create_dir_all(&folder).map_err(|error| unwritten(&folder, &error))?;
    let machine = Machine::under(&asked.root);
    let header = Header {
        slots: ring::SLOTS,
        began: began(&asked.root),
        processors: machine.processors,
        rests: machine.rest_names.clone(),
    };
    let path = folder.join(RING);
    let mut ring = Ring::open(&path, &header).map_err(|error| unwritten(&path, &error))?;
    let mut sampler = Sampler::of(machine);
    let mut written = 0;
    loop {
        let started = Instant::now();
        let stopping = STOP.load(Ordering::SeqCst);
        let sleeping = asleep();
        if !sleeping {
            awake();
        }
        let now = sample::since_boot_ms();
        let mut record: Record = if stopping || sleeping {
            sampler.read_quickly(now)
        } else {
            sampler.read(now)
        };
        record.stopped = stopping;
        record.asleep = sleeping && !stopping;
        ring.put(&mut record)
            .map_err(|error| unwritten(&path, &error))?;
        written += 1;
        if record.stopped || asked.seconds == Some(written) {
            return Ok(());
        }
        if record.asleep {
            let said = Instant::now();
            while asleep() && !STOP.load(Ordering::SeqCst) && said.elapsed() < ASLEEP_AT_MOST {
                rest(asked.every, || !asleep() || STOP.load(Ordering::SeqCst));
            }
            awake();
            sampler.woke();
            continue;
        }
        rest(asked.every.saturating_sub(started.elapsed()), || {
            STOP.load(Ordering::SeqCst) || asleep()
        });
    }
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        keep(&asked(&context.args)?)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(args: &[&str]) -> Vec<String> {
        args.iter().map(|&arg| arg.to_owned()).collect()
    }

    #[test]
    fn no_seconds_at_all_is_refused() {
        let refused = asked(&words(&["--seconds", "0"]))
            .err()
            .map(|failure| failure.exit);
        assert_eq!(refused, Some(Exit::Usage));
    }

    #[test]
    fn a_boot_id_that_couldnt_name_a_folder_the_reader_lists_is_refused() {
        let root = env::temp_dir().join(format!("last-seconds-keep-id-{}", std::process::id()));
        let file = root.join("proc/sys/kernel/random/boot_id");
        fs::create_dir_all(file.parent().expect("its folder")).expect("the folders");
        let mut read = Vec::new();
        for id in [
            "6bccd85d-8071-4583-afd8-05c687f0cdb6\n",
            "6bccd85d-8071-4583-afd8-05c687f0cdb6x\n",
            "../../etc\n",
        ] {
            fs::write(&file, id).expect("a stand-in id");
            read.push(boot(&root).ok());
        }
        let _ = fs::remove_dir_all(&root);
        assert_eq!(
            read,
            [
                Some("6bccd85d80714583afd805c687f0cdb6".to_owned()),
                None,
                None
            ]
        );
    }

    #[test]
    fn a_stand_ins_rings_go_only_where_theyre_named_to() {
        let stand_in = asked(&words(&["--root", "/tmp/machine"])).expect("asked");
        let machine = asked(&[]).expect("asked");
        let named = || Some(OsString::from("/tmp/rings"));
        assert_eq!(
            state(&stand_in, None).err().map(|failure| failure.exit),
            Some(Exit::Usage)
        );
        assert_eq!(
            state(&stand_in, Some(OsString::new()))
                .err()
                .map(|failure| failure.exit),
            Some(Exit::Usage)
        );
        assert_eq!(
            state(&stand_in, named()).ok(),
            Some(PathBuf::from("/tmp/rings"))
        );
        assert_eq!(state(&machine, None).ok(), Some(PathBuf::from(STATE)));
        assert_eq!(
            state(&machine, named()).ok(),
            Some(PathBuf::from("/tmp/rings"))
        );
    }

    #[test]
    // One test for both cases, since they share the counts.
    fn signals_held_back_through_a_second_that_overran_are_let_in_in_order() {
        let held = signals(true);
        let slept: extern "C" fn(libc::c_int) = sleeps;
        let woke: extern "C" fn(libc::c_int) = wakes;
        // SAFETY: the handlers only add to atomics; the signals are held
        // back on this test's thread alone, and raise sends them there.
        unsafe {
            libc::signal(libc::SIGUSR1, slept as libc::sighandler_t);
            libc::signal(libc::SIGUSR2, woke as libc::sighandler_t);
            libc::pthread_sigmask(libc::SIG_BLOCK, &raw const held, std::ptr::null_mut());
            libc::raise(libc::SIGUSR1);
        }
        let before = asleep();
        rest(Duration::ZERO, asleep);
        let heard = asleep();
        awake();
        // SAFETY: as above; a sleep and its wake both held through a second.
        unsafe {
            libc::raise(libc::SIGUSR1);
            libc::raise(libc::SIGUSR2);
        }
        rest(Duration::ZERO, || false);
        let woken = !asleep();
        awake();
        // SAFETY: as above, letting this thread's signals in again.
        unsafe {
            libc::pthread_sigmask(libc::SIG_UNBLOCK, &raw const held, std::ptr::null_mut());
        }
        assert!(!before, "a signal held back was heard before the rest");
        assert!(
            heard,
            "a signal held back through a second that overran was never let in"
        );
        assert!(woken, "a sleep and its wake let in together read as asleep");
    }

    #[test]
    fn a_rest_asked_to_end_ends_before_it_waits() {
        let started = Instant::now();
        rest(Duration::from_secs(5), || true);
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "a rest whose end had already come waited"
        );
        let started = Instant::now();
        rest(Duration::from_millis(50), || false);
        assert!(
            started.elapsed() >= Duration::from_millis(50),
            "a rest ended early with nothing to end it"
        );
    }
}
