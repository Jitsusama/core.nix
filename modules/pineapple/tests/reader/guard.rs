//! The guard every reader case holds its reader by: a case that fails
//! part way leaves no reader still running.

use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use super::{PATIENCE, home, reader, stand_in};

#[test]
fn a_case_failing_part_way_leaves_no_reader_running() {
    let (origin, asked) = stand_in();
    let home = home();
    let running = reader(&home, &origin);
    let pid = running.id();
    // Asked once, it has started rather than died on the way up.
    let began = Instant::now();
    while asked.lock().expect("what was asked").is_empty() {
        assert!(began.elapsed() < PATIENCE, "the reader never asked Slack");
        thread::sleep(Duration::from_millis(50));
    }
    // Its state follows the parenthesised name; Z is one gone but unreaped.
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
    let state = stat.rsplit_once(')').map(|(_, rest)| rest.trim_start());
    assert!(
        state.is_some_and(|state| !state.starts_with('Z')),
        "the reader runs before the case fails, not {stat:?}"
    );
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let _held = running;
        panic!("this case fails on purpose, to see its reader stopped");
    }));
    assert!(failed.is_err(), "the case failed");
    assert!(
        !Path::new(&format!("/proc/{pid}")).exists(),
        "the failed case's reader still runs"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}
