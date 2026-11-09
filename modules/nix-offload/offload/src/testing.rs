//! What the unit tests share: stand-in programs they write and run.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

/// Write a script to run through `cp`, so no thread of this process ever
/// holds it open for writing: a test forking meanwhile would keep that
/// handle until its child execs, and a script run then is refused as busy
/// (ETXTBSY), which failed a run on 31 October and another on 9 November.
pub fn executable(path: &Path, script: &str) {
    let draft = path.with_extension("draft");
    fs::write(&draft, script).expect("writing the draft");
    let copied = Command::new("cp")
        .arg(&draft)
        .arg(path)
        .status()
        .expect("running cp");
    assert!(copied.success(), "copying {}", path.display());
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("making it executable");
}
