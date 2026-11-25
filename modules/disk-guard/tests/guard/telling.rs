//! What the guard says on standard error: told whether or not it can be
//! written, since on a Mac that is a file on the disk the guard watches.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// A standard error on a disk with no room left.
struct Full;

impl Write for Full {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::from_raw_os_error(28))
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::from_raw_os_error(28))
    }
}

#[test]
fn a_line_nobody_can_be_told_stops_nothing() {
    // Returning at all is the proof: eprintln! panics here.
    disk_guard::tell(&mut Full, format_args!("disk-guard: the disk is full"));
    let mut told = Vec::new();
    disk_guard::tell(&mut told, format_args!("disk-guard: {}", "a pass"));
    assert_eq!(
        String::from_utf8(told).unwrap(),
        "disk-guard: a pass\n",
        "a line that could be written wasn't written whole"
    );
}

/// Every `.rs` file under `dir`.
fn sources(dir: &Path, found: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
}

#[test]
fn the_guard_says_every_line_through_what_lets_a_failed_write_go() {
    // On 24 November a review found that the guard, told to log and go on
    // when its disk is too full for its status, would panic doing it.
    let mut found = Vec::new();
    sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut found,
    );
    let panicking: Vec<String> = found
        .iter()
        .filter(|path| fs::read_to_string(path).unwrap().contains("eprintln!("))
        .map(|path| path.display().to_string())
        .collect();
    assert!(
        panicking.is_empty(),
        "these say lines with eprintln!, which panics on a full disk: {panicking:?}"
    );
}
