//! camera-guard keeps every agent from opening a camera from a quarter
//! hour before one of Joel's meetings until it ends, and leaves Joel's own
//! programs free to.
//!
//! At 09:51 on 24 Nov Joel saw his camera in use before a meeting, and it
//! was an agent's read, asked for by another agent. Agents run as Joel's
//! own account, so no permission tells them from his Meet; only where
//! systemd runs them does: his programs in app.slice, agents and their
//! work in jobs.slice and agents.slice. So this runs as root, asks the
//! google daemon for his meetings each minute, and from fifteen minutes
//! before each until it ends keeps a cgroup device program on the agents'
//! slices that refuses every video4linux device (src/bpf.rs). app.slice,
//! the system's camera relays and everything else are never touched.
//!
//! It fails closed: a calendar that can't be read leaves the meetings it
//! last said standing, kept in the state folder across restarts
//! (src/meetings.rs), so a known meeting keeps the guard up. It looks every
//! five seconds, so a slice made again since the last look is guarded
//! within that (src/guard.rs). It says in the journal when the guard goes
//! up and comes down, and each slice it couldn't guard.

mod bpf;
mod guard;
mod meetings;

use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use terminal_kit::{Exit, Failure, Help};

/// How often the calendar is asked, and how often the slices are looked at.
const ASK_EVERY: Duration = Duration::from_secs(60);
const LOOK_EVERY: Duration = Duration::from_secs(5);

/// What `camera-guard --help` says.
fn help() -> Help {
    Help::new(
        "camera-guard",
        "keeps agents off the camera near Joel's meetings",
    )
    .example(
        "camera-guard joel jobs.slice agents.slice",
        "as its unit runs it",
    )
    .argument("ACCOUNT", "the account whose meetings and agents it minds")
    .argument(
        "SLICE",
        "an agents' slice under the account's manager, one or more",
    )
    .exit(Exit::Unavailable, "the kernel refused the guard's program")
    .exit(Exit::Config, "the account or its slices can't be found")
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let (account, slices) = words(&context.args)?;
        let passwd = std::fs::read_to_string("/etc/passwd").unwrap_or_default();
        let uid = uid(&passwd, &account)
            .ok_or_else(|| Failure::new(Exit::Config, format!("there's no account {account}")))?;
        let manager = PathBuf::from(format!(
            "/sys/fs/cgroup/user.slice/user-{uid}.slice/user@{uid}.service"
        ));
        let paths: Vec<PathBuf> = slices.iter().map(|slice| manager.join(slice)).collect();
        let socket = PathBuf::from(format!("/run/user/{uid}/daemons/google.sock"));
        let state = std::env::var_os("STATE_DIRECTORY")
            .map_or_else(|| PathBuf::from("/var/lib/camera-guard"), PathBuf::from);
        let program = bpf::Guard::load().map_err(|error| {
            Failure::new(Exit::Unavailable, "the kernel refused the guard's program")
                .because(error.to_string())
        })?;
        watch(&program, &paths, &socket, &state.join("meetings.json"));
        Ok(())
    })
}

/// The account and its slices, from the words it's started with.
///
/// # Errors
///
/// Asked wrongly, 64: no account, no slice, a flag, or a slice that climbs
/// out of the account's manager.
fn words(args: &[String]) -> Result<(String, Vec<PathBuf>), Failure> {
    let wrongly = |what: String| Failure::new(Exit::Usage, what).try_this("camera-guard --help");
    if let Some(flag) = args.iter().find(|arg| arg.starts_with('-')) {
        return Err(wrongly(format!("{flag} isn't a flag it takes")));
    }
    let [account, slices @ ..] = args else {
        return Err(wrongly(
            "camera-guard takes an account and its slices".into(),
        ));
    };
    if slices.is_empty() {
        return Err(wrongly(format!(
            "camera-guard takes {account}'s slices to guard"
        )));
    }
    let slices: Vec<PathBuf> = slices.iter().map(PathBuf::from).collect();
    if let Some(out) = slices.iter().find(|slice| !within(slice)) {
        return Err(wrongly(format!(
            "{} isn't a slice under the account's manager",
            out.display()
        )));
    }
    Ok((account.clone(), slices))
}

/// Whether `slice` names a place under the manager, not outside it.
fn within(slice: &Path) -> bool {
    slice
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
}

/// The account's number, from passwd's text.
fn uid(passwd: &str, account: &str) -> Option<u32> {
    passwd.lines().find_map(|line| {
        let mut fields = line.split(':');
        (fields.next() == Some(account))
            .then(|| fields.nth(1)?.parse().ok())
            .flatten()
    })
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

/// Guards `paths` for as long as it runs.
fn watch(program: &bpf::Guard, paths: &[PathBuf], socket: &Path, kept: &Path) {
    let mut watch = guard::Watch::from(meetings::kept(kept).unwrap_or_else(|why| {
        println!(
            "<4>the meetings kept can't be read, so none stand until the calendar says: {why}"
        );
        Vec::new()
    }));
    let mut asked: Option<Instant> = None;
    loop {
        if asked.is_none_or(|at| at.elapsed() >= ASK_EVERY) {
            let answer = meetings::ask(socket).and_then(|answer| meetings::read(&answer));
            let (said, changed) = watch.heard(answer);
            for line in said {
                println!("{line}");
            }
            if changed && let Err(why) = meetings::keep(kept, &watch.meetings) {
                println!("<4>the meetings couldn't be kept for the next run: {why}");
            }
            asked = Some(Instant::now());
        }
        for line in watch.look(program, paths, now()) {
            println!("{line}");
        }
        thread::sleep(LOOK_EVERY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terminal_kit::Environment;

    /// How the kit ends camera-guard started with `words`, its body reading
    /// only its words.
    fn started(said: &[&str]) -> (Exit, String) {
        let ending = terminal_kit::run(
            &help(),
            "0.1.0",
            &Environment::of([false; 3], &[]),
            said.iter().map(|word| (*word).to_owned()).collect(),
            |context| words(&context.args).map(drop),
        );
        (ending.exit, ending.stderr)
    }

    #[test]
    fn its_unit_s_words_are_read_as_the_account_and_its_slices() {
        let unit = ["joel", "jobs.slice", "agents.slice"];
        let (account, slices) = words(&unit.map(String::from)).unwrap();
        assert_eq!(account, "joel", "the account wasn't the first word");
        assert_eq!(
            slices,
            vec![PathBuf::from("jobs.slice"), PathBuf::from("agents.slice")],
            "the slices weren't the words after it"
        );
        assert_eq!(
            started(&unit).0,
            Exit::Done,
            "the kit refused its unit's words"
        );
    }

    #[test]
    fn a_start_asked_wrongly_ends_64_naming_its_help() {
        for said in [
            &[][..],
            &["joel"],
            &["joel", "--all"],
            &["joel", "../user-0.slice"],
            &["joel", "/sys/fs/cgroup"],
        ] {
            let (exit, error) = started(said);
            assert_eq!(
                exit,
                Exit::Usage,
                "{said:?} wasn't refused as asked wrongly: {error}"
            );
            assert!(
                error.contains("camera-guard --help"),
                "{said:?}'s refusal didn't name its help: {error}"
            );
        }
    }

    #[test]
    fn the_account_s_number_is_read_from_passwd() {
        let passwd = "root:x:0:0::/root:/bin/sh\njoel:x:1000:100::/home/joel:/bin/zsh\n\
                      joelle:x:1001:100::/home/joelle:/bin/sh\n";
        assert_eq!(uid(passwd, "joel"), Some(1000), "joel's number wasn't read");
        assert_eq!(
            uid(passwd, "joelle"),
            Some(1001),
            "a longer name was confused"
        );
        assert_eq!(
            uid(passwd, "ada"),
            None,
            "an account not there was given a number"
        );
    }
}
