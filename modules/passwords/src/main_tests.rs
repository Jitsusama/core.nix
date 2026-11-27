//! The daemon's path through the terminal kit, whose stdout is the shell's
//! pipe: the kit writes no byte of its own there, whether the daemon ends
//! well, fails or panics, each said on stderr alone; and how a copier's
//! refusal is said from what it said as it ended.

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
