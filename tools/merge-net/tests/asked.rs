//! merge-net asked for its help, or asked wrongly: each answers at once in
//! the terminal kit's shape (docs/commands.md), before it reads git or the
//! net's state, so none of these touches either.

use std::process::{Command, Output, Stdio};

/// What merge-net says and ends with, run with `args` in a folder of its
/// own, so a run that reached git would find no repository.
fn ran(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_merge-net"))
        .args(args)
        .current_dir(std::env::temp_dir())
        .env("NO_COLOR", "1")
        .env(
            "MERGE_NET_STATE",
            std::env::temp_dir().join("merge-net-asked"),
        )
        .stdin(Stdio::null())
        .output()
        .expect("merge-net ran")
}

fn said(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn each_subcommand_answers_its_own_help_before_it_reads_a_word() {
    for (asked, title, names) in [
        (&["--help"][..], "merge-net", "ready"),
        (&["ready", "--help"], "merge-net ready", "COMMIT"),
        (&["needs", "--help"], "merge-net needs", "--base"),
        (&["watch", "--help"], "merge-net watch", "--follow"),
        (&["status", "--help"], "merge-net status", "status"),
        (&["run-again", "--help"], "merge-net run-again", "COMMIT"),
        (&["ride", "--help"], "merge-net ride", "train"),
    ] {
        let output = ran(asked);
        let help = said(&output.stdout);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{asked:?} ended {:?}, saying {}",
            output.status,
            said(&output.stderr)
        );
        assert!(help.starts_with(title), "{asked:?}'s help opened {help:?}");
        assert!(help.contains(names), "{asked:?}'s help never named {names}");
        assert!(
            output.stderr.is_empty(),
            "{asked:?}'s help said {:?} on stderr",
            said(&output.stderr)
        );
    }
}

#[test]
fn a_mistyped_word_or_flag_is_refused_naming_the_nearest() {
    for (asked, nearest) in [
        (&["stauts"][..], "did you mean status?"),
        (&["watch", "--folow", "here"], "did you mean --follow?"),
        (&["needs", "--bsae", "main"], "did you mean --base?"),
    ] {
        let output = ran(asked);
        let why = said(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(64),
            "{asked:?} ended {:?}",
            output.status
        );
        assert!(why.contains(nearest), "{asked:?}'s refusal said {why:?}");
        assert!(
            why.contains("--help"),
            "{asked:?}'s refusal never said where to look: {why}"
        );
        assert!(
            output.stdout.is_empty(),
            "{asked:?}'s refusal said {:?} on stdout",
            said(&output.stdout)
        );
    }
}

#[test]
fn a_subcommand_short_of_its_word_or_given_one_too_many_is_refused() {
    for asked in [
        &[][..],
        &["ready"],
        &["run-again"],
        &["watch", "--from"],
        &["status", "now"],
        &["ride", "now"],
    ] {
        let output = ran(asked);
        assert_eq!(
            output.status.code(),
            Some(64),
            "{asked:?} ended {:?}, saying {}",
            output.status,
            said(&output.stderr)
        );
    }
}
