//! claude-statusline run as Claude Code runs it: the session on standard input, ink asked
//! for since it reads through a pipe, the theme read from the account's config, and the
//! line it prints on standard output.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// The theme a run reads, with each colour from channels so no look is named in the code,
/// and each mark a letter.
fn theme() -> String {
    let hex = |red: u8, green: u8, blue: u8| format!("#{red:02x}{green:02x}{blue:02x}");
    let mark = |glyph: &str| serde_json::json!({"glyph": glyph, "console": glyph});
    serde_json::json!({
        "colours": {
            "strong": hex(1, 2, 3),
            "machine": hex(4, 5, 6),
            "program": hex(7, 8, 9),
            "success": hex(10, 11, 12),
            "rule": hex(13, 14, 15),
        },
        "terminal": {},
        "marks": {"folder": mark("F"), "agent": mark("A"), "tokens": mark("T")},
    })
    .to_string()
}

/// A home of the test's own, emptied first, holding the theme.
fn home(test: &str) -> PathBuf {
    let home =
        std::env::temp_dir().join(format!("claude-statusline-{}-{test}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let config = home.join(".config/jitsusama");
    fs::create_dir_all(&config).expect("a home for the test");
    fs::write(config.join("theme.json"), theme()).expect("the theme is written");
    home
}

/// Run the status line with those arguments in that home, as kitty's environment has it,
/// handing it the session on standard input.
fn run(home: &Path, arguments: &[&str], session: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_claude-statusline"))
        .args(arguments)
        .env_clear()
        .env("HOME", home)
        .env("TERM", "xterm-kitty")
        .env("COLORTERM", "truecolor")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the status line starts");
    let written = child
        .stdin
        .take()
        .expect("its standard input")
        .write_all(session.as_bytes());
    // A run that ends before it reads the session, as one refusing its
    // words does, closes the pipe first, which is how it's meant to end,
    // so what it printed is the test's to judge.
    if let Err(error) = written {
        assert_eq!(
            error.kind(),
            std::io::ErrorKind::BrokenPipe,
            "the session is written"
        );
    }
    child.wait_with_output().expect("the status line ends")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("text")
}

#[test]
fn it_prints_the_line_with_the_context_its_transcript_holds() {
    let home = home("transcript");
    let transcript = home.join("transcript.jsonl");
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

    let ran = run(&home, &["--color", "always"], &session);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    let line = text(&ran.stdout);
    assert!(
        line.starts_with("\x1b[38;2;4;5;6mF\x1b[0m \x1b[1;38;2;1;2;3mcore.nix\x1b[0m"),
        "the folder's mark in the machine's colour, its name strong: {line:?}"
    );
    assert!(
        line.ends_with("\x1b[38;2;10;11;12mT\x1b[0m 1.2k\n"),
        "the context's size last, its mark in success: {line:?}"
    );
}

#[test]
fn a_transcript_not_written_yet_leaves_the_context_out() {
    let home = home("unwritten");
    let session = format!(
        r#"{{"model": {{"id": "opus"}}, "transcript_path": "{}"}}"#,
        home.join("none.jsonl").display()
    );

    let ran = run(&home, &["--color", "always"], &session);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    let line = text(&ran.stdout);
    assert!(
        line.ends_with("\x1b[38;2;7;8;9mA\x1b[0m opus\n"),
        "the model last: {line:?}"
    );
}

#[test]
fn into_a_pipe_unasked_it_draws_no_ink() {
    let home = home("plain");

    let ran = run(
        &home,
        &[],
        r#"{"workspace": {"current_dir": "/src/core.nix"}}"#,
    );

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "F core.nix · A unknown\n",
        "the marks and words alone"
    );
}

#[test]
fn as_json_it_answers_the_parts() {
    let home = home("json");

    let ran = run(&home, &["--json"], r#"{"model": {"id": "opus"}}"#);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    let answer: serde_json::Value = serde_json::from_slice(&ran.stdout).expect("JSON");
    assert_eq!(
        answer,
        serde_json::json!({
            "schema": "claude-statusline.line/1",
            "folder": "unknown",
            "model": "opus",
            "context_tokens": null,
        }),
        "the line's parts as facts"
    );
}

#[test]
fn a_word_it_doesnt_take_is_refused() {
    let home = home("misused");

    let ran = run(&home, &["colours.json"], "{}");

    assert_eq!(ran.status.code(), Some(64), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains("it takes no colours.json"),
        "{}",
        text(&ran.stderr)
    );
}

#[test]
fn a_session_that_isnt_json_is_named_and_no_line_is_printed() {
    let home = home("garbled");

    let ran = run(&home, &[], "{not json");

    assert_eq!(ran.status.code(), Some(65), "{}", text(&ran.stderr));
    assert_eq!(ran.stdout, [] as [u8; 0], "no line");
    assert!(
        text(&ran.stderr).contains("the session isn't JSON"),
        "{}",
        text(&ran.stderr)
    );
}
