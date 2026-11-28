// The planted tree's test module, declared under a test mark in its parent,
// so no read in it is judged.
use std::fs;
use std::path::Path;

fn fixture(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}
