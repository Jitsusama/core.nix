//! boot-stopped's command line, run as the console runs it: what it
//! refuses before it asks systemd anything, so no machine's units reach
//! these tests.

use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_boot-stopped"))
        .args(args)
        .env_remove("NO_COLOR")
        .output()
        .expect("boot-stopped didn't run")
}

#[test]
fn a_form_for_a_program_is_refused_as_usage() {
    for form in ["json", "jsonl"] {
        let ran = run(&["--format", form]);
        assert_eq!(
            ran.status.code(),
            Some(64),
            "--format {form} wasn't refused as usage"
        );
        // A stream of records still ends with the kit's own record of
        // how it ended; nothing of a summary goes with it.
        let said = String::from_utf8_lossy(&ran.stdout);
        assert!(
            said.is_empty() || said == "{\"exit\":64,\"outcome\":\"failed\",\"type\":\"end\"}\n",
            "--format {form} said more than how it ended: {said:?}"
        );
        assert!(
            String::from_utf8_lossy(&ran.stderr).contains("says only text"),
            "--format {form} wasn't told why: {:?}",
            String::from_utf8_lossy(&ran.stderr)
        );
    }
}

#[test]
fn a_word_is_refused_as_usage() {
    let ran = run(&["now"]);
    assert_eq!(ran.status.code(), Some(64), "a word wasn't refused as usage");
    assert!(
        String::from_utf8_lossy(&ran.stderr).contains("takes no words, not now"),
        "a word wasn't named in its refusal: {:?}",
        String::from_utf8_lossy(&ran.stderr)
    );
}
