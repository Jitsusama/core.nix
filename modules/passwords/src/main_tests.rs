//! The daemon's path through the terminal kit, whose stdout is the shell's
//! pipe: the kit writes no byte of its own there, whether the daemon ends
//! well, fails or panics, each said on stderr alone; how a copier's
//! refusal is said from what it said as it ended; and that a copy nothing
//! can wait to clear is ended at once and refused, never left held.

use std::cell::Cell;

use super::*;

/// What the kit says once the daemon's path ends as this stand-in does.
fn ended(daemon: impl FnOnce(&str, Option<&str>) -> Result<(), Failure>) -> terminal_kit::Ending {
    let args = ["daemon", "managers.json", "words.json"]
        .map(str::to_owned)
        .to_vec();
    terminal_kit::run(
        &cli::help(),
        "0",
        &terminal_kit::Environment::of([false; 3], &[]),
        args,
        |context| command_line(context, daemon),
    )
}

#[test]
fn the_daemons_path_leaves_the_shells_pipe_to_the_daemon() {
    let well = ended(|managers, words| {
        assert_eq!(
            (managers, words),
            ("managers.json", Some("words.json")),
            "the daemon wasn't handed what was typed"
        );
        Ok(())
    });
    assert_eq!(well.exit, Exit::Done, "{}", well.stderr);
    assert_eq!(
        well.stdout, "",
        "the kit wrote to the pipe of a daemon ending well"
    );
    assert_eq!(
        well.stderr, "",
        "the kit said something of a daemon ending well"
    );

    let failed = ended(|_, _| {
        Err(Failure::new(
            Exit::Config,
            "couldn't read the password managers",
        ))
    });
    assert_eq!(failed.exit, Exit::Config, "{}", failed.stderr);
    assert_eq!(
        failed.stdout, "",
        "the kit wrote a failure to the shell's pipe"
    );
    assert!(
        failed
            .stderr
            .contains("couldn't read the password managers"),
        "the failure wasn't said on stderr: {}",
        failed.stderr
    );

    let panicked = ended(|_, _| panic!("a manager's thread was gone"));
    assert_eq!(panicked.exit, Exit::Bug, "{}", panicked.stderr);
    assert_eq!(
        panicked.stdout, "",
        "the kit wrote a panic to the shell's pipe"
    );
}

/// A stand-in copier that has said `script`'s standard error and ended,
/// as a refused copy's has by the time its reason is read.
fn copier_ended_by(script: &str) -> Child {
    let mut child = Command::new("/bin/sh")
        .args(["-c", script])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("sh runs");
    child.wait().expect("the stand-in ends");
    child
}

#[test]
fn a_copier_saying_nothing_is_refused_for_the_daemons_reason() {
    let said = refused_by(copier_ended_by("exit 1"), "it never said it was ready");
    assert_eq!(said, "couldn't copy: it never said it was ready");
}

#[test]
fn a_copiers_reason_is_its_first_line_with_words() {
    let script = r"printf '\n   \n  it is past 1 MiB  \nand more\n' >&2; exit 1";
    let said = refused_by(copier_ended_by(script), "it never said it was ready");
    assert_eq!(said, "couldn't copy: it is past 1 MiB");
}

#[test]
fn a_copiers_reason_is_read_no_further_than_4_kib() {
    // 8 KiB with no line break, then a line that's never reached.
    let script = r"i=0
        while [ $i -lt 128 ]; do
          printf '%s' xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx >&2
          i=$((i + 1))
        done
        printf '\nlater\n' >&2
        exit 1";
    let said = refused_by(copier_ended_by(script), "it never said it was ready");
    assert_eq!(said, format!("couldn't copy: {}", "x".repeat(4096)));
}

#[test]
fn a_copy_nothing_can_wait_to_clear_is_ended_and_refused() {
    let ended = Cell::new(0);
    let answer = waited_to_clear(
        |_| Err(std::io::Error::other("no threads left")),
        || {},
        || ended.set(ended.get() + 1),
    );
    assert_eq!(
        ended.get(),
        1,
        "a copy nothing could wait to clear wasn't ended once"
    );
    let why = answer.expect_err("a copy nothing could wait to clear was answered as copied");
    assert!(
        why.starts_with("couldn't copy: nothing could wait to clear it:"),
        "it was refused saying {why:?}, not that nothing could wait to clear it"
    );
    assert!(
        why.contains("no threads left"),
        "it was refused saying {why:?}, without why: no threads left"
    );
}

#[test]
fn a_copy_whose_wait_began_is_left_held() {
    let ended = Cell::new(0);
    let waited = Cell::new(0);
    let wait = || waited.set(waited.get() + 1);
    let answer = waited_to_clear(
        |wait| {
            wait();
            Ok(())
        },
        wait,
        || ended.set(ended.get() + 1),
    );
    assert_eq!(ended.get(), 0, "a copy whose wait began was ended at once");
    assert_eq!(waited.get(), 1, "the wait to clear it didn't run once");
    assert_eq!(answer, Ok(()), "a copy whose wait began was refused");
}
