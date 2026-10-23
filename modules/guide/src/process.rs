//! The programs this process runs: finding them, and ending them when the
//! guide stops partway through a step.
//!
//! The terminal is held in raw mode, so Ctrl+C is a key rather than a
//! signal, and nothing the guide runs hears it. When the person presses
//! it while a step is at work, the guide says it's stopping, so no program
//! starts from then on, asks each one it runs to end, and ends any still
//! there after a few seconds. A step is only counted done once its proof
//! is seen, so the next run finds what was done and carries on from there.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Whether the guide is stopping.
static STOPPING: AtomicBool = AtomicBool::new(false);

/// How long a program is given to end once it's asked to.
pub const GRACE: Duration = Duration::from_secs(5);

/// Says the guide is stopping, so no program starts from now on.
pub fn stop_starting() {
    STOPPING.store(true, Ordering::SeqCst);
}

/// Waits here for good once the guide is stopping, so a step starts
/// nothing more while the guide ends around it.
pub fn hold_if_stopping() {
    while STOPPING.load(Ordering::SeqCst) {
        std::thread::park();
    }
}

/// Asks each program to end, waits up to `grace` for them to, then ends
/// any still running.
pub fn end(programs: &[u32], grace: Duration) {
    for program in programs {
        signal(*program, libc::SIGTERM);
    }
    let until = Instant::now() + grace;
    while programs.iter().any(|program| running(*program)) && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(50));
    }
    for program in programs.iter().filter(|program| running(**program)) {
        signal(*program, libc::SIGKILL);
    }
}

fn signal(program: u32, signal: libc::c_int) {
    let Ok(pid) = libc::pid_t::try_from(program) else {
        return;
    };
    // SAFETY: kill reads its two arguments and touches no memory.
    unsafe { libc::kill(pid, signal) };
}

/// The programs this process runs, with `known` among them: on Linux every
/// child it has that hasn't ended, found through /proc; elsewhere only
/// `known`.
pub fn running_now(known: Option<u32>) -> Vec<u32> {
    #[cfg(target_os = "linux")]
    let mut programs: Vec<u32> = children()
        .into_iter()
        .filter(|child| child.state != 'Z')
        .map(|child| child.pid)
        .collect();
    #[cfg(not(target_os = "linux"))]
    let mut programs: Vec<u32> = vec![];
    if let Some(known) = known.filter(|known| !programs.contains(known)) {
        programs.push(known);
    }
    programs
}

/// Whether a program is still running. On Linux one that has ended and
/// not yet been waited on has stopped running; elsewhere it counts as
/// running until it's waited on.
fn running(program: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        stat(program).is_some_and(|child| child.state != 'Z')
    }
    #[cfg(not(target_os = "linux"))]
    {
        let Ok(pid) = libc::pid_t::try_from(program) else {
            return false;
        };
        // SAFETY: kill with no signal only checks the process is there.
        unsafe { libc::kill(pid, 0) == 0 }
    }
}

/// One of this process's children, as /proc says it is.
#[cfg(target_os = "linux")]
#[derive(Debug)]
pub struct Child {
    pub pid: u32,
    /// The program's name, as the kernel keeps it.
    pub name: String,
    /// Its state: Z once it has ended and not yet been waited on.
    pub state: char,
}

/// Every child of this process, ended or not.
#[cfg(target_os = "linux")]
pub fn children() -> Vec<Child> {
    let own = std::process::id();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return vec![];
    };
    entries
        .filter_map(|entry| entry.ok()?.file_name().to_str()?.parse::<u32>().ok())
        .filter_map(|pid| stat(pid).filter(|child| child.parent == own))
        .map(|child| Child {
            pid: child.pid,
            name: child.name,
            state: child.state,
        })
        .collect()
}

#[cfg(target_os = "linux")]
struct Stat {
    pid: u32,
    name: String,
    state: char,
    parent: u32,
}

#[cfg(target_os = "linux")]
fn stat(pid: u32) -> Option<Stat> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // The name sits in parentheses and may hold any byte, so the fields
    // after it are read from the last closing one.
    let (open, close) = (stat.find('(')?, stat.rfind(')')?);
    let mut after = stat[close + 1..].split_whitespace();
    let state = after.next()?.chars().next()?;
    let parent = after.next()?.parse().ok()?;
    Some(Stat {
        pid,
        name: stat[open + 1..close].to_string(),
        state,
        parent,
    })
}

#[cfg(test)]
mod tests {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    use super::*;

    #[test]
    fn a_program_asked_to_end_ends_at_once() {
        let mut program = Command::new("sleep").arg("30").spawn().unwrap();
        let started = Instant::now();
        end(&[program.id()], GRACE);
        assert!(started.elapsed() < Duration::from_secs(2));
        assert_eq!(program.wait().unwrap().signal(), Some(libc::SIGTERM));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_program_that_wont_end_is_ended_once_its_time_is_up() {
        // An ignored signal stays ignored across exec, so sleep hears
        // nothing of SIGTERM.
        let mut program = Command::new("sh")
            .args(["-c", "trap '' TERM; exec sleep 30"])
            .spawn()
            .unwrap();
        while stat(program.id()).is_none_or(|child| child.name != "sleep") {
            std::thread::sleep(Duration::from_millis(10));
        }
        let started = Instant::now();
        end(&[program.id()], Duration::from_millis(300));
        assert!(started.elapsed() >= Duration::from_millis(300));
        assert_eq!(program.wait().unwrap().signal(), Some(libc::SIGKILL));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_programs_running_now_are_this_processs_children() {
        let mut program = Command::new("sleep").arg("30").spawn().unwrap();
        let running = running_now(None);
        program.kill().unwrap();
        program.wait().unwrap();
        assert!(running.contains(&program.id()), "{running:?}");
        assert!(!running_now(None).contains(&program.id()));
    }
}
