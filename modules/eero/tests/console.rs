//! eero as it's run: the views take their marks and their ink from the
//! kit's pen, so the console's font is given the stand-ins it can draw
//! and the colour flag and NO_COLOR are settled as every command settles
//! them. Run as the binary, since the marks a view draws are taken once,
//! for the whole run, which a test inside eero's own tests would share
//! with the golden pages, and since a page is inked or not by the pen
//! `main` builds, which only the binary reaches.

use std::fs;
use std::path::Path;
use std::process::Command;

/// What `eero nodes` writes into a pipe given `args` and `vars`, over the
/// record the golden pages are drawn from, with no theme of the account's.
/// The stand-in home is under cargo's own temporary folder, which only
/// the build owns, named for the test, and made fresh.
fn nodes(test: &str, args: &[&str], vars: &[(&str, &str)]) -> String {
    let home = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("eero-console-{test}"));
    if home.exists() {
        fs::remove_dir_all(&home).expect("a stand-in left by an earlier run can be removed");
    }
    fs::create_dir(&home).expect("the stand-in home can be made");
    let record = home.join("run/eero");
    fs::create_dir_all(&record).expect("the stand-in's folders can be made");
    fs::copy(
        concat!(env!("CARGO_MANIFEST_DIR"), "/home-network.json"),
        record.join("home-network.json"),
    )
    .expect("the record can be copied in");
    let ran = Command::new(env!("CARGO_BIN_EXE_eero"))
        .arg("nodes")
        .args(args)
        .env_clear()
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_RUNTIME_DIR", home.join("run"))
        .envs(vars.iter().copied())
        .output()
        .expect("eero runs");
    fs::remove_dir_all(&home).expect("the stand-in can be removed");
    assert!(
        ran.status.success(),
        "eero nodes failed for {test}: {}",
        String::from_utf8_lossy(&ran.stderr)
    );
    String::from_utf8(ran.stdout).expect("eero writes UTF-8")
}

/// Whether a page holds any of the terminal's escapes, which only ink
/// writes into a pipe.
fn inked(page: &str) -> bool {
    page.contains('\u{1b}')
}

#[test]
fn a_run_on_the_console_is_given_the_marks_its_font_can_draw() {
    let page = nodes("console", &[], &[("TERM", "linux")]);
    let first = page.lines().next().unwrap_or_default();
    assert!(
        first.contains(" . ") && !page.contains('\u{b7}'),
        "the console was given a separator its font can't draw: {first:?}"
    );
}

#[test]
fn a_run_off_the_console_is_given_the_marks_own_glyphs() {
    let page = nodes("glyphs", &[], &[("TERM", "xterm-256color")]);
    let first = page.lines().next().unwrap_or_default();
    assert!(
        first.contains(" \u{b7} "),
        "a run off the console wasn't given the separator's glyph: {first:?}"
    );
}

#[test]
fn a_pipe_is_given_no_ink_unless_its_asked_for() {
    let page = nodes("pipe", &[], &[("TERM", "xterm-256color")]);
    assert!(!inked(&page), "a pipe was inked unasked: {page:?}");
}

#[test]
fn the_colour_flag_inks_a_pipe() {
    let page = nodes("always", &["--color=always"], &[("TERM", "xterm-256color")]);
    assert!(inked(&page), "--color=always left the page plain: {page:?}");
}

#[test]
fn force_color_inks_a_pipe_and_no_color_wins_over_it() {
    let forced = nodes(
        "forced",
        &[],
        &[("TERM", "xterm-256color"), ("FORCE_COLOR", "1")],
    );
    assert!(
        inked(&forced),
        "FORCE_COLOR left the page plain: {forced:?}"
    );
    let refused = nodes(
        "refused",
        &[],
        &[
            ("TERM", "xterm-256color"),
            ("FORCE_COLOR", "1"),
            ("NO_COLOR", "1"),
        ],
    );
    assert!(
        !inked(&refused),
        "NO_COLOR didn't win over FORCE_COLOR: {refused:?}"
    );
}
