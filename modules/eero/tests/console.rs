//! eero as the Linux console runs it: the views take their marks from the
//! kit's pen, so the console's font is given the stand-ins it can draw.
//! Run as the binary, since the marks a view draws are taken once, for
//! the whole run, which a test inside eero's own tests would share with
//! the golden pages.

use std::fs;
use std::process::Command;

/// What `eero nodes` writes into a pipe under `term`, over the record the
/// golden pages are drawn from, with no theme of the account's.
fn nodes_under(term: &str) -> String {
    let home = std::env::temp_dir().join(format!("eero-console-{}-{term}", std::process::id()));
    let record = home.join("run/eero");
    fs::create_dir_all(&record).expect("the stand-in's folders can be made");
    fs::copy(
        concat!(env!("CARGO_MANIFEST_DIR"), "/home-network.json"),
        record.join("home-network.json"),
    )
    .expect("the record can be copied in");
    let ran = Command::new(env!("CARGO_BIN_EXE_eero"))
        .arg("nodes")
        .env_clear()
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_RUNTIME_DIR", home.join("run"))
        .env("TERM", term)
        .output()
        .expect("eero runs");
    fs::remove_dir_all(&home).expect("the stand-in can be removed");
    assert!(
        ran.status.success(),
        "eero nodes failed under {term}: {}",
        String::from_utf8_lossy(&ran.stderr)
    );
    String::from_utf8(ran.stdout).expect("eero writes UTF-8")
}

#[test]
fn the_console_is_given_the_marks_its_font_can_draw() {
    let page = nodes_under("linux");
    let first = page.lines().next().unwrap_or_default();
    assert!(
        first.contains(" . ") && !page.contains('\u{b7}'),
        "the console was given a separator its font can't draw: {first:?}"
    );
}

#[test]
fn a_terminal_is_given_the_marks_own_glyphs() {
    let page = nodes_under("xterm-256color");
    let first = page.lines().next().unwrap_or_default();
    assert!(
        first.contains(" \u{b7} "),
        "a terminal wasn't given the separator's glyph: {first:?}"
    );
}
