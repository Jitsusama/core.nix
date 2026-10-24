//! claude-statusline run as Claude Code runs it: the session on standard input, the colours
//! named as its argument, and the line it prints on standard output.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

const COLOURS: &str =
    r##"{"folder": "#010203", "model": "#040506", "context": "#070809", "separator": "#0a0b0c"}"##;

/// A folder of the test's own, emptied first, for the files a run reads.
fn folder(test: &str) -> PathBuf {
    let folder =
        std::env::temp_dir().join(format!("claude-statusline-{}-{test}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).expect("a folder for the test");
    folder
}

/// Run the status line with those arguments, handing it the session on standard input.
fn run(arguments: &[&str], session: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_claude-statusline"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the status line starts");
    child
        .stdin
        .take()
        .expect("its standard input")
        .write_all(session.as_bytes())
        .expect("the session is written");
    child.wait_with_output().expect("the status line ends")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("text")
}

#[test]
fn it_prints_the_line_with_the_context_its_transcript_holds() {
    let folder = folder("transcript");
    let colours = folder.join("colours.json");
    fs::write(&colours, COLOURS).expect("the colours are written");
    let transcript = folder.join("transcript.jsonl");
    fs::write(
        &transcript,
        r#"{"type": "assistant", "message": {"usage": {"input_tokens": 1234}}}"#,
    )
    .expect("the transcript is written");
    let session = format!(
        r#"{{"workspace": {{"current_dir": "/src/core.nix"}}, "model": {{"id": "opus"}},
            "transcript_path": "{}"}}"#,
        transcript.display()
    );

    let ran = run(&[colours.to_str().expect("a path")], &session);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    let line = text(&ran.stdout);
    assert!(
        line.starts_with("\x1b[1;38;2;1;2;3m\u{f07b} core.nix\x1b[0m"),
        "{line:?}"
    );
    assert!(
        line.ends_with("\x1b[38;2;7;8;9m\u{f019} 1.2k\x1b[0m\n"),
        "{line:?}"
    );
}

#[test]
fn a_transcript_not_written_yet_leaves_the_context_out() {
    let folder = folder("unwritten");
    let colours = folder.join("colours.json");
    fs::write(&colours, COLOURS).expect("the colours are written");
    let session = format!(
        r#"{{"model": {{"id": "opus"}}, "transcript_path": "{}"}}"#,
        folder.join("none.jsonl").display()
    );

    let ran = run(&[colours.to_str().expect("a path")], &session);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(text(&ran.stdout).ends_with("\x1b[38;2;4;5;6m\u{f2db} opus\x1b[0m\n"));
}

#[test]
fn without_its_colours_it_says_how_it_is_used() {
    let ran = run(&[], "{}");

    assert_eq!(ran.status.code(), Some(2));
    assert_eq!(text(&ran.stderr), "Usage: claude-statusline COLOURS\n");
}

#[test]
fn colours_it_cant_read_are_named_and_no_line_is_printed() {
    let missing = folder("missing").join("colours.json");

    let ran = run(&[missing.to_str().expect("a path")], "{}");

    assert_eq!(ran.status.code(), Some(1));
    assert!(ran.stdout.is_empty());
    let said = text(&ran.stderr);
    assert!(said.starts_with(&format!(
        "claude-statusline: can't read {}",
        missing.display()
    )));
}
